// `create` with a trim and fades: the fades have to land on the first and last kept frames,
// not on the stretch of the source the trim cuts away.

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const TITLE: &str = "Trimmed Fades";
const SOURCE_COLOUR: &str = "0xc0c0c0";
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const FRAME_RATE: u32 = 24;
const SOURCE_SECONDS: u32 = 3;
// a second off each end keeps the middle second, 24 frames
const TRIM: &str = "1s";
const KEPT_FRAMES: usize = 24;
// shorter than the trims, so a fade placed on the source clock is cut away whole
const FADE_SECONDS: &str = "0.5";

const READBACK_WIDTH: usize = 64;
const READBACK_HEIGHT: usize = 36;
// a black frame after the X'Y'Z' and J2K round trip stays under this 8-bit grey level
const DARK_CEILING: f64 = 8.0;
const LIT_FLOOR: f64 = 64.0;

const SAMPLE_RATE: usize = 48_000;
const SOUND_CHANNELS: usize = 2;
// the last 10 ms of a half second linear fade carry at most 2 percent of the level
const TAIL_SAMPLES: usize = SAMPLE_RATE / 100;
const TAIL_CEILING_RATIO: f64 = 0.05;

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

// the mean grey level of every frame of a picture MXF
fn frame_means(picture_mxf: &Path) -> Vec<f64> {
    let grey = run(
        ffmpeg()
            .arg("-i")
            .arg(picture_mxf)
            .arg("-vf")
            .arg(format!(
                "scale={READBACK_WIDTH}:{READBACK_HEIGHT},format=gray"
            ))
            .args(["-f", "rawvideo", "-"]),
        "ffmpeg has to decode the picture MXF",
    );
    grey.chunks(READBACK_WIDTH * READBACK_HEIGHT)
        .map(|frame| frame.iter().map(|&level| f64::from(level)).sum::<f64>() / frame.len() as f64)
        .collect()
}

// every sample of a sound MXF, interleaved
fn sound_samples(sound_mxf: &Path) -> Vec<i32> {
    let pcm = run(
        ffmpeg()
            .arg("-i")
            .arg(sound_mxf)
            .args(["-f", "s32le", "-c:a", "pcm_s32le", "-"]),
        "ffmpeg has to decode the sound MXF",
    );
    pcm.as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| i32::from_le_bytes(*bytes))
        .collect()
}

fn peak(samples: &[i32]) -> f64 {
    samples
        .iter()
        .map(|sample| f64::from(sample.unsigned_abs()))
        .fold(0.0, f64::max)
}

#[test]
fn fades_land_on_the_kept_frames_of_a_trimmed_source() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    let master = directory.path().join("master.mkv");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "color=c={SOURCE_COLOUR}:size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}:duration={SOURCE_SECONDS}"
            ))
            .args(["-c:v", "ffv1", "-pix_fmt", "gbrp"])
            .arg(&master),
        "ffmpeg must be installed to write the flat colour master",
    );
    let tone = directory.path().join("tone.wav");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:duration={SOURCE_SECONDS}:sample_rate={SAMPLE_RATE}"
            ))
            .args(["-ac", &SOUND_CHANNELS.to_string(), "-c:a", "pcm_s24le"])
            .arg(&tone),
        "the tone has to be written",
    );

    let output = directory.path().join("dcp");
    Command::cargo_bin("dcpwizard")
        .unwrap()
        .env("XDG_CONFIG_HOME", config_home.path())
        .args(["create", "--title", TITLE, "--video"])
        .arg(&master)
        .arg("--audio")
        .arg(&tone)
        .arg("-o")
        .arg(&output)
        .args(["--container-dims", &format!("{WIDTH}x{HEIGHT}")])
        .args(["--trim-start", TRIM, "--trim-end", TRIM])
        .args(["--video-fade-in", FADE_SECONDS])
        .args(["--audio-fade-out", FADE_SECONDS])
        .assert()
        .success();
    let package = output.join(TITLE);

    let means = frame_means(&only_file_starting_with(&package, "picture_"));
    assert_eq!(
        means.len(),
        KEPT_FRAMES,
        "the package carries the kept frames"
    );
    assert!(
        means[0] < DARK_CEILING,
        "the first kept frame is not dark: grey level {}",
        means[0]
    );
    assert!(
        means[KEPT_FRAMES - 1] > LIT_FLOOR,
        "the picture never came up: grey level {}",
        means[KEPT_FRAMES - 1]
    );

    let samples = sound_samples(&only_file_starting_with(&package, "sound_"));
    let channels = samples.len() / (KEPT_FRAMES * SAMPLE_RATE / FRAME_RATE as usize);
    let tail = &samples[samples.len() - TAIL_SAMPLES * channels..];
    let opening = &samples[..samples.len() / 4];
    assert!(
        peak(tail) < TAIL_CEILING_RATIO * peak(opening),
        "the sound does not fade out at the last kept frame: tail peak {}, opening peak {}",
        peak(tail),
        peak(opening)
    );
}
