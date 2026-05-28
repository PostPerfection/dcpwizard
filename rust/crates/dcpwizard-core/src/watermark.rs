//! Forensic watermarking for DCP content.
//!
//! Delegates to [`postkit::watermark`] for embedding and detecting forensic
//! watermarks using internal or external backends (NexGuard, Civolution).

pub use postkit::watermark::{
    WatermarkBackend, WatermarkOptions, WatermarkResult, detect_watermark, embed_watermark,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watermark_backend_default() {
        assert_eq!(WatermarkBackend::default(), WatermarkBackend::Internal);
    }

    #[test]
    fn test_watermark_options_default() {
        let opts = WatermarkOptions::default();
        assert!(opts.operator_id.is_empty());
        assert!(opts.session_id.is_empty());
        assert_eq!(opts.strength, 0.0);
    }

    #[test]
    fn test_embed_watermark_no_frames() {
        let dir = tempfile::tempdir().unwrap();
        let opts = WatermarkOptions {
            input_dir: dir.path().to_path_buf(),
            output_dir: dir.path().join("out"),
            operator_id: "OP1".into(),
            session_id: "S1".into(),
            ..Default::default()
        };
        let result = embed_watermark(&opts);
        assert!(!result.success);
    }
}
