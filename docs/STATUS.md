# Project Status

## Why This Project Exists

Linux removed the entire IrDA subsystem in kernel 4.17 (2018). The Kingsun KS-959 dongle
had a kernel driver (`ks959-sir.c`) that lived inside that subsystem. On any modern kernel
(Fedora 43, kernel 7.0.14), the dongle is a dead USB device with no driver.

This is NOT a kernel driver replacement — the old IrDA stack was tested and doesn't work
with the Cressi Donatello. Instead, this project bypasses the entire IrDA protocol stack,
reverse-engineering direct USB communication with the dongle.

### The IrDA Protocol Stack

The full IrDA stack has 5 layers. The kernel handled layers 2–4; the dongle handles
layers 1 and 5 in hardware:

```
┌─────────────────────────────────────────────────┐
│  Application (libdivecomputer / dctool)         │
│  Opens /dev/ircomm0, reads/writes serial data   │
├─────────────────────────────────────────────────┤
│  IrCOMM (ircomm_tty kernel module)              │
│  Serial port emulation over IrDA                │
│  Presents /dev/ircomm* character devices        │
├─────────────────────────────────────────────────┤
│  IrLMP (IrDA Link Management Protocol)          │
│  Service discovery (IAS), multiplexing          │
├─────────────────────────────────────────────────┤
│  IrLAP (IrDA Link Access Protocol)              │
│  Framing, connection management, flow control   │
│  SNRM/UA handshake, I-frames, RR/RNR            │
├─────────────────────────────────────────────────┤
│  SIR (Serial Infrared) — kernel driver          │
│  Byte stuffing, BOF/EOF, CRC                    │
│  ks959_sir driver for Kingsun adapter           │
├─────────────────────────────────────────────────┤
│  IrPHY — hardware (KS-959 transceiver)          │
│  IrDA SIR modulation/demodulation               │
│  940nm IR LED + photodiode                      │
└─────────────────────────────────────────────────┘
```

The Cressi Donatello only implements the physical layer + minimal discovery (XID). It
does **not** implement the full IrLAP connection protocol. This is why the kernel path
fails and why ks959-bridge bypasses layers 2–4 entirely.

### Why the Kernel IrDA Path Is a Dead End

Confirmed via `irdadump` capture on an Ubuntu 18.04 VM with the kernel IrDA stack loaded:

```
21:01:30.667420 xid:cmd 4410ab90 > ffffffff S=6 s=* cressi hint=8404 [ Computer IrCOMM ] (23)
```

**XID discovery succeeds** — the Donatello responds and advertises itself as a Cressi
IrCOMM device (hint bits `0x8404`). The IrDA SIR physical layer works.

**No IrLAP connection handshake follows** — no SNRM (connect request), no UA
(acknowledge), no data frames. The Donatello implements just enough IrDA to be
discoverable, but never completes a full IrLAP session.

The hang sequence when running `dctool` against `/dev/ircomm0`:

1. dctool opens `/dev/ircomm0`
2. IrCOMM layer requests an IrLAP connection to the discovered device
3. IrLAP sends SNRM (or attempts to)
4. The Donatello does not respond to SNRM
5. IrLAP waits indefinitely (no timeout on connection establishment)
6. IrCOMM never gets a connected channel
7. dctool's write blocks forever

This cannot be fixed without modifying the kernel IrDA stack to add a "raw" mode.

### Approaches Evaluated

| # | Approach                                          | Verdict                                                             | Cost        |
|---|---------------------------------------------------|---------------------------------------------------------------------|-------------|
| 1 | Official Cressi BT/USB dock                       | Works but expensive; uses MCP2221A + IrDA transceiver internally    | ~$80–120    |
| 2 | KS-959 + kernel IrDA stack (VM)                   | **Dead end** — Donatello never completes IrLAP handshake            | $0          |
| 3 | Out-of-tree IrDA kernel modules (`cschramm/irda`) | Viable for other IrDA devices, same IrLAP dead end for Donatello    | $0          |
| 4 | Flipper Zero (RX capture or TX raw replay)        | **Not viable** — TSOP75338TR hardware-locked to 38kHz carrier       | $0          |
| 5 | Bare LED + Arduino GPIO/UART                      | **Not viable** — produces raw baseband NRZ, not IrDA SIR modulation | ~$5         |
| 6 | ESP32 + TFDU4101 (hardware IrDA UART mode)        | **Proven** by hb9eue for Cressi Leonardo                            | ~$8–15      |
| 7 | USB-to-Serial (FTDI/CH340) + TFDU4101             | **Proven** by Daniel Samarin                                        | ~$5–8       |
| 8 | BLE transport                                     | Supported by libdivecomputer for Goa family                         | BLE adapter |
| 9 | **libusb bypass of KS-959**                       | **Chosen path** — no new hardware, no soldering                     | $0          |

**Why approach #9 was selected:** Zero additional cost, uses existing hardware, avoids
the proven IrLAP dead end, avoids SMD soldering risk.

## What Works (verified on hardware)

