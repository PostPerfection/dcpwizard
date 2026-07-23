//! kdm email delivery (dom#2515, dom#2516, dom#3076). builds a message with the
//! KDMs zipped as one attachment (one email per cinema, dom#2516) and sends it
//! over SMTP via lettre. the smtp config file holds the password: it is never
//! logged, never echoed in errors, and Debug redacts it.

use lettre::message::{Attachment, MultiPart, SinglePart, header};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::Deserialize;
use std::fmt;
use std::io::Write;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Security {
    /// implicit TLS (smtps, usually port 465).
    #[default]
    Tls,
    /// upgrade a plaintext connection with STARTTLS (usually port 587).
    Starttls,
    /// no transport encryption (test servers only).
    None,
}

#[derive(Clone, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub security: Security,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    pub from: String,
    #[serde(default)]
    pub subject_template: Option<String>,
    #[serde(default)]
    pub body_template: Option<String>,
}

// redact the password so it can never leak through Debug/log output.
impl fmt::Debug for SmtpConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmtpConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("security", &self.security)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("from", &self.from)
            .finish()
    }
}

impl SmtpConfig {
    pub fn from_toml(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| format!("invalid smtp config: {e}"))
    }

    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read smtp config {}: {e}", path.display()))?;
        Self::from_toml(&text)
    }
}

/// substitute {title} and {cinema} tokens in a subject/body template (dom#3076).
pub fn substitute(template: &str, title: &str, cinema: &str) -> String {
    template
        .replace("{title}", title)
        .replace("{cinema}", cinema)
}

/// zip a set of (filename, bytes) into an in-memory zip (dom#2516: all of a
/// cinema's KDMs in one attachment).
pub fn zip_files(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in files {
            zip.start_file(name, opts)
                .map_err(|e| format!("zip entry '{name}': {e}"))?;
            zip.write_all(bytes)
                .map_err(|e| format!("zip write '{name}': {e}"))?;
        }
        zip.finish().map_err(|e| format!("finish zip: {e}"))?;
    }
    Ok(buf.into_inner())
}

/// build a MIME message: a plain-text body plus one attachment.
pub fn build_message(
    from: &str,
    to: &[String],
    subject: &str,
    body: &str,
    attachment_name: &str,
    attachment_bytes: Vec<u8>,
    attachment_mime: &str,
) -> Result<Message, String> {
    if to.is_empty() {
        return Err("no recipient email addresses".to_string());
    }
    let mut builder = Message::builder()
        .from(
            from.parse()
                .map_err(|e| format!("invalid from address '{from}': {e}"))?,
        )
        .subject(subject);
    for addr in to {
        builder = builder.to(addr
            .parse()
            .map_err(|e| format!("invalid recipient '{addr}': {e}"))?);
    }
    let ctype = header::ContentType::parse(attachment_mime)
        .map_err(|e| format!("bad attachment mime '{attachment_mime}': {e}"))?;
    let part = MultiPart::mixed()
        .singlepart(SinglePart::plain(body.to_string()))
        .singlepart(Attachment::new(attachment_name.to_string()).body(attachment_bytes, ctype));
    builder
        .multipart(part)
        .map_err(|e| format!("build message: {e}"))
}

/// assemble the KDM delivery email for one cinema: zip the KDM files and build a
/// message with the config's subject/body templates ({title}/{cinema}). no
/// network I/O, so this is unit-tested.
pub fn build_kdm_email(
    config: &SmtpConfig,
    cinema: &str,
    title: &str,
    to: &[String],
    kdm_files: &[std::path::PathBuf],
) -> Result<Message, String> {
    let mut entries = Vec::new();
    for f in kdm_files {
        let bytes = std::fs::read(f).map_err(|e| format!("cannot read {}: {e}", f.display()))?;
        let name = f
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("kdm.xml")
            .to_string();
        entries.push((name, bytes));
    }
    let zip = zip_files(&entries)?;
    let subject = config
        .subject_template
        .as_deref()
        .map(|t| substitute(t, title, cinema))
        .unwrap_or_else(|| format!("KDM(s) for {title}"));
    let body = config
        .body_template
        .as_deref()
        .map(|t| substitute(t, title, cinema))
        .unwrap_or_else(|| format!("Attached are the KDM(s) for \"{title}\"."));
    build_message(
        &config.from,
        to,
        &subject,
        &body,
        "kdms.zip",
        zip,
        "application/zip",
    )
}

