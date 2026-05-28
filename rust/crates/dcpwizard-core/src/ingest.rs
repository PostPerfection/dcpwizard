//! Camera media ingest pipeline.
//!
//! Delegates to [`postkit::ingest`] for detecting camera formats (ARRI, RED,
//! Blackmagic, Sony, Canon), scanning media, and transcoding to DCP-ready
//! intermediates.

pub use postkit::ingest::{
    CameraFormat, ClipInfo, IngestOptions, detect_format, ingest, scan_media,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_detect_format_by_extension() {
        assert_eq!(detect_format(Path::new("clip.ari")), CameraFormat::Arriraw);
        assert_eq!(detect_format(Path::new("clip.r3d")), CameraFormat::RedR3d);
        assert_eq!(
            detect_format(Path::new("clip.braw")),
            CameraFormat::BlackmagicBraw
        );
        assert_eq!(detect_format(Path::new("clip.txt")), CameraFormat::Unknown);
    }

    #[test]
    fn test_scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let clips = scan_media(dir.path());
        assert!(clips.is_empty());
    }

    #[test]
    fn test_ingest_options_default() {
        let opts = IngestOptions::default();
        assert_eq!(opts.output_format, "dpx");
        assert_eq!(opts.colour_space, "ACES");
        assert_eq!(opts.debayer_quality, 3);
    }
}
