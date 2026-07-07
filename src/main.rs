mod pty_bridge;
mod sir_framing;
mod usb_dongle;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::io::unix::AsyncFd;
use tokio::io::Interest;
use tracing::{debug, error, info, warn};

/// Userspace IrDA SIR driver for Kingsun KS-959 USB dongle.
///
/// Bridges the dongle to a PTY so libdivecomputer/Subsurface can communicate
/// with a Cressi Donatello dive computer as if using a normal serial port.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path for the PTY symlink (what Subsurface opens as a serial port).
    #[arg(short, long, default_value = "/tmp/cressi-irda")]
    symlink: PathBuf,

    /// Initial baud rate for the IrDA link.
    #[arg(short, long, default_value_t = 9600)]
    baud: u32,

    /// USB RX polling interval in milliseconds.
    #[arg(long, default_value_t = 10)]
    poll_ms: u64,

    /// Enable IrDA SIR framing (BOF/EOF/escape/CRC wrapping).
    /// Not needed for Cressi Donatello which uses raw serial over IrDA SIR.
    #[arg(long)]
    sir_framing: bool,

    /// Number of extra BOFs prepended in SIR mode (only with --sir-framing).
    #[arg(long, default_value_t = 10)]
    extra_bofs: usize,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Initialize tracing (set RUST_LOG=debug or RUST_LOG=trace for more).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    info!(
        symlink = %args.symlink.display(),
        baud = args.baud,
        poll_ms = args.poll_ms,
        sir_framing = args.sir_framing,
        "starting irda2tty"
    );

    // --- Open the USB dongle ---
    let mut dongle = usb_dongle::KingsunDongle::open()
        .context("failed to open Kingsun KS-959 dongle")?;

    // --- Set initial speed ---
    dongle
        .set_speed(args.baud)
        .await
        .context("failed to set initial baud rate")?;
    let mut current_baud = args.baud;

    // --- Create PTY bridge ---
    let mut pty = pty_bridge::PtyBridge::new(&args.symlink)
        .context("failed to create PTY bridge")?;

    info!(
        "ready — open {} in Subsurface (or: minicom -D {})",
        args.symlink.display(),
        args.symlink.display()
    );

    // --- SIR framing state (if enabled) ---
    let mut unwrapper = sir_framing::SirUnwrapper::new();

    // --- Wrap the PTY master fd for async I/O ---
    let async_master = AsyncFd::with_interest(
        pty.master_raw_fd(),
        Interest::READABLE,
    )
    .context("failed to create AsyncFd for PTY master")?;

    // We don't take ownership of the fd — AsyncFd<RawFd> doesn't close it.
    // The PtyBridge still owns the OwnedFd.

    // --- RX poll timer ---
    let mut poll_interval = tokio::time::interval(Duration::from_millis(args.poll_ms));
    poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    // --- Signal handler ---
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .context("failed to register SIGINT handler")?;
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("failed to register SIGTERM handler")?;

    // --- Main event loop ---
    let mut pty_buf = [0u8; 4096];

    loop {
        tokio::select! {
            // --- PTY master readable: data from the application ---
            readable = async_master.readable() => {
                let mut guard = readable.context("AsyncFd readable error")?;

                match pty.read(&mut pty_buf) {
                    Ok(0) => {
                        info!("PTY slave closed (EOF), exiting");
                        break;
                    }
                    Ok(n) => {
                        debug!(len = n, "PTY → dongle");

                        // Check for baud rate change before forwarding.
                        if let Some(new_baud) = pty.check_baud_rate_change()? {
                            if new_baud != current_baud {
                                match dongle.set_speed(new_baud).await {
                                    Ok(()) => {
                                        current_baud = new_baud;
                                    }
                                    Err(e) => {
                                        warn!(baud = new_baud, error = %e, "speed change failed, keeping {}", current_baud);
                                    }
                                }
                            }
                        }

                        // Forward to dongle (with optional SIR wrapping).
                        let tx_data = if args.sir_framing {
                            sir_framing::wrap_frame(&pty_buf[..n], args.extra_bofs)
                        } else {
                            pty_buf[..n].to_vec()
                        };

                        if let Err(e) = dongle.send(&tx_data).await {
                            error!(error = %e, "USB TX failed");
                        }
                    }
                    Err(e) => {
                        // EAGAIN is normal — clear readiness and retry.
                        if is_would_block(&e) {
                            guard.clear_ready();
                        } else {
                            error!(error = %e, "PTY read error");
                            break;
                        }
                    }
                }
            }

            // --- USB RX poll tick ---
            _ = poll_interval.tick() => {
                match dongle.poll_receive().await {
                    Ok(data) if data.is_empty() => {
                        // No data — normal.
                    }
                    Ok(data) => {
                        debug!(len = data.len(), "dongle → PTY");

                        let rx_data = if args.sir_framing {
                            // Feed through SIR unwrapper, concatenate payloads.
                            let mut frames = Vec::new();
                            for frame in &unwrapper.process_bytes(&data) {
                                frames.extend_from_slice(frame);
                            }
                            frames
                        } else {
                            data
                        };

                        if !rx_data.is_empty() {
                            match pty.write(&rx_data) {
                                Ok(n) if n < rx_data.len() => {
                                    warn!(
                                        written = n,
                                        total = rx_data.len(),
                                        "partial PTY write (application not reading fast enough)"
                                    );
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    if is_would_block(&e) {
                                        warn!("PTY write would block — dropping {} bytes", rx_data.len());
                                    } else {
                                        error!(error = %e, "PTY write error");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "USB RX poll failed");
                        break;
                    }
                }
            }

            // --- Clean shutdown on signal ---
            _ = sigint.recv() => {
                info!("received SIGINT, shutting down");
                break;
            }
            _ = sigterm.recv() => {
                info!("received SIGTERM, shutting down");
                break;
            }
        }
    }

    // PtyBridge::drop cleans up the symlink.
    info!("goodbye");
    Ok(())
}

/// Check if a PtyError wraps an EAGAIN/EWOULDBLOCK.
fn is_would_block(e: &pty_bridge::PtyError) -> bool {
    match e {
        pty_bridge::PtyError::Nix(nix::Error::EAGAIN) => true,
        pty_bridge::PtyError::Io(io) => io.kind() == std::io::ErrorKind::WouldBlock,
        _ => false,
    }
}
