// IrDA SIR (Serial Infrared) async framing layer.
//
// Wraps raw payload bytes into SIR frames for transmission, and unwraps
// received SIR-framed bytes back into payloads.
//
// CRC: Reflected CRC-CCITT (polynomial 0x8408 — bit-reversal of 0x1021,
// init 0xFFFF, no final XOR).  This matches the Linux kernel's
// `crc_ccitt_byte()` in `lib/crc-ccitt.c` / `include/linux/crc-ccitt.h`.

use tracing::{debug, trace, warn};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Beginning of Frame marker.
const BOF: u8 = 0xC0;
/// End of Frame marker.
const EOF: u8 = 0xC1;
/// Control Escape byte.
const CE: u8 = 0x7D;
/// Extra BOF (padding / pre-frame idle).
const XBOF: u8 = 0xFF;
/// XOR mask applied to escaped bytes during transparency processing (bit 5).
const IRDA_TRANS: u8 = 0x20;
/// Initial value for the Frame Check Sequence (CRC).
const INIT_FCS: u16 = 0xFFFF;
/// Value of a correctly-received FCS after processing the full frame
/// (payload + trailing FCS bytes).
const GOOD_FCS: u16 = 0xF0B8;

// ---------------------------------------------------------------------------
// CRC-CCITT reflected  (polynomial 0x8408, table-driven, compile-time)
//
// This is identical to the Linux kernel's lib/crc-ccitt.c which uses the
// reflected polynomial.  The table entry for index 1 is 0x1189.
// ---------------------------------------------------------------------------

/// Build the 256-entry reflected CRC-CCITT lookup table at compile time.
const fn make_crc_ccitt_table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u16;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

/// Pre-computed reflected CRC-CCITT lookup table (matches Linux kernel).
const CRC_CCITT_TABLE: [u16; 256] = make_crc_ccitt_table();

/// Update a running CRC with one byte (same as Linux `crc_ccitt_byte`).
#[inline]
fn irda_fcs(fcs: u16, byte: u8) -> u16 {
    (fcs >> 8) ^ CRC_CCITT_TABLE[((fcs ^ byte as u16) & 0xFF) as usize]
}

/// Compute CRC over an entire byte slice starting from INIT_FCS.
fn crc_ccitt(data: &[u8]) -> u16 {
    let mut fcs = INIT_FCS;
    for &byte in data {
        fcs = irda_fcs(fcs, byte);
    }
    fcs
}

// ---------------------------------------------------------------------------
// Byte-stuffing helper
// ---------------------------------------------------------------------------

/// Apply IrDA byte-stuffing to a single byte.
///
/// If the byte is `BOF` (0xC0), `EOF` (0xC1), or `CE` (0x7D), emit
/// `[CE, byte ^ IRDA_TRANS]`.  Otherwise emit the byte as-is.
fn byte_stuff(byte: u8) -> Vec<u8> {
    match byte {
        BOF | EOF | CE => vec![CE, byte ^ IRDA_TRANS],
        _ => vec![byte],
    }
}

// ===========================================================================
// TX: Wrapping
// ===========================================================================

/// Wrap a raw payload into a complete SIR frame.
///
/// # Arguments
///
/// * `payload` - The raw data bytes to transmit.
/// * `extra_bofs` - Number of extra XBOF (0xFF) bytes to prepend before the
///   frame.  These act as inter-frame padding / pre-frame idle.
///
/// # Output format
///
/// 1. `extra_bofs` copies of `XBOF` (0xFF)
/// 2. `BOF` (0xC0)
/// 3. Byte-stuffed payload bytes
/// 4. FCS (Frame Check Sequence) — `~CRC-CCITT-FALSE(payload)`, byte-stuffed,
///    appended in little-endian order
/// 5. `EOF` (0xC1)
pub fn wrap_frame(payload: &[u8], extra_bofs: usize) -> Vec<u8> {
    let mut frame = Vec::with_capacity(
        extra_bofs + 1 // XBOFs + BOF
        + payload.len() * 2 // worst-case: every payload byte is escaped
        + 2 * 2 // two FCS bytes, worst-case escaped
        + 1, // EOF
    );

    // 1. Extra BOFs
    frame.extend(std::iter::repeat_n(XBOF, extra_bofs));

    // 2. BOF
    frame.push(BOF);

    // 3. Byte-stuff payload
    for &byte in payload {
        frame.extend_from_slice(&byte_stuff(byte));
    }

    // 4. Compute FCS = ~CRC-CCITT(payload), little-endian
    let crc = crc_ccitt(payload);
    let fcs = !crc; // bitwise NOT (one's complement)
    let fcs_lo = (fcs & 0xFF) as u8;
    let fcs_hi = ((fcs >> 8) & 0xFF) as u8;

    frame.extend_from_slice(&byte_stuff(fcs_lo));
    frame.extend_from_slice(&byte_stuff(fcs_hi));

    // 5. EOF
    frame.push(EOF);

    frame
}

