//! `transcode-dcp` with a container: the source picture is cropped to the active
//! area its CPL declares, fitted into the container with its aspect kept, and
//! the output CPL declares the container. Every check reads the output back.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use dcpwizard_core::dcp::{DcpConfig, create_dcp};
use dcpwizard_core::j2k_transcode::{DcpTranscodeConfig, check_picture_changes, transcode_dcp};
use postkit::grok_encoder::{self, CompressParams, RawFrame};

const FPS: u32 = 24;
const FRAMES: usize = 2;
const SAMPLE_RATE: u32 = 48_000;
const DCI_PRECISION: u8 = 12;
const FLAT: (u32, u32) = (1998, 1080);
const FULL: (u32, u32) = (2048, 1080);
const SCOPE: (u32, u32) = (2048, 858);
const BANDWIDTH_MBPS: u32 = 100;
// the even column create centres a 1998 wide picture at on a 2048 raster
const FLAT_ACTIVE_AREA_LEFT: u32 = 24;

// mid grey on every component, far from both 0 and the 12-bit maximum
const PICTURE_CODE_VALUE: i32 = 2000;
const OUTSIDE_ACTIVE_AREA_CODE_VALUE: i32 = 4000;
// how far a re-encoded sample may move off the code value it was encoded at
const REENCODE_TOLERANCE: i32 = 32;
// columns next to a picture edge, where the scaler and the wavelet ring
const EDGE_MARGIN: u32 = 8;

/// Encode one 12-bit frame whose every sample is `code_value_at(column)`, and
/// copy it into `FRAMES` codestreams.
fn make_frames(dir: &Path, (width, height): (u32, u32), code_value_at: impl Fn(u32) -> i32) {
    std::fs::create_dir_all(dir).unwrap();
    let row: Vec<i32> = (0..width).map(code_value_at).collect();
    let plane: Vec<i32> = row
        .iter()
        .copied()
        .cycle()
        .take(row.len() * height as usize)
        .collect();
    let mut frame = Some(RawFrame::Planar {
        components: [plane.clone(), plane.clone(), plane],
        width,
        height,
        precision: DCI_PRECISION,
        index: 0,
    });
    let work = dir.join("seed");
    grok_encoder::initialize(0);
    let result = grok_encoder::encode_pipeline(
        &work,
        &CompressParams {
            edit_rate: postkit::encode::FrameRate::whole(FPS),
            ..CompressParams::default()
        },
        1,
        &Arc::new(AtomicBool::new(false)),
        &Arc::new(grok_encoder::PhaseClocks::default()),
        |_| frame.take(),
        |_| {},
    );
    assert!(result.success, "seed frame encode failed: {}", result.error);
    let seed = work.join("frame_00000000.j2c");
    for index in 0..FRAMES {
        std::fs::copy(&seed, dir.join(format!("frame_{index:05}.j2c"))).unwrap();
    }
    std::fs::remove_dir_all(&work).unwrap();
}

/// Silent stereo PCM covering the picture, so the CPL carries the metadata
/// asset that holds the stored and active areas.
fn make_wav(path: &Path) {
    let seconds = FRAMES as f64 / f64::from(FPS);
    let output = std::process::Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg(format!("anullsrc=r={SAMPLE_RATE}:cl=stereo"))
        .args(["-t", &seconds.to_string(), "-c:a", "pcm_s24le"])
        .arg(path)
        .output()
        .expect("ffmpeg must be installed to build the test sound");
    assert!(
        output.status.success(),
        "ffmpeg wrote no sound: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn source_dcp(root: &Path, frames: PathBuf, active_area: Option<(u32, u32)>) -> PathBuf {
    let wav = root.join("audio.wav");
    make_wav(&wav);
    let out = root.join("source");
    let (container_width, container_height) = active_area.unwrap_or((0, 0));
    let config = DcpConfig {
        title: "Container".into(),
        standard: dcpwizard_core::Standard::Smpte,
        resolution: dcpwizard_core::Resolution::TwoK,
        content_type: dcpwizard_core::ContentType::Test,
        frame_rate_num: FPS,
        frame_rate_den: 1,
        output_dir: out.clone(),
        j2k_dir: Some(frames),
        audio_path: Some(wav),
        container_width,
        container_height,
        ..Default::default()
    };
    assert_eq!(create_dcp(&config), 0, "build the source DCP");
    out
}

fn transcode_into(source: &Path, output: &Path, container: (u32, u32)) {
    let config = DcpTranscodeConfig {
        input_dir: source.to_path_buf(),
        output_dir: output.to_path_buf(),
        target_bitrate_mbps: Some(BANDWIDTH_MBPS),
        container: Some(container),
        ..Default::default()
    };
    assert_eq!(transcode_dcp(&config), 0, "transcode into {container:?}");
}

fn picture_changes(
    source: &Path,
    target_bitrate_mbps: Option<u32>,
    container: Option<(u32, u32)>,
) -> Result<(), String> {
    check_picture_changes(&DcpTranscodeConfig {
        input_dir: source.to_path_buf(),
        target_bitrate_mbps,
        container,
        ..Default::default()
    })
}

fn assert_nothing_would_change(refused: Result<(), String>) {
    let error = refused.expect_err("a transcode that changes nothing is refused");
    assert!(error.contains("nothing would change"), "{error}");
}

fn file_starting_with(dir: &Path, prefix: &str, extension: &str) -> PathBuf {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(prefix) && n.ends_with(extension))
        })
        .unwrap_or_else(|| panic!("no {prefix}*{extension} in {}", dir.display()))
}

