# Protocol Reference

## USB Protocol (Kingsun KS-959)

Source: `reference/ks959-sir.c` — reverse-engineered by Alex Villacís Lasso in 2007
by sniffing WinXP driver USB traffic with USBSnoopy.

### Hardware Constraints

- **Low-speed USB device** — max 8 bytes per control transfer packet
- **Only 1 interface** (interface 0)
- **Interrupt endpoint 0x81 is a DUMMY** — never used for actual data
- **ALL communication uses control transfers on endpoint 0**

### IrDA SIR vs TV Remote IR

IrDA SIR (Serial Infrared) is NOT the same as 38kHz TV remote IR. This is why components
like TSOP1738 or the Flipper Zero's TSOP75338TR cannot interact with IrDA devices:

| Aspect                    | IrDA SIR                    | TV Remote (TSOP1738) |
|---------------------------|-----------------------------|----------------------|
| Carrier                   | Pulse at 3/16 of bit period | 38kHz continuous     |
| Pulse width (9600 baud)   | ~19.5µs                     | ~263µs min burst     |
| Pulse width (115200 baud) | ~1.6µs                      | N/A                  |
| Framing                   | UART (start+8data+stop)     | Burst coding         |
| Component                 | IrDA transceiver (TFDU4101) | TSOP1738 module      |

A TSOP1738 cannot detect IrDA SIR pulses — the pulse width is far below its minimum burst
detection threshold. IrDA requires a dedicated transceiver chip (TFDU4101) or a
microcontroller with hardware IrDA mode (ESP32's `UART_IRDA_EN` register bit).

**All TSOP-series receivers are incompatible** — they share the same bandwidth limitation.

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

**Fragmentation:** Max 240 cleartext bytes per transfer: `(256 & ~7) - 16 = 240`.

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

**The garbage byte:** Every time `rx_counter` wraps from 0xFF to 0x00, the byte at
that position is garbage and must be discarded. (The decoded value depends on the raw
byte received; it is not fixed at 0x95.) This happens every 255th real byte.

**Critical: RX counter desync.** If any RX bytes are lost (USB error, buffer overflow,
stale data from previous session), the counter gets permanently out of sync and all
subsequent decoding produces garbage. See [STATUS.md](STATUS.md) for the stale byte
bug we found and fixed.

### Speed Change

```
bmRequestType: 0x21  (OUT | Class | Interface)
bRequest:      0x09
wValue:        0x0200
wIndex:        0x0001       ← protocol flag "this is a speed change", NOT an interface number
wLength:       0x0008
data:          8-byte struct
```

#### Speed Payload (8 bytes, packed, little-endian)

```
struct ks959_speedparams {
    __le32 baudrate;     // e.g., 115200 = [0x00, 0xC2, 0x01, 0x00]
    __u8   flags;        // 0x03 = 8 data bits (KS_DATA_8_BITS)
    __u8   reserved[3];  // zeroes
};
```

**Supported baud rates:** 2400, 9600, 19200, 38400, 57600, 115200.

**This transfer is blocked by usbfs** — requires the kernel module. See [STATUS.md](STATUS.md).

---

## IrDA SIR Framing (NOT used by Donatello)

The Cressi Donatello uses `DC_TRANSPORT_SERIAL` — raw serial bytes over IrDA's
physical layer. No BOF/EOF/escape/CRC wrapping. The `--sir-framing` flag is for
devices that use the full IrDA protocol stack.

### SIR Frame Format

```
[XBOF × N] [BOF] [stuffed_payload] [stuffed_FCS_lo] [stuffed_FCS_hi] [EOF]
```

Constants: `BOF=0xC0`, `EOF=0xC1`, `CE=0x7D`, `XBOF=0xFF`, `IRDA_TRANS=0x20`

### CRC: Reflected CRC-CCITT

- **Polynomial:** `0x8408` (bit-reversal of 0x1021)
- **Init:** `0xFFFF`, no final XOR
- **Table entry[1]:** `0x1189`
- **Check value for "123456789":** `0x6F91`
- **Good FCS residue:** `0xF0B8`

---

## Cressi Donatello Protocol

Source: `reference/libdivecomputer/src/cressi_goa.c`

### Serial Configuration

```
115200 baud, 8 data bits, No parity, 1 stop bit, No flow control
DTR cleared, RTS cleared
Timeout: 3000ms
100ms delay before each command
```

### Packet Format

```
TX: [AA AA AA] [len] [cmd] [data...] [CRC16_LE] [55]
     header     size   cmd   payload    checksum  trailer
```

- Header: 3× `0xAA`
- len: size of data (not including header, cmd, CRC, trailer)
- CRC: `checksum_crc16_ccitt(packet+3, size+2, 0xFFFF, 0x0000)` — over len+cmd+data
- Trailer: `0x55`
- Max payload: 12 bytes (`SZ_PACKET`)

### Commands

| Command           | Code   | Description                                        |
|-------------------|--------|----------------------------------------------------|
| `CMD_VERSION`     | `0x00` | Get device version/ID                              |
| `CMD_SET_TIME`    | `0x13` | Set device time                                    |
| `CMD_EXIT_PCLINK` | `0x1D` | Exit PC-link mode                                  |
| `CMD_LOGBOOK`     | `0x21` | Download logbook (firmware < 200, 23-byte entries) |
| `CMD_DIVE`        | `0x22` | Download dive data                                 |
| `CMD_LOGBOOK_V4`  | `0x23` | Logbook (firmware >= 200, 15-byte entries)         |

### CMD_VERSION Response Format

| Offset | Size | Field                                                   |
|--------|------|---------------------------------------------------------|
| 0–3    | 4    | Serial number (uint32_le)                               |
| 4      | 1    | Model number                                            |
| 5–6    | 2    | Firmware version (uint16_le)                            |
| 7–8    | 2    | Reserved/padding                                        |
| 9–10   | 2    | Data format version (uint16_le) — only if id_size == 11 |

### Firmware Version → Logbook Format

```c
if (firmware >= 200) → CMD_LOGBOOK_V4 (0x23), logbook entry = 15 bytes
else                  → CMD_LOGBOOK (0x21),    logbook entry = 23 bytes
```

### Download Protocol (per dive)

1. Host sends command packet (CMD_DIVE + 2-byte dive number)
2. Device sends response packet (small, contains total size)
3. Device sends data packets — **516 bytes each:** `[2 bytes total_size_LE] [512 bytes data] [2 bytes CRC16-LE]`
4. Host sends ACK (0x06) after each data packet
5. Device sends END (0x04) when done
6. Host sends final ACK

### Communication Flow

1. Open serial port
2. Configure 115200 8N1, clear DTR/RTS
3. Sleep 100ms, purge buffers
4. Send `CMD_VERSION` → get device ID
5. Send `CMD_LOGBOOK` → download logbook entries
6. For each dive: send `CMD_DIVE` → download dive profile
7. Send `CMD_EXIT_PCLINK` → release device

### Observed Response (from hardware test)

```
CMD_VERSION sent:  AA AA AA 00 00 00 00 55
Response:          AA AA AA 0B 21 1A AD 00 00 04 31 01 11 00 05 00 75 38 55

Parsed:
  Serial:  0x0000AD1A = 44314
  Model:   0x04 = 4 (Donatello)
  FW:      0x0131 = 305
```
