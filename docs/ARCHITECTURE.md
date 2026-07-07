# Architecture

## Module Structure

```
src/
  main.rs          — tokio event loop, CLI (clap), signal handling
  usb_dongle.rs    — Kingsun KS-959 protocol (obfuscation, speed, fragmentation)
  sir_framing.rs   — IrDA SIR wrap/unwrap (optional, off by default)
  pty_bridge.rs    — PTY pair, symlink, baud rate polling
kmod/
  ks959_speed.c    — Kernel module to bypass usbfs for speed change
  Makefile          — Build against running kernel's headers
```

## Event Loop (main.rs)

Single-threaded `tokio` runtime (`current_thread` flavor) with `select!` over three sources:

```
loop {
    tokio::select! {
        // 1. PTY master readable — data from dctool/Subsurface
        readable = async_master.readable() => {
            - Read from PTY master (non-blocking)
            - Check for baud rate change via tcgetattr() polling
            - If baud changed: reset RX counter, send speed change to dongle
            - Forward data to dongle (with optional SIR wrapping)
        }

        // 2. USB RX poll timer (10ms interval) — data from IR
        _ = poll_interval.tick() => {
            - Poll dongle for received bytes (control-IN transfer)
            - De-obfuscate with persistent counter
            - Write to PTY master (so dctool reads it)
        }

        // 3. Signal handler (SIGINT/SIGTERM) — clean shutdown
        _ = sigint.recv() => { break; }
        _ = sigterm.recv() => { break; }
    }
}
```

### Key Design Decisions

- **tokio `current_thread`** — no need for multi-threaded runtime; all I/O is sequential
- **`AsyncFd` for PTY master** — wraps the raw fd for async readability notifications
- **10ms poll interval** — matches kernel driver's URB resubmission cadence
- **Baud rate detection via `tcgetattr()` polling** — TIOCPKT was evaluated and rejected
  (doesn't fire for plain `tcsetattr()` baud rate changes on Linux)

## Data Flow

### TX Path (dctool → dongle → IR)

```
dctool writes to PTY slave
  → PTY master becomes readable
  → bridge reads from PTY master
  → check_baud_rate_change() (tcgetattr on slave fd)
  → dongle.send() obfuscates + fragments
  → USB control-OUT transfer (bRequestType=0x21, wIndex=0x0000)
  → dongle transmits over IrDA SIR
```

### RX Path (IR → dongle → dctool)

```
dongle receives IrDA SIR data
  → bridge polls every 10ms (USB control-IN, bRequestType=0xA1)
  → deobfuscate_rx_buffer() with persistent counter
  → write to PTY master
  → dctool reads from PTY slave
```

### Speed Change Path

```
dctool calls tcsetattr() on PTY slave (sets 115200 baud)
  → bridge detects via tcgetattr() polling on next PTY read
  → dongle.reset_rx_counter()  ← critical: flush stale bytes
  → dongle.set_speed(115200)
  → USB control-OUT (bRequestType=0x41, wIndex=0x0001)
  → dongle reconfigures IR link
  (or STALL if usbfs blocks it — kernel module is the reliable path)
```

## Dependencies

```toml
nusb = "0.1"           # async USB (pure Rust, no libusb C dependency)
tokio = "1"            # async runtime (current_thread flavor)
nix = "0.29"           # PTY, termios, fcntl (features: term, ioctl, fs)
clap = "4"             # CLI parsing (derive feature)
tracing = "0.1"        # structured logging
tracing-subscriber = "0.3"  # log output (env-filter feature)
thiserror = "2"        # typed errors per module
anyhow = "1"           # top-level error chaining in main
crc = "3"              # in Cargo.toml but NOT USED — CRC computed at compile time
```

## Error Handling

- `thiserror` enums in each module (`DongleError`, `PtyError`, `SirError`)
- `anyhow` in `main.rs` for `.context()` chaining
- No `unwrap()` on the data path — only in one-time setup where failure is fatal
- `EAGAIN`/`EWOULDBLOCK` from PTY reads is normal → clear readiness, retry

## CLI Options

```
-s, --symlink PATH    PTY symlink path          [default: /tmp/cressi-irda]
-b, --baud RATE       Initial IrDA baud rate    [default: 9600]
    --skip-speed-change  Skip USB speed change at startup (use with kernel module)
    --poll-ms MS      USB RX poll interval      [default: 10]
    --sir-framing     Enable SIR BOF/EOF/CRC    [default: off]
    --extra-bofs N    Extra BOFs in SIR mode    [default: 10]
```

## Reference Code (do not modify)

| File                                           | Description                                               |
|------------------------------------------------|-----------------------------------------------------------|
| `reference/ks959-sir.c`                        | Original kernel driver — canonical USB protocol reference |
| `reference/libdivecomputer/src/cressi_goa.c`   | Donatello protocol (commands, framing, CRC)               |
| `reference/libdivecomputer/src/serial_posix.c` | ENABLE_PTY handling, ioctl tolerance                      |
| `reference/libdivecomputer/examples/`          | dctool source (builds to `.libs/dctool`)                  |
