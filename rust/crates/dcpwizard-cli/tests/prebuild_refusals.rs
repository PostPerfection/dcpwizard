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

const PACKAGED_BITS_PER_SAMPLE: u32 = 24;

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

fn create_is_refused(command: &mut Command, out: &Path, naming: &[&str]) -> String {
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
    printed
}

#[test]
fn a_sixteen_bit_wav_is_packaged_at_the_dci_depth() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let wav = write_wav(directory.path(), "sixteen.wav", "pcm_s16le", 48_000);
    let out = directory.path().join("dcp");

    dcpwizard(config_home.path())
        .args([
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
        ])
        .assert()
        .success();

    let package = out.join("Sixteen Bit");
    let sound = only_file_starting_with(&package, "sound_");
    let mut reader = asdcplib::pcm::MxfReader::new();
    reader
        .open_read(&sound.to_string_lossy())
        .expect("the sound MXF has to open");
    assert_eq!(
        reader
            .audio_descriptor()
            .expect("audio descriptor")
            .quantization_bits,
        PACKAGED_BITS_PER_SAMPLE,
        "a 16-bit master has to be widened, not wrapped as it stands"
    );
    let verified = dcpwizard_core::verify::verify_dcp(&package);
    assert!(verified.valid, "dcpdoctor errors: {:?}", verified.errors);
}

#[test]
fn a_float_wav_is_refused_before_any_frame_is_encoded() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let wav = write_wav(directory.path(), "float.wav", "pcm_f32le", 48_000);
    let out = directory.path().join("dcp");

    create_is_refused(
        dcpwizard(config_home.path()).args([
            "create",
            "--title",
            "Float",
            "--video",
            source.to_str().unwrap(),
            "--audio",
            wav.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--twok",
        ]),
        &out,
        &["32-bit float", "pcm_s24le", "float.wav"],
    );
}

#[test]
fn a_thirty_two_bit_integer_wav_is_refused_before_any_frame_is_encoded() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let wav = write_wav(directory.path(), "thirty_two.wav", "pcm_s32le", 48_000);
    let out = directory.path().join("dcp");

    create_is_refused(
        dcpwizard(config_home.path()).args([
            "create",
            "--title",
            "Thirty Two Bit",
            "--video",
            source.to_str().unwrap(),
            "--audio",
            wav.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--twok",
        ]),
        &out,
        &["32-bit integer", "pcm_s24le", "thirty_two.wav"],
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

#[test]
fn an_unknown_content_type_is_refused_before_any_frame_is_encoded() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        dcpwizard(config_home.path()).args([
            "create",
            "--title",
            "Typo",
            "--video",
            source.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--twok",
            "--content-type",
            "TRL",
        ]),
        &out,
        &["unknown content type 'TRL'", "TLR"],
    );
}

#[test]
fn encrypting_without_a_signer_is_refused_before_any_frame_is_encoded() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        dcpwizard(config_home.path()).args([
            "create",
            "--title",
            "No Signer",
            "--video",
            source.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--twok",
            "--encrypt",
            "--key-out",
            directory.path().join("KEYS.json").to_str().unwrap(),
        ]),
        &out,
        &["signed CPL and PKL", "--signer-cert"],
    );
}

// the same job with a signer gets past the check and packages
#[test]
fn encrypting_with_a_signer_still_builds() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let certificates = directory.path().join("certs");
    dcpwizard(config_home.path())
        .args([
            "certificate",
            "chain",
            "--organization",
            "Prebuild Test",
            "-o",
            certificates.to_str().unwrap(),
        ])
        .assert()
        .success();

    let out = directory.path().join("dcp");
    dcpwizard(config_home.path())
        .args([
            "create",
            "--title",
            "Signed",
            "--video",
            source.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--twok",
            "--encrypt",
            "--key-out",
            directory.path().join("KEYS.json").to_str().unwrap(),
            "--signer-cert",
            certificates.join("signer.pem").to_str().unwrap(),
            "--signer-key",
            certificates.join("signer.key").to_str().unwrap(),
            "--signer-chain",
            certificates.join("intermediate.pem").to_str().unwrap(),
            "--signer-chain",
            certificates.join("root.pem").to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(out.join("Signed").join("ASSETMAP.xml").exists());
}

const REFUSED_CHECK: &str = "Pre-build check refused the job";
const FRAME_RATE_HINT: &str = "The DCP is 25 fps";

const CHECKED_TITLE: &str = "Checked";

fn checked_create(config_home: &Path, source: &Path, out: &Path) -> Command {
    let mut command = dcpwizard(config_home);
    command.args([
        "create",
        "--title",
        CHECKED_TITLE,
        "--video",
        source.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--twok",
        "--check",
    ]);
    command
}

fn write_still(directory: &Path) -> PathBuf {
    let path = directory.join("still.png");
    run_ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}"),
            "-frames:v",
            "1",
            path.to_str().unwrap(),
        ],
        "the still",
    );
    path
}

