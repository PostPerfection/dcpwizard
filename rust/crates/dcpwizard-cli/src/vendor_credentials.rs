use postkit::kdm_distribution::certificate_fetch::Credentials;
use std::path::Path;

pub const VENDOR_PASSWORD_ENVIRONMENT_VARIABLE: &str = "DCPWIZARD_VENDOR_PASSWORD";

pub fn resolve_credentials(
    user: Option<String>,
    password_on_command_line: bool,
    password_file: Option<&Path>,
    environment_password: Option<String>,
) -> Result<Option<Credentials>, String> {
    if password_on_command_line {
        return Err(format!(
            "--password is refused because the command line is visible to other users: pass --password-file <path> or set {VENDOR_PASSWORD_ENVIRONMENT_VARIABLE}"
        ));
    }
    let Some(user) = user else {
        if password_file.is_some() {
            return Err("--password-file needs --user".to_string());
        }
        return Ok(None);
    };
    let password = match password_file {
        Some(path) => read_password_file(path)?,
        None => environment_password.ok_or_else(|| {
            format!(
                "--user needs a password: pass --password-file <path> or set {VENDOR_PASSWORD_ENVIRONMENT_VARIABLE}"
            )
        })?,
    };
    if password.is_empty() {
        return Err("the vendor password is empty".to_string());
    }
    Ok(Some(Credentials { user, password }))
}

fn read_password_file(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read password file {}: {e}", path.display()))?;
    Ok(text.lines().next().unwrap_or_default().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_password_file_first_line_wins_over_the_environment() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vendor-password");
        std::fs::write(&path, "from-file\r\nsecond line\n").unwrap();
        let credentials = resolve_credentials(
            Some("projectionist".into()),
            false,
            Some(&path),
            Some("from-environment".into()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(credentials.user, "projectionist");
        assert_eq!(credentials.password, "from-file");

        let credentials = resolve_credentials(
            Some("projectionist".into()),
            false,
            None,
            Some("from-environment".into()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(credentials.password, "from-environment");
    }

    #[test]
    fn a_user_without_a_password_source_is_refused_naming_both() {
        let error =
            resolve_credentials(Some("projectionist".into()), false, None, None).unwrap_err();
        assert!(error.contains("--password-file"), "{error}");
        assert!(
            error.contains(VENDOR_PASSWORD_ENVIRONMENT_VARIABLE),
            "{error}"
        );
    }

    #[test]
    fn the_password_argument_is_refused_naming_both_alternatives() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vendor-password");
        std::fs::write(&path, "from-file\n").unwrap();
        let error = resolve_credentials(
            Some("projectionist".into()),
            true,
            Some(&path),
            Some("from-environment".into()),
        )
        .unwrap_err();
        assert!(error.contains("--password-file"), "{error}");
        assert!(
            error.contains(VENDOR_PASSWORD_ENVIRONMENT_VARIABLE),
            "{error}"
        );
    }

    #[test]
    fn no_user_means_anonymous_and_a_password_file_alone_is_refused() {
        assert!(
            resolve_credentials(None, false, None, Some("from-environment".into()))
                .unwrap()
                .is_none()
        );
        let error =
            resolve_credentials(None, false, Some(Path::new("vendor-password")), None).unwrap_err();
        assert!(error.contains("--user"), "{error}");
    }
}
