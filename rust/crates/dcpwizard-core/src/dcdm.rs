//! DCDM (Digital Cinema Distribution Master) creation and export.
//!
//! Delegates to [`postkit::dcdm`] for converting source image sequences
//! to X'Y'Z' 12/16-bit TIFF DCDM and exporting back to viewable formats.

pub use postkit::dcdm::{DcdmColourEncoding, DcdmOptions, DcdmResult, create_dcdm, export_dcdm};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dcdm_options_default() {
        let opts = DcdmOptions::default();
        assert_eq!(opts.encoding, DcdmColourEncoding::Xyz12Bit);
        assert_eq!(opts.width, 4096);
        assert_eq!(opts.height, 2160);
        assert_eq!(opts.fps_num, 24);
        assert_eq!(opts.fps_den, 1);
    }

    #[test]
    fn test_create_dcdm_no_source() {
        let dir = tempfile::tempdir().unwrap();
        let opts = DcdmOptions {
            input_dir: dir.path().to_path_buf(),
            output_dir: dir.path().join("dcdm_out"),
            ..Default::default()
        };
        let result = create_dcdm(&opts);
        assert!(!result.success);
    }
}
