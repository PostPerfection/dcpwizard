// `create --timeline`: clips of different sizes, each trimmed, built into one package with a
// reel start at the second clip.

use assert_cmd::Command;
use postkit::encode::FrameRate;
use postkit::timeline::{Clip, ClipSound, SoundTrack, TIMELINE_FORMAT_VERSION, Timeline};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const TITLE: &str = "Timeline Build";
const FRAME_RATE: u32 = 24;
const OTHER_FRAME_RATE: u32 = 25;
const CLIP_SECONDS: u32 = 2;
const SMALL_SIZE: &str = "320x180";
const LARGE_SIZE: &str = "640x360";
const SAMPLE_RATE: u64 = 48_000;
const SAMPLES_PER_FRAME: u64 = SAMPLE_RATE / FRAME_RATE as u64;

// 48 frames each, cut to 36 and 24
const FIRST_IN: u64 = 6;
const FIRST_OUT: u64 = 42;
const SECOND_IN: u64 = 12;
const SECOND_OUT: u64 = 36;
const FIRST_KEPT: u64 = FIRST_OUT - FIRST_IN;
const SECOND_KEPT: u64 = SECOND_OUT - SECOND_IN;

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

fn write_clip(directory: &Path, name: &str, size: &str, frame_rate: u32) -> PathBuf {
    let path = directory.join(name);
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "testsrc=size={size}:rate={frame_rate}:duration={CLIP_SECONDS}"
            ))
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:sample_rate={SAMPLE_RATE}:duration={CLIP_SECONDS}"
            ))
            .args(["-c:v", "ffv1", "-c:a", "pcm_s24le", "-ac", "2"])
            .arg(&path),
        "ffmpeg must be installed to write a clip",
    );
    path
}

fn clip(picture: PathBuf, in_frame: u64, out_frame: u64) -> Clip {
    Clip {
        picture,
        in_frame,
        out_frame,
        sound: ClipSound::Embedded,
        subtitle: None,
        source_colour: Default::default(),
        picture_processing: Default::default(),
    }
}

fn write_timeline(directory: &Path, clips: Vec<Clip>, reel_starts: Vec<u64>) -> PathBuf {
    let path = directory.join("timeline.json");
    Timeline {
        version: TIMELINE_FORMAT_VERSION,
        fps: FrameRate::whole(FRAME_RATE),
        clips,
        sound: SoundTrack::PerClip,
        subtitle: None,
        reel_starts,
    }
    .write(&path)
    .unwrap();
    path
}

fn two_clip_timeline(directory: &Path) -> PathBuf {
    let first = write_clip(directory, "first.mkv", SMALL_SIZE, FRAME_RATE);
    let second = write_clip(directory, "second.mkv", LARGE_SIZE, FRAME_RATE);
    write_timeline(
        directory,
        vec![
            clip(first, FIRST_IN, FIRST_OUT),
            clip(second, SECOND_IN, SECOND_OUT),
        ],
        vec![FIRST_KEPT],
    )
}

fn create(config_home: &Path, timeline: &Path, output: &Path) -> Command {
    let mut command = Command::cargo_bin("dcpwizard").unwrap();
    command
        .env("XDG_CONFIG_HOME", config_home)
        .args(["create", "--title", TITLE, "--timeline"])
        .arg(timeline)
        .arg("-o")
        .arg(output);
    command
}

fn files_starting_with(directory: &Path, prefix: &str) -> Vec<PathBuf> {
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
    found
}

// each reel's picture duration, in reel order
fn reel_picture_durations(package: &Path) -> Vec<u64> {
    let cpls = files_starting_with(package, "CPL_");
    assert_eq!(cpls.len(), 1, "one CPL in {}", package.display());
    let text = std::fs::read_to_string(&cpls[0]).unwrap();
    let document = roxmltree::Document::parse(&text).unwrap();
    let child_number = |node: roxmltree::Node, name: &str| -> Option<u64> {
        node.children()
            .find(|child| child.has_tag_name(name))
            .and_then(|child| child.text())
            .map(|text| text.trim().parse().unwrap())
    };
    document
        .descendants()
        .filter(|node| node.has_tag_name("MainPicture"))
        .map(|picture| {
            child_number(picture, "Duration")
                .or_else(|| child_number(picture, "IntrinsicDuration"))
                .expect("a reel picture declares its length")
        })
        .collect()
}

// every sample of a sound MXF, its channels mixed to one
fn sound_samples(sound_mxf: &Path) -> Vec<i32> {
    let pcm = run(
        ffmpeg().arg("-i").arg(sound_mxf).args([
            "-ac",
            "1",
            "-f",
            "s32le",
            "-c:a",
            "pcm_s32le",
            "-",
        ]),
        "ffmpeg has to decode the sound MXF",
    );
    pcm.as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| i32::from_le_bytes(*bytes))
        .collect()
}

fn printed(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr)
}

