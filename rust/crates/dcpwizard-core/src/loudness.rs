use serde::{Deserialize, Serialize};
use std::path::Path;

/// EBU R128 loudness measurement result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoudnessResult {
    pub integrated_lufs: f64,
    pub true_peak_dbtp: f64,
    pub loudness_range_lu: f64,
    pub short_term_max_lufs: f64,
}

/// Measure loudness of an audio file using ffmpeg's ebur128 filter.
pub fn measure_loudness(audio_file: &Path) -> Result<LoudnessResult, String> {
    if !audio_file.exists() {
        return Err(format!("Audio file not found: {}", audio_file.display()));
    }

    let result = std::process::Command::new("ffmpeg")
        .arg("-i")
        .arg(audio_file)
        .arg("-af")
        .arg("ebur128=peak=true")
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {e}"))?;

    // ebur128 outputs summary to stderr
    let stderr = String::from_utf8_lossy(&result.stderr);

    let integrated = parse_loudness_value(&stderr, "I:")
        .or_else(|| parse_loudness_value(&stderr, "Integrated loudness:"))
        .unwrap_or(f64::NAN);
    let true_peak = parse_loudness_value(&stderr, "Peak:")
        .or_else(|| parse_loudness_value(&stderr, "True peak:"))
        .unwrap_or(f64::NAN);
    let lra = parse_loudness_value(&stderr, "LRA:")
        .or_else(|| parse_loudness_value(&stderr, "Loudness range:"))
        .unwrap_or(f64::NAN);

    Ok(LoudnessResult {
        integrated_lufs: integrated,
        true_peak_dbtp: true_peak,
        loudness_range_lu: lra,
        short_term_max_lufs: f64::NAN,
    })
}

fn parse_loudness_value(output: &str, key: &str) -> Option<f64> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find(key) {
            let after = &trimmed[pos + key.len()..];
            // Extract the first floating point number after the key
            let num_str: String = after
                .trim()
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            if let Ok(val) = num_str.parse::<f64>() {
                return Some(val);
            }
        }
    }
    None
}
