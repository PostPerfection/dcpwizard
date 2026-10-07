// `create --audio-gain` beside `--loudness-target`: the gain sets the level and the target is
// only logged as not applied.
#![cfg(unix)]

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const TITLE: &str = "Audio Level";
const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const FRAME_RATE: u32 = 24;
const SOURCE_SECONDS: u32 = 1;
const GAIN_DB: &str = "-6";
const GAIN_RATIO: f64 = 0.501_187;
const RATIO_TOLERANCE: f64 = 0.005;
const TARGET: &str = "lufs=-20";
const UNAPPLIED_LINE: &str = "Loudness target lufs=-20 not applied: the gain sets the level";

fn ffmpeg() -> std::process::Command {
    let mut command = std::process::Command::new("ffmpeg");
    command.args(["-hide_banner", "-v", "error", "-y"]);
    command
}

fn run(command: &mut std::process::Command, what: &str) -> Vec<u8> {
    let output = command.output().unwrap_or_else(|e| panic!("{what}: {e}"));
    assert!(
        output.status.success(),
        "{what}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn only_file_starting_with(directory: &Path, prefix: &str) -> PathBuf {
    let mut found: Vec<PathBuf> = std::fs::read_dir(directory)
        .expect("the package directory has to be readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix))
        })
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "one {prefix}* in {}", directory.display());
    found.remove(0)
}

fn sound_peak(file: &Path) -> f64 {
    let pcm = run(
        ffmpeg()
            .arg("-i")
            .arg(file)
            .args(["-f", "s32le", "-c:a", "pcm_s32le", "-"]),
        "ffmpeg has to decode the sound",
    );
    pcm.as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f64::from(i32::from_le_bytes(*bytes).unsigned_abs()))
        .fold(0.0, f64::max)
}

#[test]
fn a_gain_beside_a_loudness_target_sets_the_level_alone() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = directory.path().join("source.mp4");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "testsrc=size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}:duration={SOURCE_SECONDS}"
            ))
            .args(["-pix_fmt", "yuv420p"])
            .arg(&source),
        "ffmpeg must be installed to write the source",
    );
    let tone = directory.path().join("tone.wav");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:duration={SOURCE_SECONDS}:sample_rate=48000"
            ))
            .args(["-ac", "2", "-c:a", "pcm_s24le"])
            .arg(&tone),
        "the tone has to be written",
    );

    let output = directory.path().join("dcp");
    let created = Command::cargo_bin("dcpwizard")
        .unwrap()
        .env("XDG_CONFIG_HOME", config_home.path())
        .env("HOME", config_home.path())
        .env("GRK_NO_PLUGIN", "1")
        .args(["create", "--title", TITLE, "--video"])
        .arg(&source)
        .arg("--audio")
        .arg(&tone)
        .arg("-o")
        .arg(&output)
        .args([
            "--twok",
            "--audio-gain",
            GAIN_DB,
            "--loudness-target",
            TARGET,
        ])
        .assert()
        .success();
    let package = output.join(TITLE);
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&created.get_output().stdout),
        String::from_utf8_lossy(&created.get_output().stderr)
    );

    let ratio = sound_peak(&only_file_starting_with(&package, "sound_")) / sound_peak(&tone);
    assert!(
        (ratio - GAIN_RATIO).abs() < RATIO_TOLERANCE,
        "the packaged sound is {ratio} of the source, not the gain alone"
    );
    assert!(printed.contains(UNAPPLIED_LINE), "{printed}");
    assert!(
        !printed.contains("-> -20.0 dB"),
        "the target must not normalise: {printed}"
    );
}
