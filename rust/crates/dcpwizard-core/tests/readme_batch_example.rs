use dcpwizard_core::dcp::DcpConfig;

#[test]
fn the_readme_batch_add_example_is_a_config_the_daemon_accepts() {
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../README.md"))
            .unwrap();
    let line = readme
        .lines()
        .find(|line| line.contains("batch add") && line.contains("-p '{"))
        .expect("README has a batch add example with -p params");
    let start = line.find("'{").unwrap() + 1;
    let end = line.rfind("}'").unwrap() + 1;
    let params = &line[start..end];
    let config: DcpConfig = serde_json::from_str(params).unwrap();
    assert_eq!(config.title, "My Film");
    assert_eq!(
        config.j2k_dir.as_deref(),
        Some(std::path::Path::new("./j2k"))
    );
    assert_eq!(config.output_dir, std::path::PathBuf::from("./dcp"));
}
