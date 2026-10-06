// the smtp config file is dcpwizard's, the email itself is postkit's
use postkit::kdm_distribution::email::SmtpConfig;

const MISSING_FIELD_MESSAGE_PREFIX: &str = "missing field";
const VALUE_REFUSED_DESCRIPTION: &str = "a value has the wrong type or is out of range";

// the error names the line, never the value, so a password cannot reach a log
pub fn smtp_config_from_toml(text: &str) -> Result<SmtpConfig, String> {
    toml::from_str(text).map_err(|e| {
        format!(
            "invalid smtp config: {}",
            toml_error_without_values(text, &e)
        )
    })
}

pub fn load_smtp_config(path: &std::path::Path) -> Result<SmtpConfig, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read smtp config {}: {e}", path.display()))?;
    smtp_config_from_toml(&text)
}

pub(crate) fn toml_error_without_values(text: &str, error: &toml::de::Error) -> String {
    let message = error.message();
    if message.starts_with(MISSING_FIELD_MESSAGE_PREFIX) {
        return message.to_string();
    }
    // syntax error messages are fixed parser text, type error messages quote the value
    let is_syntax_error = text.parse::<toml::Table>().is_err();
    let description = if is_syntax_error {
        message
    } else {
        VALUE_REFUSED_DESCRIPTION
    };
    match error.span() {
        Some(span) => format!("line {}: {description}", line_number(text, span.start)),
        None => description.to_string(),
    }
}

fn line_number(text: &str, byte_offset: usize) -> usize {
    text.bytes()
        .take(byte_offset)
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_malformed_password_line_is_named_by_line_without_its_value() {
        for password_line in ["password = hunter2", "password = 1234"] {
            let text = format!(
                "host = \"smtp.example.test\"\nport = 587\n{password_line}\nfrom = \"kdm@example.test\"\n"
            );
            let error = smtp_config_from_toml(&text).unwrap_err();
            assert!(!error.contains("hunter2"), "{error}");
            assert!(!error.contains("1234"), "{error}");
            assert!(error.contains("line 3"), "{error}");
        }
    }

    #[test]
    fn a_missing_key_is_named() {
        let error =
            smtp_config_from_toml("host = \"smtp.example.test\"\nport = 587\n").unwrap_err();
        assert!(error.contains("missing field `from`"), "{error}");
    }
}