fn is_refused(command: &mut Command, out: &Path, naming: &[&str]) {
    let run = command.output().expect("dcpwizard has to run");
    let printed = printed(&run);
    assert!(!run.status.success(), "not refused: {printed}");
    for needle in naming {
        assert!(printed.contains(needle), "has to name {needle}: {printed}");
    }
    assert!(!out.exists(), "a refusal writes nothing: {}", out.display());
}

#[test]
fn two_trimmed_clips_build_one_reel_each() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let timeline = two_clip_timeline(directory.path());
    let output = directory.path().join("dcp");

    create(config_home.path(), &timeline, &output)
        .assert()
        .success();
    let package = output.join(TITLE);

    assert_eq!(
        reel_picture_durations(&package),
        vec![FIRST_KEPT, SECOND_KEPT],
        "each reel carries one clip's kept frames"
    );
    let sounds: Vec<Vec<i32>> = files_starting_with(&package, "sound_")
        .iter()
        .map(|sound| sound_samples(sound))
        .collect();
    assert!(
        sounds
            .iter()
            .all(|samples| samples.iter().any(|&sample| sample != 0)),
        "every reel carries its clip's tone"
    );
    let mut sound_lengths: Vec<u64> = sounds.iter().map(|samples| samples.len() as u64).collect();
    sound_lengths.sort();
    assert_eq!(
        sound_lengths,
        vec![
            SECOND_KEPT * SAMPLES_PER_FRAME,
            FIRST_KEPT * SAMPLES_PER_FRAME
        ],
        "each reel's sound runs as long as its picture"
    );
    let verified = dcpwizard_core::verify::verify_dcp(&package);
    assert!(verified.valid, "verify errors: {:?}", verified.errors);
}

#[test]
fn one_trimmed_clip_builds_the_kept_frames() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let picture = write_clip(directory.path(), "only.mkv", SMALL_SIZE, FRAME_RATE);
    let timeline = write_timeline(
        directory.path(),
        vec![clip(picture, FIRST_IN, FIRST_OUT)],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    create(config_home.path(), &timeline, &output)
        .assert()
        .success();

    assert_eq!(
        reel_picture_durations(&output.join(TITLE)),
        vec![FIRST_KEPT]
    );
}

#[test]
fn check_on_a_timeline_writes_nothing() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let timeline = two_clip_timeline(directory.path());
    let output = directory.path().join("dcp");

    let run = create(config_home.path(), &timeline, &output)
        .arg("--check")
        .output()
        .unwrap();

    assert!(run.status.success(), "{}", printed(&run));
    assert!(printed(&run).contains("Pre-build check passed"));
    assert!(!output.exists(), "the check wrote {}", output.display());
}

#[test]
fn timeline_beside_video_is_refused() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let timeline = two_clip_timeline(directory.path());
    let output = directory.path().join("dcp");

    is_refused(
        create(config_home.path(), &timeline, &output)
            .arg("--video")
            .arg(directory.path().join("first.mkv")),
        &output,
        &["--video", "--timeline"],
    );
}

#[test]
fn timeline_beside_a_trim_is_refused() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let timeline = two_clip_timeline(directory.path());
    let output = directory.path().join("dcp");

    is_refused(
        create(config_home.path(), &timeline, &output).args(["--trim-start", "1s"]),
        &output,
        &["--trim-start", "--timeline"],
    );
}

#[test]
fn a_fade_on_two_clips_is_refused() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let timeline = two_clip_timeline(directory.path());
    let output = directory.path().join("dcp");

    is_refused(
        create(config_home.path(), &timeline, &output).args(["--video-fade-in", "0.5"]),
        &output,
        &["--video-fade-in"],
    );
}

#[test]
fn a_clip_at_another_rate_is_refused() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let first = write_clip(directory.path(), "first.mkv", SMALL_SIZE, FRAME_RATE);
    let other = write_clip(directory.path(), "pal.mkv", SMALL_SIZE, OTHER_FRAME_RATE);
    let timeline = write_timeline(
        directory.path(),
        vec![
            clip(first, FIRST_IN, FIRST_OUT),
            clip(other, SECOND_IN, SECOND_OUT),
        ],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    is_refused(
        &mut create(config_home.path(), &timeline, &output),
        &output,
        &["clip 2", "pal.mkv", "25/1 fps"],
    );
}

#[test]
fn a_clip_with_alpha_is_refused_by_name() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let first = write_clip(directory.path(), "first.mkv", SMALL_SIZE, FRAME_RATE);
    let alpha = directory.path().join("alpha.mov");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "testsrc=size={SMALL_SIZE}:rate={FRAME_RATE}:duration={CLIP_SECONDS}"
            ))
            .args(["-c:v", "png", "-pix_fmt", "rgba"])
            .arg(&alpha),
        "ffmpeg has to write the clip with alpha",
    );
    let timeline = write_timeline(
        directory.path(),
        vec![
            clip(first, FIRST_IN, FIRST_OUT),
            Clip {
                sound: ClipSound::None,
                ..clip(alpha, SECOND_IN, SECOND_OUT)
            },
        ],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    is_refused(
        create(config_home.path(), &timeline, &output).arg("--check"),
        &output,
        &["clip 2", "alpha.mov", "Input video has alpha"],
    );
}
