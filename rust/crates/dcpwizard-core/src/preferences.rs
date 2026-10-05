use postkit::preferences::PrefsMigration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub const CURRENT_PREFERENCES_VERSION: u32 = 2;
const FIRST_MIGRATION_VERSION: u32 = 2;
pub const DEFAULT_GPU_REGISTRATION_URL: &str = "https://grokcompression.com/api/register";
pub const AUTOMATIC_ENCODE_THREADS: u32 = 0;
const DEFAULT_BANDWIDTH_MBPS: u32 = 230;
const SNAKE_CASE_RENAMES: [(&str, &str); 10] = [
    ("default_standard", "standard"),
    ("default_resolution", "resolution"),
    ("default_frame_rate", "framerate"),
    ("creator_name", "creator"),
    ("isdcf_facility_code", "facility"),
    ("default_bandwidth_mbps", "bandwidth"),
    ("signing_certificate_path", "signingCert"),
    ("signing_key_path", "signingKey"),
    ("default_output_dir", "outputDir"),
    ("default_channel_config", "channels"),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub version: u32,
    pub standard: String,
    pub resolution: String,
    pub framerate: u32,
    pub encrypt: bool,
    pub stereo3d: bool,
    pub validate: bool,
    pub creator: String,
    pub facility: String,
    pub bandwidth: u32,
    pub gpu: bool,
    pub gpu_license: String,
    pub gpu_registration_url: String,
    pub encode_threads: u32,
    pub detect_picture_findings: bool,
    pub signing_cert: String,
    pub signing_key: String,
    pub signing_intermediate: String,
    // signs a new recipient certificate
    pub signing_intermediate_key: String,
    pub signing_root: String,
    pub recipient_cert: String,
    pub recipient_key: String,
    pub output_dir: String,
    pub isdcf_naming: bool,
    pub channels: String,
    pub show_hints_before_build: bool,
    #[serde(flatten)]
    pub additional: BTreeMap<String, serde_json::Value>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: CURRENT_PREFERENCES_VERSION,
            standard: "SMPTE".to_string(),
            resolution: "2K".to_string(),
            framerate: 24,
            encrypt: false,
            stereo3d: false,
            validate: true,
            creator: String::new(),
            facility: String::new(),
            bandwidth: DEFAULT_BANDWIDTH_MBPS,
            gpu: false,
            gpu_license: String::new(),
            gpu_registration_url: DEFAULT_GPU_REGISTRATION_URL.to_string(),
            encode_threads: AUTOMATIC_ENCODE_THREADS,
            detect_picture_findings: false,
            signing_cert: String::new(),
            signing_key: String::new(),
            signing_intermediate: String::new(),
            signing_intermediate_key: String::new(),
            signing_root: String::new(),
            recipient_cert: String::new(),
            recipient_key: String::new(),
            output_dir: String::new(),
            isdcf_naming: false,
            channels: "5.1".to_string(),
            show_hints_before_build: true,
            additional: BTreeMap::new(),
        }
    }
}

pub fn preferences_directory() -> PathBuf {
    postkit::preferences::config_dir("dcpwizard")
}

pub fn preferences_path() -> PathBuf {
    preferences_directory().join("preferences.json")
}

pub fn load_preferences() -> io::Result<Preferences> {
    Ok(load_preferences_if_present()?.unwrap_or_default())
}

pub fn load_preferences_if_present() -> io::Result<Option<Preferences>> {
    load_preferences_from(&preferences_path())
}

pub fn preference_migrations() -> Vec<PrefsMigration> {
    vec![PrefsMigration {
        version: FIRST_MIGRATION_VERSION,
        description: "rename snake_case keys, cap bandwidth, make gpu a boolean".to_string(),
        apply: Box::new(migrate_to_version_two),
    }]
}

