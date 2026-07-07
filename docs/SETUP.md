# Setup & Environment

## Dev Machine

- **OS:** Fedora 43
- **Kernel:** `7.0.14-101.fc43.x86_64`
- **Kernel headers:** `/lib/modules/7.0.14-101.fc43.x86_64/build/`

## Hardware

- **Dongle:** Kingsun KS-959, VID=`07d0`, PID=`4959`
- **Target device:** Cressi Donatello dive computer
- **Communication:** 115200 baud, 8N1, no flow control, via IrDA SIR

## Build Commands

```bash
cargo build --release          # builds to ./target/release/ks959-bridge (~2.7MB)
cargo test                     # runs all 45 unit tests (no hardware needed)
cargo fmt --check              # format check
cargo clippy                   # linter (if available)
```

## USB Permissions

Run as `root`, or create a udev rule:

```
# /etc/udev/rules.d/99-kingsun.rules
SUBSYSTEM=="usb", ATTR{idVendor}=="07d0", ATTR{idProduct}=="4959", MODE="0666"
```

Then reload:

```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
```

## Kernel Module (Speed Change)

The dongle defaults to 9600 baud. The Donatello needs 115200. The USB speed change
control transfer is blocked by `usbfs check_ctrlrecip()` — see [STATUS.md](STATUS.md)
for the full story. The solution is a minimal kernel module.

### Build

```bash
cd kmod/
make            # produces ks959_speed.ko
```

### Load

```bash
sudo insmod kmod/ks959_speed.ko baud=115200
```

**Important:** The module can only be used **once per USB plug cycle**. It returns
`-ENODEV` from `probe()` so it doesn't permanently claim the device, but this means
the kernel won't re-probe until the dongle is physically unplugged and replugged.

### Verify

```bash
lsmod | grep ks959    # should show ks959_speed
```

### Unload

```bash
sudo rmmod ks959_speed
```

The speed setting persists in the dongle firmware until power cycle (unplug).

## LD_PRELOAD Shim (for libdivecomputer)

libdivecomputer calls `ioctl(TIOCMBIC)` to clear the RTS line. PTYs don't support
modem control ioctls (they return `ENOTTY`). A tiny `LD_PRELOAD` shim silently
succeeds for these calls:

### Build

```bash
cat > /tmp/pty_modem_shim.c << 'EOF'
#define _GNU_SOURCE
#include <dlfcn.h>
#include <sys/ioctl.h>
#include <errno.h>
#include <stddef.h>

int ioctl(int fd, unsigned long request, ...) {
    static int (*real_ioctl)(int, unsigned long, ...) = NULL;
    if (!real_ioctl)
        real_ioctl = dlsym(RTLD_NEXT, "ioctl");

    __builtin_va_list ap;
    __builtin_va_start(ap, request);
    void *arg = __builtin_va_arg(ap, void *);
    __builtin_va_end(ap);

    int result = real_ioctl(fd, request, arg);

    if (result < 0 && (errno == ENOTTY || errno == EINVAL)) {
        switch (request) {
            case TIOCMBIS:
            case TIOCMBIC:
            case TIOCMSET:
                errno = 0;
                return 0;
            case TIOCMGET:
                errno = 0;
                if (arg) *(int *)arg = 0;
                return 0;
        }
    }
    return result;
}
EOF
gcc -shared -fPIC -o /tmp/pty_modem_shim.so /tmp/pty_modem_shim.c -ldl
```

### Usage

```bash
LD_PRELOAD=/tmp/pty_modem_shim.so dctool ...
```

## Full End-to-End Procedure

