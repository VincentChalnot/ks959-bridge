# irda2tty

Userspace IrDA SIR driver for the **Kingsun KS-959** USB dongle (VID `07d0`, PID `4959`).  
Bridges the dongle to a PTY so [libdivecomputer](https://libdivecomputer.org/) / [Subsurface](https://subsurface-divelog.org/) can talk to a **Cressi Donatello** dive computer as if using a normal serial port.

## Why

Linux removed the IrDA subsystem in kernel 4.17 (2018). The KS-959 dongle no longer has a working kernel driver. This program replaces the kernel driver entirely in userspace.

## Build

```
cargo build --release
```

## Usage

```
# Plug in the Kingsun KS-959 dongle, then:
sudo ./target/release/irda2tty

# In Subsurface: select /tmp/cressi-irda as the serial port
```

The program creates a PTY and symlinks it to `/tmp/cressi-irda` (configurable with `--symlink`). Point Subsurface at this path.

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `-s, --symlink PATH` | `/tmp/cressi-irda` | PTY symlink path |
| `-b, --baud RATE` | `9600` | Initial IrDA baud rate |
| `--poll-ms MS` | `10` | USB RX polling interval |
| `--sir-framing` | off | Enable IrDA SIR framing (BOF/EOF/CRC) |
| `--extra-bofs N` | `10` | Extra BOFs in SIR mode |

### Baud rate

The dongle starts at the initial baud rate (default 9600). When the application (libdivecomputer) calls `tcsetattr()` to change speed (typically to 115200 for the Donatello), irda2tty detects the change and reconfigures the dongle automatically.

### SIR framing

The Cressi Donatello uses `DC_TRANSPORT_SERIAL` in libdivecomputer — it sends raw serial bytes over IrDA's physical layer, **not** SIR-framed packets. The `--sir-framing` flag is for other IrDA devices that use the full IrDA protocol stack.

### Logging

```
RUST_LOG=debug ./target/release/irda2tty    # protocol events
RUST_LOG=trace ./target/release/irda2tty    # every USB transfer (hex dumps)
```

## Architecture

```
  Subsurface / libdivecomputer
         |
    /tmp/cressi-irda (PTY slave)
         |
  +----- irda2tty -----+
  | pty_bridge          |  PTY master, non-blocking I/O, baud rate polling
  | usb_dongle          |  Kingsun KS-959: obfuscation, fragmentation, USB control transfers
  | sir_framing         |  IrDA SIR wrap/unwrap (optional, off by default)
  | main.rs             |  tokio select! loop: PTY ↔ USB bridge
  +---------------------+
         |
    USB control transfers (endpoint 0)
         |
    Kingsun KS-959 dongle
         |
    IrDA SIR (infrared)
         |
    Cressi Donatello
```

## Permissions

The program needs USB access. Run as `root`, or set up a udev rule:

```
# /etc/udev/rules.d/99-kingsun.rules
SUBSYSTEM=="usb", ATTR{idVendor}=="07d0", ATTR{idProduct}=="4959", MODE="0666"
```

Then reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`

## Tests

```
cargo test
```

45 tests cover SIR framing (CRC, state machine, round-trips), USB obfuscation/deobfuscation, and PTY bridge operations.

## Testing with dctool

To test against the real device without Subsurface:

```bash
# Download dives
dctool -v -l cressi.log -f goa -m 4 download -o dives.xml /tmp/cressi-irda

# Raw memory dump
dctool -f goa -m 4 dump -o dump.bin /tmp/cressi-irda

# Parse previously downloaded dives
dctool parse -i dives.xml
```

The `-f goa` flag selects the Cressi Goa family; `-m 4` selects the Donatello model.

## Alternatives

If irda2tty doesn't work for your setup, other approaches exist:

| Approach | Cost | Status |
|----------|------|--------|
| ESP32 + TFDU4101 (hardware IrDA UART mode) | ~$8–15 | Proven by hb9eue |
| USB-to-Serial (FTDI/CH340) + TFDU4101 | ~$5–8 | Proven by Daniel Samarin |
| BLE transport (no IrDA hardware needed) | BLE adapter | Supported by libdivecomputer |
| Out-of-tree kernel modules (`github.com/cschramm/irda`) | $0 | Works for other IrDA devices; dead end for Donatello specifically |

See `KNOWLEDGE.md` for full details on each approach.

## References

### Source Code

- `reference/ks959-sir.c` — original Linux kernel driver (reverse-engineered USB protocol)
- `reference/irda/` — Linux IrDA subsystem (SIR framing, CRC)
- `reference/libdivecomputer/` — dive computer communication (Cressi Donatello protocol)

### External Links

- [Subsurface issue #4147](https://github.com/subsurface/subsurface/issues/4147) — GitHub discussion with hb9eue's ESP32 solution
- [Subsurface mailing list thread](https://groups.google.com/g/subsurface-divelog/c/ku56SSlCtZU) — Google Groups discussion
- [Out-of-tree IrDA kernel modules](https://github.com/cschramm/irda) — DKMS build of the removed IrDA stack
- `docs/forum-message.txt` — Andrea Lusuardi's guide for Uwatec dive computers on Ubuntu with out-of-tree IrDA
- `docs/perplexity.md` — full research investigation summary with decision rationale
- `docs/irda-kernel-module/` — VM setup, protocol analysis, and alternative approaches

## License

GPL-2.0 (matching the original ks959-sir.c kernel driver).
