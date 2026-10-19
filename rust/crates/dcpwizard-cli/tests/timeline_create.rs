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

fn clip(picture: PathBuf, start: u64, in_frame: u64, out_frame: u64) -> Clip {
    Clip {
        picture,
        start,
        layer: 0,
        opacity: 1.0,
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
            clip(first, 0, FIRST_IN, FIRST_OUT),
            clip(second, FIRST_KEPT, SECOND_IN, SECOND_OUT),
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
        vec![clip(picture, 0, FIRST_IN, FIRST_OUT)],
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
            clip(first, 0, FIRST_IN, FIRST_OUT),
            clip(other, FIRST_KEPT, SECOND_IN, SECOND_OUT),
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

fn write_movie_with_alpha(directory: &Path) -> PathBuf {
    let alpha = directory.join("alpha.mov");
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
    alpha
}

#[test]
fn a_clip_with_alpha_is_refused_by_name() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let first = write_clip(directory.path(), "first.mkv", SMALL_SIZE, FRAME_RATE);
    let alpha = write_movie_with_alpha(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![
            clip(first, 0, FIRST_IN, FIRST_OUT),
            Clip {
                sound: ClipSound::None,
                ..clip(alpha, FIRST_KEPT, SECOND_IN, SECOND_OUT)
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

#[test]
fn a_clip_with_alpha_above_layer_0_passes_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let first = write_clip(directory.path(), "first.mkv", SMALL_SIZE, FRAME_RATE);
    let alpha = write_movie_with_alpha(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![
            clip(first, 0, FIRST_IN, FIRST_OUT),
            Clip {
                layer: 1,
                sound: ClipSound::None,
                ..clip(alpha, 0, SECOND_IN, SECOND_OUT)
            },
        ],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    let run = create(config_home.path(), &timeline, &output)
        .arg("--check")
        .output()
        .unwrap();

    assert!(run.status.success(), "{}", printed(&run));
    assert!(printed(&run).contains("Pre-build check passed"));
}

const LAYERED_SIZE: (u32, u32) = (320, 180);
// a reel runs at least a second
const GAP_FRAMES: u64 = 24;
const BASE_SECONDS: u64 = 2;
const BASE_FRAMES: u64 = BASE_SECONDS * FRAME_RATE as u64;
const OVERLAY_START: u64 = GAP_FRAMES + 4;
const OVERLAY_FRAMES: u64 = 8;
const CODESTREAM_BUFFER_BYTES: usize = 4 * 1024 * 1024;

// rgb all the way, with a tone under it
fn write_blue_clip(directory: &Path) -> PathBuf {
    let path = directory.join("blue.mkv");
    let (width, height) = LAYERED_SIZE;
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "color=c=blue:s={width}x{height}:rate={FRAME_RATE}:duration={BASE_SECONDS}"
            ))
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:sample_rate={SAMPLE_RATE}:duration={BASE_SECONDS}"
            ))
            .args(["-pix_fmt", "gbrp", "-c:v", "ffv1", "-c:a", "pcm_s24le"])
            .arg(&path),
        "ffmpeg has to write the blue clip",
    );
    path
}

// red over the left half, transparent over the right
fn write_half_red_stills(directory: &Path) -> PathBuf {
    let stills = directory.join("half_red");
    std::fs::create_dir_all(&stills).unwrap();
    let (width, height) = LAYERED_SIZE;
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "color=c=red:s={}x{height}:rate={FRAME_RATE}",
                width / 2
            ))
            .arg("-vf")
            .arg(format!(
                "format=rgba,pad={width}:{height}:0:0:color=black@0"
            ))
            .args(["-frames:v", &OVERLAY_FRAMES.to_string(), "-pix_fmt", "rgba"])
            .arg(stills.join("frame_%04d.png")),
        "ffmpeg has to write the stills with alpha",
    );
    stills
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Seen {
    Black,
    Red,
    Blue,
}

// X'Y'Z' codes: red carries far more X than Z, blue far more Z than X
fn seen(xyz: [i32; 3]) -> Option<Seen> {
    const BLACK_CEILING: i32 = 200;
    const MARGIN: i32 = 1000;
    let [x, _, z] = xyz;
    if xyz.iter().all(|&code| code < BLACK_CEILING) {
        return Some(Seen::Black);
    }
    if x - z > MARGIN {
        return Some(Seen::Red);
    }
    if z - x > MARGIN {
        return Some(Seen::Blue);
    }
    None
}

