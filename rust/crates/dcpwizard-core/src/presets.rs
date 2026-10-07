use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub const CURRENT_PRESETS_VERSION: u32 = 1;
const PRESETS_FILE_NAME: &str = "presets.json";

// form values keyed like the GUI project file's form object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub name: String,
    pub form: BTreeMap<String, Value>,
    pub audio_map: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct PresetsFile {
    version: u32,
    presets: Vec<Preset>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedPresets {
    pub presets: Vec<Preset>,
    pub imported: Vec<String>,
    pub replaced: Vec<String>,
}

pub fn presets_path() -> PathBuf {
    crate::preferences::preferences_directory().join(PRESETS_FILE_NAME)
}

fn invalid_data(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read_presets_file(path: &Path) -> io::Result<Option<Vec<Preset>>> {
    let Some(contents) = postkit::preferences::read_preferences_file(path)? else {
        return Ok(None);
    };
    let file: PresetsFile = serde_json::from_str(&contents)
        .map_err(|error| invalid_data(format!("{}: {error}", path.display())))?;
    if file.version > CURRENT_PRESETS_VERSION {
        return Err(invalid_data(format!(
            "{}: presets file version {} is newer than supported version {CURRENT_PRESETS_VERSION}",
            path.display(),
            file.version
        )));
    }
    Ok(Some(file.presets))
}

fn write_presets_file(path: &Path, presets: &[Preset]) -> io::Result<()> {
    let file = PresetsFile {
        version: CURRENT_PRESETS_VERSION,
        presets: presets.to_vec(),
    };
    let contents = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
    postkit::preferences::write_preferences_file(path, &contents)
}

pub fn load_presets(path: &Path) -> io::Result<Vec<Preset>> {
    Ok(read_presets_file(path)?.unwrap_or_default())
}

// the GUI lists saved presets in the same select as the built-in profiles
fn built_in_profile_names<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    names
        .filter(|name| crate::profiles::get_profile(name).is_some())
        .map(str::to_string)
        .collect()
}

pub fn save_preset(path: &Path, preset: Preset) -> io::Result<Vec<Preset>> {
    if preset.name.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a preset needs a name",
        ));
    }
    if crate::profiles::get_profile(&preset.name).is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is a built-in delivery profile, choose another name",
                preset.name
            ),
        ));
    }
    let mut presets = load_presets(path)?;
    match presets.iter_mut().find(|saved| saved.name == preset.name) {
        Some(saved) => *saved = preset,
        None => presets.push(preset),
    }
    write_presets_file(path, &presets)?;
    Ok(presets)
}

pub fn delete_preset(path: &Path, name: &str) -> io::Result<Vec<Preset>> {
    let mut presets = load_presets(path)?;
    let count_before = presets.len();
    presets.retain(|preset| preset.name != name);
    if presets.len() == count_before {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no preset named {name}"),
        ));
    }
    write_presets_file(path, &presets)?;
    Ok(presets)
}

pub fn export_presets(path: &Path, destination: &Path) -> io::Result<()> {
    write_presets_file(destination, &load_presets(path)?)
}

