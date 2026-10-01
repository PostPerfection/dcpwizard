// dirs::config_dir on windows reads the win32 known-folder api, not an env var,
// so a sibling test's gpu preference cannot be isolated from these create runs
#![cfg(unix)]

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const FRAME_RATE: u32 = 24;
const FRAMES: u32 = 3;
const TITLE: &str = "Job Log";
const LOG_NAME_INSIDE_THE_PACKAGE: &str = "dcpwizard.log";
const FOREIGN_FILE_CODE: &str = "foreign_file_in_package";
const DEVICE_WARNING: &str =
    "[ENCODE] WARNING: the GPU was requested and no frame ran on the device";

fn dcpwizard(config_home: &Path) -> Command {
    let mut command = Command::cargo_bin("dcpwizard").unwrap();
    // dirs::config_dir reads XDG on Linux but HOME on macOS and APPDATA on Windows,
    // so all three point at the temp dir or a sibling test's gpu preference leaks in
    command.env("XDG_CONFIG_HOME", config_home);
    command.env("HOME", config_home);
    command.env("APPDATA", config_home);
    // no accelerator plugin loads, so every run here encodes on the CPU
    command.env("GRK_NO_PLUGIN", "1");
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

fn create(source: &Path, output: &Path, config_home: &Path, extra: &[&str]) {
    dcpwizard(config_home)
        .args([
            "create",
            "--title",
            TITLE,
            "--video",
            source.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--twok",
        ])
        .args(extra)
        .assert()
        .success();
}

fn read_log_beside(package: &Path) -> String {
    assert!(
        !package.join(LOG_NAME_INSIDE_THE_PACKAGE).exists(),
        "the job log has to stay out of the package folder"
    );
    std::fs::read_to_string(package.with_extension("log"))
        .unwrap_or_else(|e| panic!("the job log has to sit beside the package: {e}"))
}

fn header_line<'a>(log: &'a str, prefix: &str) -> (usize, &'a str) {
    log.lines()
        .enumerate()
        .find(|(_, line)| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("the log has to hold a {prefix:?} line: {log}"))
}

fn finished_line(log: &str) -> &str {
    let last = log.lines().last().unwrap_or_default();
    assert!(
        last.starts_with("Finished: "),
        "the log has to end with its Finished line: {log}"
    );
    last
}

fn create_and_read_log(source: &Path, output: &Path, config_home: &Path) -> String {
    create(source, output, config_home, &[]);
    read_log_beside(&output.join(TITLE))
}

#[test]
fn dcpdoctor_finds_no_foreign_file_in_a_fresh_package() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let output = directory.path().join("dcp");
    create(&source, &output, config_home.path(), &[]);

    let verified = dcpwizard_core::verify::verify_dcp(&output.join(TITLE));
    assert!(verified.valid, "dcpdoctor errors: {:?}", verified.errors);
    let foreign: Vec<&String> = verified
        .warnings
        .iter()
        .filter(|warning| warning.contains(FOREIGN_FILE_CODE))
        .collect();
    assert!(
        foreign.is_empty(),
        "the package holds a file its ASSETMAP does not list: {foreign:?}"
    );
}

#[test]
fn a_cpu_create_logs_the_accelerator_off_and_no_frames_on_the_device() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let output = directory.path().join("dcp");

    let log = create_and_read_log(&source, &output, config_home.path());
    assert!(
        log.contains("Accelerator: off"),
        "a run that asked for no device says so: {log}"
    );
    let thread_line = log
        .lines()
        .find(|line| line.starts_with("Encode threads: "))
        .unwrap_or_else(|| panic!("the log has to name the encode threads: {log}"));
    assert!(
        thread_line.ends_with(" (automatic)"),
        "with no preferences file the thread count is chosen automatically: {thread_line}"
    );
    assert!(
        log.lines().any(|line| line == "Picture findings: off"),
        "with no preferences file the picture is not scanned: {log}"
    );
    assert!(
        log.contains(&format!("[ENCODE] Frames on the device: 0 of {FRAMES}")),
        "the encode has to report the count against the frames it encoded: {log}"
    );
    assert!(
        !log.contains(DEVICE_WARNING),
        "nothing asked for the device, so nothing is warned about: {log}"
    );
    assert!(
        !log.lines().any(|line| line.starts_with("GPU: ")),
        "nothing asked for the device, so no GPU is looked for: {log}"
    );
}

#[test]
fn the_header_names_the_machine_the_source_and_every_setting_and_the_log_ends_done() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let output = directory.path().join("dcp");

    let log = create_and_read_log(&source, &output, config_home.path());
    let (started, _) = header_line(&log, "Started: ");
    let (machine, _) = header_line(&log, "Machine: ");
    let (source_index, source_line) = header_line(&log, "Source: ");
    let (settings_index, settings_line) = header_line(&log, "Settings: ");
    assert!(
        started < machine && machine < source_index && source_index < settings_index,
        "Started, Machine, Source and Settings come in that order: {log}"
    );
    assert!(
        source_line.starts_with(&format!(
            "Source: h264 {WIDTH}x{HEIGHT} {FRAME_RATE}/1 {FRAMES} frames, yuv420p, colour "
        )),
        "{source_line}"
    );
    let settings: serde_json::Value =
        serde_json::from_str(settings_line.strip_prefix("Settings: ").unwrap())
            .unwrap_or_else(|e| panic!("the settings have to be one JSON object: {e}"));
    assert_eq!(settings["title"], TITLE, "{settings_line}");
    assert_eq!(
        settings["video"],
        source.to_str().unwrap(),
        "{settings_line}"
    );
    assert_eq!(settings["twok"], "true", "{settings_line}");
    assert!(
        settings.get("license").is_none(),
        "the global options stay off the settings line: {settings_line}"
    );
    assert!(
        log.lines().any(|line| {
            line.starts_with("[ENCODE] decoding to the pipe pixel_format=")
                && line.ends_with(" hardware_decode=false")
        }),
        "the encode names the pixel format it decodes to: {log}"
    );
    assert!(finished_line(&log).ends_with(", done"), "{log}");
}