// every frame of a picture MXF, sampled inside the left half and inside the right half
fn left_and_right_of_every_frame(picture_mxf: &Path) -> Vec<[Option<Seen>; 2]> {
    let mut reader = asdcplib::jp2k::MxfReader::new();
    reader
        .open_read(&picture_mxf.to_string_lossy())
        .expect("the picture MXF has to open");
    let frames = reader
        .picture_descriptor()
        .expect("picture descriptor")
        .container_duration;
    let mut buffer = vec![0u8; CODESTREAM_BUFFER_BYTES];
    (0..frames)
        .map(|index| {
            let read = reader
                .read_frame(index, &mut buffer, None, None)
                .unwrap_or_else(|e| panic!("frame {index} has to read: {e}"));
            let frame = postkit::grok_decoder::decode(buffer[..read].to_vec(), 0)
                .unwrap_or_else(|e| panic!("frame {index} has to decode: {e}"));
            let row = (frame.height / 2 * frame.width) as usize;
            [frame.width / 8, frame.width * 7 / 8].map(|x| {
                let at = row + x as usize;
                seen([0, 1, 2].map(|component| frame.components[component][at]))
            })
        })
        .collect()
}

fn picture_frames_by_length(package: &Path) -> Vec<Vec<[Option<Seen>; 2]>> {
    let mut reels: Vec<Vec<[Option<Seen>; 2]>> = files_starting_with(package, "picture_")
        .iter()
        .map(|picture| left_and_right_of_every_frame(picture))
        .collect();
    reels.sort_by_key(Vec::len);
    reels
}

#[test]
fn a_layer_with_alpha_composites_over_the_base_and_a_gap_is_black() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let blue = write_blue_clip(directory.path());
    let half_red = write_half_red_stills(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![
            clip(blue, GAP_FRAMES, 0, BASE_FRAMES),
            Clip {
                layer: 1,
                sound: ClipSound::None,
                ..clip(half_red, OVERLAY_START, 0, OVERLAY_FRAMES)
            },
        ],
        vec![GAP_FRAMES],
    );
    let output = directory.path().join("dcp");

    let run = create(config_home.path(), &timeline, &output)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", printed(&run));
    let package = output.join(TITLE);

    assert_eq!(
        reel_picture_durations(&package),
        vec![GAP_FRAMES, BASE_FRAMES],
        "the gap is a reel of its own and the base clip fills the second"
    );
    let reels = picture_frames_by_length(&package);
    assert_eq!(reels.len(), 2);
    for (frame, samples) in reels[0].iter().enumerate() {
        assert_eq!(
            samples,
            &[Some(Seen::Black); 2],
            "gap frame {frame} is black"
        );
    }
    let overlaid = (OVERLAY_START - GAP_FRAMES)..(OVERLAY_START - GAP_FRAMES + OVERLAY_FRAMES);
    for (frame, samples) in reels[1].iter().enumerate() {
        let want = if overlaid.contains(&(frame as u64)) {
            [Some(Seen::Red), Some(Seen::Blue)]
        } else {
            [Some(Seen::Blue); 2]
        };
        assert_eq!(samples, &want, "reel 2 frame {frame}");
    }

    let mut sounds: Vec<Vec<i32>> = files_starting_with(&package, "sound_")
        .iter()
        .map(|sound| sound_samples(sound))
        .collect();
    sounds.sort_by_key(Vec::len);
    assert_eq!(
        sounds.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![
            (GAP_FRAMES * SAMPLES_PER_FRAME) as usize,
            (BASE_FRAMES * SAMPLES_PER_FRAME) as usize
        ]
    );
    assert!(
        sounds[0].iter().all(|&sample| sample == 0),
        "the gap is silent"
    );
    assert!(
        sounds[1].iter().any(|&sample| sample != 0),
        "the base clip's tone plays"
    );
    let verified = dcpwizard_core::verify::verify_dcp(&package);
    assert!(verified.valid, "verify errors: {:?}", verified.errors);
}

#[test]
fn an_image_sequence_clip_is_checked_as_a_picture_to_encode() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let blue = write_blue_clip(directory.path());
    let half_red = write_half_red_stills(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![
            clip(blue, 0, 0, BASE_FRAMES),
            Clip {
                layer: 1,
                sound: ClipSound::None,
                ..clip(half_red, 0, 0, OVERLAY_FRAMES)
            },
        ],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    let run = create(config_home.path(), &timeline, &output)
        .args(["--source-colourspace", "p3", "--check"])
        .output()
        .unwrap();

    assert!(run.status.success(), "{}", printed(&run));
    assert!(printed(&run).contains("Pre-build check passed"));
}