pub fn import_presets(path: &Path, source: &Path) -> io::Result<ImportedPresets> {
    let incoming = read_presets_file(source)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} does not exist", source.display()),
        )
    })?;
    let clashing = built_in_profile_names(incoming.iter().map(|preset| preset.name.as_str()));
    if !clashing.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} holds presets named like built-in delivery profiles: {}",
                source.display(),
                clashing.join(", ")
            ),
        ));
    }
    let mut presets = load_presets(path)?;
    let mut imported = Vec::new();
    let mut replaced = Vec::new();
    for preset in incoming {
        imported.push(preset.name.clone());
        match presets.iter_mut().find(|saved| saved.name == preset.name) {
            Some(saved) => {
                replaced.push(preset.name.clone());
                *saved = preset;
            }
            None => presets.push(preset),
        }
    }
    write_presets_file(path, &presets)?;
    Ok(ImportedPresets {
        presets,
        imported,
        replaced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn preset(name: &str, standard: &str) -> Preset {
        Preset {
            name: name.to_string(),
            form: BTreeMap::from([
                ("standard".to_string(), json!(standard)),
                ("encrypt".to_string(), json!(true)),
            ]),
            audio_map: Some("1:L,2:R@-3".to_string()),
        }
    }

    #[test]
    fn a_saved_preset_loads_back_and_a_second_save_replaces_it() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(PRESETS_FILE_NAME);

        save_preset(&path, preset("Festival", "smpte")).unwrap();
        save_preset(&path, preset("Trailer", "interop")).unwrap();
        save_preset(&path, preset("Festival", "interop")).unwrap();

        assert_eq!(
            load_presets(&path).unwrap(),
            vec![preset("Festival", "interop"), preset("Trailer", "interop")]
        );
    }

    #[test]
    fn a_missing_file_holds_no_presets() {
        let directory = TempDir::new().unwrap();

        assert_eq!(
            load_presets(&directory.path().join(PRESETS_FILE_NAME)).unwrap(),
            vec![]
        );
    }

    #[test]
    fn the_file_carries_the_version() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(PRESETS_FILE_NAME);

        save_preset(&path, preset("Festival", "smpte")).unwrap();

        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["version"], CURRENT_PRESETS_VERSION);
        assert_eq!(saved["presets"][0]["audioMap"], "1:L,2:R@-3");
    }

    #[test]
    fn a_newer_file_is_refused_and_left_alone() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(PRESETS_FILE_NAME);
        let contents = r#"{"version":99,"presets":[]}"#;
        std::fs::write(&path, contents).unwrap();

        let error = load_presets(&path).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("version 99"), "{error}");
        assert!(save_preset(&path, preset("Festival", "smpte")).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
    }

    #[test]
    fn a_file_without_a_version_is_refused() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(PRESETS_FILE_NAME);
        std::fs::write(&path, r#"{"presets":[]}"#).unwrap();

        assert_eq!(
            load_presets(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn deleting_removes_only_that_preset() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(PRESETS_FILE_NAME);
        save_preset(&path, preset("Festival", "smpte")).unwrap();
        save_preset(&path, preset("Trailer", "interop")).unwrap();

        let left = delete_preset(&path, "Festival").unwrap();

        assert_eq!(left, vec![preset("Trailer", "interop")]);
        assert_eq!(load_presets(&path).unwrap(), left);
        assert_eq!(
            delete_preset(&path, "Festival").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn import_adds_new_presets_and_replaces_same_named_ones() {
        let directory = TempDir::new().unwrap();
        let here = directory.path().join("here").join(PRESETS_FILE_NAME);
        let there = directory.path().join("there").join(PRESETS_FILE_NAME);
        let exported = directory.path().join("exported.json");
        save_preset(&here, preset("Festival", "smpte")).unwrap();
        save_preset(&here, preset("Trailer", "smpte")).unwrap();
        save_preset(&there, preset("Festival", "interop")).unwrap();
        save_preset(&there, preset("Advert", "interop")).unwrap();

        export_presets(&there, &exported).unwrap();
        let result = import_presets(&here, &exported).unwrap();

        let expected = vec![
            preset("Festival", "interop"),
            preset("Trailer", "smpte"),
            preset("Advert", "interop"),
        ];
        assert_eq!(result.presets, expected);
        assert_eq!(result.imported, vec!["Festival", "Advert"]);
        assert_eq!(result.replaced, vec!["Festival"]);
        assert_eq!(load_presets(&here).unwrap(), expected);
    }

    #[test]
    fn an_import_from_a_newer_version_changes_nothing() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(PRESETS_FILE_NAME);
        let source = directory.path().join("future.json");
        save_preset(&path, preset("Festival", "smpte")).unwrap();
        std::fs::write(&source, r#"{"version":2,"presets":[]}"#).unwrap();

        assert!(import_presets(&path, &source).is_err());
        assert_eq!(
            load_presets(&path).unwrap(),
            vec![preset("Festival", "smpte")]
        );
    }

    #[test]
    fn a_built_in_profile_name_is_refused() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(PRESETS_FILE_NAME);
        let built_in = crate::profiles::all_profiles()[0].name.clone();

        let error = save_preset(&path, preset(&built_in, "smpte")).unwrap_err();

        assert!(error.to_string().contains(&built_in), "{error}");
        assert!(!path.exists());
    }
}
