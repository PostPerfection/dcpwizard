use assert_cmd::Command;
use postkit::certificate::{
    CertOptions, CertType, generate_certificate, generate_chain, parse_kdm, unwrap_kdm_file,
};
use std::path::{Path, PathBuf};

const CPL_ID: &str = "urn:uuid:6f2b1a5e-4c8d-4f3a-9b1e-7c0a2d5e8f31";
const TITLE: &str = "Test Feature";
const SMPTE_NS: &str = "http://www.smpte-ra.org/schemas/430-1/2006/KDM";
const CONTENT_KEY_LEN: usize = 16;

// a KDM starting on the day its signer certificate does is refused
fn tomorrow() -> String {
    (chrono::Utc::now() + chrono::Duration::days(1))
        .format("%Y-%m-%dT%H:%M:%S+00:00")
        .to_string()
}

fn recipient_cert(dir: &Path, chain: &Path, stem: &str) -> PathBuf {
    let cert = dir.join(format!("{stem}.pem"));
    let options = CertOptions {
        cert_type: CertType::Leaf,
        common_name: stem.into(),
        organization: "Cinema".into(),
        output_cert: cert.clone(),
        output_key: dir.join(format!("{stem}.key")),
        issuer_cert: chain.join("root.pem"),
        issuer_key: chain.join("root.key"),
        ..Default::default()
    };
    assert_eq!(generate_certificate(&options), 0, "recipient cert {stem}");
    cert
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    chain: PathBuf,
    recipient: PathBuf,
    recipient_key: PathBuf,
}

impl Fixture {
    fn build() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        let chain = root.join("chain");
        assert_eq!(generate_chain("Studio", &chain), 0, "signer chain");
        let recipient = recipient_cert(&root, &chain, "screen1");
        let recipient_key = root.join("screen1.key");
        Self {
            _temp: temp,
            root,
            chain,
            recipient,
            recipient_key,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("dcpwizard").unwrap();
        command
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_DATA_HOME", self.root.join("data"));
        command
    }

    fn kdm(&self, output: &Path) -> Command {
        let mut command = self.command();
        command
            .args(["kdm", "--cpl-id", CPL_ID, "--content-title", TITLE])
            .args(["--valid-from", &tomorrow(), "--valid-to", "2 weeks"])
            .arg("--cert")
            .arg(&self.recipient)
            .arg("--signer-cert")
            .arg(self.chain.join("signer.pem"))
            .arg("--signer-key")
            .arg(self.chain.join("signer.key"))
            .arg("--signer-chain")
            .arg(self.chain.join("intermediate.pem"))
            .arg("--signer-chain")
            .arg(self.chain.join("root.pem"))
            .arg("--output")
            .arg(output);
        command
    }
}

// the same arguments postkit's own xmlsec1 test uses
mod xmlsec1 {
    use std::path::Path;
    use std::process::{Command, Output};
    use std::sync::OnceLock;

    // matches a key the document names by certificate rather than by name
    const LAX_KEY_SEARCH: &str = "--lax-key-search";
    // 1.3 prints only "Failure reason: SIGNATURE" without this
    const VERBOSE: &str = "--verbose";
    // the Id-bearing elements of a KDM, which xmlsec1 will not find on its own
    const ID_ATTRIBUTES: &[&str] = &["AuthenticatedPublic", "AuthenticatedPrivate"];

    // `--help` lists only the commands, so the options are behind `--help-all`
    fn help_all() -> &'static str {
        static HELP: OnceLock<String> = OnceLock::new();
        HELP.get_or_init(|| {
            let help = run(Command::new("xmlsec1").arg("--help-all"));
            String::from_utf8_lossy(&help.stdout).into_owned()
        })
    }

    fn run(command: &mut Command) -> Output {
        command
            .output()
            .unwrap_or_else(|error| panic!("could not run xmlsec1: {error}"))
    }

    pub fn verify(document: &Path, trusted: &Path, untrusted: &[&Path]) -> Output {
        let mut command = Command::new("xmlsec1");
        command.arg("--verify");
        // the msys2 build defaults to mscrypto, which cannot load a pem cert
        if cfg!(windows) {
            command.args(["--crypto", "openssl"]);
        }
        for option in [LAX_KEY_SEARCH, VERBOSE] {
            if help_all().contains(option) {
                command.arg(option);
            }
        }
        command.arg("--trusted-pem").arg(trusted);
        for certificate in untrusted {
            command.arg("--untrusted-pem").arg(certificate);
        }
        for name in ID_ATTRIBUTES {
            command.args(["--id-attr:Id", name]);
        }
        run(command.arg(document))
    }

    // everything xmlsec1 said, so a failed assertion names the reason
    pub fn report(output: &Output) -> String {
        format!(
            "status: {}\n  stdout: {}\n  stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim(),
        )
    }
}

#[test]
fn kdm_is_written_in_the_smpte_namespace_and_unwraps() {
    let fixture = Fixture::build();
    let out = fixture.root.join("smpte.kdm.xml");
    fixture.kdm(&out).assert().success();

    let xml = std::fs::read_to_string(&out).expect("KDM written");
    assert!(xml.contains(SMPTE_NS), "SMPTE namespace missing");

    let metadata = parse_kdm(&xml).expect("parse kdm");
    assert_eq!(
        format!("urn:uuid:{}", metadata.cpl_id),
        CPL_ID,
        "the KDM is for the requested composition"
    );
    assert_eq!(metadata.content_title, TITLE);
    assert_eq!(metadata.key_ids.len(), 1, "one fresh content key");

    let unwrapped = unwrap_kdm_file(&out, &fixture.recipient_key)
        .expect("recipient private key must unwrap the KDM");
    assert_eq!(unwrapped.cpl_id, metadata.cpl_id);
    assert_eq!(unwrapped.keys.len(), 1);
    assert_eq!(unwrapped.keys[0].key_id, metadata.key_ids[0].key_id);
    let content_key = unwrapped
        .content_key(&metadata.key_ids[0].key_id)
        .expect("content key for the listed key id");
    assert_eq!(content_key.len(), CONTENT_KEY_LEN);
    assert_ne!(
        content_key, &[0u8; CONTENT_KEY_LEN],
        "the unwrapped content key is all zeroes"
    );
}

#[test]
fn kdm_signature_verifies_with_xmlsec1() {
    let fixture = Fixture::build();
    let out = fixture.root.join("smpte.kdm.xml");
    fixture.kdm(&out).assert().success();

    let result = xmlsec1::verify(
        &out,
        &fixture.chain.join("root.pem"),
        &[&fixture.chain.join("intermediate.pem")],
    );
    assert!(
        result.status.success(),
        "KDM signature must verify\n  {}",
        xmlsec1::report(&result)
    );
}

#[test]
fn format_flag_is_rejected() {
    let fixture = Fixture::build();
    let out = fixture.root.join("rejected.kdm.xml");
    let assertion = fixture
        .kdm(&out)
        .args(["--format", "interop"])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
    assert!(
        stderr.contains("unexpected argument"),
        "clap must refuse --format on kdm, said: {stderr}"
    );
    assert!(!out.exists(), "no KDM is written when the flag is refused");
}
