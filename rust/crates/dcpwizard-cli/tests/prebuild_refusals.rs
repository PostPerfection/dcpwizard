use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const FRAME_RATE: u32 = 24;
const FRAMES: u32 = 3;

// 44100 divides into whole samples per frame at 25 fps, so the frame alignment
// refusal lets it through and only the DCI rate catches it
const NON_DCI_SAMPLE_RATE: u32 = 44_100;
const NON_DCI_SAMPLE_RATE_FPS: u32 = 25;

fn dcpwizard(config_home: &Path) -> Command {
    let mut command = Command::cargo_bin("dcpwizard").unwrap();
    command.env("XDG_CONFIG_HOME", config_home);
    command
}

fn run_ffmpeg(arguments: &[&str], what: &str) {
    let made = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error"])
        .args(arguments)
        .output()
        .expect("ffmpeg has to run");
    assert!(
        made.status.success(),
        "ffmpeg could not write {what}: {}",
        String::from_utf8_lossy(&made.stderr)
    );
}

fn write_source(directory: &Path) -> PathBuf {
    let path = directory.join("source.mp4");
    run_ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}"),
            "-frames:v",
            &FRAMES.to_string(),
            "-pix_fmt",
            "yuv420p",
            path.to_str().unwrap(),
        ],
        "the source",
    );
    path
}

fn write_wav(directory: &Path, name: &str, codec: &str, sample_rate: u32) -> PathBuf {
    let path = directory.join(name);
    run_ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("anullsrc=r={sample_rate}:cl=stereo"),
            "-t",
            "0.5",
            "-c:a",
            codec,
            path.to_str().unwrap(),
        ],
        name,
    );
    path
}

fn create_is_refused(command: &mut Command, out: &Path, naming: &[&str]) {
    let run = command.output().expect("dcpwizard has to run");
    let printed =
        String::from_utf8_lossy(&run.stdout).into_owned() + &String::from_utf8_lossy(&run.stderr);
    assert!(
        !run.status.success(),
        "the build was not refused: {printed}"
    );
    for needle in naming {
        assert!(
            printed.contains(needle),
            "the refusal has to name {needle}: {printed}"
        );
    }
    assert!(
        !out.exists(),
        "a refusal before the encode writes nothing under {}",
        out.display()
    );
}

#[test]
fn a_sixteen_bit_wav_is_refused_before_any_frame_is_encoded() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let wav = write_wav(directory.path(), "sixteen.wav", "pcm_s16le", 48_000);
    let out = directory.path().join("dcp");

    create_is_refused(
        dcpwizard(config_home.path()).args([
            "create",
            "--title",
            "Sixteen Bit",
            "--video",
            source.to_str().unwrap(),
            "--audio",
            wav.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--twok",
        ]),
        &out,
        &["16-bit", "24-bit PCM", "sixteen.wav"],
    );
}

#[test]
fn a_sample_rate_dci_does_not_allow_is_refused_before_any_frame_is_encoded() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let wav = write_wav(
        directory.path(),
        "forty_four.wav",
        "pcm_s24le",
        NON_DCI_SAMPLE_RATE,
    );
    let out = directory.path().join("dcp");

    create_is_refused(
        dcpwizard(config_home.path()).args([
            "create",
            "--title",
            "Wrong Rate",
            "--video",
            source.to_str().unwrap(),
            "--audio",
            wav.to_str().unwrap(),
            "--frame-rate",
            &NON_DCI_SAMPLE_RATE_FPS.to_string(),
            "-o",
            out.to_str().unwrap(),
            "--twok",
        ]),
        &out,
        &[
            &NON_DCI_SAMPLE_RATE.to_string(),
            "48000 or 96000",
            "forty_four.wav",
        ],
    );
}
