use std::path::Path;

/// Burn-in text overlay onto video frames using ffmpeg drawtext filter.
///
/// Supports burning timecode, custom text, or subtitle file content
/// directly into the video.
pub fn burnin(
    input_file: &Path,
    output_file: &Path,
    text: &str,
    font_size: u32,
    position_x: u32,
    position_y: u32,
    font_color: &str,
) -> i32 {
    if !input_file.exists() {
        tracing::error!("Input file not found: {}", input_file.display());
        return -1;
    }

    let size = if font_size == 0 { 24 } else { font_size };
    let color = if font_color.is_empty() {
        "white"
    } else {
        font_color
    };

    // Escape special characters for ffmpeg drawtext
    let escaped_text = text
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'");

    let filter = format!(
        "drawtext=text='{}':fontsize={}:fontcolor={}:x={}:y={}:shadowcolor=black:shadowx=2:shadowy=2",
        escaped_text, size, color, position_x, position_y
    );

    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(input_file)
        .arg("-vf")
        .arg(&filter)
        .arg("-c:a")
        .arg("copy")
        .arg(output_file)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("Burned text into {}", output_file.display());
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg burn-in failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            -1
        }
        Err(e) => {
            tracing::error!("Failed to run ffmpeg: {e}");
            -1
        }
    }
}

/// Burn timecode overlay into video frames.
pub fn burnin_timecode(
    input_file: &Path,
    output_file: &Path,
    frame_rate: u32,
    font_size: u32,
) -> i32 {
    if !input_file.exists() {
        tracing::error!("Input file not found: {}", input_file.display());
        return -1;
    }

    let fps = if frame_rate == 0 { 24 } else { frame_rate };
    let size = if font_size == 0 { 36 } else { font_size };

    let filter = format!(
        "drawtext=timecode='00\\:00\\:00\\:00':rate={}:fontsize={}:fontcolor=white:x=(w-text_w)/2:y=h-th-20:shadowcolor=black:shadowx=2:shadowy=2",
        fps, size
    );

    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(input_file)
        .arg("-vf")
        .arg(&filter)
        .arg("-c:a")
        .arg("copy")
        .arg(output_file)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("Burned timecode into {}", output_file.display());
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg timecode burn-in failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            -1
        }
        Err(e) => {
            tracing::error!("Failed to run ffmpeg: {e}");
            -1
        }
    }
}