// ===========================================================================
// RX: Unwrapping  (state machine matching Linux kernel's wrapper.c)
// ===========================================================================

/// State of the SIR frame unwrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnwrapState {
    /// Waiting for a BOF marker; all other bytes are ignored (except XBOF).
    OutsideFrame,
    /// BOF has been received; waiting for the first data byte.
    BeginFrame,
    /// Actively collecting frame bytes.
    InsideFrame,
    /// Control-Escape received; the next byte must be XORed with IRDA_TRANS.
    LinkEscape,
}

/// Stateful unwrapper that processes a stream of raw SIR bytes and extracts
/// completed, validated payloads.
pub struct SirUnwrapper {
    state: UnwrapState,
    buffer: Vec<u8>,
    fcs: u16,
}

impl SirUnwrapper {
    /// Create a new unwrapper in the initial `OutsideFrame` state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: UnwrapState::OutsideFrame,
            buffer: Vec::new(),
            fcs: INIT_FCS,
        }
    }

    /// Feed one received byte into the state machine.
    ///
    /// Returns `Some(payload)` when a complete valid frame has been received
    /// and its CRC passes.  Corrupt frames are silently discarded (logged at
    /// `warn` level).
    ///
    /// Structure mirrors the kernel's `async_unwrap_char()`: dispatch on the
    /// byte class first, then on the current state.
    #[must_use]
    pub fn process_byte(&mut self, byte: u8) -> Option<Vec<u8>> {
        trace!(byte = format_args!("0x{:02X}", byte), state = ?self.state, "sir_unwrap");

        match byte {
            // --- BOF: Beginning Of Frame ---
            BOF => {
                match self.state {
                    UnwrapState::LinkEscape | UnwrapState::InsideFrame => {
                        warn!("BOF inside frame — discarding incomplete frame");
                    }
                    UnwrapState::OutsideFrame | UnwrapState::BeginFrame => {
                        // Multiple BOFs at start of frame are normal
                    }
                }
                self.state = UnwrapState::BeginFrame;
                self.buffer.clear();
                self.fcs = INIT_FCS;
                None
            }

            // --- EOF: End Of Frame ---
            EOF => {
                match self.state {
                    UnwrapState::OutsideFrame => {
                        warn!("EOF outside frame — missed BOF");
                        None
                    }
                    UnwrapState::BeginFrame
                    | UnwrapState::InsideFrame
                    | UnwrapState::LinkEscape => {
                        // In BeginFrame/LinkEscape, FCS will almost certainly
                        // fail — expected per kernel behaviour.
                        self.state = UnwrapState::OutsideFrame;
                        self.finish_frame()
                    }
                }
            }

            // --- CE: Control Escape ---
            CE => {
                match self.state {
                    UnwrapState::OutsideFrame => {
                        // carrier sense noise, ignore
                    }
                    UnwrapState::LinkEscape => {
                        warn!("double CE — undefined state");
                    }
                    UnwrapState::BeginFrame | UnwrapState::InsideFrame => {
                        self.state = UnwrapState::LinkEscape;
                    }
                }
                None
            }

            // --- Any other byte ---
            other => {
                match self.state {
                    UnwrapState::OutsideFrame => {
                        if other != XBOF {
                            trace!(
                                byte = format_args!("0x{:02X}", other),
                                "noise outside frame"
                            );
                        }
                    }
                    UnwrapState::BeginFrame => {
                        self.buffer.push(other);
                        self.fcs = irda_fcs(self.fcs, other);
                        self.state = UnwrapState::InsideFrame;
                    }
                    UnwrapState::InsideFrame => {
                        self.buffer.push(other);
                        self.fcs = irda_fcs(self.fcs, other);
                    }
                    UnwrapState::LinkEscape => {
                        let unstuffed = other ^ IRDA_TRANS;
                        self.buffer.push(unstuffed);
                        self.fcs = irda_fcs(self.fcs, unstuffed);
                        self.state = UnwrapState::InsideFrame;
                    }
                }
                None
            }
        }
    }

    /// Process a slice of received bytes, returning all completed frame
    /// payloads extracted during processing.
    #[must_use]
    pub fn process_bytes(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        for &byte in data {
            if let Some(payload) = self.process_byte(byte) {
                frames.push(payload);
            }
        }
        frames
    }

    /// Called when EOF is received.  Validates the FCS and returns the
    /// payload (excluding the trailing 2 CRC bytes) if good.
    fn finish_frame(&mut self) -> Option<Vec<u8>> {
        if self.fcs != GOOD_FCS {
            warn!(
                fcs = ?format_args!("0x{:04X}", self.fcs),
                len = self.buffer.len(),
                "CRC error — discarding frame"
            );
            return None;
        }

        let payload_len = self.buffer.len().saturating_sub(2);
        let payload = self.buffer[..payload_len].to_vec();

        debug!(len = payload.len(), "valid frame received");

        Some(payload)
    }
}