fn migrate_to_version_two(json: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(json) else {
        return json.to_string();
    };
    let Some(object) = value.as_object_mut() else {
        return json.to_string();
    };
    for (snake_case_name, camel_case_name) in SNAKE_CASE_RENAMES {
        if let Some(stored) = object.remove(snake_case_name) {
            object.entry(camel_case_name).or_insert(stored);
        }
    }
    let bandwidth_above_default = object
        .get("bandwidth")
        .and_then(Value::as_f64)
        .is_some_and(|bandwidth| bandwidth > f64::from(DEFAULT_BANDWIDTH_MBPS));
    if bandwidth_above_default {
        object.insert("bandwidth".to_string(), Value::from(DEFAULT_BANDWIDTH_MBPS));
    }
    let gpu_not_boolean = object.get("gpu").is_some_and(|gpu| !gpu.is_boolean());
    if gpu_not_boolean {
        object.insert("gpu".to_string(), Value::Bool(false));
    }
    value.to_string()
}

fn newer_file_error(stored_version: u32) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "preferences file version {stored_version} is newer than supported version {CURRENT_PREFERENCES_VERSION}"
        ),
    )
}

pub fn load_preferences_from(path: &Path) -> io::Result<Option<Preferences>> {
    let Some(contents) = postkit::preferences::read_preferences_file(path)? else {
        return Ok(None);
    };
    let stored_version = postkit::preferences::prefs_version(&contents);
    if stored_version > CURRENT_PREFERENCES_VERSION {
        return Err(newer_file_error(stored_version));
    }
    // migration hides the parse error of invalid json
    serde_json::from_str::<Value>(&contents)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let migrated = postkit::preferences::migrate_preferences(&contents, &preference_migrations());
    let preferences: Preferences = serde_json::from_str(&migrated)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    if stored_version < CURRENT_PREFERENCES_VERSION {
        save_preferences_to(&preferences, path)?;
    }

    Ok(Some(preferences))
}

pub fn save_preferences(preferences: &Preferences) -> io::Result<()> {
    save_preferences_to(preferences, &preferences_path())
}

pub fn save_preferences_to(preferences: &Preferences, path: &Path) -> io::Result<()> {
    if preferences.version > CURRENT_PREFERENCES_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "preferences version {} is newer than supported version {}",
                preferences.version, CURRENT_PREFERENCES_VERSION
            ),
        ));
    }

    if let Some(existing) = postkit::preferences::read_preferences_file(path)? {
        let existing_version = postkit::preferences::prefs_version(&existing);
        if existing_version > CURRENT_PREFERENCES_VERSION {
            return Err(newer_file_error(existing_version));
        }
    }

    let mut current = preferences.clone();
    current.version = CURRENT_PREFERENCES_VERSION;
    let contents = serde_json::to_string_pretty(&current).map_err(io::Error::other)?;
    postkit::preferences::write_preferences_file(path, &contents)
}

pub fn reset_preferences() -> io::Result<Preferences> {
    reset_preferences_to(&preferences_path())
}

pub fn reset_preferences_to(path: &Path) -> io::Result<Preferences> {
    let preferences = Preferences::default();
    let contents = serde_json::to_string_pretty(&preferences).map_err(io::Error::other)?;
    postkit::preferences::write_preferences_file(path, &contents)?;
    Ok(preferences)
}

pub fn set_preference(name: &str, value: &str) -> Result<Preferences, String> {
    let preferences = load_preferences().map_err(|error| error.to_string())?;
    let preferences = postkit::preferences::set_json_preference(&preferences, name, value)?;
    save_preferences(&preferences).map_err(|error| error.to_string())?;
    Ok(preferences)
}

