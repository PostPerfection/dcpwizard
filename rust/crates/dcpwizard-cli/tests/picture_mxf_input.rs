use assert_cmd::Command;
use postkit::mxf_wrap::{
    EssenceType, MxfEncryption, MxfStandard, MxfWrapOptions, StereoscopicWrapOptions, mxf_wrap,
    wrap_stereoscopic,
};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const FRAMES: usize = 4;
const FRAME_RATE: u32 = 24;
const OTHER_FRAME_RATE: u32 = 25;
const CODESTREAM_BUFFER_BYTES: usize = 4 * 1024 * 1024;
const TITLE: &str = "Picture MXF";

fn cinema_codestream() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../extern/postkit/tests/fixtures/cinema2k_64x64.j2c"),
    )
    .expect("the postkit submodule holds the cinema 2K codestream")
}

// each frame carries its index after the codestream
fn write_picture_mxf(
    directory: &Path,
    name: &str,
    encryption: Option<MxfEncryption>,
) -> (PathBuf, Vec<Vec<u8>>) {
    let frames: Vec<Vec<u8>> = (0..FRAMES)
        .map(|index| {
            let mut frame = cinema_codestream();
            frame.extend_from_slice(format!("{name}{index:04}").as_bytes());
            frame
        })
        .collect();
    let input_files = frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let path = directory.join(format!("{name}_{index}.j2c"));
            std::fs::write(&path, frame).unwrap();
            path
        })
        .collect();
    let mxf = directory.join(format!("{name}.mxf"));
    let track = mxf_wrap(&MxfWrapOptions {
        input_files,
        output: mxf.clone(),
        essence_type: EssenceType::J2k,
        standard: MxfStandard::AsDcp,
        fps_num: FRAME_RATE,
        fps_den: 1,
        partition_size: 0,
        encryption,
        mca_config: None,
        resource_ids: Vec::new(),
        hdr: None,
        asset_uuid: None,
        timed_text_duration_frames: None,
    });
    assert!(track.success, "wrap failed: {}", track.error);
    (mxf, frames)
}

fn write_tone(directory: &Path) -> PathBuf {
    let wav = directory.join("tone.wav");
    let made = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
        .arg("sine=frequency=440:duration=1:sample_rate=48000")
        .args(["-ac", "2", "-c:a", "pcm_s24le"])
        .arg(&wav)
        .output()
        .expect("ffmpeg has to run");
    assert!(
        made.status.success(),
        "ffmpeg could not write the tone: {}",
        String::from_utf8_lossy(&made.stderr)
    );
    wav
}

fn create(config_home: &Path, mxf: &Path, wav: &Path, output: &Path) -> Command {
    let mut command = Command::cargo_bin("dcpwizard").unwrap();
    command
        .env("XDG_CONFIG_HOME", config_home)
        .args(["create", "--title", TITLE, "--video"])
        .arg(mxf)
        .arg("--audio")
        .arg(wav)
        .arg("-o")
        .arg(output);
    command
}

fn create_is_refused(command: &mut Command, output: &Path, expected: &str) {
    let run = command.output().expect("dcpwizard has to run");
    let printed =
        String::from_utf8_lossy(&run.stdout).into_owned() + &String::from_utf8_lossy(&run.stderr);
    assert!(
        !run.status.success(),
        "the build was not refused: {printed}"
    );
    assert!(
        printed.contains(expected),
        "the refusal has to read {expected}: {printed}"
    );
    assert!(
        !output.exists(),
        "a refusal before the build writes nothing under {}",
        output.display()
    );
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

#[test]
fn a_picture_mxf_is_packaged_with_its_codestreams_unchanged() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let (mxf, frames) = write_picture_mxf(directory.path(), "source", None);
    let wav = write_tone(directory.path());
    let output = directory.path().join("dcp");

    create(config_home.path(), &mxf, &wav, &output)
        .assert()
        .success();

    let package = output.join(TITLE);
    assert!(
        package.join("ASSETMAP.xml").is_file(),
        "{} has no ASSETMAP",
        package.display()
    );
    let picture = only_file_starting_with(&package, "picture_");
    let mut reader = asdcplib::jp2k::MxfReader::new();
    reader
        .open_read(&picture.to_string_lossy())
        .expect("the picture MXF has to open");
    let descriptor = reader.picture_descriptor().expect("picture descriptor");
    assert_eq!(descriptor.container_duration, FRAMES as u32);
    assert_eq!(
        (
            descriptor.edit_rate.numerator,
            descriptor.edit_rate.denominator
        ),
        (FRAME_RATE as i32, 1)
    );
    let mut buffer = vec![0u8; CODESTREAM_BUFFER_BYTES];
    for (index, frame) in frames.iter().enumerate() {
        let read = reader
            .read_frame(index as u32, &mut buffer, None, None)
            .unwrap_or_else(|e| panic!("frame {index} has to read: {e}"));
        assert!(
            buffer[..read] == frame[..],
            "frame {index} differs from the codestream wrapped into the source MXF"
        );
    }
}

#[test]
fn a_frame_rate_other_than_the_picture_mxf_edit_rate_is_refused_before_the_build() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let (mxf, _) = write_picture_mxf(directory.path(), "source", None);
    let wav = write_tone(directory.path());
    let output = directory.path().join("dcp");

    create_is_refused(
        create(config_home.path(), &mxf, &wav, &output)
            .args(["--frame-rate", &OTHER_FRAME_RATE.to_string()]),
        &output,
        &format!(
            "{} runs at {FRAME_RATE} fps, the job asks for {OTHER_FRAME_RATE} fps",
            mxf.display()
        ),
    );
}

#[test]
fn an_encrypted_picture_mxf_is_refused_with_the_reason() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let encryption = MxfEncryption {
        content_key: [0x11; 16],
        key_id: [0x22; 16],
    };
    let (mxf, _) = write_picture_mxf(directory.path(), "encrypted", Some(encryption));
    let wav = write_tone(directory.path());
    let output = directory.path().join("dcp");

    create_is_refused(
        &mut create(config_home.path(), &mxf, &wav, &output),
        &output,
        &format!(
            "{} is encrypted, a KDM-protected picture MXF cannot be imported",
            mxf.display()
        ),
    );
}

#[test]
fn a_stereoscopic_picture_mxf_is_refused_with_the_reason() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let codestream = cinema_codestream();
    let eye_files = |eye: &str| -> Vec<PathBuf> {
        (0..FRAMES)
            .map(|index| {
                let path = directory.path().join(format!("{eye}_{index}.j2c"));
                std::fs::write(&path, &codestream).unwrap();
                path
            })
            .collect()
    };
    let mxf = directory.path().join("stereoscopic.mxf");
    let track = wrap_stereoscopic(&StereoscopicWrapOptions {
        left_files: eye_files("left"),
        right_files: eye_files("right"),
        output: mxf.clone(),
        fps_num: FRAME_RATE,
        fps_den: 1,
        encryption: None,
        asset_uuid: None,
    });
    assert!(track.success, "wrap failed: {}", track.error);
    let wav = write_tone(directory.path());
    let output = directory.path().join("dcp");

    create_is_refused(
        &mut create(config_home.path(), &mxf, &wav, &output),
        &output,
        &format!(
            "{} is a stereoscopic picture MXF, which cannot be imported",
            mxf.display()
        ),
    );
}
