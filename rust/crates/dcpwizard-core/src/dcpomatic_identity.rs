use crate::certificate::{CERTIFICATE_PEM_LABEL, PEM_BEGIN_MARKER, PEM_MARKER_END, position_of};
use crate::preferences::{Preferences, load_preferences, preferences_directory, save_preferences};
use std::io::Write;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const DCPOMATIC_CONFIG_VERSIONS: [&str; 2] = ["2.18", "2.16"];
const DCPOMATIC_CONFIG_FILE: &str = "config.xml";
#[cfg(not(target_os = "macos"))]
const DCPOMATIC_CONFIG_DIRECTORY: &str = "dcpomatic2";
#[cfg(not(target_os = "macos"))]
const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
#[cfg(target_os = "macos")]
const DCPOMATIC_PREFERENCES_DIRECTORY: &str = "com.dcpomatic";
#[cfg(target_os = "macos")]
const DCPOMATIC_MAJOR_VERSION_DIRECTORY: &str = "2";

const LINK_ELEMENT: &str = "Link";
const DECRYPTION_ELEMENT: &str = "Decryption";
const CERTIFICATE_ELEMENT: &str = "Certificate";
const PRIVATE_KEY_ELEMENT: &str = "PrivateKey";
const PRIVATE_KEY_PEM_LABELS: [&str; 2] = ["PRIVATE KEY", "RSA PRIVATE KEY"];
const PEM_END_MARKER: &str = "-----END ";
const PEM_LINE_END: &[u8] = b"\n";

const IMPORTED_IDENTITY_DIRECTORY: &str = "recipient-identity";
const IMPORTED_CERTIFICATE_FILE: &str = "dcpomatic-recipient.pem";
const IMPORTED_PRIVATE_KEY_FILE: &str = "dcpomatic-recipient.key";
#[cfg(unix)]
const PRIVATE_KEY_FILE_MODE: u32 = 0o600;

#[derive(Debug)]
pub struct ImportedIdentity {
    pub certificate: PathBuf,
    pub private_key: PathBuf,
    pub thumbprint: String,
}

pub fn dcpomatic_config_candidates() -> Vec<PathBuf> {
    let Some(root) = dcpomatic_config_root() else {
        return Vec::new();
    };
    DCPOMATIC_CONFIG_VERSIONS
        .iter()
        .map(|version| root.join(version).join(DCPOMATIC_CONFIG_FILE))
        .chain(std::iter::once(root.join(DCPOMATIC_CONFIG_FILE)))
        .collect()
}

#[cfg(target_os = "macos")]
fn dcpomatic_config_root() -> Option<PathBuf> {
    dirs::preference_dir().map(|preferences| {
        preferences
            .join(DCPOMATIC_PREFERENCES_DIRECTORY)
            .join(DCPOMATIC_MAJOR_VERSION_DIRECTORY)
    })
}

#[cfg(not(target_os = "macos"))]
fn dcpomatic_config_root() -> Option<PathBuf> {
    glib_user_config_directory().map(|config| config.join(DCPOMATIC_CONFIG_DIRECTORY))
}

// glib takes any non-empty XDG_CONFIG_HOME, dirs ignores a relative one
#[cfg(not(target_os = "macos"))]
fn glib_user_config_directory() -> Option<PathBuf> {
    std::env::var_os(XDG_CONFIG_HOME)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(platform_user_config_directory)
}

#[cfg(target_os = "windows")]
fn platform_user_config_directory() -> Option<PathBuf> {
    dirs::config_local_dir()
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_user_config_directory() -> Option<PathBuf> {
    dirs::config_dir()
}

pub fn find_dcpomatic_config() -> Result<PathBuf, String> {
    let candidates = dcpomatic_config_candidates();
    if let Some(found) = candidates.iter().find(|candidate| candidate.is_file()) {
        return Ok(found.clone());
    }
    let looked_for: Vec<String> = candidates
        .iter()
        .map(|candidate| candidate.display().to_string())
        .collect();
    Err(format!(
        "no DCP-o-matic {DCPOMATIC_CONFIG_FILE} found, looked for {}",
        looked_for.join(", ")
    ))
}

#[derive(Clone, Copy)]
struct PemBlock<'a> {
    label: &'a str,
    text: &'a str,
}

fn is_pem_label(label: &str) -> bool {
    !label.is_empty()
        && label
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b' ')
}

