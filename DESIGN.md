# ks959-bridge Design Document

## Problem Statement

Linux removed the IrDA subsystem in kernel 4.17+. The Kingsun KS-959 USB-to-IrDA dongle
(VID=07d0, PID=4959) has no usable driver on modern distros. The old kernel IrDA stack
doesn't work with the Cressi Donatello anyway — it never completes the IrLAP connection
handshake (confirmed via `irdadump`). We need a userspace program that bridges this dongle
to libdivecomputer so the Donatello can download dives over infrared. This requires
reverse-engineering both the dongle's USB protocol and what the dive computer expects as
IR signals.

## IrDA Protocol Stack

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
│  SNRM/UA handshake, I-frames, RR/RNR           │
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
fails and why ks959-bridge bypasses layers 2–4 entirely, talking directly to the dongle
over USB.

## Why the Kernel IrDA Path Is a Dead End

This was confirmed via `irdadump` capture on an Ubuntu 18.04 VM with the kernel IrDA
stack loaded:

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
8. The 3000ms serial timeout in `cressi_goa.c` only applies to reads *after* the
   connection is established — not to the connection itself

This cannot be fixed without modifying the kernel IrDA stack to add a "raw" mode
that skips IrLAP.

## Approaches Evaluated

| # | Approach                                          | Verdict                                                                                                                  | Cost        |
|---|---------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|-------------|
| 1 | Official Cressi BT/USB dock                       | Works but expensive; uses MCP2221A + IrDA transceiver internally                                                         | ~$80–120    |
| 2 | KS-959 + kernel IrDA stack (VM)                   | **Dead end** — Donatello never completes IrLAP handshake                                                                 | $0          |
| 3 | Out-of-tree IrDA kernel modules (`cschramm/irda`) | Viable for other IrDA devices (Uwatec, etc.), same IrLAP dead end for Donatello                                          | $0          |
| 4 | Flipper Zero (RX capture or TX raw replay)        | **Not viable** — TSOP75338TR hardware-locked to 38kHz carrier; IrDA SIR uses 3/16 pulse-width modulation without carrier | $0          |
| 5 | Bare LED + Arduino GPIO/UART                      | **Not viable** — produces raw baseband NRZ, not IrDA SIR modulation                                                      | ~$5         |
| 6 | ESP32 + TFDU4101 (hardware IrDA UART mode)        | **Proven** by hb9eue for Cressi Leonardo; requires SMD soldering of TFDU4101                                             | ~$8–15      |
| 7 | USB-to-Serial (FTDI/CH340/MCP2221A) + TFDU4101    | **Proven** by Daniel Samarin; architecturally identical to Cressi dock                                                   | ~$5–8       |
| 8 | BLE transport                                     | Supported by libdivecomputer for Goa family; avoids IrDA entirely                                                        | BLE adapter |
| 9 | **libusb bypass of KS-959**                       | **Chosen path** — no new hardware, no soldering                                                                          | $0          |

**Why approach #9 was selected:** Zero additional cost, uses existing hardware, avoids
the proven IrLAP dead end, avoids SMD soldering risk. The trade-off is reverse-engineering
the KS-959 USB protocol (documented in the kernel driver) and the risk that the dongle
firmware may require minimal IrLAP negotiation before transmitting raw bytes.

## Architecture Overview

```
                         PTY slave (/tmp/cressi-irda)
                               |
                     Subsurface / libdivecomputer
                     (opens as serial port, 115200 8N1)
                               |
                               v
 +---------------------------------------------------------+
 |                     ks959-bridge                             |
 |                                                          |
 |  +-------------------+     +--------------------------+  |
 |  |   pty_bridge       |<-->|   main.rs event loop     |  |
 |  | PTY master fd      |    | (tokio or poll-based)    |  |
 |  | TIOCPKT for baud   |    +-----------+--------------+  |
 |  +-------------------+                |                  |
 |                                        |                  |
 |  +-------------------+     +--------------------------+  |
 |  |   sir_framing     |     |   usb_dongle              |  |
 |  | (OPTIONAL bypass) |<--->|   Kingsun KS-959 protocol |  |
 |  | BOF/EOF/CE/CRC    |     |   obfuscation, polling    |  |
 |  +-------------------+     +--------------------------+  |
 +---------------------------------------------------------+
                               |
                         USB control transfers
                               |
                     Kingsun KS-959 dongle
                     (IrDA SIR physical layer)
                               |
                         IrDA SIR IR link
                               |
                     Cressi Donatello
```

## Critical Design Decision: SIR Framing

**The Donatello does NOT use IrDA SIR framing (BOF/EOF/escape/CRC).** Evidence:

1. libdivecomputer registers the Donatello as `DC_TRANSPORT_SERIAL` (not `DC_TRANSPORT_IRDA`),
   meaning it sends raw serial bytes — no IrLAP, no SIR framing.
2. Users have successfully communicated with the Donatello using an ESP32 + TFDU4101 IrDA
   transceiver, which only handles physical-layer modulation (NRZ UART <-> IrDA SIR pulses)
   and does NOT add SIR framing.
3. The Kingsun dongle handles IrDA SIR modulation in hardware (converting between electrical
   UART and IR pulses). The SIR async wrapper (BOF/EOF/escape/CRC) in the kernel driver is
   for the IrDA *protocol* stack — which the Donatello doesn't use.

**Therefore:** the default mode is **raw passthrough** (no SIR framing). The SIR framing module
is implemented for completeness and can be enabled via `--sir-framing` for devices that actually
use the full IrDA protocol. The module is separately testable regardless.

## Module Design

### 1. `usb_dongle` — Kingsun KS-959 USB Protocol

**Crate:** `nusb` (async, pure-Rust, actively maintained — preferred over `rusb` which wraps
libusb C).

**USB communication plan** (all via control transfers on endpoint 0 — the dongle exposes an
interrupt endpoint but it's a dummy):

#### TX (host -> dongle)

```
bmRequestType: USB_DIR_OUT | USB_TYPE_CLASS | USB_RECIP_INTERFACE  (0x21)
bRequest:      0x09
wValue:        <cleartext length>  (LE16)
wIndex:        0x0000
wLength:       <padded length>     (LE16)
data:          obfuscated + padded payload
```

Obfuscation algorithm (from ks959-sir.c):

```
lookup = "wangshuofei19710"
xor_mask = lookup[(cleartext_len & 0x0f) ^ 0x06] ^ 0x55
for each byte: output = input ^ xor_mask
padding: align to 8 bytes + 16 bytes overhead = ((len + 7) & ~7) + 0x10
```

Max cleartext per fragment: `(256 & ~7) - 16 = 240 bytes`. Fragment if larger.

#### RX (dongle -> host) — via polling

```
bmRequestType: USB_DIR_IN | USB_TYPE_CLASS | USB_RECIP_INTERFACE  (0xA1)
bRequest:      0x01
wValue:        0x0200
wIndex:        0x0000
wLength:       0x0800  (2048 byte buffer)
```

De-obfuscation:

```
rx_counter: u8, starts at 0, persists across ALL reads for the session
for each byte:
    rx_counter = rx_counter.wrapping_add(1)
    decoded = raw ^ rx_counter ^ 0x55
    if rx_counter == 0: skip this byte (garbage 0x95 after decode)
    else: emit decoded byte
```

#### Speed Change

```
bmRequestType: 0x21
bRequest:      0x09
wValue:        0x0200
wIndex:        0x0001
wLength:       0x0008
data:          [baudrate_le32, flags, 0, 0, 0]
```

Where `flags = 0x03` (8 data bits). Supported: 2400, 9600, 19200, 38400, 57600, 115200.

**Polling interval:** 10ms for RX (same as kernel driver's URB resubmission pattern).

#### Interface

```rust
pub struct KingsunDongle { /* nusb::Device, rx_counter, etc. */ }

impl KingsunDongle {
    pub fn open() -> Result<Self>;           // find+claim device
    pub fn set_speed(&mut self, baud: u32) -> Result<()>;
    pub fn send(&mut self, data: &[u8]) -> Result<()>;      // obfuscate+fragment+send
    pub fn poll_receive(&mut self) -> Result<Vec<u8>>;       // poll+deobfuscate
}
```

### 2. `sir_framing` — IrDA SIR Async Wrapper

Constants:

- `BOF = 0xC0`, `EOF = 0xC1`, `CE = 0x7D`, `IRDA_TRANS = 0x20`, `XBOF = 0xFF`
- `INIT_FCS = 0xFFFF`, `GOOD_FCS = 0xF0B8`
- CRC: CRC-CCITT (same polynomial as `crc_ccitt` in Linux, which is CRC-CCITT-FALSE / polynomial 0x1021)

#### Wrap (TX)

```
output = [XBOF * num_xbofs] [BOF] [stuff(payload_bytes)] [stuff(~FCS_lo)] [stuff(~FCS_hi)] [EOF]

stuff(byte):
  if byte in {BOF, EOF, CE}: emit CE, byte ^ 0x20
  else: emit byte
```

#### Unwrap (RX) — state machine

States: `OutsideFrame`, `BeginFrame`, `LinkEscape`, `InsideFrame`

Process each byte:

- `BOF` -> reset to BeginFrame, clear buffer, init FCS
- `EOF` -> check FCS == GOOD_FCS, if good emit frame (minus 2 CRC bytes), go to OutsideFrame
- `CE`  -> enter LinkEscape
- other -> if LinkEscape: byte ^= 0x20, go to InsideFrame; if InsideFrame: append, update FCS

#### Interface

```rust
pub fn wrap_frame(payload: &[u8], extra_bofs: usize) -> Vec<u8>;
pub struct Unwrapper { state, buffer, fcs }
impl Unwrapper {
    pub fn process_byte(&mut self, byte: u8) -> Option<Vec<u8>>;  // Some if complete frame
}
```

### 3. `pty_bridge` — PTY Creation and Termios Monitoring

#### PTY creation

Use `nix::pty::openpty()` to create master/slave pair. Symlink slave path to `/tmp/cressi-irda`.
Set master to non-blocking. Configure slave with initial raw mode settings.

#### Termios / baud rate detection — polling approach

**TIOCPKT was evaluated and rejected:** Despite documentation suggesting `TIOCPKT_IOCTL` would
fire on termios changes, the Linux kernel's `pty_set_termios()` only sets `TIOCPKT_IOCTL` when
`EXTPROC` is toggled or IXON/IXOFF flow control changes — NOT for plain `tcsetattr()` baud rate
changes. Verified empirically on Linux 6.x.

**Actual approach:** On Linux, PTY master and slave share the same termios struct in the kernel.
When the slave calls `tcsetattr()`, the change is immediately visible via `tcgetattr()` on the
slave fd (which we keep open). We call `check_baud_rate_change()` on every PTY→USB data path —
this reads `tcgetattr()` and compares against the last known baud rate. The overhead is negligible
since the baud rate changes exactly once per session (when libdivecomputer calls `configure()` at
session start).

#### Interface

```rust
pub struct PtyBridge {
    master_fd: OwnedFd,
    slave_path: PathBuf,
}

impl PtyBridge {
    pub fn new(symlink: &Path) -> Result<Self>;  // openpty + symlink + TIOCPKT
    pub fn read(&mut self, buf: &mut [u8]) -> Result<PtyEvent>;
    pub fn write(&mut self, data: &[u8]) -> Result<usize>;
    pub fn slave_path(&self) -> &Path;
}

pub enum PtyEvent {
    Data(usize),         // n bytes written into buf
    TermiosChanged,      // need to call get_slave_speed()
    WouldBlock,
}
```

### 4. `main.rs` — Event Loop

Single-threaded, poll-based loop (no need for async runtime complexity):

```
1. Open Kingsun dongle (usb_dongle::KingsunDongle::open)
2. Set initial speed to 9600 (IrDA SIR default)
3. Create PTY bridge (pty_bridge::PtyBridge::new)
4. Print slave path for user
5. Loop:
   a. poll(master_fd, usb_fd, 10ms timeout)
   b. If master_fd readable:
      - Read from PTY master (handle TIOCPKT status byte)
      - If TermiosChanged: read slave baud rate, call dongle.set_speed()
      - If Data: send to dongle (optionally via SIR framing)
   c. Every 10ms (or when USB has data):
      - Poll dongle for received data
      - If data: (optionally SIR-unwrap), write to PTY master
   d. Handle SIGINT for clean shutdown
```

Note: `nusb` is async (tokio-based). We'll use a minimal tokio runtime just for USB I/O,
or use `nusb`'s blocking wrappers if available. If blocking isn't available, we use
`futures_lite::future::block_on` for individual USB operations within the poll loop.

Actually — reconsidering: since `nusb` requires an async executor, and we need to poll
both the PTY fd and USB simultaneously, we'll use `tokio` with:

- PTY I/O via `tokio::io::AsyncFd` wrapping the master fd
- USB I/O via `nusb` async API
- A `tokio::select!` loop

This is cleaner than mixing blocking and non-blocking I/O.

## Baud Rate Strategy

1. The dongle starts at 9600 (IrDA SIR default).
2. libdivecomputer's `cressi_goa.c` calls `dc_iostream_configure(115200, 8N1)` immediately
   on open. This results in `tcsetattr()` on the PTY slave.
3. Our TIOCPKT handler detects the termios change, reads the new baud rate (115200).
4. We issue a USB speed change control transfer to the dongle.
5. The dongle reconfigures its IR link to 115200.

This means there's a brief window where the dongle is at 9600 and the application thinks it's
at 115200. In practice this is fine because libdivecomputer doesn't send data until after
`configure()` returns, and we process the baud rate change synchronously before returning
data availability to the PTY.

## Error Handling

- `thiserror` for typed errors in each module
- `anyhow` in `main.rs` for top-level error chaining
- No `unwrap()` on the main data path; only in one-time setup where failure is fatal

## Logging

`tracing` with `tracing-subscriber` (fmt layer):

- `TRACE`: every USB control transfer (hex dump), every PTY read/write
- `DEBUG`: SIR frame boundaries, baud rate changes, dongle poll results
- `INFO`:  dongle found, PTY created, speed changes
- `WARN`:  SIR CRC errors, USB retry events
- `ERROR`: fatal USB/PTY errors

Default level: `INFO`. Set `RUST_LOG=trace` for full byte-level debugging.

## Test Plan

### Unit Tests (no hardware)

1. **SIR framing round-trip:** wrap arbitrary payloads, verify BOF/EOF/CRC present; unwrap
   and verify identical payload recovered. Test edge cases: payload containing BOF/EOF/CE bytes.
2. **Kingsun obfuscation round-trip:** obfuscate test data, verify padding and XOR; check
   against known test vectors derived from the kernel driver.
3. **RX de-obfuscation:** simulate multi-read session with counter persistence; verify garbage
   byte at counter=0xFF is skipped.
4. **CRC-CCITT:** verify against known CRC values.

### Integration Tests (with hardware)

1. **USB enumeration:** Open dongle, verify VID/PID, claim interface.
2. **Speed change:** Send speed change control transfer, verify no USB error.
3. **TX loopback:** Send known data, verify control transfer succeeds.
4. **RX polling:** Poll dongle repeatedly, verify we get empty responses (no IR source).
5. **PTY bridge:** Open slave with `minicom`, type characters, verify they reach `usb_dongle::send`.
6. **End-to-end:** Run Subsurface against `/tmp/cressi-irda` with Donatello positioned near dongle.

### Hardware test sequence

1. Validate USB communication (open, control transfers, bulk data)
2. Validate SIR framing round-trip or raw passthrough
3. Validate PTY bridge in isolation (cat/minicom)
4. End-to-end with Subsurface + Donatello

## Dependencies

```toml
[dependencies]
nusb = "0.1"              # async USB
tokio = { version = "1", features = ["rt", "macros", "io-util", "net", "signal"] }
nix = { version = "0.29", features = ["pty", "term", "fs"] }
crc = "3"                 # CRC-CCITT
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2"
anyhow = "1"
```

## Known Risks

1. **SIR framing uncertainty:** We believe the Donatello expects raw bytes (no SIR framing),
   based on the ESP32+TFDU4101 evidence. If this is wrong, enabling `--sir-framing` should fix it.
   First hardware test will settle this quickly.

2. **KS-959 firmware may apply SIR framing or require IrLAP commands:** The dongle firmware
   is opaque. It might apply BOF/EOF/byte-stuffing automatically at the hardware level, or
   it might require IrLAP-level commands before it will actually transmit. The kernel driver
   (`ks959-sir.c`) necessarily documents whatever sequence is required — reading it fully is
   the mitigation. If the firmware requires IrLAP, we'd need to either implement minimal
   IrLAP framing or switch to a different hardware approach (ESP32+TFDU4101).

3. **Kingsun RX counter state:** The rx_variable_xormask persists across the entire session.
   If we miss bytes (USB error, buffer overflow), the counter gets out of sync and all
   subsequent decoding is garbage. Mitigation: log the counter value, detect patterns that
   suggest desync (e.g., no valid SIR frames for N polls), reset the dongle.

4. **TIOCPKT behavior:** TIOCPKT is well-supported on Linux but has edge cases — the status
   byte is only prepended to `read()` calls, not `readv()`. We must always use `read()`.

5. **libdivecomputer ioctl tolerance:** libdivecomputer's `ENABLE_PTY` mode tolerates EINVAL/
   ENOTTY from ioctls like TIOCEXCL, TIOCMBIS (DTR/RTS), TIOCGSERIAL. Our PTY naturally
   returns these errors. TIOCINQ (FIONREAD) works on PTY masters. This should be fine but
   needs verification — if Subsurface is built without `ENABLE_PTY`, those ioctls will cause
   hard errors.

6. **check_ctrlrecip blocker:** The speed change control transfer uses `wIndex=0x0001` which
   the Linux kernel's `usbfs` rejects (see KNOWLEDGE.md for full analysis). Current mitigation:
   try `bRequestType=0x41` (Vendor+Interface) which bypasses kernel validation. If that also
   fails, fallback options are a minimal kernel module, an eBPF hook, or avoiding the speed
   change entirely.
