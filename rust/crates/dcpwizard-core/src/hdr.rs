//! DCI HDR Addendum (v1.2.1) signaling.
//!
//! The addendum (s7) requires the picture MXF's Generic Picture Essence
//! Descriptor to carry TransferCharacteristic = ST 2084 (the UL below) plus a CPL
//! ExtensionMetadata EOTF="ST 2084" claim. The cargo-resolved asdcplib-rs jp2k
//! writer (rev 66de9d0) exposes no way to set TransferCharacteristic, so dcpwizard
//! cannot honestly author a DCI HDR DCP: emitting the CPL HDR claim over essence
//! that lacks the descriptor UL would mislabel the package. Following the
//! imfwizard precedent (HDR deliberately skipped), --hdr-dci fails loud rather
//! than mislabel. The one honest, spec-backed constraint is enforced: the raised
//! per-codestream byte cap.

/// ST 2084 (PQ) TransferCharacteristic UL (DCI HDR Addendum s7; asdcplib
/// TransferCharacteristic_SMPTEST2084). Documented for the fail-loud message; it
/// is not emittable through the current jp2k writer.
pub const ST2084_TRANSFER_UL: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x01, 0x01, 0x01, 0x0a, 0x00, 0x00,
];

/// DCI HDR Addendum monoscopic per-codestream byte cap: floor(56,250,000 / R)
/// bytes per frame at edit rate R fps (= 450 Mbit/s). Stereoscopic halves it.
pub fn hdr_codestream_byte_cap(edit_rate: u32) -> u64 {
    56_250_000 / edit_rate.max(1) as u64
}

/// The DCI HDR bitrate ceiling in Mbit/s (constant 450 across edit rates: the
/// per-frame byte cap times fps times 8 bits is always 450 Mbit/s).
pub const HDR_MAX_MBPS: u32 = 450;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_cap_is_floor_of_450mbit() {
        assert_eq!(hdr_codestream_byte_cap(24), 2_343_750);
        assert_eq!(hdr_codestream_byte_cap(25), 2_250_000);
        assert_eq!(hdr_codestream_byte_cap(48), 1_171_875);
        // cap * fps * 8 bits stays at the 450 Mbit/s ceiling
        assert_eq!(hdr_codestream_byte_cap(24) * 24 * 8 / 1_000_000, 450);
    }
}