/// build and send one cinema's KDM email. performs network I/O.
pub fn send_kdms(
    config: &SmtpConfig,
    cinema: &str,
    title: &str,
    to: &[String],
    kdm_files: &[std::path::PathBuf],
) -> Result<(), String> {
    let msg = build_kdm_email(config, cinema, title, to, kdm_files)?;
    send(config, &msg)
}

/// send a message over SMTP. performs real network I/O; not unit-tested.
pub fn send(config: &SmtpConfig, message: &Message) -> Result<(), String> {
    let mut builder = match config.security {
        Security::Tls => {
            SmtpTransport::relay(&config.host).map_err(|e| format!("smtp tls setup: {e}"))?
        }
        Security::Starttls => SmtpTransport::starttls_relay(&config.host)
            .map_err(|e| format!("smtp starttls setup: {e}"))?,
        Security::None => SmtpTransport::builder_dangerous(&config.host),
    }
    .port(config.port);

    if let (Some(u), Some(p)) = (&config.username, &config.password) {
        builder = builder.credentials(Credentials::new(u.clone(), p.clone()));
    }
    let mailer = builder.build();
    // map only the high-level failure; the error type does not print the
    // password, but we still summarize rather than forward blindly.
    mailer
        .send(message)
        .map(|_| ())
        .map_err(|e| format!("smtp send to {} failed: {e}", config.host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parses_and_debug_redacts_password() {
        let cfg = SmtpConfig::from_toml(
            r#"
            host = "smtp.example.test"
            port = 587
            security = "starttls"
            username = "user@example.test"
            password = "hunter2"
            from = "kdm@example.test"
            subject_template = "KDM for {title}"
            "#,
        )
        .unwrap();
        assert_eq!(cfg.port, 587);
        assert_eq!(cfg.security, Security::Starttls);
        let dbg = format!("{cfg:?}");
        assert!(
            !dbg.contains("hunter2"),
            "password must not appear in Debug"
        );
        assert!(dbg.contains("<redacted>"));
    }

    #[test]
    fn substitution_fills_tokens() {
        assert_eq!(
            substitute("KDM for {title} at {cinema}", "Feature", "Odeon"),
            "KDM for Feature at Odeon"
        );
    }

    #[test]
    fn zip_contains_all_entries() {
        let files = vec![
            ("a.kdm.xml".to_string(), b"<kdm-a/>".to_vec()),
            ("b.kdm.xml".to_string(), b"<kdm-b/>".to_vec()),
        ];
        let bytes = zip_files(&files).unwrap();
        // read it back to prove both entries are present with their content
        let mut zr = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(zr.len(), 2);
        use std::io::Read;
        let mut s = String::new();
        zr.by_name("a.kdm.xml")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        assert_eq!(s, "<kdm-a/>");
    }

    #[test]
    fn message_has_headers_and_attachment() {
        let msg = build_message(
            "kdm@example.test",
            &["a@cinema.test".into(), "b@cinema.test".into()],
            "KDM for Feature",
            "See attached.",
            "kdms.zip",
            b"PK\x03\x04zipbytes".to_vec(),
            "application/zip",
        )
        .unwrap();
        let out = String::from_utf8_lossy(&msg.formatted()).to_string();
        assert!(out.contains("Subject: KDM for Feature"));
        assert!(out.contains("a@cinema.test"));
        assert!(out.contains("b@cinema.test"));
        assert!(out.contains("kdms.zip"), "attachment filename present");
    }

    #[test]
    fn kdm_email_zips_files_and_applies_templates() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("001_screen.kdm.xml");
        std::fs::write(&a, b"<kdm/>").unwrap();
        let cfg = SmtpConfig::from_toml(
            r#"
            host = "smtp.test"
            port = 465
            from = "kdm@dist.test"
            subject_template = "Keys for {title} at {cinema}"
            "#,
        )
        .unwrap();
        let msg =
            build_kdm_email(&cfg, "Odeon", "Big Feature", &["a@odeon.test".into()], &[a]).unwrap();
        let out = String::from_utf8_lossy(&msg.formatted()).to_string();
        assert!(out.contains("Subject: Keys for Big Feature at Odeon"));
        assert!(out.contains("kdms.zip"));
    }

    #[test]
    fn message_requires_a_recipient() {
        let r = build_message(
            "kdm@example.test",
            &[],
            "s",
            "b",
            "f.zip",
            vec![1, 2, 3],
            "application/zip",
        );
        assert!(r.is_err());
    }
}
