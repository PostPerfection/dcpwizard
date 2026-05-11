use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// KDM generation configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KdmConfig {
    pub cpl_id: String,
    pub content_title: String,
    pub not_valid_before: String,
    pub not_valid_after: String,
    pub recipient_cert_file: PathBuf,
    pub signer_cert_file: PathBuf,
    pub signer_key_file: PathBuf,
    pub content_keys: Vec<ContentKey>,
    pub output_file: PathBuf,
}

/// A content key entry within a KDM.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentKey {
    pub key_id: String,
    pub key_type: String,
    pub cipher_data: String,
}

/// Generate a single KDM XML file.
pub fn generate_kdm(config: &KdmConfig) -> i32 {
    if config.cpl_id.is_empty() {
        tracing::error!("CPL ID is required for KDM generation");
        return -1;
    }

    if config.content_keys.is_empty() {
        tracing::error!("At least one content key is required");
        return -1;
    }

    let message_id = uuid::Uuid::new_v4();

    let not_before = if config.not_valid_before.is_empty() {
        "2024-01-01T00:00:00+00:00".to_string()
    } else {
        config.not_valid_before.clone()
    };

    let not_after = if config.not_valid_after.is_empty() {
        "2025-01-01T00:00:00+00:00".to_string()
    } else {
        config.not_valid_after.clone()
    };

    // Read recipient certificate
    let recipient_cert = if config.recipient_cert_file.exists() {
        std::fs::read_to_string(&config.recipient_cert_file).unwrap_or_default()
    } else {
        String::new()
    };

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(
        "<DCinemaSecurityMessage xmlns=\"http://www.smpte-ra.org/schemas/430-3/2006/ETM\">\n",
    );
    xml.push_str("  <AuthenticatedPublic Id=\"AuthenticatedPublic\">\n");
    xml.push_str(&format!(
        "    <MessageId>urn:uuid:{message_id}</MessageId>\n"
    ));
    xml.push_str(
        "    <MessageType>http://www.smpte-ra.org/430-1/2006/KDM#kdm-key-type</MessageType>\n",
    );
    xml.push_str("    <AnnotationText>KDM for DCP Wizard</AnnotationText>\n");
    xml.push_str(&format!("    <IssueDate>{not_before}</IssueDate>\n"));
    xml.push_str("    <Signer>\n");
    xml.push_str("      <X509IssuerSerial>\n");
    xml.push_str("        <X509IssuerName>DCP Wizard</X509IssuerName>\n");
    xml.push_str("        <X509SerialNumber>1</X509SerialNumber>\n");
    xml.push_str("      </X509IssuerSerial>\n");
    xml.push_str("    </Signer>\n");
    xml.push_str("    <RequiredExtensions>\n");
    xml.push_str(
        "      <KDMRequiredExtensions xmlns=\"http://www.smpte-ra.org/schemas/430-1/2006/KDM\">\n",
    );
    xml.push_str("        <Recipient>\n");
    xml.push_str("          <X509IssuerSerial>\n");
    xml.push_str("            <X509IssuerName>Recipient</X509IssuerName>\n");
    xml.push_str("            <X509SerialNumber>1</X509SerialNumber>\n");
    xml.push_str("          </X509IssuerSerial>\n");
    if !recipient_cert.is_empty() {
        xml.push_str("          <X509SubjectName>Recipient</X509SubjectName>\n");
    }
    xml.push_str("        </Recipient>\n");
    xml.push_str(&format!(
        "        <CompositionPlaylistId>urn:uuid:{}</CompositionPlaylistId>\n",
        config.cpl_id
    ));
    xml.push_str(&format!(
        "        <ContentTitleText>{}</ContentTitleText>\n",
        escape_xml(&config.content_title)
    ));
    xml.push_str("        <ContentKeysNotValidBefore>");
    xml.push_str(&not_before);
    xml.push_str("</ContentKeysNotValidBefore>\n");
    xml.push_str("        <ContentKeysNotValidAfter>");
    xml.push_str(&not_after);
    xml.push_str("</ContentKeysNotValidAfter>\n");
    xml.push_str("        <KeyIdList>\n");

    for key in &config.content_keys {
        xml.push_str("          <TypedKeyPair>\n");
        xml.push_str(&format!(
            "            <KeyId>urn:uuid:{}</KeyId>\n",
            key.key_id
        ));
        xml.push_str(&format!(
            "            <KeyType>{}</KeyType>\n",
            escape_xml(&key.key_type)
        ));
        xml.push_str("          </TypedKeyPair>\n");
    }

    xml.push_str("        </KeyIdList>\n");
    xml.push_str("      </KDMRequiredExtensions>\n");
    xml.push_str("    </RequiredExtensions>\n");
    xml.push_str("  </AuthenticatedPublic>\n");
    xml.push_str("  <AuthenticatedPrivate>\n");

    for key in &config.content_keys {
        xml.push_str(&format!(
            "    <EncryptedKey xmlns=\"http://www.w3.org/2001/04/xmlenc#\">\n      <CipherData>\n        <CipherValue>{}</CipherValue>\n      </CipherData>\n    </EncryptedKey>\n",
            key.cipher_data
        ));
    }

    xml.push_str("  </AuthenticatedPrivate>\n");
    xml.push_str("</DCinemaSecurityMessage>\n");

    match std::fs::write(&config.output_file, &xml) {
        Ok(()) => {
            tracing::info!("Generated KDM: {}", config.output_file.display());
            0
        }
        Err(e) => {
            tracing::error!("Failed to write KDM: {e}");
            -1
        }
    }
}

/// Generate KDMs in batch for multiple recipients.
pub fn generate_kdm_batch(
    base_config: &KdmConfig,
    recipient_certs: &[PathBuf],
    output_dir: &Path,
) -> i32 {
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    let total = recipient_certs.len();
    tracing::info!("Generating {total} KDMs");

    for (i, cert_path) in recipient_certs.iter().enumerate() {
        let cert_stem = cert_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("recipient");

        let output_file = output_dir.join(format!(
            "KDM_{}_{}.xml",
            cert_stem,
            &base_config.cpl_id[..8.min(base_config.cpl_id.len())]
        ));

        let mut config = base_config.clone();
        config.recipient_cert_file = cert_path.clone();
        config.output_file = output_file;

        let result = generate_kdm(&config);
        if result != 0 {
            tracing::error!(
                "Failed to generate KDM for recipient [{}/{}]: {}",
                i + 1,
                total,
                cert_path.display()
            );
            return -1;
        }
    }

    tracing::info!("Generated {total} KDMs in {}", output_dir.display());
    0
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
