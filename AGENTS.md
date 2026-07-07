# AGENTS.md

## Project Overview

ks959-bridge bridges a Kingsun KS-959 IrDA USB dongle (VID `07d0`, PID `4959`) to libdivecomputer so a Cressi
Donatello dive computer can download dives over infrared. It exposes a PTY that looks like a serial port to
Subsurface/dctool.

**This is NOT a kernel driver replacement.** The old kernel IrDA stack (`irda` + `ks959-sir`) was tested and fails — the
Donatello doesn't complete the IrLAP connection handshake (confirmed via `irdadump`). This project bypasses the entire
IrDA protocol stack, talking directly to the dongle over USB control transfers. It requires significant
reverse-engineering to determine what the dive computer expects as IR signals and how to craft them with this specific
hardware.

**Current status:** All unit tests pass (45/45). USB TX, RX, PTY bridge, and speed change (via kernel module) all
verified on hardware. CMD_VERSION response successfully received from Donatello. The remaining work is end-to-end
dive download testing. See `docs/STATUS.md` for details.

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

### Kernel module (`kmod/`)

| File             | Purpose                                                                 |
|------------------|-------------------------------------------------------------------------|
| `ks959_speed.c`  | Minimal kernel module to bypass usbfs for speed change                  |
| `Makefile`        | Build against running kernel's headers                                  |

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
cd kmod && make                # build kernel module
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

## Critical Domain Knowledge

- **The Donatello uses raw serial over IrDA SIR** — no BOF/EOF/escape/CRC framing. `--sir-framing` is off by default.
- **RX de-obfuscation counter** (`rx_counter: u8`) persists across ALL reads for the entire session. If bytes are lost,
  the counter desyncs and all subsequent decoding is garbage. Recovery requires dongle reset.
- **Stale data drain + counter reset** on startup and baud rate change prevents the known desync bug.
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
- **Speed change requires kernel module** — usbfs `check_ctrlrecip()` blocks `wIndex=0x0001` with `bRequestType=0x21`.
  The `ks959_speed.ko` module bypasses this. Can only be used once per USB plug cycle.
- **LD_PRELOAD shim needed** — libdivecomputer calls `TIOCMBIC` (clear RTS) which PTYs don't support. The shim silently
  succeeds for modem-control ioctls.
- **dctool binary** — use `.libs/dctool` (real binary), not the libtool wrapper. Set `LD_LIBRARY_PATH` to
  `./reference/libdivecomputer/src/.libs`.
- **Donatello hibernates after ~1 minute** in PC mode. Have everything staged before putting it in PC mode.

## Before You Commit

```bash
cargo test
cargo fmt --check
```

## Documentation

- `docs/README.md` — overview and quick start
- `docs/SETUP.md` — build, environment, hardware, kernel module, LD_PRELOAD shim
- `docs/PROTOCOL.md` — USB protocol, IrDA SIR, Cressi Donatello wire format
- `docs/ARCHITECTURE.md` — code structure, modules, event loop, dependencies
- `docs/STATUS.md` — what works, what doesn't, the speed change saga, known issues
- `docs/TESTING.md` — dctool commands, debugging, log analysis, common issues
- `DESIGN.md` — original design document (architecture, approaches evaluated)
- `KNOWLEDGE.md` — complete knowledge base (may be partially outdated, prefer `docs/`)
- `README.md` — user-facing build/usage instructions
