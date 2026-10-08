use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const FRAME_RATE: u32 = 24;
const FRAMES: u32 = 3;
const TITLE: &str = "Verified";

const PASSED_LINE: &str = "DCP verification PASSED";
const SKIP_LINE: &str = "--no-verify: the finished package was not verified";
const BV21_SECTION_LINE: &str = "Bv2.1 profile check:";
const BV21_EXTENSION_METADATA_NOTE: &str = "BV2.1 recommends ExtensionMetadata in CPL";
const CODESTREAM_SUMMARY_CODE: &str = "j2k_codestream_summary";

fn dcpwizard(config_home: &Path) -> Command {
    let mut command = Command::cargo_bin("dcpwizard").unwrap();
    command.env("XDG_CONFIG_HOME", config_home);
    command
}

fn write_source(directory: &Path) -> PathBuf {
    let path = directory.join("source.mp4");
    let made = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}"),
            "-frames:v",
            &FRAMES.to_string(),
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&path)
        .output()
        .expect("ffmpeg has to run");
    assert!(
        made.status.success(),
        "ffmpeg could not write the source: {}",
        String::from_utf8_lossy(&made.stderr)
    );
    path
}

fn everything_printed(run: &std::process::Output) -> String {
    String::from_utf8_lossy(&run.stdout).into_owned() + &String::from_utf8_lossy(&run.stderr)
}

fn create(source: &Path, out: &Path, config_home: &Path, extra: &[&str]) -> std::process::Output {
    let mut command = dcpwizard(config_home);
    command.args([
        "create",
        "--title",
        TITLE,
        "--video",
        source.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--twok",
    ]);
    command.args(extra).output().expect("dcpwizard has to run")
}

#[test]
fn a_create_verifies_the_package_it_wrote() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    let run = create(&source, &out, config_home.path(), &[]);
    let printed = everything_printed(&run);
    assert!(
        run.status.success(),
        "a package that verifies exits 0: {printed}"
    );
    assert!(
        printed.contains(PASSED_LINE),
        "create has to end with the verification the verify command prints: {printed}"
    );
    assert!(!printed.contains(SKIP_LINE), "{printed}");
    // the verification read a real package, not an empty directory
    let package = out.join(TITLE);
    assert!(package.join("ASSETMAP.xml").exists() || package.join("ASSETMAP").exists());
}

#[test]
fn no_verify_says_the_package_went_out_unread() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    let run = create(&source, &out, config_home.path(), &["--no-verify"]);
    assert!(run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stderr).contains(SKIP_LINE),
        "the skip has to be said out loud: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let printed = everything_printed(&run);
    assert!(
        !printed.contains(PASSED_LINE),
        "nothing was verified, so nothing passed: {printed}"
    );
}

#[test]
fn verify_strict_runs_the_bv21_profile_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");
    assert!(
        create(&source, &out, config_home.path(), &["--no-verify"])
            .status
            .success()
    );

    let verify = |extra: &[&str]| {
        dcpwizard(config_home.path())
            .arg("verify")
            .args(extra)
            .arg(out.join(TITLE))
            .output()
            .expect("dcpwizard has to run")
    };

    let report = directory.path().join("report.html");
    let strict = verify(&["--strict", "--output", report.to_str().unwrap()]);
    let printed = everything_printed(&strict);
    assert!(strict.status.success(), "{printed}");
    assert!(printed.contains(BV21_SECTION_LINE), "{printed}");
    assert!(printed.contains(BV21_EXTENSION_METADATA_NOTE), "{printed}");
    let written = std::fs::read_to_string(&report).unwrap();
    assert!(written.contains(BV21_SECTION_LINE), "{written}");
    assert!(written.contains(BV21_EXTENSION_METADATA_NOTE), "{written}");

    let plain = everything_printed(&verify(&[]));
    assert!(!plain.contains(BV21_SECTION_LINE), "{plain}");
}

// the GUI's Inspect MXF essence box is the picture check
#[test]
fn the_picture_check_reads_every_frame() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");
    assert!(
        create(&source, &out, config_home.path(), &["--no-verify"])
            .status
            .success()
    );
    let verify = |extra: &[&str]| {
        let run = dcpwizard(config_home.path())
            .args(["verify", "--strict", "--no-hash-check"])
            .args(extra)
            .arg(out.join(TITLE))
            .output()
            .expect("dcpwizard has to run");
        everything_printed(&run)
    };

    let checked = verify(&[]);
    assert!(
        checked.contains(CODESTREAM_SUMMARY_CODE)
            && checked.contains(&format!("across {FRAMES} frames")),
        "{checked}"
    );
    let skipped = verify(&["--no-picture-check"]);
    assert!(!skipped.contains(CODESTREAM_SUMMARY_CODE), "{skipped}");
}

#[test]
fn a_title_with_a_slash_names_a_folder_with_an_underscore() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    let run = dcpwizard(config_home.path())
        .args([
            "create",
            "--title",
            "Part 1/2",
            "--video",
            source.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--twok",
            "--no-verify",
        ])
        .output()
        .expect("dcpwizard has to run");
    assert!(run.status.success(), "{}", everything_printed(&run));
    assert!(out.join("Part 1_2").join("ASSETMAP.xml").exists());
    assert!(!out.join("Part 1").exists());
}

fn files_with_sizes_and_times(directory: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_with_sizes_and_times(&path));
            continue;
        }
        let metadata = std::fs::metadata(&path).unwrap();
        files.push((path, metadata.len(), metadata.modified().unwrap()));
    }
    files.sort();
    files
}

#[test]
fn a_second_create_under_the_same_title_is_refused_and_leaves_the_first_package_alone() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    let first = create(&source, &out, config_home.path(), &["--no-verify"]);
    assert!(first.status.success(), "{}", everything_printed(&first));
    let first_files = files_with_sizes_and_times(&out);

    let second = create(&source, &out, config_home.path(), &["--no-verify"]);
    let printed = everything_printed(&second);
    assert!(!second.status.success(), "{printed}");
    let expected = format!(
        "A DCP already exists at {}. Use a new title or output folder, or delete the old package first.",
        out.join(TITLE).display()
    );
    assert!(printed.contains(&expected), "{printed}");
    assert_eq!(files_with_sizes_and_times(&out), first_files);
}
