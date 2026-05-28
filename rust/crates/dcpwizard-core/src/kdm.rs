use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// KDM formulation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum KdmFormulation {
    /// Modified Transitional 1 (most common)
    #[default]
    ModifiedTransitional1,
    /// DCI Any (less restrictive)
    DciAny,
    /// DCI Specific (most restrictive)
    DciSpecific,
}

impl KdmFormulation {
    /// Parse from CLI string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().replace(' ', "-").as_str() {
            "dci-any" => Self::DciAny,
            "dci-specific" => Self::DciSpecific,
            _ => Self::ModifiedTransitional1,
        }
    }

    /// Returns the SMPTE formulation URI for the KDM.
    pub fn as_kdm_type_uri(&self) -> &'static str {
        match self {
            Self::ModifiedTransitional1 => "http://www.smpte-ra.org/430-1/2006/KDM#kdm-key-type",
            Self::DciAny => "http://www.smpte-ra.org/430-1/2006/KDM#kdm-key-type",
            Self::DciSpecific => "http://www.smpte-ra.org/430-1/2006/KDM#kdm-key-type",
        }
    }
}

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
    pub formulation: KdmFormulation,
    pub disable_forensic_marking_picture: bool,
    pub disable_forensic_marking_audio: bool,
}

/// A content key entry within a KDM.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentKey {
    pub key_id: String,
    pub key_type: String,
    pub cipher_data: String,
}

/// Parse a human-friendly duration string like "2 weeks", "30 days", "4 hours".
pub fn parse_duration_string(s: &str) -> Option<chrono::Duration> {
    let s = s.trim().to_lowercase();
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        // Try single-word like "24h" or "7d"
        if let Some(stripped) = s.strip_suffix('h') {
            let hours: i64 = stripped.parse().ok()?;
            return Some(chrono::Duration::hours(hours));
        }
        if let Some(stripped) = s.strip_suffix('d') {
            let days: i64 = stripped.parse().ok()?;
            return Some(chrono::Duration::days(days));
        }
        if let Some(stripped) = s.strip_suffix('w') {
            let weeks: i64 = stripped.parse().ok()?;
            return Some(chrono::Duration::weeks(weeks));
        }
        return None;
    }

    let num: i64 = parts[0].parse().ok()?;
    match parts[1].trim_end_matches('s') {
        "second" => Some(chrono::Duration::seconds(num)),
        "minute" => Some(chrono::Duration::minutes(num)),
        "hour" => Some(chrono::Duration::hours(num)),
        "day" => Some(chrono::Duration::days(num)),
        "week" => Some(chrono::Duration::weeks(num)),
        "month" => Some(chrono::Duration::days(num * 30)),
        "year" => Some(chrono::Duration::days(num * 365)),
        _ => None,
    }
}