// 16 MB covers a 4K codestream, so the read never holds a whole essence
const MAX_FRAME_BUFFER: usize = 16 * 1024 * 1024;

/// The picture MXF's stored raster and its frame 0, decoded.
fn read_picture(dcp: &Path) -> ((u32, u32), postkit::grok_decoder::DecodedFrame) {
    let mxf = file_starting_with(dcp, "picture", ".mxf");
    let mut reader = asdcplib::jp2k::MxfReader::new();
    reader
        .open_read(&mxf.to_string_lossy())
        .expect("open picture");
    let descriptor = reader.picture_descriptor().expect("picture descriptor");
    let mut buffer = vec![0u8; MAX_FRAME_BUFFER];
    let length = reader
        .read_frame(0, &mut buffer, None, None)
        .expect("read frame 0");
    buffer.truncate(length);
    let decoded = postkit::grok_decoder::decode(buffer, 0).expect("decode frame 0");
    ((descriptor.stored_width, descriptor.stored_height), decoded)
}

struct DeclaredGeometry {
    stored_area: (u32, u32),
    active_area: (u32, u32),
    screen_aspect_ratio: String,
}

fn declared_geometry(dcp: &Path) -> DeclaredGeometry {
    let xml = std::fs::read_to_string(file_starting_with(dcp, "CPL_", ".xml")).unwrap();
    let document = roxmltree::Document::parse(&xml).expect("parse CPL");
    let area = |name: &str| -> (u32, u32) {
        let block = document
            .descendants()
            .find(|n| n.has_tag_name(name))
            .unwrap_or_else(|| panic!("the CPL declares no {name}: {xml}"));
        let edge = |edge_name: &str| -> u32 {
            block
                .children()
                .find(|n| n.has_tag_name(edge_name))
                .and_then(|n| n.text())
                .and_then(|t| t.trim().parse().ok())
                .unwrap_or_else(|| panic!("{name} has no {edge_name}"))
        };
        (edge("Width"), edge("Height"))
    };
    let screen_aspect_ratio = document
        .descendants()
        .find(|n| n.has_tag_name("ScreenAspectRatio"))
        .and_then(|n| n.text())
        .expect("the CPL declares a ScreenAspectRatio")
        .trim()
        .to_string();
    DeclaredGeometry {
        stored_area: area("MainPictureStoredArea"),
        active_area: area("MainPictureActiveArea"),
        screen_aspect_ratio,
    }
}

fn assert_declares_container(dcp: &Path, container: (u32, u32)) {
    let declared = declared_geometry(dcp);
    assert_eq!(declared.stored_area, container);
    assert_eq!(declared.active_area, container);
    assert_eq!(
        declared.screen_aspect_ratio,
        format!("{} {}", container.0, container.1)
    );
}

/// Every sample of every component in columns `first..last` of the frame.
fn columns(frame: &postkit::grok_decoder::DecodedFrame, first: u32, last: u32) -> Vec<i32> {
    let width = frame.width as usize;
    frame
        .components
        .iter()
        .flat_map(|component| {
            component
                .chunks_exact(width)
                .flat_map(move |row| row[first as usize..last as usize].iter().copied())
        })
        .collect()
}

