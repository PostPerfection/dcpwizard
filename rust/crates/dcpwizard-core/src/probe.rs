pub use postkit::probe::*;

use std::path::Path;

/// Detect whether the primary video stream carries an alpha channel.
pub fn video_has_alpha(path: &Path) -> Result<bool, String> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=pix_fmt",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .map_err(|error| format!("failed to run ffprobe: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffprobe could not inspect {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let pixel_format = String::from_utf8_lossy(&output.stdout);
    Ok(pixel_format_has_alpha(pixel_format.trim()))
}

fn pixel_format_has_alpha(pixel_format: &str) -> bool {
    pixel_format.starts_with("yuva")
        || pixel_format.starts_with("gbrap")
        || matches!(
            pixel_format,
            "rgba" | "bgra" | "argb" | "abgr" | "ya8" | "ya16le" | "ya16be" | "pal8"
        )
}

#[cfg(test)]
mod tests {
    use super::pixel_format_has_alpha;

    #[test]
    fn detects_alpha_pixel_formats() {
        assert!(pixel_format_has_alpha("yuva444p12le"));
        assert!(pixel_format_has_alpha("gbrap12le"));
        assert!(pixel_format_has_alpha("rgba"));
        assert!(pixel_format_has_alpha("pal8"));
        assert!(!pixel_format_has_alpha("yuv444p12le"));
        assert!(!pixel_format_has_alpha("gbrp12le"));
    }
}