/// Resolve validity period from CLI arguments.
/// Returns (not_valid_before, not_valid_after) as ISO 8601 strings.
pub fn resolve_validity_period(
    valid_from: Option<&str>,
    valid_to: Option<&str>,
    valid_duration: Option<&str>,
) -> (String, String) {
    use chrono::Utc;

    let now = Utc::now();

    let from = match valid_from {
        Some("now") | None => now,
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| {
                // Try date-only format
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
                    .unwrap_or(now)
            }),
    };

    let to = if let Some(to_str) = valid_to {
        chrono::DateTime::parse_from_rfc3339(to_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| {
                chrono::NaiveDate::parse_from_str(to_str, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc())
                    .unwrap_or(from + chrono::Duration::weeks(2))
            })
    } else if let Some(dur_str) = valid_duration {
        match parse_duration_string(dur_str) {
            Some(d) => from + d,
            None => {
                tracing::warn!("Could not parse duration '{dur_str}', defaulting to 2 weeks");
                from + chrono::Duration::weeks(2)
            }
        }
    } else {
        // Default: 2 weeks from start
        from + chrono::Duration::weeks(2)
    };

    (from.to_rfc3339(), to.to_rfc3339())
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

    // Forensic marking flags
    if config.disable_forensic_marking_picture || config.disable_forensic_marking_audio {
        xml.push_str("        <ForensicMarkFlagList>\n");
        if config.disable_forensic_marking_picture {
            xml.push_str(
                "          <ForensicMarkFlag>http://www.smpte-ra.org/430-1/2006/KDM#mrkflg-picture-disable</ForensicMarkFlag>\n",
            );
        }
        if config.disable_forensic_marking_audio {
            xml.push_str(
                "          <ForensicMarkFlag>http://www.smpte-ra.org/430-1/2006/KDM#mrkflg-audio-disable</ForensicMarkFlag>\n",
            );
        }
        xml.push_str("        </ForensicMarkFlagList>\n");
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_days() {
        let d = parse_duration_string("30 days").unwrap();
        assert_eq!(d.num_days(), 30);
    }

    #[test]
    fn test_parse_duration_weeks() {
        let d = parse_duration_string("2 weeks").unwrap();
        assert_eq!(d.num_days(), 14);
    }

    #[test]
    fn test_parse_duration_hours() {
        let d = parse_duration_string("4 hours").unwrap();
        assert_eq!(d.num_hours(), 4);
    }

    #[test]
    fn test_parse_duration_shorthand() {
        assert_eq!(parse_duration_string("7d").unwrap().num_days(), 7);
        assert_eq!(parse_duration_string("24h").unwrap().num_hours(), 24);
        assert_eq!(parse_duration_string("1w").unwrap().num_days(), 7);
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration_string("bogus").is_none());
        assert!(parse_duration_string("").is_none());
    }

    #[test]
    fn test_resolve_validity_now_with_duration() {
        let (from, to) = resolve_validity_period(Some("now"), None, Some("7 days"));
        // Should parse as valid RFC3339
        let from_dt = chrono::DateTime::parse_from_rfc3339(&from).unwrap();
        let to_dt = chrono::DateTime::parse_from_rfc3339(&to).unwrap();
        let diff = to_dt - from_dt;
        assert_eq!(diff.num_days(), 7);
    }

    #[test]
    fn test_resolve_validity_explicit_dates() {
        let (from, to) = resolve_validity_period(
            Some("2024-06-01T00:00:00+00:00"),
            Some("2024-06-15T23:59:59+00:00"),
            None,
        );
        assert!(from.contains("2024-06-01"));
        assert!(to.contains("2024-06-15"));
    }

    #[test]
    fn test_resolve_validity_date_only() {
        let (from, to) = resolve_validity_period(Some("2024-03-01"), Some("2024-03-31"), None);
        assert!(from.contains("2024-03-01"));
        assert!(to.contains("2024-03-31"));
    }

    #[test]
    fn test_resolve_validity_defaults_two_weeks() {
        let (from, to) = resolve_validity_period(None, None, None);
        let from_dt = chrono::DateTime::parse_from_rfc3339(&from).unwrap();
        let to_dt = chrono::DateTime::parse_from_rfc3339(&to).unwrap();
        let diff = to_dt - from_dt;
        assert_eq!(diff.num_days(), 14);
    }

    #[test]
    fn test_formulation_parsing() {
        assert_eq!(
            KdmFormulation::parse("modified-transitional-1"),
            KdmFormulation::ModifiedTransitional1
        );
        assert_eq!(KdmFormulation::parse("dci-any"), KdmFormulation::DciAny);
        assert_eq!(
            KdmFormulation::parse("dci-specific"),
            KdmFormulation::DciSpecific
        );
        // Unknown falls back to default
        assert_eq!(
            KdmFormulation::parse("unknown"),
            KdmFormulation::ModifiedTransitional1
        );
    }

    #[test]
    fn test_generate_kdm_with_forensic_marking() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let config = KdmConfig {
            cpl_id: "urn:uuid:12345678-1234-1234-1234-123456789abc".into(),
            content_title: "Test Film".into(),
            not_valid_before: "2024-01-01T00:00:00+00:00".into(),
            not_valid_after: "2024-12-31T23:59:59+00:00".into(),
            recipient_cert_file: PathBuf::from("/dev/null"),
            output_file: tmp.path().to_path_buf(),
            disable_forensic_marking_picture: true,
            disable_forensic_marking_audio: false,
            content_keys: vec![ContentKey {
                key_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
                key_type: "MDIK".into(),
                cipher_data: "dGVzdA==".into(),
            }],
            ..Default::default()
        };

        let result = generate_kdm(&config);
        assert_eq!(result, 0);

        let xml = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(xml.contains("mrkflg-picture-disable"));
        assert!(!xml.contains("mrkflg-audio-disable"));
    }

    #[test]
    fn test_generate_kdm_validity_dates_in_output() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let config = KdmConfig {
            cpl_id: "test-cpl-id".into(),
            content_title: "Date Test".into(),
            not_valid_before: "2024-06-01T10:00:00+00:00".into(),
            not_valid_after: "2024-06-30T22:00:00+00:00".into(),
            recipient_cert_file: PathBuf::from("/dev/null"),
            output_file: tmp.path().to_path_buf(),
            content_keys: vec![ContentKey {
                key_id: "11111111-2222-3333-4444-555555555555".into(),
                key_type: "MDIK".into(),
                cipher_data: "YWJj".into(),
            }],
            ..Default::default()
        };

        let result = generate_kdm(&config);
        assert_eq!(result, 0);

        let xml = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(xml.contains("2024-06-01T10:00:00+00:00"));
        assert!(xml.contains("2024-06-30T22:00:00+00:00"));
    }
}
