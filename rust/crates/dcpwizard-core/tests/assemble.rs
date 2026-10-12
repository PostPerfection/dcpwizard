//! Assemble OV end-to-end (dom#1675): build small real DCPs, assemble them into
//! one composition, validate with dcpdoctor, and check the loud-failure cases
//! (frame-rate mismatch, encrypted input).

use dcpwizard_core::assemble::{AssembleConfig, assemble, assemble_with_progress};
use dcpwizard_core::dcp::{DcpConfig, ProgressSink, create_dcp};
use dcpwizard_core::hash::hash_file;
use dcpwizard_core::isdcf_name::Rating;
use dcpwizard_core::library::{Library, LibraryItemKind};
use dcpwizard_core::multi_cpl::{TimelineEntry, get_timeline};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

const W: u32 = 2048;
const H: u32 = 1080;

/// What `create_dcp` packages when a job brings no audio: 5.1 silence.
const FILL_SOUND_CONFIGURATION: &str = "51/L,R,C,LFE,Ls,Rs";
const DCP_SAMPLE_RATE: u32 = 48_000;

fn make_content_frames(dir: &Path, count: usize, fps: u32) {
    std::fs::create_dir_all(dir).unwrap();
    let seed = dir.join("seed.j2c");
    dcpwizard_core::pad::generate_black_frame(W, H, fps, &seed).expect("encode content frame");
    for i in 0..count {
        std::fs::copy(&seed, dir.join(format!("frame_{i:05}.j2c"))).unwrap();
    }
    std::fs::remove_file(&seed).unwrap();
}

fn make_dcp(
    root: &Path,
    name: &str,
    title: &str,
    frames: usize,
    fps: u32,
    encrypt: bool,
) -> PathBuf {
    let content = root.join(format!("{name}_j2k"));
    make_content_frames(&content, frames, fps);
    let out = root.join(name);
    let config = DcpConfig {
        title: title.into(),
        standard: dcpwizard_core::Standard::Smpte,
        resolution: dcpwizard_core::Resolution::TwoK,
        content_type: dcpwizard_core::ContentType::Test,
        frame_rate_num: fps,
        frame_rate_den: 1,
        output_dir: out.clone(),
        j2k_dir: Some(content),
        encrypt,
        key_out: encrypt.then(|| root.join(format!("{name}_keys.json"))),
        ..Default::default()
    };
    assert_eq!(create_dcp(&config), 0, "create {name} must succeed");
    out
}

fn cpl_reel_count(dir: &Path) -> usize {
    let cpl = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("CPL_"))
        })
        .unwrap();
    let xml = std::fs::read_to_string(cpl).unwrap();
    xml.matches("<Reel>").count()
}

#[test]
fn assembles_two_dcps_into_one_valid_ov() {
    let root = tempfile::tempdir().unwrap();
    let a = make_dcp(root.path(), "short", "The Short", 48, 24, false);
    let b = make_dcp(root.path(), "feature", "The Feature", 72, 24, false);

    let out = root.path().join("program");
    let config = AssembleConfig {
        inputs: vec![a, b],
        output_dir: out.clone(),
        title: "Program".into(),
        signer: None,
        ..Default::default()
    };
    assert_eq!(assemble(&config), 0);

    // one new CPL with both source reels, in input order
    let cpls: Vec<PathBuf> = std::fs::read_dir(&out)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("CPL_"))
        })
        .collect();
    assert_eq!(cpls.len(), 1, "assemble writes exactly one CPL");
    assert_eq!(cpl_reel_count(&out), 2, "two reels, one per input");
    let xml = std::fs::read_to_string(&cpls[0]).unwrap();
    assert!(xml.contains("<ContentTitleText>Program</ContentTitleText>"));

    // the sound the program ships is the sources' 5.1 silent fill, read back off
    // the copied track file
    assert!(
        xml.contains("<meta:CompositionMetadataAsset"),
        "SMPTE Bv2.1 wants the metadata asset: {xml}"
    );
    assert!(
        xml.contains(&format!(
            "<meta:MainSoundConfiguration>{FILL_SOUND_CONFIGURATION}</meta:MainSoundConfiguration>"
        )),
        "assembled CPL must declare the sound it ships: {xml}"
    );
    assert!(
        xml.contains(&format!(
            "<meta:MainSoundSampleRate>{DCP_SAMPLE_RATE} 1</meta:MainSoundSampleRate>"
        )),
        "{xml}"
    );

    let result = dcpwizard_core::verify::verify_dcp(&out);
    assert!(result.valid, "dcpdoctor errors: {:?}", result.errors);
}