fn write_tagged_source(directory: &Path, name: &str, arguments: &[&str]) -> PathBuf {
    let path = directory.join(name);
    let size = format!("testsrc=size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}");
    let frames = FRAMES.to_string();
    let mut all = vec!["-f", "lavfi", "-i", &size, "-frames:v", &frames];
    all.extend_from_slice(arguments);
    all.push(path.to_str().unwrap());
    run_ffmpeg(&all, name);
    path
}

fn position(printed: &str, needle: &str) -> usize {
    printed
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} is missing: {printed}"))
}

#[test]
fn check_prints_every_refusal_of_a_job_with_two_faults() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let wav = write_wav(directory.path(), "float.wav", "pcm_f32le", 48_000);
    let out = directory.path().join("dcp");

    let printed = create_is_refused(
        checked_create(config_home.path(), &source, &out).args([
            "--audio",
            wav.to_str().unwrap(),
            "--encrypt",
            "--key-out",
            directory.path().join("KEYS.json").to_str().unwrap(),
        ]),
        &out,
        &["32-bit float", "signed CPL and PKL", "2 refusal(s)"],
    );
    assert!(
        position(&printed, "32-bit float") < position(&printed, "signed CPL and PKL"),
        "the refusals keep the check order: {printed}"
    );
}

#[test]
fn check_prints_the_hints_after_the_refusals() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    let printed = create_is_refused(
        checked_create(config_home.path(), &source, &out).args([
            "--frame-rate",
            "25",
            "--encrypt",
            "--key-out",
            directory.path().join("KEYS.json").to_str().unwrap(),
        ]),
        &out,
        &["signed CPL and PKL", FRAME_RATE_HINT, REFUSED_CHECK],
    );
    assert!(
        position(&printed, "signed CPL and PKL") < position(&printed, FRAME_RATE_HINT),
        "every refusal prints before the first hint: {printed}"
    );
}

#[test]
fn a_video_ffmpeg_cannot_read_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = directory.path().join("unreadable.mp4");
    std::fs::write(&source, b"this is not a video").unwrap();
    let out = directory.path().join("dcp");

    create_is_refused(
        &mut checked_create(config_home.path(), &source, &out),
        &out,
        &["ffprobe could not inspect", "unreadable.mp4", REFUSED_CHECK],
    );
}

#[test]
fn a_video_with_alpha_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_tagged_source(
        directory.path(),
        "alpha.mov",
        &["-c:v", "png", "-pix_fmt", "rgba"],
    );
    let out = directory.path().join("dcp");

    create_is_refused(
        &mut checked_create(config_home.path(), &source, &out),
        &out,
        &["Input video has alpha", REFUSED_CHECK],
    );
}

#[test]
fn an_hdr_source_with_no_path_to_dci_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_tagged_source(
        directory.path(),
        "pq.mkv",
        &[
            "-vf",
            "setparams=color_primaries=bt2020:color_trc=smpte2084:colorspace=bt2020nc",
            "-pix_fmt",
            "yuv420p10le",
            "-c:v",
            "libx265",
            "-x265-params",
            "log-level=none",
        ],
    );
    let out = directory.path().join("dcp");

    create_is_refused(
        &mut checked_create(config_home.path(), &source, &out),
        &out,
        &["HDR source requires --hdr-to-dci-lut", REFUSED_CHECK],
    );
}

#[test]
fn a_missing_hdr_to_dci_lut_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let lut = directory.path().join("missing.cube");
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &source, &out)
            .args(["--hdr-to-dci-lut", lut.to_str().unwrap()]),
        &out,
        &["HDR-to-DCI LUT not found", "missing.cube", REFUSED_CHECK],
    );
}

#[test]
fn a_trim_that_leaves_nothing_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &source, &out).args([
            "--trim-start",
            "2f",
            "--trim-end",
            "2f",
        ]),
        &out,
        &["leaves nothing of the 3-frame source", REFUSED_CHECK],
    );
}

#[test]
fn a_video_fade_longer_than_the_picture_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &source, &out).args(["--video-fade-in", "10"]),
        &out,
        &["video fade-in of 10s is longer than", REFUSED_CHECK],
    );
}

