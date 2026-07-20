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

/// Collect recipient certificate paths from a directory: every *.pem/*.crt/*.cer
/// in it, sorted for a deterministic KDM order. Errors if the directory cannot be
/// read or holds no certificate, so a mistyped path fails loud instead of
/// silently producing zero KDMs.
pub fn certs_in_dir(dir: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read cert directory {}: {e}", dir.display()))?;
    let mut certs: Vec<String> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|x| x.to_str())
                    .map(|x| matches!(x.to_ascii_lowercase().as_str(), "pem" | "crt" | "cer"))
                    .unwrap_or(false)
        })
        .filter_map(|p| p.to_str().map(String::from))
        .collect();
    certs.sort();
    if certs.is_empty() {
        return Err(format!(
            "no certificates (*.pem/*.crt/*.cer) found in {}",
            dir.display()
        ));
    }
    Ok(certs)
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

    #[test]
    fn certs_in_dir_lists_only_certs_sorted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.pem"), "x").unwrap();
        std::fs::write(dir.path().join("a.crt"), "x").unwrap();
        std::fs::write(dir.path().join("k.key"), "x").unwrap(); // private key, excluded
        std::fs::write(dir.path().join("notes.txt"), "x").unwrap();
        let found = certs_in_dir(dir.path()).unwrap();
        assert_eq!(found.len(), 2, "only cert extensions counted");
        assert!(
            found[0].ends_with("a.crt") && found[1].ends_with("b.pem"),
            "sorted"
        );
    }

    #[test]
    fn certs_in_dir_empty_or_missing_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            certs_in_dir(dir.path()).is_err(),
            "empty dir must fail loud"
        );
        assert!(certs_in_dir(Path::new("/nonexistent/certs")).is_err());
    }

    #[test]
    fn batch_reports_failure_for_a_bad_recipient() {
        let dir = tempfile::tempdir().unwrap();
        let code = generate_kdm_batch(
            "8a2b1c3d-4e5f-6071-8293-a4b5c6d7e8f9".into(),
            "T".into(),
            vec![PathBuf::from("/nonexistent/recipient.pem")],
            PathBuf::from("/dev/null"),
            PathBuf::from("/dev/null"),
            Vec::new(),
            "now".into(),
            "7 days".into(),
            "modified-transitional-1".into(),
            Vec::new(),
            dir.path().join("out"),
        );
        assert_ne!(code, 0);
    }

    use postkit::certificate::{CertOptions, CertType, generate_certificate, generate_chain};

    /// Generate a signer chain in `dir` plus `n` recipient leaf certs in a
    /// `recipients/` subdir. Returns (signer_cert, signer_key, chain, recipient_dir).
    fn batch_fixtures(dir: &Path, n: usize) -> (PathBuf, PathBuf, Vec<PathBuf>, PathBuf) {
        assert_eq!(generate_chain("Acme", dir), 0, "chain generation failed");
        let recipients = dir.join("recipients");
        std::fs::create_dir_all(&recipients).unwrap();
        for i in 0..n {
            let opts = CertOptions {
                cert_type: CertType::Leaf,
                common_name: format!("Screen {i}"),
                organization: "Cinema".into(),
                output_cert: recipients.join(format!("screen_{i}.pem")),
                output_key: recipients.join(format!("screen_{i}.key")),
                issuer_cert: dir.join("root.pem"),
                issuer_key: dir.join("root.key"),
                ..Default::default()
            };
            assert_eq!(generate_certificate(&opts), 0, "recipient {i} gen failed");
        }
        (
            dir.join("signer.pem"),
            dir.join("signer.key"),
            vec![dir.join("intermediate.pem"), dir.join("root.pem")],
            recipients,
        )
    }

    fn xmlsec1_available() -> bool {
        std::process::Command::new("xmlsec1")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    // Real end-to-end batch: distinct recipients, bound content KeyId, signed.
    #[test]
    fn batch_generates_one_signed_kdm_per_recipient_bound_to_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let (signer_cert, signer_key, chain, recipients) = batch_fixtures(dir.path(), 2);
        let certs = certs_in_dir(&recipients).expect("recipients found");
        assert_eq!(certs.len(), 2, "cert_dir globbing skips the .key files");

        // A known content KeyId, as if taken from the DCP's KEYS.json.
        let key_id = uuid::Uuid::new_v4();
        let content_keys = vec![postkit::certificate::KdmContentKey {
            key_type: *b"MDIK",
            key_id,
            content_key: [7u8; 16],
        }];

        let cpl_id = "8a2b1c3d-4e5f-6071-8293-a4b5c6d7e8f9";
        let out = dir.path().join("kdms");
        let code = generate_kdm_batch(
            cpl_id.into(),
            "Test Feature".into(),
            certs.iter().map(PathBuf::from).collect(),
            signer_cert,
            signer_key,
            chain,
            "now".into(),
            "7 days".into(),
            "modified-transitional-1".into(),
            content_keys,
            out.clone(),
        );
        assert_eq!(code, 0, "batch must succeed for every recipient");

        let kdms: Vec<PathBuf> = std::fs::read_dir(&out)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("xml"))
            .collect();
        assert_eq!(kdms.len(), 2, "one KDM per recipient");

        for kdm in &kdms {
            let xml = std::fs::read_to_string(kdm).unwrap();
            assert!(
                xml.contains(&format!(
                    "<CompositionPlaylistId>urn:uuid:{cpl_id}</CompositionPlaylistId>"
                )),
                "KDM must reference the CPL"
            );
            assert!(
                xml.contains(
                    "<MessageType>http://www.smpte-ra.org/430-1/2006/KDM#kdm-key-type</MessageType>"
                ),
                "standard SMPTE KDM MessageType"
            );
            assert!(
                xml.contains(&format!("<KeyId>urn:uuid:{key_id}</KeyId>")),
                "KeyId must match the DCP's content key"
            );
            assert!(xml.contains("<ds:Signature"), "KDM must be signed");

            if xmlsec1_available() {
                let ok = std::process::Command::new("xmlsec1")
                    .arg("--verify")
                    .arg("--trusted-pem")
                    .arg(dir.path().join("root.pem"))
                    .args(["--id-attr:Id", "AuthenticatedPublic"])
                    .args(["--id-attr:Id", "AuthenticatedPrivate"])
                    .arg(kdm)
                    .output()
                    .expect("run xmlsec1")
                    .status
                    .success();
                assert!(ok, "xmlsec1 must verify the batch KDM against the root");
            }
        }
    }
}