#[test]
fn frame_rate_mismatch_fails_loud() {
    let root = tempfile::tempdir().unwrap();
    let a = make_dcp(root.path(), "a", "A", 48, 24, false);
    let b = make_dcp(root.path(), "b", "B", 50, 25, false);

    let config = AssembleConfig {
        inputs: vec![a, b],
        output_dir: root.path().join("out"),
        title: "X".into(),
        signer: None,
        ..Default::default()
    };
    assert_eq!(assemble(&config), -1, "mixed frame rates must fail");
}

#[test]
fn encrypted_input_fails_loud() {
    let root = tempfile::tempdir().unwrap();
    let a = make_dcp(root.path(), "a", "A", 48, 24, false);
    let enc = make_dcp(root.path(), "enc", "Enc", 48, 24, true);

    let config = AssembleConfig {
        inputs: vec![a, enc],
        output_dir: root.path().join("out"),
        title: "X".into(),
        signer: None,
        ..Default::default()
    };
    assert_eq!(assemble(&config), -1, "encrypted input must fail");
}

const FEATURE_FRAMES: usize = 72;
const FEATURE_CHANNELS: u32 = 8;
const FLAT_ACTIVE_WIDTH: u32 = 1998;
const HEAD_STILL_SECONDS: f64 = 2.0;
const TAIL_VIDEO_SECONDS: f64 = 1.0;
const ITEM_SOURCE_SIZE: &str = "1920x1080";
const MPAA_AGENCY: &str = "http://www.mpaa.org/2003-ratings";
const BBFC_AGENCY: &str = "http://www.bbfc.co.uk";
const CONTENT_VERSION: &str = "Final Cut";
const TRAILER_CONTENT_KIND: &str = "<ContentKind>trailer</ContentKind>";

/// A 2K SMPTE trailer masked to flat, with 8 channels of sound, a rating and a
/// content version, so every carried value differs from a default.
fn make_feature(root: &Path) -> PathBuf {
    let content = root.join("feature_j2k");
    make_content_frames(&content, FEATURE_FRAMES, 24);
    let out = root.join("feature");
    let config = DcpConfig {
        title: "The Feature".into(),
        standard: dcpwizard_core::Standard::Smpte,
        resolution: dcpwizard_core::Resolution::TwoK,
        content_type: dcpwizard_core::ContentType::Trailer,
        frame_rate_num: 24,
        frame_rate_den: 1,
        container_width: FLAT_ACTIVE_WIDTH,
        container_height: H,
        audio_channels: Some(FEATURE_CHANNELS),
        ratings: vec![Rating {
            agency: MPAA_AGENCY.into(),
            label: "PG-13".into(),
        }],
        content_versions: vec![CONTENT_VERSION.into()],
        output_dir: out.clone(),
        j2k_dir: Some(content),
        ..Default::default()
    };
    assert_eq!(create_dcp(&config), 0, "create the feature");
    out
}

fn run_ffmpeg(arguments: &[&str], output: &Path) {
    let status = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error"])
        .args(arguments)
        .arg(output)
        .status()
        .expect("ffmpeg must be installed to build the library items");
    assert!(
        status.success(),
        "ffmpeg failed to write {}",
        output.display()
    );
}

