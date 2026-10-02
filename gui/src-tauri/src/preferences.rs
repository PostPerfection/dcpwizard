use dcpwizard_core::preferences::Preferences;
use serde::Serialize;
use std::path::Path;

#[tauri::command]
pub fn load_preferences() -> Result<Preferences, String> {
    dcpwizard_core::preferences::load_preferences().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_preferences(preferences: Preferences) -> Result<(), String> {
    dcpwizard_core::preferences::save_preferences(&preferences).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reset_preferences() -> Result<Preferences, String> {
    dcpwizard_core::preferences::reset_preferences().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_recipient_certificate(
    certificate: String,
    destination: String,
) -> Result<(), String> {
    dcpwizard_core::certificate::export_certificate(
        Path::new(&certificate),
        Path::new(&destination),
    )
}

#[tauri::command]
pub fn find_dcpomatic_config() -> Result<String, String> {
    dcpwizard_core::dcpomatic_identity::find_dcpomatic_config()
        .map(|config| config.display().to_string())
}

#[derive(Serialize)]
pub struct ImportedDcpomaticIdentity {
    preferences: Preferences,
    thumbprint: String,
}

#[tauri::command]
pub fn import_dcpomatic_identity(config: String) -> Result<ImportedDcpomaticIdentity, String> {
    let (preferences, identity) =
        dcpwizard_core::dcpomatic_identity::import_dcpomatic_identity_into_preferences(Some(
            Path::new(&config),
        ))?;
    Ok(ImportedDcpomaticIdentity {
        preferences,
        thumbprint: identity.thumbprint,
    })
}
