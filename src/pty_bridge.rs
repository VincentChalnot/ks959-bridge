// PTY bridge: creates a pseudo-terminal pair for serial port emulation.
//
// The slave side (symlinked to a user-chosen path like `/tmp/cressi-irda`)
// is opened by libdivecomputer/Subsurface as a normal serial port.
// The master side is read/written by our event loop.
//
// Baud rate detection: the event loop calls `slave_baud_rate()` before
// forwarding data to the dongle.  TIOCPKT was considered but it does NOT
// generate TIOCPKT_IOCTL for plain `tcsetattr()` on Linux — only for
// EXTPROC and IXON/IXOFF changes (see `pty_set_termios` in the kernel).
// Since the baud rate changes once at session start, polling on data
// arrival is more than sufficient.

use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::path::{Path, PathBuf};

use thiserror::Error;
use tracing::{debug, info, trace, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from PTY bridge operations.
#[derive(Debug, Error)]
pub enum PtyError {
    /// PTY creation or I/O error.
    #[error("PTY error: {0}")]
    Io(#[from] std::io::Error),

    /// nix system call error.
    #[error("system error: {0}")]
    Nix(#[from] nix::Error),

    /// Symlink creation failed.
    #[error("failed to create symlink {path}: {source}")]
    Symlink {
        path: PathBuf,
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// BaudRate → u32 conversion
// ---------------------------------------------------------------------------

/// Convert a `nix::sys::termios::BaudRate` enum to the numeric baud rate.
///
/// On Linux the enum variants are opaque constants (e.g., `B9600 = 0x0D`),
/// NOT the raw speed value like on BSDs, so we need an explicit mapping.
fn baudrate_to_u32(b: nix::sys::termios::BaudRate) -> u32 {
    use nix::sys::termios::BaudRate;
    match b {
        BaudRate::B0 => 0,
        BaudRate::B50 => 50,
        BaudRate::B75 => 75,
        BaudRate::B110 => 110,
        BaudRate::B134 => 134,
        BaudRate::B150 => 150,
        BaudRate::B200 => 200,
        BaudRate::B300 => 300,
        BaudRate::B600 => 600,
        BaudRate::B1200 => 1200,
        BaudRate::B1800 => 1800,
        BaudRate::B2400 => 2400,
        BaudRate::B4800 => 4800,
        BaudRate::B9600 => 9600,
        BaudRate::B19200 => 19200,
        BaudRate::B38400 => 38400,
        BaudRate::B57600 => 57600,
        BaudRate::B115200 => 115200,
        BaudRate::B230400 => 230400,
        BaudRate::B460800 => 460800,
        BaudRate::B500000 => 500000,
        BaudRate::B576000 => 576000,
        BaudRate::B921600 => 921600,
        BaudRate::B1000000 => 1_000_000,
        BaudRate::B1152000 => 1_152_000,
        BaudRate::B1500000 => 1_500_000,
        BaudRate::B2000000 => 2_000_000,
        BaudRate::B2500000 => 2_500_000,
        BaudRate::B3000000 => 3_000_000,
        BaudRate::B3500000 => 3_500_000,
        BaudRate::B4000000 => 4_000_000,
        _ => {
            warn!(baud = ?b, "unknown BaudRate variant, returning 0");
            0
        }
    }
}

// ---------------------------------------------------------------------------
// PtyBridge
// ---------------------------------------------------------------------------

/// A PTY master/slave pair for serial port emulation.
///
/// The master fd is set to non-blocking mode.  The slave fd is kept open
/// so we can call `tcgetattr()` to read baud rate changes.
pub struct PtyBridge {
    /// Master file descriptor (our side).
    master: OwnedFd,
    /// Slave file descriptor (kept open for `tcgetattr` calls).
    slave: OwnedFd,
    /// Filesystem path to the slave device (e.g., `/dev/pts/3`).
    #[allow(dead_code)]
    slave_dev_path: PathBuf,
    /// Symlink path we created (for cleanup on drop).
    symlink_path: Option<PathBuf>,
    /// Last known baud rate, to detect changes.
    last_baud: u32,
}

impl PtyBridge {
    /// Create a new PTY pair and symlink the slave device to `symlink`.
    ///
    /// The slave path (e.g., `/tmp/cressi-irda`) is what the application
    /// (Subsurface) opens as a serial port.
    pub fn new(symlink: &Path) -> Result<Self, PtyError> {
        // Create the PTY pair.
        let pty = nix::pty::openpty(None, None)?;

        // Determine the slave device path (e.g., /dev/pts/N).
        let slave_dev_path = nix::unistd::ttyname(&pty.slave)?;
        debug!(slave = %slave_dev_path.display(), "PTY slave device");

        // Set the master fd to non-blocking.
        let flags = nix::fcntl::fcntl(
            pty.master.as_raw_fd(),
            nix::fcntl::FcntlArg::F_GETFL,
        )?;
        nix::fcntl::fcntl(
            pty.master.as_raw_fd(),
            nix::fcntl::FcntlArg::F_SETFL(
                nix::fcntl::OFlag::from_bits_truncate(flags) | nix::fcntl::OFlag::O_NONBLOCK,
            ),
        )?;
        debug!("master fd set to non-blocking");

        // Remove stale symlink if it exists, then create the new one.
        let symlink_path = symlink.to_path_buf();
        if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
            std::fs::remove_file(&symlink_path).ok(); // best-effort
        }
        std::os::unix::fs::symlink(&slave_dev_path, &symlink_path).map_err(|e| {
            PtyError::Symlink {
                path: symlink_path.clone(),
                source: e,
            }
        })?;

        // Read the initial baud rate (typically 38400 after openpty on Linux).
        let termios = nix::sys::termios::tcgetattr(&pty.slave)?;
        let initial_baud = baudrate_to_u32(nix::sys::termios::cfgetospeed(&termios));

        info!(
            slave = %slave_dev_path.display(),
            symlink = %symlink_path.display(),
            initial_baud,
            "PTY bridge created"
        );

        Ok(Self {
            master: pty.master,
            slave: pty.slave,
            slave_dev_path,
            symlink_path: Some(symlink_path),
            last_baud: initial_baud,
        })
    }

    /// Read from the PTY master (non-blocking).
    ///
    /// Returns the number of bytes read into `buf`, or `Err` with EAGAIN
    /// if no data is available.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, PtyError> {
        let n = nix::unistd::read(self.master.as_raw_fd(), buf)?;
        if n > 0 {
            trace!(len = n, "PTY master read");
        }
        Ok(n)
    }

    /// Write data to the PTY master (sent to the slave application).
    pub fn write(&self, data: &[u8]) -> Result<usize, PtyError> {
        if data.is_empty() {
            return Ok(0);
        }
        let n = nix::unistd::write(&self.master, data)?;
        trace!(len = n, "PTY master write");
        Ok(n)
    }

    /// Read the slave's current output baud rate.
    pub fn slave_baud_rate(&self) -> Result<u32, PtyError> {
        let termios = nix::sys::termios::tcgetattr(&self.slave)?;
        let baud = nix::sys::termios::cfgetospeed(&termios);
        let speed = baudrate_to_u32(baud);
        Ok(speed)
    }

    /// Check if the baud rate changed since last check.
    ///
    /// Returns `Some(new_baud)` if the baud rate changed, `None` otherwise.
    /// Updates the internal last-known baud rate on change.
    pub fn check_baud_rate_change(&mut self) -> Result<Option<u32>, PtyError> {
        let current = self.slave_baud_rate()?;
        if current != self.last_baud {
            info!(
                old = self.last_baud,
                new = current,
                "baud rate changed"
            );
            self.last_baud = current;
            Ok(Some(current))
        } else {
            Ok(None)
        }
    }

    /// Path to the slave device (e.g., `/dev/pts/3`).
    #[allow(dead_code)]
    pub fn slave_dev_path(&self) -> &Path {
        &self.slave_dev_path
    }

    /// Path to the symlink (e.g., `/tmp/cressi-irda`).
    #[allow(dead_code)]
    pub fn symlink_path(&self) -> Option<&Path> {
        self.symlink_path.as_deref()
    }

    /// Borrow the master fd (for use with `tokio::io::unix::AsyncFd`).
    #[allow(dead_code)]
    pub fn master_fd(&self) -> BorrowedFd<'_> {
        self.master.as_fd()
    }

    /// Raw master fd (for use with poll/select/epoll).
    pub fn master_raw_fd(&self) -> std::os::fd::RawFd {
        self.master.as_raw_fd()
    }
}

impl Drop for PtyBridge {
    fn drop(&mut self) {
        // Clean up the symlink.
        if let Some(ref path) = self.symlink_path {
            if let Err(e) = std::fs::remove_file(path) {
                warn!(path = %path.display(), error = %e, "failed to remove PTY symlink");
            } else {
                debug!(path = %path.display(), "removed PTY symlink");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;

    /// Create a PTY bridge and verify the symlink exists and points correctly.
    #[test]
    fn create_and_symlink() {
        let symlink = std::env::temp_dir().join("irda2tty-test-pty-create");
        let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");

        let target = std::fs::read_link(&symlink).expect("symlink missing");
        assert_eq!(target, bridge.slave_dev_path());

        drop(bridge);
        assert!(!symlink.exists(), "symlink should be removed on drop");
    }

    /// Helper: open slave, set raw mode, return the file.
    fn open_slave_raw(bridge: &PtyBridge) -> std::fs::File {
        let slave_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(bridge.slave_dev_path())
            .expect("open slave");

        let mut termios = nix::sys::termios::tcgetattr(&slave_file).unwrap();
        nix::sys::termios::cfmakeraw(&mut termios);
        nix::sys::termios::tcsetattr(
            &slave_file,
            nix::sys::termios::SetArg::TCSANOW,
            &termios,
        )
        .unwrap();
        slave_file
    }

    /// Write data to the slave side, read it from the master.
    #[test]
    fn data_roundtrip_slave_to_master() {
        let symlink = std::env::temp_dir().join("irda2tty-test-pty-s2m");
        let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
        let mut slave_file = open_slave_raw(&bridge);

        slave_file.write_all(b"hello").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut buf = [0u8; 256];
        let mut total = 0;
        for _ in 0..20 {
            match bridge.read(&mut buf[total..]) {
                Ok(n) if n > 0 => {
                    total += n;
                    if total >= 5 {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
        assert_eq!(&buf[..total], b"hello");
    }

    /// Write data to the master, read from the slave.
    #[test]
    fn data_roundtrip_master_to_slave() {
        let symlink = std::env::temp_dir().join("irda2tty-test-pty-m2s");
        let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
        let mut slave_file = open_slave_raw(&bridge);

        // Set slave non-blocking for the read.
        let fd = slave_file.as_raw_fd();
        let flags = nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_GETFL).unwrap();
        nix::fcntl::fcntl(
            fd,
            nix::fcntl::FcntlArg::F_SETFL(
                nix::fcntl::OFlag::from_bits_truncate(flags) | nix::fcntl::OFlag::O_NONBLOCK,
            ),
        )
        .unwrap();

        bridge.write(b"world").expect("master write");
        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut buf = [0u8; 256];
        let n = slave_file.read(&mut buf).expect("slave read");
        assert_eq!(&buf[..n], b"world");
    }

    /// Detect a baud rate change via polling tcgetattr.
    #[test]
    fn detect_baud_rate_change() {
        let symlink = std::env::temp_dir().join("irda2tty-test-pty-baud");
        let mut bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");

        let slave_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(bridge.slave_dev_path())
            .expect("open slave");

        // Set 115200 baud on the slave (simulating libdivecomputer).
        let mut termios = nix::sys::termios::tcgetattr(&slave_file).unwrap();
        nix::sys::termios::cfmakeraw(&mut termios);
        nix::sys::termios::cfsetospeed(
            &mut termios,
            nix::sys::termios::BaudRate::B115200,
        )
        .unwrap();
        nix::sys::termios::cfsetispeed(
            &mut termios,
            nix::sys::termios::BaudRate::B115200,
        )
        .unwrap();
        nix::sys::termios::tcsetattr(
            &slave_file,
            nix::sys::termios::SetArg::TCSANOW,
            &termios,
        )
        .unwrap();

        // check_baud_rate_change should detect the change.
        let change = bridge.check_baud_rate_change().expect("check failed");
        assert_eq!(change, Some(115200));

        // Second call: no change.
        let no_change = bridge.check_baud_rate_change().expect("check failed");
        assert_eq!(no_change, None);
    }

    /// Verify initial baud rate.
    #[test]
    fn initial_baud_rate() {
        let symlink = std::env::temp_dir().join("irda2tty-test-pty-initbaud");
        let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
        let speed = bridge.slave_baud_rate().expect("slave_baud_rate");
        assert!(speed > 0, "initial baud rate should be > 0");
    }

    /// Symlink cleanup is idempotent.
    #[test]
    fn drop_cleanup_idempotent() {
        let symlink = std::env::temp_dir().join("irda2tty-test-pty-drop");
        let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
        std::fs::remove_file(&symlink).ok();
        drop(bridge); // should not panic
    }

    /// BaudRate conversion covers all standard rates.
    #[test]
    fn baudrate_conversion() {
        use nix::sys::termios::BaudRate;
        assert_eq!(baudrate_to_u32(BaudRate::B9600), 9600);
        assert_eq!(baudrate_to_u32(BaudRate::B115200), 115200);
        assert_eq!(baudrate_to_u32(BaudRate::B0), 0);
        assert_eq!(baudrate_to_u32(BaudRate::B2400), 2400);
        assert_eq!(baudrate_to_u32(BaudRate::B57600), 57600);
    }
}
