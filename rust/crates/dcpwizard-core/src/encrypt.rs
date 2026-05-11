use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Encryption configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub dcp_dir: PathBuf,
    pub content_key: String,
    pub key_id: String,
}

/// Encrypt a DCP in-place with AES-128 content encryption.
///
/// This requires asdcplib for MXF-level encryption. Currently delegates
/// to the `asdcp-wrap` tool if available.
pub fn encrypt_dcp(config: &EncryptionConfig) -> i32 {
    let key = if config.content_key.is_empty() {
        // Generate random 16-byte key
        let mut key = [0u8; 16];
        for b in &mut key {
            *b = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
                & 0xFF) as u8;
        }
        hex_encode(&key)
    } else {
        config.content_key.clone()
    };

    let key_id = if config.key_id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        config.key_id.clone()
    };

    tracing::info!(
        "Encrypting DCP at {} with key_id {}",
        config.dcp_dir.display(),
        key_id
    );

    // Find MXF files to encrypt
    let mxf_files: Vec<PathBuf> = std::fs::read_dir(&config.dcp_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("mxf"))
        })
        .collect();

    for mxf in &mxf_files {
        let encrypted = mxf.with_extension("encrypted.mxf");
        let result = std::process::Command::new("asdcp-wrap")
            .arg("-e")
            .arg("-k")
            .arg(&key)
            .arg("-a")
            .arg(&key_id)
            .arg(mxf)
            .arg(&encrypted)
            .output();

        match result {
            Ok(o) if o.status.success() => {
                // Replace original with encrypted
                let _ = std::fs::rename(&encrypted, mxf);
            }
            Ok(o) => {
                tracing::error!(
                    "Encryption failed for {}: {}",
                    mxf.display(),
                    String::from_utf8_lossy(&o.stderr)
                );
                return -1;
            }
            Err(e) => {
                tracing::error!("Failed to run asdcp-wrap: {e}");
                return -1;
            }
        }
    }

    tracing::info!("Encryption complete ({} MXF files)", mxf_files.len());
    0
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}
