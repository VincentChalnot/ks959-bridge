# Testing & Debugging

## Unit Tests (no hardware needed)

```bash
cargo test                              # all 45 tests
cargo test -- --nocapture               # with stdout output
cargo test -p ks959-bridge usb_dongle   # only usb_dongle tests
```

## Debug Logging

```bash
RUST_LOG=info  ./target/release/ks959-bridge   # default (startup, speed changes)
RUST_LOG=debug ./target/release/ks959-bridge   # protocol events (TX/RX, baud changes)
RUST_LOG=trace ./target/release/ks959-bridge   # hex dumps of every USB transfer
```

### What Each Level Shows

| Level   | What's Logged                                                                 |
|---------|-------------------------------------------------------------------------------|
| `info`  | Dongle found, PTY created, speed changes, baud rate changes                   |
| `debug` | PTY→dongle and dongle→PTY data lengths, TX completion, RX decoded lengths     |
| `trace` | Raw USB bytes (hex), obfuscation details, SIR frame state machine transitions |

## Testing with dctool

### The Command

```bash
# Build the LD_PRELOAD shim first (see SETUP.md)
# The kernel module must be loaded and bridge running

LD_PRELOAD=/tmp/pty_modem_shim.so \
LD_LIBRARY_PATH=./reference/libdivecomputer/src/.libs \
  ./reference/libdivecomputer/examples/.libs/dctool \
  -v -l /tmp/cressi.log -f goa -m 4 download -o /tmp/dives.xml /tmp/cressi-irda
```

**Important:** Use `.libs/dctool` (the real binary), NOT the libtool wrapper script.
The wrapper at `./reference/libdivecomputer/examples/dctool` doesn't pass arguments
correctly.

### dctool Flags

| Flag       | Meaning                                              |
|------------|------------------------------------------------------|
| `-v`       | Verbose (shows INFO/ERROR logs from libdivecomputer) |
| `-l FILE`  | Log file for libdivecomputer internal logging        |
| `-f goa`   | Device family: `DC_FAMILY_CRESSI_GOA`                |
| `-m 4`     | Model number: 4 = Donatello                          |
| `-o FILE`  | Output file (XML format, default)                    |
| `download` | Command to download dives                            |

### Family Name Mapping

| String     | Constant                    | Default Model |
|------------|-----------------------------|---------------|
| `goa`      | `DC_FAMILY_CRESSI_GOA`      | 2             |
| `leonardo` | `DC_FAMILY_CRESSI_LEONARDO` | 1             |
| `edy`      | `DC_FAMILY_CRESSI_EDY`      | 0x08          |

### Other dctool Commands

```bash
dctool list                                    # list supported devices
dctool version                                 # show version
dctool help download                           # help for download command
dctool -f goa -m 4 dump -o dump.bin /tmp/cressi-irda   # raw memory dump
dctool parse -i dives.xml                      # parse previously downloaded dives
```

## End-to-End Test Procedure

```bash
# Terminal 1: Start bridge
sudo insmod kmod/ks959_speed.ko baud=115200
sudo ./target/release/ks959-bridge --baud 115200 --skip-speed-change

# Terminal 2: Run dctool (put Donatello in PC mode FIRST, then immediately:)
LD_PRELOAD=/tmp/pty_modem_shim.so \
LD_LIBRARY_PATH=./reference/libdivecomputer/src/.libs \
  ./reference/libdivecomputer/examples/.libs/dctool \
  -v -l /tmp/cressi.log -f goa -m 4 download -o /tmp/dives.xml /tmp/cressi-irda
```

**Timing is critical:** The Donatello hibernates after ~1 minute in PC mode.
Have dctool ready to fire before putting the Donatello in PC mode.

## Analyzing Logs

### Successful CMD_VERSION Exchange

```
# Bridge log (debug level):
PTY → dongle len=8                          # CMD_VERSION sent
TX complete total_len=8                     # USB transfer succeeded
RX decoded decoded_len=19 counter_after=0x14  # Version response decoded
dongle → PTY len=19                         # Forwarded to dctool

# dctool log:
INFO: Write: size=8, data=AAAAAA0000000055   # CMD_VERSION
INFO: Read: size=4, data=AAAAAA0B            # Response header
INFO: Read: size=15, data=211AAD000004310111000500753855  # Response data
Event: model=4 (0x00000004), firmware=305 (0x00000131), serial=44314 (0x0000ad1a)
```

### RX Counter Desync (garbage output)

```
# Bridge log:
RX decoded decoded_len=19 counter_after=0x14  # But counter started at 0x01, not 0x00!

# dctool log:
INFO: Read: size=4, data=8F818724             # NOT a valid header (should be AAAAAA..)
ERROR: Unexpected answer header byte.
```

**Fix:** Ensure stale data drain + counter reset are working. Check that the kernel
module was loaded and unloaded before starting the bridge.

### Speed Change STALL (expected)

```
# Bridge log:
USB speed change STALLED — if the ks959_speed kernel module already set this speed,
  IR comms will still work baud=115200 error=USB transfer error: endpoint STALL condition
```

This is **normal** when using `--skip-speed-change`. The bridge attempts the usbfs
speed change, it STALLs, and the bridge continues assuming the kernel module already
set the speed.

### Stale Byte Drain

```
# Bridge log (debug level):
drained stale data from dongle len=1    # Stale byte found and discarded
resetting RX de-obfuscation counter     # Counter reset to 0
```

## Common Issues

| Symptom                            | Cause                                             | Fix                                                           |
|------------------------------------|---------------------------------------------------|---------------------------------------------------------------|
| `Device or resource busy`          | Another process or kernel module holds the dongle | `rmmod ks959_speed`, kill other bridge instances              |
| `No such file or directory` on PTY | Bridge not running or crashed                     | Restart bridge                                                |
| dctool gets garbage response       | RX counter desync                                 | Check stale byte drain, ensure counter reset on baud change   |
| dctool times out                   | Donatello hibernated                              | Put it back in PC mode, have dctool ready to fire immediately |
| `ENOTTY` on ioctl                  | PTY doesn't support modem control                 | Use `LD_PRELOAD=/tmp/pty_modem_shim.so`                       |
| Speed change STALL                 | Expected with usbfs                               | Use kernel module (`insmod ks959_speed.ko baud=115200`)       |
| Kernel module won't load           | Already loaded or device not found                | `lsmod \| grep ks959`, check `lsusb \| grep 07d0`             |
| Module won't re-probe              | Single-use per plug cycle                         | Unplug and replug the dongle                                  |

## Useful One-Liners

```bash
# Check if dongle is connected
lsusb | grep 07d0:4959

# Check if kernel module is loaded
lsmod | grep ks959

# Check if bridge is running
ps aux | grep ks959-bridge

# Check what's claiming the USB device
ls -la /sys/bus/usb/devices/*/idVendor 2>/dev/null | xargs grep -l "07d0"

# Kill bridge
pkill -f ks959-bridge

# Check PTY symlink
ls -la /tmp/cressi-irda

# Test PTY manually (in another terminal while bridge is running)
minicom -D /tmp/cressi-irda
# or
echo "test" > /tmp/cressi-irda
```