| Component                         | Status       | Notes                                                         |
|-----------------------------------|--------------|---------------------------------------------------------------|
| USB enumeration + interface claim | ✅ Works      | nusb finds dongle, detaches kernel driver, claims interface 0 |
| USB TX (data → dongle)            | ✅ Works      | bRequestType=0x21, wIndex=0x0000 passes usbfs validation      |
| USB RX polling (dongle → host)    | ✅ Works      | Polls every 10ms, all URBs complete with status=0             |
| RX de-obfuscation                 | ✅ Works      | Counter arithmetic correct, garbage byte skip works           |
| PTY bridge creation               | ✅ Works      | Creates slave, symlink to `/tmp/cressi-irda`                  |
| Baud rate detection               | ✅ Works      | tcgetattr() polling detects tcsetattr() from dctool           |
| Speed change via kernel module    | ✅ Works      | `insmod ks959_speed.ko baud=115200` succeeds                  |
| CMD_VERSION response              | ✅ Works      | Donatello responds with model=4, firmware=305, serial=44314   |
| LD_PRELOAD shim for modem ioctls  | ✅ Works      | Silently succeeds for TIOCMBIC/TIOCMBIS/TIOCMGET              |
| Unit tests                        | ✅ 45/45 pass | SIR framing, obfuscation, PTY bridge                          |

## What Doesn't Work (yet)

| Component                                  | Status      | Notes                                           |
|--------------------------------------------|-------------|-------------------------------------------------|
| Speed change via usbfs (bRequestType=0x41) | ❌ STALLs    | Dongle firmware checks type bits, requires 0x21 |
| Speed change via usbfs (bRequestType=0x21) | ❌ Blocked   | usbfs check_ctrlrecip rejects wIndex=1          |
| End-to-end dive download                   | 🔲 Untested | Need Donatello in PC mode + correct IR link     |

## The Speed Change Saga

### The Problem

The speed change control transfer uses `wIndex=0x0001` with `bRequestType=0x21`
(Class+Interface). In the dongle's protocol, `wIndex=1` is a flag meaning "this is
a speed change" — it's NOT a USB interface number. But the Linux kernel's `usbfs`
validates control transfers in `check_ctrlrecip()`:

```c
case USB_RECIP_INTERFACE:
    ret = findintfep(ps->dev, index);    // look for endpoint with address == wIndex
    if (ret < 0)
        ret = checkintf(ps, index);      // check if interface wIndex is claimed
    break;
```

For `wIndex=1`: no endpoint with address 1 exists, interface 1 doesn't exist →
returns `-ENOENT`. The URB submission is rejected before it reaches the dongle.

### What Was Tried

| Attempt                    | bRequestType | Kernel               | Dongle       | Result                       |
|----------------------------|--------------|----------------------|--------------|------------------------------|
| Original (Class+Interface) | `0x21`       | **REJECTS** (ENOENT) | Would accept | Kernel blocks it             |
| Class+Device               | `0x20`       | Passes               | **STALL**    | Dongle checks recipient bits |
| Vendor+Interface           | `0x41`       | Passes               | **STALL**    | Dongle checks type bits too  |

### The Solution: Kernel Module

`kmod/ks959_speed.c` — a minimal kernel module that calls `usb_control_msg()` directly,
bypassing usbfs. It matches the dongle by VID/PID, changes the speed in its `probe()`
function, then returns `-ENODEV` so it doesn't permanently claim the device.

```bash
sudo insmod kmod/ks959_speed.ko baud=115200
```

**Limitation:** Can only be used once per USB plug cycle (returning `-ENODEV` prevents
re-probing until physical replug). This is fine — we only need one speed change per
session.

## The RX Counter Desync Bug (Found & Fixed)

### Symptom

After the kernel module set the speed and the bridge started, the first CMD_VERSION
response decoded to garbage (e.g., `8F818724` instead of `AAAA...`).

### Root Cause

The kernel module's speed change operation leaves a stale byte in the dongle's buffer.
When the bridge starts and polls the dongle, it reads this stale byte (e.g., `0x8C`)
and increments the RX counter from `0x00` to `0x01`. When the real CMD_VERSION response
arrives, the de-obfuscation starts at counter `0x01` instead of `0x00` — off by one,
producing garbage.

Trace log showing the bug:

```
RX poll raw_len=1 counter_before=0x00 raw_head=[8C]   ← stale byte!
RX decoded decoded_len=1 counter_after=0x01            ← counter now wrong
...
RX poll raw_len=19 counter_before=0x01 raw_head=[D8, D7, ...]  ← version response
RX decoded decoded_len=19 counter_after=0x14           ← decoded with wrong counter
```

### Fix (two parts)

1. **Stale data drain on startup:** Poll the dongle up to 10 times and discard any
   stale data, then reset the counter to 0.

2. **Counter reset on baud rate change:** When dctool opens the PTY and sets the baud
   rate, the bridge resets the RX counter. This handles the case where stale bytes
   arrive between bridge startup and dctool connection.

```rust
// In main.rs — drain stale data
for _ in 0..10 {
    let stale = dongle.poll_receive().await?;
    if stale.is_empty() { break; }
}
dongle.reset_rx_counter();

// In main.rs — reset on baud change
if new_baud != current_baud {
    dongle.reset_rx_counter();  // ← critical
    dongle.set_speed(new_baud).await;
}
```

## Known Issues & Risks

1. **Donatello hibernates after ~1 minute** in PC mode. Everything must be staged
   before putting it in PC mode.

2. **Kernel module single-use:** Can only be used once per USB plug cycle. If you
   need to re-run, unplug and replug the dongle.

3. **RX counter desync:** If bytes are ever lost (USB error, buffer overflow), the
   counter gets permanently out of sync. Recovery requires dongle reset (USB
   re-enumeration). The drain+reset fix handles the known stale byte case, but
   there may be other edge cases.

4. **IR link quality:** The version response decoded correctly in one test but the
   logbook response was garbage in the same session. This could be IR interference,
   weak signal, or the Donatello moving out of range.

5. **The `crc` crate is unused:** CRC-CCITT is computed at compile time with `const fn`
   to match the Linux kernel's reflected polynomial (0x8408). The `crc` crate is in
   `Cargo.toml` but never imported.
