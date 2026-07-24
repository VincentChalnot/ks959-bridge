# Full codebase review and audit

**Session ID:** ses_0c34ea2dcffeLOdMm5mpdMOv5D
**Created:** 7/7/2026, 3:08:04 PM
**Updated:** 7/7/2026, 3:14:03 PM

---

## User

Read the documentation of the project from the following files:
docs/ARCHITECTURE.md
docs/PROTOCOL.md
docs/SETUP.md
docs/STATUS.md
docs/TESTING.md 
AGENTS.md
README.md
When you have understood the project and it's goal feel free to explore the codebase, just DON'T go into the reference folder, it contains the entire linux kernel and the entire libdivecomputer library.

Create a reference document right here in this conversation of eveything that should be flagged as tricky, dangerous, misleading or just wrong.
I want a complete 360 view of whether or not this project is sane and could work properly eventually.
This project is mainly the work of "lesser" models that could have make mistakes or drown in their own context so also flag any code that looks messy or difficult to maintain.
If you find "quick wins" along the way (stuff you could fix by yourself immediately), please refrain to fix them, simply list the solution in the final document.

---

## Assistant (Architect · Claude Fable 5 · 8.2s)

_Thinking:_

I'll start by reviewing the documentation to understand the structure, then dive into the codebase.

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/docs/ARCHITECTURE.md"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/docs/ARCHITECTURE.md</path>
<type>file</type>
<content>
1: # Architecture
2: 
3: ## Module Structure
4: 
5: ```
6: src/
7:   main.rs          — tokio event loop, CLI (clap), signal handling
8:   usb_dongle.rs    — Kingsun KS-959 protocol (obfuscation, speed, fragmentation)
9:   sir_framing.rs   — IrDA SIR wrap/unwrap (optional, off by default)
10:   pty_bridge.rs    — PTY pair, symlink, baud rate polling
11: kmod/
12:   ks959_speed.c    — Kernel module to bypass usbfs for speed change
13:   Makefile          — Build against running kernel's headers
14: ```
15: 
16: ## Event Loop (main.rs)
17: 
18: Single-threaded `tokio` runtime (`current_thread` flavor) with `select!` over three sources:
19: 
20: ```
21: loop {
22:     tokio::select! {
23:         // 1. PTY master readable — data from dctool/Subsurface
24:         readable = async_master.readable() => {
25:             - Read from PTY master (non-blocking)
26:             - Check for baud rate change via tcgetattr() polling
27:             - If baud changed: reset RX counter, send speed change to dongle
28:             - Forward data to dongle (with optional SIR wrapping)
29:         }
30: 
31:         // 2. USB RX poll timer (10ms interval) — data from IR
32:         _ = poll_interval.tick() => {
33:             - Poll dongle for received bytes (control-IN transfer)
34:             - De-obfuscate with persistent counter
35:             - Write to PTY master (so dctool reads it)
36:         }
37: 
38:         // 3. Signal handler (SIGINT/SIGTERM) — clean shutdown
39:         _ = sigint.recv() => { break; }
40:         _ = sigterm.recv() => { break; }
41:     }
42: }
43: ```
44: 
45: ### Key Design Decisions
46: 
47: - **tokio `current_thread`** — no need for multi-threaded runtime; all I/O is sequential
48: - **`AsyncFd` for PTY master** — wraps the raw fd for async readability notifications
49: - **10ms poll interval** — matches kernel driver's URB resubmission cadence
50: - **Baud rate detection via `tcgetattr()` polling** — TIOCPKT was evaluated and rejected
51:   (doesn't fire for plain `tcsetattr()` baud rate changes on Linux)
52: 
53: ## Data Flow
54: 
55: ### TX Path (dctool → dongle → IR)
56: 
57: ```
58: dctool writes to PTY slave
59:   → PTY master becomes readable
60:   → bridge reads from PTY master
61:   → check_baud_rate_change() (tcgetattr on slave fd)
62:   → dongle.send() obfuscates + fragments
63:   → USB control-OUT transfer (bRequestType=0x21, wIndex=0x0000)
64:   → dongle transmits over IrDA SIR
65: ```
66: 
67: ### RX Path (IR → dongle → dctool)
68: 
69: ```
70: dongle receives IrDA SIR data
71:   → bridge polls every 10ms (USB control-IN, bRequestType=0xA1)
72:   → deobfuscate_rx_buffer() with persistent counter
73:   → write to PTY master
74:   → dctool reads from PTY slave
75: ```
76: 
77: ### Speed Change Path
78: 
79: ```
80: dctool calls tcsetattr() on PTY slave (sets 115200 baud)
81:   → bridge detects via tcgetattr() polling on next PTY read
82:   → dongle.reset_rx_counter()  ← critical: flush stale bytes
83:   → dongle.set_speed(115200)
84:   → USB control-OUT (bRequestType=0x41, wIndex=0x0001)
85:   → dongle reconfigures IR link
86:   (or STALL if usbfs blocks it — kernel module is the reliable path)
87: ```
88: 
89: ## Dependencies
90: 
91: ```toml
92: nusb = "0.1"           # async USB (pure Rust, no libusb C dependency)
93: tokio = "1"            # async runtime (current_thread flavor)
94: nix = "0.29"           # PTY, termios, fcntl (features: term, ioctl, fs)
95: clap = "4"             # CLI parsing (derive feature)
96: tracing = "0.1"        # structured logging
97: tracing-subscriber = "0.3"  # log output (env-filter feature)
98: thiserror = "2"        # typed errors per module
99: anyhow = "1"           # top-level error chaining in main
100: crc = "3"              # in Cargo.toml but NOT USED — CRC computed at compile time
101: ```
102: 
103: ## Error Handling
104: 
105: - `thiserror` enums in each module (`DongleError`, `PtyError`, `SirError`)
106: - `anyhow` in `main.rs` for `.context()` chaining
107: - No `unwrap()` on the data path — only in one-time setup where failure is fatal
108: - `EAGAIN`/`EWOULDBLOCK` from PTY reads is normal → clear readiness, retry
109: 
110: ## CLI Options
111: 
112: ```
113: -s, --symlink PATH    PTY symlink path          [default: /tmp/cressi-irda]
114: -b, --baud RATE       Initial IrDA baud rate    [default: 9600]
115:     --skip-speed-change  Skip USB speed change at startup (use with kernel module)
116:     --poll-ms MS      USB RX poll interval      [default: 10]
117:     --sir-framing     Enable SIR BOF/EOF/CRC    [default: off]
118:     --extra-bofs N    Extra BOFs in SIR mode    [default: 10]
119: ```
120: 
121: ## Reference Code (do not modify)
122: 
123: | File                                           | Description                                               |
124: |------------------------------------------------|-----------------------------------------------------------|
125: | `reference/ks959-sir.c`                        | Original kernel driver — canonical USB protocol reference |
126: | `reference/libdivecomputer/src/cressi_goa.c`   | Donatello protocol (commands, framing, CRC)               |
127: | `reference/libdivecomputer/src/serial_posix.c` | ENABLE_PTY handling, ioctl tolerance                      |
128: | `reference/libdivecomputer/examples/`          | dctool source (builds to `.libs/dctool`)                  |

(End of file - total 128 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/docs/PROTOCOL.md"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/docs/PROTOCOL.md</path>
<type>file</type>
<content>
1: # Protocol Reference
2: 
3: ## USB Protocol (Kingsun KS-959)
4: 
5: Source: `reference/ks959-sir.c` — reverse-engineered by Alex Villacís Lasso in 2007
6: by sniffing WinXP driver USB traffic with USBSnoopy.
7: 
8: ### Hardware Constraints
9: 
10: - **Low-speed USB device** — max 8 bytes per control transfer packet
11: - **Only 1 interface** (interface 0)
12: - **Interrupt endpoint 0x81 is a DUMMY** — never used for actual data
13: - **ALL communication uses control transfers on endpoint 0**
14: 
15: ### IrDA SIR vs TV Remote IR
16: 
17: IrDA SIR (Serial Infrared) is NOT the same as 38kHz TV remote IR. This is why components
18: like TSOP1738 or the Flipper Zero's TSOP75338TR cannot interact with IrDA devices:
19: 
20: | Aspect                    | IrDA SIR                    | TV Remote (TSOP1738) |
21: |---------------------------|-----------------------------|----------------------|
22: | Carrier                   | Pulse at 3/16 of bit period | 38kHz continuous     |
23: | Pulse width (9600 baud)   | ~19.5µs                     | ~263µs min burst     |
24: | Pulse width (115200 baud) | ~1.6µs                      | N/A                  |
25: | Framing                   | UART (start+8data+stop)     | Burst coding         |
26: | Component                 | IrDA transceiver (TFDU4101) | TSOP1738 module      |
27: 
28: A TSOP1738 cannot detect IrDA SIR pulses — the pulse width is far below its minimum burst
29: detection threshold. IrDA requires a dedicated transceiver chip (TFDU4101) or a
30: microcontroller with hardware IrDA mode (ESP32's `UART_IRDA_EN` register bit).
31: 
32: **All TSOP-series receivers are incompatible** — they share the same bandwidth limitation.
33: 
34: ### TX: Host → Dongle (data to transmit over IR)
35: 
36: ```
37: bmRequestType: 0x21  (OUT | Class | Interface)
38: bRequest:      0x09
39: wValue:        <cleartext length>   (u16 LE)
40: wIndex:        0x0000
41: wLength:       <padded length>      (u16 LE)
42: data:          obfuscated + padded payload
43: ```
44: 
45: #### TX Obfuscation Algorithm
46: 
47: ```
48: LOOKUP = "wangshuofei19710"   (16 bytes, the firmware author's name + digits)
49: 
50: padded_len = ((cleartext_len + 7) & ~7) + 16
51: xor_mask   = LOOKUP[(cleartext_len & 0x0F) ^ 0x06] ^ 0x55
52: 
53: for each cleartext byte:
54:     obfuscated[i] = cleartext[i] ^ xor_mask
55: 
56: remaining bytes (padding) = 0x00
57: ```
58: 
59: **Fragmentation:** Max 240 cleartext bytes per transfer: `(256 & ~7) - 16 = 240`.
60: 
61: #### TX Test Vector (hand-calculated)
62: 
63: ```
64: cleartext = [0x01, 0x02, 0x03], len = 3
65: padded_len = ((3+7) & ~7) + 16 = 24
66: mask_index = (3 & 0x0F) ^ 0x06 = 5
67: LOOKUP[5]  = 'h' = 0x68
68: xor_mask   = 0x68 ^ 0x55 = 0x3D
69: obfuscated = [0x3C, 0x3F, 0x3E, 0x00 × 21]
70: ```
71: 
72: ### RX: Dongle → Host (data received from IR) — Polling
73: 
74: ```
75: bmRequestType: 0xA1  (IN | Class | Interface)
76: bRequest:      0x01
77: wValue:        0x0200
78: wIndex:        0x0000
79: wLength:       0x0800  (2048 bytes — max buffer)
80: ```
81: 
82: Returns raw bytes in the response data. Empty response = no data available.
83: Poll interval: **10ms** (matches kernel driver's URB resubmission cadence).
84: 
85: #### RX De-obfuscation Algorithm
86: 
87: ```
88: rx_counter: u8   — starts at 0, persists across ALL reads for the entire session
89:                    (NOT reset between polls or frames)
90: 
91: for each raw byte received:
92:     rx_counter = rx_counter.wrapping_add(1)    // increment first
93:     decoded = raw_byte ^ rx_counter ^ 0x55
94:     if rx_counter == 0:
95:         skip this byte                         // garbage byte every 255 real bytes
96:     else:
97:         emit decoded byte
98: ```
99: 
100: **The garbage byte:** Every time `rx_counter` wraps from 0xFF to 0x00, the byte at
101: that position decodes to 0x95 and must be discarded. This is every 255th real byte.
102: 
103: **Critical: RX counter desync.** If any RX bytes are lost (USB error, buffer overflow,
104: stale data from previous session), the counter gets permanently out of sync and all
105: subsequent decoding produces garbage. See [STATUS.md](STATUS.md) for the stale byte
106: bug we found and fixed.
107: 
108: ### Speed Change
109: 
110: ```
111: bmRequestType: 0x21  (OUT | Class | Interface)
112: bRequest:      0x09
113: wValue:        0x0200
114: wIndex:        0x0001       ← protocol flag "this is a speed change", NOT an interface number
115: wLength:       0x0008
116: data:          8-byte struct
117: ```
118: 
119: #### Speed Payload (8 bytes, packed, little-endian)
120: 
121: ```
122: struct ks959_speedparams {
123:     __le32 baudrate;     // e.g., 115200 = [0x00, 0xC2, 0x01, 0x00]
124:     __u8   flags;        // 0x03 = 8 data bits (KS_DATA_8_BITS)
125:     __u8   reserved[3];  // zeroes
126: };
127: ```
128: 
129: **Supported baud rates:** 2400, 9600, 19200, 38400, 57600, 115200.
130: 
131: **This transfer is blocked by usbfs** — requires the kernel module. See [STATUS.md](STATUS.md).
132: 
133: ---
134: 
135: ## IrDA SIR Framing (NOT used by Donatello)
136: 
137: The Cressi Donatello uses `DC_TRANSPORT_SERIAL` — raw serial bytes over IrDA's
138: physical layer. No BOF/EOF/escape/CRC wrapping. The `--sir-framing` flag is for
139: devices that use the full IrDA protocol stack.
140: 
141: ### SIR Frame Format
142: 
143: ```
144: [XBOF × N] [BOF] [stuffed_payload] [stuffed_FCS_lo] [stuffed_FCS_hi] [EOF]
145: ```
146: 
147: Constants: `BOF=0xC0`, `EOF=0xC1`, `CE=0x7D`, `XBOF=0xFF`, `IRDA_TRANS=0x20`
148: 
149: ### CRC: Reflected CRC-CCITT
150: 
151: - **Polynomial:** `0x8408` (bit-reversal of 0x1021)
152: - **Init:** `0xFFFF`, no final XOR
153: - **Table entry[1]:** `0x1189`
154: - **Check value for "123456789":** `0x6F91`
155: - **Good FCS residue:** `0xF0B8`
156: 
157: ---
158: 
159: ## Cressi Donatello Protocol
160: 
161: Source: `reference/libdivecomputer/src/cressi_goa.c`
162: 
163: ### Serial Configuration
164: 
165: ```
166: 115200 baud, 8 data bits, No parity, 1 stop bit, No flow control
167: DTR cleared, RTS cleared
168: Timeout: 3000ms
169: 100ms delay before each command
170: ```
171: 
172: ### Packet Format
173: 
174: ```
175: TX: [AA AA AA] [len] [cmd] [data...] [CRC16_LE] [55]
176:      header     size   cmd   payload    checksum  trailer
177: ```
178: 
179: - Header: 3× `0xAA`
180: - len: size of data (not including header, cmd, CRC, trailer)
181: - CRC: `checksum_crc16_ccitt(packet+3, size+2, 0x000, 0x0000)` — over len+cmd+data
182: - Trailer: `0x55`
183: - Max payload: 12 bytes (`SZ_PACKET`)
184: 
185: ### Commands
186: 
187: | Command           | Code   | Description                                        |
188: |-------------------|--------|----------------------------------------------------|
189: | `CMD_VERSION`     | `0x00` | Get device version/ID                              |
190: | `CMD_SET_TIME`    | `0x13` | Set device time                                    |
191: | `CMD_EXIT_PCLINK` | `0x1D` | Exit PC-link mode                                  |
192: | `CMD_LOGBOOK`     | `0x21` | Download logbook (firmware < 200, 23-byte entries) |
193: | `CMD_DIVE`        | `0x22` | Download dive data                                 |
194: | `CMD_LOGBOOK_V4`  | `0x23` | Logbook (firmware >= 200, 15-byte entries)         |
195: 
196: ### CMD_VERSION Response Format
197: 
198: | Offset | Size | Field                                                   |
199: |--------|------|---------------------------------------------------------|
200: | 0–3    | 4    | Serial number (uint32_le)                               |
201: | 4      | 1    | Model number                                            |
202: | 5–6    | 2    | Firmware version (uint16_le)                            |
203: | 7–8    | 2    | Reserved/padding                                        |
204: | 9–10   | 2    | Data format version (uint16_le) — only if id_size == 11 |
205: 
206: ### Firmware Version → Logbook Format
207: 
208: ```c
209: if (firmware >= 200) → CMD_LOGBOOK_V4 (0x23), logbook entry = 15 bytes
210: else                  → CMD_LOGBOOK (0x21),    logbook entry = 23 bytes
211: ```
212: 
213: ### Download Protocol (per dive)
214: 
215: 1. Host sends command packet (CMD_DIVE + 2-byte dive number)
216: 2. Device sends response packet (small, contains total size)
217: 3. Device sends data packets — **516 bytes each:** `[2 bytes total_size_LE] [512 bytes data] [2 bytes CRC16-LE]`
218: 4. Host sends ACK (0x06) after each data packet
219: 5. Device sends END (0x04) when done
220: 6. Host sends final ACK
221: 
222: ### Communication Flow
223: 
224: 1. Open serial port
225: 2. Configure 115200 8N1, clear DTR/RTS
226: 3. Sleep 100ms, purge buffers
227: 4. Send `CMD_VERSION` → get device ID
228: 5. Send `CMD_LOGBOOK` → download logbook entries
229: 6. For each dive: send `CMD_DIVE` → download dive profile
230: 7. Send `CMD_EXIT_PCLINK` → release device
231: 
232: ### Observed Response (from hardware test)
233: 
234: ```
235: CMD_VERSION sent:  AA AA AA 00 00 00 00 55
236: Response:          AA AA AA 0B 21 1A AD 00 00 04 31 01 11 00 05 00 75 38 55
237: 
238: Parsed:
239:   Serial:  0x0000AD1A = 44314
240:   Model:   0x04 = 4 (Donatello)
241:   FW:      0x0131 = 305
242: ```

(End of file - total 242 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/docs/SETUP.md"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/docs/SETUP.md</path>
<type>file</type>
<content>
1: # Setup & Environment
2: 
3: ## Dev Machine
4: 
5: - **OS:** Fedora 43
6: - **Kernel:** `7.0.14-101.fc43.x86_64`
7: - **Kernel headers:** `/lib/modules/7.0.14-101.fc43.x86_64/build/`
8: 
9: ## Hardware
10: 
11: - **Dongle:** Kingsun KS-959, VID=`07d0`, PID=`4959`
12: - **Target device:** Cressi Donatello dive computer
13: - **Communication:** 115200 baud, 8N1, no flow control, via IrDA SIR
14: 
15: ## Build Commands
16: 
17: ```bash
18: cargo build --release          # builds to ./target/release/ks959-bridge (~2.7MB)
19: cargo test                     # runs all 45 unit tests (no hardware needed)
20: cargo fmt --check              # format check
21: cargo clippy                   # linter (if available)
22: ```
23: 
24: ## USB Permissions
25: 
26: Run as `root`, or create a udev rule:
27: 
28: ```
29: # /etc/udev/rules.d/99-kingsun.rules
30: SUBSYSTEM=="usb", ATTR{idVendor}=="07d0", ATTR{idProduct}=="4959", MODE="0666"
31: ```
32: 
33: Then reload:
34: 
35: ```bash
36: sudo udevadm control --reload-rules && sudo udevadm trigger
37: ```
38: 
39: ## Kernel Module (Speed Change)
40: 
41: The dongle defaults to 9600 baud. The Donatello needs 115200. The USB speed change
42: control transfer is blocked by `usbfs check_ctrlrecip()` — see [STATUS.md](STATUS.md)
43: for the full story. The solution is a minimal kernel module.
44: 
45: ### Build
46: 
47: ```bash
48: cd kmod/
49: make            # produces ks959_speed.ko
50: ```
51: 
52: ### Load
53: 
54: ```bash
55: sudo insmod kmod/ks959_speed.ko baud=115200
56: ```
57: 
58: **Important:** The module can only be used **once per USB plug cycle**. It returns
59: `-ENODEV` from `probe()` so it doesn't permanently claim the device, but this means
60: the kernel won't re-probe until the dongle is physically unplugged and replugged.
61: 
62: ### Verify
63: 
64: ```bash
65: lsmod | grep ks959    # should show ks959_speed
66: ```
67: 
68: ### Unload
69: 
70: ```bash
71: sudo rmmod ks959_speed
72: ```
73: 
74: The speed setting persists in the dongle firmware until power cycle (unplug).
75: 
76: ## LD_PRELOAD Shim (for libdivecomputer)
77: 
78: libdivecomputer calls `ioctl(TIOCMBIC)` to clear the RTS line. PTYs don't support
79: modem control ioctls (they return `ENOTTY`). A tiny `LD_PRELOAD` shim silently
80: succeeds for these calls:
81: 
82: ### Build
83: 
84: ```bash
85: cat > /tmp/pty_modem_shim.c << 'EOF'
86: #define _GNU_SOURCE
87: #include <dlfcn.h>
88: #include <sys/ioctl.h>
89: #include <errno.h>
90: #include <stddef.h>
91: 
92: int ioctl(int fd, unsigned long request, ...) {
93:     static int (*real_ioctl)(int, unsigned long, ...) = NULL;
94:     if (!real_ioctl)
95:         real_ioctl = dlsym(RTLD_NEXT, "ioctl");
96: 
97:     __builtin_va_list ap;
98:     __builtin_va_start(ap, request);
99:     void *arg = __builtin_va_arg(ap, void *);
100:     __builtin_va_end(ap);
101: 
102:     int result = real_ioctl(fd, request, arg);
103: 
104:     if (result < 0 && (errno == ENOTTY || errno == EINVAL)) {
105:         switch (request) {
106:             case TIOCMBIS:
107:             case TIOCMBIC:
108:             case TIOCMSET:
109:                 errno = 0;
110:                 return 0;
111:             case TIOCMGET:
112:                 errno = 0;
113:                 if (arg) *(int *)arg = 0;
114:                 return 0;
115:         }
116:     }
117:     return result;
118: }
119: EOF
120: gcc -shared -fPIC -o /tmp/pty_modem_shim.so /tmp/pty_modem_shim.c -ldl
121: ```
122: 
123: ### Usage
124: 
125: ```bash
126: LD_PRELOAD=/tmp/pty_modem_shim.so dctool ...
127: ```
128: 
129: ## Full End-to-End Procedure
130: 
131: ```bash
132: # 1. Build everything
133: cargo build --release
134: cd kmod && make && cd ..
135: 
136: # 2. Load kernel module (sets dongle to 115200)
137: sudo insmod kmod/ks959_speed.ko baud=115200
138: 
139: # 3. Start bridge
140: sudo ./target/release/ks959-bridge --baud 115200 --skip-speed-change &
141: 
142: # 4. Put Donatello in PC mode (have ~1 minute before hibernation!)
143: 
144: # 5. Run dctool immediately
145: LD_PRELOAD=/tmp/pty_modem_shim.so \
146: LD_LIBRARY_PATH=./reference/libdivecomputer/src/.libs \
147:   ./reference/libdivecomputer/examples/.libs/dctool \
148:   -v -l cressi.log -f goa -m 4 download -o dives.xml /tmp/cressi-irda
149: 
150: # 6. Clean up
151: sudo rmmod ks959_speed    # optional, speed persists until unplug
152: ```
153: 
154: ---
155: 
156: ## VM/USB Passthrough Setup (for kernel IrDA testing)
157: 
158: This was used only for testing the kernel IrDA stack (to confirm the IrLAP dead end).
159: The production ks959-bridge tool runs on the host directly.
160: 
161: ### Environment
162: 
163: - Host: Linux (Fedora/RHEL-based), QEMU/KVM via libvirt
164: - VM: Ubuntu 18.04, kernel 4.15 (last version with staging IrDA drivers)
165: 
166: ### USB Passthrough
167: 
168: ```bash
169: # Find device on host
170: lsusb | grep 07d0
171: 
172: # Create XML and attach to running VM
173: cat > /tmp/kingsun-usb.xml << 'EOF'
174: <hostdev mode='subsystem' type='usb'>
175:   <source>
176:     <vendor id='0x07d0'/>
177:     <product id='0x4959'/>
178:   </source>
179: </hostdev>
180: EOF
181: virsh -c qemu:///system attach-device ubuntu18.04 /tmp/kingsun-usb.xml --live
182: ```
183: 
184: The KS-959 moves bus/device numbers on replug — re-run `virsh attach-device` after each
185: physical reconnect.
186: 
187: ### Kernel Module Loading (in VM)
188: 
189: ```bash
190: ssh root@<vm-ip> "modprobe ircomm_tty"
191: ssh <vm-ip> "lsmod | grep irda"           # Expected: irda, ks959_sir, ircomm, ircomm_tty
192: ssh <vm-ip> "ls -la /dev/ircomm0"          # Should exist
193: ssh root@<vm-ip> "chmod 666 /dev/ircomm0 && usermod -aG dialout vincent"
194: ```
195: 
196: ### Monitoring Tools (in VM)
197: 
198: ```bash
199: ssh root@<vm-ip> "timeout 30 irdadump -i irda0"   # Raw IrDA frames
200: ssh <vm-ip> "cat /proc/net/irda/discovery"          # Discovery log
201: ssh <vm-ip> "ifconfig irda0"                         # Interface stats
202: ```
203: 
204: ### USB Disconnect Issue
205: 
206: The KS-959 can disconnect from the VM under load. Fix: re-attach with `virsh attach-device`.
207: Also ensure the VM's USB controller is EHCI/UHCI (USB 2), not XHCI (USB 3) — low-speed USB
208: devices passed through XHCI in QEMU/KVM are known to be unstable.
209: 
210: ---
211: 
212: ## Alternative Hardware Approaches
213: 
214: If ks959-bridge doesn't work for your setup, these alternatives exist:
215: 
216: ### ESP32 + TFDU4101 (Proven)
217: 
218: hb9eue confirmed this works with a Cressi Leonardo. The ESP32's UART peripheral has a
219: hardware IrDA mode (`UART_IRDA_EN` register bit) that handles SIR modulation automatically.
220: 
221: **Parts:** ESP32 dev board (~$5–8), TFDU4101 IrDA transceiver (~$3–5), 100Ω resistor.
222: 
223: **Wiring:**
224: 
225: ```
226: ESP32 GPIO 17 (TXD2) ──→ TFDU4101 TXD
227: ESP32 GPIO 16 (RXD2) ←── TFDU4101 RXD
228: ESP32 3.3V ────────────→ TFDU4101 VCC
229: ESP32 GND ─────────────→ TFDU4101 GND
230: ```
231: 
232: **Code (from hb9eue):**
233: 
234: ```cpp
235: HardwareSerial irda(2); // GPIO 17: TXD2, GPIO 16: RXD2
236: void setup() {
237:     Serial.begin(115200);
238:     irda.begin(115200);
239:     // Enable hardware IrDA mode on UART2
240:     WRITE_PERI_REG(0x3FF6E020,
241:         READ_PERI_REG(0x3FF6E020) | (1<<16) | (1<<9));
242:     // Bit 16: UART_IRDA_EN, Bit 9: UART_IRDA_DPLX (half-duplex)
243: }
244: ```
245: 
246: **Known issues:**
247: 
248: - Echo on TFDU4101 RX: transmitted bytes appear on RX pin. Filter: `if (irdaByte == serialByte) discard;`
249: - DTR/RTS toggling resets ESP32 via auto-reset circuit (not an issue for Donatello, which only clears RTS=0)
250: 
251: ### USB-to-Serial + TFDU4101 (Simplest Hardware — Proven)
252: 
253: No microcontroller — just a USB serial adapter and an IrDA transceiver. **Proven by Daniel Samarin.**
254: Architecturally identical to the Cressi BT Interface dock.
255: 
256: **Parts:** FTDI FT232RL / CH340G / CP2102 (~$3–5), TFDU4101 (~$3–5).
257: 
258: **Wiring:**
259: 
260: ```
261: FTDI TX  ──→ TFDU4101 TXD
262: FTDI RX  ←── TFDU4101 RXD
263: FTDI VCC ──→ TFDU4101 VCC
264: FTDI GND ──→ TFDU4101 GND
265: ```
266: 
267: **Usage:** `dctool -f goa -m 4 download -o dives.xml /dev/ttyUSB0`
268: 
269: ### BLE Transport
270: 
271: The Donatello supports BLE (`DC_TRANSPORT_BLE`). Uses different commands
272: (`CMD_LOGBOOK_BLE=0x02`, `CMD_DIVE_BLE=0x03`) and Nordic UART Service characteristics.
273: Avoids the entire IrDA problem but requires a BLE adapter.
274: 
275: ```bash
276: bluetoothctl scan on
277: dctool scan -t ble
278: ```
279: 
280: ## External References
281: 
282: - [Subsurface issue #4147](https://github.com/subsurface/subsurface/issues/4147) — GitHub discussion with hb9eue's ESP32
283:   solution
284: - [Subsurface mailing list thread](https://groups.google.com/g/subsurface-divelog/c/ku56SSlCtZU) — Google Groups
285:   discussion
286: - [Out-of-tree IrDA kernel modules](https://github.com/cschramm/irda) — DKMS build of the removed IrDA stack
287: 

(End of file - total 287 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/docs/STATUS.md"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/docs/STATUS.md</path>
<type>file</type>
<content>
1: # Project Status
2: 
3: ## Why This Project Exists
4: 
5: Linux removed the entire IrDA subsystem in kernel 4.17 (2018). The Kingsun KS-959 dongle
6: had a kernel driver (`ks959-sir.c`) that lived inside that subsystem. On any modern kernel
7: (Fedora 43, kernel 7.0.14), the dongle is a dead USB device with no driver.
8: 
9: This is NOT a kernel driver replacement — the old IrDA stack was tested and doesn't work
10: with the Cressi Donatello. Instead, this project bypasses the entire IrDA protocol stack,
11: reverse-engineering direct USB communication with the dongle.
12: 
13: ### The IrDA Protocol Stack
14: 
15: The full IrDA stack has 5 layers. The kernel handled layers 2–4; the dongle handles
16: layers 1 and 5 in hardware:
17: 
18: ```
19: ┌─────────────────────────────────────────────────┐
20: │  Application (libdivecomputer / dctool)         │
21: │  Opens /dev/ircomm0, reads/writes serial data   │
22: ├─────────────────────────────────────────────────┤
23: │  IrCOMM (ircomm_tty kernel module)              │
24: │  Serial port emulation over IrDA                │
25: │  Presents /dev/ircomm* character devices        │
26: ├─────────────────────────────────────────────────┤
27: │  IrLMP (IrDA Link Management Protocol)          │
28: │  Service discovery (IAS), multiplexing          │
29: ├─────────────────────────────────────────────────┤
30: │  IrLAP (IrDA Link Access Protocol)              │
31: │  Framing, connection management, flow control   │
32: │  SNRM/UA handshake, I-frames, RR/RNR            │
33: ├─────────────────────────────────────────────────┤
34: │  SIR (Serial Infrared) — kernel driver          │
35: │  Byte stuffing, BOF/EOF, CRC                    │
36: │  ks959_sir driver for Kingsun adapter           │
37: ├─────────────────────────────────────────────────┤
38: │  IrPHY — hardware (KS-959 transceiver)          │
39: │  IrDA SIR modulation/demodulation               │
40: │  940nm IR LED + photodiode                      │
41: └─────────────────────────────────────────────────┘
42: ```
43: 
44: The Cressi Donatello only implements the physical layer + minimal discovery (XID). It
45: does **not** implement the full IrLAP connection protocol. This is why the kernel path
46: fails and why ks959-bridge bypasses layers 2–4 entirely.
47: 
48: ### Why the Kernel IrDA Path Is a Dead End
49: 
50: Confirmed via `irdadump` capture on an Ubuntu 18.04 VM with the kernel IrDA stack loaded:
51: 
52: ```
53: 21:01:30.667420 xid:cmd 4410ab90 > ffffffff S=6 s=* cressi hint=8404 [ Computer IrCOMM ] (23)
54: ```
55: 
56: **XID discovery succeeds** — the Donatello responds and advertises itself as a Cressi
57: IrCOMM device (hint bits `0x8404`). The IrDA SIR physical layer works.
58: 
59: **No IrLAP connection handshake follows** — no SNRM (connect request), no UA
60: (acknowledge), no data frames. The Donatello implements just enough IrDA to be
61: discoverable, but never completes a full IrLAP session.
62: 
63: The hang sequence when running `dctool` against `/dev/ircomm0`:
64: 
65: 1. dctool opens `/dev/ircomm0`
66: 2. IrCOMM layer requests an IrLAP connection to the discovered device
67: 3. IrLAP sends SNRM (or attempts to)
68: 4. The Donatello does not respond to SNRM
69: 5. IrLAP waits indefinitely (no timeout on connection establishment)
70: 6. IrCOMM never gets a connected channel
71: 7. dctool's write blocks forever
72: 
73: This cannot be fixed without modifying the kernel IrDA stack to add a "raw" mode.
74: 
75: ### Approaches Evaluated
76: 
77: | # | Approach                                          | Verdict                                                             | Cost        |
78: |---|---------------------------------------------------|---------------------------------------------------------------------|-------------|
79: | 1 | Official Cressi BT/USB dock                       | Works but expensive; uses MCP2221A + IrDA transceiver internally    | ~$80–120    |
80: | 2 | KS-959 + kernel IrDA stack (VM)                   | **Dead end** — Donatello never completes IrLAP handshake            | $0          |
81: | 3 | Out-of-tree IrDA kernel modules (`cschramm/irda`) | Viable for other IrDA devices, same IrLAP dead end for Donatello    | $0          |
82: | 4 | Flipper Zero (RX capture or TX raw replay)        | **Not viable** — TSOP75338TR hardware-locked to 38kHz carrier       | $0          |
83: | 5 | Bare LED + Arduino GPIO/UART                      | **Not viable** — produces raw baseband NRZ, not IrDA SIR modulation | ~$5         |
84: | 6 | ESP32 + TFDU4101 (hardware IrDA UART mode)        | **Proven** by hb9eue for Cressi Leonardo                            | ~$8–15      |
85: | 7 | USB-to-Serial (FTDI/CH340) + TFDU4101             | **Proven** by Daniel Samarin                                        | ~$5–8       |
86: | 8 | BLE transport                                     | Supported by libdivecomputer for Goa family                         | BLE adapter |
87: | 9 | **libusb bypass of KS-959**                       | **Chosen path** — no new hardware, no soldering                     | $0          |
88: 
89: **Why approach #9 was selected:** Zero additional cost, uses existing hardware, avoids
90: the proven IrLAP dead end, avoids SMD soldering risk.
91: 
92: ## What Works (verified on hardware)
93: 
94: | Component                         | Status       | Notes                                                         |
95: |-----------------------------------|--------------|---------------------------------------------------------------|
96: | USB enumeration + interface claim | ✅ Works      | nusb finds dongle, detaches kernel driver, claims interface 0 |
97: | USB TX (data → dongle)            | ✅ Works      | bRequestType=0x21, wIndex=0x0000 passes usbfs validation      |
98: | USB RX polling (dongle → host)    | ✅ Works      | Polls every 10ms, all URBs complete with status=0             |
99: | RX de-obfuscation                 | ✅ Works      | Counter arithmetic correct, garbage byte skip works           |
100: | PTY bridge creation               | ✅ Works      | Creates slave, symlink to `/tmp/cressi-irda`                  |
101: | Baud rate detection               | ✅ Works      | tcgetattr() polling detects tcsetattr() from dctool           |
102: | Speed change via kernel module    | ✅ Works      | `insmod ks959_speed.ko baud=115200` succeeds                  |
103: | CMD_VERSION response              | ✅ Works      | Donatello responds with model=4, firmware=305, serial=44314   |
104: | LD_PRELOAD shim for modem ioctls  | ✅ Works      | Silently succeeds for TIOCMBIC/TIOCMBIS/TIOCMGET              |
105: | Unit tests                        | ✅ 45/45 pass | SIR framing, obfuscation, PTY bridge                          |
106: 
107: ## What Doesn't Work (yet)
108: 
109: | Component                                  | Status      | Notes                                           |
110: |--------------------------------------------|-------------|-------------------------------------------------|
111: | Speed change via usbfs (bRequestType=0x41) | ❌ STALLs    | Dongle firmware checks type bits, requires 0x21 |
112: | Speed change via usbfs (bRequestType=0x21) | ❌ Blocked   | usbfs check_ctrlrecip rejects wIndex=1          |
113: | End-to-end dive download                   | 🔲 Untested | Need Donatello in PC mode + correct IR link     |
114: 
115: ## The Speed Change Saga
116: 
117: ### The Problem
118: 
119: The speed change control transfer uses `wIndex=0x0001` with `bRequestType=0x21`
120: (Class+Interface). In the dongle's protocol, `wIndex=1` is a flag meaning "this is
121: a speed change" — it's NOT a USB interface number. But the Linux kernel's `usbfs`
122: validates control transfers in `check_ctrlrecip()`:
123: 
124: ```c
125: case USB_RECIP_INTERFACE:
126:     ret = findintfep(ps->dev, index);    // look for endpoint with address == wIndex
127:     if (ret < 0)
128:         ret = checkintf(ps, index);      // check if interface wIndex is claimed
129:     break;
130: ```
131: 
132: For `wIndex=1`: no endpoint with address 1 exists, interface 1 doesn't exist →
133: returns `-ENOENT`. The URB submission is rejected before it reaches the dongle.
134: 
135: ### What Was Tried
136: 
137: | Attempt                    | bRequestType | Kernel               | Dongle       | Result                       |
138: |----------------------------|--------------|----------------------|--------------|------------------------------|
139: | Original (Class+Interface) | `0x21`       | **REJECTS** (ENOENT) | Would accept | Kernel blocks it             |
140: | Class+Device               | `0x20`       | Passes               | **STALL**    | Dongle checks recipient bits |
141: | Vendor+Interface           | `0x41`       | Passes               | **STALL**    | Dongle checks type bits too  |
142: 
143: ### The Solution: Kernel Module
144: 
145: `kmod/ks959_speed.c` — a minimal kernel module that calls `usb_control_msg()` directly,
146: bypassing usbfs. It matches the dongle by VID/PID, changes the speed in its `probe()`
147: function, then returns `-ENODEV` so it doesn't permanently claim the device.
148: 
149: ```bash
150: sudo insmod kmod/ks959_speed.ko baud=115200
151: ```
152: 
153: **Limitation:** Can only be used once per USB plug cycle (returning `-ENODEV` prevents
154: re-probing until physical replug). This is fine — we only need one speed change per
155: session.
156: 
157: ## The RX Counter Desync Bug (Found & Fixed)
158: 
159: ### Symptom
160: 
161: After the kernel module set the speed and the bridge started, the first CMD_VERSION
162: response decoded to garbage (e.g., `8F818724` instead of `AAAA...`).
163: 
164: ### Root Cause
165: 
166: The kernel module's speed change operation leaves a stale byte in the dongle's buffer.
167: When the bridge starts and polls the dongle, it reads this stale byte (e.g., `0x8C`)
168: and increments the RX counter from `0x00` to `0x01`. When the real CMD_VERSION response
169: arrives, the de-obfuscation starts at counter `0x01` instead of `0x00` — off by one,
170: producing garbage.
171: 
172: Trace log showing the bug:
173: 
174: ```
175: RX poll raw_len=1 counter_before=0x00 raw_head=[8C]   ← stale byte!
176: RX decoded decoded_len=1 counter_after=0x01            ← counter now wrong
177: ...
178: RX poll raw_len=19 counter_before=0x01 raw_head=[D8, D7, ...]  ← version response
179: RX decoded decoded_len=19 counter_after=0x14           ← decoded with wrong counter
180: ```
181: 
182: ### Fix (two parts)
183: 
184: 1. **Stale data drain on startup:** Poll the dongle up to 10 times and discard any
185:    stale data, then reset the counter to 0.
186: 
187: 2. **Counter reset on baud rate change:** When dctool opens the PTY and sets the baud
188:    rate, the bridge resets the RX counter. This handles the case where stale bytes
189:    arrive between bridge startup and dctool connection.
190: 
191: ```rust
192: // In main.rs — drain stale data
193: for _ in 0..10 {
194:     let stale = dongle.poll_receive().await?;
195:     if stale.is_empty() { break; }
196: }
197: dongle.reset_rx_counter();
198: 
199: // In main.rs — reset on baud change
200: if new_baud != current_baud {
201:     dongle.reset_rx_counter();  // ← critical
202:     dongle.set_speed(new_baud).await;
203: }
204: ```
205: 
206: ## Known Issues & Risks
207: 
208: 1. **Donatello hibernates after ~1 minute** in PC mode. Everything must be staged
209:    before putting it in PC mode.
210: 
211: 2. **Kernel module single-use:** Can only be used once per USB plug cycle. If you
212:    need to re-run, unplug and replug the dongle.
213: 
214: 3. **RX counter desync:** If bytes are ever lost (USB error, buffer overflow), the
215:    counter gets permanently out of sync. Recovery requires dongle reset (USB
216:    re-enumeration). The drain+reset fix handles the known stale byte case, but
217:    there may be other edge cases.
218: 
219: 4. **IR link quality:** The version response decoded correctly in one test but the
220:    logbook response was garbage in the same session. This could be IR interference,
221:    weak signal, or the Donatello moving out of range.
222: 
223: 5. **The `crc` crate is unused:** CRC-CCITT is computed at compile time with `const fn`
224:    to match the Linux kernel's reflected polynomial (0x8408). The `crc` crate is in
225:    `Cargo.toml` but never imported.

(End of file - total 225 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/docs/TESTING.md"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/docs/TESTING.md</path>
<type>file</type>
<content>
1: # Testing & Debugging
2: 
3: ## Unit Tests (no hardware needed)
4: 
5: ```bash
6: cargo test                              # all 45 tests
7: cargo test -- --nocapture               # with stdout output
8: cargo test -p ks959-bridge usb_dongle   # only usb_dongle tests
9: ```
10: 
11: ## Debug Logging
12: 
13: ```bash
14: RUST_LOG=info  ./target/release/ks959-bridge   # default (startup, speed changes)
15: RUST_LOG=debug ./target/release/ks959-bridge   # protocol events (TX/RX, baud changes)
16: RUST_LOG=trace ./target/release/ks959-bridge   # hex dumps of every USB transfer
17: ```
18: 
19: ### What Each Level Shows
20: 
21: | Level   | What's Logged                                                                 |
22: |---------|-------------------------------------------------------------------------------|
23: | `info`  | Dongle found, PTY created, speed changes, baud rate changes                   |
24: | `debug` | PTY→dongle and dongle→PTY data lengths, TX completion, RX decoded lengths     |
25: | `trace` | Raw USB bytes (hex), obfuscation details, SIR frame state machine transitions |
26: 
27: ## Testing with dctool
28: 
29: ### The Command
30: 
31: ```bash
32: # Build the LD_PRELOAD shim first (see SETUP.md)
33: # The kernel module must be loaded and bridge running
34: 
35: LD_PRELOAD=/tmp/pty_modem_shim.so \
36: LD_LIBRARY_PATH=./reference/libdivecomputer/src/.libs \
37:   ./reference/libdivecomputer/examples/.libs/dctool \
38:   -v -l /tmp/cressi.log -f goa -m 4 download -o /tmp/dives.xml /tmp/cressi-irda
39: ```
40: 
41: **Important:** Use `.libs/dctool` (the real binary), NOT the libtool wrapper script.
42: The wrapper at `./reference/libdivecomputer/examples/dctool` doesn't pass arguments
43: correctly.
44: 
45: ### dctool Flags
46: 
47: | Flag       | Meaning                                              |
48: |------------|------------------------------------------------------|
49: | `-v`       | Verbose (shows INFO/ERROR logs from libdivecomputer) |
50: | `-l FILE`  | Log file for libdivecomputer internal logging        |
51: | `-f goa`   | Device family: `DC_FAMILY_CRESSI_GOA`                |
52: | `-m 4`     | Model number: 4 = Donatello                          |
53: | `-o FILE`  | Output file (XML format, default)                    |
54: | `download` | Command to download dives                            |
55: 
56: ### Family Name Mapping
57: 
58: | String     | Constant                    | Default Model |
59: |------------|-----------------------------|---------------|
60: | `goa`      | `DC_FAMILY_CRESSI_GOA`      | 2             |
61: | `leonardo` | `DC_FAMILY_CRESSI_LEONARDO` | 1             |
62: | `edy`      | `DC_FAMILY_CRESSI_EDY`      | 0x08          |
63: 
64: ### Other dctool Commands
65: 
66: ```bash
67: dctool list                                    # list supported devices
68: dctool version                                 # show version
69: dctool help download                           # help for download command
70: dctool -f goa -m 4 dump -o dump.bin /tmp/cressi-irda   # raw memory dump
71: dctool parse -i dives.xml                      # parse previously downloaded dives
72: ```
73: 
74: ## End-to-End Test Procedure
75: 
76: ```bash
77: # Terminal 1: Start bridge
78: sudo insmod kmod/ks959_speed.ko baud=115200
79: sudo ./target/release/ks959-bridge --baud 115200 --skip-speed-change
80: 
81: # Terminal 2: Run dctool (put Donatello in PC mode FIRST, then immediately:)
82: LD_PRELOAD=/tmp/pty_modem_shim.so \
83: LD_LIBRARY_PATH=./reference/libdivecomputer/src/.libs \
84:   ./reference/libdivecomputer/examples/.libs/dctool \
85:   -v -l /tmp/cressi.log -f goa -m 4 download -o /tmp/dives.xml /tmp/cressi-irda
86: ```
87: 
88: **Timing is critical:** The Donatello hibernates after ~1 minute in PC mode.
89: Have dctool ready to fire before putting the Donatello in PC mode.
90: 
91: ## Analyzing Logs
92: 
93: ### Successful CMD_VERSION Exchange
94: 
95: ```
96: # Bridge log (debug level):
97: PTY → dongle len=8                          # CMD_VERSION sent
98: TX complete total_len=8                     # USB transfer succeeded
99: RX decoded decoded_len=19 counter_after=0x14  # Version response decoded
100: dongle → PTY len=19                         # Forwarded to dctool
101: 
102: # dctool log:
103: INFO: Write: size=8, data=AAAAAA0000000055   # CMD_VERSION
104: INFO: Read: size=4, data=AAAAAA0B            # Response header
105: INFO: Read: size=15, data=211AAD000004310111000500753855  # Response data
106: Event: model=4 (0x00000004), firmware=305 (0x00000131), serial=44314 (0x0000ad1a)
107: ```
108: 
109: ### RX Counter Desync (garbage output)
110: 
111: ```
112: # Bridge log:
113: RX decoded decoded_len=19 counter_after=0x14  # But counter started at 0x01, not 0x00!
114: 
115: # dctool log:
116: INFO: Read: size=4, data=8F818724             # NOT a valid header (should be AAAAAA..)
117: ERROR: Unexpected answer header byte.
118: ```
119: 
120: **Fix:** Ensure stale data drain + counter reset are working. Check that the kernel
121: module was loaded and unloaded before starting the bridge.
122: 
123: ### Speed Change STALL (expected)
124: 
125: ```
126: # Bridge log:
127: USB speed change STALLED — if the ks959_speed kernel module already set this speed,
128:   IR comms will still work baud=115200 error=USB transfer error: endpoint STALL condition
129: ```
130: 
131: This is **normal** when using `--skip-speed-change`. The bridge attempts the usbfs
132: speed change, it STALLs, and the bridge continues assuming the kernel module already
133: set the speed.
134: 
135: ### Stale Byte Drain
136: 
137: ```
138: # Bridge log (debug level):
139: drained stale data from dongle len=1    # Stale byte found and discarded
140: resetting RX de-obfuscation counter     # Counter reset to 0
141: ```
142: 
143: ## Common Issues
144: 
145: | Symptom                            | Cause                                             | Fix                                                           |
146: |------------------------------------|---------------------------------------------------|---------------------------------------------------------------|
147: | `Device or resource busy`          | Another process or kernel module holds the dongle | `rmmod ks959_speed`, kill other bridge instances              |
148: | `No such file or directory` on PTY | Bridge not running or crashed                     | Restart bridge                                                |
149: | dctool gets garbage response       | RX counter desync                                 | Check stale byte drain, ensure counter reset on baud change   |
150: | dctool times out                   | Donatello hibernated                              | Put it back in PC mode, have dctool ready to fire immediately |
151: | `ENOTTY` on ioctl                  | PTY doesn't support modem control                 | Use `LD_PRELOAD=/tmp/pty_modem_shim.so`                       |
152: | Speed change STALL                 | Expected with usbfs                               | Use kernel module (`insmod ks959_speed.ko baud=115200`)       |
153: | Kernel module won't load           | Already loaded or device not found                | `lsmod \| grep ks959`, check `lsusb \| grep 07d0`             |
154: | Module won't re-probe              | Single-use per plug cycle                         | Unplug and replug the dongle                                  |
155: 
156: ## Useful One-Liners
157: 
158: ```bash
159: # Check if dongle is connected
160: lsusb | grep 07d0:4959
161: 
162: # Check if kernel module is loaded
163: lsmod | grep ks959
164: 
165: # Check if bridge is running
166: ps aux | grep ks959-bridge
167: 
168: # Check what's claiming the USB device
169: ls -la /sys/bus/usb/devices/*/idVendor 2>/dev/null | xargs grep -l "07d0"
170: 
171: # Kill bridge
172: pkill -f ks959-bridge
173: 
174: # Check PTY symlink
175: ls -la /tmp/cressi-irda
176: 
177: # Test PTY manually (in another terminal while bridge is running)
178: minicom -D /tmp/cressi-irda
179: # or
180: echo "test" > /tmp/cressi-irda
181: ```

(End of file - total 181 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/README.md"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/README.md</path>
<type>file</type>
<content>
1: # ks959-bridge
2: 
3: Bridge between the **Kingsun KS-959** IrDA USB dongle (VID `07d0`, PID `4959`) and
4: [libdivecomputer](https://libdivecomputer.org/) / [Subsurface](https://subsurface-divelog.org/) for downloading dives
5: from a **Cressi Donatello** dive computer over infrared.
6: 
7: ## Why
8: 
9: Linux removed the IrDA subsystem in kernel 4.17 (2018), and the old kernel driver doesn't work with the Donatello anyway
10: — it never completes the IrLAP handshake. This project bypasses the entire IrDA protocol stack, reverse-engineering
11: direct USB control transfers to the dongle so libdivecomputer can communicate with the dive computer.
12: 
13: ## Build
14: 
15: ```bash
16: cargo build --release
17: ```
18: 
19: ## Quick Start
20: 
21: ```bash
22: # 1. Build the kernel module (sets dongle to 115200 baud)
23: cd kmod && make && cd ..
24: 
25: # 2. Load kernel module
26: sudo insmod kmod/ks959_speed.ko baud=115200
27: 
28: # 3. Start bridge
29: sudo ./target/release/ks959-bridge --baud 115200 --skip-speed-change
30: 
31: # 4. In another terminal, run dctool against the PTY
32: LD_PRELOAD=/tmp/pty_modem_shim.so \
33: LD_LIBRARY_PATH=./reference/libdivecomputer/src/.libs \
34:   ./reference/libdivecomputer/examples/.libs/dctool \
35:   -v -l cressi.log -f goa -m 4 download -o dives.xml /tmp/cressi-irda
36: ```
37: 
38: The program creates a PTY and symlinks it to `/tmp/cressi-irda` (configurable with `--symlink`).
39: 
40: ### Options
41: 
42: | Flag | Default | Description |
43: |------|---------|-------------|
44: | `-s, --symlink PATH` | `/tmp/cressi-irda` | PTY symlink path |
45: | `-b, --baud RATE` | `9600` | Initial IrDA baud rate |
46: | `--skip-speed-change` | off | Skip USB speed change at startup (use with kernel module) |
47: | `--poll-ms MS` | `10` | USB RX polling interval |
48: | `--sir-framing` | off | Enable IrDA SIR framing (BOF/EOF/CRC) |
49: 
50: ### Logging
51: 
52: ```bash
53: RUST_LOG=debug ./target/release/ks959-bridge    # protocol events
54: RUST_LOG=trace ./target/release/ks959-bridge    # every USB transfer (hex dumps)
55: ```
56: 
57: ## Tests
58: 
59: ```bash
60: cargo test
61: ```
62: 
63: 45 tests cover SIR framing (CRC, state machine, round-trips), USB obfuscation/deobfuscation, and PTY bridge operations.
64: 
65: ## Permissions
66: 
67: The program needs USB access. Run as `root`, or set up a udev rule:
68: 
69: ```
70: # /etc/udev/rules.d/99-kingsun.rules
71: SUBSYSTEM=="usb", ATTR{idVendor}=="07d0", ATTR{idProduct}=="4959", MODE="0666"
72: ```
73: 
74: Then reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`
75: 
76: ## Documentation
77: 
78: - **[docs/SETUP.md](docs/SETUP.md)** — Build, environment, hardware, kernel module, LD_PRELOAD shim, alternative hardware
79: - **[docs/PROTOCOL.md](docs/PROTOCOL.md)** — USB protocol, IrDA SIR, Cressi Donatello wire format
80: - **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — Code structure, modules, event loop, dependencies
81: - **[docs/STATUS.md](docs/STATUS.md)** — What works, what doesn't, the speed change saga, known issues
82: - **[docs/TESTING.md](docs/TESTING.md)** — dctool commands, debugging, log analysis, common issues
83: 
84: ## Architecture
85: 
86: ```
87:   Subsurface / dctool
88:        |
89:   /tmp/cressi-irda (PTY slave — looks like a serial port)
90:        |
91:   +--- ks959-bridge ---+
92:   | pty_bridge         |  PTY master, non-blocking I/O, baud rate polling
93:   | usb_dongle         |  Kingsun KS-959: obfuscation, fragmentation, USB control transfers
94:   | sir_framing        |  IrDA SIR wrap/unwrap (optional, off by default)
95:   | main.rs            |  tokio select! loop: PTY ↔ USB bridge
96:   +--------------------+
97:        |
98:   USB control transfers (endpoint 0)
99:        |
100:   Kingsun KS-959 dongle
101:        |
102:   IrDA SIR (infrared)
103:        |
104:   Cressi Donatello
105: ```
106: 
107: ## Alternatives
108: 
109: If ks959-bridge doesn't work for your setup, other approaches exist:
110: 
111: | Approach | Cost | Status |
112: |----------|------|--------|
113: | ESP32 + TFDU4101 (hardware IrDA UART mode) | ~$8–15 | Proven by hb9eue |
114: | USB-to-Serial (FTDI/CH340) + TFDU4101 | ~$5–8 | Proven by Daniel Samarin |
115: | BLE transport (no IrDA hardware needed) | BLE adapter | Supported by libdivecomputer |
116: | Out-of-tree kernel modules (`github.com/cschramm/irda`) | $0 | Works for other IrDA devices; dead end for Donatello specifically |
117: 
118: See [docs/SETUP.md](docs/SETUP.md) for full details on each approach.
119: 
120: ## Source Code
121: 
122: - `reference/ks959-sir.c` — original Linux kernel driver (reverse-engineered USB protocol)
123: - `reference/irda/` — Linux IrDA subsystem (SIR framing, CRC)
124: - `reference/libdivecomputer/` — dive computer communication (Cressi Donatello protocol)
125: 
126: ## License
127: 
128: GPL-2.0 (matching the original ks959-sir.c kernel driver).

(End of file - total 128 lines)
</content>
```

---

## Assistant (Architect · Claude Fable 5 · 7.5s)

_Thinking:_

I should start by examining the codebase structure, looking at the source files, kernel module, and the project configuration to understand what I'm working with.

Now let me read the actual source code.

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/src/main.rs"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/src/main.rs</path>
<type>file</type>
<content>
1: mod pty_bridge;
2: mod sir_framing;
3: mod usb_dongle;
4: 
5: use std::path::PathBuf;
6: use std::time::Duration;
7: 
8: use anyhow::{Context, Result};
9: use clap::Parser;
10: use tokio::io::unix::AsyncFd;
11: use tokio::io::Interest;
12: use tracing::{debug, error, info, warn};
13: 
14: /// Userspace IrDA SIR driver for Kingsun KS-959 USB dongle.
15: ///
16: /// Bridges the dongle to a PTY so libdivecomputer/Subsurface can communicate
17: /// with a Cressi Donatello dive computer as if using a normal serial port.
18: #[derive(Parser, Debug)]
19: #[command(version, about)]
20: struct Args {
21:     /// Path for the PTY symlink (what Subsurface opens as a serial port).
22:     #[arg(short, long, default_value = "/tmp/cressi-irda")]
23:     symlink: PathBuf,
24: 
25:     /// Initial baud rate for the IrDA link.
26:     ///
27:     /// For rates other than 9600, the ks959_speed kernel module must be loaded
28:     /// first: `sudo insmod kmod/ks959_speed.ko baud=115200`.  The bridge will
29:     /// attempt the speed change via usbfs (Vendor+Interface workaround), but
30:     /// the dongle may STALL it — the kernel module is the reliable path.
31:     #[arg(short, long, default_value_t = 9600)]
32:     baud: u32,
33: 
34:     /// Skip the USB speed-change control transfer at startup.
35:     ///
36:     /// Use this when the ks959_speed kernel module has already set the dongle
37:     /// to the desired baud rate.  The bridge will trust that the dongle is
38:     /// at --baud and skip the (likely-to-STALL) usbfs speed change.
39:     #[arg(long)]
40:     skip_speed_change: bool,
41: 
42:     /// USB RX polling interval in milliseconds.
43:     #[arg(long, default_value_t = 10)]
44:     poll_ms: u64,
45: 
46:     /// Enable IrDA SIR framing (BOF/EOF/escape/CRC wrapping).
47:     /// Not needed for Cressi Donatello which uses raw serial over IrDA SIR.
48:     #[arg(long)]
49:     sir_framing: bool,
50: 
51:     /// Number of extra BOFs prepended in SIR mode (only with --sir-framing).
52:     #[arg(long, default_value_t = 10)]
53:     extra_bofs: usize,
54: }
55: 
56: #[tokio::main(flavor = "current_thread")]
57: async fn main() -> Result<()> {
58:     // Initialize tracing (set RUST_LOG=debug or RUST_LOG=trace for more).
59:     tracing_subscriber::fmt()
60:         .with_env_filter(
61:             tracing_subscriber::EnvFilter::try_from_default_env()
62:                 .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
63:         )
64:         .init();
65: 
66:     let args = Args::parse();
67: 
68:     info!(
69:         symlink = %args.symlink.display(),
70:         baud = args.baud,
71:         poll_ms = args.poll_ms,
72:         sir_framing = args.sir_framing,
73:         "starting ks959-bridge"
74:     );
75: 
76:     // --- Open the USB dongle ---
77:     let mut dongle =
78:         usb_dongle::KingsunDongle::open().context("failed to open Kingsun KS-959 dongle")?;
79: 
80:     // --- Set initial speed ---
81:     // The dongle defaults to 9600 baud.  For higher rates (e.g. 115200 for
82:     // Cressi Donatello), the ks959_speed kernel module should be loaded first
83:     // because usbfs check_ctrlrecip blocks the wIndex=1 speed-change transfer.
84:     // The bridge tries the usbfs path as a fallback (Vendor+Interface bRequestType
85:     // 0x41) but the dongle typically STALLs it.
86:     let mut current_baud = if args.skip_speed_change {
87:         // Kernel module already set the speed — trust it.
88:         info!(
89:             baud = args.baud,
90:             "skipping USB speed change (--skip-speed-change); assuming dongle is already at target baud rate"
91:         );
92:         args.baud
93:     } else if args.baud != 9600 {
94:         // Try usbfs speed change.  Warn on failure instead of aborting.
95:         match dongle.set_speed(args.baud).await {
96:             Ok(()) => {
97:                 info!(baud = args.baud, "dongle speed changed via usbfs");
98:                 args.baud
99:             }
100:             Err(e) => {
101:                 warn!(
102:                     baud = args.baud,
103:                     error = %e,
104:                     "USB speed change STALLED (expected — dongle needs bRequestType=0x21). \
105:                      Load the ks959_speed kernel module first: \
106:                      sudo insmod kmod/ks959_speed.ko baud={}",
107:                     args.baud
108:                 );
109:                 // Assume the dongle is at the requested speed (e.g., module
110:                 // loaded externally).  If it's actually at 9600, IR comms
111:                 // will fail — but at least the bridge starts.
112:                 args.baud
113:             }
114:         }
115:     } else {
116:         // Default 9600 — dongle powers on at 9600, no change needed.
117:         9600
118:     };
119: 
120:     // --- Drain stale data from the dongle ---
121:     // The kernel module's speed change may leave stale bytes in the dongle's
122:     // buffer.  Drain them before starting the main loop so the RX counter
123:     // starts clean.
124:     for _ in 0..10 {
125:         let stale = dongle.poll_receive().await?;
126:         if stale.is_empty() {
127:             break;
128:         }
129:         debug!(len = stale.len(), "drained stale data from dongle");
130:     }
131:     dongle.reset_rx_counter();
132: 
133:     // --- Create PTY bridge ---
134:     let mut pty =
135:         pty_bridge::PtyBridge::new(&args.symlink).context("failed to create PTY bridge")?;
136: 
137:     info!(
138:         "ready — open {} in Subsurface (or: minicom -D {})",
139:         args.symlink.display(),
140:         args.symlink.display()
141:     );
142: 
143:     // --- SIR framing state (if enabled) ---
144:     let mut unwrapper = sir_framing::SirUnwrapper::new();
145: 
146:     // --- Wrap the PTY master fd for async I/O ---
147:     let async_master = AsyncFd::with_interest(pty.master_raw_fd(), Interest::READABLE)
148:         .context("failed to create AsyncFd for PTY master")?;
149: 
150:     // We don't take ownership of the fd — AsyncFd<RawFd> doesn't close it.
151:     // The PtyBridge still owns the OwnedFd.
152: 
153:     // --- RX poll timer ---
154:     let mut poll_interval = tokio::time::interval(Duration::from_millis(args.poll_ms));
155:     poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
156: 
157:     // --- Signal handler ---
158:     let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
159:         .context("failed to register SIGINT handler")?;
160:     let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
161:         .context("failed to register SIGTERM handler")?;
162: 
163:     // --- Main event loop ---
164:     let mut pty_buf = [0u8; 4096];
165: 
166:     loop {
167:         tokio::select! {
168:             // --- PTY master readable: data from the application ---
169:             readable = async_master.readable() => {
170:                 let mut guard = readable.context("AsyncFd readable error")?;
171: 
172:                 match pty.read(&mut pty_buf) {
173:                     Ok(0) => {
174:                         info!("PTY slave closed (EOF), exiting");
175:                         break;
176:                     }
177:                     Ok(n) => {
178:                         debug!(len = n, "PTY → dongle");
179: 
180:                         // Check for baud rate change before forwarding.
181:                         if let Some(new_baud) = pty.check_baud_rate_change()? {
182:                             if new_baud != current_baud {
183:                                 // A baud rate change means a new client
184:                                 // (dctool/Subsurface) just opened the PTY.
185:                                 // Reset the RX counter to flush stale bytes
186:                                 // that may have desynchronized it.
187:                                 dongle.reset_rx_counter();
188: 
189:                                 match dongle.set_speed(new_baud).await {
190:                                     Ok(()) => {
191:                                         info!(baud = new_baud, "dongle speed changed");
192:                                         current_baud = new_baud;
193:                                     }
194:                                     Err(e) => {
195:                                         // The dongle STALLs the Vendor+Interface
196:                                         // workaround.  If the kernel module already
197:                                         // set the correct speed, this is harmless.
198:                                         warn!(
199:                                             baud = new_baud,
200:                                             error = %e,
201:                                             "USB speed change STALLED — if the ks959_speed kernel \
202:                                              module already set this speed, IR comms will still work"
203:                                         );
204:                                         current_baud = new_baud;
205:                                     }
206:                                 }
207:                             }
208:                         }
209: 
210:                         // Forward to dongle (with optional SIR wrapping).
211:                         let tx_data = if args.sir_framing {
212:                             sir_framing::wrap_frame(&pty_buf[..n], args.extra_bofs)
213:                         } else {
214:                             pty_buf[..n].to_vec()
215:                         };
216: 
217:                         if let Err(e) = dongle.send(&tx_data).await {
218:                             error!(error = %e, "USB TX failed");
219:                         }
220:                     }
221:                     Err(e) => {
222:                         // EAGAIN is normal — clear readiness and retry.
223:                         if is_would_block(&e) {
224:                             guard.clear_ready();
225:                         } else {
226:                             error!(error = %e, "PTY read error");
227:                             break;
228:                         }
229:                     }
230:                 }
231:             }
232: 
233:             // --- USB RX poll tick ---
234:             _ = poll_interval.tick() => {
235:                 match dongle.poll_receive().await {
236:                     Ok(data) if data.is_empty() => {
237:                         // No data — normal.
238:                     }
239:                     Ok(data) => {
240:                         debug!(len = data.len(), "dongle → PTY");
241: 
242:                         let rx_data = if args.sir_framing {
243:                             // Feed through SIR unwrapper, concatenate payloads.
244:                             let mut frames = Vec::new();
245:                             for frame in &unwrapper.process_bytes(&data) {
246:                                 frames.extend_from_slice(frame);
247:                             }
248:                             frames
249:                         } else {
250:                             data
251:                         };
252: 
253:                         if !rx_data.is_empty() {
254:                             match pty.write(&rx_data) {
255:                                 Ok(n) if n < rx_data.len() => {
256:                                     warn!(
257:                                         written = n,
258:                                         total = rx_data.len(),
259:                                         "partial PTY write (application not reading fast enough)"
260:                                     );
261:                                 }
262:                                 Ok(_) => {}
263:                                 Err(e) => {
264:                                     if is_would_block(&e) {
265:                                         warn!("PTY write would block — dropping {} bytes", rx_data.len());
266:                                     } else {
267:                                         error!(error = %e, "PTY write error");
268:                                         break;
269:                                     }
270:                                 }
271:                             }
272:                         }
273:                     }
274:                     Err(e) => {
275:                         error!(error = %e, "USB RX poll failed");
276:                         break;
277:                     }
278:                 }
279:             }
280: 
281:             // --- Clean shutdown on signal ---
282:             _ = sigint.recv() => {
283:                 info!("received SIGINT, shutting down");
284:                 break;
285:             }
286:             _ = sigterm.recv() => {
287:                 info!("received SIGTERM, shutting down");
288:                 break;
289:             }
290:         }
291:     }
292: 
293:     // PtyBridge::drop cleans up the symlink.
294:     info!("goodbye");
295:     Ok(())
296: }
297: 
298: /// Check if a PtyError wraps an EAGAIN/EWOULDBLOCK.
299: fn is_would_block(e: &pty_bridge::PtyError) -> bool {
300:     match e {
301:         pty_bridge::PtyError::Nix(nix::Error::EAGAIN) => true,
302:         pty_bridge::PtyError::Io(io) => io.kind() == std::io::ErrorKind::WouldBlock,
303:         _ => false,
304:     }
305: }

(End of file - total 305 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/src/usb_dongle.rs"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/src/usb_dongle.rs</path>
<type>file</type>
<content>
1: // Kingsun KS-959 USB-to-IrDA dongle driver.
2: //
3: // Implements the proprietary USB control-transfer protocol reverse-engineered
4: // in the Linux kernel driver `ks959-sir.c`.  All communication uses endpoint 0
5: // control transfers — the dongle's interrupt endpoint is a dummy.
6: //
7: // TX: obfuscate with XOR mask derived from "wangshuofei19710", pad to
8: //     8-byte alignment + 16 overhead, fragment at 240 bytes.
9: // RX: poll via control-IN, de-obfuscate with a persistent wrapping counter
10: //     XOR'd with 0x55, skip the garbage byte every 255 real bytes.
11: // Speed: 8-byte packed struct via control-OUT to wIndex=0x0001.
12: 
13: use thiserror::Error;
14: use tracing::{debug, info, trace};
15: 
16: // ---------------------------------------------------------------------------
17: // Constants
18: // ---------------------------------------------------------------------------
19: 
20: /// USB Vendor ID for Kingsun.
21: const VENDOR_ID: u16 = 0x07D0;
22: /// USB Product ID for the KS-959 dongle.
23: const PRODUCT_ID: u16 = 0x4959;
24: 
25: /// Control request code used for both TX data and speed change.
26: const REQ_SEND: u8 = 0x09;
27: /// Control request code used for RX polling.
28: const REQ_RECV: u8 = 0x01;
29: 
30: /// Maximum buffer the dongle accepts in one transfer.
31: const SND_PACKET_SIZE: usize = 256;
32: /// RX polling buffer size (matches kernel's KINGSUN_RCV_FIFO_SIZE).
33: const RCV_FIFO_SIZE: u16 = 2048;
34: 
35: /// Maximum cleartext bytes per TX fragment: `(256 & ~7) - 16 = 240`.
36: const MAX_CLEARTEXT_PER_FRAGMENT: usize = (SND_PACKET_SIZE & !0x07) - 0x10;
37: 
38: /// The obfuscation lookup string — yes, really.
39: const LOOKUP: &[u8; 16] = b"wangshuofei19710";
40: 
41: /// Supported baud rates.
42: const SUPPORTED_SPEEDS: &[u32] = &[2400, 9600, 19200, 38400, 57600, 115200];
43: 
44: /// Flag byte for 8 data bits in the speed-change struct.
45: const KS_DATA_8_BITS: u8 = 0x03;
46: 
47: // ---------------------------------------------------------------------------
48: // Errors
49: // ---------------------------------------------------------------------------
50: 
51: /// Errors from Kingsun dongle operations.
52: #[derive(Debug, Error)]
53: pub enum DongleError {
54:     /// No Kingsun KS-959 dongle found on the USB bus.
55:     #[error(
56:         "no Kingsun KS-959 dongle found (VID=0x{:04X} PID=0x{:04X})",
57:         VENDOR_ID,
58:         PRODUCT_ID
59:     )]
60:     NotFound,
61: 
62:     /// USB operation failed.
63:     #[error("USB error: {0}")]
64:     Usb(#[from] nusb::Error),
65: 
66:     /// A USB transfer completed with a non-success status.
67:     #[error("USB transfer error: {0}")]
68:     Transfer(#[from] nusb::transfer::TransferError),
69: 
70:     /// Requested baud rate is not supported by the dongle.
71:     #[error("unsupported baud rate: {0} (supported: 2400..115200)")]
72:     UnsupportedSpeed(u32),
73: }
74: 
75: // ---------------------------------------------------------------------------
76: // TX obfuscation
77: // ---------------------------------------------------------------------------
78: 
79: /// Obfuscate and pad a cleartext buffer for transmission to the dongle.
80: ///
81: /// Returns the padded, obfuscated buffer ready to be sent as the control
82: /// transfer payload.  The cleartext length is encoded in `wValue` of the
83: /// setup packet (handled by the caller).
84: ///
85: /// Algorithm (from `ks959-sir.c`):
86: /// - Padded length = `((len + 7) & ~7) + 16`
87: /// - XOR mask = `LOOKUP[(len & 0x0F) ^ 0x06] ^ 0x55`
88: /// - Each cleartext byte is XOR'd with the mask
89: /// - Padding bytes are zero
90: fn obfuscate_tx_buffer(cleartext: &[u8]) -> Vec<u8> {
91:     let len = cleartext.len();
92:     let padded_len = ((len + 7) & !0x07) + 0x10;
93:     let xor_mask = LOOKUP[(len & 0x0F) ^ 0x06] ^ 0x55;
94: 
95:     let mut out = vec![0u8; padded_len];
96:     for (i, &b) in cleartext.iter().enumerate() {
97:         out[i] = b ^ xor_mask;
98:     }
99: 
100:     trace!(
101:         cleartext_len = len,
102:         padded_len,
103:         xor_mask = format_args!("0x{:02X}", xor_mask),
104:         "obfuscate_tx"
105:     );
106:     out
107: }
108: 
109: // ---------------------------------------------------------------------------
110: // RX de-obfuscation
111: // ---------------------------------------------------------------------------
112: 
113: /// De-obfuscate raw bytes received from the dongle.
114: ///
115: /// `counter` is the persistent session counter (starts at 0, wraps as `u8`).
116: /// It is mutated in place and must be preserved across calls.
117: ///
118: /// For each raw byte:
119: ///   1. Increment counter (wrapping).
120: ///   2. Decoded = raw XOR counter XOR 0x55.
121: ///   3. If counter wrapped to 0: skip this byte (garbage).
122: ///   4. Otherwise: emit decoded byte.
123: fn deobfuscate_rx_buffer(raw: &[u8], counter: &mut u8) -> Vec<u8> {
124:     let mut out = Vec::with_capacity(raw.len());
125:     for &b in raw {
126:         *counter = counter.wrapping_add(1);
127:         let decoded = b ^ *counter ^ 0x55;
128:         if *counter != 0 {
129:             out.push(decoded);
130:         } else {
131:             trace!(
132:                 raw_byte = format_args!("0x{:02X}", b),
133:                 decoded = format_args!("0x{:02X}", decoded),
134:                 "skipping garbage byte at counter wrap"
135:             );
136:         }
137:     }
138:     out
139: }
140: 
141: // ---------------------------------------------------------------------------
142: // Speed-change payload
143: // ---------------------------------------------------------------------------
144: 
145: /// Build the 8-byte speed-change payload.
146: ///
147: /// Layout (packed, little-endian):
148: ///   `[baudrate_le32, flags, 0, 0, 0]`
149: /// where `flags = KS_DATA_8_BITS (0x03)`.
150: fn speed_payload(baud: u32) -> [u8; 8] {
151:     let le = baud.to_le_bytes();
152:     [le[0], le[1], le[2], le[3], KS_DATA_8_BITS, 0, 0, 0]
153: }
154: 
155: // ---------------------------------------------------------------------------
156: // Dongle struct
157: // ---------------------------------------------------------------------------
158: 
159: /// Kingsun KS-959 USB-to-IrDA dongle handle.
160: ///
161: /// All I/O is via async control transfers on endpoint 0.
162: pub struct KingsunDongle {
163:     /// Claimed USB interface (interface 0).
164:     interface: nusb::Interface,
165:     /// Persistent RX de-obfuscation counter.
166:     rx_counter: u8,
167: }
168: 
169: impl KingsunDongle {
170:     /// Find and open the first Kingsun KS-959 dongle on the USB bus.
171:     ///
172:     /// Detaches any kernel driver and claims interface 0.
173:     pub fn open() -> Result<Self, DongleError> {
174:         let device_info = nusb::list_devices()?
175:             .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
176:             .ok_or(DongleError::NotFound)?;
177: 
178:         info!(
179:             bus = device_info.bus_number(),
180:             addr = device_info.device_address(),
181:             "found Kingsun KS-959 dongle"
182:         );
183: 
184:         let device = device_info.open()?;
185: 
186:         // Try to detach kernel driver first (may fail if none attached — that's fine).
187:         let interface = match device.detach_and_claim_interface(0) {
188:             Ok(iface) => {
189:                 info!("detached kernel driver and claimed interface 0");
190:                 iface
191:             }
192:             Err(_) => {
193:                 debug!("no kernel driver to detach, claiming interface 0 directly");
194:                 device.claim_interface(0)?
195:             }
196:         };
197: 
198:         Ok(Self {
199:             interface,
200:             rx_counter: 0,
201:         })
202:     }
203: 
204:     /// Change the IrDA link baud rate.
205:     ///
206:     /// Sends a speed-change control transfer to the dongle.
207:     pub async fn set_speed(&self, baud: u32) -> Result<(), DongleError> {
208:         if !SUPPORTED_SPEEDS.contains(&baud) {
209:             return Err(DongleError::UnsupportedSpeed(baud));
210:         }
211: 
212:         let payload = speed_payload(baud);
213:         trace!(
214:             baud,
215:             payload = format_args!("{:02X?}", payload),
216:             "speed change control transfer"
217:         );
218: 
219:         // The kernel driver (ks959-sir.c) uses bRequestType=0x21 (Class +
220:         // Interface + OUT) with wIndex=0x0001 for speed changes.  But the
221:         // kernel's usbfs check_ctrlrecip() rejects USB_RECIP_INTERFACE when
222:         // wIndex doesn't match an existing interface number — and this device
223:         // only has interface 0.  wIndex=1 is a protocol flag, not an interface
224:         // number, but the kernel doesn't know that.  The original kernel driver
225:         // bypasses this because it submits URBs directly, not through usbfs.
226:         //
227:         // Workaround: use ControlType::Vendor (bRequestType=0x41 instead of
228:         // 0x21).  The kernel's check_ctrlrecip has an early return for
229:         // USB_TYPE_VENDOR that skips all recipient/wIndex validation.  The
230:         // dongle firmware is expected to dispatch on the recipient bits (0x01 =
231:         // Interface) and wIndex, ignoring the Class-vs-Vendor distinction in
232:         // the type field.  Confirmed: Recipient::Device (0x20) was STALL'd,
233:         // meaning the firmware checks the recipient — but may not check the
234:         // type.
235:         let completion = self
236:             .interface
237:             .control_out(nusb::transfer::ControlOut {
238:                 control_type: nusb::transfer::ControlType::Vendor,
239:                 recipient: nusb::transfer::Recipient::Interface,
240:                 request: REQ_SEND,
241:                 value: 0x0200,
242:                 index: 0x0001,
243:                 data: &payload,
244:             })
245:             .await;
246:         completion.status?;
247: 
248:         info!(baud, "dongle speed changed");
249:         Ok(())
250:     }
251: 
252:     /// Reset the RX de-obfuscation counter to zero.
253:     ///
254:     /// Call this when a new client session starts (e.g., when the PTY slave
255:     /// is opened by dctool/Subsurface) to flush any stale bytes that may
256:     /// have desynchronized the counter.
257:     pub fn reset_rx_counter(&mut self) {
258:         if self.rx_counter != 0 {
259:             info!(
260:                 old_counter = format_args!("0x{:02X}", self.rx_counter),
261:                 "resetting RX de-obfuscation counter"
262:             );
263:             self.rx_counter = 0;
264:         }
265:     }
266: 
267:     /// Send data to the dongle (TX direction: host → IrDA link).
268:     ///
269:     /// Handles obfuscation, padding, and fragmentation. Data larger than
270:     /// 240 bytes is automatically split into multiple control transfers.
271:     pub async fn send(&self, data: &[u8]) -> Result<(), DongleError> {
272:         if data.is_empty() {
273:             return Ok(());
274:         }
275: 
276:         for (frag_idx, chunk) in data.chunks(MAX_CLEARTEXT_PER_FRAGMENT).enumerate() {
277:             let obfuscated = obfuscate_tx_buffer(chunk);
278: 
279:             trace!(
280:                 fragment = frag_idx,
281:                 cleartext_len = chunk.len(),
282:                 padded_len = obfuscated.len(),
283:                 data = format_args!("{:02X?}", &chunk[..chunk.len().min(32)]),
284:                 "TX fragment"
285:             );
286: 
287:             let completion = self
288:                 .interface
289:                 .control_out(nusb::transfer::ControlOut {
290:                     control_type: nusb::transfer::ControlType::Class,
291:                     recipient: nusb::transfer::Recipient::Interface,
292:                     request: REQ_SEND,
293:                     value: chunk.len() as u16,
294:                     index: 0x0000,
295:                     data: &obfuscated,
296:                 })
297:                 .await;
298:             completion.status?;
299:         }
300: 
301:         debug!(total_len = data.len(), "TX complete");
302:         Ok(())
303:     }
304: 
305:     /// Poll the dongle for received data (RX direction: IrDA link → host).
306:     ///
307:     /// Returns decoded bytes with the obfuscation removed. Returns an empty
308:     /// `Vec` if no data is available. The internal de-obfuscation counter
309:     /// persists across calls for the lifetime of the session.
310:     pub async fn poll_receive(&mut self) -> Result<Vec<u8>, DongleError> {
311:         let completion = self
312:             .interface
313:             .control_in(nusb::transfer::ControlIn {
314:                 control_type: nusb::transfer::ControlType::Class,
315:                 recipient: nusb::transfer::Recipient::Interface,
316:                 request: REQ_RECV,
317:                 value: 0x0200,
318:                 index: 0x0000,
319:                 length: RCV_FIFO_SIZE,
320:             })
321:             .await;
322:         completion.status?;
323: 
324:         let raw = completion.data;
325:         if raw.is_empty() {
326:             return Ok(Vec::new());
327:         }
328: 
329:         trace!(
330:             raw_len = raw.len(),
331:             counter_before = format_args!("0x{:02X}", self.rx_counter),
332:             raw_head = format_args!("{:02X?}", &raw[..raw.len().min(32)]),
333:             "RX poll"
334:         );
335: 
336:         let decoded = deobfuscate_rx_buffer(&raw, &mut self.rx_counter);
337: 
338:         if !decoded.is_empty() {
339:             debug!(
340:                 decoded_len = decoded.len(),
341:                 counter_after = format_args!("0x{:02X}", self.rx_counter),
342:                 "RX decoded"
343:             );
344:         }
345: 
346:         Ok(decoded)
347:     }
348: }
349: 
350: // ---------------------------------------------------------------------------
351: // Tests
352: // ---------------------------------------------------------------------------
353: 
354: #[cfg(test)]
355: mod tests {
356:     use super::*;
357: 
358:     // -- TX obfuscation tests -----------------------------------------------
359: 
360:     /// Hand-calculated obfuscation for a 3-byte cleartext.
361:     ///
362:     /// cleartext = [0x01, 0x02, 0x03], len = 3
363:     /// padded_len = ((3+7) & ~7) + 16 = 8 + 16 = 24
364:     /// mask_index = (3 & 0x0F) ^ 0x06 = 3 ^ 6 = 5
365:     /// LOOKUP[5] = b'h' = 0x68
366:     /// xor_mask  = 0x68 ^ 0x55 = 0x3D
367:     /// obfuscated[0] = 0x01 ^ 0x3D = 0x3C
368:     /// obfuscated[1] = 0x02 ^ 0x3D = 0x3F
369:     /// obfuscated[2] = 0x03 ^ 0x3D = 0x3E
370:     /// obfuscated[3..24] = 0x00 (padding)
371:     #[test]
372:     fn obfuscate_known_vector() {
373:         let cleartext = [0x01u8, 0x02, 0x03];
374:         let out = obfuscate_tx_buffer(&cleartext);
375: 
376:         assert_eq!(out.len(), 24);
377:         assert_eq!(out[0], 0x3C);
378:         assert_eq!(out[1], 0x3F);
379:         assert_eq!(out[2], 0x3E);
380:         // All padding is zero
381:         for &b in &out[3..] {
382:             assert_eq!(b, 0x00, "padding byte must be zero");
383:         }
384:     }
385: 
386:     /// Verify padded length formula for various cleartext sizes.
387:     #[test]
388:     fn obfuscate_padded_length() {
389:         for len in [1, 2, 7, 8, 9, 16, 100, 200, MAX_CLEARTEXT_PER_FRAGMENT] {
390:             let data = vec![0xAA; len];
391:             let out = obfuscate_tx_buffer(&data);
392:             let expected = ((len + 7) & !0x07) + 0x10;
393:             assert_eq!(out.len(), expected, "padded_len for cleartext_len={len}");
394:         }
395:     }
396: 
397:     /// Padding bytes beyond the cleartext must be zero (not XOR'd).
398:     #[test]
399:     fn obfuscate_padding_is_zero() {
400:         let cleartext = vec![0xFF; 5];
401:         let out = obfuscate_tx_buffer(&cleartext);
402:         for (i, &b) in out.iter().enumerate().skip(5) {
403:             assert_eq!(b, 0x00, "padding byte at index {i} must be zero");
404:         }
405:     }
406: 
407:     /// Obfuscation is invertible: XOR with the same mask recovers cleartext.
408:     #[test]
409:     fn obfuscate_invertible() {
410:         let cleartext = b"Hello IrDA dongle!";
411:         let len = cleartext.len();
412:         let out = obfuscate_tx_buffer(cleartext);
413:         let xor_mask = LOOKUP[(len & 0x0F) ^ 0x06] ^ 0x55;
414:         for (i, &orig) in cleartext.iter().enumerate() {
415:             assert_eq!(out[i] ^ xor_mask, orig, "byte {i} round-trip failed");
416:         }
417:     }
418: 
419:     // -- RX de-obfuscation tests --------------------------------------------
420: 
421:     /// Basic de-obfuscation: encode known cleartext as the dongle would,
422:     /// then decode and verify.
423:     #[test]
424:     fn deobfuscate_basic() {
425:         let cleartext = b"test";
426:         let mut counter: u8 = 0;
427: 
428:         // Simulate what the dongle produces: raw = cleartext ^ (counter+1) ^ 0x55
429:         let mut raw = Vec::new();
430:         let mut sim_counter: u8 = 0;
431:         for &b in cleartext.iter() {
432:             sim_counter = sim_counter.wrapping_add(1);
433:             raw.push(b ^ sim_counter ^ 0x55);
434:         }
435: 
436:         let decoded = deobfuscate_rx_buffer(&raw, &mut counter);
437:         assert_eq!(&decoded, cleartext);
438:         assert_eq!(counter, 4);
439:     }
440: 
441:     /// Counter wrap: when counter goes from 0xFF to 0x00, the byte is garbage
442:     /// and must be skipped.
443:     #[test]
444:     fn deobfuscate_counter_wrap() {
445:         // Start at 0xFD so we process bytes at counters 0xFE, 0xFF, 0x00, 0x01
446:         let cleartext = [0x41u8, 0x42, 0x99, 0x43]; // 0x99 is arbitrary (will be skipped)
447:         let mut counter: u8 = 0xFD;
448: 
449:         // Simulate dongle encoding
450:         let mut raw = Vec::new();
451:         let mut sim_counter: u8 = 0xFD;
452:         for &b in &cleartext {
453:             sim_counter = sim_counter.wrapping_add(1);
454:             raw.push(b ^ sim_counter ^ 0x55);
455:         }
456: 
457:         let decoded = deobfuscate_rx_buffer(&raw, &mut counter);
458:         // Byte at counter=0x00 (third byte) is skipped
459:         assert_eq!(decoded.len(), 3, "garbage byte must be skipped");
460:         assert_eq!(decoded[0], 0x41); // counter=0xFE
461:         assert_eq!(decoded[1], 0x42); // counter=0xFF
462:         assert_eq!(decoded[2], 0x43); // counter=0x01
463:         assert_eq!(counter, 0x01);
464:     }
465: 
466:     /// Counter persists across multiple calls.
467:     #[test]
468:     fn deobfuscate_counter_persistence() {
469:         let mut counter: u8 = 0;
470: 
471:         // First call: 3 bytes → counter ends at 3
472:         let raw1 = {
473:             let mut r = Vec::new();
474:             let mut c: u8 = 0;
475:             for &b in &[0x10u8, 0x20, 0x30] {
476:                 c = c.wrapping_add(1);
477:                 r.push(b ^ c ^ 0x55);
478:             }
479:             r
480:         };
481:         let d1 = deobfuscate_rx_buffer(&raw1, &mut counter);
482:         assert_eq!(counter, 3);
483:         assert_eq!(d1, vec![0x10, 0x20, 0x30]);
484: 
485:         // Second call: 2 bytes → counter continues from 3, ends at 5
486:         let raw2 = {
487:             let mut r = Vec::new();
488:             let mut c: u8 = 3; // continue from where we left off
489:             for &b in &[0x40u8, 0x50] {
490:                 c = c.wrapping_add(1);
491:                 r.push(b ^ c ^ 0x55);
492:             }
493:             r
494:         };
495:         let d2 = deobfuscate_rx_buffer(&raw2, &mut counter);
496:         assert_eq!(counter, 5);
497:         assert_eq!(d2, vec![0x40, 0x50]);
498:     }
499: 
500:     /// Empty raw input produces empty output without touching the counter.
501:     #[test]
502:     fn deobfuscate_empty() {
503:         let mut counter: u8 = 42;
504:         let decoded = deobfuscate_rx_buffer(&[], &mut counter);
505:         assert!(decoded.is_empty());
506:         assert_eq!(counter, 42);
507:     }
508: 
509:     // -- Fragmentation tests ------------------------------------------------
510: 
511:     /// Verify that data is split into correct fragment sizes.
512:     #[test]
513:     fn fragmentation_sizes() {
514:         let sizes: Vec<usize> = (0..500usize)
515:             .collect::<Vec<_>>()
516:             .chunks(MAX_CLEARTEXT_PER_FRAGMENT)
517:             .map(|c| c.len())
518:             .collect();
519:         assert_eq!(sizes, vec![240, 240, 20]);
520:     }
521: 
522:     /// A single fragment (≤240) is not split.
523:     #[test]
524:     fn fragmentation_single() {
525:         let sizes: Vec<usize> = (0..240usize)
526:             .collect::<Vec<_>>()
527:             .chunks(MAX_CLEARTEXT_PER_FRAGMENT)
528:             .map(|c| c.len())
529:             .collect();
530:         assert_eq!(sizes, vec![240]);
531:     }
532: 
533:     // -- Speed-change payload tests -----------------------------------------
534: 
535:     /// Verify the 8-byte payload for 115200 baud.
536:     ///
537:     /// 115200 = 0x0001C200
538:     /// LE bytes: [0x00, 0xC2, 0x01, 0x00]
539:     /// flags: 0x03 (8 data bits)
540:     /// reserved: [0x00, 0x00, 0x00]
541:     #[test]
542:     fn speed_payload_115200() {
543:         let p = speed_payload(115200);
544:         assert_eq!(p, [0x00, 0xC2, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00]);
545:     }
546: 
547:     /// Verify the payload for 9600 baud.
548:     ///
549:     /// 9600 = 0x00002580
550:     /// LE bytes: [0x80, 0x25, 0x00, 0x00]
551:     #[test]
552:     fn speed_payload_9600() {
553:         let p = speed_payload(9600);
554:         assert_eq!(p, [0x80, 0x25, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00]);
555:     }
556: 
557:     /// Unsupported speeds should be rejected.
558:     #[test]
559:     fn unsupported_speed() {
560:         assert!(!SUPPORTED_SPEEDS.contains(&1200));
561:         assert!(!SUPPORTED_SPEEDS.contains(&0));
562:         assert!(!SUPPORTED_SPEEDS.contains(&230400));
563:         // The actual error is returned by set_speed() which needs a device,
564:         // so we just verify the lookup here.
565:         assert!(SUPPORTED_SPEEDS.contains(&9600));
566:         assert!(SUPPORTED_SPEEDS.contains(&115200));
567:     }
568: 
569:     /// MAX_CLEARTEXT_PER_FRAGMENT must be 240.
570:     #[test]
571:     fn max_fragment_size() {
572:         assert_eq!(MAX_CLEARTEXT_PER_FRAGMENT, 240);
573:     }
574: }

(End of file - total 574 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/src/pty_bridge.rs"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/src/pty_bridge.rs</path>
<type>file</type>
<content>
1: // PTY bridge: creates a pseudo-terminal pair for serial port emulation.
2: //
3: // The slave side (symlinked to a user-chosen path like `/tmp/cressi-irda`)
4: // is opened by libdivecomputer/Subsurface as a normal serial port.
5: // The master side is read/written by our event loop.
6: //
7: // Baud rate detection: the event loop calls `slave_baud_rate()` before
8: // forwarding data to the dongle.  TIOCPKT was considered but it does NOT
9: // generate TIOCPKT_IOCTL for plain `tcsetattr()` on Linux — only for
10: // EXTPROC and IXON/IXOFF changes (see `pty_set_termios` in the kernel).
11: // Since the baud rate changes once at session start, polling on data
12: // arrival is more than sufficient.
13: 
14: use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
15: use std::path::{Path, PathBuf};
16: 
17: use thiserror::Error;
18: use tracing::{debug, info, trace, warn};
19: 
20: // ---------------------------------------------------------------------------
21: // Errors
22: // ---------------------------------------------------------------------------
23: 
24: /// Errors from PTY bridge operations.
25: #[derive(Debug, Error)]
26: pub enum PtyError {
27:     /// PTY creation or I/O error.
28:     #[error("PTY error: {0}")]
29:     Io(#[from] std::io::Error),
30: 
31:     /// nix system call error.
32:     #[error("system error: {0}")]
33:     Nix(#[from] nix::Error),
34: 
35:     /// Symlink creation failed.
36:     #[error("failed to create symlink {path}: {source}")]
37:     Symlink {
38:         path: PathBuf,
39:         source: std::io::Error,
40:     },
41: }
42: 
43: // ---------------------------------------------------------------------------
44: // BaudRate → u32 conversion
45: // ---------------------------------------------------------------------------
46: 
47: /// Convert a `nix::sys::termios::BaudRate` enum to the numeric baud rate.
48: ///
49: /// On Linux the enum variants are opaque constants (e.g., `B9600 = 0x0D`),
50: /// NOT the raw speed value like on BSDs, so we need an explicit mapping.
51: fn baudrate_to_u32(b: nix::sys::termios::BaudRate) -> u32 {
52:     use nix::sys::termios::BaudRate;
53:     match b {
54:         BaudRate::B0 => 0,
55:         BaudRate::B50 => 50,
56:         BaudRate::B75 => 75,
57:         BaudRate::B110 => 110,
58:         BaudRate::B134 => 134,
59:         BaudRate::B150 => 150,
60:         BaudRate::B200 => 200,
61:         BaudRate::B300 => 300,
62:         BaudRate::B600 => 600,
63:         BaudRate::B1200 => 1200,
64:         BaudRate::B1800 => 1800,
65:         BaudRate::B2400 => 2400,
66:         BaudRate::B4800 => 4800,
67:         BaudRate::B9600 => 9600,
68:         BaudRate::B19200 => 19200,
69:         BaudRate::B38400 => 38400,
70:         BaudRate::B57600 => 57600,
71:         BaudRate::B115200 => 115200,
72:         BaudRate::B230400 => 230400,
73:         BaudRate::B460800 => 460800,
74:         BaudRate::B500000 => 500000,
75:         BaudRate::B576000 => 576000,
76:         BaudRate::B921600 => 921600,
77:         BaudRate::B1000000 => 1_000_000,
78:         BaudRate::B1152000 => 1_152_000,
79:         BaudRate::B1500000 => 1_500_000,
80:         BaudRate::B2000000 => 2_000_000,
81:         BaudRate::B2500000 => 2_500_000,
82:         BaudRate::B3000000 => 3_000_000,
83:         BaudRate::B3500000 => 3_500_000,
84:         BaudRate::B4000000 => 4_000_000,
85:         _ => {
86:             warn!(baud = ?b, "unknown BaudRate variant, returning 0");
87:             0
88:         }
89:     }
90: }
91: 
92: // ---------------------------------------------------------------------------
93: // PtyBridge
94: // ---------------------------------------------------------------------------
95: 
96: /// A PTY master/slave pair for serial port emulation.
97: ///
98: /// The master fd is set to non-blocking mode.  The slave fd is kept open
99: /// so we can call `tcgetattr()` to read baud rate changes.
100: pub struct PtyBridge {
101:     /// Master file descriptor (our side).
102:     master: OwnedFd,
103:     /// Slave file descriptor (kept open for `tcgetattr` calls).
104:     slave: OwnedFd,
105:     /// Filesystem path to the slave device (e.g., `/dev/pts/3`).
106:     #[allow(dead_code)]
107:     slave_dev_path: PathBuf,
108:     /// Symlink path we created (for cleanup on drop).
109:     symlink_path: Option<PathBuf>,
110:     /// Last known baud rate, to detect changes.
111:     last_baud: u32,
112: }
113: 
114: impl PtyBridge {
115:     /// Create a new PTY pair and symlink the slave device to `symlink`.
116:     ///
117:     /// The slave path (e.g., `/tmp/cressi-irda`) is what the application
118:     /// (Subsurface) opens as a serial port.
119:     pub fn new(symlink: &Path) -> Result<Self, PtyError> {
120:         // Create the PTY pair.
121:         let pty = nix::pty::openpty(None, None)?;
122: 
123:         // Determine the slave device path (e.g., /dev/pts/N).
124:         let slave_dev_path = nix::unistd::ttyname(&pty.slave)?;
125:         debug!(slave = %slave_dev_path.display(), "PTY slave device");
126: 
127:         // Set the master fd to non-blocking.
128:         let flags = nix::fcntl::fcntl(pty.master.as_raw_fd(), nix::fcntl::FcntlArg::F_GETFL)?;
129:         nix::fcntl::fcntl(
130:             pty.master.as_raw_fd(),
131:             nix::fcntl::FcntlArg::F_SETFL(
132:                 nix::fcntl::OFlag::from_bits_truncate(flags) | nix::fcntl::OFlag::O_NONBLOCK,
133:             ),
134:         )?;
135:         debug!("master fd set to non-blocking");
136: 
137:         // Remove stale symlink if it exists, then create the new one.
138:         let symlink_path = symlink.to_path_buf();
139:         if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
140:             std::fs::remove_file(&symlink_path).ok(); // best-effort
141:         }
142:         std::os::unix::fs::symlink(&slave_dev_path, &symlink_path).map_err(|e| {
143:             PtyError::Symlink {
144:                 path: symlink_path.clone(),
145:                 source: e,
146:             }
147:         })?;
148: 
149:         // Read the initial baud rate (typically 38400 after openpty on Linux).
150:         let termios = nix::sys::termios::tcgetattr(&pty.slave)?;
151:         let initial_baud = baudrate_to_u32(nix::sys::termios::cfgetospeed(&termios));
152: 
153:         info!(
154:             slave = %slave_dev_path.display(),
155:             symlink = %symlink_path.display(),
156:             initial_baud,
157:             "PTY bridge created"
158:         );
159: 
160:         Ok(Self {
161:             master: pty.master,
162:             slave: pty.slave,
163:             slave_dev_path,
164:             symlink_path: Some(symlink_path),
165:             last_baud: initial_baud,
166:         })
167:     }
168: 
169:     /// Read from the PTY master (non-blocking).
170:     ///
171:     /// Returns the number of bytes read into `buf`, or `Err` with EAGAIN
172:     /// if no data is available.
173:     pub fn read(&self, buf: &mut [u8]) -> Result<usize, PtyError> {
174:         let n = nix::unistd::read(self.master.as_raw_fd(), buf)?;
175:         if n > 0 {
176:             trace!(len = n, "PTY master read");
177:         }
178:         Ok(n)
179:     }
180: 
181:     /// Write data to the PTY master (sent to the slave application).
182:     pub fn write(&self, data: &[u8]) -> Result<usize, PtyError> {
183:         if data.is_empty() {
184:             return Ok(0);
185:         }
186:         let n = nix::unistd::write(&self.master, data)?;
187:         trace!(len = n, "PTY master write");
188:         Ok(n)
189:     }
190: 
191:     /// Read the slave's current output baud rate.
192:     pub fn slave_baud_rate(&self) -> Result<u32, PtyError> {
193:         let termios = nix::sys::termios::tcgetattr(&self.slave)?;
194:         let baud = nix::sys::termios::cfgetospeed(&termios);
195:         let speed = baudrate_to_u32(baud);
196:         Ok(speed)
197:     }
198: 
199:     /// Check if the baud rate changed since last check.
200:     ///
201:     /// Returns `Some(new_baud)` if the baud rate changed, `None` otherwise.
202:     /// Updates the internal last-known baud rate on change.
203:     pub fn check_baud_rate_change(&mut self) -> Result<Option<u32>, PtyError> {
204:         let current = self.slave_baud_rate()?;
205:         if current != self.last_baud {
206:             info!(old = self.last_baud, new = current, "baud rate changed");
207:             self.last_baud = current;
208:             Ok(Some(current))
209:         } else {
210:             Ok(None)
211:         }
212:     }
213: 
214:     /// Path to the slave device (e.g., `/dev/pts/3`).
215:     #[allow(dead_code)]
216:     pub fn slave_dev_path(&self) -> &Path {
217:         &self.slave_dev_path
218:     }
219: 
220:     /// Path to the symlink (e.g., `/tmp/cressi-irda`).
221:     #[allow(dead_code)]
222:     pub fn symlink_path(&self) -> Option<&Path> {
223:         self.symlink_path.as_deref()
224:     }
225: 
226:     /// Borrow the master fd (for use with `tokio::io::unix::AsyncFd`).
227:     #[allow(dead_code)]
228:     pub fn master_fd(&self) -> BorrowedFd<'_> {
229:         self.master.as_fd()
230:     }
231: 
232:     /// Raw master fd (for use with poll/select/epoll).
233:     pub fn master_raw_fd(&self) -> std::os::fd::RawFd {
234:         self.master.as_raw_fd()
235:     }
236: }
237: 
238: impl Drop for PtyBridge {
239:     fn drop(&mut self) {
240:         // Clean up the symlink.
241:         if let Some(ref path) = self.symlink_path {
242:             if let Err(e) = std::fs::remove_file(path) {
243:                 warn!(path = %path.display(), error = %e, "failed to remove PTY symlink");
244:             } else {
245:                 debug!(path = %path.display(), "removed PTY symlink");
246:             }
247:         }
248:     }
249: }
250: 
251: // ---------------------------------------------------------------------------
252: // Tests
253: // ---------------------------------------------------------------------------
254: 
255: #[cfg(test)]
256: mod tests {
257:     use super::*;
258:     use std::io::{Read, Write};
259:     use std::os::fd::AsRawFd;
260: 
261:     /// Create a PTY bridge and verify the symlink exists and points correctly.
262:     #[test]
263:     fn create_and_symlink() {
264:         let symlink = std::env::temp_dir().join("ks959-bridge-test-pty-create");
265:         let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
266: 
267:         let target = std::fs::read_link(&symlink).expect("symlink missing");
268:         assert_eq!(target, bridge.slave_dev_path());
269: 
270:         drop(bridge);
271:         assert!(!symlink.exists(), "symlink should be removed on drop");
272:     }
273: 
274:     /// Helper: open slave, set raw mode, return the file.
275:     fn open_slave_raw(bridge: &PtyBridge) -> std::fs::File {
276:         let slave_file = std::fs::OpenOptions::new()
277:             .read(true)
278:             .write(true)
279:             .open(bridge.slave_dev_path())
280:             .expect("open slave");
281: 
282:         let mut termios = nix::sys::termios::tcgetattr(&slave_file).unwrap();
283:         nix::sys::termios::cfmakeraw(&mut termios);
284:         nix::sys::termios::tcsetattr(&slave_file, nix::sys::termios::SetArg::TCSANOW, &termios)
285:             .unwrap();
286:         slave_file
287:     }
288: 
289:     /// Write data to the slave side, read it from the master.
290:     #[test]
291:     fn data_roundtrip_slave_to_master() {
292:         let symlink = std::env::temp_dir().join("ks959-bridge-test-pty-s2m");
293:         let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
294:         let mut slave_file = open_slave_raw(&bridge);
295: 
296:         slave_file.write_all(b"hello").unwrap();
297:         std::thread::sleep(std::time::Duration::from_millis(50));
298: 
299:         let mut buf = [0u8; 256];
300:         let mut total = 0;
301:         for _ in 0..20 {
302:             match bridge.read(&mut buf[total..]) {
303:                 Ok(n) if n > 0 => {
304:                     total += n;
305:                     if total >= 5 {
306:                         break;
307:                     }
308:                 }
309:                 Ok(_) => {}
310:                 Err(_) => {
311:                     std::thread::sleep(std::time::Duration::from_millis(10));
312:                 }
313:             }
314:         }
315:         assert_eq!(&buf[..total], b"hello");
316:     }
317: 
318:     /// Write data to the master, read from the slave.
319:     #[test]
320:     fn data_roundtrip_master_to_slave() {
321:         let symlink = std::env::temp_dir().join("ks959-bridge-test-pty-m2s");
322:         let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
323:         let mut slave_file = open_slave_raw(&bridge);
324: 
325:         // Set slave non-blocking for the read.
326:         let fd = slave_file.as_raw_fd();
327:         let flags = nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_GETFL).unwrap();
328:         nix::fcntl::fcntl(
329:             fd,
330:             nix::fcntl::FcntlArg::F_SETFL(
331:                 nix::fcntl::OFlag::from_bits_truncate(flags) | nix::fcntl::OFlag::O_NONBLOCK,
332:             ),
333:         )
334:         .unwrap();
335: 
336:         bridge.write(b"world").expect("master write");
337:         std::thread::sleep(std::time::Duration::from_millis(50));
338: 
339:         let mut buf = [0u8; 256];
340:         let n = slave_file.read(&mut buf).expect("slave read");
341:         assert_eq!(&buf[..n], b"world");
342:     }
343: 
344:     /// Detect a baud rate change via polling tcgetattr.
345:     #[test]
346:     fn detect_baud_rate_change() {
347:         let symlink = std::env::temp_dir().join("ks959-bridge-test-pty-baud");
348:         let mut bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
349: 
350:         let slave_file = std::fs::OpenOptions::new()
351:             .read(true)
352:             .write(true)
353:             .open(bridge.slave_dev_path())
354:             .expect("open slave");
355: 
356:         // Set 115200 baud on the slave (simulating libdivecomputer).
357:         let mut termios = nix::sys::termios::tcgetattr(&slave_file).unwrap();
358:         nix::sys::termios::cfmakeraw(&mut termios);
359:         nix::sys::termios::cfsetospeed(&mut termios, nix::sys::termios::BaudRate::B115200).unwrap();
360:         nix::sys::termios::cfsetispeed(&mut termios, nix::sys::termios::BaudRate::B115200).unwrap();
361:         nix::sys::termios::tcsetattr(&slave_file, nix::sys::termios::SetArg::TCSANOW, &termios)
362:             .unwrap();
363: 
364:         // check_baud_rate_change should detect the change.
365:         let change = bridge.check_baud_rate_change().expect("check failed");
366:         assert_eq!(change, Some(115200));
367: 
368:         // Second call: no change.
369:         let no_change = bridge.check_baud_rate_change().expect("check failed");
370:         assert_eq!(no_change, None);
371:     }
372: 
373:     /// Verify initial baud rate.
374:     #[test]
375:     fn initial_baud_rate() {
376:         let symlink = std::env::temp_dir().join("ks959-bridge-test-pty-initbaud");
377:         let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
378:         let speed = bridge.slave_baud_rate().expect("slave_baud_rate");
379:         assert!(speed > 0, "initial baud rate should be > 0");
380:     }
381: 
382:     /// Symlink cleanup is idempotent.
383:     #[test]
384:     fn drop_cleanup_idempotent() {
385:         let symlink = std::env::temp_dir().join("ks959-bridge-test-pty-drop");
386:         let bridge = PtyBridge::new(&symlink).expect("PtyBridge::new failed");
387:         std::fs::remove_file(&symlink).ok();
388:         drop(bridge); // should not panic
389:     }
390: 
391:     /// BaudRate conversion covers all standard rates.
392:     #[test]
393:     fn baudrate_conversion() {
394:         use nix::sys::termios::BaudRate;
395:         assert_eq!(baudrate_to_u32(BaudRate::B9600), 9600);
396:         assert_eq!(baudrate_to_u32(BaudRate::B115200), 115200);
397:         assert_eq!(baudrate_to_u32(BaudRate::B0), 0);
398:         assert_eq!(baudrate_to_u32(BaudRate::B2400), 2400);
399:         assert_eq!(baudrate_to_u32(BaudRate::B57600), 57600);
400:     }
401: }

(End of file - total 401 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/src/sir_framing.rs"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/src/sir_framing.rs</path>
<type>file</type>
<content>
1: // IrDA SIR (Serial Infrared) async framing layer.
2: //
3: // Wraps raw payload bytes into SIR frames for transmission, and unwraps
4: // received SIR-framed bytes back into payloads.
5: //
6: // CRC: Reflected CRC-CCITT (polynomial 0x8408 — bit-reversal of 0x1021,
7: // init 0xFFFF, no final XOR).  This matches the Linux kernel's
8: // `crc_ccitt_byte()` in `lib/crc-ccitt.c` / `include/linux/crc-ccitt.h`.
9: 
10: use tracing::{debug, trace, warn};
11: 
12: // ---------------------------------------------------------------------------
13: // Constants
14: // ---------------------------------------------------------------------------
15: 
16: /// Beginning of Frame marker.
17: const BOF: u8 = 0xC0;
18: /// End of Frame marker.
19: const EOF: u8 = 0xC1;
20: /// Control Escape byte.
21: const CE: u8 = 0x7D;
22: /// Extra BOF (padding / pre-frame idle).
23: const XBOF: u8 = 0xFF;
24: /// XOR mask applied to escaped bytes during transparency processing (bit 5).
25: const IRDA_TRANS: u8 = 0x20;
26: /// Initial value for the Frame Check Sequence (CRC).
27: const INIT_FCS: u16 = 0xFFFF;
28: /// Value of a correctly-received FCS after processing the full frame
29: /// (payload + trailing FCS bytes).
30: const GOOD_FCS: u16 = 0xF0B8;
31: 
32: // ---------------------------------------------------------------------------
33: // CRC-CCITT reflected  (polynomial 0x8408, table-driven, compile-time)
34: //
35: // This is identical to the Linux kernel's lib/crc-ccitt.c which uses the
36: // reflected polynomial.  The table entry for index 1 is 0x1189.
37: // ---------------------------------------------------------------------------
38: 
39: /// Build the 256-entry reflected CRC-CCITT lookup table at compile time.
40: const fn make_crc_ccitt_table() -> [u16; 256] {
41:     let mut table = [0u16; 256];
42:     let mut i = 0usize;
43:     while i < 256 {
44:         let mut crc = i as u16;
45:         let mut j = 0;
46:         while j < 8 {
47:             if crc & 1 != 0 {
48:                 crc = (crc >> 1) ^ 0x8408;
49:             } else {
50:                 crc >>= 1;
51:             }
52:             j += 1;
53:         }
54:         table[i] = crc;
55:         i += 1;
56:     }
57:     table
58: }
59: 
60: /// Pre-computed reflected CRC-CCITT lookup table (matches Linux kernel).
61: const CRC_CCITT_TABLE: [u16; 256] = make_crc_ccitt_table();
62: 
63: /// Update a running CRC with one byte (same as Linux `crc_ccitt_byte`).
64: #[inline]
65: fn irda_fcs(fcs: u16, byte: u8) -> u16 {
66:     (fcs >> 8) ^ CRC_CCITT_TABLE[((fcs ^ byte as u16) & 0xFF) as usize]
67: }
68: 
69: /// Compute CRC over an entire byte slice starting from INIT_FCS.
70: fn crc_ccitt(data: &[u8]) -> u16 {
71:     let mut fcs = INIT_FCS;
72:     for &byte in data {
73:         fcs = irda_fcs(fcs, byte);
74:     }
75:     fcs
76: }
77: 
78: // ---------------------------------------------------------------------------
79: // Byte-stuffing helper
80: // ---------------------------------------------------------------------------
81: 
82: /// Apply IrDA byte-stuffing to a single byte.
83: ///
84: /// If the byte is `BOF` (0xC0), `EOF` (0xC1), or `CE` (0x7D), emit
85: /// `[CE, byte ^ IRDA_TRANS]`.  Otherwise emit the byte as-is.
86: fn byte_stuff(byte: u8) -> Vec<u8> {
87:     match byte {
88:         BOF | EOF | CE => vec![CE, byte ^ IRDA_TRANS],
89:         _ => vec![byte],
90:     }
91: }
92: 
93: // ===========================================================================
94: // TX: Wrapping
95: // ===========================================================================
96: 
97: /// Wrap a raw payload into a complete SIR frame.
98: ///
99: /// # Arguments
100: ///
101: /// * `payload` - The raw data bytes to transmit.
102: /// * `extra_bofs` - Number of extra XBOF (0xFF) bytes to prepend before the
103: ///   frame.  These act as inter-frame padding / pre-frame idle.
104: ///
105: /// # Output format
106: ///
107: /// 1. `extra_bofs` copies of `XBOF` (0xFF)
108: /// 2. `BOF` (0xC0)
109: /// 3. Byte-stuffed payload bytes
110: /// 4. FCS (Frame Check Sequence) — `~CRC-CCITT-FALSE(payload)`, byte-stuffed,
111: ///    appended in little-endian order
112: /// 5. `EOF` (0xC1)
113: pub fn wrap_frame(payload: &[u8], extra_bofs: usize) -> Vec<u8> {
114:     let mut frame = Vec::with_capacity(
115:         extra_bofs + 1 // XBOFs + BOF
116:         + payload.len() * 2 // worst-case: every payload byte is escaped
117:         + 2 * 2 // two FCS bytes, worst-case escaped
118:         + 1, // EOF
119:     );
120: 
121:     // 1. Extra BOFs
122:     for _ in 0..extra_bofs {
123:         frame.push(XBOF);
124:     }
125: 
126:     // 2. BOF
127:     frame.push(BOF);
128: 
129:     // 3. Byte-stuff payload
130:     for &byte in payload {
131:         frame.extend_from_slice(&byte_stuff(byte));
132:     }
133: 
134:     // 4. Compute FCS = ~CRC-CCITT(payload), little-endian
135:     let crc = crc_ccitt(payload);
136:     let fcs = !crc; // bitwise NOT (one's complement)
137:     let fcs_lo = (fcs & 0xFF) as u8;
138:     let fcs_hi = ((fcs >> 8) & 0xFF) as u8;
139: 
140:     frame.extend_from_slice(&byte_stuff(fcs_lo));
141:     frame.extend_from_slice(&byte_stuff(fcs_hi));
142: 
143:     // 5. EOF
144:     frame.push(EOF);
145: 
146:     frame
147: }
148: 
149: // ===========================================================================
150: // RX: Unwrapping  (state machine matching Linux kernel's wrapper.c)
151: // ===========================================================================
152: 
153: /// State of the SIR frame unwrapper.
154: #[derive(Debug, Clone, Copy, PartialEq, Eq)]
155: enum UnwrapState {
156:     /// Waiting for a BOF marker; all other bytes are ignored (except XBOF).
157:     OutsideFrame,
158:     /// BOF has been received; waiting for the first data byte.
159:     BeginFrame,
160:     /// Actively collecting frame bytes.
161:     InsideFrame,
162:     /// Control-Escape received; the next byte must be XORed with IRDA_TRANS.
163:     LinkEscape,
164: }
165: 
166: /// Stateful unwrapper that processes a stream of raw SIR bytes and extracts
167: /// completed, validated payloads.
168: pub struct SirUnwrapper {
169:     state: UnwrapState,
170:     buffer: Vec<u8>,
171:     fcs: u16,
172: }
173: 
174: impl SirUnwrapper {
175:     /// Create a new unwrapper in the initial `OutsideFrame` state.
176:     #[must_use]
177:     pub fn new() -> Self {
178:         Self {
179:             state: UnwrapState::OutsideFrame,
180:             buffer: Vec::new(),
181:             fcs: INIT_FCS,
182:         }
183:     }
184: 
185:     /// Feed one received byte into the state machine.
186:     ///
187:     /// Returns `Some(payload)` when a complete valid frame has been received
188:     /// and its CRC passes.  Corrupt frames are silently discarded (logged at
189:     /// `warn` level).
190:     ///
191:     /// Structure mirrors the kernel's `async_unwrap_char()`: dispatch on the
192:     /// byte class first, then on the current state.
193:     #[must_use]
194:     pub fn process_byte(&mut self, byte: u8) -> Option<Vec<u8>> {
195:         trace!(byte = format_args!("0x{:02X}", byte), state = ?self.state, "sir_unwrap");
196: 
197:         match byte {
198:             // --- BOF: Beginning Of Frame ---
199:             BOF => {
200:                 match self.state {
201:                     UnwrapState::LinkEscape | UnwrapState::InsideFrame => {
202:                         warn!("BOF inside frame — discarding incomplete frame");
203:                     }
204:                     UnwrapState::OutsideFrame | UnwrapState::BeginFrame => {
205:                         // Multiple BOFs at start of frame are normal
206:                     }
207:                 }
208:                 self.state = UnwrapState::BeginFrame;
209:                 self.buffer.clear();
210:                 self.fcs = INIT_FCS;
211:                 None
212:             }
213: 
214:             // --- EOF: End Of Frame ---
215:             EOF => {
216:                 match self.state {
217:                     UnwrapState::OutsideFrame => {
218:                         warn!("EOF outside frame — missed BOF");
219:                         None
220:                     }
221:                     UnwrapState::BeginFrame
222:                     | UnwrapState::InsideFrame
223:                     | UnwrapState::LinkEscape => {
224:                         // In BeginFrame/LinkEscape, FCS will almost certainly
225:                         // fail — expected per kernel behaviour.
226:                         self.state = UnwrapState::OutsideFrame;
227:                         self.finish_frame()
228:                     }
229:                 }
230:             }
231: 
232:             // --- CE: Control Escape ---
233:             CE => {
234:                 match self.state {
235:                     UnwrapState::OutsideFrame => {
236:                         // carrier sense noise, ignore
237:                     }
238:                     UnwrapState::LinkEscape => {
239:                         warn!("double CE — undefined state");
240:                     }
241:                     UnwrapState::BeginFrame | UnwrapState::InsideFrame => {
242:                         self.state = UnwrapState::LinkEscape;
243:                     }
244:                 }
245:                 None
246:             }
247: 
248:             // --- Any other byte ---
249:             other => {
250:                 match self.state {
251:                     UnwrapState::OutsideFrame => {
252:                         if other != XBOF {
253:                             trace!(
254:                                 byte = format_args!("0x{:02X}", other),
255:                                 "noise outside frame"
256:                             );
257:                         }
258:                     }
259:                     UnwrapState::BeginFrame => {
260:                         self.buffer.push(other);
261:                         self.fcs = irda_fcs(self.fcs, other);
262:                         self.state = UnwrapState::InsideFrame;
263:                     }
264:                     UnwrapState::InsideFrame => {
265:                         self.buffer.push(other);
266:                         self.fcs = irda_fcs(self.fcs, other);
267:                     }
268:                     UnwrapState::LinkEscape => {
269:                         let unstuffed = other ^ IRDA_TRANS;
270:                         self.buffer.push(unstuffed);
271:                         self.fcs = irda_fcs(self.fcs, unstuffed);
272:                         self.state = UnwrapState::InsideFrame;
273:                     }
274:                 }
275:                 None
276:             }
277:         }
278:     }
279: 
280:     /// Process a slice of received bytes, returning all completed frame
281:     /// payloads extracted during processing.
282:     #[must_use]
283:     pub fn process_bytes(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
284:         let mut frames = Vec::new();
285:         for &byte in data {
286:             if let Some(payload) = self.process_byte(byte) {
287:                 frames.push(payload);
288:             }
289:         }
290:         frames
291:     }
292: 
293:     /// Called when EOF is received.  Validates the FCS and returns the
294:     /// payload (excluding the trailing 2 CRC bytes) if good.
295:     fn finish_frame(&mut self) -> Option<Vec<u8>> {
296:         if self.fcs != GOOD_FCS {
297:             warn!(
298:                 fcs = ?format_args!("0x{:04X}", self.fcs),
299:                 len = self.buffer.len(),
300:                 "CRC error — discarding frame"
301:             );
302:             return None;
303:         }
304: 
305:         let payload_len = self.buffer.len().saturating_sub(2);
306:         let payload = self.buffer[..payload_len].to_vec();
307: 
308:         debug!(len = payload.len(), "valid frame received");
309: 
310:         Some(payload)
311:     }
312: }
313: 
314: impl Default for SirUnwrapper {
315:     fn default() -> Self {
316:         Self::new()
317:     }
318: }
319: 
320: // ===========================================================================
321: // Tests
322: // ===========================================================================
323: 
324: #[cfg(test)]
325: mod tests {
326:     use super::*;
327: 
328:     // -----------------------------------------------------------------------
329:     // CRC correctness
330:     // -----------------------------------------------------------------------
331: 
332:     /// Verify reflected CRC-CCITT against the known check value for "123456789".
333:     /// With polynomial 0x8408, init 0xFFFF, no final XOR, the result is 0x6F91.
334:     #[test]
335:     fn crc_ccitt_check_value() {
336:         let data = b"123456789";
337:         let expected: u16 = 0x6F91;
338:         assert_eq!(
339:             crc_ccitt(data),
340:             expected,
341:             "reflected CRC-CCITT for '123456789' must be 0x6F91"
342:         );
343:     }
344: 
345:     /// CRC of empty data should be 0xFFFF (the init value, since nothing
346:     /// shifted it out).
347:     #[test]
348:     fn crc_ccitt_empty() {
349:         assert_eq!(crc_ccitt(b""), INIT_FCS, "CRC of empty data = INIT_FCS");
350:     }
351: 
352:     /// Verify that CRC table entry [1] matches the Linux kernel's value.
353:     #[test]
354:     fn crc_table_spot_check() {
355:         assert_eq!(CRC_CCITT_TABLE[1], 0x1189, "table[1] must be 0x1189");
356:     }
357: 
358:     /// Verify the GOOD_FCS residue: CRC over (data + ~CRC(data) as LE) == 0xF0B8.
359:     #[test]
360:     fn good_fcs_residue() {
361:         let data = b"test payload";
362:         let crc = crc_ccitt(data);
363:         let fcs = !crc;
364:         let fcs_lo = (fcs & 0xFF) as u8;
365:         let fcs_hi = ((fcs >> 8) & 0xFF) as u8;
366: 
367:         let mut full = data.to_vec();
368:         full.push(fcs_lo);
369:         full.push(fcs_hi);
370:         assert_eq!(crc_ccitt(&full), GOOD_FCS, "residue must equal GOOD_FCS");
371:     }
372: 
373:     // -----------------------------------------------------------------------
374:     // Byte-stuffing
375:     // -----------------------------------------------------------------------
376: 
377:     #[test]
378:     fn byte_stuff_normal_bytes_passthrough() {
379:         for b in 0x00..=0xFFu8 {
380:             if b == BOF || b == EOF || b == CE {
381:                 continue;
382:             }
383:             assert_eq!(byte_stuff(b), vec![b], "0x{:02X} should pass through", b);
384:         }
385:     }
386: 
387:     #[test]
388:     fn byte_stuff_special_bytes_escaped() {
389:         assert_eq!(byte_stuff(BOF), vec![CE, BOF ^ IRDA_TRANS]);
390:         assert_eq!(byte_stuff(EOF), vec![CE, EOF ^ IRDA_TRANS]);
391:         assert_eq!(byte_stuff(CE), vec![CE, CE ^ IRDA_TRANS]);
392:     }
393: 
394:     // -----------------------------------------------------------------------
395:     // Round-trip
396:     // -----------------------------------------------------------------------
397: 
398:     /// Helper: wrap a payload (with 0 extra BOFs), then feed the frame into
399:     /// a fresh unwrapper and collect the result.
400:     fn roundtrip(payload: &[u8]) -> Vec<u8> {
401:         let frame = wrap_frame(payload, 0);
402:         let mut unwrapper = SirUnwrapper::new();
403:         let results = unwrapper.process_bytes(&frame);
404:         assert_eq!(
405:             results.len(),
406:             1,
407:             "round-trip should produce exactly 1 frame, got {}",
408:             results.len()
409:         );
410:         results.into_iter().next().unwrap()
411:     }
412: 
413:     #[test]
414:     fn roundtrip_empty_payload() {
415:         assert_eq!(roundtrip(b""), b"");
416:     }
417: 
418:     #[test]
419:     fn roundtrip_single_byte() {
420:         assert_eq!(roundtrip(&[0x42]), vec![0x42]);
421:     }
422: 
423:     #[test]
424:     fn roundtrip_ascii_hello() {
425:         let payload = b"hello, irda!";
426:         assert_eq!(roundtrip(payload), payload);
427:     }
428: 
429:     #[test]
430:     fn roundtrip_all_special_bytes() {
431:         // Payload containing every special byte — they must survive
432:         // byte-stuffing and un-stuffing.
433:         let payload: Vec<u8> = vec![BOF, EOF, CE, BOF, CE, EOF];
434:         assert_eq!(roundtrip(&payload), payload);
435:     }
436: 
437:     #[test]
438:     fn roundtrip_mixed_special_and_normal() {
439:         let payload: Vec<u8> = (0..=255u8).collect();
440:         assert_eq!(roundtrip(&payload), payload);
441:     }
442: 
443:     #[test]
444:     fn roundtrip_large_payload() {
445:         let payload: Vec<u8> = (0..2048).map(|i| (i % 256) as u8).collect();
446:         assert_eq!(roundtrip(&payload), payload);
447:     }
448: 
449:     #[test]
450:     fn roundtrip_all_zeroes() {
451:         let payload = vec![0u8; 512];
452:         assert_eq!(roundtrip(&payload), payload);
453:     }
454: 
455:     // -----------------------------------------------------------------------
456:     // extra_bofs
457:     // -----------------------------------------------------------------------
458: 
459:     #[test]
460:     fn extra_bofs_stripped() {
461:         let payload = b"data";
462:         let frame = wrap_frame(payload, 3);
463:         // First 3 bytes must be XBOF
464:         assert_eq!(&frame[0..3], &[XBOF, XBOF, XBOF]);
465:         assert_eq!(frame[3], BOF);
466:         // Unwrapper should ignore them
467:         let mut unwrapper = SirUnwrapper::new();
468:         let results = unwrapper.process_bytes(&frame);
469:         assert_eq!(results.len(), 1);
470:         assert_eq!(&results[0], payload);
471:     }
472: 
473:     // -----------------------------------------------------------------------
474:     // CRC error detection
475:     // -----------------------------------------------------------------------
476: 
477:     #[test]
478:     fn corrupt_frame_rejected() {
479:         let payload = b"important data";
480:         let mut frame = wrap_frame(payload, 0);
481: 
482:         // Flip a bit in the frame body (after BOF, before the last non-EOF).
483:         // Find the first non-BOF, non-XBOF byte and flip its LSB.
484:         if let Some(pos) = frame.iter().position(|&b| b != BOF && b != XBOF && b != CE) {
485:             frame[pos] ^= 1;
486:         } else if frame.len() > 2 {
487:             // Fallback: flip byte at position 1 (after BOF)
488:             frame[1] ^= 1;
489:         }
490: 
491:         let mut unwrapper = SirUnwrapper::new();
492:         let results = unwrapper.process_bytes(&frame);
493:         assert!(
494:             results.is_empty(),
495:             "corrupt frame must be rejected (got {} frames)",
496:             results.len()
497:         );
498:     }
499: 
500:     #[test]
501:     fn truncated_frame_rejected() {
502:         let payload = b"truncated";
503:         let frame = wrap_frame(payload, 0);
504:         // Drop the last 3 bytes (EOF + maybe part of FCS)
505:         let truncated = &frame[..frame.len().saturating_sub(3)];
506:         let mut unwrapper = SirUnwrapper::new();
507:         let results = unwrapper.process_bytes(truncated);
508:         assert!(
509:             results.is_empty(),
510:             "truncated frame must not produce output"
511:         );
512:     }
513: 
514:     // -----------------------------------------------------------------------
515:     // Multiple frames in a stream
516:     // -----------------------------------------------------------------------
517: 
518:     #[test]
519:     fn multiple_frames_in_stream() {
520:         let payloads: [&[u8]; 3] = [b"first", b"second", b"third"];
521:         let mut stream = Vec::new();
522:         for p in &payloads {
523:             stream.extend_from_slice(&wrap_frame(p, 0));
524:         }
525: 
526:         let mut unwrapper = SirUnwrapper::new();
527:         let results = unwrapper.process_bytes(&stream);
528:         assert_eq!(results.len(), 3);
529:         for (i, result) in results.iter().enumerate() {
530:             assert_eq!(result, payloads[i], "frame {} mismatch", i);
531:         }
532:     }
533: 
534:     #[test]
535:     fn multiple_frames_with_extra_bofs() {
536:         let payloads: [&[u8]; 2] = [b"alpha", b"beta"];
537:         let mut stream = Vec::new();
538:         stream.extend_from_slice(&wrap_frame(payloads[0], 2));
539:         stream.extend_from_slice(&wrap_frame(payloads[1], 5));
540: 
541:         let mut unwrapper = SirUnwrapper::new();
542:         let results = unwrapper.process_bytes(&stream);
543:         assert_eq!(results.len(), 2);
544:         assert_eq!(&results[0], payloads[0]);
545:         assert_eq!(&results[1], payloads[1]);
546:     }
547: 
548:     // -----------------------------------------------------------------------
549:     // Garbage / noise between frames — resilience
550:     // -----------------------------------------------------------------------
551: 
552:     #[test]
553:     fn garbage_between_frames() {
554:         let frame_a = wrap_frame(b"frame-A", 0);
555:         let frame_b = wrap_frame(b"frame-B", 0);
556: 
557:         let mut stream = Vec::new();
558:         stream.extend_from_slice(&frame_a);
559:         // Inject garbage bytes (not XBOF, BOF, EOF, CE — just noise)
560:         stream.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);
561:         stream.extend_from_slice(&frame_b);
562: 
563:         let mut unwrapper = SirUnwrapper::new();
564:         let results = unwrapper.process_bytes(&stream);
565:         assert_eq!(
566:             results.len(),
567:             2,
568:             "garbage should not prevent frame extraction"
569:         );
570:         assert_eq!(&results[0], b"frame-A");
571:         assert_eq!(&results[1], b"frame-B");
572:     }
573: 
574:     #[test]
575:     fn xbof_between_frames_ignored() {
576:         let frame_a = wrap_frame(b"A", 0);
577:         let frame_b = wrap_frame(b"B", 0);
578: 
579:         let mut stream = Vec::new();
580:         stream.extend_from_slice(&frame_a);
581:         stream.extend_from_slice(&[XBOF; 10]);
582:         stream.extend_from_slice(&frame_b);
583: 
584:         let mut unwrapper = SirUnwrapper::new();
585:         let results = unwrapper.process_bytes(&stream);
586:         assert_eq!(results.len(), 2);
587:         assert_eq!(&results[0], b"A");
588:         assert_eq!(&results[1], b"B");
589:     }
590: 
591:     // -----------------------------------------------------------------------
592:     // State machine edge cases
593:     // -----------------------------------------------------------------------
594: 
595:     #[test]
596:     fn consecutive_bof_resets() {
597:         // Multiple BOFs before any data should be harmless.
598:         let mut unwrapper = SirUnwrapper::new();
599:         let _ = unwrapper.process_byte(BOF);
600:         let _ = unwrapper.process_byte(BOF);
601:         let _ = unwrapper.process_byte(BOF);
602:         // Now send a minimal frame: BOF data EOF
603:         let frame = wrap_frame(b"ok", 0);
604:         // Skip the leading BOF since we're already past it
605:         let results = unwrapper.process_bytes(&frame[1..]);
606:         // The second BOF in the frame resets... wait, after the second BOF
607:         // the data and EOF should still be there.
608:         assert_eq!(results.len(), 1);
609:         assert_eq!(&results[0], b"ok");
610:     }
611: 
612:     #[test]
613:     fn bof_inside_frame_discards() {
614:         let payload = b"partial";
615:         let mut frame = wrap_frame(payload, 0);
616:         let eof_pos = frame.iter().rposition(|&b| b == EOF).unwrap();
617: 
618:         // Insert a spurious BOF before the real EOF
619:         frame.insert(eof_pos, BOF);
620: 
621:         let mut unwrapper = SirUnwrapper::new();
622:         let results = unwrapper.process_bytes(&frame);
623:         // The spurious BOF resets the frame, so the incomplete first portion
624:         // is discarded.  Whatever follows may or may not form a valid frame.
625:         // We care that the original payload is NOT in the output.
626:         for r in &results {
627:             assert_ne!(
628:                 r, payload,
629:                 "original payload should be discarded after BOF reset"
630:             );
631:         }
632:     }
633: 
634:     #[test]
635:     fn empty_frame_body() {
636:         // A frame with no payload bytes: BOF, FCS bytes, EOF.
637:         // The FCS bytes are computed over empty data → FCS = ~0xFFFF = 0x0000.
638:         let mut frame = vec![BOF];
639:         let fcs: u16 = !crc_ccitt(b"");
640:         frame.extend_from_slice(&byte_stuff((fcs & 0xFF) as u8));
641:         frame.extend_from_slice(&byte_stuff(((fcs >> 8) & 0xFF) as u8));
642:         frame.push(EOF);
643: 
644:         let mut unwrapper = SirUnwrapper::new();
645:         let results = unwrapper.process_bytes(&frame);
646:         // Buffer will have 2 FCS bytes, FCS check passes → payload_len 0.
647:         assert_eq!(results.len(), 1);
648:         assert!(results[0].is_empty());
649:     }
650: 
651:     // -----------------------------------------------------------------------
652:     // process_byte returns None until EOF
653:     // -----------------------------------------------------------------------
654: 
655:     #[test]
656:     fn process_byte_returns_none_until_eof() {
657:         let mut unwrapper = SirUnwrapper::new();
658:         let frame = wrap_frame(b"test", 0);
659:         for &byte in &frame[..frame.len() - 1] {
660:             assert!(
661:                 unwrapper.process_byte(byte).is_none(),
662:                 "process_byte should return None until EOF"
663:             );
664:         }
665:         // Last byte (EOF) should complete the frame
666:         let result = unwrapper.process_byte(frame[frame.len() - 1]);
667:         assert!(result.is_some(), "EOF should yield a completed frame");
668:         assert_eq!(&result.unwrap(), b"test");
669:     }
670: }

(End of file - total 670 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/kmod/ks959_speed.c"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/kmod/ks959_speed.c</path>
<type>file</type>
<content>
1: // SPDX-License-Identifier: GPL-2.0
2: /*
3:  * ks959_speed - Minimal kernel module to change Kingsun KS-959 IrDA dongle speed.
4:  *
5:  * The Linux kernel's usbfs check_ctrlrecip() blocks userspace control transfers
6:  * with USB_TYPE_CLASS + USB_RECIP_INTERFACE when wIndex doesn't match an existing
7:  * interface number. The KS-959 dongle uses wIndex=0x0001 as a protocol flag
8:  * (meaning "speed change"), but the device only has interface 0. This creates a
9:  * deadlock: the dongle needs bRequestType=0x21 (Class+Interface), but the kernel
10:  * rejects it because interface 1 doesn't exist.
11:  *
12:  * This module bypasses usbfs by calling usb_control_msg() directly from kernel
13:  * context. It matches the dongle by VID/PID, changes the speed in its probe()
14:  * function, then returns -ENODEV so it doesn't permanently claim the device.
15:  * The userspace bridge (ks959-bridge) can then open the dongle normally via nusb.
16:  *
17:  * Usage:
18:  *   sudo insmod ks959_speed.ko baud=115200
19:  *
20:  * The module can only be used once per USB plug cycle (returning -ENODEV prevents
21:  * re-probing until the device is physically reconnected).
22:  */
23: 
24: #include <linux/module.h>
25: #include <linux/moduleparam.h>
26: #include <linux/kernel.h>
27: #include <linux/usb.h>
28: 
29: #define KS959_VENDOR_ID   0x07d0
30: #define KS959_PRODUCT_ID  0x4959
31: #define KS959_REQ_SEND    0x09
32: 
33: /* Default baud rate: 115200 (what the Cressi Donatello uses). */
34: static unsigned int baud = 115200;
35: module_param(baud, uint, 0444);
36: MODULE_PARM_DESC(baud, "Desired IrDA link baud rate (default: 115200)");
37: 
38: /*
39:  * Speed-change payload (8 bytes, packed, little-endian):
40:  *   [baudrate_le32] [flags=0x03] [reserved=0,0,0]
41:  * flags = 0x03 means 8 data bits.
42:  */
43: static int ks959_change_speed(struct usb_device *udev, unsigned int baud_rate)
44: {
45: 	u8 payload[8];
46: 	int ret;
47: 
48: 	memset(payload, 0, sizeof(payload));
49: 	payload[0] = (u8)(baud_rate & 0xff);
50: 	payload[1] = (u8)((baud_rate >> 8) & 0xff);
51: 	payload[2] = (u8)((baud_rate >> 16) & 0xff);
52: 	payload[3] = (u8)((baud_rate >> 24) & 0xff);
53: 	payload[4] = 0x03; /* KS_DATA_8_BITS */
54: 
55: 	/*
56: 	 * bRequestType: USB_DIR_OUT | USB_TYPE_CLASS | USB_RECIP_INTERFACE = 0x21
57: 	 * bRequest:     0x09 (KINGSUN_REQ_SEND)
58: 	 * wValue:       0x0200 (speed change identifier)
59: 	 * wIndex:       0x0001 (protocol flag: "this is a speed change")
60: 	 * wLength:      8
61: 	 */
62: 	ret = usb_control_msg(udev,
63: 			      usb_sndctrlpipe(udev, 0),
64: 			      KS959_REQ_SEND,
65: 			      USB_DIR_OUT | USB_TYPE_CLASS | USB_RECIP_INTERFACE,
66: 			      0x0200, 0x0001,
67: 			      payload, sizeof(payload),
68: 			      1000 /* timeout ms */);
69: 
70: 	if (ret < 0)
71: 		return ret;
72: 	if (ret != sizeof(payload))
73: 		return -EIO;
74: 
75: 	return 0;
76: }
77: 
78: static int ks959_speed_probe(struct usb_interface *interface,
79: 			     const struct usb_device_id *id)
80: {
81: 	struct usb_device *udev = interface_to_usbdev(interface);
82: 	int ret;
83: 
84: 	ret = ks959_change_speed(udev, baud);
85: 	if (ret) {
86: 		dev_err(&interface->dev,
87: 			"ks959_speed: speed change to %u baud failed: %d\n",
88: 			baud, ret);
89: 		return ret;
90: 	}
91: 
92: 	dev_info(&interface->dev,
93: 		 "ks959_speed: dongle speed changed to %u baud\n", baud);
94: 
95: 	/*
96: 	 * Return -ENODEV so we don't permanently claim the device.
97: 	 * The dongle retains the speed setting. The userspace bridge
98: 	 * (ks959-bridge) will open the dongle via nusb afterward.
99: 	 *
100: 	 * Caveat: this driver won't re-probe until the device is physically
101: 	 * unplugged and replugged. That's fine — we only need one speed
102: 	 * change per session.
103: 	 */
104: 	return -ENODEV;
105: }
106: 
107: static void ks959_speed_disconnect(struct usb_interface *interface)
108: {
109: 	/* Never called because probe returns -ENODEV. */
110: }
111: 
112: static const struct usb_device_id ks959_speed_table[] = {
113: 	{ USB_DEVICE(KS959_VENDOR_ID, KS959_PRODUCT_ID) },
114: 	{ } /* terminator */
115: };
116: MODULE_DEVICE_TABLE(usb, ks959_speed_table);
117: 
118: static struct usb_driver ks959_speed_driver = {
119: 	.name       = "ks959_speed",
120: 	.probe      = ks959_speed_probe,
121: 	.disconnect = ks959_speed_disconnect,
122: 	.id_table   = ks959_speed_table,
123: };
124: 
125: module_usb_driver(ks959_speed_driver);
126: 
127: MODULE_LICENSE("GPL");
128: MODULE_AUTHOR("ks959-bridge project");
129: MODULE_DESCRIPTION("Bypass usbfs check_ctrlrecip to change Kingsun KS-959 IrDA dongle speed");

(End of file - total 129 lines)
</content>
```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/Cargo.toml"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/Cargo.toml</path>
<type>file</type>
<content>
1: [package]
2: name = "ks959-bridge"
3: version = "0.1.0"
4: edition = "2021"
5: description = "Userspace IrDA SIR driver for Kingsun KS-959 USB dongle, exposing a PTY for libdivecomputer"
6: 
7: [dependencies]
8: # USB
9: nusb = "0.1"
10: futures-lite = "2"
11: 
12: # Async runtime
13: tokio = { version = "1", features = ["rt", "macros", "io-util", "net", "signal", "time", "sync"] }
14: 
15: # PTY / Unix
16: nix = { version = "0.29", features = ["term", "ioctl", "fs"] }
17: libc = "0.2"
18: 
19: # CRC
20: crc = "3"
21: 
22: # Logging
23: tracing = "0.1"
24: tracing-subscriber = { version = "0.3", features = ["env-filter"] }
25: 
26: # Error handling
27: thiserror = "2"
28: anyhow = "1"
29: 
30: # CLI
31: clap = { version = "4", features = ["derive"] }

(End of file - total 31 lines)
</content>
```

---

## Assistant (Architect · Claude Fable 5 · 166.7s)

_Thinking:_

I'm weighing whether to check the reference kernel driver against the USB protocol specs for verification, but I'm running into the constraint about not accessing the reference folder since it contains massive libraries. The verification would be important for a sanity check, though I'm uncertain about how to proceed given that limitation. I'm pulling up the actual constants and obfuscation logic from memory, though I'm uncertain about the exact details—I recall the driver using `KINGSUN_REQ_RECV 0x01`, `KINGSUN_REQ_SEND 0x09`, and some form of XOR-based obfuscation with a base value and lookup table, but I don't want to rely on potentially confabulated recollection rather than verifying against the source directly.

I'm recognizing that while the reference folder is off-limits to avoid context flooding, comparing the Rust implementation against the original C driver would be the most reliable verification—so I should flag that as a follow-up rather than guessing. In the meantime, I can check other things: running the test suite to confirm the 45/45 claim, linting with clippy and fmt, reviewing the git history, examining the Makefile, and looking for documentation files like KNOWLEDGE.md or DESIGN.md that were mentioned in AGENTS.md.

Now I'm diving into the actual code issues I've spotted so far, starting with main.rs and the baud-rate handling.

I'm noticing a critical bug in the baud-change logic: when using the recommended workflow with `--skip-speed-change` and the kernel module, dctool sets the PTY to 115200, but the bridge's `current_baud` is already 115200 from the command-line argument, so the comparison `new_baud != current_baud` evaluates to false and the RX counter never resets. The stale-drain at startup handles module-induced garbage, but any IR bytes arriving between bridge startup and dctool connecting will desynchronize the counter permanently—this is a real bug that contradicts what STATUS.md claims about counter resets on baud changes.

The core issue is that the reset should trigger whenever dctool opens the PTY, not just when the baud differs from the current value. A simpler fix would be to reset the counter whenever a baud change is detected on the PTY, regardless of whether it matches the current baud, moving the reset outside the inequality check.

For the AsyncFd readiness handling, the current approach of not clearing readiness in the success path works with epoll's edge-triggered semantics — the next readable() call returns immediately and read returns EAGAIN, triggering the clear. It's slightly inefficient but correct per tokio's documentation.

The partial PTY write issue is more serious: when fewer bytes are written than available, the remainder gets silently dropped, corrupting the data stream to dctool without desynchronizing the RX counter. Since the Donatello protocol relies on per-packet ACKs and likely has no retry mechanism, this data loss needs to be handled by buffering the unwritten bytes rather than discarding them.

A transient USB transfer error in the RX poll path will kill the entire bridge, which might be acceptable but worth flagging. The dongle.send() calls with control transfers could block the select loop for tens of milliseconds on low-speed USB, though this is probably fine since host-to-device commands are tiny. Both TX and RX use the same interface but serialize through the select loop since each branch awaits to completion before the next can run. Tokio's random branch selection handles fairness between PTY reads and poll ticks naturally.

In usb_dongle.rs, the wValue field correctly encodes the cleartext length. However, the set_speed function uses ControlType::Vendor (0x41), which STATUS.md confirms causes the dongle to STALL — meaning the in-band speed change is dead code that always fails, contradicting the comment suggesting it might work. The fallback in main.rs warns and continues, but it also sets current_baud even on failure, so it won't retry. The RX garbage-byte skip logic appears tied to a de-obfuscation scheme from the kernel driver that XORs incoming bytes with an incrementing mask and a fixed 0x55 constant. The kernel driver always wraps frames with SIR framing before obfuscation and transmission, while the bridge in raw mode sends unframed bytes directly — empirically this works since the dongle is just a SIR modulator that transmits whatever bytes it receives. On the RX side, the kernel unwraps SIR frames while the bridge passes decoded bytes straight through. I'm verifying the fragmentation math: max cleartext per fragment is 240 bytes, which pads to exactly 256 bytes matching the USB packet size. Control transfers requesting 2048 bytes on a low-speed device work fine since the device just returns a short packet. And I'm checking the poll_receive logic handles device errors correctly before processing data, plus confirming the rx_counter behavior across these edge cases.

Now looking at the PTY bridge implementation — it's using nix's read function on the master file descriptor, which is straightforward. The bridge keeps its own slave fd open permanently for tcgetattr, which means it never sees EOF when dctool closes its slave connection; the master stays open as long as the bridge's fd exists. This is good for persistence across multiple dctool sessions but creates a subtle issue: since the bridge never gets a hangup signal, the counter-reset heuristic that's supposed to detect new clients relies entirely on baud rate changes, which is already broken. There's also a potential symlink race condition when creating symlinks in /tmp that needs security review.

One more thing I'm noticing: the PTY slave starts with echo and canonical mode enabled by default. If the Donatello sends unsolicited bytes before dctool opens the connection and switches to raw mode, the line discipline will buffer and echo them back to the master, and the bridge will read those echoed bytes and transmit them back to the IR dongle as if dctool had sent them — that's a data corruption issue.

The test helper calls cfmakeraw on the slave, but the production PtyBridge::new doesn't, which is a real gap. The dangerous window is between bridge startup and dctool opening the port, especially since the recommended workflow has the bridge starting before Donatello in PC mode with dctool running immediately after. Setting raw mode on the slave at creation would close this hole.

Also, check_baud_rate_change only triggers when PTY data arrives. dctool sets the baud rate, sleeps 100ms, purges, then writes CMD_VERSION, so the bridge detects the speed change on that first write before forwarding it — that part's fine. But if the speed change needed to propagate to the dongle in the non-kernel-module flow, the timing gets trickier.

The baudrate_to_u32 function has a wildcard arm that returns 0 for unknown rates, which could theoretically match if both current and last are 0, but that's not a real issue. Looking at sir_framing.rs, the code quality is solid though the byte_stuff function creates a Vec per byte which is wasteful, and there's a confused comment in the consecutive_bof_resets test that reads like leftover thinking notes.

For the kernel module, ks959_speed.ko stays loaded after probe returns -ENODEV and doesn't claim the device, which is good. On replug the dongle resets to 9600 and the module probe automatically sets it back to the configured baud — that's actually convenient even though the docs frame it as single-use. There's a potential conflict if the module remains loaded while nusb detaches and claims the device.

Checking the Cargo.toml for unused dependencies like crc, futures-lite, and libc. The SETUP.md references kernel 7.0.14 which doesn't exist — Linux is still on 6.x as of mid-2026, so that's likely a hallucination. The PROTOCOL.md has a CRC line with what looks like a typo ("0x000" instead of "0x0000") and the init value of 0x0000 is unverified against the actual libdivecomputer implementation. There's also a test vector to check for the mask_index calculation.

The RX deobfuscation documentation is unclear about whether the dongle inserts a fixed garbage byte or if the decoded value depends on the raw input. The "every 255 real bytes" phrasing is fine. The STATUS.md note about IR link quality is concerning — it could indicate a fundamental issue with RX counter desynchronization if bytes are lost during USB transfers or FIFO overflows at 115200 baud, which would permanently break the protocol.

The half-duplex nature of IrDA SIR raises questions about echo: if the dongle receives its own TX transmission, the RX FIFO would contain echoed command bytes. But the dctool logs show the KS-959 responds directly without echoing sent bytes, so either the device suppresses echo or the kernel driver filters it. The 10ms polling latency and 3000ms timeout are acceptable.

The default baud rate of 9600 doesn't match the Donatello's requirement of 115200, making the default invocation useless without the kernel module — users must know to specify `--baud 115200`. The symlink existence check is redundant but harmless. The PTY buffer (4096–8192 bytes) could theoretically cause issues during large transfers, though active reading during dive downloads likely prevents problems. The bigger concern: if the user runs dctool twice without restarting, the second session won't detect a baud change and won't reset the counter, but this is actually fine as long as both the host and device counters stay synchronized — the device's counter persists across sessions as long as no bytes are lost.

Buffer purging via tcflush on the PTY slave discards line-discipline data but doesn't desync the counter since deobfuscation happens before the PTY write. The deobfuscation logic correctly skips when the counter is zero, matching the kernel behavior. Error propagation from `check_baud_rate_change` inside the select branch will exit the bridge, but tcgetattr on the retained slave fd shouldn't fail. I need to verify the kmod Makefile, check that referenced documentation files actually exist (docs/README.md, DESIGN.md, KNOWLEDGE.md), and grep for usage of `futures-lite` and `libc` dependencies.

The PTY master fd is registered with AsyncFd for polling, which is safe since the fd remains valid through the PtyBridge lifetime and drop order is correct. The 10ms RX poll cycle does control-IN transfers even when idle, matching the kernel driver behavior. DTR/RTS control and purge operations work through the PTY shim using tcflush and TIOCM ioctl calls. The speed change uses wValue=0x0200 as the speed change identifier in the setup request, consistent with the kernel driver implementation.

For end-to-end viability, the main risk factors are bulk data handling at 516-byte packets through a 2048-byte FIFO polled every 10ms (manageable), the garbage byte every 256 bytes (already handled), and counter desynchronization on lost bytes which could be catastrophic—though detecting the AA AA AA protocol header and brute-forcing the counter offset across 256 possibilities could recover from this. Half-duplex turnaround timing looks safe since the device waits for ACK after sending full packets before transmitting again, avoiding collision risk.

Now I need to verify the test count with cargo test, check clippy and formatting, review the kernel module Makefile and git history, confirm documentation files exist, search for any futures_lite or libc dependencies, and cross-check the kernel version claim in AGENTS.md against the actual system to see if SETUP.md was accurate.

_Thinking:_

I've reviewed the docs. Now I'll verify everything by running the tests, checking lints, inspecting the remaining files, and looking for unused dependencies.

**Tool: bash**

**Input:**
```json
{
  "command": "cargo test 2>&1 | tail -20"
}
```

**Output:**
```
test usb_dongle::tests::deobfuscate_basic ... ok
test usb_dongle::tests::deobfuscate_counter_persistence ... ok
test usb_dongle::tests::deobfuscate_counter_wrap ... ok
test usb_dongle::tests::deobfuscate_empty ... ok
test usb_dongle::tests::fragmentation_single ... ok
test usb_dongle::tests::fragmentation_sizes ... ok
test usb_dongle::tests::max_fragment_size ... ok
test usb_dongle::tests::obfuscate_invertible ... ok
test usb_dongle::tests::obfuscate_known_vector ... ok
test usb_dongle::tests::obfuscate_padded_length ... ok
test usb_dongle::tests::obfuscate_padding_is_zero ... ok
test usb_dongle::tests::speed_payload_115200 ... ok
test sir_framing::tests::roundtrip_large_payload ... ok
test usb_dongle::tests::speed_payload_9600 ... ok
test usb_dongle::tests::unsupported_speed ... ok
test pty_bridge::tests::data_roundtrip_master_to_slave ... ok
test pty_bridge::tests::data_roundtrip_slave_to_master ... ok

test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s


```

**Tool: read**

**Input:**
```json
{
  "filePath": "/home/vincent/www/ks959-bridge/kmod/Makefile"
}
```

**Output:**
```
<path>/home/vincent/www/ks959-bridge/kmod/Makefile</path>
<type>file</type>
<content>
1: obj-m := ks959_speed.o
2: 
3: KDIR ?= /lib/modules/$(shell uname -r)/build
4: 
5: all:
6: 	$(MAKE) -C $(KDIR) M=$(PWD) modules
7: 
8: clean:
9: 	$(MAKE) -C $(KDIR) M=$(PWD) clean
10: 
11: install: all
12: 	$(MAKE) -C $(KDIR) M=$(PWD) modules_install
13: 	depmod -a
14: 
15: .PHONY: all clean install

(End of file - total 15 lines)
</content>
```

**Tool: glob**

**Input:**
```json
{
  "pattern": "*.md"
}
```

**Output:**
```
/home/vincent/www/ks959-bridge/README.md
/home/vincent/www/ks959-bridge/AGENTS.md
/home/vincent/www/ks959-bridge/docs/TESTING.md
/home/vincent/www/ks959-bridge/docs/STATUS.md
/home/vincent/www/ks959-bridge/docs/ARCHITECTURE.md
/home/vincent/www/ks959-bridge/docs/PROTOCOL.md
/home/vincent/www/ks959-bridge/docs/SETUP.md
/home/vincent/www/ks959-bridge/reference/irda/README.md
/home/vincent/www/ks959-bridge/reference/linux/Documentation/filesystems/path-lookup.md
```

**Tool: glob**

**Input:**
```json
{
  "pattern": "docs/*"
}
```

**Output:**
```
/home/vincent/www/ks959-bridge/docs/TESTING.md
/home/vincent/www/ks959-bridge/docs/PROTOCOL.md
/home/vincent/www/ks959-bridge/docs/STATUS.md
/home/vincent/www/ks959-bridge/docs/ARCHITECTURE.md
/home/vincent/www/ks959-bridge/docs/SETUP.md
```

**Tool: grep**

**Input:**
```json
{
  "pattern": "futures_lite|futures-lite|libc::",
  "path": "src"
}
```

**Output:**
```
No files found
```

**Tool: bash**

**Input:**
```json
{
  "command": "uname -r; git log --oneline -10; git status --short"
}
```

**Output:**
```
7.0.14-101.fc43.x86_64
c702786 Updating documentation, adding kernel module to change interface bauds and improving the bridge
f5dd1a4 Clarifying the goal of the project
103789d Adding AGENTS.md and reformating the docs.
4163f47 Renaming project
e9815e0 Consolidating documentation even more
234a668 Initial commit
7fabf0c Initial commit
AD docs/README.md
?? reference/irda/
?? reference/libdivecomputer/
?? reference/linux/

```

---

## Assistant (Architect · Claude Fable 5 · 11.5s)

_Thinking:_

The kernel version checks out—it's actually reporting 7.0.14 on this system, so that's not a hallucination. But I'm noticing a documentation issue: `docs/README.md` is staged for deletion while `AGENTS.md` still references it, creating an inconsistency between the files.

Now I'll run clippy and the formatter checks to catch any linting issues.

_Thinking:_

All 45 tests pass and the kernel version matches the documentation. I'll run the linters next to check for any remaining issues.

**Tool: bash**

**Input:**
```json
{
  "command": "cargo clippy 2>&1 | tail -30"
}
```

**Output:**
```
    Checking ks959-bridge v0.1.0 (/home/vincent/www/ks959-bridge)
warning: it looks like the same item is being pushed into this `Vec`
   --> src/sir_framing.rs:123:9
    |
123 |         frame.push(XBOF);
    |         ^^^^^
    |
    = help: consider using `vec![XBOF;SIZE]`
    = help: or `frame.extend(std::iter::repeat_n(XBOF, SIZE))`
    = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.95.0/index.html#same_item_push
    = note: `#[warn(clippy::same_item_push)]` on by default

warning: `ks959-bridge` (bin "ks959-bridge") generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.68s

```

**Tool: bash**

**Input:**
```json
{
  "command": "cargo fmt --check 2>&1 | head -20; echo \"fmt exit: $?\""
}
```

**Output:**
```
fmt exit: 0

```

**Tool: bash**

**Input:**
```json
{
  "command": "git show :docs/README.md 2>/dev/null | head -40"
}
```

**Output:**
```
# ks959-bridge Documentation

## What This Project Does

Bridges a **Kingsun KS-959** IrDA USB dongle to **libdivecomputer** so a **Cressi Donatello** dive computer can download dives over infrared on modern Linux kernels (4.17+ removed the IrDA subsystem).

```
  Subsurface / dctool
       |
  /tmp/cressi-irda (PTY slave — looks like a serial port)
       |
  ks959-bridge (this project)
       |
  USB control transfers
       |
  Kingsun KS-959 dongle (IrDA SIR physical layer)
       |
  Infrared
       |
  Cressi Donatello
```

## Documentation Index

| Document | Contents |
|----------|----------|
| [SETUP.md](SETUP.md) | Build, environment, hardware prerequisites, kernel module |
| [PROTOCOL.md](PROTOCOL.md) | USB protocol, IrDA SIR, Cressi Donatello wire format |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Code structure, modules, event loop, dependencies |
| [STATUS.md](STATUS.md) | What works, what doesn't, known issues, the speed change saga |
| [TESTING.md](TESTING.md) | How to test with dctool, debugging commands, log analysis |

## Quick Start

```bash
# Build
cargo build --release

# Load kernel module (sets dongle to 115200 baud)
sudo insmod kmod/ks959_speed.ko baud=115200

```

---

## Assistant (Architect · Claude Fable 5 · 79.4s)

_Thinking:_

I'm reconsidering whether reading that single reference file would actually violate the spirit of the instruction. The user's concern was about context flooding from the entire reference folder, but a targeted 600-line file doesn't create that problem, so it might be worth examining.

But the directive was explicit: don't go into the reference folder. Respecting that boundary is more important than the marginal gain from verification, so I'll document this as a limitation in my report and offer targeted file comparison as a follow-up option.

Let me also verify the imports and dependencies — checking that `warn` is actually used in main.rs, and scanning Cargo.toml for unused dependencies like `crc`, `futures-lite`, and `libc`, plus confirming whether the tokio features being pulled in are actually needed by the code.

Now I'm compiling the complete findings organized by severity, starting with critical correctness issues. The RX-counter reset logic has a flaw: it only triggers when the baud rate changes, but in the recommended workflow with `--skip-speed-change`, the baud rate stays the same so the reset never fires — contradicting what STATUS.md claims. This leaves the session vulnerable to desync from stray IR bytes. The fix would be to reset whenever a PTY baud transition is observed, regardless of whether it differs from the current value. There's also a data loss problem in the RX path where partial or blocked PTY writes just get dropped instead of being retried or buffered.

The PTY slave is never put into raw mode by the bridge itself, only by dctool later. This means the line discipline is in canonical mode with echo enabled, causing bytes from the dongle to echo back to the master and get retransmitted over IR, creating a loop. The bridge keeps the slave fd open so the line discipline is fully active. A quick fix would be to call cfmakeraw and tcsetattr in PtyBridge::new before dctool connects. There's also the known issue that counter desync has no in-band recovery mechanism — a single lost byte at 115200 can cause unrecoverable desynchronization.

The usbfs speed-change path is dead code that always fails with STALL, yet the code treats it as a fallback. With default 9600 baud or without the kernel module, the bridge claims the dongle is at the target speed when it's actually still at 9600, silently producing garbage. The recommendation is to remove the fake fallback and either make non-9600 without the kernel module a hard error or redesign the speed-change mechanism entirely. The in-band speed path through dctool also always fails because the kernel module is what actually does the real work.

The `--baud` flag semantics are problematic: the default 9600 is useless for the only supported target device which needs 115200, and with skip-speed-change enabled, the flag becomes just an assertion rather than a functional parameter.

On the kernel module side, I'm reconsidering the replug behavior. When the dongle is physically unplugged and replugged while the module is loaded, the device re-enumerates and the probe runs again, resetting speed to 9600 before the module sets it back to 115200 — so replug is actually self-healing. More interestingly, rmmod followed by insmod on an already-connected device would trigger probe again since the driver core attempts to bind the newly-registered driver to all unbound matching devices. The docs frame this as a one-time limitation, but it's actually more flexible than described. The real constraint is whether usbfs has claimed the interface, which would prevent the kernel driver from binding.

On the medium-severity issues: transient USB errors like EPIPE or timeouts currently kill the main loop and remove the symlink, leaving dctool confused mid-session — these should probably retry with backoff instead. The AsyncFd readiness handling works but isn't optimal; each PTY write triggers an extra wakeup and syscall cycle because the pattern doesn't loop reads until EAGAIN. And there's a security concern with the predictable symlink name in /tmp when running as root.

The `Ok(0)` branch from master read is actually dead code since the bridge holds the slave fd open, so there's no EOF signal when dctool disconnects — this ties into the broader issue of lacking reliable new-session detection. I should also clean up unused dependencies like `crc`, `futures-lite`, and `libc`, and fix a Clippy warning about duplicate pushes.

There are some repo hygiene issues: AGENTS.md references a deleted README, and the .gitignore isn't catching untracked reference directories including a full kernel tree. I'll need to check and update that. There are also doc typos in PROTOCOL.md around the CRC init value and an incomplete claim about the garbage byte decoding.

The core math checks out — obfuscation, deobfuscation counter semantics, SIR CRC values, and fragmentation calculations all match the kernel driver behavior. All 45 tests pass, formatting is clean, and the overall architecture of bridging PTY to USB control transfers is the simplest viable approach given the constraints. The real validation comes from the CMD_VERSION round-trip on actual hardware, which exercises the full TX/RX pipeline and IR link.

The remaining risks are sustained throughput issues — counter desync, PTY backpressure, and lack of recovery — plus some inconsistencies between code comments and the status document. I'm spot-checking a few more details: the baud rate check timing aligns with dctool's behavior (set baud, sleep 100ms, purge slave input, then write), tcflush correctly discards queued data, and the bridge forwarding PTY data before dctool's termios purge isn't a problem. The RX FIFO size of 2048 bytes matches the wLength claim in the docs. Timing-wise, the 10ms polling adds latency per packet, but a 516-byte packet at 115200 baud takes ~45ms anyway, and the 3-second timeout is reasonable. Half-duplex echo isn't present for CMD_VERSION based on empirical testing.

Now I'm verifying the tokio features — AsyncFd requires the "net" feature on Unix, so that's necessary. Checking the .gitignore next.

_Thinking:_

I'll do two final checks, then write the report.

**Tool: bash**

**Input:**
```json
{
  "command": "cat .gitignore 2>/dev/null; echo \"---\"; git ls-files | grep -v '^reference/' | head -30; echo \"---\"; du -sh reference 2>/dev/null"
}
```

**Output:**
```
/target
.idea/---
.gitignore
AGENTS.md
Cargo.lock
Cargo.toml
LICENSE
README.md
docs/ARCHITECTURE.md
docs/PROTOCOL.md
docs/README.md
docs/SETUP.md
docs/STATUS.md
docs/TESTING.md
kmod/..module-common.o.cmd
kmod/.Module.symvers.cmd
kmod/.ks959_speed.ko.cmd
kmod/.ks959_speed.mod.cmd
kmod/.ks959_speed.mod.o.cmd
kmod/.ks959_speed.o.cmd
kmod/.module-common.o
kmod/.modules.order.cmd
kmod/Makefile
kmod/Module.symvers
kmod/ks959_speed.c
kmod/ks959_speed.ko
kmod/ks959_speed.mod
kmod/ks959_speed.mod.c
kmod/ks959_speed.mod.o
kmod/ks959_speed.o
kmod/modules.order
src/main.rs
---
7.4G	reference

```

**Tool: bash**

**Input:**
```json
{
  "command": "cargo tree --depth 1 2>/dev/null | head -20"
}
```

**Output:**
```
ks959-bridge v0.1.0 (/home/vincent/www/ks959-bridge)
├── anyhow v1.0.103
├── clap v4.6.1
├── crc v3.4.0
├── futures-lite v2.6.1
├── libc v0.2.186
├── nix v0.29.0
│   [build-dependencies]
├── nusb v0.1.14
├── thiserror v2.0.18
├── tokio v1.52.3
├── tracing v0.1.44
└── tracing-subscriber v0.3.23

```

---

## Assistant (Architect · Claude Fable 5 · 86.0s)

_Thinking:_

I'm noticing some repository hygiene issues—kernel module build artifacts like .o and .ko files are committed to git, and the .gitignore file is missing a trailing newline. There's also a large untracked reference directory taking up 7.4GB, though I'm not sure if all the files in there should be tracked or ignored.

Now I need to organize my findings into a structured report: starting with the verdict, then listing what's verified as working, followed by critical bugs, documentation issues, design risks, code quality concerns, repository problems, and quick wins I can suggest. I'll keep it concrete with file and line references where possible, and note that I couldn't fully verify everything against the reference directory.

All checks done. Here is the audit document.

---

# ks959-bridge — Sanity & Risk Audit

**Verdict up front:** the architecture is sane and is the simplest viable approach given the constraints (PTY ↔ USB control-transfer bridge, kernel module only for the one transfer usbfs blocks). The hard part — a full CMD_VERSION round-trip over real IR — is already proven on hardware, which simultaneously validates TX obfuscation, RX de-obfuscation, speed change, and the PTY plumbing. **However**, I found one confirmed logic bug that contradicts the documentation, two data-path bugs that will plausibly break sustained dive downloads, and several places where code comments and docs contradict each other. The project can work, but the remaining failure surface is concentrated exactly in the untested part (bulk transfer phase), and the current code is not ready for it.

**Audit limitation:** per your instruction I did not open `reference/`. That means I could **not** verify `usb_dongle.rs` against the canonical `reference/ks959-sir.c` (counter semantics, `wValue` meaning, RX request fields), nor the Donatello packet format against `cressi_goa.c`. From memory of the upstream kernel driver, the deobfuscation logic (`++counter; byte ^ counter ^ 0x55; skip when counter==0`) matches, but memory is not verification. I maintain that a targeted read of that *one* file (~700 lines, top of `reference/`, not the kernel tree) is the single highest-value check left undone — say the word and I'll do it.

---

## 1. CRITICAL — Confirmed bugs

### C1. The documented "RX counter reset on new client" never fires in the recommended workflow
`src/main.rs:181–207`. The reset is gated on `new_baud != current_baud`:

```
--baud 115200 --skip-speed-change   → current_baud = 115200
dctool opens PTY, sets 115200       → check_baud_rate_change() → Some(115200)
115200 != 115200                     → false → reset_rx_counter() NEVER RUNS
```

`docs/STATUS.md` explicitly claims this protection exists ("When dctool opens the PTY and sets the baud rate, the bridge resets the RX counter. This handles the case where stale bytes arrive between bridge startup and dctool connection"). **It does not, in the only workflow the docs recommend.** Any stray IR byte received between bridge startup and dctool connecting desyncs the counter permanently and silently. This may well be the real cause of STATUS.md known issue #4 ("version OK, logbook garbage").

**Fix:** treat any `Some(_)` from `check_baud_rate_change()` as "new client session": always `reset_rx_counter()` + drain the dongle, and only attempt `set_speed()` when the value actually differs. Compounding factor: the bridge holds its own slave fd forever (`pty_bridge.rs:104`), so it never sees HUP/EOF on the master — baud-change polling is its *only* new-client signal, and it's broken.

### C2. RX bytes are silently dropped on PTY backpressure
`src/main.rs:254–270`. Partial write → warn and **drop the remainder**; `EAGAIN` → **drop the whole buffer**. The tty line-discipline buffer is ~4–8 KB. A dive download streams 516-byte packets continuously; one slow `read()` in dctool corrupts the stream with no recovery. Note this doesn't desync the USB counter (decode already happened), but it guarantees protocol-level CRC failures downstream.

**Fix:** keep a pending `VecDeque<u8>`, retry on the next poll tick (or register WRITABLE interest on the AsyncFd). Do not drop.

### C3. The PTY slave is never put into raw mode
`pty_bridge.rs:119–167`. `openpty(None, None)` leaves default termios: `ICANON | ECHO` on. The bridge keeps a slave fd open, so the line discipline is fully active. Between bridge startup and dctool's `cfmakeraw`, any byte from the dongle written to the master is **echoed back to the master** — the bridge reads it and retransmits it over IR to the Donatello. That's an echo loop into the device during exactly the window when you're fumbling to put it in PC mode. Input processing (`ICRNL` etc.) also mangles bytes in that window. Telling detail: the unit tests carefully call `cfmakeraw` on the slave (`open_slave_raw`, line 275) — production code never does.

**Fix (quick win):** `cfmakeraw` + `tcsetattr(TCSANOW)` on the slave inside `PtyBridge::new()`. dctool will overwrite termios at open; that's fine.

---

## 2. HIGH — Misleading code and docs

### H1. The usbfs speed-change "fallback" is dead code presented as plausible
STATUS.md's own empirical table says `bRequestType=0x41` → **STALL (confirmed)**. Yet `usb_dongle.rs:219–234` has a long comment arguing the firmware "may not check the type" — an hypothesis the project already disproved — and `main.rs:93–114` treats it as a fallback that then *assumes success on failure* (`current_baud = args.baud` after a STALL). Consequences:

- Default invocation (`--baud 9600`) against a Donatello can never work, silently.
- `--baud 115200` without the kernel module: bridge warns, then lies to itself about the dongle's speed and produces IR garbage.
- On every dctool baud change, `set_speed()` is called, always STALLs, always warns — pure theater.

**Fix:** delete the fallback path. If `baud != 9600` and `--skip-speed-change` is absent, hard-error with the insmod instruction. An option that "attempts something known-impossible then proceeds on faith" is worse than an error.

### H2. "Kernel module can only be used once per USB plug cycle" is likely wrong
`kmod/ks959_speed.c:20`, repeated in SETUP.md/STATUS.md/AGENTS.md. Returning `-ENODEV` from probe leaves the interface *unbound*. `rmmod ks959_speed && insmod ks959_speed.ko` re-registers the driver, and the USB core re-probes all unbound matching devices → the speed change runs again, no replug needed (as long as nusb isn't holding the interface at that moment). Also, physically replugging *while the module stays loaded* re-triggers probe automatically — which is actually a self-healing feature the docs present as a limitation. Both claims are trivially testable on hardware and matter operationally (the current belief forces unnecessary replug rituals during a 60-second hibernation window).

### H3. No detection or recovery for counter desync — and the symptom has already been observed
Docs acknowledge desync is fatal, but the design treats it as an act of god. It isn't: the Donatello protocol has known plaintext (`AA AA AA` headers, `0x04`/`0x06` control bytes, fixed 516-byte packet structure). On garbage detection you can brute-force the 256 possible counter offsets against the expected header and resync in-band. Given that STATUS.md #4 reports garbage mid-session *on the only sustained-transfer test ever run*, this is the project's #1 open risk for "could it eventually work properly." Until the desync cause of that test is root-caused (my money is on C1 or a lost byte, not "IR interference"), end-to-end reliability is unproven.

---

## 3. MEDIUM

| # | Issue | Location | Notes |
|---|-------|----------|-------|
| M1 | Any single USB RX error or PTY write error `break`s the main loop, kills the bridge, removes the symlink mid-session | `main.rs:274–277, 267–269` | Transient errors should retry with backoff; dying mid-download is the worst outcome |
| M2 | `Ok(n)` read path never clears AsyncFd readiness → one guaranteed spurious wakeup + `EAGAIN` syscall per event | `main.rs:172–230` | Correct pattern: read in a loop until `EAGAIN`, then `clear_ready()`. Works today, wastes cycles, fragile if refactored |
| M3 | Predictable symlink name in world-writable `/tmp`, process runs as root | `pty_bridge.rs:137–147` | Sticky bit limits exploitability (creation race just errors), but `/run/` or a mandated user path is the right default |
| M4 | `Ok(0)` "PTY slave closed (EOF)" branch is dead code — the bridge's own slave fd keeps the line open forever | `main.rs:173–176` | Harmless, but reveals there is *no* client-disconnect signal at all (feeds C1) |
| M5 | Baud change handling: no settle delay after speed change before forwarding the buffered command bytes | `main.rs:181–219` | Moot today (H1 makes the path dead) but a trap if the speed path is ever fixed |
| M6 | `speed_payload` flags byte hardcodes 8N1 (`0x03`); dctool's actual parity/stop settings from termios are never propagated | `usb_dongle.rs:150` | Fine for Donatello (8N1), wrong for any other device — deserves a comment |

---

## 4. Docs — errors and drift

1. **`docs/README.md` is deleted from the worktree but still staged in git** (`git status`: `AD docs/README.md`) and still referenced by `AGENTS.md`. Decide: restore it or `git rm` it and fix AGENTS.md.
2. **PROTOCOL.md line 181:** `checksum_crc16_ccitt(packet+3, size+2, 0x000, 0x0000)` — `0x000` is a typo, and I could not verify the init value against `cressi_goa.c` (see audit limitation).
3. **PROTOCOL.md line 100:** "the byte at that position decodes to 0x95" — with `counter==0`, decoded = `raw ^ 0x55`; it decodes to 0x95 only if the raw byte is fixed at 0xC0. Either the dongle inserts a constant filler (say so) or this sentence is wrong. As written it's a trap for anyone reading trace logs.
4. **ARCHITECTURE.md/STATUS.md vs code on the counter-reset guarantee** — see C1; the docs describe a protection the code doesn't deliver.
5. **Git history has two "Initial commit"s** (`7fabf0c`, `234a668`) — cosmetic, but typical LLM-workflow residue.

---

## 5. Code smells / maintainability

- **`main.rs` event loop is one 130-line `select!` body** with baud-detection, speed-change policy, SIR framing, and error policy inlined. It's still readable, but the next feature (write buffering per C2, retry policy per M1) will tip it over. Extract `handle_pty_readable()` / `handle_rx_tick()`.
- **Leftover model self-talk in a test comment**, `sir_framing.rs:606`: *"The second BOF in the frame resets... wait, after the second BOF the data and EOF should still be there."* Harmless but exactly the "drowning in own context" artifact you asked me to flag. The test itself is valid.
- **`byte_stuff()` returns a `Vec` per byte** (`sir_framing.rs:86`) — allocation per byte on the TX path. Irrelevant while SIR framing is off by default; would matter if it ever became the default.
- **One clippy warning** (`same_item_push`, `sir_framing.rs:123`) — the "run clippy before commit" rule in AGENTS.md isn't being honored.
- **Unused dependencies:** `crc` (documented), `futures-lite`, `libc` — the latter two are undocumented dead weight. Likely-unneeded tokio features: `sync`, `io-util` (`net` **is** required by `AsyncFd`).
- **Kernel module build artifacts committed to git:** `kmod/*.o`, `*.ko`, `*.mod.c`, `.cmd` files, `Module.symvers`, `modules.order` are tracked. These are machine/kernel-specific garbage; a stale `.ko` in-repo is actively dangerous (someone insmods a module built for another kernel).
- **`.gitignore` is 2 entries with no trailing newline**; `reference/` (7.4 GB!) is partially untracked, partially tracked, not ignored. One `git add .` away from a catastrophe.

---

## 6. What checks out (verified in this audit)

- **45/45 tests pass**, `cargo fmt --check` clean. Test quality is genuinely good — hand-derived vectors, wrap-around cases, counter persistence, CRC residue (`0xF0B8`), check value (`0x6F91`).
- **Obfuscation math verified by hand:** len=3 → index 5 → `'h'`=0x68 → mask 0x3D → `[0x3C,0x3F,0x3E]`. Matches PROTOCOL.md. Fragment limit `(256 & ~7) − 16 = 240` and padding formula are self-consistent.
- **The `tcgetattr`-polling-over-TIOCPKT decision is correct** — `TIOCPKT_IOCTL` really doesn't fire for plain baud changes on Linux.
- **The `check_ctrlrecip` analysis is accurate** — usbfs does reject `Class+Interface` with `wIndex` pointing at a nonexistent interface; the kernel-module bypass is the legitimate minimal solution (and the module itself is clean, correct kernel code).
- **The compile-time CRC table replacing the `crc` crate is fine** — matches the kernel's reflected 0x8408, `table[1]==0x1189` asserted.
- **SETUP.md's "kernel 7.0.14"** — I suspected hallucination; `uname -r` confirms `7.0.14-101.fc43.x86_64` on this machine. Not a doc bug.
- **Architecture choice is Occam-compliant:** a userspace bridge + 129-line kernel module beats resurrecting the IrDA stack (empirically a dead end via the IrLAP evidence) and beats a full kernel driver.

---

## 7. Quick wins (not applied, per your instruction)

| # | Fix | Effort |
|---|-----|--------|
| 1 | `cfmakeraw` the slave in `PtyBridge::new()` (C3) | 4 lines |
| 2 | Move `reset_rx_counter()` (+ a dongle drain) outside the `new_baud != current_baud` check in `main.rs` (C1) | 5 lines |
| 3 | Hard-error instead of warn-and-pretend when usbfs speed change fails without `--skip-speed-change` (H1) | 10 lines |
| 4 | Buffer-and-retry instead of dropping on partial/blocked PTY writes (C2) | ~30 lines |
| 5 | Remove `crc`, `futures-lite`, `libc`, tokio `sync`/`io-util` from Cargo.toml | 3 lines |
| 6 | Fix clippy `same_item_push` (`frame.resize(extra_bofs, XBOF)` or `extend(repeat_n(...))`) | 1 line |
| 7 | `git rm --cached` the kmod build artifacts; extend `.gitignore` (`kmod/*.o`, `kmod/*.ko`, `kmod/.*.cmd`, `kmod/*.mod*`, `Module.symvers`, `modules.order`, `reference/`) | 5 min |
| 8 | Resolve the `docs/README.md` staged-deleted limbo + AGENTS.md reference | 2 min |
| 9 | Fix PROTOCOL.md `0x000` typo and the "decodes to 0x95" claim | 2 min |
| 10 | Delete the self-talk comment in `sir_framing.rs:606` | 1 min |

---

## 8. Bottom line

**Is the project sane?** Yes. The layering is right, the reverse-engineering is coherent, the empirical method (STALL tables, trace-log forensics on the desync bug) is solid engineering, and the hardware-proven CMD_VERSION round-trip removes most existential risk.

**Will it eventually work?** Probably, but not with the current code. The three data-path issues (C1, C2, C4/H3) all sit precisely in the untested bulk-download phase, and the one sustained-transfer test ever run already produced the predicted failure signature. Root-cause that garbage-logbook session before believing "IR interference" — the code offers at least two mundane explanations. Priority order: fix C1 + C3 (trivial), fix C2, re-run the end-to-end test with `RUST_LOG=trace`, and only then decide whether counter-resync brute-forcing (H3) is needed.

---

