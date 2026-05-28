//! GPU-accelerated JPEG 2000 encoding via the Grok library.
//!
//! Delegates to [`postkit::grok`] for TIFF loading and `grk_compress` invocation.

pub use postkit::grok::{
    TiffFrame, compress_file_subprocess, find_grk_compress, grok_lib_path, load_tiff,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_grk_compress_returns_option() {
        // May or may not be installed; just verify it doesn't panic
        let _ = find_grk_compress();
    }

    #[test]
    fn test_grok_lib_path_returns_string() {
        let p = grok_lib_path();
        // Returns empty string if grok not installed
        assert!(p.is_empty() || std::path::Path::new(&p).exists());
    }

    #[test]
    fn test_load_tiff_missing_file() {
        let result = load_tiff(std::path::Path::new("/nonexistent.tif"));
        assert!(result.is_err());
    }
}
