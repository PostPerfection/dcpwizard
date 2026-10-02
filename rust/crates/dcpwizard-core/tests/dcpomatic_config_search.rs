#![cfg(not(target_os = "macos"))]

use dcpwizard_core::dcpomatic_identity::{dcpomatic_config_candidates, find_dcpomatic_config};
use std::path::Path;
use tempfile::TempDir;

fn write_config(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "<Config/>").unwrap();
}

#[test]
fn the_newest_dcpomatic_config_under_xdg_config_home_is_found() {
    let config_home = TempDir::new().unwrap();
    // the only test in this binary, so no other thread reads the environment
    unsafe { std::env::set_var("XDG_CONFIG_HOME", config_home.path()) };
    let root = config_home.path().join("dcpomatic2");
    let newest = root.join("2.18").join("config.xml");
    let older = root.join("2.16").join("config.xml");
    let unversioned = root.join("config.xml");

    assert_eq!(
        dcpomatic_config_candidates(),
        vec![newest.clone(), older.clone(), unversioned.clone()]
    );

    let error = find_dcpomatic_config().unwrap_err();
    for candidate in [&newest, &older, &unversioned] {
        assert!(error.contains(&candidate.display().to_string()), "{error}");
    }

    write_config(&unversioned);
    write_config(&older);
    assert_eq!(find_dcpomatic_config().unwrap(), older);

    write_config(&newest);
    assert_eq!(find_dcpomatic_config().unwrap(), newest);
}