```bash
# 1. Build everything
cargo build --release
cd kmod && make && cd ..

# 2. Load kernel module (sets dongle to 115200)
sudo insmod kmod/ks959_speed.ko baud=115200

# 3. Start bridge
sudo ./target/release/ks959-bridge --baud 115200 --skip-speed-change &

# 4. Put Donatello in PC mode (have ~1 minute before hibernation!)

# 5. Run dctool immediately
LD_PRELOAD=/tmp/pty_modem_shim.so \
LD_LIBRARY_PATH=./reference/libdivecomputer/src/.libs \
  ./reference/libdivecomputer/examples/.libs/dctool \
  -v -l cressi.log -f goa -m 4 download -o dives.xml /tmp/cressi-irda

# 6. Clean up
sudo rmmod ks959_speed    # optional, speed persists until unplug
```

---

## VM/USB Passthrough Setup (for kernel IrDA testing)

This was used only for testing the kernel IrDA stack (to confirm the IrLAP dead end).
The production ks959-bridge tool runs on the host directly.

### Environment

- Host: Linux (Fedora/RHEL-based), QEMU/KVM via libvirt
- VM: Ubuntu 18.04, kernel 4.15 (last version with staging IrDA drivers)

### USB Passthrough

```bash
# Find device on host
lsusb | grep 07d0

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
ssh root@<vm-ip> "modprobe ircomm_tty"
ssh <vm-ip> "lsmod | grep irda"           # Expected: irda, ks959_sir, ircomm, ircomm_tty
ssh <vm-ip> "ls -la /dev/ircomm0"          # Should exist
ssh root@<vm-ip> "chmod 666 /dev/ircomm0 && usermod -aG dialout vincent"
```

### Monitoring Tools (in VM)

```bash
ssh root@<vm-ip> "timeout 30 irdadump -i irda0"   # Raw IrDA frames
ssh <vm-ip> "cat /proc/net/irda/discovery"          # Discovery log
ssh <vm-ip> "ifconfig irda0"                         # Interface stats
```

### USB Disconnect Issue

The KS-959 can disconnect from the VM under load. Fix: re-attach with `virsh attach-device`.
Also ensure the VM's USB controller is EHCI/UHCI (USB 2), not XHCI (USB 3) — low-speed USB
devices passed through XHCI in QEMU/KVM are known to be unstable.

---

## Alternative Hardware Approaches

If ks959-bridge doesn't work for your setup, these alternatives exist:

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

**Known issues:**

- Echo on TFDU4101 RX: transmitted bytes appear on RX pin. Filter: `if (irdaByte == serialByte) discard;`
- DTR/RTS toggling resets ESP32 via auto-reset circuit (not an issue for Donatello, which only clears RTS=0)

### USB-to-Serial + TFDU4101 (Simplest Hardware — Proven)

No microcontroller — just a USB serial adapter and an IrDA transceiver. **Proven by Daniel Samarin.**
Architecturally identical to the Cressi BT Interface dock.

**Parts:** FTDI FT232RL / CH340G / CP2102 (~$3–5), TFDU4101 (~$3–5).

**Wiring:**

```
FTDI TX  ──→ TFDU4101 TXD
FTDI RX  ←── TFDU4101 RXD
FTDI VCC ──→ TFDU4101 VCC
FTDI GND ──→ TFDU4101 GND
```

**Usage:** `dctool -f goa -m 4 download -o dives.xml /dev/ttyUSB0`

### BLE Transport

The Donatello supports BLE (`DC_TRANSPORT_BLE`). Uses different commands
(`CMD_LOGBOOK_BLE=0x02`, `CMD_DIVE_BLE=0x03`) and Nordic UART Service characteristics.
Avoids the entire IrDA problem but requires a BLE adapter.

```bash
bluetoothctl scan on
dctool scan -t ble
```

## External References

- [Subsurface issue #4147](https://github.com/subsurface/subsurface/issues/4147) — GitHub discussion with hb9eue's ESP32
  solution
- [Subsurface mailing list thread](https://groups.google.com/g/subsurface-divelog/c/ku56SSlCtZU) — Google Groups
  discussion
- [Out-of-tree IrDA kernel modules](https://github.com/cschramm/irda) — DKMS build of the removed IrDA stack

