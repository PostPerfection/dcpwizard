use std::path::Path;

pub use postkit::certificate::{CertInfo, CertOptions, CertType, TrustedDevice};

/// Generate a single X.509 certificate.
pub fn generate_certificate(opts: &CertOptions) -> i32 {
    postkit::certificate::generate_certificate(opts)
}

/// Generate a full certificate chain (root → intermediate → signer).
pub fn generate_chain(organization: &str, output_dir: &Path) -> i32 {
    postkit::certificate::generate_chain(organization, output_dir)
}

/// Read and display certificate info from a PEM file.
pub fn read_certificate(cert_path: &Path) -> CertInfo {
    postkit::certificate::read_certificate(cert_path)
}

const NO_RECIPIENT_CERTIFICATE: &str = "no recipient certificate is configured";
const PEM_BEGIN_MARKER: &[u8] = b"-----BEGIN ";
const PEM_MARKER_END: &[u8] = b"-----";
const CERTIFICATE_PEM_LABEL: &[u8] = b"CERTIFICATE";

fn position_of(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn first_pem_label_other_than_certificate(contents: &[u8]) -> Option<String> {
    let mut rest = contents;
    while let Some(start) = position_of(rest, PEM_BEGIN_MARKER) {
        let after_marker = &rest[start + PEM_BEGIN_MARKER.len()..];
        let label_length = position_of(after_marker, PEM_MARKER_END).unwrap_or(after_marker.len());
        let label = &after_marker[..label_length];
        if label != CERTIFICATE_PEM_LABEL {
            return Some(String::from_utf8_lossy(label).into_owned());
        }
        rest = &after_marker[label_length..];
    }
    None
}

pub fn export_certificate(source: &Path, destination: &Path) -> Result<(), String> {
    if source.as_os_str().is_empty() {
        return Err(NO_RECIPIENT_CERTIFICATE.to_string());
    }
    postkit::certificate::cert_info_from_file(source)?;
    let contents = std::fs::read(source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    // a bundle of certificate and private key parses as its first certificate
    if let Some(label) = first_pem_label_other_than_certificate(&contents) {
        return Err(format!(
            "{} holds a {label} beside the certificate, export a file with certificates only",
            source.display()
        ));
    }
    std::fs::write(destination, &contents)
        .map_err(|error| format!("cannot write {}: {error}", destination.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn recipient_identity(directory: &Path) -> (PathBuf, PathBuf) {
        let certificate = directory.join("recipient.pem");
        let key = directory.join("recipient.key");
        let options = CertOptions {
            cert_type: CertType::Root,
            common_name: "recipient".into(),
            organization: "Cinema".into(),
            output_cert: certificate.clone(),
            output_key: key.clone(),
            ..Default::default()
        };
        assert_eq!(generate_certificate(&options), 0, "recipient certificate");
        (certificate, key)
    }

    #[test]
    fn an_exported_certificate_is_the_configured_file_byte_for_byte() {
        let directory = TempDir::new().unwrap();
        let (certificate, _) = recipient_identity(directory.path());
        let destination = directory.path().join("for-the-distributor.pem");

        export_certificate(&certificate, &destination).unwrap();

        assert_eq!(
            std::fs::read(&destination).unwrap(),
            std::fs::read(&certificate).unwrap()
        );
    }

    #[test]
    fn a_private_key_is_refused_by_name() {
        let directory = TempDir::new().unwrap();
        let (_, key) = recipient_identity(directory.path());
        let destination = directory.path().join("for-the-distributor.pem");

        let error = export_certificate(&key, &destination).unwrap_err();

        assert!(error.contains(&key.display().to_string()), "{error}");
        assert!(!destination.exists());
    }

    #[test]
    fn a_certificate_bundled_with_its_private_key_is_refused() {
        let directory = TempDir::new().unwrap();
        let (certificate, key) = recipient_identity(directory.path());
        let bundle = directory.path().join("bundle.pem");
        let mut contents = std::fs::read(&certificate).unwrap();
        contents.extend(std::fs::read(&key).unwrap());
        std::fs::write(&bundle, contents).unwrap();
        let destination = directory.path().join("for-the-distributor.pem");

        let error = export_certificate(&bundle, &destination).unwrap_err();

        assert!(error.contains(&bundle.display().to_string()), "{error}");
        assert!(error.contains("PRIVATE KEY"), "{error}");
        assert!(!destination.exists());
    }

    #[test]
    fn no_configured_certificate_is_refused() {
        let directory = TempDir::new().unwrap();
        let destination = directory.path().join("for-the-distributor.pem");

        let error = export_certificate(Path::new(""), &destination).unwrap_err();

        assert_eq!(error, NO_RECIPIENT_CERTIFICATE);
        assert!(!destination.exists());
    }
}