/// A temporary library holding a still head ident and a video tail ident with
/// stereo sound.
fn make_library(root: &Path) -> Library {
    let media = root.join("item_media");
    std::fs::create_dir_all(&media).unwrap();
    let still = media.join("ident.png");
    run_ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=size={ITEM_SOURCE_SIZE}"),
            "-frames:v",
            "1",
        ],
        &still,
    );
    let video = media.join("logo.mov");
    run_ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=size={ITEM_SOURCE_SIZE}:rate=24:duration={TAIL_VIDEO_SECONDS}"),
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=1000:duration={TAIL_VIDEO_SECONDS}"),
            "-ac",
            "2",
            "-pix_fmt",
            "yuv420p",
        ],
        &video,
    );
    let library = Library::open_at(root.join("library"));
    library
        .import(
            &still,
            "Head Ident",
            LibraryItemKind::HeadIdent,
            Some(HEAD_STILL_SECONDS),
        )
        .unwrap();
    library
        .import(&video, "Tail Logo", LibraryItemKind::TailIdent, None)
        .unwrap();
    library
}

fn only_cpl(dir: &Path) -> PathBuf {
    let cpls: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("CPL_"))
        })
        .collect();
    assert_eq!(cpls.len(), 1, "one CPL in {}", dir.display());
    cpls.into_iter().next().unwrap()
}

fn timeline(dir: &Path) -> Vec<TimelineEntry> {
    get_timeline(&only_cpl(dir))
}

fn cpl_xml(dir: &Path) -> String {
    std::fs::read_to_string(only_cpl(dir)).unwrap()
}

fn picture_descriptor(file: &str) -> asdcplib::jp2k::PictureDescriptor {
    let mut reader = asdcplib::jp2k::MxfReader::new();
    reader.open_read(file).unwrap();
    reader.picture_descriptor().unwrap()
}

fn sound_descriptor(file: &str) -> asdcplib::pcm::AudioDescriptor {
    let mut reader = asdcplib::pcm::MxfReader::new();
    reader.open_read(file).unwrap();
    reader.audio_descriptor().unwrap()
}

fn element_text<'a>(xml: &'a str, open: &str, close: &str) -> &'a str {
    let (_, rest) = xml
        .split_once(open)
        .unwrap_or_else(|| panic!("{open}: {xml}"));
    rest.split_once(close).unwrap().0
}

#[derive(Default)]
struct RecordedProgress {
    percents: RefCell<Vec<u32>>,
}

impl ProgressSink for RecordedProgress {
    fn stage(&self, percent: u32, _message: &str) {
        self.percents.borrow_mut().push(percent);
    }
    fn cancelled(&self) -> bool {
        false
    }
}

