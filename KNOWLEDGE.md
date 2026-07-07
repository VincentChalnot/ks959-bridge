# irda2tty — Complete Knowledge Base

Everything learned while building a userspace IrDA SIR driver for the Kingsun KS-959
USB dongle, bridging it to a PTY for libdivecomputer/Subsurface to talk to a Cressi
Donatello dive computer.

## Table of Contents

1. [Why This Exists](#why-this-exists)
2. [Hardware: Kingsun KS-959 Dongle](#hardware-kingsun-ks-959-dongle)
3. [USB Protocol (Reverse-Engineered)](#usb-protocol-reverse-engineered)
4. [The usbfs `check_ctrlrecip` Problem (UNSOLVED)](#the-usbfs-check_ctrlrecip-problem)
5. [IrDA SIR Framing Layer](#irda-sir-framing-layer)
6. [Target Device: Cressi Donatello](#target-device-cressi-donatello)
7. [dctool Quick Reference](#dctool-quick-reference)
8. [PTY Bridge](#pty-bridge)
9. [Application Architecture](#application-architecture)
10. [What Works, What Doesn't](#what-works-what-doesnt)
11. [VM/USB Passthrough Setup](#vmusb-passthrough-setup-for-kernel-irda-testing)
12. [Alternative Hardware Approaches](#alternative-hardware-approaches)
13. [External References](#external-references)
14. [Key Files](#key-files)

---

## Why This Exists

Linux removed the entire IrDA subsystem in kernel 4.17 (2018). The Kingsun KS-959 dongle
had a kernel driver (`ks959-sir.c`) that lived inside that subsystem. On any modern kernel
(the dev machine runs **Fedora 43, kernel 7.0.14**), the dongle is a dead USB device with
no driver. This program replaces the kernel driver entirely in userspace, exposing a PTY
that looks like a serial port to applications.

IrDA was also removed from Windows, making this class of problem universal. An alternative
approach using out-of-tree kernel modules exists (`github.com/cschramm/irda` — builds the
IrDA stack as DKMS modules for modern kernels), but even with those modules the Donatello
won't work because it doesn't complete the IrLAP connection handshake (see DESIGN.md).

Linus Torvalds on why the IrDA stack was removed:

> "There are multiple 'levels' of the IrDA stack... IrDA SIR is basically just half
> duplex rs232 over infrared... Then there's 'IrLAP' on top of that, which is a
> device discovery, and connection establishment, and reliable data link layer."

> "The whole IrDA thing is a much more complex mess that tries to support multiple
> concurrent devices, and is why Linux dropped support for it."

> "I also suspect the Linux code was written based off a spec, rather than a sane
> technical understanding of what the implementation was meant to be."

The Linux IrDA stack implements the full spec. The Cressi dive computer implements only
the physical layer + minimal discovery. The stack's complexity is why it was removed from
the kernel, and why it doesn't work with simple devices.

---

## Hardware: Kingsun KS-959 Dongle

### USB Descriptor (from `lsusb -v`)

```
Bus 003 Device 003: ID 07d0:4959 Dazzle Kingsun KS-959 Infrared Adapter
Negotiated speed: Low Speed (1Mbps)
  bcdUSB               1.10
  bMaxPacketSize0         8          ← LOW-SPEED: 8 bytes max per control packet
  bNumConfigurations      1
  Configuration 1:
    bNumInterfaces          1
    Interface 0:
      bInterfaceClass       255      ← Vendor Specific (not HID)
      bNumEndpoints           1
      Endpoint 0x81:
        EP 1 IN, Interrupt, wMaxPacketSize 8, bInterval 1
```

**Critical facts:**
- **Low-speed USB device** — max packet size 8 bytes for control transfers
- **Only 1 interface** (interface 0) — this matters for the usbfs bug (see below)
- **Interrupt endpoint 0x81 is a DUMMY** — never used for actual data
- **ALL communication uses control transfers on endpoint 0** — TX, RX, and speed change

### Physical

- VID `0x07D0`, PID `0x4959`
- IrDA SIR physical layer (infrared LED/photodiode)
- SIR modulation (NRZ UART ↔ IrDA 3/16 pulses) handled in **hardware**
- The SIR framing (BOF/EOF/escape/CRC) was handled by the **kernel driver**, NOT the dongle

### IrDA SIR vs TV Remote IR

IrDA SIR (Serial Infrared) is NOT the same as 38kHz TV remote IR. This is why components
like TSOP1738 or the Flipper Zero's TSOP75338TR cannot interact with IrDA devices:

| Aspect | IrDA SIR | TV Remote (TSOP1738) |
|--------|----------|---------------------|
| Carrier | Pulse at 3/16 of bit period | 38kHz continuous |
| Pulse width (9600 baud) | ~19.5µs | ~263µs min burst |
| Pulse width (115200 baud) | ~1.6µs | N/A |
| Framing | UART (start+8data+stop) | Burst coding |
| Component | IrDA transceiver (TFDU4101) | TSOP1738 module |

A TSOP1738 cannot detect IrDA SIR pulses — the pulse width is far below its minimum burst
detection threshold. IrDA requires a dedicated transceiver chip (TFDU4101) or a
microcontroller with hardware IrDA mode (ESP32's `UART_IRDA_EN` register bit).

---

## USB Protocol (Reverse-Engineered)

Source: `reference/ks959-sir.c` — reverse-engineered by Alex Villacís Lasso in 2007 by
sniffing WinXP driver USB traffic with USBSnoopy. The designer of the dongle firmware
used **obfuscation** on the USB traffic with his own name as the XOR key ("wangshuofei19710").

### TX: Host → Dongle (data to transmit over IR)

```
bmRequestType: 0x21  (OUT | Class | Interface)
bRequest:      0x09
wValue:        <cleartext length>   (u16 LE)
wIndex:        0x0000
wLength:       <padded length>      (u16 LE)
data:          obfuscated + padded payload
```

#### TX Obfuscation Algorithm

```
LOOKUP = "wangshuofei19710"   (16 bytes, the firmware author's name + digits)

padded_len = ((cleartext_len + 7) & ~7) + 16
xor_mask   = LOOKUP[(cleartext_len & 0x0F) ^ 0x06] ^ 0x55

for each cleartext byte:
    obfuscated[i] = cleartext[i] ^ xor_mask

remaining bytes (padding) = 0x00
```

**Fragmentation:** Max 240 cleartext bytes per transfer. Formula:
`(256 & ~7) - 16 = 240`. Larger payloads must be fragmented into multiple control
transfers, each independently obfuscated.

#### TX Test Vector (hand-calculated)

```
cleartext = [0x01, 0x02, 0x03], len = 3
padded_len = ((3+7) & ~7) + 16 = 24
mask_index = (3 & 0x0F) ^ 0x06 = 5
LOOKUP[5]  = 'h' = 0x68
xor_mask   = 0x68 ^ 0x55 = 0x3D
obfuscated = [0x3C, 0x3F, 0x3E, 0x00 × 21]
```

### RX: Dongle → Host (data received from IR) — Polling

```
bmRequestType: 0xA1  (IN | Class | Interface)
bRequest:      0x01
wValue:        0x0200
wIndex:        0x0000
wLength:       0x0800  (2048 bytes — max buffer)
```

Returns raw bytes in the response data. Empty response = no data available.
Poll interval: **10ms** (matches kernel driver's URB resubmission cadence).

#### RX De-obfuscation Algorithm

```
rx_counter: u8   — starts at 0, persists across ALL reads for the entire session
                   (NOT reset between polls or frames)

for each raw byte received:
    rx_counter = rx_counter.wrapping_add(1)    // increment first
    decoded = raw_byte ^ rx_counter ^ 0x55
    if rx_counter == 0:
        skip this byte                         // garbage byte every 255 real bytes
    else:
        emit decoded byte
```

**The garbage byte:** Every time `rx_counter` wraps from 0xFF to 0x00, the byte at that
position decodes to 0x95 and must be discarded. This is every 255th real byte. The counter
is **never reset** — it persists for the entire USB session.

**Desync risk:** If any RX bytes are lost (USB error, buffer overflow), the counter gets
permanently out of sync and all subsequent decoding produces garbage. Recovery requires
resetting the dongle (USB re-enumeration).

### Speed Change

```
bmRequestType: 0x21  (OUT | Class | Interface)
bRequest:      0x09
wValue:        0x0200
wIndex:        0x0001       ← THIS IS THE PROBLEM (see usbfs section)
wLength:       0x0008
data:          8-byte struct (see below)
```

#### Speed Payload (8 bytes, packed, little-endian)

```
struct ks959_speedparams {
    __le32 baudrate;     // e.g., 115200 = [0x00, 0xC2, 0x01, 0x00]
    __u8   flags;        // 0x03 = 8 data bits (KS_DATA_8_BITS)
    __u8   reserved[3];  // zeroes
};
```

Flags field bits:
```
bits [1:0] = data bits    (0x00=5, 0x01=6, 0x02=7, 0x03=8)
bit  [3]   = stop bits    (0x00=1, 0x08=2)
bit  [4]   = parity en    (0x00=disable, 0x10=enable)
bit  [5]   = parity type  (0x00=even if enabled, 0x20=odd)
bit  [7]   = reset         (0x80)
```

**Supported baud rates:** 2400, 9600, 19200, 38400, 57600, 115200.
The kernel driver officially supports up to 57600; 115200 TX works but RX shows
corruption with some phones. The Cressi Donatello needs 115200.

---

## The usbfs `check_ctrlrecip` Problem

**THIS IS THE MAIN UNSOLVED BLOCKER.** The speed change control transfer cannot be
sent from userspace on modern Linux kernels.

### Root Cause

The speed change uses `wIndex=0x0001` with `USB_RECIP_INTERFACE` (bRequestType=0x21).
In the dongle's protocol, `wIndex=1` is a flag meaning "this is a speed change" (vs
`wIndex=0` for data). It is NOT a USB interface number.

But the Linux kernel's `usbfs` (the interface userspace programs use to talk to USB
devices) validates control transfers in `check_ctrlrecip()` (`drivers/usb/core/devio.c`):

```c
case USB_RECIP_INTERFACE:
    ret = findintfep(ps->dev, index);    // look for endpoint with address == wIndex
    if (ret < 0)
        ret = checkintf(ps, index);      // check if interface wIndex is claimed
    break;
```

For `wIndex=1`:
1. `findintfep(dev, 1)` → searches for endpoint address 0x01. The device only has
   endpoint 0x81 (IN). 0x81 ≠ 0x01 → returns `-ENOENT`.
2. `checkintf(ps, 1)` → checks if interface 1 is claimed. The device only has
   interface 0 → `usb_ifnum_to_if(dev, 1)` returns NULL → returns `-ENOENT`.
3. Kernel rejects the URB submission with `-ENOENT`.
4. nusb maps `ENOENT` to `TransferError::Cancelled` ("transfer was cancelled").

The original kernel driver (`ks959-sir.c`) bypasses this because kernel drivers call
`usb_submit_urb()` directly — they don't go through `usbfs` and its `check_ctrlrecip`.

### What Was Tried

| Attempt | bRequestType | Kernel | Dongle | Result |
|---------|-------------|--------|--------|--------|
| Original: `Class + Interface` | `0x21` | **REJECTS** (ENOENT → "cancelled") | Would accept | Kernel blocks it |
| `Class + Device` | `0x20` | Passes (no interface check for Device recipient) | **STALL** | Dongle checks recipient bits |
| `Vendor + Interface` | `0x41` | Passes (vendor-type early return in check_ctrlrecip) | **UNTESTED** | In code now, needs hardware test |

### Why Each Bypass Fails or Might Work

- **`Recipient::Device` (0x20):** The kernel's `check_ctrlrecip` switch only handles
  `USB_RECIP_ENDPOINT` and `USB_RECIP_INTERFACE`. `USB_RECIP_DEVICE` falls through with
  `ret=0` (success). But the **dongle firmware STALL'd** — it checks the recipient bits
  of bRequestType and requires `USB_RECIP_INTERFACE` (0x01).

- **`ControlType::Vendor` (0x41):** The kernel has an early return:
  `if (USB_TYPE_VENDOR == (USB_TYPE_MASK & requesttype)) return 0;` which bypasses ALL
  validation. This changes bRequestType from `0x21` (Class+Interface) to `0x41`
  (Vendor+Interface). The question is whether the dongle firmware checks the type
  field (bits 6:5). Since it STALL'd with `0x20` (same type, different recipient),
  we know it checks at least the recipient. If it dispatches on recipient bits and
  ignores type bits, `0x41` would work.

- **`Recipient::Other` (0x23):** Also falls through the kernel's switch (no case for
  `USB_RECIP_OTHER`). But since the dongle rejected `0x20` (wrong recipient), it would
  likely also reject `0x23` (also wrong recipient).

### Possible Solutions (Not Yet Implemented)

1. **If `0x41` works:** Problem solved. The `Vendor + Interface` combination bypasses
   the kernel check while keeping the recipient bits the dongle wants.

2. **If `0x41` also STALL's** (firmware checks full bRequestType == 0x21):
   - **Write a minimal kernel module** that does only the speed change via
     `usb_control_msg()` and exposes a sysfs/ioctl interface
   - **eBPF hook** on `check_ctrlrecip` to return 0 for this specific VID/PID
   - **Avoid speed changes entirely:** If you can ensure the dongle is already at
     the right speed (e.g., if a kernel driver previously set it), skip the speed
     change. The kernel driver sets 9600 on probe; if Donatello needs 115200,
     you're stuck.
   - **Patch libdivecomputer** to start at 9600 instead of 115200 — impractical
     since the Donatello firmware runs at 115200.

### Key Kernel Code Path

```
Userspace: interface.control_out(...)
  → nusb: USBDEVFS_SUBMITURB ioctl (or USBDEVFS_CONTROL for blocking)
    → kernel: proc_do_submiturb() / proc_control()
      → check_ctrlrecip(ps, bRequestType=0x21, bRequest=0x09, wIndex=0x0001)
        → USB_RECIP_INTERFACE case
          → findintfep(dev, 1) → -ENOENT (no endpoint addr 1)
          → checkintf(ps, 1) → claimintf(ps, 1) → usb_ifnum_to_if(dev, 1) → NULL → -ENOENT
        → return -ENOENT
      → URB submission rejected
    → nusb: errno_to_transfer_error(ENOENT) → TransferError::Cancelled
```

This affects ALL userspace USB libraries (nusb, rusb/libusb, pyusb) — they all go
through usbfs. Only in-kernel drivers can bypass this.

---

## IrDA SIR Framing Layer

### Important: The Donatello Does NOT Use SIR Framing

The Cressi Donatello uses `DC_TRANSPORT_SERIAL` in libdivecomputer, NOT
`DC_TRANSPORT_IRDA`. It sends **raw serial bytes** over the IrDA SIR physical layer.
No BOF/EOF/escape/CRC wrapping. The default mode is **raw passthrough** (`--sir-framing`
is off by default).

Evidence:
- libdivecomputer registers the Donatello as `DC_TRANSPORT_SERIAL`
- Users communicate with it using ESP32 + TFDU4101 (physical-layer-only transceiver)
- The SIR wrapping in `ks959-sir.c` was part of the kernel's IrDA protocol stack,
  not needed by every IrDA device

The `sir_framing` module exists for devices that DO use the full IrDA protocol.

### SIR Frame Format

```
[XBOF × N] [BOF] [stuffed_payload] [stuffed_FCS_lo] [stuffed_FCS_hi] [EOF]
```

Constants: `BOF=0xC0`, `EOF=0xC1`, `CE=0x7D`, `XBOF=0xFF`, `IRDA_TRANS=0x20`

### Byte Stuffing (Transparency)

If byte ∈ {0xC0, 0xC1, 0x7D}: emit `[CE, byte ^ 0x20]`
Otherwise: emit byte as-is.

### CRC: Reflected CRC-CCITT

- **Polynomial:** `0x8408` (bit-reversal of 0x1021)
- **NOT** the MSB-first 0x1021 — this is the **reflected/LSB-first** variant
- Init: `0xFFFF`, no final XOR
- Table entry[1] = `0x1189` (use this to verify your table)
- Check value for "123456789" = **`0x6F91`**
- Good FCS residue: **`0xF0B8`** (CRC over payload + appended ~CRC_LE)
- FCS appended to frame: `~CRC` in little-endian byte order

This matches the Linux kernel's `lib/crc-ccitt.c` / `crc_ccitt_byte()` exactly. It does
NOT match the `crc` Rust crate's `CRC_16_IBM_SDLC` preset — we compute the table at
compile time with `const fn`.

### Unwrapper State Machine

States: `OutsideFrame`, `BeginFrame`, `InsideFrame`, `LinkEscape`

Dispatch on **byte class first, then state** (mirrors kernel's `async_unwrap_char()`):

```
BOF received:
  - OutsideFrame/BeginFrame: reset buffer, init FCS, → BeginFrame
  - InsideFrame/LinkEscape: warn (BOF inside frame), reset → BeginFrame

EOF received:
  - OutsideFrame: warn, ignore
  - BeginFrame/InsideFrame/LinkEscape: check FCS == 0xF0B8, emit payload if good

CE received:
  - OutsideFrame: ignore (noise)
  - LinkEscape: warn (double CE)
  - BeginFrame/InsideFrame: → LinkEscape

Other byte:
  - OutsideFrame: ignore (XBOF or noise)
  - BeginFrame: append, update FCS, → InsideFrame
  - InsideFrame: append, update FCS
  - LinkEscape: unstuff (byte ^ 0x20), append, update FCS, → InsideFrame
```

---

## Target Device: Cressi Donatello

Source: `reference/libdivecomputer/src/cressi_goa.c`

### Physical

- Wrist-mount dive computer
- Communicates via **IrDA SIR** (Serial Infrared) at 115200 baud
- IR port location: small hole on the side of the watch case
- PC Link mode: activated by a specific button press (varies by firmware)
- Once in PC mode, responds to IrDA XID discovery with device name `cressi` and hint bits
  `0x0400` (Computer) or `0x8404` (Computer IrCOMM)
- Does NOT complete IrLAP connection handshake (SNRM/UA) — see DESIGN.md for proof
- Uses `DC_FAMILY_CRESSI_GOA` protocol (model=4) in libdivecomputer
- Also supports BLE transport (`DC_TRANSPORT_BLE`)

### Serial Configuration

```
115200 baud, 8 data bits, No parity, 1 stop bit, No flow control
DTR cleared, RTS cleared
Timeout: 3000ms (serial), 5000ms (BLE)
100ms delay before each command
```

### Packet Format (Serial, NOT BLE)

```
TX: [AA AA AA] [len] [cmd] [data...] [CRC16_LE] [55]
     header     size   cmd   payload    checksum  trailer
```

- Header: 3× `0xAA`
- len: size of data (not including header, cmd, CRC, trailer)
- cmd: command byte
- CRC: `checksum_crc16_ccitt(packet+3, size+2, 0x000, 0x0000)` — over len+cmd+data
- CRC appended little-endian
- Trailer: `0x55`
- Max payload: 12 bytes (`SZ_PACKET`)

### Commands

| Command | Code | Description |
|---------|------|-------------|
| `CMD_VERSION` | `0x00` | Get device version/ID |
| `CMD_SET_TIME` | `0x13` | Set device time |
| `CMD_EXIT_PCLINK` | `0x1D` | Exit PC-link mode |
| `CMD_LOGBOOK` | `0x21` | Download logbook (firmware < 200, 23-byte entries) |
| `CMD_DIVE` | `0x22` | Download dive data |
| `CMD_LOGBOOK_V4` | `0x23` | Logbook (firmware >= 200, 15-byte entries) |

### Protocol Constants

```c
HEADER   = 0xAA    // 3-byte frame header
TRAILER  = 0x55    // 1-byte frame trailer
END      = 0x04    // end-of-transmission
ACK      = 0x06    // acknowledgement
SZ_DATA  = 512     // max data bytes per transfer packet
SZ_PACKET = 12     // max payload bytes per command packet
FP_SIZE  = 6       // fingerprint size
```

### CMD_VERSION Response Format

The first response to CMD_VERSION contains an 11-byte device ID:

| Offset | Size | Field |
|--------|------|-------|
| 0–3 | 4 | Serial number (uint32_le) |
| 4 | 1 | Model number |
| 5–6 | 2 | Firmware version (uint16_le) |
| 7–8 | 2 | Reserved/padding |
| 9–10 | 2 | Data format version (uint16_le) — only if id_size == 11 |

### Firmware Version → Logbook Format

```c
if (firmware >= 200) → CMD_LOGBOOK_V4 (0x23), logbook entry = 15 bytes
else                  → CMD_LOGBOOK (0x21),    logbook entry = 23 bytes
```

### Download Protocol (per dive)

1. Host sends command packet (CMD_DIVE + 2-byte dive number)
2. Device sends response packet (small, contains total size)
3. Device sends data packets (512 bytes each + 3-byte header + 2-byte CRC)
4. Host sends ACK (0x06) after each data packet
5. Device sends END (0x04) when done
6. Host sends final ACK

### BLE Variant

The Donatello also supports BLE. Different commands are used:
- `CMD_LOGBOOK_BLE` = 0x02
- `CMD_DIVE_BLE` = 0x03
- BLE characteristics: Nordic UART Service (`6E400003/04/05-B5A3-...`)
- No CMD_VERSION over BLE; version info read from characteristics instead

### Wake-Up Command

From hb9eue's ESP32 code, the Leonardo uses an ASCII wake-up command:

```
{123DBA}  →  7B 31 32 33 44 42 41 7D
```

Response example: `{!D5B3}`

The Goa/Donatello may NOT use this wake-up — libdivecomputer sends CMD_VERSION (0x00)
as the first command after opening the serial port. The wake-up is specific to the
Leonardo (`cressi_leonardo.c`).

### Other Cressi Models

| Model | Family | Notes |
|-------|--------|-------|
| Donatello | `DC_FAMILY_CRESSI_GOA` (model 4) | This project's target |
| Giotto | `DC_FAMILY_CRESSI_GOA` | Same family, different model number |
| Newton, Drake, Cartesio, Goa, Neon, Nepto | `DC_FAMILY_CRESSI_GOA` or `DC_FAMILY_CRESSI_EDY` | Same protocol family |
| Leonardo | `DC_FAMILY_CRESSI_LEONARDO` | Requires DTR/RTS toggles for original cradle |
| Edy | `DC_FAMILY_CRESSI_EDY` | Separate family |

### Communication Flow

1. Open serial port
2. Configure 115200 8N1, clear DTR/RTS
3. Sleep 100ms, purge buffers
4. Send `CMD_VERSION` → get device ID
5. Send `CMD_LOGBOOK` → download logbook entries
6. For each dive: send `CMD_DIVE` → download dive profile
7. Send `CMD_EXIT_PCLINK` → release device

---

## dctool Quick Reference

Source: libdivecomputer examples (`reference/libdivecomputer/examples/`)

### Download Dives

```bash
dctool -v -l cressi.log -f goa -m 4 download -o dives.xml /dev/ircomm0
```

Flags:
- `-v` — verbose (shows INFO/ERROR logs)
- `-l cressi.log` — log file
- `-f goa` — device family (`DC_FAMILY_CRESSI_GOA`)
- `-m 4` — device model number (Donatello=4)
- `-o dives.xml` — output file (XML format, default)

### Other Commands

```bash
dctool list                                    # list supported devices
dctool version                                 # show version
dctool help download                           # help for download command
dctool -f goa -m 4 dump -o dump.bin /dev/ircomm0   # raw memory dump
dctool parse -i dives.xml                      # parse previously downloaded dives
```

### Output Formats

- **XML** (default): all dives in one file
- **RAW** (`-f raw`): each dive as separate binary file
  - Template placeholders: `%n` (4-digit number), `%f` (fingerprint hex), `%t` (timestamp)

### Family Name Mapping

The `-f` flag takes a string name (not the full constant):

| String | Constant | Default Model |
|--------|----------|---------------|
| `goa` | `DC_FAMILY_CRESSI_GOA` | 2 |
| `leonardo` | `DC_FAMILY_CRESSI_LEONARDO` | 1 |
| `edy` | `DC_FAMILY_CRESSI_EDY` | 0x08 |

The `-m` flag overrides the default model number. For the Donatello: `-f goa -m 4`.

### Source Code Locations

| File | Description |
|------|-------------|
| `src/cressi_goa.c` | Goa/Donatello driver |
| `src/cressi_goa_parser.c` | Dive data parser |
| `src/cressi_goa.h` | Header |
| `src/descriptor.c` | Device descriptor table |
| `examples/common.c` | Family name mapping |

---

## PTY Bridge

### Creation

- `nix::pty::openpty()` creates master/slave pair
- Master fd set to `O_NONBLOCK`
- Slave symlinked to user path (default `/tmp/cressi-irda`)
- Slave fd kept open for `tcgetattr()` baud rate polling

### Baud Rate Detection

**TIOCPKT was evaluated and rejected.** Despite documentation, the Linux kernel's
`pty_set_termios()` only sets `TIOCPKT_IOCTL` when `EXTPROC` is toggled or
IXON/IXOFF flow control changes — **NOT for `tcsetattr()` baud rate changes**.
Verified empirically on Linux 6.x.

**Actual approach:** PTY master and slave share the same `termios` struct in the kernel.
`tcsetattr()` on the slave is immediately visible via `tcgetattr()` on the slave fd
(which we keep open). We call `check_baud_rate_change()` on every PTY→USB data path.
Overhead is negligible — baud rate changes once per session.

### nix Crate Gotchas

- **nix 0.29 has no `pty` feature** — PTY functions (`openpty`, `ttyname`) live under
  the `term` feature flag. Use `features = ["term", "ioctl", "fs"]`.
- **BaudRate → u32 conversion:** On Linux, `nix::sys::termios::BaudRate` variants are
  opaque enum constants (e.g., `B9600 = 0x0D`), NOT the raw speed value. You must use
  an explicit `match` mapping. On BSDs they ARE the raw values — don't assume portability.

### libdivecomputer Compatibility

libdivecomputer's `serial_posix.c` with `ENABLE_PTY` tolerates `EINVAL`/`ENOTTY` from:
- `TIOCEXCL` (exclusive access)
- `TIOCMBIS` (DTR/RTS control)
- `TIOCGSERIAL` (serial port info)

PTYs naturally return these errors. Subsurface must be built with `ENABLE_PTY` for this
to work. Without it, those ioctl failures are hard errors.

---

## Application Architecture

### Module Structure

```
src/
  main.rs          — tokio event loop, CLI (clap), signal handling
  usb_dongle.rs    — Kingsun KS-959 protocol (obfuscation, speed, fragmentation)
  sir_framing.rs   — IrDA SIR wrap/unwrap (optional, off by default)
  pty_bridge.rs    — PTY pair, symlink, baud rate polling
```

### Event Loop (main.rs)

Single-threaded `tokio` runtime with `select!` over three sources:

1. **PTY master readable** (`AsyncFd<RawFd>`) — data from Subsurface
   - Check for baud rate change via `tcgetattr()` polling
   - Forward data to dongle (optionally SIR-wrapped)

2. **USB RX poll timer** (10ms `tokio::time::interval`) — data from IR
   - Poll dongle for received bytes
   - Deobfuscate, optionally SIR-unwrap
   - Write to PTY master

3. **Signal handler** (SIGINT/SIGTERM) — clean shutdown

### Dependencies

```toml
nusb = "0.1"           # async USB (pure Rust, no libusb C dependency)
tokio = "1"            # async runtime (current_thread flavor)
nix = "0.29"           # PTY, termios, fcntl (features: term, ioctl, fs)
libc = "0.2"           # raw ioctl for TIOCPKT (ended up unused)
clap = "4"             # CLI parsing (derive feature)
tracing = "0.1"        # structured logging
tracing-subscriber = "0.3"  # log output (env-filter feature)
thiserror = "2"        # typed errors per module
anyhow = "1"           # top-level error chaining in main
futures-lite = "2"     # required by nusb
crc = "3"              # in Cargo.toml but NOT USED — we compute CRC at compile time
```

### Error Handling

- `thiserror` enums in each module (`DongleError`, `PtyError`)
- `anyhow` in `main.rs` for `.context()` chaining
- No `unwrap()` on the data path — only in one-time setup where failure is fatal
- EAGAIN/EWOULDBLOCK from PTY reads is normal → clear readiness, retry

---

## What Works, What Doesn't

### Working (45/45 tests pass, 0 warnings, compiles in release)

| Component | Tests | Status |
|-----------|-------|--------|
| SIR framing (CRC, wrap, unwrap, state machine) | 24 | All pass |
| USB obfuscation/deobfuscation/fragmentation | 14 | All pass, verified with hand-calculated vectors |
| PTY bridge (create, symlink, read/write, baud detection) | 7 | All pass |
| Main event loop | — | Compiles, CLI works |
| USB device enumeration + interface claim | — | Works on hardware |

### NOT Working

| Component | Problem | Status |
|-----------|---------|--------|
| **Speed change** | `check_ctrlrecip` rejects bRequestType=0x21 with wIndex=1 | **BLOCKER** |
| | bRequestType=0x20 (Class+Device): kernel passes, dongle STALL's | Failed |
| | bRequestType=0x41 (Vendor+Interface): kernel passes, dongle untested | **Current code** |
| End-to-end with Subsurface | Blocked by speed change | Not tested |

### Untested

- USB TX (sending obfuscated data to dongle)
- USB RX (polling dongle for received data)
- Full PTY↔USB bridge with minicom
- End-to-end with Subsurface + Donatello

---

## VM/USB Passthrough Setup (for kernel IrDA testing)

This project uses a VM only for testing the kernel IrDA stack (to confirm the IrLAP dead
end). The production irda2tty tool runs on the host directly.

### Environment

- Host: Linux (Fedora/RHEL-based, kernel 6.x), QEMU/KVM via libvirt
- VM: Ubuntu 18.04, kernel 4.15 (last version with staging IrDA drivers)
- VM IP: 192.168.122.26

Ubuntu 18.04 was chosen because its kernel 4.15 still has the IrDA subsystem. Debian 9
(also kernel 4.x) was tried first but its archived mirrors broke the installer.

### USB Passthrough

```bash
# Find device on host
lsusb | grep 07d0
# Bus 005 Device 010: ID 07d0:4959 Dazzle Kingsun KS-959 Infrared Adapter

# Create XML and attach to running VM
cat > /tmp/kingsun-usb.xml << 'EOF'
<hostdev mode='subsystem' type='usb'>
  <source>
    <vendor id='0x07d0'/>
    <product id='0x4959'/>
  </source>
</hostdev>
EOF
virsh -c qemu:///system attach-device ubuntu18.04 /tmp/kingsun-usb.xml --live
```

The KS-959 moves bus/device numbers on replug — re-run `virsh attach-device` after each
physical reconnect.

### Kernel Module Loading (in VM)

```bash
# irda and ks959_sir auto-load on USB detection
# ircomm and ircomm_tty need manual loading:
ssh root@192.168.122.26 "modprobe ircomm_tty"

# Verify:
ssh 192.168.122.26 "lsmod | grep irda"
# Expected: irda, ks959_sir, ircomm, ircomm_tty

# Check /dev/ircomm0 exists:
ssh 192.168.122.26 "ls -la /dev/ircomm0"
# crw-rw---- 1 root dialout 161, 0 ... /dev/ircomm0

# Fix permissions:
ssh root@192.168.122.26 "chmod 666 /dev/ircomm0 && usermod -aG dialout vincent"
```

### USB Disconnect Issue

The KS-959 can disconnect from the VM under load:

```
usb 3-2: ks959_speed_irq: urb asynchronously failed - -84
usb 3-2: kingsun_rcv_irq: urb asynchronously failed - -71
usb 3-2: USB disconnect, device number 2
```

Fix: re-attach with `virsh attach-device`. Also ensure the VM's USB controller is
EHCI/UHCI (USB 2), not XHCI (USB 3) — low-speed USB devices passed through XHCI in
QEMU/KVM are known to be unstable.

### Monitoring Tools

```bash
# Raw IrDA frames
ssh root@192.168.122.26 "timeout 30 irdadump -i irda0"

# Discovery log
ssh 192.168.122.26 "cat /proc/net/irda/discovery"

# Transport layer state
ssh 192.168.122.26 "cat /proc/net/irda/irttp"

# Link management state
ssh 192.168.122.26 "cat /proc/net/irda/irlmp"

# Interface stats (TX/RX packet counts)
ssh 192.168.122.26 "ifconfig irda0"
```

### Troubleshooting

If `irda0` doesn't exist:
1. Check USB device: `lsusb | grep 07d0`
2. Reload modules: `rmmod ks959_sir ircomm_tty ircomm irda && modprobe irda && modprobe ks959_sir`
3. Re-attach USB device via virsh

If `irda0` shows `No such device`: the USB device disconnected — re-attach via virsh.

---

## Alternative Hardware Approaches

### ESP32 + TFDU4101 (Proven)

hb9eue confirmed this works with a Cressi Leonardo. The ESP32's UART peripheral has a
hardware IrDA mode (`UART_IRDA_EN` register bit) that handles SIR modulation automatically.

**Parts:** ESP32 dev board (~$5–8), TFDU4101 IrDA transceiver (~$3–5), 100Ω resistor.

**Wiring:**
```
ESP32 GPIO 17 (TXD2) ──→ TFDU4101 TXD
ESP32 GPIO 16 (RXD2) ←── TFDU4101 RXD
ESP32 3.3V ────────────→ TFDU4101 VCC
ESP32 GND ─────────────→ TFDU4101 GND
```

**Code (from hb9eue):**
```cpp
HardwareSerial irda(2); // GPIO 17: TXD2, GPIO 16: RXD2
void setup() {
    Serial.begin(115200);
    irda.begin(115200);
    // Enable hardware IrDA mode on UART2
    WRITE_PERI_REG(0x3FF6E020,
        READ_PERI_REG(0x3FF6E020) | (1<<16) | (1<<9));
    // Bit 16: UART_IRDA_EN, Bit 9: UART_IRDA_DPLX (half-duplex)
}
```

**Known issues:** Echo on TFDU4101 RX (transmitted bytes appear on RX — discard if
matches last sent byte). sebdbr reported "Unexpected answer byte" errors — may need
timing adjustments.

**Integration:** Bridge mode (ESP32 as transparent serial-to-IrDA converter, host runs
dctool against `/dev/ttyUSB0`) is simpler than compiling libdivecomputer for ESP32.

### USB-to-Serial + TFDU4101 (Simplest Hardware)

No microcontroller — just a USB serial adapter and an IrDA transceiver. Architecturally
identical to the Cressi BT Interface dock (which uses an MCP2221A internally, showing up
as `/dev/ttyACM0`).

**Parts:** FTDI FT232RL / CH340G / CP2102 (~$3–5), TFDU4101 (~$3–5).

**Usage:** `dctool -f goa -m 4 download -o dives.xml /dev/ttyUSB0`

**Risk:** FTDI adapter DTR/RTS behavior on open may cause issues. The Goa driver clears
them, which should be fine.

### BLE Transport

The Donatello supports BLE (`DC_TRANSPORT_BLE`). Uses different commands
(`CMD_LOGBOOK_BLE=0x02`, `CMD_DIVE_BLE=0x03`) and Nordic UART Service characteristics.
Avoids the entire IrDA problem but requires a BLE adapter.

```bash
bluetoothctl scan on
dctool scan -t ble
```

---

## External References

- GitHub issue: https://github.com/subsurface/subsurface/issues/4147
- Google Groups thread: https://groups.google.com/g/subsurface-divelog/c/ku56SSlCtZU
- hb9eue's ESP32 IrDA bridge code (in GitHub issue comments)
- Daniel Samarin's USB-to-IrDA adapter design (in Google Groups thread)
- Out-of-tree IrDA kernel modules: https://github.com/cschramm/irda
- Andrea Lusuardi's guide for Uwatec on Ubuntu: `docs/forum-message.txt`

---

## Key Files

### Source Code

| File | Lines | Description |
|------|-------|-------------|
| `src/main.rs` | 233 | Tokio event loop, CLI, signal handling |
| `src/usb_dongle.rs` | 559 | Kingsun protocol + 14 tests |
| `src/sir_framing.rs` | 663 | SIR wrap/unwrap + CRC + 24 tests |
| `src/pty_bridge.rs` | 424 | PTY pair + symlink + baud polling + 7 tests |
| `Cargo.toml` | 31 | Dependencies |

### Reference Material

| File | Description |
|------|-------------|
| `reference/ks959-sir.c` | Original kernel driver — canonical USB protocol reference |
| `reference/libdivecomputer/src/cressi_goa.c` | Donatello protocol (commands, framing, CRC) |
| `reference/libdivecomputer/src/serial_posix.c` | ENABLE_PTY handling, ioctl tolerance |
| `reference/irda/` | Linux IrDA subsystem (wrapper.c state machine, CRC) |

### Build & Run

```bash
cargo test                              # 45 tests
cargo build --release                   # ~2.7MB binary
sudo ./target/release/irda2tty          # run (needs USB access)
sudo ./target/release/irda2tty --help   # CLI options

# Debug logging:
RUST_LOG=debug sudo ./target/release/irda2tty
RUST_LOG=trace sudo ./target/release/irda2tty    # hex dumps of every USB transfer
```

### CLI Options

```
-s, --symlink PATH    PTY symlink path          [default: /tmp/cressi-irda]
-b, --baud RATE       Initial IrDA baud rate    [default: 9600]
    --poll-ms MS      USB RX poll interval      [default: 10]
    --sir-framing     Enable SIR BOF/EOF/CRC    [default: off]
    --extra-bofs N    Extra BOFs in SIR mode    [default: 10]
```