// a recipient key without a KDM is refused
pub fn recipient_key_or_preference(
    kdm: Option<&Path>,
    explicit: Option<PathBuf>,
    preferences: &Preferences,
) -> Option<PathBuf> {
    if explicit.is_some() || kdm.is_none() || preferences.recipient_key.is_empty() {
        return explicit;
    }
    Some(PathBuf::from(&preferences.recipient_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn older_file_adds_defaults_and_updates_version() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");
        let contents =
            r#"{"version":1,"creator_name":"Studio","default_bandwidth_mbps":180,"theme":"dark"}"#;
        postkit::preferences::write_preferences_file(&path, contents).unwrap();

        let preferences = load_preferences_from(&path).unwrap().unwrap();

        assert_eq!(preferences.creator, "Studio");
        assert_eq!(preferences.bandwidth, 180);
        assert_eq!(preferences.additional["theme"], "dark");
        assert_eq!(preferences.version, CURRENT_PREFERENCES_VERSION);
        let saved = postkit::preferences::read_preferences_file(&path)
            .unwrap()
            .unwrap();
        assert_eq!(
            postkit::preferences::prefs_version(&saved),
            CURRENT_PREFERENCES_VERSION
        );
        assert!(saved.contains("gpuRegistrationUrl"));
        assert!(saved.contains("theme"));
        assert_eq!(
            preferences.gpu_registration_url,
            DEFAULT_GPU_REGISTRATION_URL
        );
    }

    #[test]
    fn new_preferences_use_the_registration_server() {
        assert_eq!(
            Preferences::default().gpu_registration_url,
            DEFAULT_GPU_REGISTRATION_URL
        );
    }

    #[test]
    fn a_file_without_encode_threads_encodes_on_automatic_threads() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");
        let contents = format!(r#"{{"version":{CURRENT_PREFERENCES_VERSION},"gpu":true}}"#);
        postkit::preferences::write_preferences_file(&path, &contents).unwrap();

        let preferences = load_preferences_from(&path).unwrap().unwrap();

        assert_eq!(preferences.encode_threads, AUTOMATIC_ENCODE_THREADS);
        assert!(!preferences.additional.contains_key("encodeThreads"));
    }

    #[test]
    fn a_file_without_the_findings_setting_leaves_the_picture_unscanned() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");
        let contents = format!(r#"{{"version":{CURRENT_PREFERENCES_VERSION},"gpu":true}}"#);
        postkit::preferences::write_preferences_file(&path, &contents).unwrap();

        let preferences = load_preferences_from(&path).unwrap().unwrap();

        assert!(!preferences.detect_picture_findings);
    }

    #[test]
    fn saved_preferences_name_the_encode_threads() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");

        save_preferences_to(&Preferences::default(), &path).unwrap();

        let saved = postkit::preferences::read_preferences_file(&path)
            .unwrap()
            .unwrap();
        assert!(saved.contains(r#""encodeThreads": 0"#));
    }

    #[test]
    fn newer_file_is_refused_until_reset() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");
        let contents = r#"{"version":99,"creator":"Future","futureField":true}"#;
        postkit::preferences::write_preferences_file(&path, contents).unwrap();

        let error = load_preferences_from(&path).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("version 99"));
        assert!(save_preferences_to(&Preferences::default(), &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), contents.as_bytes());

        reset_preferences_to(&path).unwrap();

        let reset = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            postkit::preferences::prefs_version(&reset),
            CURRENT_PREFERENCES_VERSION
        );
    }

    #[test]
    fn the_migration_steps_run_from_the_first_to_the_current_version() {
        let versions: Vec<u32> = preference_migrations()
            .iter()
            .map(|migration| migration.version)
            .collect();

        let expected: Vec<u32> = (FIRST_MIGRATION_VERSION..=CURRENT_PREFERENCES_VERSION).collect();
        assert_eq!(versions, expected);
    }

    const EVERY_SNAKE_CASE_KEY: &str = r#""default_standard":"Interop","default_resolution":"4K","default_frame_rate":25,"creator_name":"Studio","isdcf_facility_code":"FAC","default_bandwidth_mbps":180,"signing_certificate_path":"/keys/cert.pem","signing_key_path":"/keys/key.pem","default_output_dir":"/out","default_channel_config":"7.1""#;

    fn assert_every_snake_case_key_renamed(preferences: &Preferences, path: &Path) {
        assert_eq!(preferences.standard, "Interop");
        assert_eq!(preferences.resolution, "4K");
        assert_eq!(preferences.framerate, 25);
        assert_eq!(preferences.creator, "Studio");
        assert_eq!(preferences.facility, "FAC");
        assert_eq!(preferences.bandwidth, 180);
        assert_eq!(preferences.signing_cert, "/keys/cert.pem");
        assert_eq!(preferences.signing_key, "/keys/key.pem");
        assert_eq!(preferences.output_dir, "/out");
        assert_eq!(preferences.channels, "7.1");
        assert!(preferences.additional.is_empty());

        let saved = postkit::preferences::read_preferences_file(path)
            .unwrap()
            .unwrap();
        let saved: serde_json::Map<String, Value> = serde_json::from_str(&saved).unwrap();
        assert_eq!(saved["version"], CURRENT_PREFERENCES_VERSION);
        for (snake_case_name, camel_case_name) in SNAKE_CASE_RENAMES {
            assert!(!saved.contains_key(snake_case_name), "{snake_case_name}");
            assert!(saved.contains_key(camel_case_name), "{camel_case_name}");
        }
        assert_eq!(saved["creator"], "Studio");
        assert_eq!(saved["signingCert"], "/keys/cert.pem");
    }

    #[test]
    fn version_one_snake_case_keys_are_renamed() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");
        let contents = format!(r#"{{"version":1,{EVERY_SNAKE_CASE_KEY}}}"#);
        postkit::preferences::write_preferences_file(&path, &contents).unwrap();

        let preferences = load_preferences_from(&path).unwrap().unwrap();

        assert_every_snake_case_key_renamed(&preferences, &path);
    }

    #[test]
    fn unversioned_snake_case_keys_are_renamed() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");
        let contents = format!("{{{EVERY_SNAKE_CASE_KEY}}}");
        postkit::preferences::write_preferences_file(&path, &contents).unwrap();

        let preferences = load_preferences_from(&path).unwrap().unwrap();

        assert_every_snake_case_key_renamed(&preferences, &path);
    }

    #[test]
    fn camel_case_key_wins_over_its_snake_case_name() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");
        let contents = r#"{"version":1,"creator_name":"Old","creator":"New"}"#;
        postkit::preferences::write_preferences_file(&path, contents).unwrap();

        let preferences = load_preferences_from(&path).unwrap().unwrap();

        assert_eq!(preferences.creator, "New");
        assert!(!preferences.additional.contains_key("creator_name"));
    }

    fn with_recipient_key(recipient_key: &str) -> Preferences {
        Preferences {
            recipient_key: recipient_key.to_string(),
            ..Preferences::default()
        }
    }

    const KDM: &str = "/kdm/film.kdm.xml";

    #[test]
    fn an_explicit_recipient_key_wins_over_the_preference() {
        let explicit = recipient_key_or_preference(
            Some(Path::new(KDM)),
            Some(PathBuf::from("/keys/explicit.key")),
            &with_recipient_key("/keys/configured.key"),
        );

        assert_eq!(explicit, Some(PathBuf::from("/keys/explicit.key")));
    }

    #[test]
    fn a_kdm_without_a_recipient_key_takes_the_configured_one() {
        let configured = recipient_key_or_preference(
            Some(Path::new(KDM)),
            None,
            &with_recipient_key("/keys/configured.key"),
        );

        assert_eq!(configured, Some(PathBuf::from("/keys/configured.key")));
    }

    #[test]
    fn no_recipient_key_is_given_or_configured() {
        let missing =
            recipient_key_or_preference(Some(Path::new(KDM)), None, &with_recipient_key(""));

        assert_eq!(missing, None);
    }

    #[test]
    fn the_configured_recipient_key_stays_out_without_a_kdm() {
        let without_kdm =
            recipient_key_or_preference(None, None, &with_recipient_key("/keys/configured.key"));

        assert_eq!(without_kdm, None);
    }

    #[test]
    fn version_one_bandwidth_is_capped_and_gpu_made_boolean() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("preferences.json");
        let contents = r#"{"version":1,"default_bandwidth_mbps":500,"gpu":"yes"}"#;
        postkit::preferences::write_preferences_file(&path, contents).unwrap();

        let preferences = load_preferences_from(&path).unwrap().unwrap();

        assert_eq!(preferences.bandwidth, DEFAULT_BANDWIDTH_MBPS);
        assert!(!preferences.gpu);
    }
}