#[test]
fn library_items_become_reels_around_an_untouched_feature() {
    let root = tempfile::tempdir().unwrap();
    let feature = make_feature(root.path());
    let library = make_library(root.path());

    let out = root.path().join("program");
    let config = AssembleConfig {
        inputs: vec![feature.clone()],
        output_dir: out.clone(),
        title: "Program".into(),
        head_items: vec![library.attach("Head Ident").unwrap()],
        tail_items: vec![library.attach("Tail Logo").unwrap()],
        ..Default::default()
    };
    let progress = RecordedProgress::default();
    assemble_with_progress(&config, &progress).expect("assemble with items");

    let percents = progress.percents.into_inner();
    assert!(percents.is_sorted(), "progress only rises: {percents:?}");
    assert_eq!(percents.last(), Some(&100));

    let source = timeline(&feature);
    assert_eq!(source.len(), 1);
    let source = &source[0];
    let reels = timeline(&out);
    let frames_at_24 = |seconds: f64| (seconds * 24.0).round() as u64;
    let shape: Vec<(bool, u64)> = reels
        .iter()
        .map(|r| {
            (
                r.picture_asset_id == source.picture_asset_id,
                r.duration_frames,
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            (false, frames_at_24(HEAD_STILL_SECONDS)),
            (true, FEATURE_FRAMES as u64),
            (false, frames_at_24(TAIL_VIDEO_SECONDS)),
        ],
        "head item, feature, tail item"
    );

    // the feature's essence is copied, never rewritten
    let feature_reel = &reels[1];
    assert_eq!(feature_reel.sound_asset_id, source.sound_asset_id);
    for (copied, original) in [
        (&feature_reel.picture_file, &source.picture_file),
        (&feature_reel.sound_file, &source.sound_file),
    ] {
        assert_ne!(copied, original);
        assert_eq!(
            hash_file(Path::new(copied)).unwrap(),
            hash_file(Path::new(original)).unwrap(),
            "{copied} must be byte identical to {original}"
        );
    }

    let feature_picture = picture_descriptor(&source.picture_file);
    let feature_sound = sound_descriptor(&source.sound_file);
    assert_eq!(feature_sound.channel_count, FEATURE_CHANNELS);
    for item_reel in [&reels[0], &reels[2]] {
        let picture = picture_descriptor(&item_reel.picture_file);
        assert_eq!(
            (picture.stored_width, picture.stored_height),
            (feature_picture.stored_width, feature_picture.stored_height)
        );
        assert_eq!(picture.edit_rate, feature_picture.edit_rate);
        assert_eq!(item_reel.edit_rate, source.edit_rate);
        let sound = sound_descriptor(&item_reel.sound_file);
        assert_eq!(sound.channel_count, feature_sound.channel_count);
        assert_eq!(sound.audio_sampling_rate, feature_sound.audio_sampling_rate);
    }
    // the head item's reel carries the metadata asset, which describes the feature
    // the head item reel now carries the metadata asset, which has to describe the feature
    let xml = cpl_xml(&out);
    let first_reel = element_text(&xml, "<Reel>", "</Reel>");
    let stored = element_text(
        first_reel,
        "<meta:MainPictureStoredArea>",
        "</meta:MainPictureStoredArea>",
    );
    assert!(
        stored.contains(&format!("<meta:Width>{W}</meta:Width>")),
        "{stored}"
    );
    let active = element_text(
        first_reel,
        "<meta:MainPictureActiveArea>",
        "</meta:MainPictureActiveArea>",
    );
    assert!(
        active.contains(&format!("<meta:Width>{FLAT_ACTIVE_WIDTH}</meta:Width>")),
        "{active}"
    );
    assert!(first_reel.contains("FFOC"), "{first_reel}");
    let last_reel = xml.rsplit_once("<Reel>").unwrap().1;
    assert!(last_reel.contains("LFOC"), "{last_reel}");

    let result = dcpwizard_core::verify::verify_dcp(&out);
    assert!(result.valid, "dcpdoctor errors: {:?}", result.errors);
}

#[test]
fn ratings_and_content_version_are_carried_unless_replaced() {
    let root = tempfile::tempdir().unwrap();
    let feature = make_feature(root.path());

    let carried = root.path().join("carried");
    let config = AssembleConfig {
        inputs: vec![feature.clone()],
        output_dir: carried.clone(),
        title: "Retitled".into(),
        ..Default::default()
    };
    assert_eq!(assemble(&config), 0);
    let xml = cpl_xml(&carried);
    assert!(xml.contains(TRAILER_CONTENT_KIND), "{xml}");
    assert!(
        xml.contains(&format!("<Agency>{MPAA_AGENCY}</Agency>")),
        "{xml}"
    );
    assert!(xml.contains("<Label>PG-13</Label>"), "{xml}");
    assert!(
        xml.contains(&format!("<LabelText>{CONTENT_VERSION}</LabelText>")),
        "{xml}"
    );

    let replaced = root.path().join("replaced");
    let config = AssembleConfig {
        inputs: vec![feature],
        output_dir: replaced.clone(),
        title: "Retitled".into(),
        ratings: Some(vec![Rating {
            agency: BBFC_AGENCY.into(),
            label: "12A".into(),
        }]),
        ..Default::default()
    };
    assert_eq!(assemble(&config), 0);
    let xml = cpl_xml(&replaced);
    assert!(xml.contains(TRAILER_CONTENT_KIND), "{xml}");
    assert!(
        xml.contains(&format!("<Agency>{BBFC_AGENCY}</Agency>")),
        "{xml}"
    );
    assert!(xml.contains("<Label>12A</Label>"), "{xml}");
    assert!(
        !xml.contains(MPAA_AGENCY),
        "the given ratings replace the source's: {xml}"
    );
    assert!(
        xml.contains(&format!("<LabelText>{CONTENT_VERSION}</LabelText>")),
        "{xml}"
    );
}

#[test]
fn one_input_becomes_a_new_ov() {
    let root = tempfile::tempdir().unwrap();
    let source = make_dcp(root.path(), "short", "The Short", 48, 24, false);

    let out = root.path().join("copy");
    let config = AssembleConfig {
        inputs: vec![source.clone()],
        output_dir: out.clone(),
        title: "New Title".into(),
        ..Default::default()
    };
    assert_eq!(assemble(&config), 0);

    assert_ne!(
        only_cpl(&out).file_name(),
        only_cpl(&source).file_name(),
        "a new CPL id"
    );
    let xml = cpl_xml(&out);
    assert!(
        xml.contains("<ContentTitleText>New Title</ContentTitleText>"),
        "{xml}"
    );
    // the source's label was its own title
    assert!(xml.contains("<LabelText>New Title</LabelText>"), "{xml}");
    let reels = timeline(&out);
    assert_eq!(reels.len(), 1);
    assert_eq!(
        reels[0].picture_asset_id,
        timeline(&source)[0].picture_asset_id
    );

    let result = dcpwizard_core::verify::verify_dcp(&out);
    assert!(result.valid, "dcpdoctor errors: {:?}", result.errors);
}

/// Asset id to hash, as the directory's PKL records them.
fn pkl_hashes(dir: &Path) -> std::collections::HashMap<String, String> {
    let pkl = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("PKL_"))
        })
        .expect("a PKL");
    let xml = std::fs::read_to_string(pkl).unwrap();
    xml.split("<Asset>")
        .skip(1)
        .map(|asset| {
            let id = element_text(asset, "<Id>urn:uuid:", "</Id>").to_string();
            let hash = element_text(asset, "<Hash>", "</Hash>").to_string();
            (id, hash)
        })
        .collect()
}

