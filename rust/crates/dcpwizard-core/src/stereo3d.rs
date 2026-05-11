use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Stereoscopic eye.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Eye {
    Left,
    Right,
}

/// Stereo 3D packaging configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stereo3DConfig {
    pub left_dir: PathBuf,
    pub right_dir: PathBuf,
    pub output_mxf: PathBuf,
    pub fps: u32,
}

/// Create a stereo 3D MXF by interleaving left and right eye J2K frames.
///
/// Uses asdcp-wrap with the -3 flag for stereo interleave.
pub fn create_stereo3d(config: &Stereo3DConfig) -> i32 {
    let fps = if config.fps == 0 { 24 } else { config.fps };

    // Verify both directories exist and contain frames
    for (label, dir) in [("left", &config.left_dir), ("right", &config.right_dir)] {
        if !dir.exists() {
            tracing::error!(
                "Stereo3D {label} eye directory not found: {}",
                dir.display()
            );
            return -1;
        }
        let count = count_j2k_frames(dir);
        if count == 0 {
            tracing::error!(
                "No J2K frames found in {label} eye directory: {}",
                dir.display()
            );
            return -1;
        }
        tracing::info!("Found {count} J2K frames in {label} eye directory");
    }

    let left_count = count_j2k_frames(&config.left_dir);
    let right_count = count_j2k_frames(&config.right_dir);
    if left_count != right_count {
        tracing::error!("Frame count mismatch: left={left_count}, right={right_count}");
        return -1;
    }

    // Interleave left/right into a single directory for asdcp-wrap
    let interleave_dir = config.output_mxf.with_extension("stereo_interleave");
    if let Err(e) = std::fs::create_dir_all(&interleave_dir) {
        tracing::error!("Failed to create interleave directory: {e}");
        return -1;
    }

    let mut left_frames = list_j2k_frames(&config.left_dir);
    let mut right_frames = list_j2k_frames(&config.right_dir);
    left_frames.sort();
    right_frames.sort();

    for (i, (left, right)) in left_frames.iter().zip(right_frames.iter()).enumerate() {
        let left_dest = interleave_dir.join(format!("{:08}_L.j2c", i));
        let right_dest = interleave_dir.join(format!("{:08}_R.j2c", i));
        if let Err(e) = std::fs::copy(left, &left_dest) {
            tracing::error!("Failed to copy left frame: {e}");
            let _ = std::fs::remove_dir_all(&interleave_dir);
            return -1;
        }
        if let Err(e) = std::fs::copy(right, &right_dest) {
            tracing::error!("Failed to copy right frame: {e}");
            let _ = std::fs::remove_dir_all(&interleave_dir);
            return -1;
        }
    }

    let result = std::process::Command::new("asdcp-wrap")
        .arg("-3")
        .arg("-p")
        .arg(fps.to_string())
        .arg(&interleave_dir)
        .arg(&config.output_mxf)
        .output();

    let _ = std::fs::remove_dir_all(&interleave_dir);

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!(
                "Created stereo 3D MXF ({} frame pairs): {}",
                left_count,
                config.output_mxf.display()
            );
            0
        }
        Ok(o) => {
            tracing::error!(
                "asdcp-wrap stereo 3D failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            -1
        }
        Err(e) => {
            tracing::error!("Failed to run asdcp-wrap: {e}");
            -1
        }
    }
}

fn count_j2k_frames(dir: &std::path::Path) -> usize {
    list_j2k_frames(dir).len()
}

fn list_j2k_frames(dir: &std::path::Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "j2c" | "j2k"))
        })
        .collect()
}
