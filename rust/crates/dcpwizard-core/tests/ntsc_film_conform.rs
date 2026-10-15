//! An N*1000/1001 source packaged at N fps: the picture is read 1:1 and plays
//! 0.1% faster, and the sound is pulled up by the same 1000/1001, so both tracks
//! hold exactly as many frames as the source did.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use dcpwizard_core::dcp::{DcpConfig, create_dcp};

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const NTSC_RATE_DENOMINATOR: u32 = 1_001;
const AUDIO_SAMPLE_RATE: u64 = 48_000;
// aresample can land a sample either side of the exact count
const PULL_UP_SAMPLE_TOLERANCE: u64 = 2;

fn make_ntsc_clip(path: &Path, fps_num: u32, frames: u32) {
    let ok = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=size={WIDTH}x{HEIGHT}:rate={fps_num}/{NTSC_RATE_DENOMINATOR}"),
            "-frames:v",
            &frames.to_string(),
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(path)
        .output()
        .expect("run ffmpeg");
    assert!(
        ok.status.success() && path.exists(),
        "ffmpeg wrote no {fps_num}/{NTSC_RATE_DENOMINATOR} clip
  stdout: {}
  stderr: {}",
        String::from_utf8_lossy(&ok.stdout).trim(),
        String::from_utf8_lossy(&ok.stderr).trim(),
    );
}

fn make_sine(path: &Path, samples: u64) {
    let ok = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-af",
            &format!("atrim=end_sample={samples}"),
            "-ac",
            "2",
            "-c:a",
            "pcm_s24le",
        ])
        .arg(path)
        .output()
        .expect("run ffmpeg");
    assert!(
        ok.status.success() && path.exists(),
        "{}
  stdout: {}
  stderr: {}",
        "ffmpeg wrote no sine",
        String::from_utf8_lossy(&ok.stdout).trim(),
        String::from_utf8_lossy(&ok.stderr).trim(),
    );
}

fn find_essence(dir: &Path, prefix: &str) -> std::path::PathBuf {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".mxf"))
        })
        .unwrap_or_else(|| panic!("{prefix} MXF written to {}", dir.display()))
}

fn wav_sample_count(path: &Path) -> u64 {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=duration_ts",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .expect("ffprobe");
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("a sample count")
}

fn package_ntsc_source(fps_num: u32, dcp_fps: u32, source_frames: u32) {
    let dir = tempfile::tempdir().unwrap();
    let clip = dir.path().join("ntsc.mp4");
    let sine = dir.path().join("sound.wav");
    // the sound covers the whole source, which runs 1001/fps_num s a frame
    let source_samples =
        u64::from(source_frames) * u64::from(NTSC_RATE_DENOMINATOR) * AUDIO_SAMPLE_RATE
            / u64::from(fps_num);
    make_ntsc_clip(&clip, fps_num, source_frames);
    make_sine(&sine, source_samples);

    let probed = dcpwizard_core::probe::probe_video(&clip).expect("probe the source");
    assert_eq!(
        (probed.fps_num, probed.fps_den),
        (fps_num, NTSC_RATE_DENOMINATOR)
    );
    assert_eq!(
        dcpwizard_core::hfr::source_rate_to_dcp(probed.fps_num, probed.fps_den),
        (dcp_fps, true),
        "the source's own rate picks the DCP rate"
    );

    let conform =
        dcpwizard_core::hfr::conform_source_to_dcp(probed.fps_num, probed.fps_den, dcp_fps);
    assert!(conform.audio_pull_up);

    let work = dir.path().join("encode");
    let encoded = postkit::pipeline::run_encode_with_options(
        &clip,
        &work,
        &postkit::pipeline::EncodeRunOptions {
            fps: postkit::encode::FrameRate::whole(dcp_fps),
            read_source_at: conform.read_source_at,
            ..postkit::pipeline::EncodeRunOptions::default()
        },
        &Arc::new(AtomicBool::new(false)),
        &Arc::new(AtomicBool::new(false)),
        |_progress| {},
        |_message| {},
    )
    .expect("the conformed encode runs");
    assert_eq!(
        encoded.frames_encoded,
        u64::from(source_frames),
        "every source frame is packaged once, none duplicated"
    );

    let pulled_up = dir.path().join("pullup.wav");
    dcpwizard_core::hfr::audio_pull_up(&sine, &pulled_up).expect("pull the sound up");
    let samples = wav_sample_count(&pulled_up);
    let picture_samples = u64::from(source_frames) * AUDIO_SAMPLE_RATE / u64::from(dcp_fps);
    assert!(
        samples.abs_diff(picture_samples) <= PULL_UP_SAMPLE_TOLERANCE,
        "{samples} samples against {picture_samples} of picture"
    );

    let out = dir.path().join("dcp");
    let config = DcpConfig {
        title: "NTSC Conform".into(),
        standard: dcpwizard_core::Standard::Smpte,
        resolution: dcpwizard_core::Resolution::TwoK,
        content_type: dcpwizard_core::ContentType::Test,
        frame_rate_num: dcp_fps,
        frame_rate_den: 1,
        output_dir: out.clone(),
        j2k_dir: Some(encoded.j2k_dir),
        audio_path: Some(pulled_up),
        ..Default::default()
    };
    assert_eq!(create_dcp(&config), 0, "the conformed DCP must package");

    let cpl = dcpwizard_core::info::inspect_dcp(&out).expect("read the CPL back");
    assert_eq!(cpl.frame_rate, format!("{dcp_fps} 1"));
    assert_eq!(cpl.duration_frames, i64::from(source_frames));

    let mut picture = asdcplib::jp2k::MxfReader::new();
    picture
        .open_read(find_essence(&out, "picture").to_str().unwrap())
        .expect("open picture MXF");
    let picture = picture.picture_descriptor().expect("picture descriptor");
    assert_eq!(
        (picture.edit_rate.numerator, picture.edit_rate.denominator),
        (dcp_fps as i32, 1)
    );
    assert_eq!(
        picture.container_duration, source_frames,
        "an fps filter on its own would have retimed the source onto {dcp_fps} fps"
    );

    let mut sound = asdcplib::pcm::MxfReader::new();
    sound
        .open_read(find_essence(&out, "sound").to_str().unwrap())
        .expect("open sound MXF");
    let sound = sound.audio_descriptor().expect("audio descriptor");
    assert_eq!(
        (sound.edit_rate.numerator, sound.edit_rate.denominator),
        (dcp_fps as i32, 1)
    );
    assert_eq!(
        sound.container_duration, source_frames,
        "the pulled-up sound covers the picture to the frame"
    );

    let result = dcpwizard_core::verify::verify_dcp(&out);
    assert!(
        result.errors.is_empty(),
        "verify errors: {:?}",
        result.errors
    );
}

#[test]
fn a_23_976_source_packages_1001_frames_of_picture_and_sound_at_24_fps() {
    // one frame in every 1001 is where a plain fps filter would duplicate
    package_ntsc_source(24_000, 24, 1001);
}

#[test]
fn a_29_97_source_packages_every_frame_of_picture_and_sound_at_30_fps() {
    package_ntsc_source(30_000, 30, 60);
}

#[test]
fn a_59_94_source_packages_every_frame_of_picture_and_sound_at_60_fps() {
    package_ntsc_source(60_000, 60, 120);
}
