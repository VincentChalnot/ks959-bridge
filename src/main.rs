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
    ///
    /// For rates other than 9600, the ks959_speed kernel module must be loaded
    /// first: `sudo insmod kmod/ks959_speed.ko baud=115200`.  The bridge will
    /// attempt the speed change via usbfs (Vendor+Interface workaround), but
    /// the dongle may STALL it — the kernel module is the reliable path.
    #[arg(short, long, default_value_t = 9600)]
    baud: u32,

    /// Skip the USB speed-change control transfer at startup.
    ///
    /// Use this when the ks959_speed kernel module has already set the dongle
    /// to the desired baud rate.  The bridge will trust that the dongle is
    /// at --baud and skip the (likely-to-STALL) usbfs speed change.
    #[arg(long)]
    skip_speed_change: bool,

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
        "starting ks959-bridge"
    );

    // --- Open the USB dongle ---
    let mut dongle =
        usb_dongle::KingsunDongle::open().context("failed to open Kingsun KS-959 dongle")?;

    // --- Set initial speed ---
    // The dongle defaults to 9600 baud.  For higher rates (e.g. 115200 for
    // Cressi Donatello), the ks959_speed kernel module should be loaded first
    // because usbfs check_ctrlrecip blocks the wIndex=1 speed-change transfer.
    // The bridge tries the usbfs path as a fallback (Vendor+Interface bRequestType
    // 0x41) but the dongle typically STALLs it.
    let mut current_baud = if args.skip_speed_change {
        // Kernel module already set the speed — trust it.
        info!(
            baud = args.baud,
            "skipping USB speed change (--skip-speed-change); assuming dongle is already at target baud rate"
        );
        args.baud
    } else if args.baud != 9600 {
        // Try usbfs speed change.  Warn on failure instead of aborting.
        match dongle.set_speed(args.baud).await {
            Ok(()) => {
                info!(baud = args.baud, "dongle speed changed via usbfs");
                args.baud
            }
            Err(e) => {
                warn!(
                    baud = args.baud,
                    error = %e,
                    "USB speed change STALLED (expected — dongle needs bRequestType=0x21). \
                     Load the ks959_speed kernel module first: \
                     sudo insmod kmod/ks959_speed.ko baud={}",
                    args.baud
                );
                // Assume the dongle is at the requested speed (e.g., module
                // loaded externally).  If it's actually at 9600, IR comms
                // will fail — but at least the bridge starts.
                args.baud
            }
        }
    } else {
        // Default 9600 — dongle powers on at 9600, no change needed.
        9600
    };

    // --- Drain stale data from the dongle ---
    // The kernel module's speed change may leave stale bytes in the dongle's
    // buffer.  Drain them before starting the main loop so the RX counter
    // starts clean.
    for _ in 0..10 {
        let stale = dongle.poll_receive().await?;
        if stale.is_empty() {
            break;
        }
        debug!(len = stale.len(), "drained stale data from dongle");
    }
    dongle.reset_rx_counter();

    // --- Create PTY bridge ---
    let mut pty =
        pty_bridge::PtyBridge::new(&args.symlink).context("failed to create PTY bridge")?;

    info!(
        "ready — open {} in Subsurface (or: minicom -D {})",
        args.symlink.display(),
        args.symlink.display()
    );

    // --- SIR framing state (if enabled) ---
    let mut unwrapper = sir_framing::SirUnwrapper::new();

    // --- Wrap the PTY master fd for async I/O ---
    let async_master = AsyncFd::with_interest(pty.master_raw_fd(), Interest::READABLE)
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
                                // A baud rate change means a new client
                                // (dctool/Subsurface) just opened the PTY.
                                // Reset the RX counter to flush stale bytes
                                // that may have desynchronized it.
                                dongle.reset_rx_counter();

                                match dongle.set_speed(new_baud).await {
                                    Ok(()) => {
                                        info!(baud = new_baud, "dongle speed changed");
                                        current_baud = new_baud;
                                    }
                                    Err(e) => {
                                        // The dongle STALLs the Vendor+Interface
                                        // workaround.  If the kernel module already
                                        // set the correct speed, this is harmless.
                                        warn!(
                                            baud = new_baud,
                                            error = %e,
                                            "USB speed change STALLED — if the ks959_speed kernel \
                                             module already set this speed, IR comms will still work"
                                        );
                                        current_baud = new_baud;
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