const CODESTREAM_SIZE: &str = "1998x1080";

// a second of blue stills compressed by the encode command, as a mastered J2K sequence arrives
fn write_codestreams(directory: &Path) -> PathBuf {
    let stills = directory.join("stills");
    std::fs::create_dir_all(&stills).unwrap();
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "color=c=blue:s={CODESTREAM_SIZE}:rate={FRAME_RATE}:duration={BASE_SECONDS}"
            ))
            .arg(stills.join("frame_%04d.png")),
        "ffmpeg has to write the stills",
    );
    let encoded = directory.join("encoded");
    Command::cargo_bin("dcpwizard")
        .unwrap()
        .args(["encode", "--input"])
        .arg(&stills)
        .arg("--output")
        .arg(&encoded)
        .args(["--bandwidth", "250"])
        .assert()
        .success();
    encoded.join("j2k")
}

fn write_tone(path: &Path, frames: u64) {
    let seconds = frames as f64 / f64::from(FRAME_RATE);
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:sample_rate={SAMPLE_RATE}:duration={seconds}"
            ))
            .args(["-c:a", "pcm_s24le", "-ac", "2"])
            .arg(path),
        "ffmpeg has to write the tone",
    );
}

#[test]
fn codestreams_on_layer_0_after_a_gap_build_with_the_gap_black() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let codestreams = write_codestreams(directory.path());
    let tone = directory.path().join("tone.wav");
    write_tone(&tone, GAP_FRAMES + BASE_FRAMES);
    let timeline = directory.path().join("timeline.json");
    Timeline {
        version: TIMELINE_FORMAT_VERSION,
        fps: FrameRate::whole(FRAME_RATE),
        clips: vec![Clip {
            sound: ClipSound::None,
            ..clip(codestreams, GAP_FRAMES, 0, BASE_FRAMES)
        }],
        sound: SoundTrack::Override(tone),
        subtitle: None,
        reel_starts: vec![GAP_FRAMES],
    }
    .write(&timeline)
    .unwrap();
    let output = directory.path().join("dcp");

    let run = create(config_home.path(), &timeline, &output)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", printed(&run));
    let package = output.join(TITLE);

    assert_eq!(
        reel_picture_durations(&package),
        vec![GAP_FRAMES, BASE_FRAMES],
        "the gap is a reel of its own and the codestreams fill the second"
    );
    let reels = picture_frames_by_length(&package);
    assert!(
        reels[0]
            .iter()
            .all(|samples| samples == &[Some(Seen::Black); 2]),
        "every gap frame is black"
    );
    assert!(
        reels[1]
            .iter()
            .all(|samples| samples == &[Some(Seen::Blue); 2]),
        "every codestream frame is the blue it was encoded from"
    );
    let verified = dcpwizard_core::verify::verify_dcp(&package);
    assert!(verified.valid, "verify errors: {:?}", verified.errors);
}

const LEAD_FRAMES: u64 = 24;
const HELD_FRAMES: u64 = 48;

// one red picture at the blue clip's size
fn write_red_still(directory: &Path) -> PathBuf {
    let path = directory.join("card.png");
    let (width, height) = LAYERED_SIZE;
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!("color=c=red:s={width}x{height}"))
            .args(["-frames:v", "1"])
            .arg(&path),
        "ffmpeg has to write the still",
    );
    path
}

fn held_still(picture: PathBuf, start: u64, out_frame: u64) -> Clip {
    Clip {
        sound: ClipSound::None,
        ..clip(picture, start, 0, out_frame)
    }
}

