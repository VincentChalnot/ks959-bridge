# Building Arduino IR interface for Cressi Donatello

**Session ID:** ses_0e8667f17ffenFUElFYpn2B6WO
**Created:** 6/30/2026, 10:16:03 AM
**Updated:** 6/30/2026, 10:31:00 AM

---

## User

You are tasked with building an Arduino-based IR interface to replace the Cressi BT Interface dock for the Cressi
Donatello dive computer.

## Context

The Cressi Donatello uses a proprietary IR (infrared) optical port to transfer dive data to an external dock. The
official dock (Cressi BT Interface, ~70€) is essentially:

- A CR2032-powered device
- A physical dock that aligns an IR LED/receiver with the Donatello's optical port
- A FTDI/CH340 USB-to-serial chip that presents as /dev/ttyUSBx on Linux
- A Bluetooth module (which we don't need)

The `libdivecomputer` open-source library already implements the Donatello protocol. Subsurface (which uses
libdivecomputer) can communicate with the Donatello over a serial port.

## Your task

1. **Locate the Cressi Donatello driver in libdivecomputer source code.**
   Start here: https://github.com/subsurface/libdivecomputer
   Look for files referencing "donatello", "cressi", or related models under `src/`.
   Read and summarize the IR communication protocol:
    - Baud rate / timing
    - Byte framing, start/stop bits
    - Command/response structure
    - Any special IR modulation frequency expected

2. **Design an Arduino sketch** that:
    - Uses a 38kHz IR LED (standard TSOP/TSAL-type) to send commands to the Donatello
    - Uses an IR receiver (TSOP1738 or equivalent) to receive the response
    - Bridges this over USB Serial (Arduino acts as a USB-Serial adapter via its native USB or a CH340/FTDI)
    - Presents itself to the host PC as a standard serial port at the correct baud rate
    - Is compatible with how libdivecomputer/Subsurface expects to talk to the device

3. **Identify the correct IR modulation frequency** used by the Cressi Donatello (likely 38kHz but confirm from source).

4. **Write the complete Arduino sketch** (`.ino` file), handling:
    - IR TX using `IRremote` library or direct timer-based PWM
    - IR RX using hardware interrupt or `IRremote` library
    - USB serial bridging at the correct baud rate
    - Proper timing and framing matching the libdivecomputer driver

5. **Write a wiring diagram** (text-based ASCII is fine) showing:
    - IR LED connection (with appropriate resistor for ~50mA peak current)
    - IR receiver module connection
    - Power considerations (5V Arduino, 3.3V if needed)

## Constraints

- Hardware available: Arduino Uno or Nano, standard 38kHz IR LED (TSAL6400 or equivalent), TSOP1738 IR receiver,
  resistors, breadboard
- Target OS: Linux (Fedora), the serial port will be `/dev/ttyUSB0` or `/dev/ttyACM0`
- The goal is for Subsurface on Linux to detect and download dives from the Donatello using this Arduino as the
  interface
- No Bluetooth needed — USB cable only

## Deliverables

- Summary of the Donatello IR protocol extracted from libdivecomputer
- Complete Arduino `.ino` sketch
- Wiring diagram
- Any caveats about physical alignment of the IR components with the Donatello's optical port

---

## Assistant (Plan · MiMo-V2.5-Pro · 140.4s)

_Thinking:_

The user wants me to build an Arduino-based IR interface for the Cressi Donatello dive computer. Let me start by
researching the libdivecomputer source code to understand the Donatello IR protocol.

Let me first explore the libdivecomputer repository to find the relevant driver files.

I'll start by exploring the libdivecomputer source code to find the Cressi Donatello driver and understand the IR
protocol. Let me search the codebase systematically.

**Tool: task**

**Input:**

```json
{
  "description": "Find Cressi Donatello driver",
  "prompt": "Search the libdivecomputer codebase at /home/vincent/www/libdivecomputer for files related to the Cressi Donatello dive computer driver. Look for:\n1. Any files with \"cressi\" or \"donatello\" in the filename under src/\n2. Any header files referencing cressi/donatello\n3. The main driver implementation file\n4. Any protocol documentation\n\nUse glob patterns like **/*cressi*, **/*donatello*, and grep for \"donatello\" and \"cressi\" in source files. Return the full paths of all relevant files found.",
  "subagent_type": "explore"
}
```

**Output:**

```
<task id="ses_0e8666dd0ffekC2IPppc8FgiOm" state="completed">
<task_result>
Here is a comprehensive summary of all files related to the Cressi Donatello dive computer driver in the libdivecomputer codebase.

---

## Key Finding

The Cressi Donatello does **not** have its own dedicated driver files. It is supported through the **Cressi Goa driver family** (`DC_FAMILY_CRESSI_GOA`) with model number `4`. The Donatello shares this driver with several other Cressi computers: Cartesio, Goa, Leonardo 2.0, Michelangelo, Neon, and Nepto.

---

## All Relevant Files

### 1. Core Driver Implementation (the Goa family driver that handles Donatello)

| File | Purpose |
|------|---------|
| `/home/vincent/www/libdivecomputer/src/cressi_goa.c` | Main device driver implementation (communication, dive download, etc.) |
| `/home/vincent/www/libdivecomputer/src/cressi_goa.h` | Driver header file (public API for device and parser creation) |
| `/home/vincent/www/libdivecomputer/src/cressi_goa_parser.c` | Dive data parser implementation (interprets raw dive data) |

### 2. Family Enum Definition (public API header)

| File | Purpose |
|------|---------|
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/common.h` | Defines `DC_FAMILY_CRESSI_GOA` enum (line 90), used to identify the Donatello's device family |

### 3. Device Registration and Descriptor (model/metadata mapping)

| File | Purpose |
|------|---------|
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | Registers the Donatello as `{"Cressi", "Donatello", DC_FAMILY_CRESSI_GOA, 4, ...}` (line 355) with SERIAL+BLE transport; also defines `dc_filter_cressi()` for BLE device filtering (line 932) with Donatello as model 4 (line 938) |
| `/home/vincent/www/libdivecomputer/src/device.c` | Dispatches `DC_FAMILY_CRESSI_GOA` to `cressi_goa_device_open()` (line 198); includes `cressi_goa.h` (line 51) |
| `/home/vincent/www/libdivecomputer/src/parser.c` | Dispatches `DC_FAMILY_CRESSI_GOA` to `cressi_goa_parser_create()` (line 158) |

### 4. Example/Test Code

| File | Purpose |
|------|---------|
| `/home/vincent/www/libdivecomputer/examples/common.c` | Maps the CLI name `"goa"` to `DC_FAMILY_CRESSI_GOA` with model 2 (line 85) |

### 5. Build System Files

| File | Purpose |
|------|---------|
| `/home/vincent/www/libdivecomputer/src/Makefile.am` | Lists `cressi_goa.h cressi_goa.c cressi_goa_parser.c` as source files (line 59) |
| `/home/vincent/www/libdivecomputer/contrib/android/Android.mk` | Lists `src/cressi_goa.c` and `src/cressi_goa_parser.c` for Android builds (lines 24-25) |
| `/home/vincent/www/libdivecomputer/contrib/msvc/libdivecomputer.vcxproj` | Lists `cressi_goa.c`, `cressi_goa_parser.c`, and `cressi_goa.h` for MSVC builds (lines 192-193, 335) |

### 6. Release Notes / Documentation

| File | Purpose |
|------|---------|
| `/home/vincent/www/libdivecomputer/NEWS` | Documents the addition of Donatello support: "Cressi: Donatello, Michelangelo, Neon" (line 61) |

### 7. Related Sibling Cressi Drivers (for reference)

These are separate driver families for other Cressi models and are **not** used by the Donatello, but are in the same `src/` directory:

| File | Purpose |
|------|---------|
| `/home/vincent/www/libdivecomputer/src/cressi_edy.c` | Cressi Edy/Archimede/IQ-700 driver |
| `/home/vincent/www/libdivecomputer/src/cressi_edy.h` | Cressi Edy header |
| `/home/vincent/www/libdivecomputer/src/cressi_edy_parser.c` | Cressi Edy parser |
| `/home/vincent/www/libdivecomputer/src/cressi_leonardo.c` | Cressi Leonardo/Giotto/Newton/Drake driver |
| `/home/vincent/www/libdivecomputer/src/cressi_leonardo.h` | Cressi Leonardo header |
| `/home/vincent/www/libdivecomputer/src/cressi_leonardo_parser.c` | Cressi Leonardo parser |

---

## No Files Found

- **No files with "donatello" in the filename** -- the Donatello has no standalone source files.
- **No protocol documentation files** -- there are no dedicated protocol docs for the Cressi Goa family in the `doc/` directory.
- **No CMakeLists.txt or meson.build** referencing cressi_goa (the project uses Autotools/Makefile.am as its primary build system).
</task_result>
</task>
```

**Tool: task**

**Input:**

```json
{
  "description": "Find IR protocol details",
  "prompt": "Search the libdivecomputer codebase at /home/vincent/www/libdivecomputer for IR communication protocol details. Look for:\n1. Baud rate settings (grep for \"baud\", \"9600\", \"19200\", \"115200\", \"57600\")\n2. Serial port configuration (grep for \"serial\", \"uart\", \"parity\", \"stopbit\")\n3. IR modulation or carrier frequency settings (grep for \"38000\", \"38khz\", \"infrared\", \"carrier\")\n4. Any protocol constants or command bytes\n\nFocus on files under src/ directory. Return all findings with file paths and line numbers.",
  "subagent_type": "explore"
}
```

**Output:**

```
<task id="ses_0e8666559ffefNAJbGgL75aF04" state="completed">
<task_result>
Here is a comprehensive report of all IR communication protocol details found in the libdivecomputer codebase under `src/`.

---

## 1. Baud Rate Settings

### 1200 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/cressi_edy.c` | 256-257 | `1200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/suunto_solution.c` | 81-82 | `1200, 8, DC_PARITY_NONE, DC_STOPBITS_TWO, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/suunto_eon.c` | 87-88 | `1200, 8, DC_PARITY_NONE, DC_STOPBITS_TWO, DC_FLOWCONTROL_NONE` |

### 2400 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/suunto_vyper.c` | 107-108 | `2400, 8, DC_PARITY_ODD, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` (the only device using ODD parity) |

### 4800 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/zeagle_n2ition3.c` | 153-154 | `4800, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/cressi_edy.c` | 305-306 | `4800, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/citizen_aqualand.c` | 79-80 | `4800, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |

### 9600 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/uwatec_memomouse.c` | 88-89 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/suunto_vyper2.c` | 106-107, 220 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/suunto_d9.c` | 97, 149-150 | Baudrate array `{9600, 115200}`; initial config at 9600 8N1 |
| `/home/vincent/www/libdivecomputer/src/cochran_commander.c` | 124, 305-306 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_TWO, DC_FLOWCONTROL_NONE` (note: 8N2) |
| `/home/vincent/www/libdivecomputer/src/mares_nemo.c` | 104-105 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/mares_darwin.c` | 126-127 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/diverite_nitekq.c` | 169-170 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/oceanic_vtpro.c` | 467-468 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/liquivision_lynx.c` | 241-242 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/oceanic_veo250.c` | 256-257 | `9600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |

### 19200 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/uwatec_aladin.c` | 92-93 | `19200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/reefnet_sensuspro.c` | 87-88 | `19200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/reefnet_sensus.c` | 111-112 | `19200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/hw_ostc.c` | 877 | Baudrate array `{19200, 115200}` (tries both) |

### 38400 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/oceanic_atom2.c` | 890, 895-896 | `38400, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` (default, may upgrade to 115200) |
| `/home/vincent/www/libdivecomputer/src/mares_puck.c` | 110-111 | `38400, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |

### 57600 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/uwatec_smart.c` | 484-485 | `57600, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |

### 115200 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/hw_ostc.c` | 141-142 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/hw_ostc3.c` | 449-450 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/hw_frog.c` | 220-221 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/cressi_leonardo.c` | 184-185 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/cressi_goa.c` | 396-397 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/shearwater_common.c` | 54-55 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/reefnet_sensusultra.c` | 94-95 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/mclean_extreme.c` | 421-422 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/tecdiving_divecomputereu.c` | 365-366 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/divesystem_idive.c` | 184-185 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/divesoft_freedom.c` | 321-322 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/deepsix_excursion.c` | 218-219 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/seac_screen.c` | 341-342 | `115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |
| `/home/vincent/www/libdivecomputer/src/mares_iconhd.c` | 593-594 | `115200, 8, DC_PARITY_EVEN, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` (the only device using EVEN parity at 115200) |
| `/home/vincent/www/libdivecomputer/src/cochran_commander.c` | 149, 175 | 115200 baudrate in layout structs |

### 460800 baud
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/sporasub_sp2.c` | 247-248 | `460800, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE` |

### 850000 baud (custom/odd rate)
| File | Line | Configuration |
|------|------|---------------|
| `/home/vincent/www/libdivecomputer/src/cochran_commander.c` | 200, 225, 250 | 850000 baud in layout structs (EMC models); comment at line 373: "Rates are odd, like 850400 for the EMC" |

### POSIX Serial Baud Rate Mapping Table
| File | Line | Supported Rates |
|------|------|-----------------|
| `/home/vincent/www/libdivecomputer/src/serial_posix.c` | 405-466 | Full POSIX baud rate table: 0 through 4000000, including B0, B50, B75, B110, B134, B150, B200, B300, B600, B1200, B1800, B2400, B4800, B9600, B19200, B38400, B57600, B115200, B230400, B460800, B500000, B576000, B921600, B1000000, B1152000, B1500000, B2000000, B2500000, B3000000, B3500000, B4000000 |

---

## 2. Serial Port Configuration

### Core I/O Stream Configure Function
| File | Line | Description |
|------|------|-------------|
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/iostream.h` | 196 | Public API: `dc_iostream_configure(iostream, baudrate, databits, parity, stopbits, flowcontrol)` |
| `/home/vincent/www/libdivecomputer/src/iostream.c` | 166-174 | Implementation dispatches to vtable; logs `"Configure: baudrate=%i, databits=%i, parity=%i, stopbits=%i, flowcontrol=%i"` |
| `/home/vincent/www/libdivecomputer/src/iostream-private.h` | 56 | Vtable: `dc_status_t (*configure)(dc_iostream_t *iostream, unsigned int baudrate, unsigned int databits, dc_parity_t parity, dc_stopbits_t stopbits, dc_flowcontrol_t flowcontrol)` |

### Parity Enum Definitions
| File | Line | Values |
|------|------|--------|
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/iostream.h` | 41-47 | `DC_PARITY_NONE`, `DC_PARITY_ODD`, `DC_PARITY_EVEN`, `DC_PARITY_MARK`, `DC_PARITY_SPACE` |

### Stop Bits Enum Definitions
| File | Line | Values |
|------|------|--------|
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/iostream.h` | 52-56 | `DC_STOPBITS_ONE`, `DC_STOPBITS_ONEPOINTFIVE`, `DC_STOPBITS_TWO` |

### Flow Control Enum Definitions
| File | Line | Values |
|------|------|--------|
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/iostream.h` | 61-65 | `DC_FLOWCONTROL_NONE`, `DC_FLOWCONTROL_HARDWARE` (RTS/CTS), `DC_FLOWCONTROL_SOFTWARE` (XON/XOFF) |

### Serial Port Implementation (POSIX)
| File | Line | Description |
|------|------|-------------|
| `/home/vincent/www/libdivecomputer/src/serial_posix.c` | 74 | `dc_serial_configure()` function signature |
| `/home/vincent/www/libdivecomputer/src/serial_posix.c` | 370 | Implementation start |
| `/home/vincent/www/libdivecomputer/src/serial_posix.c` | 403-474 | Baud rate setting via `cfsetispeed`/`cfsetospeed` with full switch table |
| `/home/vincent/www/libdivecomputer/src/serial_posix.c` | 499-560 | Parity configuration (`termios.c_cflag` PARENB/PARODD/CMSPAR) |
| `/home/vincent/www/libdivecomputer/src/serial_posix.c` | 533-540 | Stop bits configuration (CSTOPB) |
| `/home/vincent/www/libdivecomputer/src/serial_posix.c` | 546-560 | Flow control (CRTSCTS for hardware, IXON/IXOFF for software) |
| `/home/vincent/www/libdivecomputer/src/serial_posix.c` | 577-607 | Custom baudrate support (Linux `ASYNC_SPD_CUST` / macOS `IOSSIOSPEED`) |

### Serial Port Implementation (Win32)
| File | Line | Description |
|------|------|-------------|
| `/home/vincent/www/libdivecomputer/src/serial_win32.c` | 45 | `dc_serial_configure()` function signature |
| `/home/vincent/www/libdivecomputer/src/serial_win32.c` | 396 | Implementation start |
| `/home/vincent/www/libdivecomputer/src/serial_win32.c` | 412 | `dcb.BaudRate = baudrate` |
| `/home/vincent/www/libdivecomputer/src/serial_win32.c` | 421-444 | Parity via `dcb.Parity` (NOPARITY, EVENPARITY, ODDPARITY, MARKPARITY, SPACEPARITY) |
| `/home/vincent/www/libdivecomputer/src/serial_win32.c` | 447-458 | Stop bits via `dcb.StopBits` (ONESTOPBIT, ONE5STOPBITS, TWOSTOPBITS) |
| `/home/vincent/www/libdivecomputer/src/serial_win32.c` | 463-480 | Flow control via `dcb.fRtsControl`, `dcb.fOutxCtsFlow`, XON/XOFF |

### Wrapper Layers
| File | Line | Description |
|------|------|-------------|
| `/home/vincent/www/libdivecomputer/src/hdlc.c` | 39, 182-186 | HDLC layer passes through to underlying iostream configure |
| `/home/vincent/www/libdivecomputer/src/packet.c` | 37, 178-182 | Packet layer passes through to underlying iostream configure |
| `/home/vincent/www/libdivecomputer/src/custom.c` | 36, 165-172 | Custom iostream layer delegates to user callbacks |

---

## 3. IR (IrDA) Communication Protocol

**No raw infrared carrier frequency (38kHz/38000Hz), modulation, or hardware-level IR settings were found.** The codebase uses the **IrDA protocol stack** (operating over standard serial/UART hardware) rather than directly manipulating IR carrier frequencies. All IR communication is abstracted through the OS-level IrDA socket API.

### IrDA Implementation File
| File | Line | Description |
|------|------|-------------|
| `/home/vincent/www/libdivecomputer/src/irda.c` | 32-41 | Platform detection: `HAVE_AF_IRDA_H` (Win32) or `HAVE_LINUX_IRDA_H` (Linux) |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 55-56 | `DISCOVER_MAX_DEVICES 16`, `DISCOVER_MAX_RETRIES 4` |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 66-71 | `dc_irda_device_t` struct: `address`, `charset`, `hints`, `name[22]` |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 89-106 | IrDA vtable: delegates to socket operations (read/write/poll/sleep/close); `configure` is **NULL** (IrDA has no baud rate/parity/stopbits configuration) |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 134-257 | `dc_irda_iterator_new()`: Opens `AF_IRDA` socket, uses `SOL_IRLMP`/`IRLMP_ENUMDEVICES` for device discovery |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 156 | Socket creation: `socket(AF_IRDA, SOCK_STREAM, 0)` |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 174 | Device enumeration: `getsockopt(fd, SOL_IRLMP, IRLMP_ENUMDEVICES, ...)` |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 212-216 | Win32 device info fields: `irdaDeviceName`, `irdaDeviceID`, `irdaCharSet`, `irdaDeviceHints1`/`irdaDeviceHints2` |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 284-341 | `dc_irda_open()`: Opens connection with address + LSAP selector |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 309-315 | Win32: `SOCKADDR_IRDA` with `irdaAddressFamily=AF_IRDA`, `irdaDeviceID[4]`, service name `"LSAP-SEL%u"` |
| `/home/vincent/www/libdivecomputer/src/irda.c` | 317-320 | Linux: `struct sockaddr_irda` with `sir_family=AF_IRDA`, `sir_addr`, `sir_lsap_sel` |

### IrDA Public Header
| File | Line | Description |
|------|------|-------------|
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/irda.h` | 38 | `typedef struct dc_irda_device_t dc_irda_device_t` |
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/irda.h` | 46 | `dc_irda_device_get_address()` |
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/irda.h` | 54 | `dc_irda_device_get_name()` |
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/irda.h` | 62 | `dc_irda_device_free()` |
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/irda.h` | 74 | `dc_irda_iterator_new()` |
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/irda.h` | 87 | `dc_irda_open(iostream, context, address, lsap)` |

### Transport Enum for IrDA
| File | Line | Description |
|------|------|-------------|
| `/home/vincent/www/libdivecomputer/include/libdivecomputer/common.h` | 49 | `DC_TRANSPORT_IRDA = (1 << 3)` |
| `/home/vincent/www/libdivecomputer/src/context.c` | 291-292 | IrDA transport enabled when `HAVE_AF_IRDA_H` defined |
| `/home/vincent/www/libdivecomputer/src/context.c` | 298-299 | IrDA transport enabled when `HAVE_LINUX_IRDA_H` defined |

### IrDA Devices (Uwatec Smart family)
| File | Line | Device | Model ID |
|------|------|--------|----------|
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 148 | Smart Pro | 0x10 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 149 | Galileo Sol | 0x11 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 150 | Galileo Luna | 0x11 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 151 | Galileo Terra | 0x11 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 152 | Aladin Tec | 0x12 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 153 | Aladin Prime | 0x12 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 154 | Aladin One | 0x12 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 155 | Aladin Tec 2G | 0x13 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 156 | Aladin 2G | 0x13 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 157 | Subgear XP-10 | 0x13 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 158 | Smart Com | 0x14 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 159 | Aladin 2G | 0x15 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 160 | Aladin Tec 3G | 0x15 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 161 | Aladin Sport | 0x15 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 162 | Subgear XP-3G | 0x15 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 165 | Smart Tec | 0x18 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 166 | Galileo Trimix | 0x19 |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 167 | Smart Z | 0x1C |
| `/home/vincent/www/libdivecomputer/src/descriptor.c` | 168 | Subgear XP Air | 0x1C |

### Uwatec Smart IrDA Send/Receive
| File | Line | Description |
|------|------|-------------|
| `/home/vincent/www/libdivecomputer/src/uwatec_smart.c` | 88-113 | `uwatec_smart_irda_send()`: Builds packet as `[cmd][data...]`, writes via `dc_iostream_write()` |
| `/home/vincent/www/libdivecomputer/src/uwatec_smart.c` | 116-184 | `uwatec_smart_irda_receive()`: Reads response with header parsing and validation |
| `/home/vincent/www/libdivecomputer/src/uwatec_smart.c` | 504-506 | Transport dispatch: `DC_TRANSPORT_IRDA` -> uses `uwatec_smart_irda_send`/`uwatec_smart_irda_receive` |
| `/home/vincent/www/libdivecomputer/src/uwatec_smart.c` | 508-509 | `DC_TRANSPORT_SERIAL` -> uses `uwatec_smart_serial_send`/`uwatec_smart_serial_receive` |

---

## 4. Protocol Constants and Command Bytes

### Uwatec Smart Protocol (`/home/vincent/www/libdivecomputer/src/uwatec_smart.c`)
| Line | Constant | Value | Description |
|------|----------|-------|-------------|
| 34 | `DATASIZE_RX` | 255 | Max receive data size |
| 35 | `DATASIZE_TX` | 254 | Max transmit data size |
| 36 | `PACKETSIZE_USBHID_RX` | 64 | USB HID receive packet size |
| 37 | `PACKETSIZE_USBHID_TX` | 32 | USB HID transmit packet size |
| 39 | `CMD_MODEL` | `0x10` | Read device model |
| 40 | `CMD_HARDWARE` | `0x11` | Read hardware version |
| 41 | `CMD_SOFTWARE` | `0x13` | Read software version |
| 42 | `CMD_SERIAL` | `0x14` | Read serial number |
| 43 | `CMD_DEVTIME` | `0x1A` | Read device time |
| 44 | `CMD_HANDSHAKE1` | `0x1B` | Handshake step 1 |
| 45 | `CMD_HANDSHAKE2` | `0x1C` | Handshake step 2 |
| 46 | `CMD_DATA` | `0xC4` | Read dive data |
| 47 | `CMD_SIZE` | `0xC6` | Read data size |
| 49 | `OK` | `0x01` | OK response |
| 50 | `ACK` | `0x11` | Acknowledge |
| 51 | `NAK` | `0x66` | Negative acknowledge |

### Uwatec Memomouse Protocol (`/home/vincent/www/libdivecomputer/src/uwatec_memomouse.c`)
| Line | Constant | Value |
|------|----------|-------|
| 34 | `PACKETSIZE` | 126 |
| 36 | `ACK` | `0x60` |
| 37 | `NAK` | `0xA8` |

### Oceanic Atom2 Protocol (`/home/vincent/www/libdivecomputer/src/oceanic_atom2.c`)
| Line | Constant | Value | Description |
|------|----------|-------|-------------|
| 43 | `CMD_INIT` | `0xA8` | Initialize |
| 44 | `CMD_VERSION` | `0x84` | Read version |
| 45 | `CMD_HANDSHAKE` | `0xE5` | Handshake |
| 46 | `CMD_READ1` | `0xB1` | Read 1 byte |
| 47 | `CMD_READ8` | `0xB4` | Read 8 bytes |
| 48 | `CMD_READ16` | `0xB8` | Read 16 bytes |
| 49 | `CMD_READ16HI` | `0xF6` | Read 16 bytes (high) |
| 50 | `CMD_WRITE` | `0xB2` | Write |
| 51 | `CMD_KEEPALIVE` | `0x91` | Keep alive |
| 52 | `CMD_QUIT` | `0x6A` | Quit |
| 54 | `ACK` | `0x5A` | Acknowledge |
| 55 | `NAK` | `0xA5` | Negative acknowledge |

### Oceanic VT Pro (`/home/vincent/www/libdivecomputer/src/oceanic_vtpro.c`)
| Line | Constant | Value |
|------|----------|-------|
| 39 | `ACK` | `0x5A` |
| 40 | `NAK` | `0xA5` |

### Oceanic Veo250 (`/home/vincent/www/libdivecomputer/src/oceanic_veo250.c`)
| Line | Constant | Value |
|------|----------|-------|
| 37 | `ACK` | `0x5A` |
| 38 | `NAK` | `0xA5` |

### HW OSTC Protocol (`/home/vincent/www/libdivecomputer/src/hw_ostc.c`)
| Line | Constant | Value | Description |
|------|----------|-------|-------------|
| 47 | `ACK` | `0x4B` | "K" for OK |
| 48 | `NAK` | `0x4E` | "N" for not OK |

### Shearwater Protocol (`/home/vincent/www/libdivecomputer/src/shearwater_common.c`)
| Line | Constant | Value |
|------|----------|-------|
| 45 | `NAK` | `0x7F` |

### Cressi Goa Protocol (`/home/vincent/www/libdivecomputer/src/cressi_goa.c`)
| Line | Constant | Value |
|------|----------|-------|
| 50 | `ACK` | `0x06` |
| 37-45 | `CMD_VERSION` | `0x00` |
| 38 | `CMD_SET_TIME` | `0x13` |
| 39 | `CMD_EXIT_PCLINK` | `0x1D` |
| 40 | `CMD_LOGBOOK` | `0x21` |
| 41 | `CMD_DIVE` | `0x22` |
| 42 | `CMD_LOGBOOK_V4` | `0x23` |
| 44 | `CMD_LOGBOOK_BLE` | `0x02` |
| 45 | `CMD_DIVE_BLE` | `0x03` |

### Mares Icon HD (`/home/vincent/www/libdivecomputer/src/mares_iconhd.c`)
| Line | Constant | Value |
|------|----------|-------|
| 88 | `ACK` | `0xAA` |

### DiveSystem iDive (`/home/vincent/www/libdivecomputer/src/divesystem_idive.c`)
| Line | Constant | Value |
|------|----------|-------|
| 42 | `ACK` | `0x06` |
| 44 | `NAK` | `0x15` |

### Halcyon Symbios (`/home/vincent/www/libdivecomputer/src/halcyon_symbios.c`)
| Line | Constant | Value |
|------|----------|-------|
| 33-40 | `CMD_GET_STATUS` | `0x01` |
| 34 | `CMD_GET_SETTINGS` | `0x02` |
| 35 | `CMD_SET_SETTINGS` | `0x03` |
| 36 | `CMD_LOGBOOK_REQUEST` | `0x04` |
| 37 | `CMD_DIVELOG_REQUEST` | `0x05` |
| 38 | `CMD_SET_TIME` | `0x07` |
| 39 | `CMD_LOGBOOK_BLOCK` | `0x08` |
| 40 | `CMD_DIVELOG_BLOCK` | `0x09` |
| 53 | `ACK` | `0x06` |
| 54 | `NAK` | `0x15` |

### Tecdiving DiveComputerEU (`/home/vincent/www/libdivecomputer/src/tecdiving_divecomputereu.c`)
| Line | Constant | Value |
|------|----------|-------|
| 34 | `STX` | `0x7E` |
| 36 | `CMD_INIT` | `0x53` |
| 37 | `CMD_LIST` | `0x57` |
| 38 | `CMD_DIVE` | `0x58` |
| 39 | `CMD_EXIT` | `0x59` |

### McLean Extreme (`/home/vincent/www/libdivecomputer/src/mclean_extreme.c`)
| Line | Constant | Value |
|------|----------|-------|
| 35 | `STX` | `0x7E` |

### Oceans S1 (`/home/vincent/www/libdivecomputer/src/oceans_s1.c`)
| Line | Constant | Value |
|------|----------|-------|
| 36 | `SOH` | `0x01` |
| 38 | `ACK` | `0x06` |
| 39 | `NAK` | `0x15` |

### Seac Screen (`/home/vincent/www/libdivecomputer/src/seac_screen.c`)
| Line | Constant | Value |
|------|----------|-------|
| 40 | `ACK` | `0x09` |
| 41 | `NAK` | `0x30` |

### Sporasub SP2 (`/home/vincent/www/libdivecomputer/src/sporasub_sp2.c`)
| Line | Constant | Value |
|------|----------|-------|
| 46 | `CMD_VERSION` | `0x10` |
| 47 | `CMD_READ` | `0x12` |
| 48 | `CMD_TIMESYNC` | `0x39` |

### Deep Six Excursion (`/home/vincent/www/libdivecomputer/src/deepsix_excursion.c`)
| Line | Constant | Value |
|------|----------|-------|
| 48-66 | `CMD_INFO_HARDWARE` | `0x01` |
| 49 | `CMD_INFO_SOFTWARE` | `0x02` |
| 50 | `CMD_INFO_SERIAL` | `0x03` |
| 51 | `CMD_INFO_LASTDIVE` | `0x04` |
| 54 | `CMD_SETTINGS_DATE` | `0x01` |
| 55 | `CMD_SETTINGS_TIME` | `0x03` |
| 56 | `CMD_SETTINGS_STORE` | `0x27` |
| 57 | `CMD_SETTINGS_LOAD` | `0x28` |
| 60 | `CMD_DIVE_HEADER` | `0x02` |
| 61 | `CMD_DIVE_PROFILE` | `0x03` |
| 62 | `CMD_DIVE_COUNT` | `0x05` |
| 63 | `CMD_DIVE_INDEX_LAST` | `0x06` |
| 64 | `CMD_DIVE_INDEX_FIRST` | `0x07` |
| 65 | `CMD_DIVE_INDEX_PREVIOUS` | `0x08` |
| 66 | `CMD_DIVE_INDEX_NEXT` | `0x09` |

### Suunto EON Steel (`/home/vincent/www/libdivecomputer/src/suunto_eonsteel.c`)
| Line | Constant | Value |
|------|----------|-------|
| 59 | `CMD_INIT` | `0x0000` |
| 63 | `CMD_READ_STRING` | `0x0411` |
| 65 | `CMD_FILE_OPEN` | `0x0010` |
| 66 | `CMD_FILE_READ` | `0x0110` |
| 67 | `CMD_FILE_STAT` | `0x0710` |
| 68 | `CMD_FILE_CLOSE` | `0x0510` |
| 70 | `CMD_DIR_OPEN` | `0x0810` |
| 71 | `CMD_DIR_READDIR` | `0x0910` |
| 72 | `CMD_DIR_CLOSE` | `0x0a10` |
| 74 | `CMD_SET_TIME` | `0x0003` |
| 75 | `CMD_GET_TIME` | `0x0103` |
| 76 | `CMD_SET_DATE` | `0x0203` |
| 77 | `CMD_GET_DATE` | `0x0303` |

---

## 5. Key Findings Summary

1. **No raw IR carrier/modulation settings exist.** The codebase does not contain any references to 38kHz carrier frequency, IR modulation, or direct infrared hardware control. All IR communication is done through the **IrDA protocol stack** (`AF_IRDA` sockets), which handles the physical layer transparently.

2. **IrDA has no serial configuration.** The IrDA vtable at `/home/vincent/www/libdivecomputer/src/irda.c:89-106` has `configure = NULL`, meaning baud rate, parity, and stop bits do not apply to IrDA connections. The IrDA protocol operates at its own fixed physical layer rates (typically 9600 bps SIR, up to 4/16 Mbps for FIR).

3. **Only Uwatec Smart devices use IrDA.** All other dive computer families use serial, USB, USB-HID, Bluetooth, or BLE transports.

4. **Most common serial configuration across all devices is 115200 8N1** (8 data bits, no parity, 1 stop bit, no flow control). Notable exceptions include:
   - Suunto Vyper: 2400 8O1 (odd parity)
   - Cochran Commander: 9600/115200/850000 8N2 (2 stop bits)
   - Mares Icon HD: 115200 8E1 (even parity)
   - Suunto Solution/Eon: 1200 8N2 (1200 baud, 2 stop bits)
</task_result>
</task>
```

---

## Assistant (Plan · MiMo-V2.5-Pro · 6.7s)

_Thinking:_

Excellent! Now I have a good understanding of the codebase. The key findings are:

1. The Cressi Donatello uses the `DC_FAMILY_CRESSI_GOA` family driver (model 4)
2. The main driver is in `src/cressi_goa.c`
3. The Goa driver uses **115200 baud, 8N1** (line 396-397)
4. Protocol constants include ACK=0x06, and various commands

Now I need to read the actual `cressi_goa.c` file to understand the full protocol, especially how it communicates. Let
me read the key files.

Important note: The search found that the Cressi Goa driver uses serial transport (115200, 8N1), NOT IrDA. This is
different from the Uwatec Smart devices which use IrDA sockets. The Cressi Donatello uses a simple serial-over-IR link -
meaning the IR is just a physical layer replacement for a serial cable, not the IrDA protocol.

This is a critical distinction - the Cressi devices use a simple UART-over-IR approach where the IR LED/receiver just
modulates/demodulates a serial signal. The 38kHz modulation is handled by the TSOP receiver chip transparently.

Let me read the cressi_goa.c file to understand the full protocol.

Now let me read the actual Cressi Goa driver to understand the full protocol:

**Tool: read**

**Input:**

```json
{
  "filePath": "/home/vincent/www/libdivecomputer/src/cressi_goa.c"
}
```

**Output:**

```
<path>/home/vincent/www/libdivecomputer/src/cressi_goa.c</path>
<type>file</type>
<content>
1: /*
2:  * libdivecomputer
3:  *
4:  * Copyright (C) 2018 Jef Driesen
5:  *
6:  * This library is free software; you can redistribute it and/or
7:  * modify it under the terms of the GNU Lesser General Public
8:  * License as published by the Free Software Foundation; either
9:  * version 2.1 of the License, or (at your option) any later version.
10:  *
11:  * This library is distributed in the hope that it will be useful,
12:  * but WITHOUT ANY WARRANTY; without even the implied warranty of
13:  * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
14:  * Lesser General Public License for more details.
15:  *
16:  * You should have received a copy of the GNU Lesser General Public
17:  * License along with this library; if not, write to the Free Software
18:  * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
19:  * MA 02110-1301 USA
20:  */
21: 
22: #include <string.h> // memcpy, memcmp
23: #include <stdlib.h> // malloc, free
24: #include <assert.h> // assert
25: 
26: #include <libdivecomputer/ble.h>
27: 
28: #include "cressi_goa.h"
29: #include "context-private.h"
30: #include "device-private.h"
31: #include "checksum.h"
32: #include "array.h"
33: #include "platform.h"
34: 
35: #define ISINSTANCE(device) dc_device_isinstance((device), &cressi_goa_device_vtable)
36: 
37: #define CMD_VERSION        0x00
38: #define CMD_SET_TIME       0x13
39: #define CMD_EXIT_PCLINK    0x1D
40: #define CMD_LOGBOOK        0x21
41: #define CMD_DIVE           0x22
42: #define CMD_LOGBOOK_V4     0x23
43: 
44: #define CMD_LOGBOOK_BLE 0x02
45: #define CMD_DIVE_BLE    0x03
46: 
47: #define HEADER  0xAA
48: #define TRAILER 0x55
49: #define END     0x04
50: #define ACK     0x06
51: 
52: #define SZ_DATA   512
53: #define SZ_PACKET 12
54: 
55: #define FP_SIZE   6
56: 
57: #define NSTEPS    1000
58: #define STEP(i,n) (NSTEPS * (i) / (n))
59: 
60: typedef struct cressi_goa_device_t {
61: 	dc_device_t base;
62: 	dc_iostream_t *iostream;
63: 	unsigned char fingerprint[FP_SIZE];
64: } cressi_goa_device_t;
65: 
66: typedef struct cressi_goa_conf_t {
67: 	unsigned int logbook_cmd;
68: 	unsigned int logbook_len;
69: 	unsigned int logbook_fp_offset;
70: 	unsigned int dive_fp_offset;
71: } cressi_goa_conf_t;
72: 
73: static dc_status_t cressi_goa_device_set_fingerprint (dc_device_t *abstract, const unsigned char data[], unsigned int size);
74: static dc_status_t cressi_goa_device_foreach (dc_device_t *abstract, dc_dive_callback_t callback, void *userdata);
75: static dc_status_t cressi_goa_device_timesync (dc_device_t *abstract, const dc_datetime_t *datetime);
76: static dc_status_t cressi_goa_device_close (dc_device_t *abstract);
77: 
78: static const dc_device_vtable_t cressi_goa_device_vtable = {
79: 	sizeof(cressi_goa_device_t),
80: 	DC_FAMILY_CRESSI_GOA,
81: 	cressi_goa_device_set_fingerprint, /* set_fingerprint */
82: 	NULL, /* read */
83: 	NULL, /* write */
84: 	NULL, /* dump */
85: 	cressi_goa_device_foreach, /* foreach */
86: 	cressi_goa_device_timesync, /* timesync */
87: 	cressi_goa_device_close /* close */
88: };
89: 
90: static const cressi_goa_conf_t version_conf[] = {
91: 	{ CMD_LOGBOOK,    23, 17, 12 },
92: 	{ CMD_LOGBOOK_V4, 15,  3,  4 },
93: };
94: 
95: static dc_status_t
96: cressi_goa_device_send (cressi_goa_device_t *device, unsigned char cmd, const unsigned char data[], unsigned int size)
97: {
98: 	dc_status_t status = DC_STATUS_SUCCESS;
99: 	dc_device_t *abstract = (dc_device_t *) device;
100: 	dc_transport_t transport = dc_iostream_get_transport (device->iostream);
101: 
102: 	if (size > SZ_PACKET) {
103: 		ERROR (abstract->context, "Unexpected payload size (%u).", size);
104: 		return DC_STATUS_INVALIDARGS;
105: 	}
106: 
107: 	// Setup the data packet.
108: 	unsigned short crc = 0;
109: 	unsigned char packet[SZ_PACKET + 8] = {
110: 		HEADER, HEADER, HEADER,
111: 		size,
112: 		cmd
113: 	};
114: 	if (size) {
115: 		memcpy (packet + 5, data, size);
116: 	}
117: 	crc = checksum_crc16_ccitt (packet + 3, size + 2, 0x000, 0x0000);
118: 	packet[5 + size + 0] = (crc     ) & 0xFF; // Low
119: 	packet[5 + size + 1] = (crc >> 8) & 0xFF; // High
120: 	packet[5 + size + 2] = TRAILER;
121: 
122: 	// Wait a small amount of time before sending the command. Without
123: 	// this delay, the transfer will fail most of the time.
124: 	unsigned int delay = transport == DC_TRANSPORT_BLE ? 2000 : 100;
125: 	dc_iostream_sleep (device->iostream, delay);
126: 
127: 	// Send the command to the device.
128: 	if (transport == DC_TRANSPORT_BLE) {
129: 		status = dc_iostream_write (device->iostream, packet + 4, size + 1, NULL);
130: 	} else {
131: 		status = dc_iostream_write (device->iostream, packet, size + 8, NULL);
132: 	}
133: 	if (status != DC_STATUS_SUCCESS) {
134: 		ERROR (abstract->context, "Failed to send the command.");
135: 		return status;
136: 	}
137: 
138: 	return status;
139: }
140: 
141: static dc_status_t
142: cressi_goa_device_receive (cressi_goa_device_t *device, dc_buffer_t *output)
143: {
144: 	dc_status_t status = DC_STATUS_SUCCESS;
145: 	dc_device_t *abstract = (dc_device_t *) device;
146: 	dc_transport_t transport = dc_iostream_get_transport (device->iostream);
147: 
148: 	unsigned char packet[SZ_PACKET + 8];
149: 
150: 	if (transport == DC_TRANSPORT_BLE) {
151: 		if (output) {
152: 			return DC_STATUS_INVALIDARGS;
153: 		} else {
154: 			return DC_STATUS_SUCCESS;
155: 		}
156: 	}
157: 
158: 	// Clear the output buffer.
159: 	dc_buffer_clear (output);
160: 
161: 	// Read the header of the data packet.
162: 	status = dc_iostream_read (device->iostream, packet, 4, NULL);
163: 	if (status != DC_STATUS_SUCCESS) {
164: 		ERROR (abstract->context, "Failed to receive the answer.");
165: 		return status;
166: 	}
167: 
168: 	// Verify the header of the packet.
169: 	if (packet[0] != HEADER || packet[1] != HEADER || packet[2] != HEADER) {
170: 		ERROR (abstract->context, "Unexpected answer header byte.");
171: 		return DC_STATUS_PROTOCOL;
172: 	}
173: 
174: 	// Get the payload length.
175: 	unsigned int length = packet[3];
176: 	if (length > SZ_PACKET) {
177: 		ERROR (abstract->context, "Unexpected payload size (%u).", length);
178: 		return DC_STATUS_PROTOCOL;
179: 	}
180: 
181: 	// Read the remainder of the data packet.
182: 	status = dc_iostream_read (device->iostream, packet + 4, length + 4, NULL);
183: 	if (status != DC_STATUS_SUCCESS) {
184: 		ERROR (abstract->context, "Failed to receive the answer.");
185: 		return status;
186: 	}
187: 
188: 	// Verify the trailer of the packet.
189: 	if (packet[length + 7] != TRAILER) {
190: 		ERROR (abstract->context, "Unexpected answer trailer byte.");
191: 		return DC_STATUS_PROTOCOL;
192: 	}
193: 
194: 	// Verify the checksum of the packet.
195: 	unsigned short crc = array_uint16_le (packet + length + 5);
196: 	unsigned short ccrc = checksum_crc16_ccitt (packet + 3, length + 2, 0x0000, 0x0000);
197: 	if (crc != ccrc) {
198: 		ERROR (abstract->context, "Unexpected answer checksum.");
199: 		return DC_STATUS_PROTOCOL;
200: 	}
201: 
202: 	if (length && output) {
203: 		if (!dc_buffer_append (output, packet + 5, length)) {
204: 			ERROR (abstract->context, "Could not append received data.");
205: 			return DC_STATUS_NOMEMORY;
206: 		}
207: 	}
208: 
209: 	return status;
210: }
211: 
212: static dc_status_t
213: cressi_goa_device_download (cressi_goa_device_t *device, dc_buffer_t *buffer, dc_event_progress_t *progress)
214: {
215: 	dc_status_t status = DC_STATUS_SUCCESS;
216: 	dc_device_t *abstract = (dc_device_t *) device;
217: 	dc_transport_t transport = dc_iostream_get_transport (device->iostream);
218: 
219: 	const unsigned char ack[] = {ACK};
220: 	const unsigned int initial = progress ? progress->current : 0;
221: 
222: 	// Erase the contents of the buffer.
223: 	if (!dc_buffer_clear (buffer)) {
224: 		ERROR (abstract->context, "Insufficient buffer space available.");
225: 		return DC_STATUS_NOMEMORY;
226: 	}
227: 
228: 	unsigned int skip = 2;
229: 	unsigned int size = 2;
230: 	unsigned int nbytes = 0;
231: 	while (nbytes < size) {
232: 		unsigned char packet[3 + SZ_DATA + 2];
233: 
234: 		if (transport == DC_TRANSPORT_BLE) {
235: 			// Read the data packet.
236: 			unsigned int packetsize = 0;
237: 			while (packetsize < SZ_DATA) {
238: 				size_t len = 0;
239: 				status = dc_iostream_read (device->iostream, packet + 3 + packetsize, SZ_DATA - packetsize, &len);
240: 				if (status != DC_STATUS_SUCCESS) {
241: 					ERROR (abstract->context, "Failed to receive the answer.");
242: 					return status;
243: 				}
244: 
245: 				packetsize += len;
246: 			}
247: 		} else {
248: 			// Read the data packet.
249: 			status = dc_iostream_read (device->iostream, packet, sizeof(packet), NULL);
250: 			if (status != DC_STATUS_SUCCESS) {
251: 				ERROR (abstract->context, "Failed to receive the answer.");
252: 				return status;
253: 			}
254: 
255: 			// Verify the checksum of the packet.
256: 			unsigned short crc = array_uint16_le (packet + sizeof(packet) - 2);
257: 			unsigned short ccrc = checksum_crc16_ccitt (packet + 3, sizeof(packet) - 5, 0x0000, 0x0000);
258: 			if (crc != ccrc) {
259: 				ERROR (abstract->context, "Unexpected answer checksum.");
260: 				return DC_STATUS_PROTOCOL;
261: 			}
262: 
263: 			// Send the ack byte to the device.
264: 			status = dc_iostream_write (device->iostream, ack, sizeof(ack), NULL);
265: 			if (status != DC_STATUS_SUCCESS) {
266: 				ERROR (abstract->context, "Failed to send the ack byte.");
267: 				return status;
268: 			}
269: 		}
270: 
271: 		// Get the total size from the first data packet.
272: 		if (nbytes == 0) {
273: 			size += array_uint16_le (packet + 3);
274: 		}
275: 
276: 		// Calculate the payload size of the packet.
277: 		unsigned int length = size - nbytes;
278: 		if (length > SZ_DATA) {
279: 			length = SZ_DATA;
280: 		}
281: 
282: 		// Append the payload to the output buffer.
283: 		if (!dc_buffer_append (buffer, packet + 3 + skip, length - skip)) {
284: 			ERROR (abstract->context, "Insufficient buffer space available.");
285: 			return DC_STATUS_NOMEMORY;
286: 		}
287: 
288: 		nbytes += length;
289: 		skip = 0;
290: 
291: 		// Update and emit a progress event.
292: 		if (progress) {
293: 			progress->current = initial + STEP(nbytes, size);
294: 			device_event_emit (abstract, DC_EVENT_PROGRESS, progress);
295: 		}
296: 	}
297: 
298: 	if (transport == DC_TRANSPORT_BLE) {
299: 		// Read the end bytes.
300: 		unsigned char end[16] = {0};
301: 		size_t len = 0;
302: 		status = dc_iostream_read (device->iostream, end, sizeof(end), &len);
303: 		if (status != DC_STATUS_SUCCESS) {
304: 			ERROR (abstract->context, "Failed to receive the end bytes.");
305: 			return status;
306: 		}
307: 
308: 		// Verify the end bytes ("EOT xmodem").
309: 		const unsigned char validate[16] = {
310: 			0x45, 0x4F, 0x54, 0x20, 0x78, 0x6D, 0x6F, 0x64,
311: 			0x65, 0x6D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
312: 		if (memcmp (end, validate, sizeof(validate)) != 0) {
313: 			ERROR (abstract->context, "Unexpected end bytes.");
314: 			return DC_STATUS_PROTOCOL;
315: 		}
316: 	} else {
317: 		// Read the end byte.
318: 		unsigned char end = 0;
319: 		status = dc_iostream_read (device->iostream, &end, 1, NULL);
320: 		if (status != DC_STATUS_SUCCESS) {
321: 			ERROR (abstract->context, "Failed to receive the end byte.");
322: 			return status;
323: 		}
324: 
325: 		// Verify the end byte.
326: 		if (end != END) {
327: 			ERROR (abstract->context, "Unexpected end byte (%02x).", end);
328: 			return DC_STATUS_PROTOCOL;
329: 		}
330: 
331: 		// Send the ack byte to the device.
332: 		status = dc_iostream_write (device->iostream, ack, sizeof(ack), NULL);
333: 		if (status != DC_STATUS_SUCCESS) {
334: 			ERROR (abstract->context, "Failed to send the ack byte.");
335: 			return status;
336: 		}
337: 	}
338: 
339: 	return status;
340: }
341: 
342: static dc_status_t
343: cressi_goa_device_transfer (cressi_goa_device_t *device,
344:                             unsigned char cmd,
345:                             const unsigned char input[], unsigned int isize,
346:                             dc_buffer_t *output,
347:                             dc_buffer_t *buffer,
348:                             dc_event_progress_t *progress)
349: {
350: 	dc_status_t status = DC_STATUS_SUCCESS;
351: 
352: 	// Send the command to the dive computer.
353: 	status = cressi_goa_device_send (device, cmd, input, isize);
354: 	if (status != DC_STATUS_SUCCESS) {
355: 		return status;
356: 	}
357: 
358: 	// Receive the answer from the dive computer.
359: 	status = cressi_goa_device_receive (device, output);
360: 	if (status != DC_STATUS_SUCCESS) {
361: 		return status;
362: 	}
363: 
364: 	// Download the optional and variable sized payload.
365: 	if (buffer) {
366: 		status = cressi_goa_device_download (device, buffer, progress);
367: 		if (status != DC_STATUS_SUCCESS) {
368: 			return status;
369: 		}
370: 	}
371: 
372: 	return status;
373: }
374: 
375: 
376: dc_status_t
377: cressi_goa_device_open (dc_device_t **out, dc_context_t *context, dc_iostream_t *iostream)
378: {
379: 	dc_status_t status = DC_STATUS_SUCCESS;
380: 	cressi_goa_device_t *device = NULL;
381: 
382: 	if (out == NULL)
383: 		return DC_STATUS_INVALIDARGS;
384: 
385: 	// Allocate memory.
386: 	device = (cressi_goa_device_t *) dc_device_allocate (context, &cressi_goa_device_vtable);
387: 	if (device == NULL) {
388: 		ERROR (context, "Failed to allocate memory.");
389: 		return DC_STATUS_NOMEMORY;
390: 	}
391: 
392: 	// Set the default values.
393: 	device->iostream = iostream;
394: 	memset (device->fingerprint, 0, sizeof (device->fingerprint));
395: 
396: 	// Set the serial communication protocol (115200 8N1).
397: 	status = dc_iostream_configure (device->iostream, 115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE);
398: 	if (status != DC_STATUS_SUCCESS) {
399: 		ERROR (context, "Failed to set the terminal attributes.");
400: 		goto error_free;
401: 	}
402: 
403: 	// Set the timeout for receiving data (3000 - 5000 ms).
404: 	dc_transport_t transport = dc_iostream_get_transport (device->iostream);
405: 	int timeout = transport == DC_TRANSPORT_BLE ? 5000 : 3000;
406: 	status = dc_iostream_set_timeout (device->iostream, timeout);
407: 	if (status != DC_STATUS_SUCCESS) {
408: 		ERROR (context, "Failed to set the timeout.");
409: 		goto error_free;
410: 	}
411: 
412: 	// Clear the RTS line.
413: 	status = dc_iostream_set_rts (device->iostream, 0);
414: 	if (status != DC_STATUS_SUCCESS) {
415: 		ERROR (context, "Failed to clear the RTS line.");
416: 		goto error_free;
417: 	}
418: 
419: 	// Clear the DTR line.
420: 	status = dc_iostream_set_dtr (device->iostream, 0);
421: 	if (status != DC_STATUS_SUCCESS) {
422: 		ERROR (context, "Failed to clear the DTR line.");
423: 		goto error_free;
424: 	}
425: 
426: 	dc_iostream_sleep (device->iostream, 100);
427: 	dc_iostream_purge (device->iostream, DC_DIRECTION_ALL);
428: 
429: 	*out = (dc_device_t *) device;
430: 
431: 	return DC_STATUS_SUCCESS;
432: 
433: error_free:
434: 	dc_device_deallocate ((dc_device_t *) device);
435: 	return status;
436: }
437: 
438: static dc_status_t
439: cressi_goa_device_set_fingerprint (dc_device_t *abstract, const unsigned char data[], unsigned int size)
440: {
441: 	cressi_goa_device_t *device = (cressi_goa_device_t *) abstract;
442: 
443: 	if (size && size != sizeof (device->fingerprint))
444: 		return DC_STATUS_INVALIDARGS;
445: 
446: 	if (size)
447: 		memcpy (device->fingerprint, data, sizeof (device->fingerprint));
448: 	else
449: 		memset (device->fingerprint, 0, sizeof (device->fingerprint));
450: 
451: 	return DC_STATUS_SUCCESS;
452: }
453: 
454: static dc_status_t
455: cressi_goa_device_foreach (dc_device_t *abstract, dc_dive_callback_t callback, void *userdata)
456: {
457: 	dc_status_t status = DC_STATUS_SUCCESS;
458: 	cressi_goa_device_t *device = (cressi_goa_device_t *) abstract;
459: 	dc_transport_t transport = dc_iostream_get_transport (device->iostream);
460: 	dc_buffer_t *logbook = NULL;
461: 	dc_buffer_t *dive = NULL;
462: 
463: 	// Enable progress notifications.
464: 	dc_event_progress_t progress = EVENT_PROGRESS_INITIALIZER;
465: 	device_event_emit (abstract, DC_EVENT_PROGRESS, &progress);
466: 
467: 	dc_buffer_t *id = dc_buffer_new(11);
468: 	if (id == NULL) {
469: 		ERROR (abstract->context, "Failed to allocate memory for the ID.");
470: 		status = DC_STATUS_NOMEMORY;
471: 		goto error_exit;
472: 	}
473: 
474: 	// Read the version information.
475: 	if (transport == DC_TRANSPORT_BLE) {
476: 		/*
477: 		 * With the BLE communication, there is no variant of the CMD_VERSION
478: 		 * command available. The corresponding information must be obtained by
479: 		 * reading some secondary characteristics instead:
480: 		 *     6E400003-B5A3-F393-E0A9-E50E24DC10B8 - 5 bytes
481: 		 *     6E400004-B5A3-F393-E0A9-E50E24DC10B8 - 2 bytes
482: 		 *     6E400005-B5A3-F393-E0A9-E50E24DC10B8 - 2 bytes
483: 		 */
484: 		const dc_ble_uuid_t characteristics[] = {
485: 			{0x6E, 0x40, 0x00, 0x03, 0xB5, 0xA3, 0xF3, 0x93, 0xE0, 0xA9, 0xE5, 0x0E, 0x24, 0xDC, 0x10, 0xB8},
486: 			{0x6E, 0x40, 0x00, 0x04, 0xB5, 0xA3, 0xF3, 0x93, 0xE0, 0xA9, 0xE5, 0x0E, 0x24, 0xDC, 0x10, 0xB8},
487: 			{0x6E, 0x40, 0x00, 0x05, 0xB5, 0xA3, 0xF3, 0x93, 0xE0, 0xA9, 0xE5, 0x0E, 0x24, 0xDC, 0x10, 0xB8},
488: 		};
489: 		const size_t sizes[] = {5, 2, 2};
490: 
491: 		for (size_t i = 0; i < C_ARRAY_SIZE(characteristics); ++i) {
492: 			unsigned char request[sizeof(dc_ble_uuid_t) + 5] = {0};
493: 
494: 			// Setup the request.
495: 			memcpy (request, characteristics[i], sizeof(dc_ble_uuid_t));
496: 			memset (request + sizeof(dc_ble_uuid_t), 0, sizes[i]);
497: 
498: 			// Read the characteristic.
499: 			status = dc_iostream_ioctl (device->iostream, DC_IOCTL_BLE_CHARACTERISTIC_READ, request, sizeof(dc_ble_uuid_t) + sizes[i]);
500: 			if (status != DC_STATUS_SUCCESS) {
501: 				char uuidstr[DC_BLE_UUID_SIZE] = {0};
502: 				ERROR (abstract->context, "Failed to read the characteristic '%s'.",
503: 					dc_ble_uuid2str(characteristics[i], uuidstr, sizeof(uuidstr)));
504: 				goto error_free_id;
505: 			}
506: 
507: 			// Copy the payload data.
508: 			if (!dc_buffer_append(id, request + sizeof(dc_ble_uuid_t), sizes[i])) {
509: 				ERROR (abstract->context, "Insufficient buffer space available.");
510: 				status = DC_STATUS_NOMEMORY;
511: 				goto error_free_id;
512: 			}
513: 		}
514: 	} else {
515: 		status = cressi_goa_device_transfer (device, CMD_VERSION, NULL, 0, id, NULL, NULL);
516: 		if (status != DC_STATUS_SUCCESS) {
517: 			ERROR (abstract->context, "Failed to read the version information.");
518: 			goto error_free_id;
519: 		}
520: 	}
521: 
522: 	const unsigned char *id_data = dc_buffer_get_data(id);
523: 	size_t id_size = dc_buffer_get_size(id);
524: 
525: 	HEXDUMP (abstract->context, DC_LOGLEVEL_DEBUG, "Version", id_data, id_size);
526: 
527: 	if (id_size < 9) {
528: 		ERROR (abstract->context, "Unexpected version length (" DC_PRINTF_SIZE ").", id_size);
529: 		status = DC_STATUS_DATAFORMAT;
530: 		goto error_free_id;
531: 	}
532: 
533: 	// Get the device info.
534: 	unsigned int model = id_data[4];
535: 	unsigned int firmware = array_uint16_le (id_data + 5);
536: 	unsigned int serial = array_uint32_le (id_data + 0);
537: 
538: 	// Get the data format version.
539: 	unsigned int version = 0;
540: 	if (id_size == 11) {
541: 		version = array_uint16_le (id_data + 9);
542: 	} else {
543: 		if (firmware >= 161 && firmware <= 165) {
544: 			version = 0;
545: 		} else if (firmware >= 166 && firmware <= 169) {
546: 			version = 1;
547: 		} else if (firmware >= 170 && firmware <= 179) {
548: 			version = 2;
549: 		} else if (firmware >= 100 && firmware <= 110) {
550: 			version = 3;
551: 		} else if (firmware >= 200 && firmware <= 299) {
552: 			version = 4;
553: 		} else if (firmware >= 300) {
554: 			version = 5;
555: 		} else {
556: 			ERROR (abstract->context, "Unknown firmware version (%u).", firmware);
557: 			status = DC_STATUS_DATAFORMAT;
558: 			goto error_free_id;
559: 		}
560: 	}
561: 
562: 	const cressi_goa_conf_t *conf = &version_conf[version >= 4];
563: 
564: 	// Emit a vendor event.
565: 	dc_event_vendor_t vendor;
566: 	vendor.data = id_data;
567: 	vendor.size = id_size;
568: 	device_event_emit (abstract, DC_EVENT_VENDOR, &vendor);
569: 
570: 	// Emit a device info event.
571: 	dc_event_devinfo_t devinfo;
572: 	devinfo.model = model;
573: 	devinfo.firmware = firmware;
574: 	devinfo.serial = serial;
575: 	device_event_emit (abstract, DC_EVENT_DEVINFO, &devinfo);
576: 
577: 	// Allocate memory for the logbook data.
578: 	logbook = dc_buffer_new(4096);
579: 	if (logbook == NULL) {
580: 		ERROR (abstract->context, "Failed to allocate memory for the logbook.");
581: 		status = DC_STATUS_NOMEMORY;
582: 		goto error_free_id;
583: 	}
584: 
585: 	// Read the logbook data.
586: 	if (transport == DC_TRANSPORT_BLE) {
587: 		unsigned char args[] = {0x00};
588: 		status = cressi_goa_device_transfer (device, CMD_LOGBOOK_BLE, args, sizeof(args), NULL, logbook, &progress);
589: 	} else {
590: 		status = cressi_goa_device_transfer (device, conf->logbook_cmd, NULL, 0, NULL, logbook, &progress);
591: 	}
592: 	if (status != DC_STATUS_SUCCESS) {
593: 		ERROR (abstract->context, "Failed to read the logbook data.");
594: 		goto error_free_logbook;
595: 	}
596: 
597: 	const unsigned char *logbook_data = dc_buffer_get_data(logbook);
598: 	size_t logbook_size = dc_buffer_get_size(logbook);
599: 
600: 	// Count the number of dives.
601: 	unsigned int count = 0;
602: 	unsigned int offset = logbook_size;
603: 	while (offset >= conf->logbook_len) {
604: 		// Move to the start of the logbook entry.
605: 		offset -= conf->logbook_len;
606: 
607: 		// Get the dive number.
608: 		unsigned int number= array_uint16_le (logbook_data + offset);
609: 		if (number == 0)
610: 			break;
611: 
612: 		// Compare the fingerprint to identify previously downloaded entries.
613: 		if (memcmp (logbook_data + offset + conf->logbook_fp_offset, device->fingerprint, sizeof(device->fingerprint)) == 0) {
614: 			break;
615: 		}
616: 
617: 		count++;
618: 	}
619: 
620: 	// Update and emit a progress event.
621: 	progress.maximum = (count + 1) * NSTEPS;
622: 	device_event_emit (abstract, DC_EVENT_PROGRESS, &progress);
623: 
624: 	// Allocate memory for the dive data.
625: 	dive = dc_buffer_new(4096);
626: 	if (dive == NULL) {
627: 		ERROR (abstract->context, "Failed to allocate memory for the dive.");
628: 		status = DC_STATUS_NOMEMORY;
629: 		goto error_free_logbook;
630: 	}
631: 
632: 	// Download the dives.
633: 	offset = logbook_size;
634: 	for (unsigned int i = 0; i < count; ++i) {
635: 		// Move to the start of the logbook entry.
636: 		offset -= conf->logbook_len;
637: 
638: 		// Read the dive data.
639: 		if (transport == DC_TRANSPORT_BLE) {
640: 			unsigned char number[2] = {
641: 				logbook_data[offset + 1],
642: 				logbook_data[offset + 0]};
643: 			status = cressi_goa_device_transfer (device, CMD_DIVE_BLE, number, 2, NULL, dive, &progress);
644: 		} else {
645: 			status = cressi_goa_device_transfer (device, CMD_DIVE, logbook_data + offset, 2, NULL, dive, &progress);
646: 		}
647: 		if (status != DC_STATUS_SUCCESS) {
648: 			ERROR (abstract->context, "Failed to read the dive data.");
649: 			goto error_free_dive;
650: 		}
651: 
652: 		const unsigned char *dive_data = dc_buffer_get_data (dive);
653: 		size_t dive_size = dc_buffer_get_size (dive);
654: 
655: 		// Verify the dive number and the fingerprint in the logbook and dive
656: 		// data are identical.
657: 		if (dive_size < conf->dive_fp_offset + FP_SIZE ||
658: 			memcmp (dive_data, logbook_data + offset, 2) != 0 ||
659: 			memcmp (dive_data + conf->dive_fp_offset, logbook_data + offset + conf->logbook_fp_offset, FP_SIZE) != 0) {
660: 			ERROR (abstract->context, "Unexpected dive header.");
661: 			status = DC_STATUS_DATAFORMAT;
662: 			goto error_free_dive;
663: 		}
664: 
665: 		// The dive computer id data and the logbook entry contain important
666: 		// information that is required for parsing the dive data, but is
667: 		// not present in the dive data itself. Therefore, both pieces of data
668: 		// are prepended to the dive data, along with a small header containing
669: 		// their size.
670: 		const unsigned char header[] = {
671: 			id_size,
672: 			conf->logbook_len,
673: 		};
674: 		unsigned int headersize = sizeof(header) + id_size + conf->logbook_len;
675: 		if (!dc_buffer_prepend(dive, logbook_data + offset, conf->logbook_len) ||
676: 			!dc_buffer_prepend(dive, id_data, id_size) ||
677: 			!dc_buffer_prepend(dive, header, sizeof(header))) {
678: 			ERROR (abstract->context, "Out of memory.");
679: 			status = DC_STATUS_NOMEMORY;
680: 			goto error_free_dive;
681: 		}
682: 
683: 		dive_data = dc_buffer_get_data (dive);
684: 		dive_size = dc_buffer_get_size (dive);
685: 
686: 		if (callback && !callback(dive_data, dive_size, dive_data + headersize + conf->dive_fp_offset, sizeof(device->fingerprint), userdata))
687: 			break;
688: 	}
689: 
690: error_free_dive:
691: 	dc_buffer_free(dive);
692: error_free_logbook:
693: 	dc_buffer_free(logbook);
694: error_free_id:
695: 	dc_buffer_free(id);
696: error_exit:
697: 	return status;
698: }
699: 
700: static dc_status_t
701: cressi_goa_device_timesync (dc_device_t *abstract, const dc_datetime_t *datetime)
702: {
703: 	dc_status_t status = DC_STATUS_SUCCESS;
704: 	cressi_goa_device_t *device = (cressi_goa_device_t *) abstract;
705: 	dc_transport_t transport = dc_iostream_get_transport (device->iostream);
706: 
707: 	if (transport == DC_TRANSPORT_BLE) {
708: 		return DC_STATUS_UNSUPPORTED;
709: 	}
710: 
711: 	unsigned char new_time[7];
712: 	array_uint16_le_set(new_time, datetime->year);
713: 	new_time[2] = datetime->month;
714: 	new_time[3] = datetime->day;
715: 	new_time[4] = datetime->hour;
716: 	new_time[5] = datetime->minute;
717: 	new_time[6] = datetime->second;
718: 	status = cressi_goa_device_transfer (device, CMD_SET_TIME, new_time, sizeof(new_time), NULL, NULL, NULL);
719: 	if (status != DC_STATUS_SUCCESS) {
720: 		ERROR (abstract->context, "Failed to set the new time.");
721: 		return status;
722: 	}
723: 
724: 	return status;
725: }
726: 
727: static dc_status_t
728: cressi_goa_device_close (dc_device_t *abstract)
729: {
730: 	dc_status_t status = DC_STATUS_SUCCESS;
731: 	cressi_goa_device_t *device = (cressi_goa_device_t *) abstract;
732: 	dc_transport_t transport = dc_iostream_get_transport (device->iostream);
733: 
734: 	if (transport == DC_TRANSPORT_BLE) {
735: 		return DC_STATUS_SUCCESS;
736: 	}
737: 
738: 	status = cressi_goa_device_transfer (device, CMD_EXIT_PCLINK, NULL, 0, NULL, NULL, NULL);
739: 	if (status != DC_STATUS_SUCCESS) {
740: 		ERROR (abstract->context, "Failed to exit PC Link.");
741: 		return status;
742: 	}
743: 
744: 	return status;
745: }

(End of file - total 745 lines)
</content>
```

**Tool: read**

**Input:**

```json
{
  "filePath": "/home/vincent/www/libdivecomputer/src/cressi_goa.h"
}
```

**Output:**

```
<path>/home/vincent/www/libdivecomputer/src/cressi_goa.h</path>
<type>file</type>
<content>
1: /*
2:  * libdivecomputer
3:  *
4:  * Copyright (C) 2018 Jef Driesen
5:  *
6:  * This library is free software; you can redistribute it and/or
7:  * modify it under the terms of the GNU Lesser General Public
8:  * License as published by the Free Software Foundation; either
9:  * version 2.1 of the License, or (at your option) any later version.
10:  *
11:  * This library is distributed in the hope that it will be useful,
12:  * but WITHOUT ANY WARRANTY; without even the implied warranty of
13:  * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
14:  * Lesser General Public License for more details.
15:  *
16:  * You should have received a copy of the GNU Lesser General Public
17:  * License along with this library; if not, write to the Free Software
18:  * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
19:  * MA 02110-1301 USA
20:  */
21: 
22: #ifndef CRESSI_GOA_H
23: #define CRESSI_GOA_H
24: 
25: #include <libdivecomputer/context.h>
26: #include <libdivecomputer/iostream.h>
27: #include <libdivecomputer/device.h>
28: #include <libdivecomputer/parser.h>
29: 
30: #ifdef __cplusplus
31: extern "C" {
32: #endif /* __cplusplus */
33: 
34: dc_status_t
35: cressi_goa_device_open (dc_device_t **device, dc_context_t *context, dc_iostream_t *iostream);
36: 
37: dc_status_t
38: cressi_goa_parser_create (dc_parser_t **parser, dc_context_t *context, const unsigned char data[], size_t size);
39: 
40: #ifdef __cplusplus
41: }
42: #endif /* __cplusplus */
43: #endif /* CRESSI_GOA_H */

(End of file - total 43 lines)
</content>
```

---

## Assistant (Plan · MiMo-V2.5-Pro · 4.7s)

_Thinking:_

Now let me also check the checksum function and look at how the Leonardo driver works for comparison, since the Leonardo
is an older Cressi model that also uses IR serial.

Let me also check the Leonardo driver (older Cressi IR model) and the checksum implementation:

**Tool: read**

**Input:**

```json
{
  "filePath": "/home/vincent/www/libdivecomputer/src/cressi_leonardo.c",
  "limit": 200
}
```

**Output:**

```
<path>/home/vincent/www/libdivecomputer/src/cressi_leonardo.c</path>
<type>file</type>
<content>
1: /*
2:  * libdivecomputer
3:  *
4:  * Copyright (C) 2013 Jef Driesen
5:  *
6:  * This library is free software; you can redistribute it and/or
7:  * modify it under the terms of the GNU Lesser General Public
8:  * License as published by the Free Software Foundation; either
9:  * version 2.1 of the License, or (at your option) any later version.
10:  *
11:  * This library is distributed in the hope that it will be useful,
12:  * but WITHOUT ANY WARRANTY; without even the implied warranty of
13:  * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
14:  * Lesser General Public License for more details.
15:  *
16:  * You should have received a copy of the GNU Lesser General Public
17:  * License along with this library; if not, write to the Free Software
18:  * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
19:  * MA 02110-1301 USA
20:  */
21: 
22: #include <string.h> // memcpy, memcmp
23: #include <stdlib.h> // malloc, free
24: #include <assert.h> // assert
25: 
26: #include "cressi_leonardo.h"
27: #include "context-private.h"
28: #include "device-private.h"
29: #include "checksum.h"
30: #include "array.h"
31: #include "ringbuffer.h"
32: 
33: #define ISINSTANCE(device) dc_device_isinstance((device), &cressi_leonardo_device_vtable)
34: 
35: #define SZ_MEMORY 32000
36: 
37: #define RB_LOGBOOK_BEGIN 0x0100
38: #define RB_LOGBOOK_END   0x1438
39: #define RB_LOGBOOK_SIZE  0x52
40: #define RB_LOGBOOK_COUNT ((RB_LOGBOOK_END - RB_LOGBOOK_BEGIN) / RB_LOGBOOK_SIZE)
41: 
42: #define RB_PROFILE_BEGIN 0x1438
43: #define RB_PROFILE_END   SZ_MEMORY
44: #define RB_PROFILE_DISTANCE(a,b) ringbuffer_distance (a, b, DC_RINGBUFFER_EMPTY, RB_PROFILE_BEGIN, RB_PROFILE_END)
45: 
46: #define MAXRETRIES 4
47: #define PACKETSIZE 32
48: 
49: typedef struct cressi_leonardo_device_t {
50: 	dc_device_t base;
51: 	dc_iostream_t *iostream;
52: 	unsigned char fingerprint[5];
53: } cressi_leonardo_device_t;
54: 
55: static dc_status_t cressi_leonardo_device_set_fingerprint (dc_device_t *abstract, const unsigned char data[], unsigned int size);
56: static dc_status_t cressi_leonardo_device_read (dc_device_t *abstract, unsigned int address, unsigned char data[], unsigned int size);
57: static dc_status_t cressi_leonardo_device_dump (dc_device_t *abstract, dc_buffer_t *buffer);
58: static dc_status_t cressi_leonardo_device_foreach (dc_device_t *abstract, dc_dive_callback_t callback, void *userdata);
59: 
60: static const dc_device_vtable_t cressi_leonardo_device_vtable = {
61: 	sizeof(cressi_leonardo_device_t),
62: 	DC_FAMILY_CRESSI_LEONARDO,
63: 	cressi_leonardo_device_set_fingerprint, /* set_fingerprint */
64: 	cressi_leonardo_device_read, /* read */
65: 	NULL, /* write */
66: 	cressi_leonardo_device_dump, /* dump */
67: 	cressi_leonardo_device_foreach, /* foreach */
68: 	NULL, /* timesync */
69: 	NULL /* close */
70: };
71: 
72: static dc_status_t
73: cressi_leonardo_extract_dives (dc_device_t *abstract, const unsigned char data[], unsigned int size, dc_dive_callback_t callback, void *userdata);
74: 
75: static void
76: cressi_leonardo_make_ascii (const unsigned char raw[], unsigned int rsize, unsigned char ascii[], unsigned int asize)
77: {
78: 	assert (asize == 2 * (rsize + 3));
79: 
80: 	// Header
81: 	ascii[0] = '{';
82: 
83: 	// Data
84: 	array_convert_bin2hex (raw, rsize, ascii + 1, 2 * rsize);
85: 
86: 	// Checksum
87: 	unsigned short crc = checksum_crc16_ccitt (ascii + 1, 2 * rsize, 0xffff, 0x0000);
88: 	unsigned char checksum[] = {
89: 			(crc >> 8) & 0xFF,  // High
90: 			(crc     ) & 0xFF}; // Low
91: 	array_convert_bin2hex (checksum, sizeof(checksum), ascii + 1 + 2 * rsize, 4);
92: 
93: 	// Trailer
94: 	ascii[asize - 1] = '}';
95: }
96: 
97: static dc_status_t
98: cressi_leonardo_packet (cressi_leonardo_device_t *device, const unsigned char command[], unsigned int csize, unsigned char answer[], unsigned int asize)
99: {
100: 	dc_status_t status = DC_STATUS_SUCCESS;
101: 	dc_device_t *abstract = (dc_device_t *) device;
102: 
103: 	if (device_is_cancelled (abstract))
104: 		return DC_STATUS_CANCELLED;
105: 
106: 	// Send the command to the device.
107: 	status = dc_iostream_write (device->iostream, command, csize, NULL);
108: 	if (status != DC_STATUS_SUCCESS) {
109: 		ERROR (abstract->context, "Failed to send the command.");
110: 		return status;
111: 	}
112: 
113: 	// Receive the answer of the device.
114: 	status = dc_iostream_read (device->iostream, answer, asize, NULL);
115: 	if (status != DC_STATUS_SUCCESS) {
116: 		ERROR (abstract->context, "Failed to receive the answer.");
117: 		return status;
118: 	}
119: 
120: 	// Verify the header and trailer of the packet.
121: 	if (answer[0] != '{' || answer[asize - 1] != '}') {
122: 		ERROR (abstract->context, "Unexpected answer header/trailer byte.");
123: 		return DC_STATUS_PROTOCOL;
124: 	}
125: 
126: 	// Convert the checksum of the packet.
127: 	unsigned char checksum[2] = {0};
128: 	array_convert_hex2bin (answer + asize - 5, 4, checksum, sizeof(checksum));
129: 
130: 	// Verify the checksum of the packet.
131: 	unsigned short crc = array_uint16_be (checksum);
132: 	unsigned short ccrc = checksum_crc16_ccitt (answer + 1, asize - 6, 0xffff, 0x0000);
133: 	if (crc != ccrc) {
134: 		ERROR (abstract->context, "Unexpected answer checksum.");
135: 		return DC_STATUS_PROTOCOL;
136: 	}
137: 
138: 	return DC_STATUS_SUCCESS;
139: }
140: 
141: static dc_status_t
142: cressi_leonardo_transfer (cressi_leonardo_device_t *device, const unsigned char command[], unsigned int csize, unsigned char answer[], unsigned int asize)
143: {
144: 	unsigned int nretries = 0;
145: 	dc_status_t rc = DC_STATUS_SUCCESS;
146: 	while ((rc = cressi_leonardo_packet (device, command, csize, answer, asize)) != DC_STATUS_SUCCESS) {
147: 		// Automatically discard a corrupted packet,
148: 		// and request a new one.
149: 		if (rc != DC_STATUS_PROTOCOL && rc != DC_STATUS_TIMEOUT)
150: 			return rc;
151: 
152: 		// Abort if the maximum number of retries is reached.
153: 		if (nretries++ >= MAXRETRIES)
154: 			return rc;
155: 
156: 		// Discard any garbage bytes.
157: 		dc_iostream_sleep (device->iostream, 100);
158: 		dc_iostream_purge (device->iostream, DC_DIRECTION_INPUT);
159: 	}
160: 
161: 	return rc;
162: }
163: 
164: dc_status_t
165: cressi_leonardo_device_open (dc_device_t **out, dc_context_t *context, dc_iostream_t *iostream)
166: {
167: 	dc_status_t status = DC_STATUS_SUCCESS;
168: 	cressi_leonardo_device_t *device = NULL;
169: 
170: 	if (out == NULL)
171: 		return DC_STATUS_INVALIDARGS;
172: 
173: 	// Allocate memory.
174: 	device = (cressi_leonardo_device_t *) dc_device_allocate (context, &cressi_leonardo_device_vtable);
175: 	if (device == NULL) {
176: 		ERROR (context, "Failed to allocate memory.");
177: 		return DC_STATUS_NOMEMORY;
178: 	}
179: 
180: 	// Set the default values.
181: 	device->iostream = iostream;
182: 	memset (device->fingerprint, 0, sizeof (device->fingerprint));
183: 
184: 	// Set the serial communication protocol (115200 8N1).
185: 	status = dc_iostream_configure (device->iostream, 115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE);
186: 	if (status != DC_STATUS_SUCCESS) {
187: 		ERROR (context, "Failed to set the terminal attributes.");
188: 		goto error_free;
189: 	}
190: 
191: 	// Set the timeout for receiving data (1000 ms).
192: 	status = dc_iostream_set_timeout (device->iostream, 1000);
193: 	if (status != DC_STATUS_SUCCESS) {
194: 		ERROR (context, "Failed to set the timeout.");
195: 		goto error_free;
196: 	}
197: 
198: 	// Set the RTS line.
199: 	status = dc_iostream_set_rts (device->iostream, 1);
200: 	if (status != DC_STATUS_SUCCESS) {

(Showing lines 1-200 of 526. Use offset=201 to continue.)
</content>
```

**Tool: read**

**Input:**

```json
{
  "filePath": "/home/vincent/www/libdivecomputer/src/cressi_edy.c",
  "limit": 50
}
```

**Output:**

```
<path>/home/vincent/www/libdivecomputer/src/cressi_edy.c</path>
<type>file</type>
<content>
1: /*
2:  * libdivecomputer
3:  *
4:  * Copyright (C) 2009 Jef Driesen
5:  *
6:  * This library is free software; you can redistribute it and/or
7:  * modify it under the terms of the GNU Lesser General Public
8:  * License as published by the Free Software Foundation; either
9:  * version 2.1 of the License, or (at your option) any later version.
10:  *
11:  * This library is distributed in the hope that it will be useful,
12:  * but WITHOUT ANY WARRANTY; without even the implied warranty of
13:  * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
14:  * Lesser General Public License for more details.
15:  *
16:  * You should have received a copy of the GNU Lesser General Public
17:  * License along with this library; if not, write to the Free Software
18:  * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
19:  * MA 02110-1301 USA
20:  */
21: 
22: #include <string.h> // memcpy, memcmp
23: #include <stdlib.h> // malloc, free
24: #include <assert.h> // assert
25: 
26: #include "cressi_edy.h"
27: #include "context-private.h"
28: #include "device-private.h"
29: #include "checksum.h"
30: #include "array.h"
31: #include "ringbuffer.h"
32: #include "rbstream.h"
33: 
34: #define ISINSTANCE(device) dc_device_isinstance((device), &cressi_edy_device_vtable)
35: 
36: #define MAXRETRIES        4
37: 
38: #define SZ_PAGE           32
39: 
40: #define SZ_HEADER 32
41: 
42: #define ARCHIMEDE 0x01
43: #define IQ700 0x05
44: #define EDY   0x08
45: 
46: typedef struct cressi_edy_layout_t {
47: 	unsigned int memsize;
48: 	unsigned int rb_profile_begin;
49: 	unsigned int rb_profile_end;
50: 	unsigned int rb_logbook_offset;

(Showing lines 1-50 of 581. Use offset=51 to continue.)
</content>
```

---

## Assistant (Plan · MiMo-V2.5-Pro · 4.4s)

_Thinking:_

Now I have a very clear picture. Let me also check the checksum function to understand the CRC16 CCITT implementation
used:

Now let me check the checksum and descriptor details:

**Tool: read**

**Input:**

```json
{
  "filePath": "/home/vincent/www/libdivecomputer/src/checksum.h"
}
```

**Output:**

```
<path>/home/vincent/www/libdivecomputer/src/checksum.h</path>
<type>file</type>
<content>
1: /*
2:  * libdivecomputer
3:  *
4:  * Copyright (C) 2008 Jef Driesen
5:  *
6:  * This library is free software; you can redistribute it and/or
7:  * modify it under the terms of the GNU Lesser General Public
8:  * License as published by the Free Software Foundation; either
9:  * version 2.1 of the License, or (at your option) any later version.
10:  *
11:  * This library is distributed in the hope that it will be useful,
12:  * but WITHOUT ANY WARRANTY; without even the implied warranty of
13:  * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
14:  * Lesser General Public License for more details.
15:  *
16:  * You should have received a copy of the GNU Lesser General Public
17:  * License along with this library; if not, write to the Free Software
18:  * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
19:  * MA 02110-1301 USA
20:  */
21: 
22: #ifndef CHECKSUM_H
23: #define CHECKSUM_H
24: 
25: #ifdef __cplusplus
26: extern "C" {
27: #endif /* __cplusplus */
28: 
29: unsigned char
30: checksum_add_uint4 (const unsigned char data[], unsigned int size, unsigned char init);
31: 
32: unsigned char
33: checksum_add_uint8 (const unsigned char data[], unsigned int size, unsigned char init);
34: 
35: unsigned short
36: checksum_add_uint16 (const unsigned char data[], unsigned int size, unsigned short init);
37: 
38: unsigned char
39: checksum_xor_uint8 (const unsigned char data[], unsigned int size, unsigned char init);
40: 
41: unsigned char
42: checksum_crc8 (const unsigned char data[], unsigned int size, unsigned char init, unsigned char xorout);
43: 
44: unsigned short
45: checksum_crc16_ccitt (const unsigned char data[], unsigned int size, unsigned short init, unsigned short xorout);
46: 
47: unsigned short
48: checksum_crc16r_ccitt (const unsigned char data[], unsigned int size, unsigned short init, unsigned short xorout);
49: 
50: unsigned short
51: checksum_crc16_ansi (const unsigned char data[], unsigned int size, unsigned short init, unsigned short xorout);
52: 
53: unsigned short
54: checksum_crc16r_ansi (const unsigned char data[], unsigned int size, unsigned short init, unsigned short xorout);
55: 
56: unsigned int
57: checksum_crc32r (const unsigned char data[], unsigned int size);
58: 
59: unsigned int
60: checksum_crc32 (const unsigned char data[], unsigned int size);
61: 
62: #ifdef __cplusplus
63: }
64: #endif /* __cplusplus */
65: 
66: #endif /* CHECKSUM_H */

(End of file - total 66 lines)
</content>
```

**Tool: grep**

**Input:**

```json
{
  "pattern": "Donatello",
  "path": "/home/vincent/www/libdivecomputer/src/descriptor.c"
}
```

**Output:**

```
Found 2 matches
/home/vincent/www/libdivecomputer/src/descriptor.c:
  Line 355: 	{"Cressi", "Donatello",    DC_FAMILY_CRESSI_GOA, 4, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},

  Line 938: 		4,  // Donatello

```

---

## Assistant (Plan · MiMo-V2.5-Pro · 4.2s)

_Thinking:_

Now let me also look at how the Leonardo driver continues (the RTS/DTR handling) since it shows how the IR link is
activated:

**Tool: read**

**Input:**

```json
{
  "filePath": "/home/vincent/www/libdivecomputer/src/cressi_leonardo.c",
  "offset": 200,
  "limit": 100
}
```

**Output:**

```
<path>/home/vincent/www/libdivecomputer/src/cressi_leonardo.c</path>
<type>file</type>
<content>
200: 	if (status != DC_STATUS_SUCCESS) {
201: 		ERROR (context, "Failed to set the RTS line.");
202: 		goto error_free;
203: 	}
204: 
205: 	// Set the DTR line.
206: 	status = dc_iostream_set_dtr (device->iostream, 1);
207: 	if (status != DC_STATUS_SUCCESS) {
208: 		ERROR (context, "Failed to set the DTR line.");
209: 		goto error_free;
210: 	}
211: 
212: 	dc_iostream_sleep (device->iostream, 200);
213: 
214: 	// Clear the DTR line.
215: 	status = dc_iostream_set_dtr (device->iostream, 0);
216: 	if (status != DC_STATUS_SUCCESS) {
217: 		ERROR (context, "Failed to clear the DTR line.");
218: 		goto error_free;
219: 	}
220: 
221: 	dc_iostream_sleep (device->iostream, 100);
222: 	dc_iostream_purge (device->iostream, DC_DIRECTION_ALL);
223: 
224: 	*out = (dc_device_t *) device;
225: 
226: 	return DC_STATUS_SUCCESS;
227: 
228: error_free:
229: 	dc_device_deallocate ((dc_device_t *) device);
230: 	return status;
231: }
232: 
233: static dc_status_t
234: cressi_leonardo_device_set_fingerprint (dc_device_t *abstract, const unsigned char data[], unsigned int size)
235: {
236: 	cressi_leonardo_device_t *device = (cressi_leonardo_device_t *) abstract;
237: 
238: 	if (size && size != sizeof (device->fingerprint))
239: 		return DC_STATUS_INVALIDARGS;
240: 
241: 	if (size)
242: 		memcpy (device->fingerprint, data, sizeof (device->fingerprint));
243: 	else
244: 		memset (device->fingerprint, 0, sizeof (device->fingerprint));
245: 
246: 	return DC_STATUS_SUCCESS;
247: }
248: 
249: static dc_status_t
250: cressi_leonardo_device_read (dc_device_t *abstract, unsigned int address, unsigned char data[], unsigned int size)
251: {
252: 	dc_status_t rc = DC_STATUS_SUCCESS;
253: 	cressi_leonardo_device_t *device = (cressi_leonardo_device_t *) abstract;
254: 
255: 	unsigned int nbytes = 0;
256: 	while (nbytes < size) {
257: 		// Calculate the packet size.
258: 		unsigned int len = size - nbytes;
259: 		if (len > PACKETSIZE)
260: 			len = PACKETSIZE;
261: 
262: 		// Build the raw command.
263: 		unsigned char raw[] = {
264: 			(address >> 8) & 0xFF, // High
265: 			(address     ) & 0xFF, // Low
266: 			(len >> 8) & 0xFF, // High
267: 			(len     ) & 0xFF}; // Low
268: 
269: 		// Build the ascii command.
270: 		unsigned char command[2 * (sizeof (raw) + 3)] = {0};
271: 		cressi_leonardo_make_ascii (raw, sizeof (raw), command, sizeof (command));
272: 
273: 		// Send the command and receive the answer.
274: 		unsigned char answer[2 * (PACKETSIZE + 3)] = {0};
275: 		rc = cressi_leonardo_transfer (device, command, sizeof (command), answer, 2 * (len + 3));
276: 		if (rc != DC_STATUS_SUCCESS)
277: 			return rc;
278: 
279: 		// Extract the raw data from the packet.
280: 		array_convert_hex2bin (answer + 1, 2 * len, data, len);
281: 
282: 		nbytes += len;
283: 		address += len;
284: 		data += len;
285: 	}
286: 
287: 	return rc;
288: }
289: 
290: static dc_status_t
291: cressi_leonardo_device_dump (dc_device_t *abstract, dc_buffer_t *buffer)
292: {
293: 	dc_status_t status = DC_STATUS_SUCCESS;
294: 	cressi_leonardo_device_t *device = (cressi_leonardo_device_t *) abstract;
295: 
296: 	// Allocate the required amount of memory.
297: 	if (!dc_buffer_resize (buffer, SZ_MEMORY)) {
298: 		ERROR (abstract->context, "Insufficient buffer space available.");
299: 		return DC_STATUS_NOMEMORY;

(Showing lines 200-299 of 526. Use offset=300 to continue.)
</content>
```

---

## Assistant (Plan · MiMo-V2.5-Pro · 241.5s)

_Thinking:_

Now I have a comprehensive understanding of the protocol. Let me also check the descriptor.c line 355 more carefully to
confirm the transport type, and then I'll write up the full plan.

Key findings:

1. The Cressi Donatello uses `DC_FAMILY_CRESSI_GOA` with model 4
2. Transport is `DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE` - so it's standard serial, not IrDA
3. Serial config: **115200 baud, 8N1** (8 data bits, no parity, 1 stop bit, no flow control)
4. The protocol uses a simple packet structure with:
    - Header: `0xAA 0xAA 0xAA`
    - Length byte (max 12 bytes payload)
    - Command byte
    - Data (0-12 bytes)
    - CRC16-CCITT (2 bytes, little-endian)
    - Trailer: `0x55`
5. Commands: VERSION (0x00), SET_TIME (0x13), EXIT_PCLINK (0x1D), LOGBOOK (0x21/0x23), DIVE (0x22)
6. Response: Same packet format
7. ACK byte: 0x06
8. Download uses 514-byte data packets (2 bytes size + 512 bytes data) with CRC and ACK
9. Pre-command delay: 100ms for serial
10. Timeout: 3000ms for serial

For the Leonardo, the setup is:

- RTS=1, DTR=1, sleep 200ms, DTR=0, sleep 100ms

For the Goa (Donatello), the setup is:

- RTS=0, DTR=0, sleep 100ms

The IR link is just a transparent serial-over-IR bridge. The Cressi BT Interface dock has an IR LED and receiver that
simply convert serial data to IR pulses and back. The TSOP1738 handles the 38kHz demodulation.

Now, the critical insight: **This is NOT IrDA protocol.** It's simple UART-over-IR. The IR modulation (likely 38kHz) is
handled by the receiver chip (TSOP1738). The data is just standard serial bytes modulated onto the IR carrier.

For the Arduino design:

1. **TX (Arduino → Donatello)**: We need to modulate the IR LED at ~38kHz when sending a '0' (space) and keep it off
   for '1' (mark). Wait - actually, in standard UART, a '0' bit is the space (low) state and '1' bit is the mark (high)
   state. For IR serial:
    - When the UART line is LOW (start bit, data bit 0), the IR LED should be ON (modulated at 38kHz)
    - When the UART line is HIGH (stop bit, idle), the IR LED should be OFF

Actually, let me reconsider. The TSOP1738 output is:

- When it receives 38kHz IR light → output LOW
- When it doesn't receive IR light → output HIGH

So the IR link inverts the signal. The Cressi dock must account for this inversion.

Actually wait - the Arduino approach is more nuanced. Let me think about this more carefully.

Option A: Use the Arduino's hardware UART (pins 0/1) to directly send/receive serial data, and use a timer to modulate
the IR LED at 38kHz based on the TX pin state. On receive, the TSOP1738 output goes directly to the RX pin (with
possible inversion handled in software or hardware).

Option B: Use the IRremote library to send/receive IR commands. But this is designed for IR remote control protocols,
not transparent serial-over-IR.

Option A is the right approach. The Arduino should act as a transparent serial bridge.

For TX: When the Arduino's TX pin goes LOW (sending a 0 bit), we need to modulate the IR LED at ~38kHz. When TX is
HIGH (idle/1 bit), LED off. We can use Timer2 to generate the 38kHz carrier and gate it with the TX pin state.

For RX: The TSOP1738 outputs LOW when it sees 38kHz IR, and HIGH when it doesn't. This is actually the correct polarity
for a serial RX signal (idle=HIGH, start bit=LOW). So the TSOP1738 output can go directly to the Arduino's RX pin.

Wait, but on an Arduino Uno/Nano, the hardware UART is shared with USB. If we use pins 0/1 for the IR serial link, we
can't also use it for USB serial communication with the host PC. We'd need to use SoftwareSerial for one of the two
links.

Better approach: Use SoftwareSerial for the IR link (using other pins), and the hardware UART (pins 0/1) for USB serial
to the host PC. Or, use an Arduino with native USB (like Leonardo/Pro Micro with ATmega32U4) where the USB serial is
separate from hardware UART.

Actually, the simplest approach for Arduino Uno/Nano:

- Hardware UART (Serial) = USB connection to host PC (via FTDI/CH340)
- SoftwareSerial = IR link to Donatello

But SoftwareSerial at 115200 baud is problematic on AVR - it's at the edge of its capability and unreliable.

Alternative: Use an Arduino Leonardo or Pro Micro (ATmega32U4) which has native USB. Then:

- Native USB Serial = connection to host PC
- Hardware UART (Serial1) = IR link to Donatello at 115200 baud

Actually, re-reading the constraints: "Hardware available: Arduino Uno or Nano". These both use FTDI/CH340 for USB
serial. So we'd have:

- USB Serial (via FTDI/CH340) = host PC connection
- Some other serial = IR link

For 115200 baud, we really need hardware UART. Let me think...

Actually, the cleanest approach for Uno/Nano would be:

1. Use the hardware UART (pins 0/1) for the IR link to the Donatello
2. Use SoftwareSerial or a bit-banged serial for USB communication

But that's backwards - the USB is handled by the FTDI/CH340 chip which connects to pins 0/1. So the hardware UART IS the
USB serial.

Hmm, this is a fundamental constraint. Let me reconsider.

Actually wait - for an Arduino Uno/Nano, the FTDI/CH340 connects to the ATmega328P's hardware UART pins (RX=pin0,
TX=pin1). So Serial is USB. We'd need SoftwareSerial for the IR link.

SoftwareSerial at 115200 on a 16MHz Arduino: The baud rate is at the limit. It can work but may have timing issues. Let
me think about this more carefully.

Actually, there's another approach. Since we're doing IR modulation (not just serial), we can use timer-based
bit-banging for the IR TX, and interrupt-based capture for IR RX. This avoids the SoftwareSerial baud rate limitation.

But this gets complex. Let me think about the simplest viable approach:

**Approach: Hardware UART bridging with timer-based IR modulation**

1. The host PC sends serial data at 115200 baud to the Arduino via USB
2. The Arduino receives this on its hardware UART RX (pin 0)
3. For TX to Donatello: The Arduino uses Timer2 to generate a 38kHz carrier, and gates it based on the serial data bits.
   Essentially, we bit-bang the IR TX using a timer interrupt that runs at 2x the baud rate (to catch each bit
   transition), and modulates at 38kHz during '0' bits.

Actually this is getting very complex. Let me simplify.

**Simplest viable approach: Use SoftwareSerial for IR link**

Despite the 115200 baud being at the edge, many people have reported SoftwareSerial working at 115200 on 16MHz Arduino.
The key is that we're only receiving data (not simultaneously transmitting and receiving on SoftwareSerial), which
helps.

Actually, there's an even better approach. Since the Cressi protocol is command-response (half-duplex), we can use a
single approach:

1. Receive commands from host PC via hardware Serial (USB)
2. For TX to Donatello: Use Timer2 to generate 38kHz PWM on a pin, and gate the LED with a MOSFET/transistor controlled
   by another pin (the serial TX data)
3. For RX from Donatello: TSOP1738 output goes to a pin, and we use interrupt-based serial reading

Actually, let me step back and think about what the Cressi BT Interface dock actually does. It's a USB-to-serial chip (
FTDI/CH340) connected to an IR LED/receiver pair. The IR link is essentially transparent - it just converts the serial
electrical signals to IR light pulses.

So the Arduino needs to:

1. Present as a USB serial device at 115200 8N1
2. Bridge the serial data over IR

The simplest approach is to use the IRremote library's raw send/receive capabilities, but that's designed for protocols,
not transparent serial.

Let me think about this differently. The key insight is:

**The Cressi IR link is NOT IrDA. It's simple OOK (On-Off Keying) at 38kHz carrier frequency.**

The TX side: When sending a '0' bit (UART space), the IR LED is pulsed at 38kHz. When sending a '1' bit (UART mark), the
LED is off.

The RX side: A TSOP1738 demodulates the 38kHz carrier and outputs the demodulated serial signal directly.

So the Arduino approach should be:

1. **RX (Donatello → Arduino → Host)**: TSOP1738 output → Arduino pin (with interrupt). We decode the serial bits in
   software (bit-bang RX) or use a timer capture.
2. **TX (Host → Arduino → Donatello)**: We bit-bang the IR LED, modulating at 38kHz during '0' bits.

Given the complexity, let me design this more carefully.

**Final Design:**

For Arduino Uno/Nano (ATmega328P at 16MHz):

**IR TX (38kHz modulation):**

- Use Timer2 to generate 38kHz on OC2B (pin 3, PD3)
- Gate the IR LED: When we need to send a '0' bit, enable Timer2 output. When sending '1' bit, disable it.
- We can use PWM mode with OCR2A=52 for ~38kHz (16MHz/64/52 ≈ 4.8kHz... no that's not right)

Let me recalculate: For 38kHz with 16MHz clock:

- Prescaler 8: 16MHz/8 = 2MHz. 2MHz/38kHz = 52.6 → OCR2A=52 → 38.4kHz (close enough)
- Or prescaler 1: 16MHz/1 = 16MHz. 16MHz/38kHz = 421 → too large for 8-bit timer

Timer2 is 8-bit. With prescaler 8:

- 2MHz / (52+1) = 37.7kHz - close enough to 38kHz

**IR RX:**

- TSOP1738 output → Arduino pin 2 (INT0) or pin 3 (INT1)
- Use interrupt to detect falling edge (start bit)
- Then sample at 115200 baud intervals (8.68µs) to read the data bits
- Forward received bytes to USB Serial

**USB Serial bridge:**

- Hardware Serial at 115200 baud connects to host via USB (FTDI/CH340)
- Receive bytes from host → modulate on IR LED
- Receive bytes from IR receiver → send to host

The challenge: At 115200 baud, each bit is 8.68µs. We need to:

1. For TX: Send each bit with proper timing while modulating at 38kHz
2. For RX: Sample each bit at the right time

This is tight but doable on a 16MHz AVR (16 cycles per µs, so ~139 cycles per bit).

Actually, let me reconsider the approach. Since we're bridging serial, maybe we should use a different strategy:

**Strategy: Use hardware UART for both directions with creative pin usage**

Wait, the Arduino only has one hardware UART which is used for USB.

**Strategy: Use timer interrupt-based serial for IR link**

We can set up Timer1 (16-bit) to interrupt at 2x the baud rate (230.4kHz, every 4.34µs). In the ISR:

- For TX: Output the current bit to the IR LED (with 38kHz modulation if bit=0)
- For RX: Sample the TSOP1738 output

But a 230kHz interrupt is very aggressive - only 69 cycles between interrupts, which is barely enough for the ISR
overhead.

**Alternative strategy: Use the IRremote library**

The IRremote library (v4+) has:

- IRsend: Can send raw IR signals with 38kHz modulation
- IRrecv: Can receive and decode raw IR signals

But this isn't designed for transparent serial bridging. The library is protocol-oriented.

**Best approach for this project:**

Given the constraints (Arduino Uno/Nano, 115200 baud, 38kHz IR), I think the best approach is:

1. Use the Arduino's hardware UART (Serial) for USB communication with the host PC
2. Use timer-based bit-banging for the IR serial link
3. Buffer data between the two

For TX (host → Donatello):

- Read bytes from Serial (USB)
- For each byte, bit-bang the IR LED: for each bit (start, data, stop), either modulate at 38kHz or keep LED off
- Use Timer2 for 38kHz carrier generation
- Use precise timing (NOP loops or Timer1) for bit timing at 115200 baud

For RX (Donatello → host):

- Use an interrupt on the TSOP1738 pin to detect start bit (falling edge)
- In the ISR, disable the interrupt and start sampling the pin at 115200 baud intervals
- After receiving a full byte (start + 8 data + stop), store it in a buffer
- Main loop forwards buffered bytes to USB Serial

This is doable but the timing is tight at 115200 baud. Let me think about whether we can make it work.

At 115200 baud, each bit period = 1/115200 = 8.68µs = ~139 clock cycles at 16MHz.

For RX: The ISR trigger (INT0 falling edge) has ~4 cycles latency. Then we need to wait 1.5 bit periods to sample the
middle of the start bit (to confirm it's valid), then sample at each bit center. With 139 cycles per bit, we have enough
time for a simple sampling loop.

For TX: We need to toggle the IR LED at 38kHz during '0' bits. A 38kHz cycle is 26.3µs, which is ~3 bit periods. So we'd
have about 420 cycles per 38kHz half-cycle. We can use Timer2 in CTC mode to generate the carrier, and gate it with a
GPIO pin.

Actually, I realize there's a much simpler approach that I'm overcomplicating this.

**Simplified approach:**

The IR LED just needs to be modulated at 38kHz when sending a UART '0' bit. The TSOP1738 handles the demodulation on the
receive side.

For TX:

- Configure Timer2 to generate ~38kHz PWM on a pin
- Connect that PWM pin through an AND gate (or transistor circuit) with a "TX enable" pin
- The "TX enable" pin is driven by software to be LOW during '0' bits and HIGH during '1' bits (inverted because the IR
  LED is active-low conceptually)

Wait, but we don't have an AND gate. We can use a transistor:

- PWM pin → resistor → NPN base
- NPN collector → IR LED → resistor → VCC
- NPN emitter → GND
- "TX enable" pin controls a second transistor in series

Actually, simpler: Just use two transistors in series, or use the PWM to directly drive the IR LED through a transistor,
with the enable controlled by turning the PWM on/off.

**Simplest TX approach:**

- Timer2 generates 38kHz on OC2B (pin 3)
- Pin 3 → resistor → NPN base → IR LED circuit
- To send '0': Enable Timer2 output (TCCR2A |= _BV(COM2B1))
- To send '1': Disable Timer2 output and set pin LOW

This means we bit-bang the serial data manually, toggling the Timer2 output for each bit.

**Simplest RX approach:**

- TSOP1738 output → Arduino pin 2 (INT0)
- Use falling edge interrupt to detect start bit
- In the ISR, delay to center of start bit, then sample 8 data bits at bit-rate intervals
- Store received byte in a circular buffer

Let me write this up as the plan.

Actually, I want to double-check one thing. The Goa protocol uses 115200 baud. But at the physical IR layer, is the
signal inverted? Let me think...

The TSOP1738 output:

- No IR light → HIGH (VCC through pull-up)
- 38kHz IR light → LOW

So the TSOP1738 inverts the signal. If the Cressi dock sends UART data where:

- Idle (HIGH) = no IR → TSOP output HIGH ✓
- Start bit (LOW) = IR on → TSOP output LOW ✓
- Data bit 1 (HIGH) = no IR → TSOP output HIGH ✓
- Data bit 0 (LOW) = IR on → TSOP output LOW ✓

So the TSOP1738 naturally preserves the UART signal polarity! The IR LED is ON during '0' bits and OFF during '1' bits,
which is the standard UART convention. The TSOP1738 sees the 38kHz modulated IR during '0' bits and outputs LOW,
matching the original UART signal.

Now let me think about the actual implementation more carefully.

For the Arduino sketch, I'll design it with:

1. **Timer2 setup**: CTC mode, generating ~38kHz on OC2B (pin 3)
2. **IR TX function**: Bit-bangs a byte over IR by controlling Timer2 output
3. **IR RX via interrupt**: INT0 (pin 2) detects start bit, then samples data bits
4. **Main loop**: Bridges between USB Serial and IR

Here's the key challenge: We can't do TX and RX simultaneously with bit-banging on a single-threaded AVR. But the Cressi
protocol is half-duplex (command-response), so this is fine.

Let me also think about the 100ms pre-command delay mentioned in the Goa driver. The driver waits 100ms before sending
each command. This is handled by libdivecomputer, not by our Arduino bridge.

The Arduino bridge just needs to be a transparent serial-to-IR converter. It doesn't need to understand the protocol at
all.

OK, let me now write the complete plan and Arduino sketch.

One more consideration: The Cressi Goa driver clears RTS and DTR lines:

```c
status = dc_iostream_set_rts (device->iostream, 0);
status = dc_iostream_set_dtr (device->iostream, 0);
```

This is interesting because the Leonardo driver uses DTR toggling to wake up the device. The Goa/Donatello just clears
both. On the Arduino, we might need to handle DTR/RTS signals if they're used to control the IR link somehow. But since
we're presenting as a standard serial port, and the driver just sets them to 0, we can ignore them.

Actually, wait. Looking at the Leonardo driver again:

```c
// Set the RTS line.
status = dc_iostream_set_rts (device->iostream, 1);
// Set the DTR line.
status = dc_iostream_set_dtr (device->iostream, 1);
dc_iostream_sleep (device->iostream, 200);
// Clear the DTR line.
status = dc_iostream_set_dtr (device->iostream, 0);
dc_iostream_sleep (device->iostream, 100);
```

The Leonardo uses RTS=1 and DTR toggling as a wake-up sequence. The Goa (Donatello) just clears both:

```c
status = dc_iostream_set_rts (device->iostream, 0);
status = dc_iostream_set_dtr (device->iostream, 0);
```

So for the Donatello, no special DTR/RTS handling is needed. Good.

Now let me finalize the plan and write the Arduino sketch.

Actually, I realize I should also research whether the TSOP1738 can handle 115200 baud data rates. The TSOP1738 is
designed for IR remote control at much lower data rates (typically up to a few kbps). At 115200 baud, the minimum pulse
width is ~8.68µs. The TSOP1738 has a typical pulse width tolerance of 10-70 carrier cycles at 38kHz. At 38kHz, one
carrier cycle is 26.3µs. So 10 cycles = 263µs, which is way too slow for 115200 baud!

This is a critical issue! The TSOP1738 is designed for IR remote control protocols where the minimum pulse is typically
500µs or longer. At 115200 baud, a single bit is only 8.68µs, and the TSOP1738 won't be able to demodulate such short
pulses.

So either:

1. The Cressi Donatello doesn't use 115200 baud over IR (maybe it uses a lower baud rate for IR and the 115200 is just
   the USB-serial chip's rate)
2. The Cressi dock uses a different type of IR receiver (not TSOP1738 but a raw photodiode/transistor with custom
   demodulation)
3. The TSOP1738 can actually handle it (some newer versions can)

Wait, let me reconsider. The libdivecomputer code configures the serial port at 115200 baud. But the Cressi BT Interface
dock has a USB-to-serial chip (FTDI/CH340) that connects to the host at 115200 baud. The dock itself might have an IR
receiver that's not a TSOP1738 but a raw photodiode with custom circuitry.

However, looking at the Cressi Edy driver, it uses 1200 baud and 4800 baud at different stages. And the Leonardo uses
115200 baud. If the Leonardo's IR link works at 115200 baud with a simple IR LED/receiver, then either:

1. The Cressi dock uses a raw photodiode (not an integrated IR receiver module)
2. The data rate is actually lower and the 115200 is just the host-side serial config

Hmm, but looking at the Cressi BT Interface product description, it's a small dock with a CR2032 battery. It likely has
simple IR components. Let me reconsider...

Actually, I think the answer is simpler than I'm making it. The Cressi dive computer's IR port likely uses a simple IR
LED and photodiode (not a TSOP-type integrated receiver). The modulation at 38kHz is NOT used. Instead, it's just direct
IR light on/off at the serial data rate.

Wait, but then why would the official dock work? It must use some form of IR communication. Let me reconsider the whole
setup.

Looking at this more carefully, I think the Cressi IR port works as follows:

- TX: IR LED modulated at a carrier frequency (likely 38kHz) during '0' bits
- RX: An IR receiver module (like TSOP) demodulates the carrier

But the TSOP1738 has bandwidth limitations. The maximum data rate for TSOP1738 is typically around 2-4 kbps.

Hmm, but the Cressi Leonardo is known to work at 115200 baud. How?

Let me reconsider. Maybe the Cressi dive computer doesn't use 38kHz modulation at all. Maybe it uses baseband IR (direct
on/off without carrier). The IR LED is simply turned on/off at the serial data rate. The receiver would then be a
photodiode or phototransistor, not a TSOP-type module.

If that's the case, then:

- TX: Direct IR LED on/off, no carrier modulation needed
- RX: Photodiode/phototransistor with amplifier circuit

But the user specifically asked about TSOP1738 and 38kHz IR. And the Cressi BT Interface dock likely does use some form
of modulation for noise immunity.

Actually, let me look at this from a different angle. The Cressi BT Interface has a Bluetooth module. Maybe the IR link
operates at a lower baud rate, and the Bluetooth module buffers/converts to the higher rate. But the USB-to-serial
chip (FTDI/CH340) is also at 115200...

OK, I think I need to just go with what the code says. The libdivecomputer code configures 115200 baud. The physical IR
link must support this rate somehow.

Looking at modern IR receiver modules, there are some that support higher data rates:

- TSOP34838: up to 38.4 kbps
- TSSP77038: designed for data communication
- TFMS5360: up to 115.2 kbps

So it's possible the Cressi dock uses a high-speed IR receiver, not a standard TSOP1738.

For our Arduino project, we have two options:

1. Use a high-speed IR receiver (like TFMS5360 or a raw photodiode) that can handle 115200 baud
2. Use a TSOP1738 but at a lower baud rate - but this won't work with libdivecomputer expecting 115200

Actually, wait. I just realized something important. The TSOP1738 CAN handle the data if the modulation is done
correctly. The issue is that the TSOP1738 expects bursts of 38kHz carrier, not continuous carrier. The minimum burst
length is typically 10 carrier cycles (263µs), but the TSOP1738 can detect shorter bursts if they're followed by a
sufficient gap.

But at 115200 baud, a continuous stream of '0' bits would be a continuous 38kHz carrier, which the TSOP1738 might
interpret as noise and suppress.

I think the real answer is that the Cressi dock does NOT use a TSOP-type integrated IR receiver. It likely uses:

- A simple IR photodiode (like BPW41N or similar)
- A transimpedance amplifier
- Possibly a bandpass filter around 38kHz

Or maybe it just uses baseband IR (no carrier) with a photodiode.

For our Arduino project, let me reconsider the approach:

**Option 1: Baseband IR (no carrier)**

- TX: IR LED directly on/off at serial bit rate
- RX: Photodiode with amplifier
- Pro: Simple, supports any baud rate
- Con: Less noise immunity, need amplifier circuit

**Option 2: 38kHz carrier with TSOP1738**

- TX: Modulate IR LED at 38kHz during '0' bits
- RX: TSOP1738 demodulates
- Pro: Good noise immunity, simple receiver
- Con: TSOP1738 may not handle 115200 baud

**Option 3: 38kHz carrier with custom receiver**

- TX: Modulate IR LED at 38kHz during '0' bits
- RX: Custom photodiode + amplifier + demodulator
- Pro: Can handle high baud rates
- Con: Complex circuit

I think the most practical approach for a hobbyist is to try Option 1 first. If the Cressi dive computer's IR port is
just a simple IR LED/photodiode (which is common for dive computer cradles), then baseband IR should work.

Actually, I just thought of something. Many dive computer IR interfaces (like Suunto, Mares, etc.) use IrDA-standard
components. IrDA SIR operates at 9600-115200 baud using 3/16th of a bit period for the optical pulse. The IrDA physical
layer uses:

- TX: 875nm IR LED, pulsed at 3/16 bit period for '0' bits
- RX: Fast photodiode + demodulator

IrDA doesn't use 38kHz carrier. It uses direct pulse modulation.

So the Cressi Donatello likely uses IrDA-compatible or IrDA-like physical layer, not TSOP-style 38kHz modulation.

For our Arduino project, this means:

- TX: Pulse the IR LED for a fraction of each '0' bit period (IrDA uses 3/16, but we can use the full bit period for
  simplicity)
- RX: Use a fast photodiode (like BPW34 or TSOP-style but without carrier demodulation)

Actually, for simplicity and reliability, let me propose this approach:

**Final Design Decision:**

Use a raw IR photodiode (BPW34 or equivalent) for RX instead of TSOP1738. The photodiode can handle any data rate. We'll
amplify its output with a transistor and feed it to the Arduino.

For TX, we use a high-current IR LED (TSAL6400 or equivalent) driven by a transistor, directly modulated by the serial
data (no carrier).

Wait, but the user specifically listed "standard 38kHz IR LED (TSAL6400 or equivalent)" and "TSOP1738 IR receiver" as
available hardware. Let me work with what's available.

Hmm, the TSAL6400 is just an IR LED (940nm, 25° beam angle). It doesn't have a built-in carrier - it's just an LED.
The "38kHz" part would be the modulation we apply to it.

So the user has:

- TSAL6400 IR LED: Just an IR LED, we drive it however we want
- TSOP1738: 38kHz IR receiver module

Given these components, let me reconsider whether the TSOP1738 can work at 115200 baud.

Actually, I just found some relevant information. The TSOP1738 datasheet says:

- Minimum burst length: 10 carrier cycles
- At 38kHz: 10 cycles = 263µs minimum burst
- After a burst, minimum gap: 14 carrier cycles = 368µs

So for 115200 baud (8.68µs per bit), a single '0' bit would be only 8.68µs of IR burst, which is way below the 263µs
minimum. The TSOP1738 would NOT detect this.

However, there's a clever workaround: if we use a lower baud rate for the IR link and buffer/conversion in the Arduino.
But libdivecomputer expects 115200 baud on the serial port...

Actually, wait. Let me re-examine the Cressi protocol more carefully. Looking at the download function:

```c
// Wait a small amount of time before sending the command. Without
// this delay, the transfer will fail most of the time.
unsigned int delay = transport == DC_TRANSPORT_BLE ? 2000 : 100;
dc_iostream_sleep (device->iostream, delay);
```

And the packet structure is command-response with ACK bytes. The actual data transfer uses 514-byte packets with CRC.
This is a fairly structured protocol, not a raw high-speed stream.

What if the Cressi IR link actually operates at a lower baud rate, and the FTDI/CH340 in the dock is just configured at
115200 but the IR link internally uses a different rate? That seems unlikely given that libdivecomputer configures the
serial port at 115200.

Let me just go with the practical approach and present the plan with both options:

1. **Primary approach**: Use baseband IR (direct on/off, no 38kHz carrier) with a photodiode for RX. This is the most
   reliable for 115200 baud.
2. **Alternative**: Use 38kHz carrier with TSOP1738, but note the bandwidth limitation.

Actually, you know what, I think there's a much simpler explanation. Let me re-read the descriptor:

```c
{"Cressi", "Donatello", DC_FAMILY_CRESSI_GOA, 4, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},
```

The transport is SERIAL, not IRDA. The Cressi BT Interface dock presents as a standard USB serial port (via FTDI/CH340).
The IR link inside the dock is transparent to the host. The dock's FTDI/CH340 chip connects to the IR LED/receiver
circuit internally, and the host just sees a serial port.

So the question is: what IR receiver does the Cressi dock use internally?

Without disassembling the dock, we can't know for sure. But given that it operates at 115200 baud, it likely uses
either:

1. A raw photodiode with amplifier circuit
2. A high-speed IR receiver (not TSOP1738)
3. The 38kHz carrier is NOT used, and it's baseband IR

For the Arduino project, I'll recommend:

- **For TX**: Drive the IR LED directly (no carrier modulation) - this is simpler and works at any baud rate
- **For RX**: Try TSOP1738 first, and if it doesn't work at 115200, use a raw photodiode with a simple amplifier circuit

Actually, let me look at this from yet another angle. Some people have successfully used TSOP receivers with serial data
by ensuring that the data has sufficient run lengths. If the data has long runs of '1' bits (idle), the TSOP can resync.
The problem is with isolated '0' bits.

At 115200 baud, a single '0' bit is 8.68µs. The TSOP1738 needs ~263µs minimum. So a single '0' bit won't trigger the
TSOP. But a start bit followed by data bits might create a long enough burst. For example, if the first data byte has
multiple '0' bits in a row, the total burst could be 8.68µs × (1 + number of consecutive 0 bits). For 8 consecutive 0
bits (byte 0x00), that's 9 × 8.68µs = 78µs, still too short.

So TSOP1738 definitely won't work at 115200 baud. We need an alternative for RX.

**Final recommendation for the plan:**

TX: Direct IR LED drive (no 38kHz carrier) - use a transistor to drive the TSAL6400 IR LED
RX: Either:
a. Use a different IR receiver that supports high data rates (e.g., TFMS5360, or a photodiode + amplifier)
b. Use baseband IR with a photodiode circuit

For the Arduino sketch, I'll design it to work with:

1. TX: Timer2 at 38kHz for IR LED modulation (if using carrier) OR direct drive (if baseband)
2. RX: Interrupt-based serial decoding from either TSOP1738 or photodiode

Actually, let me just present the most practical solution. Since the user has a TSOP1738, let me check if there's a way
to make it work.

Hmm, actually I just realized something. Maybe the Cressi dive computer's IR port uses 38kHz carrier modulation, and the
baud rate over the IR link is much lower than 115200. The Cressi BT Interface dock might have a microcontroller that:

1. Receives serial data from the host at 115200 baud via USB
2. Re-transmits it over IR at a lower baud rate with 38kHz carrier
3. Receives IR responses with TSOP1738
4. Forwards them to the host at 115200 baud

This would make sense! The dock acts as a protocol converter, not just a transparent IR link. The 115200 baud is the
host-side rate, not the IR-side rate.

But wait, the libdivecomputer code just opens a serial port and sends/receives data. There's no protocol conversion
happening on the host side. So either:

1. The dock does the conversion transparently
2. The IR link actually runs at 115200 baud

If option 1 is true, then the dock has a microcontroller, and we'd need to replicate its behavior. But we don't know
what baud rate it uses for IR.

If option 2 is true, then we need high-speed IR components.

I think the most likely scenario is that the Cressi IR port uses baseband IR (no carrier) with a simple photodiode. This
is common for dive computer cradles. The "38kHz" assumption from the user may be incorrect.

Let me just present both approaches in my plan and let the user decide. I'll focus on the baseband approach as the
primary one, since it's more likely to work at 115200 baud.

OK wait, let me actually look at the Cressi Edy driver since it also uses IR and has a different baud rate pattern:

The Edy driver uses:

- 1200 baud initially (line 256-257)
- Then 4800 baud later (line 305-306)

1200 baud = 833µs per bit. A TSOP1738 minimum burst is 263µs = ~3.2 bits at 1200 baud. So at 1200 baud, a single '0'
bit (833µs) is long enough for the TSOP1738 to detect! And at 4800 baud (208µs per bit), it's borderline.

So the Cressi Edy (older model) likely uses a TSOP-style 38kHz IR receiver, operating at 1200/4800 baud. The newer Goa
family (including Donatello) uses 115200 baud, which suggests it uses a different (faster) IR receiver.

This confirms that the TSOP1738 won't work for the Donatello at 115200 baud. The user needs either:

1. A high-speed IR receiver
2. A raw photodiode with amplifier
3. Baseband IR (no carrier)

OK, I think I have enough information now. Let me write up the complete plan.

Actually, one more thing. Let me check if there's a way to use the TSOP1738 at a lower baud rate with the Arduino doing
the conversion. The Arduino could:

1. Receive data from host at 115200 baud (via USB serial)
2. Re-transmit it over IR at, say, 9600 baud with 38kHz carrier
3. Receive IR responses at 9600 baud via TSOP1738
4. Forward them to host at 115200 baud

But this won't work because libdivecomputer sends raw bytes at 115200 baud timing, and the Arduino can't change the baud
rate of the protocol (the Donatello expects 115200 baud).

Unless... the Donatello itself can operate at different baud rates? Looking at the code, there's no baud rate
negotiation - it's fixed at 115200. So no.

OK, final plan:

**The Arduino IR bridge for the Cressi Donatello requires baseband IR (no 38kHz carrier) because 115200 baud is too fast
for TSOP-style IR receivers.**

However, there's one more possibility I want to explore. What if we use the TSOP1738 but with a trick? Instead of
sending individual bits as 38kHz bursts, we could:

1. Encode each UART byte as a longer IR burst pattern
2. Use Manchester encoding or similar

But this would require the Donatello to understand the encoding, which it doesn't (it expects standard UART).

OK, I'm confident now. The plan should recommend:

1. **TX**: Direct IR LED drive (baseband, no carrier) - OR - 38kHz carrier if using TSOP for a different project
2. **RX**: Photodiode + amplifier circuit (not TSOP1738)

But wait, the user specifically has TSOP1738. Let me offer both approaches:

- Approach A: Baseband IR with photodiode (works at 115200 baud, needs photodiode + amplifier)
- Approach B: Try TSOP1738 and see if it works (may not work at 115200 baud)
- Approach C: Use a lower baud rate (won't work with libdivecomputer)

I'll recommend Approach A as the primary, with notes about the TSOP1738 limitation.

Hmm, actually, one more thought. There are some newer IR receiver modules that support higher data rates:

- Vishay TFMS5360: supports up to 115.2 kbps
- Vishay TSSP4038: designed for data communication

The user might be able to get one of these. But for the plan, I'll work with what they have.

OK let me now write the comprehensive plan. I'll present:

1. Protocol summary
2. Design approach (baseband IR)
3. Arduino sketch
4. Wiring diagram
5. Caveats

Let me also address the TSOP1738 issue prominently, since the user specifically mentioned it.

Actually, I want to reconsider one more time. Let me look at what baud rate the Cressi IR link actually uses. The 115200
baud is set by libdivecomputer on the serial port. But the Cressi BT Interface dock is a USB-to-serial device with an IR
link. The FTDI/CH340 chip in the dock connects to the host at whatever baud rate the host requests (115200). The IR
LED/receiver is connected to the FTDI/CH340's TX/RX pins through driver circuitry.

So the IR link runs at whatever baud rate the FTDI/CH340 is configured to, which is 115200. The dock doesn't have a
microcontroller doing baud rate conversion (that would add cost and complexity). It's just:

- FTDI/CH340 TX → IR LED driver → IR LED
- IR Receiver → FTDI/CH340 RX

The IR receiver in the dock must be able to handle 115200 baud. It's NOT a TSOP1738. It's likely a raw photodiode with a
transimpedance amplifier.

OK, I'm going to finalize the plan now. The key insight is:

**The Cressi Donatello uses baseband IR (no carrier modulation) at 115200 baud. The TSOP1738 is NOT suitable for this
application. The user needs a photodiode + amplifier for RX.**

For TX, we can drive the IR LED directly (baseband) or with 38kHz modulation. Baseband is simpler and matches what the
dock likely does.

Let me write the plan and Arduino sketch now.

Wait, actually, I want to be really careful here. Let me reconsider whether 38kHz modulation is used.

The Cressi BT Interface dock description says it has a CR2032 battery. This battery powers the IR LED. If the LED is
driven directly (baseband), the current draw would be significant and drain the CR2032 quickly. Using 38kHz modulation (
with a lower duty cycle) would be more power-efficient.

Also, the TSOP1738-style receivers are the standard for IR data communication at low-to-medium speeds. Using baseband IR
would be unusual for a commercial product.

So maybe the dock does use 38kHz modulation, but at a lower baud rate? Let me check if there's any baud rate negotiation
or if the baud rate is configurable.

Looking at the code again:

```c
status = dc_iostream_configure (device->iostream, 115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE);
```

This is the only baud rate configuration. There's no negotiation. So the serial port (and thus the IR link) runs at
115200.

But what if the FTDI/CH340 in the dock is configured to always run at 115200 regardless of what the host requests? No,
that doesn't make sense - the FTDI/CH340 respects the host's baud rate setting.

OK, I think the most likely explanation is that the Cressi dock uses a high-speed IR receiver (not TSOP1738). The user's
assumption that 38kHz/TSOP1738 is correct may be wrong.

For the plan, I'll:

1. Explain the protocol
2. Note the TSOP1738 bandwidth limitation
3. Provide a design that works with both TSOP1738 (if baud rate can be lowered) and photodiode (for 115200 baud)
4. Recommend the photodiode approach for 115200 baud

Actually, let me just go ahead and provide the most practical solution. The user wants to download dives from their
Donatello. Let me design a working solution.

**Approach: Use the Arduino as a USB-to-IR serial bridge with baseband IR**

Components needed (in addition to what the user has):

- IR photodiode (e.g., BPW34, BPW41N, or even a TSOP1738 with the carrier demodulation bypassed)
- NPN transistor (2N2222 or BC547) for IR LED driver
- Resistors

Actually, I just realized that the TSOP1738 can be used in a creative way. If we don't send 38kHz carrier but instead
send raw IR pulses, the TSOP1738's output will follow the IR envelope (after its internal AGC and bandpass filter
settle). But this is unreliable.

Let me just go with the photodiode approach. It's the most reliable for 115200 baud.

OK, final answer. Let me write the comprehensive plan now.

Hmm, actually, one more critical consideration. The user asked me to "Identify the correct IR modulation frequency used
by the Cressi Donatello (likely 38kHz but confirm from source)."

From the source code analysis, there is NO mention of IR modulation frequency. The libdivecomputer code treats the IR
link as a transparent serial connection. The physical layer (IR modulation) is handled by the dock hardware, not by the
software.

So I can't confirm the modulation frequency from the source code. I can only analyze what's likely based on the baud
rate and available components.

OK, let me now write the complete plan. I'll be thorough but practical.

Let me also think about the actual Arduino sketch structure:

```cpp
// Arduino IR Bridge for Cressi Donatello
// Bridges USB Serial (115200 baud) to IR serial (115200 baud)

// IR TX: Pin 9 (OC1A) for PWM carrier generation, or direct drive via pin 7
// IR RX: Pin 2 (INT0) for interrupt-based serial reception

// For baseband IR (no carrier):
// - TX: Direct drive IR LED via transistor, bit-banged serial
// - RX: Photodiode + amplifier → Arduino pin

// For 38kHz carrier IR:
// - TX: Timer-generated 38kHz carrier, gated by serial data
// - RX: TSOP1738 → Arduino pin (limited to ~4kbps)

#define IR_TX_PIN 9      // OC1A for carrier, or any pin for baseband
#define IR_RX_PIN 2      // INT0 for interrupt-based reception

// Serial parameters
#define BAUD_RATE 115200
#define BIT_DELAY (1000000UL / BAUD_RATE)  // 8.68µs per bit

// 38kHz carrier (if used)
#define CARRIER_FREQ 38000
#define CARRIER_PERIOD (F_CPU / (2 * CARRIER_FREQ) - 1)  // For Timer1 CTC

void setup() {
  // USB Serial
  Serial.begin(BAUD_RATE);
  
  // IR TX pin
  pinMode(IR_TX_PIN, OUTPUT);
  digitalWrite(IR_TX_PIN, LOW);
  
  // IR RX pin
  pinMode(IR_RX_PIN, INPUT_PULLUP);
  
  // Setup INT0 for falling edge (start bit detection)
  attachInterrupt(digitalPinToInterrupt(IR_RX_PIN), rx_isr, FALLING);
}

// RX state machine
volatile uint8_t rx_buffer[256];
volatile uint8_t rx_head = 0;
volatile uint8_t rx_tail = 0;
volatile bool rx_active = false;

void rx_isr() {
  if (rx_active) return;  // Already receiving
  
  rx_active = true;
  detachInterrupt(digitalPinToInterrupt(IR_RX_PIN));
  
  // Wait for center of start bit
  delayMicroseconds(BIT_DELAY / 2);
  
  // Verify start bit is still low
  if (digitalRead(IR_RX_PIN) != LOW) {
    rx_active = false;
    attachInterrupt(digitalPinToInterrupt(IR_RX_PIN), rx_isr, FALLING);
    return;
  }
  
  // Read 8 data bits (LSB first)
  uint8_t data = 0;
  for (int i = 0; i < 8; i++) {
    delayMicroseconds(BIT_DELAY);
    if (digitalRead(IR_RX_PIN)) {
      data |= (1 << i);
    }
  }
  
  // Wait for stop bit
  delayMicroseconds(BIT_DELAY);
  
  // Store byte
  rx_buffer[rx_head++] = data;
  
  rx_active = false;
  attachInterrupt(digitalPinToInterrupt(IR_RX_PIN), rx_isr, FALLING);
}

// TX function - bit-bang serial over IR
void ir_send_byte(uint8_t data) {
  // Start bit
  digitalWrite(IR_TX_PIN, HIGH);  // IR LED ON (active high)
  delayMicroseconds(BIT_DELAY);
  
  // Data bits (LSB first)
  for (int i = 0; i < 8; i++) {
    if (data & (1 << i)) {
      digitalWrite(IR_TX_PIN, LOW);   // IR LED OFF for '1'
    } else {
      digitalWrite(IR_TX_PIN, HIGH);  // IR LED ON for '0'
    }
    delayMicroseconds(BIT_DELAY);
  }
  
  // Stop bit
  digitalWrite(IR_TX_PIN, LOW);  // IR LED OFF
  delayMicroseconds(BIT_DELAY);
}

void loop() {
  // Forward USB Serial → IR
  while (Serial.available()) {
    uint8_t c = Serial.read();
    ir_send_byte(c);
  }
  
  // Forward IR → USB Serial
  while (rx_head != rx_tail) {
    Serial.write(rx_buffer[rx_tail++]);
  }
}
```

Wait, there's a problem with this approach. The `delayMicroseconds()` function has a minimum resolution of ~4µs on AVR,
and 8.68µs is close to the minimum. Also, the ISR for RX takes too long (it blocks for the entire byte reception time,
which is ~87µs for 10 bits). During this time, we can't send data.

For a half-duplex protocol like Cressi's (command-response), this might be OK since we don't need to TX and RX
simultaneously. But the ISR blocking for 87µs could cause issues with the main loop timing.

A better approach would be to use Timer-based bit sampling for RX, but that's more complex.

Let me refine the sketch to be more robust.

Actually, for the Cressi protocol, the communication is half-duplex and command-response. The host sends a command, then
waits for a response. So we never need to TX and RX simultaneously. This simplifies things significantly.

The key challenge is still the timing accuracy at 115200 baud. Let me use a more precise timing approach.

For AVR at 16MHz:

- 115200 baud = 8.68µs per bit = 139 clock cycles per bit
- We can use `delayMicroseconds(8)` which gives ~8µs (128 cycles), close enough
- Or we can use inline assembly NOP loops for more precise timing

Actually, the standard approach for software serial at high baud rates is to use timer interrupts. But for simplicity
and given the half-duplex nature, let me use the bit-bang approach with `delayMicroseconds()`.

Let me also consider the IR LED driver circuit. The TSAL6400 has a typical forward voltage of 1.5V and maximum
continuous current of 100mA. For pulsed operation, we can use higher currents (up to 200mA at 10% duty cycle).

With a 5V Arduino and a 1.5V forward voltage:

- Resistor = (5V - 1.5V) / 0.050A = 70Ω → use 68Ω for ~51mA
- For higher brightness: (5V - 1.5V) / 0.100A = 35Ω → use 33Ω for ~106mA

With 100mA through the IR LED, we need a transistor (Arduino pin can only source 20mA).

OK, I think I have enough information to write the complete plan. Let me do it now.

Let me structure the plan:

1. Protocol Summary (from libdivecomputer analysis)
2. IR Modulation Analysis (38kHz vs baseband)
3. Arduino Design
4. Complete Sketch
5. Wiring Diagram
6. Caveats

I'll also note the TSOP1738 limitation and recommend alternatives.

Actually, I want to think about one more thing. Is there a way to make the TSOP1738 work? What if we use a two-stage
approach:

1. Arduino receives data from host at 115200 baud
2. Arduino re-transmits data over IR at a lower baud rate (e.g., 9600 baud) with 38kHz carrier
3. Donatello receives at... wait, the Donatello expects 115200 baud. So this won't work.

Unless the Donatello can auto-detect baud rate? Looking at the code, there's no baud rate detection. It's fixed at
115200.

OK, TSOP1738 won't work. Period. Let me finalize the plan.

One last thought: What if we use the TSOP1738 but with a modified carrier frequency? The TSOP1738 is designed for 38kHz,
but what if we use a different carrier frequency that allows higher data rates? The TSOP1738's bandpass filter has a
bandwidth of about ±2kHz, so the carrier frequency must be close to 38kHz. This doesn't help with the data rate
limitation.

OK, I'm going to write the plan now. Final approach:

**TX: Baseband IR (direct on/off, no carrier)**

- IR LED driven by NPN transistor
- Arduino bit-bangs serial data

**RX: Photodiode + amplifier (not TSOP1738)**

- IR photodiode (BPW34 or similar) with transimpedance amplifier
- Output feeds Arduino interrupt pin

**Alternative: If user insists on TSOP1738**

- Can only work at lower baud rates (~9600 or lower)
- Would require a modified libdivecomputer build or a baud rate converter

Let me now write the complete deliverable.

Hmm wait, I should present this as a plan first, get user buy-in, then implement. Let me structure it as a plan with
questions.

Actually, re-reading the task: "Your current responsibility is to think, read, search, and delegate explore agents to
construct a well-formed plan." So I should present a plan.

Let me also check one more thing - can we use SoftwareSerial at 115200? Let me look at the Arduino SoftwareSerial source
to understand the limitations.

SoftwareSerial on AVR at 16MHz:

- TX: Can do 115200 (uses delayMicroseconds)
- RX: At 115200, the interrupt-based reception may miss bits due to ISR overhead

For our use case (half-duplex), we only need one direction at a time. So we could:

- Use SoftwareSerial TX for IR TX
- Use interrupt-based custom RX for IR RX

But actually, the problem with SoftwareSerial at 115200 is that the timing in the RX ISR is too tight. The ISR has ~139
cycles per bit, and the ISR entry/exit overhead alone is ~20-30 cycles, leaving only ~110 cycles for the actual bit
sampling and processing. This is barely enough.

For our half-duplex case, since we know when we're TX-ing (so we can disable RX), and when we're RX-ing (so we can
disable TX), we could use a more relaxed timing approach.

OK, I think the best approach is:

1. Use hardware Serial (pins 0/1) for USB communication with host
2. Use a custom bit-bang implementation for IR TX and RX
3. TX: Use `delayMicroseconds()` for bit timing
4. RX: Use interrupt-driven reception with Timer1 for precise bit timing

Let me design this properly.

For the RX ISR using Timer1:

1. INT0 detects falling edge (start bit)
2. Start Timer1 to generate interrupts at 115200 baud rate
3. In Timer1 ISR, sample the RX pin and accumulate bits
4. After receiving all bits, stop Timer1 and store the byte

This is more robust than using delayMicroseconds() in the ISR.

Timer1 at 115200 baud:

- 16MHz / 115200 = 138.89 → OCR1A = 139
- CTC mode, prescaler 1

The Timer1 ISR would fire every ~8.68µs, which is 139 cycles. The ISR overhead is ~25 cycles, leaving ~114 cycles for
the actual work. This should be sufficient for sampling a pin and storing a bit.

OK, let me write the plan now. I'll include the Timer1-based RX approach.

Actually, let me reconsider. The Timer1 ISR approach means we're running a 115200Hz interrupt continuously, which adds
significant overhead. We should only enable it when we expect data.

Better approach:

1. INT0 on pin 2 detects falling edge (start bit)
2. In INT0 ISR: disable INT0, start Timer1 with a half-bit delay to center on the start bit
3. Timer1 ISR: sample the pin, shift bits into a byte, count bits
4. After 10 bits (start + 8 data + stop), stop Timer1, store byte, re-enable INT0

This is cleaner. Let me implement this.

OK, I think I have a solid design now. Let me write the complete plan and sketch.

One more consideration: The IR LED driver. We want to drive the IR LED at high current (50-100mA) for good range. The
Arduino pin can't source this much current, so we need a transistor.

Simple NPN driver:

```
Arduino pin 7 → 1kΩ → NPN base
NPN emitter → GND
NPN collector → IR LED cathode
IR LED anode → 68Ω → 5V
```

This gives ~51mA through the IR LED (assuming 1.5V forward voltage).

For the photodiode RX:

```
VCC (5V or 3.3V) → 10kΩ → photodiode cathode (also to Arduino pin 2)
Photodiode anode → GND
```

This is a simple pull-up configuration. When IR light hits the photodiode, it conducts and pulls the pin LOW. When no IR
light, the pull-up keeps the pin HIGH.

The problem is that a simple pull-up circuit may not be fast enough for 115200 baud. The rise time depends on the
pull-up resistance and the pin capacitance. With 10kΩ and ~50pF, the rise time is ~500ns, which is fine for 115200
baud (8.68µs per bit).

But the photodiode current may be too small to pull the pin LOW quickly enough. A BPW34 has a typical short-circuit
current of ~50µA at 1mW/cm² IR illumination. With 10kΩ pull-up, the voltage drop would be 50µA × 10kΩ = 0.5V, which may
not be enough to register as LOW (VIL max is 1.5V for 5V Arduino, 0.3×VCC = 1.5V). So the photodiode alone may not be
sufficient.

A better approach uses a transimpedance amplifier:

```
Photodiode → op-amp (inverting input)
Feedback resistor (100kΩ) from output to inverting input
Non-inverting input → VCC/2
```

Or simpler, use a phototransistor:

```
VCC → 10kΩ → phototransistor collector (also to Arduino pin 2)
Phototransistor emitter → GND
```

A phototransistor has higher sensitivity than a photodiode. When IR light hits the base, the transistor conducts and
pulls the pin LOW.

Actually, the simplest and most reliable approach for hobbyists is to use an IR phototransistor (like TEFT4300 or
similar) with a pull-up resistor.

OK, I'll include this in the wiring diagram. Let me write the plan now.

Alright, let me also address one more thing: the user mentioned the TSAL6400 IR LED. This is a 940nm LED with a narrow
beam angle (25°). For the Cressi Donatello, we need to match the IR wavelength and alignment. The Donatello's IR port is
likely also 940nm (standard for IrDA-compatible devices). The narrow beam angle of the TSAL6400 is actually good for
this application, as it concentrates the IR light in a narrow cone, improving range and alignment.

OK, I'm going to write the complete plan now. Let me structure it clearly.

One more question I should ask the user: Do they have access to a phototransistor or photodiode, or do they only have
the TSOP1738? This affects the design significantly.

Let me present the plan with both options and ask for clarification.

Actually, I should note that there's one more approach I haven't considered: using the TSOP1738's raw output (before the
demodulator). Some people have modified TSOP modules to tap the internal photodiode signal. But this is hacky and
unreliable.

OK, final plan. Let me write it up now. I'll present:

1. Protocol summary
2. Key finding: TSOP1738 is NOT suitable for 115200 baud
3. Recommended approach: Baseband IR with phototransistor
4. Arduino sketch
5. Wiring diagram
6. Questions for the user

Let me write this up as a comprehensive plan.

Now I have a thorough understanding of the protocol. Let me read the descriptor line for the Donatello more carefully:

**Tool: read**

**Input:**

```json
{
  "filePath": "/home/vincent/www/libdivecomputer/src/descriptor.c",
  "offset": 350,
  "limit": 10
}
```

**Output:**

```
<path>/home/vincent/www/libdivecomputer/src/descriptor.c</path>
<type>file</type>
<content>
350: 	{"Cressi", "Drake",    DC_FAMILY_CRESSI_LEONARDO, 6, DC_TRANSPORT_SERIAL, NULL},
351: 	/* Cressi Goa */
352: 	{"Cressi", "Cartesio", DC_FAMILY_CRESSI_GOA, 1, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},
353: 	{"Cressi", "Goa",      DC_FAMILY_CRESSI_GOA, 2, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},
354: 	{"Cressi", "Leonardo 2.0", DC_FAMILY_CRESSI_GOA, 3, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},
355: 	{"Cressi", "Donatello",    DC_FAMILY_CRESSI_GOA, 4, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},
356: 	{"Cressi", "Michelangelo", DC_FAMILY_CRESSI_GOA, 5, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},
357: 	{"Cressi", "Neon",     DC_FAMILY_CRESSI_GOA, 9, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},
358: 	{"Cressi", "Nepto",    DC_FAMILY_CRESSI_GOA, 10, DC_TRANSPORT_SERIAL | DC_TRANSPORT_BLE, dc_filter_cressi},
359: 	/* Zeagle N2iTiON3 */

(Showing lines 350-359 of 1090. Use offset=360 to continue.)
</content>
```

---

## Assistant (Plan · MiMo-V2.5-Pro · 31.1s)

_Thinking:_

Good. Now I have all the information I need. Let me compile the complete plan.

Now I have a comprehensive understanding of the protocol. Here is my complete analysis and plan:

---

## Cressi Donatello IR Protocol Analysis

### 1. Driver Location & Model

The Donatello is **not** a standalone driver. It uses the **Cressi Goa family** (`DC_FAMILY_CRESSI_GOA`) with model ID
`4`. Key files:

- `src/cressi_goa.c` — main driver (protocol, communication)
- `src/cressi_goa_parser.c` — dive data parser
- `src/descriptor.c:355` — model registration

### 2. Serial Protocol Parameters

| Parameter        | Value                                                                                              |
|------------------|----------------------------------------------------------------------------------------------------|
| **Baud rate**    | **115200**                                                                                         |
| **Data bits**    | 8                                                                                                  |
| **Parity**       | None                                                                                               |
| **Stop bits**    | 1                                                                                                  |
| **Flow control** | None                                                                                               |
| **Timeout**      | 3000ms                                                                                             |
| **Config**       | `dc_iostream_configure(iostream, 115200, 8, DC_PARITY_NONE, DC_STOPBITS_ONE, DC_FLOWCONTROL_NONE)` |

Initialization sequence (from `cressi_goa_device_open`):

1. Configure serial port: 115200 8N1
2. Set timeout: 3000ms
3. Clear RTS = 0, clear DTR = 0
4. Sleep 100ms, purge all buffers

### 3. Packet Structure

**Command packet (host → Donatello):**

```
[0xAA][0xAA][0xAA][length][cmd][data...][CRC16-LE][0x55]
```

| Field   | Bytes | Description                                                                       |
|---------|-------|-----------------------------------------------------------------------------------|
| Header  | 3     | Always `0xAA 0xAA 0xAA`                                                           |
| Length  | 1     | Payload data length (max 12 = `SZ_PACKET`)                                        |
| Command | 1     | Command byte                                                                      |
| Data    | 0–12  | Optional payload                                                                  |
| CRC16   | 2     | CRC16-CCITT over (length + cmd + data), init=0x0000, xorout=0x0000, little-endian |
| Trailer | 1     | Always `0x55`                                                                     |

Total command packet: `length + 8` bytes

**Response packet (Donatello → host):** Same structure as command packet.

**Data download packets (for dive data):**

```
[2 bytes: total_size_LE][512 bytes: data][2 bytes: CRC16-LE]
```

Total: 516 bytes per packet. After each packet, host sends ACK (`0x06`).

After all data packets, a single END byte (`0x04`) is sent by the Donatello, and the host responds with ACK.

### 4. Commands

| Command           | Value  | Description                                                                 |
|-------------------|--------|-----------------------------------------------------------------------------|
| `CMD_VERSION`     | `0x00` | Read version/ID info (11 bytes: 4 serial + 1 model + 2 firmware + 2 format) |
| `CMD_SET_TIME`    | `0x13` | Set device time (7 bytes: year(2) + month + day + hour + min + sec)         |
| `CMD_EXIT_PCLINK` | `0x1D` | Exit PC link mode                                                           |
| `CMD_LOGBOOK`     | `0x21` | Read logbook (pre-v4 firmware)                                              |
| `CMD_DIVE`        | `0x22` | Read dive data (2 bytes: dive number LE)                                    |
| `CMD_LOGBOOK_V4`  | `0x23` | Read logbook (v4+ firmware)                                                 |
| ACK               | `0x06` | Acknowledge                                                                 |
| END               | `0x04` | End of data transfer                                                        |

### 5. Critical Finding: IR Modulation

**The libdivecomputer source code contains NO reference to any IR carrier frequency (38kHz or otherwise).** The code
treats the IR link as a transparent serial connection — it configures baud rate, parity, stop bits via standard serial
port APIs, and sends/receives raw bytes. The physical IR modulation is handled entirely by the dock hardware.

**The TSOP1738 is NOT suitable for 115200 baud.** The TSOP1738 has a minimum burst length of ~10 carrier cycles at
38kHz = 263µs. At 115200 baud, a single bit is only 8.68µs — far too short for the TSOP1738 to detect. The Cressi BT
Interface dock likely uses either:

- A raw IR photodiode + transimpedance amplifier (baseband IR, no carrier)
- A high-speed IR receiver (not TSOP-type)

This is confirmed by comparing with the older Cressi Edy driver, which uses 1200–4800 baud (where TSOP-type receivers DO
work), vs. the Goa/Donatello at 115200 baud.

### 6. Recommended Arduino Design

Given the TSOP1738 limitation, there are two viable approaches:

**Approach A (Recommended): Baseband IR — direct on/off, no carrier**

- TX: IR LED driven by NPN transistor, directly modulated at serial bit rate
- RX: IR phototransistor (e.g., TEFT4300) or photodiode with amplifier
- Supports 115200 baud natively
- This is almost certainly what the Cressi dock uses internally

**Approach B: TSOP1738 at reduced baud rate**

- Would require modifying libdivecomputer to use a lower baud rate
- May not work if the Donatello hardware is fixed at 115200
- NOT recommended

### 7. Questions for You

Before I write the final implementation, I have a few questions:

1. **IR RX component**: Do you have access to an IR phototransistor (TEFT4300, PT334-6C, etc.) or a plain IR
   photodiode (BPW34, BPW41N)? The TSOP1738 won't work at 115200 baud. If you only have a TSOP1738, I can design for it
   but you'll need to acquire a phototransistor.

2. **Arduino model**: You mentioned Uno or Nano. Both use FTDI/CH340 for USB, meaning hardware UART (pins 0/1) is
   occupied by USB. The IR link must use bit-banged serial (software serial). At 115200 baud this is tight but feasible
   for half-duplex communication (which the Cressi protocol is). Do you have a preference, or would you consider an
   Arduino Leonardo/Pro Micro (ATmega32U4) which has native USB and a free hardware UART?

3. **Physical alignment**: The Cressi Donatello has a small optical window on the back/side of the watch. The TSAL6400
   has a narrow 25° beam angle which is good for alignment, but you'll need a physical fixture (3D printed cradle, hot
   glue, or similar) to hold the IR LED and phototransistor in precise alignment with the Donatello's optical port. Are
   you comfortable with this mechanical aspect?

4. **IR wavelength**: The TSAL6400 is 940nm, which is the standard for IrDA-compatible devices. The Donatello's IR port
   is almost certainly also 940nm. Can you confirm from any markings on the Donatello or its dock?

### 8. Proposed Arduino Sketch Architecture

Here's the high-level design I plan to implement:

```
┌─────────────┐     USB Serial      ┌──────────────┐
│  Host PC    │◄──────────────────►│   Arduino    │
│ (Subsurface)│   115200 8N1        │  (Uno/Nano)  │
└─────────────┘                     │              │
                                    │  Timer1 ISR  │──► IR LED (TX)
                                    │  INT0 + ISR  │◄── Phototransistor (RX)
                                    └──────────────┘
                                           │ IR
                                           ▼
                                    ┌──────────────┐
                                    │   Cressi     │
                                    │  Donatello   │
                                    └──────────────┘
```

- **USB Serial** (hardware UART, pins 0/1): Host PC communication at 115200 baud
- **IR TX** (pin 7 + Timer2 38kHz optional, or direct drive): Bit-banged serial output
- **IR RX** (pin 2, INT0): Interrupt-driven serial input with Timer1 for precise bit sampling
- **Half-duplex**: The Cressi protocol is command-response, so we never TX and RX simultaneously

### 9. Proposed Wiring (Approach A: Baseband IR)

```
                    ┌──────────────────┐
                    │   Arduino Uno    │
                    │   / Nano         │
                    │                  │
  USB ──────────►   │  D0 (RX)        │
  (to host PC)  ◄── │  D1 (TX)        │
                    │                  │
                    │  D7 ──►[1kΩ]──► NPN Base        (IR TX)
                    │                  NPN Emitter ──► GND
                    │                  NPN Collector──► IR LED Cathode
                    │                                   IR LED Anode ──► [68Ω] ──► 5V
                    │                  │
                    │  D2 (INT0) ◄── Phototransistor   (IR RX)
                    │                  Collector ──► 5V via 10kΩ
                    │                  Emitter ──► GND
                    │                  │
                    │  5V  ────────────┤
                    │  GND ────────────┘
                    └──────────────────┘
```

Want me to proceed with writing the full Arduino `.ino` sketch and detailed wiring diagram based on Approach A (baseband
IR with phototransistor)?

---

## User

Wait, read theses two pages first and re-assess the problem: (I want only USB connection with libdivecomputer, not BT
btw)

https://github.com/subsurface/subsurface/issues/4147

```
@hb9eue
hb9eue
opened on Mar 28, 2024
Hi Devs

Sorry for being to lazy to dig into the depths of the source.

I own a Cressi Leonardo and recently got Cressi Donatello for another family member.

The Cressi Leonardo uses serial IRDA @115200 baud for communication with it's vendor specific, very expensive RS232 cradle.
An ESP32 with a TFDU4101 can also do IRDA for a fraction of that price.

I tinkered around a bit, and after commenting out DTR/RTS toggles, which are required to initialize the vendor craddle, but which of course cause the USB-Serial interface of an ESP32 bord to reboot, I got import for the Leonardo working!

The newly purchased Donatello is advertised to communicate via Bluetooth-Serial! Yeah!

What a disappointment, when I realized you once more have to buy a very expensive additional bluetooth interface. But wait, the Donatello has some hole at the exact same location on which the Leonardo has it's IRDA interface. Is that Bluetooth vendor interface a simple IRDA to Bluetooth-Serial interface?

Let's comment out the DTR/RTS toggles in libdivecomputer for the goa/donatello model.

Success! I immediately can connect to the donatello and get's it's firmware version and modell information. (no dives yet).

And ESP32 has bluetooth too. So why not try to reconstruct that vendor bluetooth to IRDA interface with an ESP32 and then use Bluetooth-Serial to import the dives. In theory, that should also work from the subsurface anrdoid app.

Unfortunately I am stuck with the android app. I am able to pair Android to my ESP32 with the Bluetooth-Serial service. And I am able to select the bluetooth device in subsurface android.

But it looks like android is never properly opening the serial connection when I select 'import'. On the USB serial monitor on the ESP32 which I now can use to log some debugging information, I never get any data via bluetooth serial from the android app.

When I select the Bluetooth Device in subsurface andoid, vendor and type of the divecomputer also are being preset to something that is probably deducted from the BT MAC Address of my ESP32.

So that makes me wonder. Does the android app somehow check the MAC address of the BT device somehow defaults to only speak the BT protocol associated with that MAC address?

-Benoit-

Activity
KonradIT commented on Jul 20, 2024
@KonradIT
KonradIT
on Jul 20, 2024
Hi @hb9eue

I'm interested in your work, how did you wire the TFDU4101 to the ESP32? Breakout board? How did you encode whatever is received on the ESP32 from the IR sensor so Subsurface could read it?

hb9eue commented on Aug 16, 2024
@hb9eue
hb9eue
on Aug 16, 2024
Author
Sorry for the late reply.

You can switch the uart port of the esp32 to IRDA mode. So no need for any further encoding. The only real issue I faced is that the TFDU4101 is receiving the echo of the character being sent on the RX side. So I drop the character if it's the same I just sent. This would most probably not work in a real dupliex communication. Thankfully the cressi divecomputer receive the command to dump their memory and then send the dump, so it's 'half duplex' which works.

I wired the TFDU4101 as per data-sheet with a resistor (don't know remember much) in front of the RX led to limit current.

Here is my arduino code for the Heltec ESP32 board:

#include <BTAddress.h>
#include <BTAdvertisedDevice.h>
#include <BTScan.h>
#include <BluetoothSerial.h>

#define BUILTIN_LED 2

HardwareSerial irda(2); // GPIO 17: TXD U2  +  GPIO 16: RXD U2
// NOT the ones labeled TX and RX, real GPIO 17 and 16 on Heltec
int serialByte;
int irdaByte;
bool sent;
bool usbmode = false;
String device_name = "DiveComputerBT";

// Check if Bluetooth is available
#if !defined(CONFIG_BT_ENABLED) || !defined(CONFIG_BLUEDROID_ENABLED)
  #error Bluetooth is not enabled! Please run `make menuconfig` to and enable it
#endif

// Check Serial Port Profile
#if !defined(CONFIG_BT_SPP_ENABLED)
  #error Serial Port Profile for Bluetooth is not available or not enabled. It is only available for the ESP32 chip.
#endif

BluetoothSerial SerialBT;

void setup() {
  pinMode(BUILTIN_LED, OUTPUT);
    Serial.begin (115200); // (USB + TX/RX) to check
    irda.begin(115200); // GPIO 17: TXD U2  +  GPIO 16: RXD U2

    //UART_CONF0_REG  Configuration register 0
    //UART0: 0x3FF40020
    //UART1: 0x3FF50020
    //UART2: 0x3FF6E020

    WRITE_PERI_REG( 0x3FF6E020 , READ_PERI_REG(0x3FF6E020) | (1<<16 ) | (1<<9 ) );  //UART_IRDA_EN + UART_IRDA_DPLX  "Let there be light"
    //Serial.print("Reg: "); Serial.println(READ_PERI_REG(0x3FF6E020),BIN);  //For Debug only
    //Serial.println("Ready");
    digitalWrite(BUILTIN_LED, HIGH);
    delay(100);
    digitalWrite(BUILTIN_LED, LOW);
//    irda.print("{123DBA}");
  SerialBT.begin(device_name); //Bluetooth device name
  // SerialBT.deleteAllBondedDevices(); // Uncomment this to delete paired devices; Must be called after begin
  Serial.printf("The device with name \"%s\" is started.\nNow you can pair it with Bluetooth!\n", device_name.c_str());
}//setup

void loop() {
  if (usbmode) {
    while (Serial.available()) {
      serialByte = Serial.read();
      irda.print((char) serialByte);
      sent = true;
      delay(5); // Obtained by try and error wait until a potential echo is received
      break; // Immediately check if we received and echo
    }
  } else {
    while (SerialBT.available()) {
      serialByte = SerialBT.read();
      irda.print((char) serialByte);
      Serial.print((char) serialByte);
      sent = true;
      delay(5); // Obtained by try and error wait until a potential echo is received
      break; // Immediately check if we received and echo
    }
  }
  while (irda.available()) {
    irdaByte = irda.read();
    if ((irdaByte == serialByte) && sent) {
    //if (irdaByte == serialByte) {
      //Serial.print("Echo:");
      //Serial.println((char) irdaByte);
    } else {
      //Serial.print("Received:");
      //Serial.println((char) irdaByte);
      if (usbmode) {
        Serial.print((char) irdaByte);
      } else {
        SerialBT.print((char) irdaByte);
        Serial.print((char) irdaByte);
      }
    }
    sent = false;
  }
}
sebdbr commented on Nov 1, 2024
@sebdbr
sebdbr
on Nov 1, 2024
Hi @hb9eue !
Since the Cressi BT interface for Leonardo is no longer available, I'm also interested in your work, and i'm trying to build one for my son, who would like to record his dives on his PC.
As a newbie, I need more information :
I guess that I have to build subsurface from sources AFTER modifying the cressi_leonardo.c file in subfolder subsurface/libdivecomputer/src/...
Am I right ?
But which lines should I comment out exactly ?
From line 199 to 212 ?

// Set the RTS line.
	status = dc_iostream_set_rts (device->iostream, 1);
	if (status != DC_STATUS_SUCCESS) {
		ERROR (context, "Failed to set the RTS line.");
		goto error_free;
	}

	// Set the DTR line.
	status = dc_iostream_set_dtr (device->iostream, 1);
	if (status != DC_STATUS_SUCCESS) {
		ERROR (context, "Failed to set the DTR line.");
		goto error_free;
	}

	dc_iostream_sleep (device->iostream, 200);
Hope that you could help me !

On the other hand, have you finally found a way to make it work with the android app ?

Seb

hb9eue commented on Nov 1, 2024
@hb9eue
hb9eue
on Nov 1, 2024
Author
Hi Seb

It depends on how you connect the ESP32 with your computer. If you use Bluetooth serial, subsurface does not need any modification.

Only if you connect the ESP32 via USB Serial, you need to modify the source as you have to remove the part which toggles the DTR line to initialize the original craddle as this causes a reset of the ESP32.

To disable this, in ./libdivecomputer/src/cressi_leonardo.c

function: cressi_leonardo_device_open

Just comment out this whole block as I did here:
[...]

        // Set the timeout for receiving data (1000 ms).
        status = dc_iostream_set_timeout (device->iostream, 1000);
        if (status != DC_STATUS_SUCCESS) {
                ERROR (context, "Failed to set the timeout.");
                goto error_free;
        }

/*
        // Set the RTS line.
        status = dc_iostream_set_rts (device->iostream, 1);
        if (status != DC_STATUS_SUCCESS) {
                ERROR (context, "Failed to set the RTS line.");
                goto error_free;
        }

        // Set the DTR line.
        status = dc_iostream_set_dtr (device->iostream, 1);
        if (status != DC_STATUS_SUCCESS) {
                ERROR (context, "Failed to set the DTR line.");
                goto error_free;
        }

        dc_iostream_sleep (device->iostream, 200);

        // Clear the DTR line.
        status = dc_iostream_set_dtr (device->iostream, 0);
        if (status != DC_STATUS_SUCCESS) {
                ERROR (context, "Failed to clear the DTR line.");
                goto error_free;
        }
*/
        dc_iostream_sleep (device->iostream, 1000);
        dc_iostream_purge (device->iostream, DC_DIRECTION_ALL);

[...]

And no, unfortunately still no success with the Android app. I guess the app check the Bluetooth MAC address to match a device and as at least my ESP32 seems to match some other device type, it does not attempt to connect using the cresi leonardo or donatello protocol.

-Benoît-

sebdbr commented on Nov 16, 2024
@sebdbr
sebdbr
on Nov 16, 2024
Hello @hb9eue,
First of all, thanks for your tip.
After some struggles to adapt your DIY interface for the NodeMCU esp32 (on which the UART2 was out of order...) and tfdu4800 (not exactly the same pinout than the 4801) that i had, it seems to me I managed to get an answer from the Leonardo on USB serial port, but there are still some problems.
Here are some examples of the libdivecomputer log in Subsurface.

Subsurface: v6.0.5282.0, built with libdivecomputer v0.9.0-devel-Subsurface-NG (a2a5bb53d670dc520bf8035b18681dfdd9b18bfa)
[3175,516999] INFO: Open: name=/dev/ttyUSB0
[3175,525601] INFO: Configure: baudrate=115200, databits=8, parity=0, stopbits=0, flowcontrol=0
[3175,531581] INFO: Timeout: value=1000
[3175,531599] INFO: Sleep: value=100
[3175,631686] INFO: Purge: direction=3
[3175,632444] INFO: Write: size=8, data=7B3132334442417D
[3175,670707] INFO: Read: size=7, data=C000380000F8C0
[3175,670720] ERROR: Unexpected answer byte. [in src/cressi_leonardo.c:326 (cressi_leonardo_device_dump)]

Subsurface: v6.0.5282.0, built with libdivecomputer v0.9.0-devel-Subsurface-NG (a2a5bb53d670dc520bf8035b18681dfdd9b18bfa)
[9503,232466] INFO: Open: name=/dev/ttyUSB0
[9503,241359] INFO: Configure: baudrate=115200, databits=8, parity=0, stopbits=0, flowcontrol=0
[9503,241388] INFO: Timeout: value=1000
[9503,241394] INFO: Sleep: value=100
[9503,342144] INFO: Purge: direction=3
[9503,342750] INFO: Write: size=8, data=7B3132334442417D
[9503,380888] INFO: Read: size=7, data=80000000008080
[9503,380900] ERROR: Unexpected answer byte. [in src/cressi_leonardo.c:326 (cressi_leonardo_device_dump)]

If you have any ideas...

-Seb- 
```

https://groups.google.com/g/subsurface-divelog/c/ku56SSlCtZU

```
Daniel Samarin
non lue,
5 mars 2024, 19:52:17
à Subsurface Divelog
Hi all,

While this is not strictly Subsurface related, I wanted to reach out to this tech savvy group for interested beta testers for a device a coworker and I put together to support some older (but still capable!) dive computers that are quickly becoming obsoleted by their manufacturers. Obsolescence be damned!

usb-to-irda.png

I recently became certified in scuba and bought a very inexpensive Cressi Leonardo dive computer used on eBay. I couldn't find the USB dock for my Cressi for purchase anywhere, nor similar devices for an reasonable price. Discussed this with a few of you here (thanks!):
https://groups.google.com/g/subsurface-divelog/c/LDNWUFqFF-E/m/Aag04v84AAAJ

So, I came up with this simple concept (based on Chris Kuethe's initial guidance) and had one of my engineers complete and test the design as a training exercise. And it worked!

The end goal here is to open-source the design, once I confirm it works on more than a few dive computers, and sell kits/devices basically at-cost (NOT the 80-120 USD they want on eBay!).  So, I'm looking for a few people with IR-based dive computers to help us beta test. I'd send you the device at no charge, and ask you to sync with Subsurface to confirm it's working. If it doesn't work I'll ask for the log files.

I'm looking for a wide variety of dive computers as I can get, and a handful of operating systems: Win10, Win11, Linux (at least Ubuntu), and MacOS. If you also have a tablet to try on, perhaps we can try that as a bonus.

I understand these kinds of dive computers are more rare these days with many of them being direct-USB, and Bluetooth becoming less expensive to integrate, but with at least Cressi still pumping out Leonardos and similar watches, I suspect there will be some need for this device.

Thanks in advance!

--Daniel
Photo du profil de Robert C. Helling
Robert C. Helling
non lue,
5 mars 2024, 20:32:55
à Subsurface Divelog

This looks extremely cool. I am sorry I cannot help as I don’t have an irda drive computer. As far as software is concerned, IIRC macOS does not include an irda stack, windows dropped it and then later brought it back and also Linux dropped it at some point. That might be an additional challenge. Subsurface does not include one but some other die logs do. 

Best
Robert
Photo du profil de Chris Kuethe
Chris Kuethe
non lue,
5 mars 2024, 21:48:31
à subsurfac...@googlegroups.com
On Tue, Mar 5, 2024 at 11:33 AM 'Robert C. Helling' via Subsurface Divelog <subsurfac...@googlegroups.com> wrote:

This looks extremely cool. I am sorry I cannot help as I don’t have an irda drive computer. As far as software is concerned, IIRC macOS does not include an irda stack, windows dropped it and then later brought it back and also Linux dropped it at some point. That might be an additional challenge. Subsurface does not include one but some other die logs do. 

I don't think an IrDA stack is necessary since my Cressi dock just shows up like a tty (com port) and subsurface will happily talk to it - as far as I can tell, all the IrDA magic is done inside the transceiver chip inside the dock. If I plug my phone into the dock with the right combination of adapters, I can even download into subsurface on my phone, from the dive boat.

--
GDB has a 'break' feature; why doesn't it have 'fix' too?
Photo du profil de Daniel Samarin
Daniel Samarin
non lue,
5 mars 2024, 22:11:25
à Subsurface Divelog
I suspected this might be the case! I think this is really just normal serial data at the computer side, not specifically IRDA. That was the intent with the one-two punch "USB to serial chip" and "serial to IRDA chip" in the circuit design above.
If the assumption is right, then I wouldn't imagine any platform wouldn't have the right drivers by default, since so many everyday devices rely on USB-serial connections.

--Daniel
Photo du profil de Linus Torvalds
Linus Torvalds
non lue,
5 mars 2024, 22:15:36
à subsurfac...@googlegroups.com
On Tue, 5 Mar 2024 at 12:48, Chris Kuethe <chris....@gmail.com> wrote:
>
> I don't think an IrDA stack is necessary since my Cressi dock just shows up like a tty (com port) and subsurface will happily talk to it - as far as I can tell, all the IrDA magic is done inside the transceiver chip inside the dock

There are multiple "levels" of the IrDA stack, and I suspect your
Cressi simply only does the physical level (and probably only "SIR",
which is the simplest and most traditional one).

IrDA SIR is basically just "half duplex rs232 over infrared".

In fact, in many ways it's _simpler_ than rs232, because there's no
12V line, and there's none of that fancy DTR/CSR stuff or any question
about stop bits and parity. You do need to have a baud-rate, but
that's it.

Of course, modern rs232 is usually not the "real" traditional rs232
and is generally really just 5V anyway, and most USB serial devices
out there do *not* do any level shifting, and you'll find people who
complain about them not working with historical devices that really
did want the whole 12V thing...

So it sounds like the Cressi dock is really just IrDA SIR. Which is
the sane part of IrDA anyway.

The *whole* IrDA thing is a much more complex mess that tries to
support multiple concurrent devices, and is why Linux dropped support
for it - without any relevant devices to really test, all the crazy
extra complexity and odd socket-based interfaces were just completely
unmaintainable.

So there's the simple physical layer which is literally just that
"rs232 over infrared". Then there's "IrLAP" on top of that, which is a
device discovery, and connection establisment, and reliable data link
layer.

Then there's various protocols on top of _that_ that make IrDA look
like a centronics parallel port or implement teh flow control parts of
rs-232 etc.

And all of it is "IrDA", but most of it is _much_ too complicated for
simple devices that just wanted a serial line and do their own error
correction and re-sending anyway.

(I also suspect the Linux code was written based off a spec, rather
than a sane technical understanding of what the implementation was
meant to be. There's a disease in the CS community where people think
that protocols have to be described independently of their
implementation, and while the whole "OSI 7-layer model" is rightfully
ridiculed today, there's a lot of specs that really try to "layer" the
spec explanation so much that it's hard to understand what the reality
actually is supposed to look like, and then peopel who write software
based off the spec tend to create this layer mess where each
individual layer makes very little sense because it was basically
intentionally explained in a vacuum. That then made it hard to sanely
even just support the simple cases, because the kernel code was
literally written to be the most complex case possible)

Linus
Photo du profil de Daniel Samarin
Daniel Samarin
non lue,
5 mars 2024, 23:45:40
à Subsurface Divelog
Thanks Linus, that makes a lot of sense. I think, then, that we have semi-accidentally happened upon the easy way out of this, which is to just have the PC interface with a normal modern serial/COM port. I hope that the other dive computer mfgs have implemented it like Cressi has.

Still looking for beta testers!

--Daniel
Photo du profil de Charles H
Charles H
non lue,
7 mars 2024, 03:41:30
à Subsurface Divelog
I'm in Melbourne, Australia, but I have a Galileo Sol as my backup (it's more reliable than my G2), a Windows 11 desktop. a Windows 11 notebook and both a desktop and a notebook both running Linux Mint (Ubuntu base). Happy to assist if you don't have any closer takers, but I am not a computer whizz (unless you count 1980s MS-DOS Basic and Pascal, and OS2 experience :) ).Prepared to try with an Ubuntu installation if I can't get it to work with Mint.
Cheers,
Charles
Photo du profil de Charles H
Charles H
non lue,
7 mars 2024, 03:54:19
à Subsurface Divelog
By the way, in case its relevant,  I recently tried to interface with an FT232R via USB port , and had no luck at all under Mint, either with the presumed preinstalled driver or the one from FTDI, not even after disabling the Braille driver (don't ask!), but no problem under Windows 11.

On Wednesday, March 6, 2024 at 5:52:17 AM UTC+11 Daniel Samarin wrote:
Photo du profil de Daniel Samarin
Daniel Samarin
non lue,
8 mars 2024, 01:00:41
à Subsurface Divelog
Great, thanks Charles. Added you to the list of interested beta testers. Please send me an individual email so I can have yours and contact you when we are ready with our next batch.

It really surprises me that any modern linux distribution doesn't support FTDI, but that company has been weird about drivers and fake components in the past. Our USB-Serial chip is the MCP2221A, which I hope is better supported across operating systems.

--Daniel
Photo du profil de Linus Torvalds
Linus Torvalds
non lue,
8 mars 2024, 01:21:02
à subsurfac...@googlegroups.com
On Thu, 7 Mar 2024 at 16:00, Daniel Samarin <dsam...@gmail.com> wrote:
>
> It really surprises me that any modern linux distribution doesn't
> support FTDI, but that company has been weird about drivers and fake
> components in the past.

I would expect any general-purpose Linux distro to enable FTDI
support, and that should automatically include the FT232R chip.

But if there's a new chip out with a new version number, that could
certainly mean that the chip simply isn't _recognized_ as a FT232R.

Also, vendors can - and some do - take FTDI chips, and then set their
own vendor IDs etc on it, which again means that what may well be an
FTDI chip doesn't necessarily get recognized as such without some
added quirk.

> Our USB-Serial chip is the MCP2221A, which I hope is better supported
> across operating systems.

Hmm. The only MCP2221A chip I'm aware of is a USB to i2c thing? A form
of serial too, and presumably you then have a i2c irda chip?

Linus
Photo du profil de Andrew Goldby
Andrew Goldby
non lue,
8 mars 2024, 04:55:53
à subsurfac...@googlegroups.com
As an aside to this thread has anyone made a similar connector for the 2 metal pin computers eg mares watches?

Sent from Outlook for Android
From: subsurfac...@googlegroups.com <subsurfac...@googlegroups.com> on behalf of Daniel Samarin <dsam...@gmail.com>
Sent: Thursday, March 7, 2024 5:00:41 PM
To: Subsurface Divelog <subsurfac...@googlegroups.com>
Subject: Re: Beta testers wanted: infrared-to-USB transceiver
 
--
You received this message because you are subscribed to the Google Groups "Subsurface Divelog" group.
To unsubscribe from this group and stop receiving emails from it, send an email to subsurface-dive...@googlegroups.com.
To view this discussion on the web visit https://groups.google.com/d/msgid/subsurface-divelog/9b5dbe53-8f94-4425-8e25-362f5e737e63n%40googlegroups.com.
Photo du profil de chhackm
chhackm
non lue,
9 mars 2024, 12:43:23
à subsurfac...@googlegroups.com
Hi Andrew!
Charles Hackman here.
I'm not sure if it's really a driver issue in linux Mint: it's the "don't have permission" message, which has also come up in ordinary USB connection with my G2. It hasn't been especially relevant until now.
Cheers,
Charles 



Sent from my Galaxy
You received this message because you are subscribed to a topic in the Google Groups "Subsurface Divelog" group.
To unsubscribe from this topic, visit https://groups.google.com/d/topic/subsurface-divelog/ku56SSlCtZU/unsubscribe.
To unsubscribe from this group and all its topics, send an email to subsurface-dive...@googlegroups.com.
To view this discussion on the web visit https://groups.google.com/d/msgid/subsurface-divelog/PAXP251MB0727797432124ED1004F6725FF272%40PAXP251MB0727.EURP251.PROD.OUTLOOK.COM.
Photo du profil de Daniel Samarin
Daniel Samarin
non lue,
11 mars 2024, 01:42:38
à Subsurface Divelog
On Thursday, March 7, 2024 at 4:21:02 PM UTC-8 Linus Torvalds wrote:
Hmm. The only MCP2221A chip I'm aware of is a USB to i2c thing? A form
of serial too, and presumably you then have a i2c irda chip? 

The MCP2221A enumerates as both a serial com port and an i2c device! Quoting the datasheet:
"The MCP2221A enumerates as a composite USB device after POR. The device enumerates as both a Human Interface Device (HID) for I2C, GPIO control, and as CDC for the USB-to-UART converter."
Photo du profil de Aryel Sharon
Aryel Sharon
non lue,
24 avr. 2024, 12:51:49
à Subsurface Divelog
Hi, I'm in Maceió, Brazil, i have a Cressi Giotto as my main computer and my uncle have an Mares Genius(with genuine interface), a Windows 10 desktop but i can install whatever OS needed on an virtual machine or my laptop, i hope i can help the project, i'm not familiar with IRdA protocol but i have worked on some IR projects with Arduino IDE.
A hug from Brazilian divers 
Photo du profil de poe...@xs4all.nl
poe...@xs4all.nl
non lue,
30 avr. 2024, 17:28:18
à Subsurface Divelog
Hi,

Got my 2USBtoIRDA-Kit today. I'm using LinuxMint 21.3 and Subsurface 6.0.5144-CICD-release. Hooked up the device to my USB-Hub and linux registers the device:
[26675.717562] usb 3-4.1.2: new full-speed USB device number 9 using xhci_hcd
[26675.846151] usb 3-4.1.2: New USB device found, idVendor=04d8, idProduct=00dd, bcdDevice= 1.00
[26675.846154] usb 3-4.1.2: New USB device strings: Mfr=1, Product=2, SerialNumber=0
[26675.846155] usb 3-4.1.2: Product: MCP2221 USB-I2C/UART Combo
[26675.846157] usb 3-4.1.2: Manufacturer: Microchip Technology Inc.
[26675.857743] cdc_acm 3-4.1.2:1.0: ttyACM0: USB ACM device
[26675.858658] mcp2221 0003:04D8:00DD.000B: USB HID v1.11 Device [Microchip Technology Inc. MCP2221 USB-I2C/UART Combo] on usb-0000:00:14.0-4.1.2/input2

Then i start Subsurface and i found out if i select 'Import from Divecomputer' there is no Divecomputer 'Galileo Sol' with either brand 'Uwatec' or 'Scubapro', although the Sol is supported. What is missing?

By the way: if i select something else i can choose /dev/ttyACM0, but then the infrared-icon on my Sol doesn' t show.

Regards, Robert

Op woensdag 24 april 2024 om 10:51:49 UTC schreef Aryel Sharon:
Photo du profil de Daniel Samarin
Daniel Samarin
non lue,
2 mai 2024, 07:00:27
à Subsurface Divelog
Hi Robert,

Thanks for your patience! There is something different or unique with how Subsurface handles the Galileo Sol. On my computer (Win10) it greys out the "Device or mount point" selection so I can't select anything. We're going to need to dig into the source code to understand why and that may take some time.

Fortunately, we know our device works with Cressi several dive computers as expected, across Linux/Windows/MacOS/Android/iOS, so now we just need to figure out what is special about Uwatec.

To Aryel, 
I am confident the device will work with the Giotto. As for the Genius, I believe that dive computer uses Bluetooth for data transfer, no?

--Daniel
Photo du profil de Jef Driesen
Jef Driesen
non lue,
2 mai 2024, 09:33:24
à subsurfac...@googlegroups.com,Daniel Samarin
On 2024-05-02 07:00, Daniel Samarin wrote:
> Thanks for your patience! There is something different or unique with
> how Subsurface handles the Galileo Sol. On my computer (Win10) it greys
> out the "Device or mount point" selection so I can't select anything.
> We're going to need to dig into the source code to understand why and
> that may take some time.
>
> Fortunately, we know our device works with Cressi several dive
> computers as expected, across Linux/Windows/MacOS/Android/iOS, so now
> we just need to figure out what is special about Uwatec.

The explanation is easy. The Galileo and some of the other
Uwatec/Scubapro models are IrDA enabled dive computers, and are marked
as such in the source code:

https://github.com/libdivecomputer/libdivecomputer/blob/master/src/descriptor.c#L144

Thus when you select one of those models, subsurface will offer the
available IrDA devices that were found during the IrDA device discovery.
It won't offer any usb-serial devices, because normally the dive
computer doesn't support those, and it makes no sense to offer choices
to the user that are not going to work.

In theory you could add the DC_TRANSPORT_SERIAL transport (or just
select a model where this transport is already present like the Scubapro
Meridian), but I doubt that is going to work. That's because the framing
used in the serial communication is a bit different from the one used in
the IrDA communication:

https://github.com/libdivecomputer/libdivecomputer/blob/master/src/uwatec_smart.c#L500-L520

So if your device is simply passing through the data unchanged and just
translates between usb-serial and IrDA communication, then this won't
work. The easiest workaround is probably to use a custom transport that
is actually a serial transport, but disguises itself as an IrDA
transport. Something like this:

dc_custom_cbs_t callbacks = {
dc_iostream_set_timeout, /* set_timeout */
dc_iostream_set_break, /* set_break */
dc_iostream_set_dtr, /* set_dtr */
dc_iostream_set_rts, /* set_rts */
dc_iostream_get_lines, /* get_lines */
dc_iostream_get_available, /* get_available */
dc_iostream_configure, /* configure */
dc_iostream_poll, /* poll */
dc_iostream_read, /* read */
dc_iostream_write, /* write */
dc_iostream_ioctl, /* ioctl */
dc_iostream_flush, /* flush */
dc_iostream_purge, /* purge */
dc_iostream_sleep, /* sleep */
dc_iostream_close, /* close */
};

// Open the underlying serial transport.
dc_iostream_t *tmp = NULL;
status = dc_serial_open (&tmp, context, devname);

// Disguise it as an IrDA transport.
dc_iostream_t *iostream = NULL;
status = dc_custom_open (&iostream, NULL, DC_TRANSPORT_IRDA,
&callbacks, tmp);

> To Aryel,
> I am confident the device will work with the Giotto. As for the Genius,
> I believe that dive computer uses Bluetooth for data transfer, no?

The Genius supports both serial and BLE communication.

Jef
Photo du profil de Aryel Sharon
Aryel Sharon
non lue,
9 mai 2024, 03:47:31
à Subsurface Divelog
Nah, they use an serial port, i guess

mares png.PNG
Photo du profil de Aryel Sharon
Aryel Sharon
non lue,
9 mai 2024, 03:48:45
à Subsurface Divelog
>To Jef
oh, i never realized that genius have BT support, that mean i can use subsurface on my Android and sync my dives direct from the phone, right?
Photo du profil de Aryel Sharon
Aryel Sharon
non lue,
9 mai 2024, 03:53:25
à Subsurface Divelog
>to Daniel 
any news about when come for sale?i mean i just want to use with giotto, not with both genius and giotto

thx and great day for everyone
Em quinta-feira, 2 de maio de 2024 às 02:00:27 UTC-3, Daniel Samarin escreveu:
Photo du profil de Yann Liebaert
Yann Liebaert
non lue,
17 juil. 2024, 11:13:04
à Subsurface Divelog
Is there any news on the progress? I'm about to buy a Donatello, but the only thing holding me back is the Cressi proprietary interface. This would definitely solve my issue ;-) 
```

---

## Assistant (Plan · MiMo-V2.5-Pro · 80.7s)

_Thinking:_

This is extremely valuable information! Let me re-analyze the whole situation based on these two sources.

Key revelations from the GitHub issue and mailing list:

1. **The Cressi Donatello uses IrDA SIR** - not raw 38kHz IR, not baseband IR. It's standard IrDA SIR (Serial Infrared),
   which is "half duplex RS232 over infrared" as Linus Torvalds described it.

2. **The Cressi BT Interface dock is an IrDA-to-Bluetooth bridge** - The dock has an IrDA transceiver chip (like
   TFDU4101 or similar) that communicates with the dive computer optically, and a Bluetooth module that relays to the
   host. When connected via USB, the dock's USB-to-serial chip connects to the IrDA transceiver.

3. **hb9eue successfully used an ESP32 with a TFDU4101 IrDA transceiver** to communicate with the Donatello! The ESP32's
   UART has a hardware IrDA mode that handles the modulation. He connected it at 115200 baud and it worked.

4. **The IrDA SIR modulation** is NOT 38kHz like a TV remote. IrDA SIR uses a specific modulation scheme:
    - Carrier frequency: 3/16 bit period
    - At 9600 baud (standard IrDA SIR): pulse width = 3/16 × 104.17µs = ~19.5µs
    - But at 115200 baud: pulse width = 3/16 × 8.68µs = ~1.63µs

   Wait, actually IrDA SIR is defined as operating at 9600 baud for the basic rate. Higher rates use MIR (1.152 Mbps)
   and FIR (4 Mbps). But the Cressi uses 115200 baud...

   Actually, let me reconsider. The IrDA SIR specification defines the physical layer as:
    - 9600 baud default
    - Can negotiate up to 115200 baud (this is sometimes called "SIR" or "extended SIR")
    - The optical pulse is 3/16 of a bit period at 9600 baud
    - At higher baud rates, the pulse width ratio changes

   But looking at hb9eue's code, he just enables the ESP32's hardware IrDA mode on UART2 and it works at 115200 baud.
   The ESP32's IrDA hardware handles the modulation automatically.

5. **The TFDU4101 is an IrDA transceiver** - it's a complete IrDA physical layer chip that handles:
    - IR LED driving (TX)
    - Photodiode reception (RX)
    - Modulation/demodulation
    - It connects to a UART TX/RX pins directly

6. **The echo issue** - hb9eue mentions that the TFDU4101 echoes transmitted characters on the RX side. This is a known
   characteristic of some IrDA transceivers in half-duplex mode.

7. **Daniel Samarin's device** - He created a "USB-to-IrDA" adapter using an MCP2221A (USB-to-serial/I2C chip) + an IrDA
   transceiver chip. This is essentially what the Cressi BT Interface dock does internally.

So the correct approach is:

- Use an **IrDA transceiver chip** (TFDU4101, TFDU4108, or similar) - NOT a TSOP1738
- Connect it to a UART at 115200 baud
- The IrDA transceiver handles all the IR modulation/demodulation

For an Arduino Uno/Nano approach:

- The Arduino's hardware UART (pins 0/1) is used for USB (via FTDI/CH340)
- We need another UART for the IrDA link
- SoftwareSerial at 115200 is tight but possible for half-duplex
- OR use an Arduino Leonardo/Mega with multiple hardware UARTs

For an ESP32 approach (like hb9eue):

- ESP32 has 3 hardware UARTs
- UART2 can be put into IrDA mode in hardware
- Much simpler and more reliable than Arduino

But the user specified Arduino Uno or Nano. Let me think about this...

Actually, wait. The user's constraint was "Arduino Uno or Nano, standard 38kHz IR LED (TSAL6400 or equivalent), TSOP1738
IR receiver". Now that we know the protocol is IrDA SIR, not 38kHz remote control, the user's components are wrong. They
need:

- An IrDA transceiver chip (TFDU4101/TFDU4108/TFMS5360)
- NOT a TSOP1738

But can we make it work with discrete components (IR LED + photodiode) instead of an IrDA transceiver chip?

IrDA SIR at 115200 baud:

- The modulation scheme at 115200 baud: The bit period is 8.68µs. In IrDA SIR, a '0' bit is represented by a 3/16 pulse
  of the bit period at 9600 baud. But at higher baud rates (up to 115200), the standard still uses the same pulse width
  as at 9600 baud - a fixed 3/16 × 1/9600 = ~19.53µs pulse. Wait, that doesn't sound right.

Actually, let me re-read the IrDA SIR specification more carefully. The IrDA SIR physical layer defines:

- Data rate: 9600 bps (standard), up to 115.2 kbps (extended)
- Modulation: Run-length limited coding
- A '0' bit is transmitted as a pulse of light
- A '1' bit is no light (space)
- At 9600 baud: the pulse width is 3/16 of the bit period (3/16 × 104.17µs = 19.53µs)
- At 115200 baud: the pulse width is still 3/16 of the 9600 baud bit period = 19.53µs

Wait, that's the key insight! In IrDA SIR, the optical pulse width is ALWAYS 3/16 of the 9600 baud bit period,
regardless of the actual data rate. This means:

- At 9600 baud: pulse = 19.53µs, bit period = 104.17µs (18.75% duty cycle)
- At 115200 baud: pulse = 19.53µs, bit period = 8.68µs (225% duty cycle?!)

That can't be right. Let me reconsider.

Actually, I think the IrDA SIR specification at higher baud rates uses a different approach. At 115200 baud, the
modulation is different from 9600 baud. Let me think...

In the standard IrDA SIR implementation:

- The serial data is sent as standard UART frames (start + 8 data + stop)
- A '0' bit causes the IR LED to pulse
- A '1' bit means no IR
- The pulse width at 9600 baud is 3/16 of the bit period

For higher baud rates (up to 115200), the standard uses the same UART framing but the pulse width is adjusted. Some
implementations use a fixed pulse width of ~1.6µs (which is 3/16 of the 9600 baud bit period divided by 12, since
115200/9600 = 12).

Actually, I think the confusion is that there are different IrDA SIR implementations:

1. **9600 baud SIR**: The standard basic rate
2. **Extended SIR (up to 115200 baud)**: Uses the same optical encoding but at higher rates

The key point from the sources is that hb9eue used the ESP32's hardware IrDA mode at 115200 baud and it worked with the
Donatello. The ESP32's IrDA hardware handles the modulation automatically.

For an Arduino, we'd need to replicate this modulation. But it's complex.

The simplest approach, as demonstrated by hb9eue and Daniel Samarin, is to use an **IrDA transceiver chip** that handles
the physical layer modulation. The transceiver connects to standard UART TX/RX pins.

So my revised plan should recommend:

**Option A: Arduino + IrDA transceiver (TFDU4101 or similar)**

- IrDA transceiver chip connected to a UART
- Arduino bridges between USB Serial and IrDA UART
- Most reliable approach

**Option B: ESP32 + IrDA transceiver (like hb9eue's approach)**

- ESP32's UART2 in hardware IrDA mode
- USB Serial for host connection
- Simplest approach, proven to work

**Option C: Discrete components (IR LED + photodiode)**

- Custom modulation circuit
- Complex, unreliable, NOT recommended

Given that the user has an Arduino Uno/Nano, I should recommend Option A. But I should also strongly suggest considering
an ESP32 instead, as it's simpler and proven.

For the IrDA transceiver, the TFDU4101 is available on breakout boards from various suppliers. Other compatible chips
include:

- TFDU4108
- TFMS5360
- TOIM3232 (IrDA controller + transceiver)

Actually, looking at hb9eue's code more carefully, he uses:

- ESP32 with UART2 in IrDA mode
- TFDU4101 IrDA transceiver
- The ESP32's hardware UART IrDA mode handles the modulation

For an Arduino, we'd need:

1. An IrDA transceiver (TFDU4101 or similar)
2. A second UART (SoftwareSerial or hardware UART on Leonardo/Mega)
3. Bridge code

The TFDU4101 connects to UART TX/RX directly:

- TFDU4101 TXD ← Arduino UART TX
- TFDU4101 RXD → Arduino UART RX
- TFDU4101 VCC = 3.3V or 5V (depending on version)
- TFDU4101 GND

Wait, but the TFDU4101 is an IrDA transceiver with an integrated LED and photodiode. It handles the optical modulation
internally. The UART side is standard logic levels.

So the wiring is:

```
Arduino TX → TFDU4101 TXD
Arduino RX ← TFDU4101 RXD
5V → TFDU4101 VCC (if 5V tolerant) or 3.3V
GND → TFDU4101 GND
```

And the TFDU4101's IR LED/photodiode faces the Donatello's optical port.

This is much simpler than my previous analysis suggested!

Now, for the Arduino Uno/Nano, the challenge is having two UARTs:

1. Hardware UART (pins 0/1) for USB (via FTDI/CH340)
2. Second UART for IrDA link

Options:

- **SoftwareSerial**: Can work at 115200 for half-duplex (which the Cressi protocol is)
- **Arduino Leonardo/Mega**: Has multiple hardware UARTs

For SoftwareSerial at 115200 on a 16MHz AVR:

- TX: Works reliably (uses delayMicroseconds)
- RX: Uses pin change interrupt, timing is tight but feasible for half-duplex

Since the Cressi protocol is half-duplex (command → response), we never need to TX and RX simultaneously on the IrDA
link. This makes SoftwareSerial viable.

But wait, there's the echo issue hb9eue mentioned. The TFDU4101 echoes transmitted bytes on the RX pin. We need to
handle this in software (discard the echo).

Let me also reconsider: do we even need an Arduino at all? If we just use an FTDI/CH340 USB-to-serial adapter connected
to a TFDU4101, that's a complete solution! The host PC sees a serial port, sends data to the FTDI, which sends it to the
TFDU4101, which transmits IR to the Donatello. Responses come back the same way.

This is exactly what Daniel Samarin's device does: MCP2221A (USB-to-serial) + IrDA transceiver.

So the simplest solution is:

1. USB-to-serial adapter (FTDI, CH340, CP2102, etc.)
2. IrDA transceiver (TFDU4101 or similar)
3. Connect TX↔TXD, RX↔RXD, VCC, GND

No Arduino needed! Just two chips.

But the user asked for an Arduino-based solution. Let me provide both options:

1. Simplest: FTDI/CH340 + TFDU4101 (no Arduino)
2. Arduino-based: Arduino + TFDU4101 (adds flexibility but more complex)

Actually, re-reading the user's request: "building an Arduino-based IR interface to replace the Cressi BT Interface
dock". They want an Arduino solution. But I should point out the simpler option.

Let me also address the TSOP1738/TSAL6400 issue. These components are NOT suitable for IrDA SIR:

- TSOP1738: Designed for 38kHz remote control, not IrDA
- TSAL6400: A generic IR LED, not an IrDA transmitter

The user needs an IrDA transceiver chip like the TFDU4101.

OK, let me now write the revised plan. I'll:

1. Correct my previous analysis based on the new information
2. Explain that the protocol is IrDA SIR, not 38kHz remote control
3. Recommend the correct components
4. Provide the Arduino sketch for the IrDA transceiver approach
5. Also mention the simpler no-Arduino option

Let me also think about whether we can make the TSOP1738 work somehow...

The IrDA SIR modulation at 115200 baud uses very short pulses (~1.6µs). The TSOP1738 can't detect these. So no, the
TSOP1738 won't work.

What about using the TSOP1738 at 9600 baud? At 9600 baud, the IrDA pulse width is ~19.5µs, which is still below the
TSOP1738's minimum burst of 10 carrier cycles (263µs). So even at 9600 baud, the TSOP1738 won't work for IrDA.

The fundamental issue is that IrDA uses very short pulses (microseconds), while TSOP-style receivers are designed for
longer bursts (milliseconds) used in remote control protocols.

OK, revised plan coming up. Let me be clear and concise.

One more thing: the user said "I want only USB connection with libdivecomputer, not BT". So they want USB to the host,
and IR to the Donatello. This is exactly what the Cressi BT Interface does (without the Bluetooth part).

Let me also think about whether there's a way to build an IrDA transceiver from discrete components (IR LED +
photodiode + amplifier + modulator). This would avoid needing a specific IrDA transceiver chip.

An IrDA SIR transceiver needs:

- TX: IR LED (940nm) with driver circuit, pulsed at the correct width
- RX: Photodiode + amplifier + demodulator + comparator
- The demodulator needs to detect pulses as short as 1.6µs at 115200 baud

Building this from discrete components is possible but complex. The TFDU4101 integrates all of this into a single 4-pin
package (VCC, GND, TXD, RXD). It's much simpler and more reliable.

The TFDU4101 costs ~$3-5 and is available from Mouser, Digikey, etc. Some sellers have breakout boards.

OK, let me write the final plan now.

Actually, I want to also address one more thing from the sources. The hb9eue code has this interesting line:

```cpp
WRITE_PERI_REG( 0x3FF6E020 , READ_PERI_REG(0x3FF6E020) | (1<<16 ) | (1<<9 ) );  //UART_IRDA_EN + UART_IRDA_DPLX  "Let there be light"
```

This enables the ESP32's hardware IrDA mode on UART2. The ESP32's UART has built-in IrDA modulation/demodulation. When
IrDA mode is enabled, the UART automatically:

- TX: Modulates the serial data with IrDA pulses
- RX: Demodulates IrDA pulses back to serial data

This is why hb9eue's solution works - the hardware handles everything.

For an Arduino (ATmega328P), the UART does NOT have built-in IrDA support. So we need an external IrDA transceiver chip.

For an ESP32, we can use the built-in IrDA mode with just an IR LED and photodiode (no transceiver chip needed!). But
the user wants Arduino.

OK, final plan:

**For Arduino Uno/Nano:**

1. Need an IrDA transceiver chip (TFDU4101, TFDU4108, TFMS5360)
2. Connect to SoftwareSerial or hardware UART
3. Bridge between USB and IrDA

**For ESP32 (recommended alternative):**

1. Use hardware UART IrDA mode
2. Just need an IR LED and photodiode (or TFDU4101)
3. Simpler, proven to work

Let me present this clearly to the user.

Hmm, actually I want to reconsider one more time. The user asked me to design an Arduino-based solution. They have
specific components (TSAL6400, TSOP1738). Now I'm telling them these won't work and they need different components. I
should be clear about this but also offer alternatives.

Let me structure the response as:

1. What I learned from the sources (key revelations)
2. Corrected protocol analysis
3. Component assessment (TSOP1738/TSAL6400 won't work)
4. Recommended approach (IrDA transceiver)
5. Questions for the user

I should ask:

- Do they want to buy an IrDA transceiver chip?
- Would they consider an ESP32 instead of Arduino?
- Do they have a USB-to-serial adapter (FTDI/CH340)?

Let me write this up now.

This completely changes the picture. Let me re-assess.

## Key Revelations from These Sources

**The Cressi Donatello uses IrDA SIR** — not 38kHz remote control modulation, not baseband IR. This is confirmed by:

1. **hb9eue** successfully connected to the Donatello using a **TFDU4101 IrDA transceiver** + ESP32 with hardware UART
   IrDA mode at 115200 baud
2. **Linus Torvalds** confirmed: *"the Cressi dock is really just IrDA SIR... which is literally just half duplex rs232
   over infrared"*
3. **Daniel Samarin** built a working USB-to-IrDA adapter using MCP2221A (USB-serial) + IrDA transceiver chip, tested
   successfully with Cressi dive computers across Linux/Windows/Mac
4. The Cressi BT Interface dock is internally an **IrDA transceiver + Bluetooth bridge** — the IrDA chip handles all
   optical modulation

## Corrected Protocol Analysis

IrDA SIR is fundamentally different from 38kHz TV remote IR:

| Aspect          | IrDA SIR (what Cressi uses)            | TSOP1738 remote control    |
|-----------------|----------------------------------------|----------------------------|
| Carrier         | Modulated pulse at 3/16 bit period     | 38kHz continuous carrier   |
| Min pulse width | ~1.6µs at 115200 baud                  | ~263µs (10 carrier cycles) |
| Components      | IrDA transceiver chip (TFDU4101, etc.) | TSOP1738 module            |
| Protocol        | Standard UART framing over IR          | Burst-based coding         |

## Critical Problem: Your Components Won't Work

**TSOP1738**: Cannot demodulate IrDA pulses. Minimum burst is 263µs, but IrDA SIR pulses at 115200 baud are ~1.6µs. *
*Completely incompatible.**

**TSAL6400 IR LED**: Could physically emit IR light, but without proper IrDA pulse shaping and drive circuitry, it won't
produce the correct optical signal. An IrDA transceiver chip (TFDU4101) integrates the LED, photodiode, AGC, and
modulation/demodulation in one package.

## Revised Recommended Approach

### Option A (Simplest — no Arduino needed)

Exactly what Daniel Samarin built:

```
┌──────────────┐      USB       ┌──────────────┐    IrDA SIR    ┌────────────┐
│  Host PC     │◄──────────────►│  FTDI/CH340  │◄──────────────►│  TFDU4101  │◄──► Donatello
│ (Subsurface) │   /dev/ttyUSB0 │  USB-Serial  │ TX→TXD, RX←RXD │ IrDA XCVR  │
└──────────────┘                └──────────────┘                └────────────┘
```

- **Components**: FTDI FT232RL (or CH340G) USB-to-serial adapter + TFDU4101 IrDA transceiver
- **Wiring**: 4 wires — TX→TXD, RX←RXD, VCC, GND
- **No microcontroller needed** — it's a passive level-converter/bridge
- **Cost**: ~$5-8 total
- **This IS what the Cressi BT Interface does internally** (minus Bluetooth)

### Option B (Arduino Uno/Nano + IrDA transceiver)

If you want Arduino-based for future flexibility:

```
┌──────────────┐   USB Serial    ┌────────────┐  SoftwareSerial  ┌────────────┐    IrDA    ┌────────────┐
│  Host PC     │◄───────────────►│  Arduino   │◄────────────────►│  TFDU4101  │◄──────────►│  Donatello │
│              │  115200 8N1     │  Uno/Nano  │  115200 8N1      │  IrDA XCVR │            │            │
└──────────────┘  pins 0/1       │            │  pins 10/11      └────────────┘            └────────────┘
                                 └────────────┘
```

- Hardware UART (pins 0/1) = USB to host
- SoftwareSerial (pins 10/11) = IrDA link via TFDU4101
- Half-duplex bridging code (the Cressi protocol is command-response)
- Must handle TFDU4101 echo (hb9eue reported this: TX data appears on RX)

### Option C (ESP32 — proven, recommended by hb9eue)

```
┌──────────────┐   USB Serial    ┌────────────┐  UART2 IrDA mode  ┌────────────┐
│  Host PC     │◄───────────────►│   ESP32    │◄─────────────────►│  TFDU4101  │◄──► Donatello
│              │                 │            │ HW IrDA mod/demod │            │
└──────────────┘                 └────────────┘                   └────────────┘
```

- ESP32 UART2 has **hardware IrDA mode** — the chip handles modulation/demodulation automatically
- Just enable IrDA mode with one register write: `WRITE_PERI_REG(0x3FF6E020, ...)`
- This is exactly hb9eue's working solution
- IrDA transceiver still needed (TFDU4101), OR you can use a bare IR LED + photodiode since the ESP32 handles the
  modulation

## Questions

1. **Do you want to buy an IrDA transceiver chip?** The TFDU4101 (~$3-5 from Mouser/Digikey) is essential for all
   Arduino-based approaches. Some sellers have pre-made breakout boards. There is no practical way to build a working
   IrDA SIR interface from a TSOP1738 + TSAL6400.

2. **Would you consider an ESP32 instead of Arduino?** hb9eue's code is 80 lines, proven to work, and the ESP32 costs ~$
   5-8 (NodeMCU/ESP32-DevKit). The Arduino approach with SoftwareSerial at 115200 is feasible but tighter on timing.

3. **Or would you prefer the simplest Option A** — just a FTDI USB-serial adapter + TFDU4101, no microcontroller at all?
   This is the most reliable and cheapest solution.

4. **Which Cressi firmware version** does your Donatello run? The libdivecomputer driver has different logbook formats
   for v4+ firmware (firmware ≥ 200). This doesn't affect the physical layer but could matter for testing.

---

