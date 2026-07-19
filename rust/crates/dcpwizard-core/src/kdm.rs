//! Thin delegation over postkit's SMPTE 430-1 KDM implementation.
//!
//! postkit generates a fresh content key, RSA-OAEP encrypts it to the recipient
//! and signs the message per SMPTE 430-3. This layer only maps the CLI's inputs
//! onto postkit's config types and turns its `Result` into a dcpwizard exit code.

use std::path::PathBuf;

/// Generate a signed KDM: encrypt a fresh content key to `recipient_cert` and
/// sign with the signer identity. `valid_from`/`valid_to` accept "now", ISO 8601
/// or a relative duration ("2 weeks"), parsed by postkit.
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
    };
    match postkit::certificate::generate_kdm(&config) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("{e}");
            1
        }
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
