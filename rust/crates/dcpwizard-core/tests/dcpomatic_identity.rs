use dcpwizard_core::dcpomatic_identity::import_dcpomatic_identity;
use postkit::certificate::{cert_info_from_file, cert_info_from_pem, generate_chain};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tempfile::TempDir;

const PEM_BEGIN_MARKER: &str = "-----BEGIN ";

struct Chain {
    _directory: TempDir,
    path: PathBuf,
}

impl Chain {
    fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    fn pem(&self, name: &str) -> String {
        std::fs::read_to_string(self.file(name)).unwrap()
    }
}

fn generated_chain(organization: &str) -> Chain {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("chain");
    assert_eq!(
        generate_chain(organization, &path),
        0,
        "{organization} chain"
    );
    Chain {
        _directory: directory,
        path,
    }
}

fn decryption_chain() -> &'static Chain {
    static CHAIN: OnceLock<Chain> = OnceLock::new();
    CHAIN.get_or_init(|| generated_chain("Decryption"))
}

fn signing_chain() -> &'static Chain {
    static CHAIN: OnceLock<Chain> = OnceLock::new();
    CHAIN.get_or_init(|| generated_chain("Signing"))
}

fn identity_xml(element: &str, certificates: &[String], private_key: &str) -> String {
    let certificates: String = certificates
        .iter()
        .map(|certificate| format!("<Certificate>{certificate}</Certificate>"))
        .collect();
    format!("<{element}>{certificates}<PrivateKey>{private_key}</PrivateKey></{element}>")
}

// the signer chain sits first so reading the wrong element picks the wrong leaf
fn dcpomatic_config(decryption: Option<String>) -> String {
    let signing = signing_chain();
    let signer = identity_xml(
        "Signer",
        &[
            signing.pem("root.pem"),
            signing.pem("intermediate.pem"),
            signing.pem("signer.pem"),
        ],
        &signing.pem("signer.key"),
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Config><Version>3</Version>{signer}{}</Config>\n",
        decryption.unwrap_or_default()
    )
}

fn write_config(directory: &Path, contents: &str) -> PathBuf {
    let config = directory.join("config.xml");
    std::fs::write(&config, contents).unwrap();
    config
}

fn unordered_decryption_certificates() -> Vec<String> {
    let chain = decryption_chain();
    vec![
        chain.pem("intermediate.pem"),
        chain.pem("signer.pem"),
        chain.pem("root.pem"),
    ]
}

