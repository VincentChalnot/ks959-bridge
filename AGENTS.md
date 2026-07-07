# AGENTS.md

## Project Overview

ks959-bridge is a userspace IrDA SIR driver for the Kingsun KS-959 USB dongle (VID `07d0`, PID `4959`). It bridges the
dongle to a PTY so libdivecomputer/Subsurface can communicate with a Cressi Donatello dive computer as if using a normal
serial port.

**Why:** Linux removed the IrDA subsystem in kernel 4.17 (2018). The KS-959 dongle has no working kernel driver on
modern distros. The Cressi Donatello doesn't complete the IrLAP connection handshake (confirmed via `irdadump`), making
the kernel IrDA stack a dead end even with out-of-tree modules. This project bypasses the entire IrDA protocol stack,
talking directly to the dongle over USB control transfers.

**Current status:** Heavy development. All unit tests pass (45/45). The main blocker is the USB speed change control
transfer — see "Known Blocker" below.

## Tech Stack & Structure

- **Language:** Rust (edition 2021)
- **Async runtime:** tokio (current_thread flavor)
- **USB:** `nusb` (async, pure-Rust, no libusb C dependency)
- **PTY/Unix:** `nix` 0.29 (features: `term`, `ioctl`, `fs`)
- **CLI:** `clap` 4 (derive)
- **Logging:** `tracing` + `tracing-subscriber` (env-filter)
- **Errors:** `thiserror` per module, `anyhow` in main

### Source files (`src/`)

| File             | Purpose                                                                                         |
|------------------|-------------------------------------------------------------------------------------------------|
| `main.rs`        | tokio `select!` event loop: PTY ↔ USB bridge, CLI, signal handling                              |
| `usb_dongle.rs`  | Kingsun KS-959 USB protocol: TX obfuscation, RX de-obfuscation, speed change, fragmentation     |
| `sir_framing.rs` | IrDA SIR wrap/unwrap (BOF/EOF/escape/CRC). Optional, off by default — Donatello uses raw serial |
| `pty_bridge.rs`  | PTY master/slave pair, symlink creation, baud rate polling via `tcgetattr()`                    |

### Reference directory (`reference/`)

Contains source code for reference only — do not modify:

- `ks959-sir.c` — original Linux kernel driver (canonical USB protocol reference)
- `irda/`, `irda.h`, `irda.txt` — Linux IrDA subsystem source
- `libdivecomputer/` — dive computer communication library (Cressi Donatello protocol in `src/cressi_goa.c`)
- `linux/` — full Linux kernel source (for driver reference)

## Setup & Dev Environment

```bash
cargo build --release          # builds to ./target/release/ks959-bridge
cargo test                     # runs all 45 unit tests (no hardware needed)
```

The program needs USB access to the dongle. Run as `root`, or create a udev rule:

```
# /etc/udev/rules.d/99-kingsun.rules
SUBSYSTEM=="usb", ATTR{idVendor}=="07d0", ATTR{idProduct}=="4959", MODE="0666"
```

## Build, Test & Lint Commands

```bash
cargo test                              # all unit tests (no hardware required)
cargo build --release                   # release binary (~2.7MB)
cargo clippy                            # linter (if clippy is available)
cargo fmt --check                       # format check
```

**Debug logging:**

```bash
RUST_LOG=info  ./target/release/ks959-bridge   # default
RUST_LOG=debug ./target/release/ks959-bridge   # protocol events
RUST_LOG=trace ./target/release/ks959-bridge   # hex dumps of every USB transfer
```

## Workflow Rules

- Commit messages: use Conventional Commits format (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`)
- All tests must pass before committing (`cargo test`)
- Run `cargo fmt` before committing
- Run `cargo clippy` if available

## Known Blocker: USB Speed Change

The speed change control transfer uses `wIndex=0x0001` which the Linux kernel's `usbfs` rejects via
`check_ctrlrecip()` (see `KNOWLEDGE.md` § "The usbfs check_ctrlrecip Problem"). Current code uses
`ControlType::Vendor` (bRequestType=0x41) to bypass kernel validation — **untested on hardware**. If this also fails,
fallback options are a minimal kernel module, eBPF hook, or avoiding speed changes entirely.

## Critical Domain Knowledge

- **The Donatello uses raw serial over IrDA SIR** — no BOF/EOF/escape/CRC framing. `--sir-framing` is off by default.
- **RX de-obfuscation counter** (`rx_counter: u8`) persists across ALL reads for the entire session. If bytes are lost,
  the counter desyncs and all subsequent decoding is garbage. Recovery requires dongle reset.
- **Baud rate detection** polls `tcgetattr()` on the slave fd — TIOCPKT does NOT fire for plain `tcsetattr()` baud rate
  changes on Linux.
- **nix 0.29 has no `pty` feature** — PTY functions live under the `term` feature flag.
- **BaudRate enum variants** on Linux are opaque constants (e.g., `B9600 = 0x0D`), NOT raw speed values. Use the
  explicit `match` mapping in `pty_bridge.rs`.
- **The `crc` crate is in Cargo.toml but NOT used** — CRC-CCITT is computed at compile time with `const fn` to match the
  Linux kernel's reflected polynomial (0x8408).
- **Low-speed USB device** — max 8 bytes per control transfer packet. The dongle's interrupt endpoint is a dummy; all
  communication uses endpoint 0 control transfers.
- **Obfuscation key** is the firmware author's name: `"wangshuofei19710"`.

## Before You Commit

```bash
cargo test
cargo fmt --check
```

## Additional References

- `DESIGN.md` — architecture, protocol details, approaches evaluated, test plan
- `KNOWLEDGE.md` — complete knowledge base: USB protocol, IrDA SIR framing, Cressi Donatello protocol, hardware
  alternatives
- `README.md` — user-facing build/usage instructions
