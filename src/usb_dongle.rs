// Kingsun KS-959 USB-to-IrDA dongle driver.
//
// Implements the proprietary USB control-transfer protocol reverse-engineered
// in the Linux kernel driver `ks959-sir.c`.  All communication uses endpoint 0
// control transfers — the dongle's interrupt endpoint is a dummy.
//
// TX: obfuscate with XOR mask derived from "wangshuofei19710", pad to
//     8-byte alignment + 16 overhead, fragment at 240 bytes.
// RX: poll via control-IN, de-obfuscate with a persistent wrapping counter
//     XOR'd with 0x55, skip the garbage byte every 255 real bytes.
// Speed: 8-byte packed struct via control-OUT to wIndex=0x0001.

use thiserror::Error;
use tracing::{debug, info, trace};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// USB Vendor ID for Kingsun.
const VENDOR_ID: u16 = 0x07D0;
/// USB Product ID for the KS-959 dongle.
const PRODUCT_ID: u16 = 0x4959;

/// Control request code used for both TX data and speed change.
const REQ_SEND: u8 = 0x09;
/// Control request code used for RX polling.
const REQ_RECV: u8 = 0x01;

/// Maximum buffer the dongle accepts in one transfer.
const SND_PACKET_SIZE: usize = 256;
/// RX polling buffer size (matches kernel's KINGSUN_RCV_FIFO_SIZE).
const RCV_FIFO_SIZE: u16 = 2048;

/// Maximum cleartext bytes per TX fragment: `(256 & ~7) - 16 = 240`.
const MAX_CLEARTEXT_PER_FRAGMENT: usize = (SND_PACKET_SIZE & !0x07) - 0x10;

/// The obfuscation lookup string — yes, really.
const LOOKUP: &[u8; 16] = b"wangshuofei19710";

/// Supported baud rates.
const SUPPORTED_SPEEDS: &[u32] = &[2400, 9600, 19200, 38400, 57600, 115200];

/// Flag byte for 8 data bits in the speed-change struct.
const KS_DATA_8_BITS: u8 = 0x03;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from Kingsun dongle operations.
#[derive(Debug, Error)]
pub enum DongleError {
    /// No Kingsun KS-959 dongle found on the USB bus.
    #[error("no Kingsun KS-959 dongle found (VID=0x{:04X} PID=0x{:04X})", VENDOR_ID, PRODUCT_ID)]
    NotFound,

