//! Thin delegation over postkit's SMPTE 430-1 KDM implementation.
//!
//! postkit generates a fresh content key, RSA-OAEP encrypts it to the recipient
//! and signs the message per SMPTE 430-3. This layer only maps the CLI's inputs
//! onto postkit's config types and turns its `Result` into a dcpwizard exit code.

use std::path::{Path, PathBuf};

/// Load the content keys from a DCP keys file (written by `create --encrypt`)
/// for KDM generation, checking they belong to `cpl_id`.
pub fn load_content_keys(
    keys_file: &Path,
    cpl_id: &str,
) -> Result<Vec<postkit::certificate::KdmContentKey>, String> {
    let bundle = crate::encrypt::KeyBundle::read(keys_file)?;
    let want = cpl_id.trim().trim_start_matches("urn:uuid:");
    let have = bundle.cpl_id.trim().trim_start_matches("urn:uuid:");
    if !have.is_empty() && !want.is_empty() && have != want {
        return Err(format!(
            "keys file {} is for CPL {have}, not {want}",
            keys_file.display()
        ));
    }
    bundle
        .keys
        .iter()
        .map(|k| {
            let (key_type, key_id, content_key) = k.to_raw()?;
            Ok(postkit::certificate::KdmContentKey {
                key_type,
                key_id,
                content_key,
            })
        })
        .collect()
}

/// Generate a signed KDM. `content_keys` (from the DCP's keys file) binds the
/// KDM to the encrypted essence; an empty vec makes postkit mint a fresh key.
/// `valid_from`/`valid_to` accept "now", ISO 8601 or a relative duration
/// ("2 weeks"), parsed by postkit.
#[allow(clippy::too_many_arguments)]
pub fn generate_kdm(
    cpl_id: String,
    content_title: String,
    recipient_cert: PathBuf,
    signer_cert: PathBuf,
    signer_key: PathBuf,
    signer_chain: Vec<PathBuf>,
    valid_from: String,
    valid_to: String,
    formulation: String,
    content_keys: Vec<postkit::certificate::KdmContentKey>,
    output: PathBuf,
) -> i32 {
    let config = postkit::certificate::KdmConfig {
        cpl_id,
        content_title,
        recipient_cert_file: recipient_cert,
        signer_cert_file: signer_cert,
        signer_key_file: signer_key,
        signer_chain_files: signer_chain,
        output_file: output,
        valid_from,
        valid_to,
        formulation,
        content_keys,
    };
    match postkit::certificate::generate_kdm(&config) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("{e}");
            1
        }
    }
}

/// Generate one KDM per recipient certificate in a single pass. Each KDM is
/// written to `output_dir/<cert-stem>.kdm.xml`. Returns 0 only if every
/// recipient succeeded, otherwise 1.
#[allow(clippy::too_many_arguments)]
pub fn generate_kdm_batch(
    cpl_id: String,
    content_title: String,
    recipient_certs: Vec<PathBuf>,
    signer_cert: PathBuf,
    signer_key: PathBuf,
    signer_chain: Vec<PathBuf>,
    valid_from: String,
    valid_to: String,
    formulation: String,
    content_keys: Vec<postkit::certificate::KdmContentKey>,
    output_dir: PathBuf,
) -> i32 {
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return 1;
    }

    let mut failures = 0;
    for (i, cert) in recipient_certs.iter().enumerate() {
        let stem = cert
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("recipient");
        // index-prefix keeps outputs unique even when recipient certs share a filename
        let output = output_dir.join(format!("{:03}_{stem}.kdm.xml", i + 1));
        let code = generate_kdm(
            cpl_id.clone(),
            content_title.clone(),
            cert.clone(),
            signer_cert.clone(),
            signer_key.clone(),
            signer_chain.clone(),
            valid_from.clone(),
            valid_to.clone(),
            formulation.clone(),
            content_keys.clone(),
            output.clone(),
        );
        if code == 0 {
            tracing::info!("KDM for {} -> {}", cert.display(), output.display());
        } else {
            tracing::error!("KDM generation failed for {}", cert.display());
            failures += 1;
        }
    }

    if failures == 0 {
        tracing::info!("Generated {} KDM(s)", recipient_certs.len());
        0
    } else {
        tracing::error!("{failures} of {} KDM(s) failed", recipient_certs.len());
        1
    }
}

/// Re-wrap a DKDM to a new recipient: decrypt its content keys with the DKDM
/// recipient's private key, re-encrypt to `recipient_cert` and sign. Empty
/// `valid_from`/`valid_to` preserve the DKDM's validity window.
#[allow(clippy::too_many_arguments)]
pub fn rewrap_dkdm(
    dkdm: PathBuf,
    dkdm_key: PathBuf,
    recipient_cert: PathBuf,
    signer_cert: PathBuf,
    signer_key: PathBuf,
    signer_chain: Vec<PathBuf>,
    valid_from: String,
    valid_to: String,
    output: PathBuf,
) -> i32 {
    let config = postkit::certificate::RewrapConfig {
        dkdm_file: dkdm,
        dkdm_recipient_key_file: dkdm_key,
        recipient_cert_file: recipient_cert,
        signer_cert_file: signer_cert,
        signer_key_file: signer_key,
        signer_chain_files: signer_chain,
        output_file: output,
        valid_from,
        valid_to,
    };
    match postkit::certificate::rewrap_dkdm_to_file(&config) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("{e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These exercise the real postkit call and the Result->exit-code mapping.
    // The crypto/XML/signature behaviour is postkit's and is tested there.

    #[test]
    fn generate_kdm_empty_cpl_id_fails() {
        let out = tempfile::NamedTempFile::new().unwrap();
        let code = generate_kdm(
            String::new(),
            "Test".into(),
            PathBuf::from("/dev/null"),
            PathBuf::from("/dev/null"),
            PathBuf::from("/dev/null"),
            Vec::new(),
            "now".into(),
            "2 weeks".into(),
            "modified-transitional-1".into(),
            Vec::new(),
            out.path().to_path_buf(),
        );
        assert_ne!(code, 0);
    }

    #[test]
    fn rewrap_dkdm_missing_file_fails() {
        let out = tempfile::NamedTempFile::new().unwrap();
        let code = rewrap_dkdm(
            PathBuf::from("/nonexistent/dkdm.xml"),
            PathBuf::from("/dev/null"),
            PathBuf::from("/dev/null"),
            PathBuf::from("/dev/null"),
            PathBuf::from("/dev/null"),
            Vec::new(),
            String::new(),
            String::new(),
            out.path().to_path_buf(),
        );
        assert_ne!(code, 0);
    }
}