fn assert_black(frame: &postkit::grok_decoder::DecodedFrame, first: u32, last: u32) {
    let brightest = columns(frame, first, last).into_iter().max().unwrap();
    assert_eq!(
        brightest, 0,
        "columns {first}..{last} are a bar, code value 0"
    );
}

fn assert_picture(frame: &postkit::grok_decoder::DecodedFrame, first: u32, last: u32) {
    for sample in columns(frame, first, last) {
        assert!(
            (sample - PICTURE_CODE_VALUE).abs() <= REENCODE_TOLERANCE,
            "columns {first}..{last} are picture at {PICTURE_CODE_VALUE}, found {sample}"
        );
    }
}

#[test]
fn a_flat_dcp_is_fitted_into_full_and_scope_with_black_bars() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let frames = root.join("frames");
    make_frames(&frames, FLAT, |_| PICTURE_CODE_VALUE);
    let source = source_dcp(root, frames, None);
    let (_, source_frame) = read_picture(&source);
    assert_nothing_would_change(picture_changes(&source, None, None));
    assert_nothing_would_change(picture_changes(&source, None, Some(FLAT)));
    assert!(picture_changes(&source, None, Some(FULL)).is_ok());
    assert!(picture_changes(&source, Some(BANDWIDTH_MBPS), None).is_ok());
    assert_eq!(source_frame.precision, DCI_PRECISION);

    let full = root.join("full");
    transcode_into(&source, &full, FULL);
    assert_declares_container(&full, FULL);
    let (stored, frame) = read_picture(&full);
    assert_eq!(stored, FULL);
    assert_eq!((frame.width, frame.height), FULL);
    assert_eq!(frame.precision, source_frame.precision);
    // 1998 already fills the height, so it is only padded out to 2048
    let (pad_left, picture_width) = (FLAT_ACTIVE_AREA_LEFT, FLAT.0);
    let picture_right = pad_left + picture_width;
    assert_black(&frame, 0, pad_left - EDGE_MARGIN);
    assert_picture(&frame, pad_left + EDGE_MARGIN, picture_right - EDGE_MARGIN);
    assert_black(&frame, picture_right + EDGE_MARGIN, FULL.0);

    let scope = root.join("scope");
    transcode_into(&source, &scope, SCOPE);
    assert_declares_container(&scope, SCOPE);
    let (stored, frame) = read_picture(&scope);
    assert_eq!(stored, SCOPE);
    assert_eq!((frame.width, frame.height), SCOPE);
    assert_eq!(frame.precision, source_frame.precision);
    // 1998x1080 scaled to 858 high is 1586 wide, centred on 2048
    let (pad_left, picture_width) = (230, 1586);
    let picture_right = pad_left + picture_width;
    assert_black(&frame, 0, pad_left - EDGE_MARGIN);
    assert_picture(&frame, pad_left + EDGE_MARGIN, picture_right - EDGE_MARGIN);
    assert_black(&frame, picture_right + EDGE_MARGIN, SCOPE.0);
}

#[test]
fn the_source_is_cropped_to_its_active_area_before_the_fit() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let frames = root.join("frames");
    // a full raster with a flat active area where create puts it, at column 24,
    // and bright columns around it that no output column may show
    let active_columns = FLAT_ACTIVE_AREA_LEFT..FLAT_ACTIVE_AREA_LEFT + FLAT.0;
    make_frames(&frames, FULL, |column| {
        if active_columns.contains(&column) {
            PICTURE_CODE_VALUE
        } else {
            OUTSIDE_ACTIVE_AREA_CODE_VALUE
        }
    });
    let source = source_dcp(root, frames, Some(FLAT));
    assert_eq!(declared_geometry(&source).stored_area, FULL);
    assert_eq!(declared_geometry(&source).active_area, FLAT);
    // cropped to 1998 and padded back out to 2048 is the picture it already is
    assert_nothing_would_change(picture_changes(&source, None, Some(FULL)));

    let flat = root.join("flat");
    transcode_into(&source, &flat, FLAT);
    assert_declares_container(&flat, FLAT);
    let (stored, frame) = read_picture(&flat);
    assert_eq!(stored, FLAT);
    assert_eq!(frame.precision, DCI_PRECISION);
    // without the crop, 2048 would be scaled to 1998 with bars above and below
    // and the bright columns would land at both edges
    assert_picture(&frame, EDGE_MARGIN, FLAT.0 - EDGE_MARGIN);
}
