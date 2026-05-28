//! Dolby Vision and HDR metadata handling for DCP/IMF workflows.
//!
//! Delegates to [`postkit::dolby_vision`] for HDR detection, metadata injection,
//! and format conversion.

pub use postkit::dolby_vision::{
    DolbyVisionOptions, DolbyVisionProfile, Hdr10Metadata, HdrMetadataOptions, HdrType,
    convert_hdr, detect_hdr_type, inject_dolby_vision, inject_hdr10_metadata, read_hdr10_metadata,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hdr_type_default() {
        assert_eq!(HdrType::default(), HdrType::Sdr);
    }

    #[test]
    fn test_dolby_vision_profile_default() {
        assert_eq!(DolbyVisionProfile::default(), DolbyVisionProfile::Profile81);
    }

    #[test]
    fn test_hdr10_metadata_default() {
        let meta = Hdr10Metadata::default();
        assert_eq!(meta.max_luminance, 0);
        assert_eq!(meta.max_cll, 0);
        assert_eq!(meta.max_fall, 0);
    }

    #[test]
    fn test_detect_hdr_missing_file() {
        let hdr = detect_hdr_type(std::path::Path::new("/nonexistent.mxf"));
        assert_eq!(hdr, HdrType::Sdr);
    }
}
