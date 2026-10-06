use dcpwizard_core::dcp::{DcpConfig, create_dcp};
use dcpwizard_core::verify::{VerifyCliOptions, verify_dcp_with_options};
use dcpwizard_core::vf::{ReplacementReel, VfConfig, create_vf, vf_package_dir};
use std::path::{Path, PathBuf};

const FPS: u32 = 24;
const FRAMES: usize = 24;
const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const SAMPLE_RATE: u32 = 48_000;
const SIXTEEN_BITS: u16 = 16;
const PACKAGED_BITS: u16 = 24;

fn make_frames(dir: &Path, count: usize) {
    std::fs::create_dir_all(dir).unwrap();
    let seed = dir.join("seed.j2c");
    dcpwizard_core::pad::generate_black_frame(WIDTH, HEIGHT, FPS, &seed).unwrap();
    for index in 0..count {
        std::fs::copy(&seed, dir.join(format!("frame_{index:05}.j2c"))).unwrap();
    }
    std::fs::remove_file(&seed).unwrap();
}

fn write_stereo_wav(path: &Path, bits_per_sample: u16) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    let samples_per_channel = FRAMES * (SAMPLE_RATE / FPS) as usize;
    for index in 0..samples_per_channel * 2 {
        writer.write_sample((index % 64) as i32 * 100).unwrap();
    }
    writer.finalize().unwrap();
}

fn make_ov(root: &Path) -> PathBuf {
    let frames = root.join("ov_frames");
    make_frames(&frames, FRAMES);
    let wav = root.join("ov.wav");
    write_stereo_wav(&wav, PACKAGED_BITS);
    let ov = root.join("ov");
    let config = DcpConfig {
        title: "Original".into(),
        frame_rate_num: FPS,
        frame_rate_den: 1,
        output_dir: ov.clone(),
        j2k_dir: Some(frames),
        audio_path: Some(wav),
        ..Default::default()
    };
    assert_eq!(create_dcp(&config), 0);
    ov
}

fn find_mxf(dir: &Path, prefix: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".mxf"))
        })
}

fn vf_config(root: &Path, ov: PathBuf, replacement: ReplacementReel) -> VfConfig {
    VfConfig {
        ov_dir: ov,
        vf_dir: root.join("vf"),
        title: "Version".into(),
        subtitle_language: "en".into(),
        subtitle_opts: dcpwizard_core::subtitle::SubtitleOptions::default(),
        replacement_reels: vec![replacement],
        signer: None,
    }
}

fn strict_verify_against(vf: &Path, ov: &Path) -> dcpwizard_core::verify::VerifyResult {
    verify_dcp_with_options(
        vf,
        &VerifyCliOptions {
            strict: true,
            ov_dir: Some(ov.to_path_buf()),
            ..Default::default()
        },
    )
}

#[test]
fn a_picture_version_file_passes_strict_verify_against_its_ov() {
    let dir = tempfile::tempdir().unwrap();
    let ov = make_ov(dir.path());
    let replacement = dir.path().join("replacement");
    make_frames(&replacement, FRAMES);
    let config = vf_config(
        dir.path(),
        ov.clone(),
        ReplacementReel {
            reel_number: 1,
            picture: Some(replacement),
            ..Default::default()
        },
    );

    assert_eq!(create_vf(&config), 0);

    let result = strict_verify_against(&vf_package_dir(&config).unwrap(), &ov);
    assert!(
        result.errors.is_empty(),
        "verify errors: {:?}",
        result.errors
    );
    assert!(
        !result
            .warnings
            .iter()
            .any(|warning| warning.contains("supplemental_ov_not_provided")),
        "the OV was supplied, so every reference resolves: {:?}",
        result.warnings
    );
}

#[test]
fn a_replacement_longer_than_its_reel_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let ov = make_ov(dir.path());
    let replacement = dir.path().join("replacement");
    make_frames(&replacement, FRAMES * 2);
    let config = vf_config(
        dir.path(),
        ov,
        ReplacementReel {
            reel_number: 1,
            picture: Some(replacement),
            ..Default::default()
        },
    );

    assert_eq!(create_vf(&config), -1);
    assert!(
        !vf_package_dir(&config)
            .unwrap()
            .join("ASSETMAP.xml")
            .exists(),
        "no package is finished around the mismatched reel"
    );
}

#[test]
fn a_sixteen_bit_replacement_sound_is_packaged_at_twenty_four_bits() {
    let dir = tempfile::tempdir().unwrap();
    let ov = make_ov(dir.path());
    let wav = dir.path().join("sixteen.wav");
    write_stereo_wav(&wav, SIXTEEN_BITS);
    let config = vf_config(
        dir.path(),
        ov.clone(),
        ReplacementReel {
            reel_number: 1,
            sound: Some(wav),
            ..Default::default()
        },
    );

    assert_eq!(create_vf(&config), 0);

    let package = vf_package_dir(&config).unwrap();
    let sound = find_mxf(&package, "sound_").expect("the VF ships its sound MXF");
    let mut reader = asdcplib::pcm::MxfReader::new();
    reader.open_read(&sound.to_string_lossy()).unwrap();
    assert_eq!(
        reader.audio_descriptor().unwrap().quantization_bits,
        u32::from(PACKAGED_BITS)
    );
    let result = strict_verify_against(&package, &ov);
    assert!(
        result.errors.is_empty(),
        "verify errors: {:?}",
        result.errors
    );
}
