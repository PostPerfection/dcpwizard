use dcpwizard_core::presets::{self, ImportedPresets, Preset};
use std::path::Path;

#[tauri::command]
pub fn list_presets() -> Result<Vec<Preset>, String> {
    presets::load_presets(&presets::presets_path()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_preset(preset: Preset) -> Result<Vec<Preset>, String> {
    presets::save_preset(&presets::presets_path(), preset).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_preset(name: String) -> Result<Vec<Preset>, String> {
    presets::delete_preset(&presets::presets_path(), &name).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_presets(destination: String) -> Result<(), String> {
    presets::export_presets(&presets::presets_path(), Path::new(&destination))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_presets(source: String) -> Result<ImportedPresets, String> {
    presets::import_presets(&presets::presets_path(), Path::new(&source))
        .map_err(|error| error.to_string())
}