// errors name labels only, never the text between the markers
fn pem_blocks(text: &str) -> Result<Vec<PemBlock<'_>>, String> {
    let bytes = text.as_bytes();
    let mut blocks = Vec::new();
    let mut offset = 0;
    while let Some(found) = position_of(&bytes[offset..], PEM_BEGIN_MARKER) {
        let begin = offset + found;
        let label_start = begin + PEM_BEGIN_MARKER.len();
        let line_length =
            position_of(&bytes[label_start..], PEM_LINE_END).unwrap_or(bytes.len() - label_start);
        let label = position_of(
            &bytes[label_start..label_start + line_length],
            PEM_MARKER_END,
        )
        .map(|label_length| &text[label_start..label_start + label_length])
        .filter(|label| is_pem_label(label))
        .ok_or("a PEM BEGIN line has no readable label")?;
        let end_line = format!("{PEM_END_MARKER}{label}-----");
        let end = position_of(&bytes[label_start..], end_line.as_bytes())
            .map(|end_offset| label_start + end_offset + end_line.len())
            .ok_or_else(|| format!("a {label} PEM block has no END line"))?;
        blocks.push(PemBlock {
            label,
            text: &text[begin..end],
        });
        offset = end;
    }
    Ok(blocks)
}

fn single_pem_block<'a>(
    config: &Path,
    element_name: &str,
    element: roxmltree::Node<'a, '_>,
) -> Result<PemBlock<'a>, String> {
    let blocks = pem_blocks(element.text().unwrap_or_default()).map_err(|error| {
        format!(
            "a {DECRYPTION_ELEMENT} {element_name} in {}: {error}",
            config.display()
        )
    })?;
    match blocks.as_slice() {
        [block] => Ok(*block),
        _ => Err(format!(
            "a {DECRYPTION_ELEMENT} {element_name} in {} holds {} PEM blocks, expected one",
            config.display(),
            blocks.len()
        )),
    }
}

struct LeafCertificate<'a> {
    pem: &'a str,
    thumbprint: String,
}

fn leaf_certificate<'a>(
    config: &Path,
    decryption: roxmltree::Node<'a, '_>,
) -> Result<LeafCertificate<'a>, String> {
    let mut certificates = Vec::new();
    for element in decryption
        .children()
        .filter(|node| node.has_tag_name(CERTIFICATE_ELEMENT))
    {
        let block = single_pem_block(config, CERTIFICATE_ELEMENT, element)?;
        if block.label.as_bytes() != CERTIFICATE_PEM_LABEL {
            return Err(format!(
                "a {DECRYPTION_ELEMENT} {CERTIFICATE_ELEMENT} in {} holds a {}, expected a CERTIFICATE",
                config.display(),
                block.label
            ));
        }
        let info = postkit::certificate::cert_info_from_pem(block.text).map_err(|error| {
            format!(
                "a {DECRYPTION_ELEMENT} {CERTIFICATE_ELEMENT} in {}: {error}",
                config.display()
            )
        })?;
        certificates.push((block.text, info));
    }
    let certificate_count = certificates.len();
    let mut leaves: Vec<_> = certificates
        .into_iter()
        .filter(|(_, info)| !info.is_ca)
        .collect();
    if leaves.len() != 1 {
        return Err(format!(
            "{} of the {certificate_count} {DECRYPTION_ELEMENT} certificates in {} are not a CA, expected exactly one leaf",
            leaves.len(),
            config.display()
        ));
    }
    let (pem, info) = leaves.remove(0);
    Ok(LeafCertificate {
        pem,
        thumbprint: info.thumbprint,
    })
}

fn private_key_block<'a>(
    config: &Path,
    decryption: roxmltree::Node<'a, '_>,
) -> Result<&'a str, String> {
    let mut elements = decryption
        .children()
        .filter(|node| node.has_tag_name(PRIVATE_KEY_ELEMENT));
    let element = elements.next().ok_or_else(|| {
        format!(
            "the {DECRYPTION_ELEMENT} element in {} has no {PRIVATE_KEY_ELEMENT}",
            config.display()
        )
    })?;
    if elements.next().is_some() {
        return Err(format!(
            "the {DECRYPTION_ELEMENT} element in {} has more than one {PRIVATE_KEY_ELEMENT}",
            config.display()
        ));
    }
    let block = single_pem_block(config, PRIVATE_KEY_ELEMENT, element)?;
    if !PRIVATE_KEY_PEM_LABELS.contains(&block.label) {
        return Err(format!(
            "the {DECRYPTION_ELEMENT} {PRIVATE_KEY_ELEMENT} in {} holds a {}, expected a {}",
            config.display(),
            block.label,
            PRIVATE_KEY_PEM_LABELS.join(" or ")
        ));
    }
    Ok(block.text)
}