#[test]
fn a_create_that_fails_after_the_encode_ends_its_log_with_the_reason() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let not_a_video = directory.path().join("notes.txt");
    std::fs::write(&not_a_video, "no video here").unwrap();
    let output = directory.path().join("dcp");

    dcpwizard(config_home.path())
        .args([
            "create",
            "--title",
            TITLE,
            "--video",
            source.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--twok",
            "--sign-language-video",
            not_a_video.to_str().unwrap(),
            "--sign-language-lang",
            "en",
        ])
        .assert()
        .failure();
    let log = read_log_beside(&output.join(TITLE));
    let finished = finished_line(&log);
    assert!(
        finished.ends_with(&format!(
            ", failed: ffmpeg could not conform {} to VP9",
            not_a_video.display()
        )),
        "the Finished line names why the job failed: {log}"
    );
}

#[test]
fn a_create_under_the_gpu_preference_logs_why_the_device_never_started() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    dcpwizard(config_home.path())
        .args(["preferences", "set", "gpu", "true"])
        .assert()
        .success();

    let source = write_source(directory.path());
    let output = directory.path().join("dcp");
    let log = create_and_read_log(&source, &output, config_home.path());

    let accelerator = log
        .lines()
        .find(|line| line.starts_with("Accelerator: "))
        .unwrap_or_else(|| panic!("the header has to name the accelerator: {log}"));
    assert!(
        accelerator.starts_with("Accelerator: requested, inactive: "),
        "the preference asked for the device and no plugin loaded: {accelerator}"
    );
    assert!(
        log.contains(&format!("[ENCODE] Frames on the device: 0 of {FRAMES}")),
        "a CPU run under the preference still counts the frames: {log}"
    );
    assert!(
        log.contains(DEVICE_WARNING),
        "the device was asked for and took no frame: {log}"
    );
    let (_, gpu) = header_line(&log, "GPU: ");
    assert_ne!(gpu, "GPU: ", "the GPU line names what was found: {log}");
}

fn only_folder_in(output: &Path) -> PathBuf {
    let folders: Vec<PathBuf> = std::fs::read_dir(output)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    let [package] = folders.as_slice() else {
        panic!("one package folder in {}: {folders:?}", output.display());
    };
    package.clone()
}

fn content_title_of(package: &Path) -> String {
    let cpl = std::fs::read_dir(package)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("CPL_")
        })
        .expect("the package folder has to hold a CPL");
    let cpl_text = std::fs::read_to_string(cpl).unwrap();
    cpl_text
        .split_once("<ContentTitleText>")
        .and_then(|(_, rest)| rest.split_once("</ContentTitleText>"))
        .map(|(title, _)| title.to_string())
        .expect("the CPL has a content title")
}

#[test]
fn an_isdcf_named_package_is_written_to_a_folder_named_by_its_isdcf_name() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let output = directory.path().join("dcp");
    create(&source, &output, config_home.path(), &["--isdcf-name"]);

    let package = only_folder_in(&output);
    let content_title = content_title_of(&package);
    assert!(
        content_title.starts_with("JobLog_") && content_title.ends_with("_SMPTE_OV"),
        "the CPL carries the ISDCF name: {content_title}"
    );
    assert_eq!(
        package.file_name().unwrap().to_string_lossy(),
        content_title,
        "the folder takes the ISDCF name"
    );
    let log = read_log_beside(&package);
    assert!(log.contains(&format!("Title: {TITLE}")), "{log}");
    assert!(
        log.contains(&format!("Output: {}", package.display())),
        "{log}"
    );
}

#[test]
fn a_source_with_its_own_five_one_sound_and_no_wav_is_named_five_one() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = directory.path().join("five-one.mp4");
    let made = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
        .arg(format!("testsrc=size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}"))
        .args([
            "-f",
            "lavfi",
            "-i",
            "anullsrc=channel_layout=5.1:sample_rate=48000",
        ])
        .args([
            "-frames:v",
            &FRAMES.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ])
        .arg(&source)
        .output()
        .expect("ffmpeg has to run");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let output = directory.path().join("dcp");
    create(
        &source,
        &output,
        config_home.path(),
        &["--isdcf-name", "--isdcf-date", "2026-08-16"],
    );

    let package = only_folder_in(&output);
    let content_title = content_title_of(&package);
    assert_eq!(
        content_title,
        "JobLog_FTR-1_C_XX-XX_51_2K_20260816_SMPTE_OV"
    );
    assert_eq!(
        package.file_name().unwrap().to_string_lossy(),
        content_title
    );
}
