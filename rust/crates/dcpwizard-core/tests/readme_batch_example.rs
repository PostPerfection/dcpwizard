use dcpwizard_core::dcp::DcpConfig;

fn repository_file(relative_path: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../../{relative_path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn batch_add_params(text: &str) -> &str {
    let line = text
        .lines()
        .find(|line| line.contains("batch add") && line.contains("'{"))
        .expect("a batch add example with JSON params");
    let start = line.find("'{").unwrap() + 1;
    let end = line.rfind("}'").unwrap() + 1;
    &line[start..end]
}

fn assert_is_the_example_config(params: &str) {
    let config: DcpConfig = serde_json::from_str(params).unwrap();
    assert_eq!(config.title, "My Film");
    assert_eq!(
        config.j2k_dir.as_deref(),
        Some(std::path::Path::new("./j2k"))
    );
    assert_eq!(config.output_dir, std::path::PathBuf::from("./dcp"));
}

#[test]
fn the_readme_batch_add_example_is_a_config_the_daemon_accepts() {
    assert_is_the_example_config(batch_add_params(&repository_file("README.md")));
}

#[test]
fn the_site_batch_add_example_is_the_readme_config() {
    let site = repository_file("docs/index.html");
    let readme = repository_file("README.md");
    let site_params = batch_add_params(&site);
    assert_is_the_example_config(site_params);
    assert_eq!(site_params, batch_add_params(&readme));
}
