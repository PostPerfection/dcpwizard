use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("dcpwizard").unwrap()
}

#[test]
fn version_flag() {
    cmd().arg("--version").assert().success().stdout(
        predicate::str::contains("dcpwizard")
            .and(predicate::str::contains(env!("CARGO_PKG_VERSION"))),
    );
}

#[test]
fn help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("verify"))
        .stdout(predicate::str::contains("kdm"));
}

#[test]
fn verify_missing_directory() {
    let dir = TempDir::new().unwrap();
    let nonexistent = dir.path().join("does_not_exist");

    cmd()
        .args(["verify", nonexistent.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn verify_empty_directory() {
    let dir = TempDir::new().unwrap();

    cmd()
        .args(["verify", dir.path().to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn verify_with_output_report() {
    let dir = TempDir::new().unwrap();
    let report = dir.path().join("report.txt");

    cmd()
        .args([
            "verify",
            dir.path().to_str().unwrap(),
            "--output",
            report.to_str().unwrap(),
        ])
        .assert()
        .failure(); // Fails because dir is empty, but exercises the output path
}

#[test]
fn create_missing_video() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output_dcp");

    cmd()
        .args([
            "create",
            "--title",
            "Test DCP",
            "--video",
            "/nonexistent/video.mxf",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

#[test]
fn create_help() {
    cmd()
        .args(["create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--video"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--content-type"))
        .stdout(predicate::str::contains("--frame-rate"))
        .stdout(predicate::str::contains("--twok"))
        .stdout(predicate::str::contains("--fourk"));
}

#[test]
fn kdm_help() {
    cmd()
        .args(["kdm", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--cert"))
        .stdout(predicate::str::contains("--signer-cert"))
        .stdout(predicate::str::contains("--signer-key"))
        .stdout(predicate::str::contains("--valid-from"))
        .stdout(predicate::str::contains("--valid-to"));
}

#[test]
fn verify_help() {
    cmd()
        .args(["verify", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-hash-check"))
        .stdout(predicate::str::contains("--no-picture-check"))
        .stdout(predicate::str::contains("--strict"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--quiet"));
}

#[test]
fn kdm_missing_inputs() {
    cmd()
        .args([
            "kdm",
            "--cert",
            "/nonexistent/cert.pem",
            "--signer-cert",
            "/nonexistent/signer.pem",
            "--signer-key",
            "/nonexistent/signer.key",
            "--cpl-id",
            "urn:uuid:00000000-0000-0000-0000-000000000000",
            "--content-title",
            "Test",
            "--output",
            "/tmp/test.kdm.xml",
        ])
        .assert()
        .failure();
}
