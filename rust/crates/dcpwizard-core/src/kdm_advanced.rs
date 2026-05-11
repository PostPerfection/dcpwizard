use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Trusted device entry for advanced KDM generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub thumbprint: String,
    pub certificate_file: PathBuf,
    pub device_description: String,
}

/// Advanced KDM configuration with trusted device list and DKDM support.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KdmAdvancedConfig {
    pub cpl_id: String,
    pub content_title: String,
    pub not_valid_before: String,
    pub not_valid_after: String,
    pub signer_cert_chain: Vec<PathBuf>,
    pub signer_key_file: PathBuf,
    pub recipient_cert_file: PathBuf,
    pub trusted_devices: Vec<TrustedDevice>,
    pub forensic_mark_flag: bool,
    pub output_file: PathBuf,
}

/// Generate an advanced KDM with trusted device list and forensic marking support.
pub fn generate_kdm_advanced(config: &KdmAdvancedConfig) -> i32 {
    if config.cpl_id.is_empty() {
        tracing::error!("CPL ID is required for KDM generation");
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

    // Build signer cert chain
    let signer_certs: Vec<String> = config
        .signer_cert_chain
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();

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
    xml.push_str(&format!(
        "    <AnnotationText>{}</AnnotationText>\n",
        escape_xml(&config.content_title)
    ));
    xml.push_str(&format!("    <IssueDate>{not_before}</IssueDate>\n"));

    // Signer info
    xml.push_str("    <Signer>\n");
    xml.push_str("      <X509IssuerSerial>\n");
    xml.push_str("        <X509IssuerName>DCP Wizard CA</X509IssuerName>\n");
    xml.push_str("        <X509SerialNumber>1</X509SerialNumber>\n");
    xml.push_str("      </X509IssuerSerial>\n");
    xml.push_str("    </Signer>\n");

    xml.push_str("    <RequiredExtensions>\n");
    xml.push_str(
        "      <KDMRequiredExtensions xmlns=\"http://www.smpte-ra.org/schemas/430-1/2006/KDM\">\n",
    );

    // Recipient
    xml.push_str("        <Recipient>\n");
    xml.push_str("          <X509IssuerSerial>\n");
    xml.push_str("            <X509IssuerName>Recipient CA</X509IssuerName>\n");
    xml.push_str("            <X509SerialNumber>1</X509SerialNumber>\n");
    xml.push_str("          </X509IssuerSerial>\n");
    xml.push_str("        </Recipient>\n");

    xml.push_str(&format!(
        "        <CompositionPlaylistId>urn:uuid:{}</CompositionPlaylistId>\n",
        config.cpl_id
    ));
    xml.push_str(&format!(
        "        <ContentTitleText>{}</ContentTitleText>\n",
        escape_xml(&config.content_title)
    ));
    xml.push_str(&format!(
        "        <ContentKeysNotValidBefore>{not_before}</ContentKeysNotValidBefore>\n"
    ));
    xml.push_str(&format!(
        "        <ContentKeysNotValidAfter>{not_after}</ContentKeysNotValidAfter>\n"
    ));

    // Authorized device info
    if !config.trusted_devices.is_empty() {
        xml.push_str("        <AuthorizedDeviceInfo>\n");
        xml.push_str("          <DeviceList>\n");
        for device in &config.trusted_devices {
            xml.push_str("            <AuthorizedDevice>\n");
            xml.push_str(&format!(
                "              <DeviceGroupIdentifier>{}</DeviceGroupIdentifier>\n",
                escape_xml(&device.device_description)
            ));
            if !device.thumbprint.is_empty() {
                xml.push_str(&format!(
                    "              <CertificateThumbprint>{}</CertificateThumbprint>\n",
                    device.thumbprint
                ));
            }
            xml.push_str("            </AuthorizedDevice>\n");
        }
        xml.push_str("          </DeviceList>\n");
        xml.push_str("        </AuthorizedDeviceInfo>\n");
    }

    if config.forensic_mark_flag {
        xml.push_str("        <ForensicMarkFlagList>\n");
        xml.push_str("          <ForensicMarkFlag>http://www.smpte-ra.org/430-1/2006/KDM#mrkflg-picture-disable</ForensicMarkFlag>\n");
        xml.push_str("          <ForensicMarkFlag>http://www.smpte-ra.org/430-1/2006/KDM#mrkflg-audio-disable</ForensicMarkFlag>\n");
        xml.push_str("        </ForensicMarkFlagList>\n");
    }

    xml.push_str("      </KDMRequiredExtensions>\n");
    xml.push_str("    </RequiredExtensions>\n");
    xml.push_str("  </AuthenticatedPublic>\n");

    // Signer cert chain
    if !signer_certs.is_empty() {
        xml.push_str("  <Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\">\n");
        xml.push_str("    <KeyInfo>\n");
        xml.push_str("      <X509Data>\n");
        for cert in &signer_certs {
            let cert_body = extract_pem_body(cert);
            xml.push_str(&format!(
                "        <X509Certificate>{cert_body}</X509Certificate>\n"
            ));
        }
        xml.push_str("      </X509Data>\n");
        xml.push_str("    </KeyInfo>\n");
        xml.push_str("  </Signature>\n");
    }

    xml.push_str("  <AuthenticatedPrivate/>\n");
    xml.push_str("</DCinemaSecurityMessage>\n");

    match std::fs::write(&config.output_file, &xml) {
        Ok(()) => {
            tracing::info!("Generated advanced KDM: {}", config.output_file.display());
            0
        }
        Err(e) => {
            tracing::error!("Failed to write KDM: {e}");
            -1
        }
    }
}

/// Generate a KDM from a DKDM (Distribution KDM).
///
/// A DKDM is a KDM addressed to the content creator's own certificate,
/// used to re-generate target-specific KDMs without needing the original keys.
pub fn kdm_from_dkdm(
    dkdm_file: &Path,
    recipient_cert: &Path,
    not_valid_before: &str,
    not_valid_after: &str,
    output_file: &Path,
) -> i32 {
    let dkdm_content = match std::fs::read_to_string(dkdm_file) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to read DKDM: {e}");
            return -1;
        }
    };

    // Extract CPL ID from DKDM
    let cpl_id = extract_xml_value(&dkdm_content, "CompositionPlaylistId")
        .unwrap_or_default()
        .replace("urn:uuid:", "");

    if cpl_id.is_empty() {
        tracing::error!("Could not extract CPL ID from DKDM");
        return -1;
    }

    let content_title =
        extract_xml_value(&dkdm_content, "ContentTitleText").unwrap_or_else(|| "Unknown".into());

    // Extract encrypted keys from DKDM
    let mut cipher_values = Vec::new();
    let mut search_from = 0usize;
    while let Some(start) = dkdm_content[search_from..].find("<CipherValue>") {
        let abs_start = search_from + start + "<CipherValue>".len();
        if let Some(end) = dkdm_content[abs_start..].find("</CipherValue>") {
            let value = dkdm_content[abs_start..abs_start + end].trim().to_string();
            cipher_values.push(value);
            search_from = abs_start + end;
        } else {
            break;
        }
    }

    let content_keys: Vec<crate::kdm::ContentKey> = cipher_values
        .iter()
        .enumerate()
        .map(|(i, cv)| crate::kdm::ContentKey {
            key_id: uuid::Uuid::new_v4().to_string(),
            key_type: if i == 0 { "MDIK".into() } else { "MDAK".into() },
            cipher_data: cv.clone(),
        })
        .collect();

    let config = crate::kdm::KdmConfig {
        cpl_id,
        content_title,
        not_valid_before: not_valid_before.to_string(),
        not_valid_after: not_valid_after.to_string(),
        recipient_cert_file: recipient_cert.to_path_buf(),
        content_keys,
        output_file: output_file.to_path_buf(),
        ..Default::default()
    };

    crate::kdm::generate_kdm(&config)
}

fn extract_xml_value(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start_pos = text.find(&open)?;
    let after_open = &text[start_pos + open.len()..];
    let content_start = after_open.find('>')?;
    let content = &after_open[content_start + 1..];
    let end_pos = content.find(&close)?;
    let value = content[..end_pos].trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn extract_pem_body(pem: &str) -> String {
    pem.lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