fn read_config(config: &Path) -> Result<Zeroizing<String>, String> {
    std::fs::read_to_string(config)
        .map(Zeroizing::new)
        .map_err(|error| format!("cannot read {}: {error}", config.display()))
}

// the position only, a parse error can quote the text it stopped at
fn parse_config<'input>(
    config: &Path,
    contents: &'input str,
) -> Result<roxmltree::Document<'input>, String> {
    roxmltree::Document::parse(contents).map_err(|error| {
        format!(
            "{} is not readable XML at {}",
            config.display(),
            error.pos()
        )
    })
}

fn linked_config(config: &Path, contents: &str) -> Result<Option<PathBuf>, String> {
    let document = parse_config(config, contents)?;
    let link = document
        .root_element()
        .children()
        .find(|node| node.has_tag_name(LINK_ELEMENT))
        .and_then(|node| node.text())
        .map(|target| PathBuf::from(target.trim()));
    Ok(link)
}

// dcpomatic follows one Link to the file that holds the settings
fn read_settings(config: &Path) -> Result<(PathBuf, Zeroizing<String>), String> {
    let contents = read_config(config)?;
    match linked_config(config, &contents)? {
        Some(target) => {
            let linked = read_config(&target)?;
            Ok((target, linked))
        }
        None => Ok((config.to_path_buf(), contents)),
    }
}

fn write_private_key(path: &Path, private_key: &str) -> Result<(), String> {
    let cannot_write = |error: std::io::Error| format!("cannot write {}: {error}", path.display());
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, PRIVATE_KEY_FILE_MODE);
    let mut file = options.open(path).map_err(cannot_write)?;
    // an existing file keeps its old mode through open
    #[cfg(unix)]
    file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(
        PRIVATE_KEY_FILE_MODE,
    ))
    .map_err(cannot_write)?;
    file.write_all(private_key.as_bytes())
        .and_then(|()| file.write_all(PEM_LINE_END))
        .map_err(cannot_write)
}

pub fn import_dcpomatic_identity(
    config: &Path,
    destination_directory: &Path,
) -> Result<ImportedIdentity, String> {
    let (settings_file, contents) = read_settings(config)?;
    let document = parse_config(&settings_file, &contents)?;
    let decryption = document
        .root_element()
        .children()
        .find(|node| node.has_tag_name(DECRYPTION_ELEMENT))
        .ok_or_else(|| {
            format!(
                "{} has no {DECRYPTION_ELEMENT} element",
                settings_file.display()
            )
        })?;
    let leaf = leaf_certificate(&settings_file, decryption)?;
    let private_key = private_key_block(&settings_file, decryption)?;

    std::fs::create_dir_all(destination_directory)
        .map_err(|error| format!("cannot create {}: {error}", destination_directory.display()))?;
    let certificate = destination_directory.join(IMPORTED_CERTIFICATE_FILE);
    std::fs::write(&certificate, format!("{}\n", leaf.pem))
        .map_err(|error| format!("cannot write {}: {error}", certificate.display()))?;
    let private_key_path = destination_directory.join(IMPORTED_PRIVATE_KEY_FILE);
    write_private_key(&private_key_path, private_key)?;
    Ok(ImportedIdentity {
        certificate,
        private_key: private_key_path,
        thumbprint: leaf.thumbprint,
    })
}

fn preference_path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{} is not a UTF-8 path", path.display()))
}

pub fn import_dcpomatic_identity_into_preferences(
    config: Option<&Path>,
) -> Result<(Preferences, ImportedIdentity), String> {
    let config = match config {
        Some(config) => config.to_path_buf(),
        None => find_dcpomatic_config()?,
    };
    let mut preferences = load_preferences().map_err(|error| error.to_string())?;
    let identity = import_dcpomatic_identity(
        &config,
        &preferences_directory().join(IMPORTED_IDENTITY_DIRECTORY),
    )?;
    preferences.recipient_cert = preference_path_text(&identity.certificate)?;
    preferences.recipient_key = preference_path_text(&identity.private_key)?;
    save_preferences(&preferences).map_err(|error| error.to_string())?;
    Ok((preferences, identity))
}
