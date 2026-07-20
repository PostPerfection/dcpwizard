//! Visible burn-in watermark.
//!
//! Burns a plainly visible text mark (the payload) into the video/image using
//! postkit's ffmpeg drawtext burn-in. This is a visible mark, not an invisible
//! or forensic watermark, and carries no recoverable payload.
//!
//! (postkit::watermark's "internal" path is not used: it hardcodes a colon in
//! the drawtext text, which ffmpeg's filter parser rejects. postkit::burnin is
//! the working visible-text path.)

use std::path::PathBuf;

/// Burn `payload` as a visible text overlay into `input`, writing `output`.
/// Requires ffmpeg; returns Err with the ffmpeg error if it is missing or fails.
pub fn embed_watermark(input: PathBuf, output: PathBuf, payload: &str) -> std::io::Result<()> {
    let opts = postkit::burnin::BurninOptions {
        input,
        output,
        subtitle_file: None,
        text: Some(payload.to_string()),
        font_size: 24,
        font_colour: "white".to_string(),
        position: "bottom".to_string(),
    };
    postkit::burnin::burnin(&opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_watermark_missing_input_fails() {
        let dir = tempfile::tempdir().unwrap();
        let result = embed_watermark(
            dir.path().join("nope.mov"),
            dir.path().join("out.mov"),
            "DIST-001",
        );
        assert!(result.is_err());
    }
}
