// dcpwizard's KDM commands over postkit's issue path: argument mapping and exit codes
use std::path::{Path, PathBuf};

use postkit::kdm_distribution::formulation::FormulationFlagNames;
use postkit::kdm_distribution::issue::{KdmRequest, KdmSigner, issue_kdm, issue_kdm_batch};

pub use postkit::kdm_distribution::issue::{KdmOptions, certs_in_dir};

pub const COMMAND_LINE_FLAGS: FormulationFlagNames = FormulationFlagNames {
    formulation: "--formulation",
    device_certificate: "--device-cert",
};

#[allow(clippy::too_many_arguments)]
pub fn kdm_request(
    cpl_id: String,
    content_title: String,
    signer_cert: PathBuf,
    signer_key: PathBuf,
    signer_chain: Vec<PathBuf>,
    valid_from: String,
    valid_to: String,
    content_keys: Vec<postkit::certificate::KdmContentKey>,
    annotation: Option<String>,
    history: Option<PathBuf>,
    device_certs: Vec<PathBuf>,
    options: KdmOptions,
) -> KdmRequest {
    KdmRequest {
        cpl_id,
        content_title,
        signer: KdmSigner {
            certificate: signer_cert,
            key: signer_key,
            chain: signer_chain,
        },
        valid_from,
        valid_to,
        content_keys,
        annotation,
        history,
        device_certs,
        options,
        formulation_flags: COMMAND_LINE_FLAGS,
    }
}

// the content keys of a DCP keys file written by `create --encrypt`, checked against cpl_id
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

fn exit_code(result: Result<(), String>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("{e}");
            1
        }
    }
}

// empty content_keys makes postkit mint a fresh key, empty device_certs writes assume trust
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
    content_keys: Vec<postkit::certificate::KdmContentKey>,
    output: PathBuf,
    annotation: Option<String>,
    history: Option<PathBuf>,
    device_certs: Vec<PathBuf>,
    options: KdmOptions,
) -> i32 {
    let request = kdm_request(
        cpl_id,
        content_title,
        signer_cert,
        signer_key,
        signer_chain,
        valid_from,
        valid_to,
        content_keys,
        annotation,
        history,
        device_certs,
        options,
    );
    exit_code(issue_kdm(&request, &recipient_cert, &output))
}

pub fn generate_kdm_batch(
    request: &KdmRequest,
    recipient_certs: &[PathBuf],
    output_dir: &Path,
) -> i32 {
    exit_code(issue_kdm_batch(request, recipient_certs, output_dir))
}

// empty valid_from and valid_to keep the DKDM's window
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
    device_certs: Vec<PathBuf>,
    options: KdmOptions,
) -> i32 {
    let formulation = match postkit::kdm_distribution::formulation::resolve_formulation(
        options.formulation,
        device_certs.len(),
        COMMAND_LINE_FLAGS,
    ) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("{e}");
            return 1;
        }
    };
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
        device_cert_files: device_certs,
        formulation,
        picture_forensic_marking: options.picture_forensic_marking,
        audio_forensic_marking: options.audio_forensic_marking,
        issue_date: None,
    };
    exit_code(postkit::certificate::rewrap_dkdm_to_file(&config))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Vec::new(),
            out.path().to_path_buf(),
            None,
            None,
            Vec::new(),
            Default::default(),
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
            Vec::new(),
            Default::default(),
        );
        assert_ne!(code, 0);
    }
}