#[test]
fn moving_track_files_takes_them_out_of_the_input() {
    let root = tempfile::tempdir().unwrap();
    let source = make_dcp(root.path(), "intermediate", "Intermediate", 48, 24, false);
    let source_reel = timeline(&source).remove(0);
    let source_hashes = pkl_hashes(&source);

    let out = root.path().join("moved");
    let config = AssembleConfig {
        inputs: vec![source.clone()],
        output_dir: out.clone(),
        title: "Moved".into(),
        move_track_files: true,
        ..Default::default()
    };
    assert_eq!(assemble(&config), 0);

    let output_reel = timeline(&out).remove(0);
    let output_hashes = pkl_hashes(&out);
    for (id, source_file, output_file) in [
        (
            &source_reel.picture_asset_id,
            &source_reel.picture_file,
            &output_reel.picture_file,
        ),
        (
            &source_reel.sound_asset_id,
            &source_reel.sound_file,
            &output_reel.sound_file,
        ),
    ] {
        assert!(
            Path::new(output_file).is_file(),
            "{output_file} is in the output"
        );
        assert!(
            !Path::new(source_file).exists(),
            "{source_file} left the input"
        );
        assert_eq!(output_hashes[id], source_hashes[id], "PKL hash of {id}");
    }
    assert_eq!(
        hash_file(Path::new(&output_reel.picture_file)).unwrap(),
        source_hashes[&source_reel.picture_asset_id],
        "the moved picture is the one the source PKL describes"
    );
}