#[test]
fn a_still_between_two_video_clips_is_held_for_its_out_point() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let blue = write_blue_clip(directory.path());
    let card = write_red_still(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![
            clip(blue.clone(), 0, 0, LEAD_FRAMES),
            held_still(card, LEAD_FRAMES, HELD_FRAMES),
            clip(blue, LEAD_FRAMES + HELD_FRAMES, 0, LEAD_FRAMES),
        ],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    let run = create(config_home.path(), &timeline, &output)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", printed(&run));
    let package = output.join(TITLE);

    assert_eq!(
        reel_picture_durations(&package),
        vec![LEAD_FRAMES + HELD_FRAMES + LEAD_FRAMES]
    );
    let reels = picture_frames_by_length(&package);
    let hold = LEAD_FRAMES..LEAD_FRAMES + HELD_FRAMES;
    for (frame, samples) in reels[0].iter().enumerate() {
        let want = if hold.contains(&(frame as u64)) {
            Seen::Red
        } else {
            Seen::Blue
        };
        assert_eq!(samples, &[Some(want); 2], "frame {frame}");
    }
    let verified = dcpwizard_core::verify::verify_dcp(&package);
    assert!(verified.valid, "verify errors: {:?}", verified.errors);
}

#[test]
fn one_held_still_builds_the_length_the_still_length_flag_builds() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let card = write_red_still(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![held_still(card.clone(), 0, HELD_FRAMES)],
        Vec::new(),
    );
    let from_timeline = directory.path().join("from_timeline");
    let from_flag = directory.path().join("from_flag");

    create(config_home.path(), &timeline, &from_timeline)
        .assert()
        .success();
    Command::cargo_bin("dcpwizard")
        .unwrap()
        .env("XDG_CONFIG_HOME", config_home.path())
        .args(["create", "--title", TITLE, "--video"])
        .arg(&card)
        .args(["--still-length", &format!("{HELD_FRAMES}f"), "-o"])
        .arg(&from_flag)
        .assert()
        .success();

    let durations = reel_picture_durations(&from_timeline.join(TITLE));
    assert_eq!(durations, vec![HELD_FRAMES]);
    assert_eq!(durations, reel_picture_durations(&from_flag.join(TITLE)));
}

#[test]
fn a_still_with_an_in_point_is_refused_by_name() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let blue = write_blue_clip(directory.path());
    let card = write_red_still(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![
            clip(blue, 0, 0, LEAD_FRAMES),
            Clip {
                in_frame: 3,
                ..held_still(card, LEAD_FRAMES, HELD_FRAMES)
            },
        ],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    is_refused(
        &mut create(config_home.path(), &timeline, &output),
        &output,
        &[
            "clip 2",
            "card.png",
            "the in point is 3, and a still is held from its first frame",
        ],
    );
}

#[test]
fn a_timeline_opening_on_a_still_builds_the_still_then_the_video() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let blue = write_blue_clip(directory.path());
    let card = write_red_still(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![
            held_still(card, 0, LEAD_FRAMES),
            clip(blue, LEAD_FRAMES, 0, LEAD_FRAMES),
        ],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    let run = create(config_home.path(), &timeline, &output)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", printed(&run));
    let package = output.join(TITLE);

    assert_eq!(reel_picture_durations(&package), vec![2 * LEAD_FRAMES]);
    let reels = picture_frames_by_length(&package);
    for (frame, samples) in reels[0].iter().enumerate() {
        let want = if (frame as u64) < LEAD_FRAMES {
            Seen::Red
        } else {
            Seen::Blue
        };
        assert_eq!(samples, &[Some(want); 2], "frame {frame}");
    }
}

fn write_surround_clip(directory: &Path) -> PathBuf {
    let path = directory.join("surround.mkv");
    let (width, height) = LAYERED_SIZE;
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "color=c=blue:s={width}x{height}:rate={FRAME_RATE}:duration={BASE_SECONDS}"
            ))
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "anullsrc=channel_layout=5.1:sample_rate={SAMPLE_RATE}:duration={BASE_SECONDS}"
            ))
            .args(["-c:v", "ffv1", "-c:a", "pcm_s24le"])
            .arg(&path),
        "ffmpeg has to write the surround clip",
    );
    path
}

#[test]
fn a_timeline_opening_on_a_still_is_named_for_the_sound_its_clips_carry() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let card = write_red_still(directory.path());
    let surround = write_surround_clip(directory.path());
    let timeline = write_timeline(
        directory.path(),
        vec![
            held_still(card, 0, LEAD_FRAMES),
            clip(surround, LEAD_FRAMES, 0, LEAD_FRAMES),
        ],
        Vec::new(),
    );
    let output = directory.path().join("dcp");

    let run = create(config_home.path(), &timeline, &output)
        .arg("--isdcf-name")
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", printed(&run));

    let packages: Vec<String> = std::fs::read_dir(&output)
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(packages.len(), 1, "{packages:?}");
    assert!(packages[0].contains("_51_"), "{}", packages[0]);
}