    /// USB operation failed.
    #[error("USB error: {0}")]
    Usb(#[from] nusb::Error),

    /// A USB transfer completed with a non-success status.
    #[error("USB transfer error: {0}")]
    Transfer(#[from] nusb::transfer::TransferError),

    /// Requested baud rate is not supported by the dongle.
    #[error("unsupported baud rate: {0} (supported: 2400..115200)")]
    UnsupportedSpeed(u32),
}

// ---------------------------------------------------------------------------
// TX obfuscation
// ---------------------------------------------------------------------------

/// Obfuscate and pad a cleartext buffer for transmission to the dongle.
///
/// Returns the padded, obfuscated buffer ready to be sent as the control
/// transfer payload.  The cleartext length is encoded in `wValue` of the
/// setup packet (handled by the caller).
///
/// Algorithm (from `ks959-sir.c`):
/// - Padded length = `((len + 7) & ~7) + 16`
/// - XOR mask = `LOOKUP[(len & 0x0F) ^ 0x06] ^ 0x55`
/// - Each cleartext byte is XOR'd with the mask
/// - Padding bytes are zero
fn obfuscate_tx_buffer(cleartext: &[u8]) -> Vec<u8> {
    let len = cleartext.len();
    let padded_len = ((len + 7) & !0x07) + 0x10;
    let xor_mask = LOOKUP[(len & 0x0F) ^ 0x06] ^ 0x55;

    let mut out = vec![0u8; padded_len];
    for (i, &b) in cleartext.iter().enumerate() {
        out[i] = b ^ xor_mask;
    }

    trace!(
        cleartext_len = len,
        padded_len,
        xor_mask = format_args!("0x{:02X}", xor_mask),
        "obfuscate_tx"
    );
    out
}

// ---------------------------------------------------------------------------
// RX de-obfuscation
// ---------------------------------------------------------------------------

/// De-obfuscate raw bytes received from the dongle.
///
/// `counter` is the persistent session counter (starts at 0, wraps as `u8`).
/// It is mutated in place and must be preserved across calls.
///
/// For each raw byte:
///   1. Increment counter (wrapping).
///   2. Decoded = raw XOR counter XOR 0x55.
///   3. If counter wrapped to 0: skip this byte (garbage).
///   4. Otherwise: emit decoded byte.
fn deobfuscate_rx_buffer(raw: &[u8], counter: &mut u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    for &b in raw {
        *counter = counter.wrapping_add(1);
        let decoded = b ^ *counter ^ 0x55;
        if *counter != 0 {
            out.push(decoded);
        } else {
            trace!(
                raw_byte = format_args!("0x{:02X}", b),
                decoded = format_args!("0x{:02X}", decoded),
                "skipping garbage byte at counter wrap"
            );
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Speed-change payload
// ---------------------------------------------------------------------------

/// Build the 8-byte speed-change payload.
///
/// Layout (packed, little-endian):
///   `[baudrate_le32, flags, 0, 0, 0]`
/// where `flags = KS_DATA_8_BITS (0x03)`.
fn speed_payload(baud: u32) -> [u8; 8] {
    let le = baud.to_le_bytes();
    [le[0], le[1], le[2], le[3], KS_DATA_8_BITS, 0, 0, 0]
}

// ---------------------------------------------------------------------------
// Dongle struct
// ---------------------------------------------------------------------------

/// Kingsun KS-959 USB-to-IrDA dongle handle.
///
/// All I/O is via async control transfers on endpoint 0.
pub struct KingsunDongle {
    /// Claimed USB interface (interface 0).
    interface: nusb::Interface,
    /// Persistent RX de-obfuscation counter.
    rx_counter: u8,
}

impl KingsunDongle {
    /// Find and open the first Kingsun KS-959 dongle on the USB bus.
    ///
    /// Detaches any kernel driver and claims interface 0.
    pub fn open() -> Result<Self, DongleError> {
        let device_info = nusb::list_devices()?
            .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
            .ok_or(DongleError::NotFound)?;

        info!(
            bus = device_info.bus_number(),
            addr = device_info.device_address(),
            "found Kingsun KS-959 dongle"
        );

        let device = device_info.open()?;

        // Try to detach kernel driver first (may fail if none attached — that's fine).
        let interface = match device.detach_and_claim_interface(0) {
            Ok(iface) => {
                info!("detached kernel driver and claimed interface 0");
                iface
            }
            Err(_) => {
                debug!("no kernel driver to detach, claiming interface 0 directly");
                device.claim_interface(0)?
            }
        };

        Ok(Self {
            interface,
            rx_counter: 0,
        })
    }

    /// Change the IrDA link baud rate.
    ///
    /// Sends a speed-change control transfer to the dongle.
    pub async fn set_speed(&self, baud: u32) -> Result<(), DongleError> {
        if !SUPPORTED_SPEEDS.contains(&baud) {
            return Err(DongleError::UnsupportedSpeed(baud));
        }

        let payload = speed_payload(baud);
        trace!(
            baud,
            payload = format_args!("{:02X?}", payload),
            "speed change control transfer"
        );

        // The kernel driver (ks959-sir.c) uses bRequestType=0x21 (Class +
        // Interface + OUT) with wIndex=0x0001 for speed changes.  But the
        // kernel's usbfs check_ctrlrecip() rejects USB_RECIP_INTERFACE when
        // wIndex doesn't match an existing interface number — and this device
        // only has interface 0.  wIndex=1 is a protocol flag, not an interface
        // number, but the kernel doesn't know that.  The original kernel driver
        // bypasses this because it submits URBs directly, not through usbfs.
        //
        // Workaround: use ControlType::Vendor (bRequestType=0x41 instead of
        // 0x21).  The kernel's check_ctrlrecip has an early return for
        // USB_TYPE_VENDOR that skips all recipient/wIndex validation.  The
        // dongle firmware is expected to dispatch on the recipient bits (0x01 =
        // Interface) and wIndex, ignoring the Class-vs-Vendor distinction in
        // the type field.  Confirmed: Recipient::Device (0x20) was STALL'd,
        // meaning the firmware checks the recipient — but may not check the
        // type.
        let completion = self
            .interface
            .control_out(nusb::transfer::ControlOut {
                control_type: nusb::transfer::ControlType::Vendor,
                recipient: nusb::transfer::Recipient::Interface,
                request: REQ_SEND,
                value: 0x0200,
                index: 0x0001,
                data: &payload,
            })
            .await;
        completion.status?;

        info!(baud, "dongle speed changed");
        Ok(())
    }

    /// Send data to the dongle (TX direction: host → IrDA link).
    ///
    /// Handles obfuscation, padding, and fragmentation. Data larger than
    /// 240 bytes is automatically split into multiple control transfers.
    pub async fn send(&self, data: &[u8]) -> Result<(), DongleError> {
        if data.is_empty() {
            return Ok(());
        }

        for (frag_idx, chunk) in data.chunks(MAX_CLEARTEXT_PER_FRAGMENT).enumerate() {
            let obfuscated = obfuscate_tx_buffer(chunk);

            trace!(
                fragment = frag_idx,
                cleartext_len = chunk.len(),
                padded_len = obfuscated.len(),
                data = format_args!("{:02X?}", &chunk[..chunk.len().min(32)]),
                "TX fragment"
            );

            let completion = self
                .interface
                .control_out(nusb::transfer::ControlOut {
                    control_type: nusb::transfer::ControlType::Class,
                    recipient: nusb::transfer::Recipient::Interface,
                    request: REQ_SEND,
                    value: chunk.len() as u16,
                    index: 0x0000,
                    data: &obfuscated,
                })
                .await;
            completion.status?;
        }

        debug!(total_len = data.len(), "TX complete");
        Ok(())
    }

    /// Poll the dongle for received data (RX direction: IrDA link → host).
    ///
    /// Returns decoded bytes with the obfuscation removed. Returns an empty
    /// `Vec` if no data is available. The internal de-obfuscation counter
    /// persists across calls for the lifetime of the session.
    pub async fn poll_receive(&mut self) -> Result<Vec<u8>, DongleError> {
        let completion = self
            .interface
            .control_in(nusb::transfer::ControlIn {
                control_type: nusb::transfer::ControlType::Class,
                recipient: nusb::transfer::Recipient::Interface,
                request: REQ_RECV,
                value: 0x0200,
                index: 0x0000,
                length: RCV_FIFO_SIZE,
            })
            .await;
        completion.status?;

        let raw = completion.data;
        if raw.is_empty() {
            return Ok(Vec::new());
        }

        trace!(
            raw_len = raw.len(),
            counter_before = format_args!("0x{:02X}", self.rx_counter),
            raw_head = format_args!("{:02X?}", &raw[..raw.len().min(32)]),
            "RX poll"
        );

        let decoded = deobfuscate_rx_buffer(&raw, &mut self.rx_counter);

        if !decoded.is_empty() {
            debug!(
                decoded_len = decoded.len(),
                counter_after = format_args!("0x{:02X}", self.rx_counter),
                "RX decoded"
            );
        }

        Ok(decoded)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- TX obfuscation tests -----------------------------------------------

    /// Hand-calculated obfuscation for a 3-byte cleartext.
    ///
    /// cleartext = [0x01, 0x02, 0x03], len = 3
    /// padded_len = ((3+7) & ~7) + 16 = 8 + 16 = 24
    /// mask_index = (3 & 0x0F) ^ 0x06 = 3 ^ 6 = 5
    /// LOOKUP[5] = b'h' = 0x68
    /// xor_mask  = 0x68 ^ 0x55 = 0x3D
    /// obfuscated[0] = 0x01 ^ 0x3D = 0x3C
    /// obfuscated[1] = 0x02 ^ 0x3D = 0x3F
    /// obfuscated[2] = 0x03 ^ 0x3D = 0x3E
    /// obfuscated[3..24] = 0x00 (padding)
    #[test]
    fn obfuscate_known_vector() {
        let cleartext = [0x01u8, 0x02, 0x03];
        let out = obfuscate_tx_buffer(&cleartext);

        assert_eq!(out.len(), 24);
        assert_eq!(out[0], 0x3C);
        assert_eq!(out[1], 0x3F);
        assert_eq!(out[2], 0x3E);
        // All padding is zero
        for &b in &out[3..] {
            assert_eq!(b, 0x00, "padding byte must be zero");
        }
    }

    /// Verify padded length formula for various cleartext sizes.
    #[test]
    fn obfuscate_padded_length() {
        for len in [1, 2, 7, 8, 9, 16, 100, 200, MAX_CLEARTEXT_PER_FRAGMENT] {
            let data = vec![0xAA; len];
            let out = obfuscate_tx_buffer(&data);
            let expected = ((len + 7) & !0x07) + 0x10;
            assert_eq!(
                out.len(),
                expected,
                "padded_len for cleartext_len={len}"
            );
        }
    }

    /// Padding bytes beyond the cleartext must be zero (not XOR'd).
    #[test]
    fn obfuscate_padding_is_zero() {
        let cleartext = vec![0xFF; 5];
        let out = obfuscate_tx_buffer(&cleartext);
        for (i, &b) in out.iter().enumerate().skip(5) {
            assert_eq!(b, 0x00, "padding byte at index {i} must be zero");
        }
    }

    /// Obfuscation is invertible: XOR with the same mask recovers cleartext.
    #[test]
    fn obfuscate_invertible() {
        let cleartext = b"Hello IrDA dongle!";
        let len = cleartext.len();
        let out = obfuscate_tx_buffer(cleartext);
        let xor_mask = LOOKUP[(len & 0x0F) ^ 0x06] ^ 0x55;
        for (i, &orig) in cleartext.iter().enumerate() {
            assert_eq!(out[i] ^ xor_mask, orig, "byte {i} round-trip failed");
        }
    }

    // -- RX de-obfuscation tests --------------------------------------------

    /// Basic de-obfuscation: encode known cleartext as the dongle would,
    /// then decode and verify.
    #[test]
    fn deobfuscate_basic() {
        let cleartext = b"test";
        let mut counter: u8 = 0;

        // Simulate what the dongle produces: raw = cleartext ^ (counter+1) ^ 0x55
        let mut raw = Vec::new();
        let mut sim_counter: u8 = 0;
        for &b in cleartext.iter() {
            sim_counter = sim_counter.wrapping_add(1);
            raw.push(b ^ sim_counter ^ 0x55);
        }

        let decoded = deobfuscate_rx_buffer(&raw, &mut counter);
        assert_eq!(&decoded, cleartext);
        assert_eq!(counter, 4);
    }

    /// Counter wrap: when counter goes from 0xFF to 0x00, the byte is garbage
    /// and must be skipped.
    #[test]
    fn deobfuscate_counter_wrap() {
        // Start at 0xFD so we process bytes at counters 0xFE, 0xFF, 0x00, 0x01
        let cleartext = [0x41u8, 0x42, 0x99, 0x43]; // 0x99 is arbitrary (will be skipped)
        let mut counter: u8 = 0xFD;

        // Simulate dongle encoding
        let mut raw = Vec::new();
        let mut sim_counter: u8 = 0xFD;
        for &b in &cleartext {
            sim_counter = sim_counter.wrapping_add(1);
            raw.push(b ^ sim_counter ^ 0x55);
        }

        let decoded = deobfuscate_rx_buffer(&raw, &mut counter);
        // Byte at counter=0x00 (third byte) is skipped
        assert_eq!(decoded.len(), 3, "garbage byte must be skipped");
        assert_eq!(decoded[0], 0x41); // counter=0xFE
        assert_eq!(decoded[1], 0x42); // counter=0xFF
        assert_eq!(decoded[2], 0x43); // counter=0x01
        assert_eq!(counter, 0x01);
    }

    /// Counter persists across multiple calls.
    #[test]
    fn deobfuscate_counter_persistence() {
        let mut counter: u8 = 0;

        // First call: 3 bytes → counter ends at 3
        let raw1 = {
            let mut r = Vec::new();
            let mut c: u8 = 0;
            for &b in &[0x10u8, 0x20, 0x30] {
                c = c.wrapping_add(1);
                r.push(b ^ c ^ 0x55);
            }
            r
        };
        let d1 = deobfuscate_rx_buffer(&raw1, &mut counter);
        assert_eq!(counter, 3);
        assert_eq!(d1, vec![0x10, 0x20, 0x30]);

        // Second call: 2 bytes → counter continues from 3, ends at 5
        let raw2 = {
            let mut r = Vec::new();
            let mut c: u8 = 3; // continue from where we left off
            for &b in &[0x40u8, 0x50] {
                c = c.wrapping_add(1);
                r.push(b ^ c ^ 0x55);
            }
            r
        };
        let d2 = deobfuscate_rx_buffer(&raw2, &mut counter);
        assert_eq!(counter, 5);
        assert_eq!(d2, vec![0x40, 0x50]);
    }

    /// Empty raw input produces empty output without touching the counter.
    #[test]
    fn deobfuscate_empty() {
        let mut counter: u8 = 42;
        let decoded = deobfuscate_rx_buffer(&[], &mut counter);
        assert!(decoded.is_empty());
        assert_eq!(counter, 42);
    }

    // -- Fragmentation tests ------------------------------------------------

    /// Verify that data is split into correct fragment sizes.
    #[test]
    fn fragmentation_sizes() {
        let sizes: Vec<usize> = (0..500usize)
            .collect::<Vec<_>>()
            .chunks(MAX_CLEARTEXT_PER_FRAGMENT)
            .map(|c| c.len())
            .collect();
        assert_eq!(sizes, vec![240, 240, 20]);
    }

    /// A single fragment (≤240) is not split.
    #[test]
    fn fragmentation_single() {
        let sizes: Vec<usize> = (0..240usize)
            .collect::<Vec<_>>()
            .chunks(MAX_CLEARTEXT_PER_FRAGMENT)
            .map(|c| c.len())
            .collect();
        assert_eq!(sizes, vec![240]);
    }

    // -- Speed-change payload tests -----------------------------------------

    /// Verify the 8-byte payload for 115200 baud.
    ///
    /// 115200 = 0x0001C200
    /// LE bytes: [0x00, 0xC2, 0x01, 0x00]
    /// flags: 0x03 (8 data bits)
    /// reserved: [0x00, 0x00, 0x00]
    #[test]
    fn speed_payload_115200() {
        let p = speed_payload(115200);
        assert_eq!(p, [0x00, 0xC2, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00]);
    }

    /// Verify the payload for 9600 baud.
    ///
    /// 9600 = 0x00002580
    /// LE bytes: [0x80, 0x25, 0x00, 0x00]
    #[test]
    fn speed_payload_9600() {
        let p = speed_payload(9600);
        assert_eq!(p, [0x80, 0x25, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00]);
    }

    /// Unsupported speeds should be rejected.
    #[test]
    fn unsupported_speed() {
        assert!(!SUPPORTED_SPEEDS.contains(&1200));
        assert!(!SUPPORTED_SPEEDS.contains(&0));
        assert!(!SUPPORTED_SPEEDS.contains(&230400));
        // The actual error is returned by set_speed() which needs a device,
        // so we just verify the lookup here.
        assert!(SUPPORTED_SPEEDS.contains(&9600));
        assert!(SUPPORTED_SPEEDS.contains(&115200));
    }

    /// MAX_CLEARTEXT_PER_FRAGMENT must be 240.
    #[test]
    fn max_fragment_size() {
        assert_eq!(MAX_CLEARTEXT_PER_FRAGMENT, 240);
    }
}
