use sha1::Digest;
use std::path::{Path, PathBuf};

/// Copy a DCP to a target drive with SHA-1 hash verification.
pub fn copy_to_drive(dcp_dir: &Path, target_dir: &Path) -> i32 {
    if !dcp_dir.exists() {
        tracing::error!("Source DCP directory not found: {}", dcp_dir.display());
        return -1;
    }

    let dest = target_dir.join(
        dcp_dir
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("DCP")),
    );

    if let Err(e) = std::fs::create_dir_all(&dest) {
        tracing::error!("Failed to create target directory: {e}");
        return -1;
    }

    let files = collect_files(dcp_dir);
    let total = files.len();
    tracing::info!(
        "Copying {total} files from {} to {}",
        dcp_dir.display(),
        dest.display()
    );

    for (i, src_path) in files.iter().enumerate() {
        let rel = src_path.strip_prefix(dcp_dir).unwrap_or(src_path);
        let dst_path = dest.join(rel);

        if let Some(parent) = dst_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::error!("Failed to create directory {}: {e}", parent.display());
            return -1;
        }

        // Read source and compute hash
        let src_data = match std::fs::read(src_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to read {}: {e}", src_path.display());
                return -1;
            }
        };

        let src_hash = sha1_hex(&src_data);

        // Write to destination
        if let Err(e) = std::fs::write(&dst_path, &src_data) {
            tracing::error!("Failed to write {}: {e}", dst_path.display());
            return -1;
        }

        // Verify by reading back and comparing hash
        let dst_data = match std::fs::read(&dst_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to read back {}: {e}", dst_path.display());
                return -1;
            }
        };

        let dst_hash = sha1_hex(&dst_data);

        if src_hash != dst_hash {
            tracing::error!(
                "Hash mismatch for {}: src={} dst={}",
                rel.display(),
                src_hash,
                dst_hash
            );
            return -1;
        }

        tracing::info!(
            "[{}/{}] Verified: {} ({})",
            i + 1,
            total,
            rel.display(),
            src_hash
        );
    }

    tracing::info!("Successfully copied and verified {total} files");
    0
}

fn collect_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files);
    files.sort();
    files
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn sha1_hex(data: &[u8]) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex_encode(&result)
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}