impl Default for SirUnwrapper {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // CRC correctness
    // -----------------------------------------------------------------------

    /// Verify reflected CRC-CCITT against the known check value for "123456789".
    /// With polynomial 0x8408, init 0xFFFF, no final XOR, the result is 0x6F91.
    #[test]
    fn crc_ccitt_check_value() {
        let data = b"123456789";
        let expected: u16 = 0x6F91;
        assert_eq!(
            crc_ccitt(data),
            expected,
            "reflected CRC-CCITT for '123456789' must be 0x6F91"
        );
    }

    /// CRC of empty data should be 0xFFFF (the init value, since nothing
    /// shifted it out).
    #[test]
    fn crc_ccitt_empty() {
        assert_eq!(crc_ccitt(b""), INIT_FCS, "CRC of empty data = INIT_FCS");
    }

    /// Verify that CRC table entry [1] matches the Linux kernel's value.
    #[test]
    fn crc_table_spot_check() {
        assert_eq!(CRC_CCITT_TABLE[1], 0x1189, "table[1] must be 0x1189");
    }

    /// Verify the GOOD_FCS residue: CRC over (data + ~CRC(data) as LE) == 0xF0B8.
    #[test]
    fn good_fcs_residue() {
        let data = b"test payload";
        let crc = crc_ccitt(data);
        let fcs = !crc;
        let fcs_lo = (fcs & 0xFF) as u8;
        let fcs_hi = ((fcs >> 8) & 0xFF) as u8;

        let mut full = data.to_vec();
        full.push(fcs_lo);
        full.push(fcs_hi);
        assert_eq!(crc_ccitt(&full), GOOD_FCS, "residue must equal GOOD_FCS");
    }

    // -----------------------------------------------------------------------
    // Byte-stuffing
    // -----------------------------------------------------------------------

    #[test]
    fn byte_stuff_normal_bytes_passthrough() {
        for b in 0x00..=0xFFu8 {
            if b == BOF || b == EOF || b == CE {
                continue;
            }
            assert_eq!(byte_stuff(b), vec![b], "0x{:02X} should pass through", b);
        }
    }

    #[test]
    fn byte_stuff_special_bytes_escaped() {
        assert_eq!(byte_stuff(BOF), vec![CE, BOF ^ IRDA_TRANS]);
        assert_eq!(byte_stuff(EOF), vec![CE, EOF ^ IRDA_TRANS]);
        assert_eq!(byte_stuff(CE), vec![CE, CE ^ IRDA_TRANS]);
    }

    // -----------------------------------------------------------------------
    // Round-trip
    // -----------------------------------------------------------------------

