# ks959-bridge

Bridge between the **Kingsun KS-959** IrDA USB dongle (VID `07d0`, PID `4959`) and
[libdivecomputer](https://libdivecomputer.org/) / [Subsurface](https://subsurface-divelog.org/) for downloading dives
from a **Cressi Donatello** dive computer over infrared.

## Why

Linux removed the IrDA subsystem in kernel 4.17 (2018), and the old kernel driver doesn't work with the Donatello anyway
— it never completes the IrLAP handshake. This project bypasses the entire IrDA protocol stack, reverse-engineering
direct USB control transfers to the dongle so libdivecomputer can communicate with the dive computer.

## Build

```bash
cargo build --release
```

## Quick Start

```bash
# 1. Build the kernel module (sets dongle to 115200 baud)
cd kmod && make && cd ..

# 2. Load kernel module
sudo insmod kmod/ks959_speed.ko baud=115200

# 3. Start bridge
sudo ./target/release/ks959-bridge --baud 115200 --skip-speed-change

# 4. In another terminal, run dctool against the PTY
LD_PRELOAD=/tmp/pty_modem_shim.so \
LD_LIBRARY_PATH=./reference/libdivecomputer/src/.libs \
  ./reference/libdivecomputer/examples/.libs/dctool \
  -v -l cressi.log -f goa -m 4 download -o dives.xml /tmp/cressi-irda
```

The program creates a PTY and symlinks it to `/tmp/cressi-irda` (configurable with `--symlink`).

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `-s, --symlink PATH` | `/tmp/cressi-irda` | PTY symlink path |
| `-b, --baud RATE` | `9600` | Initial IrDA baud rate |
| `--skip-speed-change` | off | Skip USB speed change at startup (use with kernel module) |
| `--poll-ms MS` | `10` | USB RX polling interval |
| `--sir-framing` | off | Enable IrDA SIR framing (BOF/EOF/CRC) |

### Logging

```bash
RUST_LOG=debug ./target/release/ks959-bridge    # protocol events
RUST_LOG=trace ./target/release/ks959-bridge    # every USB transfer (hex dumps)
```

## Tests

```bash
cargo test
```

45 tests cover SIR framing (CRC, state machine, round-trips), USB obfuscation/deobfuscation, and PTY bridge operations.

## Permissions

The program needs USB access. Run as `root`, or set up a udev rule:

```
# /etc/udev/rules.d/99-kingsun.rules
SUBSYSTEM=="usb", ATTR{idVendor}=="07d0", ATTR{idProduct}=="4959", MODE="0666"
```

Then reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`

## Documentation

- **[docs/SETUP.md](docs/SETUP.md)** — Build, environment, hardware, kernel module, LD_PRELOAD shim, alternative hardware
- **[docs/PROTOCOL.md](docs/PROTOCOL.md)** — USB protocol, IrDA SIR, Cressi Donatello wire format
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — Code structure, modules, event loop, dependencies
- **[docs/STATUS.md](docs/STATUS.md)** — What works, what doesn't, the speed change saga, known issues
- **[docs/TESTING.md](docs/TESTING.md)** — dctool commands, debugging, log analysis, common issues

## Architecture

```
  Subsurface / dctool
       |
  /tmp/cressi-irda (PTY slave — looks like a serial port)
       |
  +--- ks959-bridge ---+
  | pty_bridge         |  PTY master, non-blocking I/O, baud rate polling
  | usb_dongle         |  Kingsun KS-959: obfuscation, fragmentation, USB control transfers
  | sir_framing        |  IrDA SIR wrap/unwrap (optional, off by default)
  | main.rs            |  tokio select! loop: PTY ↔ USB bridge
  +--------------------+
       |
  USB control transfers (endpoint 0)
       |
  Kingsun KS-959 dongle
       |
  IrDA SIR (infrared)
       |
  Cressi Donatello
```

## Alternatives

If ks959-bridge doesn't work for your setup, other approaches exist:

| Approach | Cost | Status |
|----------|------|--------|
| ESP32 + TFDU4101 (hardware IrDA UART mode) | ~$8–15 | Proven by hb9eue |
| USB-to-Serial (FTDI/CH340) + TFDU4101 | ~$5–8 | Proven by Daniel Samarin |
| BLE transport (no IrDA hardware needed) | BLE adapter | Supported by libdivecomputer |
| Out-of-tree kernel modules (`github.com/cschramm/irda`) | $0 | Works for other IrDA devices; dead end for Donatello specifically |

See [docs/SETUP.md](docs/SETUP.md) for full details on each approach.

## Source Code

- `reference/ks959-sir.c` — original Linux kernel driver (reverse-engineered USB protocol)
- `reference/irda/` — Linux IrDA subsystem (SIR framing, CRC)
- `reference/libdivecomputer/` — dive computer communication (Cressi Donatello protocol)

## License

GPL-2.0 (matching the original ks959-sir.c kernel driver).