#[test]
fn the_leaf_and_its_key_are_written_from_the_decryption_element() {
    let chain = decryption_chain();
    let directory = TempDir::new().unwrap();
    let decryption = identity_xml(
        "Decryption",
        &unordered_decryption_certificates(),
        &chain.pem("signer.key"),
    );
    let config = write_config(directory.path(), &dcpomatic_config(Some(decryption)));
    let destination = directory.path().join("imported");

    let imported = import_dcpomatic_identity(&config, &destination).unwrap();

    let leaf_thumbprint = cert_info_from_file(&chain.file("signer.pem"))
        .unwrap()
        .thumbprint;
    assert_eq!(imported.thumbprint, leaf_thumbprint);
    let certificate = std::fs::read_to_string(&imported.certificate).unwrap();
    assert_eq!(
        cert_info_from_pem(&certificate).unwrap().thumbprint,
        leaf_thumbprint
    );
    assert_eq!(
        certificate.matches(PEM_BEGIN_MARKER).count(),
        1,
        "{certificate}"
    );
    // xml parsing turns the crlf a windows build writes into lf
    assert_eq!(
        std::fs::read(&imported.private_key).unwrap(),
        format!("{}\n", chain.pem("signer.key").replace("\r\n", "\n").trim()).into_bytes()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&imported.private_key)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

#[cfg(unix)]
#[test]
fn an_existing_world_readable_key_file_is_made_private() {
    use std::os::unix::fs::PermissionsExt;
    let chain = decryption_chain();
    let directory = TempDir::new().unwrap();
    let decryption = identity_xml(
        "Decryption",
        &unordered_decryption_certificates(),
        &chain.pem("signer.key"),
    );
    let config = write_config(directory.path(), &dcpomatic_config(Some(decryption)));
    let destination = directory.path().join("imported");
    let first = import_dcpomatic_identity(&config, &destination).unwrap();
    std::fs::set_permissions(&first.private_key, std::fs::Permissions::from_mode(0o644)).unwrap();

    let second = import_dcpomatic_identity(&config, &destination).unwrap();

    assert_eq!(second.private_key, first.private_key);
    let mode = std::fs::metadata(&second.private_key)
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn a_linked_config_is_read_from_its_link() {
    let chain = decryption_chain();
    let directory = TempDir::new().unwrap();
    let decryption = identity_xml(
        "Decryption",
        &unordered_decryption_certificates(),
        &chain.pem("signer.key"),
    );
    let settings = directory.path().join("shared.xml");
    std::fs::write(&settings, dcpomatic_config(Some(decryption))).unwrap();
    let config = write_config(
        directory.path(),
        &format!("<Config><Link>{}</Link></Config>", settings.display()),
    );

    let imported = import_dcpomatic_identity(&config, &directory.path().join("imported")).unwrap();

    assert_eq!(
        imported.thumbprint,
        cert_info_from_file(&chain.file("signer.pem"))
            .unwrap()
            .thumbprint
    );
}

fn assert_refused(config_contents: &str, expected: &[&str]) {
    let directory = TempDir::new().unwrap();
    let config = write_config(directory.path(), config_contents);
    let destination = directory.path().join("imported");

    let error = import_dcpomatic_identity(&config, &destination).unwrap_err();

    assert!(error.contains(&config.display().to_string()), "{error}");
    for part in expected {
        assert!(error.contains(part), "{error}");
    }
    assert!(
        !destination.exists(),
        "nothing is written for a refused config"
    );
}

#[test]
fn a_config_without_a_decryption_element_is_refused_by_name() {
    assert_refused(&dcpomatic_config(None), &["has no Decryption element"]);
}

#[test]
fn a_chain_of_only_certificate_authorities_is_refused_naming_the_count() {
    let chain = decryption_chain();
    let decryption = identity_xml(
        "Decryption",
        &[chain.pem("root.pem"), chain.pem("intermediate.pem")],
        &chain.pem("signer.key"),
    );

    assert_refused(
        &dcpomatic_config(Some(decryption)),
        &["0 of the 2 Decryption certificates"],
    );
}

#[test]
fn two_leaves_are_refused_naming_the_count() {
    let mut certificates = unordered_decryption_certificates();
    certificates.push(signing_chain().pem("signer.pem"));
    let decryption = identity_xml(
        "Decryption",
        &certificates,
        &decryption_chain().pem("signer.key"),
    );

    assert_refused(
        &dcpomatic_config(Some(decryption)),
        &["2 of the 4 Decryption certificates"],
    );
}

#[test]
fn a_private_key_element_holding_a_certificate_is_refused_by_name() {
    let chain = decryption_chain();
    let decryption = identity_xml(
        "Decryption",
        &unordered_decryption_certificates(),
        &chain.pem("signer.pem"),
    );

    assert_refused(
        &dcpomatic_config(Some(decryption)),
        &["Decryption PrivateKey", "holds a CERTIFICATE"],
    );
}

#[test]
fn a_certificate_element_holding_a_private_key_is_refused_without_the_key() {
    let chain = decryption_chain();
    let private_key = chain.pem("signer.key");
    let mut certificates = unordered_decryption_certificates();
    certificates.push(private_key.clone());
    let decryption = identity_xml("Decryption", &certificates, &private_key);
    let directory = TempDir::new().unwrap();
    let config = write_config(directory.path(), &dcpomatic_config(Some(decryption)));

    let error = import_dcpomatic_identity(&config, &directory.path().join("imported")).unwrap_err();

    assert!(
        error.contains("Decryption Certificate") && error.contains("holds a PRIVATE KEY"),
        "{error}"
    );
    let key_body_line = private_key.lines().nth(1).unwrap();
    assert!(!error.contains(key_body_line), "the error quotes the key");
}