#[test]
fn hdr_dci_with_a_right_eye_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &source, &out).args([
            "--hdr-dci",
            "--hdr-already-pq",
            "--right-eye",
            source.to_str().unwrap(),
        ]),
        &out,
        &[
            "--hdr-dci is not supported for stereoscopic (3D) DCPs",
            REFUSED_CHECK,
        ],
    );
}

#[test]
fn an_input_range_on_a_still_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let still = write_still(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &still, &out).args([
            "--still-length",
            "1s",
            "--input-range",
            "full",
        ]),
        &out,
        &["--input-range applies to a video input", REFUSED_CHECK],
    );
}

#[test]
fn a_still_held_for_no_frames_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let still = write_still(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &still, &out).args(["--still-length", "0f"]),
        &out,
        &["held for at least one frame", REFUSED_CHECK],
    );
}

// the saved state names another source, so these frames are not this job's
#[test]
fn a_resume_onto_another_encode_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");
    let package = out.join(CHECKED_TITLE);
    std::fs::create_dir_all(&package).unwrap();
    let saved_state = package.join(".dcpwizard-encode.json");
    let saved = format!(
        r#"{{"source":"another.mp4","total_frames":{FRAMES},"fps":{FRAME_RATE},"width":{WIDTH},"height":{HEIGHT},"bitrate_mbps":0}}"#
    );
    std::fs::write(&saved_state, &saved).unwrap();

    let run = checked_create(config_home.path(), &source, &out)
        .arg("--resume")
        .output()
        .expect("dcpwizard has to run");
    let printed =
        String::from_utf8_lossy(&run.stdout).into_owned() + &String::from_utf8_lossy(&run.stderr);
    assert!(
        !run.status.success(),
        "the resume was not refused: {printed}"
    );
    for needle in ["--resume", "holds a different encode", REFUSED_CHECK] {
        assert!(
            printed.contains(needle),
            "the refusal has to name {needle}: {printed}"
        );
    }
    let left: Vec<PathBuf> = std::fs::read_dir(&package)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    assert_eq!(left, vec![saved_state.clone()], "the check writes nothing");
    assert_eq!(std::fs::read_to_string(&saved_state).unwrap(), saved);
}

#[test]
fn a_loudness_target_that_does_not_parse_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &source, &out).args(["--loudness-target", "85"]),
        &out,
        &["loudness target '85' must be metric=value", REFUSED_CHECK],
    );
}

#[test]
fn a_marker_label_that_does_not_exist_is_refused_by_the_check() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &source, &out).args(["--marker", "BOGUS=1"]),
        &out,
        &["unknown marker label 'BOGUS'", REFUSED_CHECK],
    );
}

const QUALITY_PSNR_OUT_OF_RANGE: &str = "1000";
const QUALITY_PSNR_REFUSAL: &str = "--quality-psnr 1000 is outside the range";

#[test]
fn check_prints_every_flag_refusal_of_a_job_with_two_bad_flags() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &source, &out).args([
            "--content-type",
            "TRL",
            "--quality-psnr",
            QUALITY_PSNR_OUT_OF_RANGE,
        ]),
        &out,
        &[
            "unknown content type 'TRL'",
            QUALITY_PSNR_REFUSAL,
            "2 refusal(s)",
        ],
    );
}

#[test]
fn check_prints_a_flag_refusal_beside_a_plan_refusal() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        checked_create(config_home.path(), &source, &out).args([
            "--quality-psnr",
            QUALITY_PSNR_OUT_OF_RANGE,
            "--encrypt",
            "--key-out",
            directory.path().join("KEYS.json").to_str().unwrap(),
        ]),
        &out,
        &[QUALITY_PSNR_REFUSAL, "signed CPL and PKL", "2 refusal(s)"],
    );
}

const START_AN_HOUR_AHEAD: &str = "+1h";
const REFUSAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

#[test]
fn a_refused_job_with_a_later_start_exits_without_waiting() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let source = write_source(directory.path());
    let out = directory.path().join("dcp");

    create_is_refused(
        dcpwizard(config_home.path())
            .args([
                "create",
                "--title",
                "Later",
                "--video",
                source.to_str().unwrap(),
                "-o",
                out.to_str().unwrap(),
                "--twok",
                "--start-at",
                START_AN_HOUR_AHEAD,
                "--encrypt",
                "--key-out",
                directory.path().join("KEYS.json").to_str().unwrap(),
            ])
            .timeout(REFUSAL_TIMEOUT),
        &out,
        &["signed CPL and PKL"],
    );
}