    /// Helper: wrap a payload (with 0 extra BOFs), then feed the frame into
    /// a fresh unwrapper and collect the result.
    fn roundtrip(payload: &[u8]) -> Vec<u8> {
        let frame = wrap_frame(payload, 0);
        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&frame);
        assert_eq!(
            results.len(),
            1,
            "round-trip should produce exactly 1 frame, got {}",
            results.len()
        );
        results.into_iter().next().unwrap()
    }

    #[test]
    fn roundtrip_empty_payload() {
        assert_eq!(roundtrip(b""), b"");
    }

    #[test]
    fn roundtrip_single_byte() {
        assert_eq!(roundtrip(&[0x42]), vec![0x42]);
    }

    #[test]
    fn roundtrip_ascii_hello() {
        let payload = b"hello, irda!";
        assert_eq!(roundtrip(payload), payload);
    }

    #[test]
    fn roundtrip_all_special_bytes() {
        // Payload containing every special byte — they must survive
        // byte-stuffing and un-stuffing.
        let payload: Vec<u8> = vec![BOF, EOF, CE, BOF, CE, EOF];
        assert_eq!(roundtrip(&payload), payload);
    }

    #[test]
    fn roundtrip_mixed_special_and_normal() {
        let payload: Vec<u8> = (0..=255u8).collect();
        assert_eq!(roundtrip(&payload), payload);
    }

    #[test]
    fn roundtrip_large_payload() {
        let payload: Vec<u8> = (0..2048).map(|i| (i % 256) as u8).collect();
        assert_eq!(roundtrip(&payload), payload);
    }

    #[test]
    fn roundtrip_all_zeroes() {
        let payload = vec![0u8; 512];
        assert_eq!(roundtrip(&payload), payload);
    }

    // -----------------------------------------------------------------------
    // extra_bofs
    // -----------------------------------------------------------------------

    #[test]
    fn extra_bofs_stripped() {
        let payload = b"data";
        let frame = wrap_frame(payload, 3);
        // First 3 bytes must be XBOF
        assert_eq!(&frame[0..3], &[XBOF, XBOF, XBOF]);
        assert_eq!(frame[3], BOF);
        // Unwrapper should ignore them
        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&frame);
        assert_eq!(results.len(), 1);
        assert_eq!(&results[0], payload);
    }

    // -----------------------------------------------------------------------
    // CRC error detection
    // -----------------------------------------------------------------------

    #[test]
    fn corrupt_frame_rejected() {
        let payload = b"important data";
        let mut frame = wrap_frame(payload, 0);

        // Flip a bit in the frame body (after BOF, before the last non-EOF).
        // Find the first non-BOF, non-XBOF byte and flip its LSB.
        if let Some(pos) = frame.iter().position(|&b| b != BOF && b != XBOF && b != CE) {
            frame[pos] ^= 1;
        } else if frame.len() > 2 {
            // Fallback: flip byte at position 1 (after BOF)
            frame[1] ^= 1;
        }

        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&frame);
        assert!(
            results.is_empty(),
            "corrupt frame must be rejected (got {} frames)",
            results.len()
        );
    }

    #[test]
    fn truncated_frame_rejected() {
        let payload = b"truncated";
        let frame = wrap_frame(payload, 0);
        // Drop the last 3 bytes (EOF + maybe part of FCS)
        let truncated = &frame[..frame.len().saturating_sub(3)];
        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(truncated);
        assert!(
            results.is_empty(),
            "truncated frame must not produce output"
        );
    }

    // -----------------------------------------------------------------------
    // Multiple frames in a stream
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_frames_in_stream() {
        let payloads: [&[u8]; 3] = [b"first", b"second", b"third"];
        let mut stream = Vec::new();
        for p in &payloads {
            stream.extend_from_slice(&wrap_frame(p, 0));
        }

        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&stream);
        assert_eq!(results.len(), 3);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result, payloads[i], "frame {} mismatch", i);
        }
    }

    #[test]
    fn multiple_frames_with_extra_bofs() {
        let payloads: [&[u8]; 2] = [b"alpha", b"beta"];
        let mut stream = Vec::new();
        stream.extend_from_slice(&wrap_frame(payloads[0], 2));
        stream.extend_from_slice(&wrap_frame(payloads[1], 5));

        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&stream);
        assert_eq!(results.len(), 2);
        assert_eq!(&results[0], payloads[0]);
        assert_eq!(&results[1], payloads[1]);
    }

    // -----------------------------------------------------------------------
    // Garbage / noise between frames — resilience
    // -----------------------------------------------------------------------

    #[test]
    fn garbage_between_frames() {
        let frame_a = wrap_frame(b"frame-A", 0);
        let frame_b = wrap_frame(b"frame-B", 0);

        let mut stream = Vec::new();
        stream.extend_from_slice(&frame_a);
        // Inject garbage bytes (not XBOF, BOF, EOF, CE — just noise)
        stream.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);
        stream.extend_from_slice(&frame_b);

        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&stream);
        assert_eq!(
            results.len(),
            2,
            "garbage should not prevent frame extraction"
        );
        assert_eq!(&results[0], b"frame-A");
        assert_eq!(&results[1], b"frame-B");
    }

    #[test]
    fn xbof_between_frames_ignored() {
        let frame_a = wrap_frame(b"A", 0);
        let frame_b = wrap_frame(b"B", 0);

        let mut stream = Vec::new();
        stream.extend_from_slice(&frame_a);
        stream.extend_from_slice(&[XBOF; 10]);
        stream.extend_from_slice(&frame_b);

        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&stream);
        assert_eq!(results.len(), 2);
        assert_eq!(&results[0], b"A");
        assert_eq!(&results[1], b"B");
    }

    // -----------------------------------------------------------------------
    // State machine edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn consecutive_bof_resets() {
        // Multiple BOFs before any data should be harmless.
        let mut unwrapper = SirUnwrapper::new();
        let _ = unwrapper.process_byte(BOF);
        let _ = unwrapper.process_byte(BOF);
        let _ = unwrapper.process_byte(BOF);
        // Now send a minimal frame: BOF data EOF
        let frame = wrap_frame(b"ok", 0);
        // Skip the leading BOF since we're already past it
        let results = unwrapper.process_bytes(&frame[1..]);
        // Multiple BOFs are harmless; the last one starts the frame.
        assert_eq!(results.len(), 1);
        assert_eq!(&results[0], b"ok");
    }

    #[test]
    fn bof_inside_frame_discards() {
        let payload = b"partial";
        let mut frame = wrap_frame(payload, 0);
        let eof_pos = frame.iter().rposition(|&b| b == EOF).unwrap();

        // Insert a spurious BOF before the real EOF
        frame.insert(eof_pos, BOF);

        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&frame);
        // The spurious BOF resets the frame, so the incomplete first portion
        // is discarded.  Whatever follows may or may not form a valid frame.
        // We care that the original payload is NOT in the output.
        for r in &results {
            assert_ne!(
                r, payload,
                "original payload should be discarded after BOF reset"
            );
        }
    }

    #[test]
    fn empty_frame_body() {
        // A frame with no payload bytes: BOF, FCS bytes, EOF.
        // The FCS bytes are computed over empty data → FCS = ~0xFFFF = 0x0000.
        let mut frame = vec![BOF];
        let fcs: u16 = !crc_ccitt(b"");
        frame.extend_from_slice(&byte_stuff((fcs & 0xFF) as u8));
        frame.extend_from_slice(&byte_stuff(((fcs >> 8) & 0xFF) as u8));
        frame.push(EOF);

        let mut unwrapper = SirUnwrapper::new();
        let results = unwrapper.process_bytes(&frame);
        // Buffer will have 2 FCS bytes, FCS check passes → payload_len 0.
        assert_eq!(results.len(), 1);
        assert!(results[0].is_empty());
    }

    // -----------------------------------------------------------------------
    // process_byte returns None until EOF
    // -----------------------------------------------------------------------

    #[test]
    fn process_byte_returns_none_until_eof() {
        let mut unwrapper = SirUnwrapper::new();
        let frame = wrap_frame(b"test", 0);
        for &byte in &frame[..frame.len() - 1] {
            assert!(
                unwrapper.process_byte(byte).is_none(),
                "process_byte should return None until EOF"
            );
        }
        // Last byte (EOF) should complete the frame
        let result = unwrapper.process_byte(frame[frame.len() - 1]);
        assert!(result.is_some(), "EOF should yield a completed frame");
        assert_eq!(&result.unwrap(), b"test");
    }
}
