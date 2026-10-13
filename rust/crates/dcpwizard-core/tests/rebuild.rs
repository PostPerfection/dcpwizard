use dcpwizard_core::dcp::{DcpConfig, ProgressSink, create_dcp};
use dcpwizard_core::hash::hash_file;
use dcpwizard_core::isdcf_name::Rating;
use dcpwizard_core::multi_cpl::{TimelineEntry, get_timeline};
use dcpwizard_core::rebuild::{RebuildConfig, inspect, rebuild_with_progress};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const FPS: u32 = 24;
const FRAMES: usize = 24;
const FLAT_ACTIVE_WIDTH: u32 = 1998;
const SCOPE: (u32, u32) = (2048, 858);
const SOURCE_TITLE: &str = "The Feature";
const REBUILT_TITLE: &str = "The Feature UK";
const MPAA_AGENCY: &str = "http://www.mpaa.org/2003-ratings";
const MPAA_LABEL: &str = "PG-13";
const BBFC_AGENCY: &str = "http://www.bbfc.co.uk";
const BBFC_LABEL: &str = "12A";
const CONTENT_VERSION: &str = "Final Cut";
const TRAILER_CONTENT_KIND: &str = "trailer";
const CAPTION_LANGUAGE: &str = "en";
const CAPTION_SRT: &str = "1\n00:00:00,100 --> 00:00:00,900\nA caption\n";
// assemble names the tracks it moves into the output in its stage messages
const MOVING_STAGE_PREFIX: &str = "moving";
// where the re-encode ends and the assemble starts on the rebuild's percent
const REENCODE_END_PERCENT: u32 = 90;

fn make_content_frames(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let seed = dir.join("seed.j2c");
    dcpwizard_core::pad::generate_black_frame(WIDTH, HEIGHT, FPS, &seed)
        .expect("encode content frame");
    for i in 0..FRAMES {
        std::fs::copy(&seed, dir.join(format!("frame_{i:05}.j2c"))).unwrap();
    }
    std::fs::remove_file(&seed).unwrap();
}

// a full raster masked to flat, with a rating, a content version and a kind
// that all differ from a default
fn make_source(root: &Path) -> PathBuf {
    make_source_with_captions(root, None)
}

fn make_source_with_captions(root: &Path, captions: Option<PathBuf>) -> PathBuf {
    let content = root.join("source_j2k");
    make_content_frames(&content);
    let out = root.join("source");
    let config = DcpConfig {
        title: SOURCE_TITLE.into(),
        standard: dcpwizard_core::Standard::Smpte,
        resolution: dcpwizard_core::Resolution::TwoK,
        content_type: dcpwizard_core::ContentType::Trailer,
        frame_rate_num: FPS,
        frame_rate_den: 1,
        container_width: FLAT_ACTIVE_WIDTH,
        container_height: HEIGHT,
        ratings: vec![Rating {
            agency: MPAA_AGENCY.into(),
            label: MPAA_LABEL.into(),
        }],
        content_versions: vec![CONTENT_VERSION.into()],
        output_dir: out.clone(),
        j2k_dir: Some(content),
        ccap_language: captions
            .as_ref()
            .map(|_| CAPTION_LANGUAGE.into())
            .unwrap_or_default(),
        ccap_path: captions,
        ..Default::default()
    };
    assert_eq!(create_dcp(&config), 0, "create the source");
    out
}

fn bbfc_rating() -> Vec<Rating> {
    vec![Rating {
        agency: BBFC_AGENCY.into(),
        label: BBFC_LABEL.into(),
    }]
}

fn rebuild_config(source: &Path, output_parent: &Path) -> RebuildConfig {
    RebuildConfig {
        source: source.to_path_buf(),
        output_parent: output_parent.to_path_buf(),
        title: REBUILT_TITLE.into(),
        ratings: bbfc_rating(),
        ..Default::default()
    }
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

fn only_reel(dir: &Path) -> TimelineEntry {
    let mut reels = get_timeline(&only_cpl(dir));
    assert_eq!(reels.len(), 1, "one reel in {}", dir.display());
    reels.remove(0)
}

fn cpl_xml(dir: &Path) -> String {
    std::fs::read_to_string(only_cpl(dir)).unwrap()
}

fn picture_raster(file: &str) -> (u32, u32) {
    let mut reader = asdcplib::jp2k::MxfReader::new();
    reader.open_read(file).unwrap();
    let descriptor = reader.picture_descriptor().unwrap();
    (descriptor.stored_width, descriptor.stored_height)
}

fn file_hashes(dir: &Path) -> BTreeMap<String, String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                hash_file(&entry.path()).unwrap(),
            )
        })
        .collect()
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn declared_area(xml: &str, name: &str) -> (u32, u32) {
    let document = roxmltree::Document::parse(xml).expect("parse CPL");
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
}

fn assert_rebuilt_identity(xml: &str) {
    assert!(
        xml.contains(&format!(
            "<ContentTitleText>{REBUILT_TITLE}</ContentTitleText>"
        )),
        "{xml}"
    );
    assert!(
        xml.contains(&format!(
            "<ContentKind>{TRAILER_CONTENT_KIND}</ContentKind>"
        )),
        "{xml}"
    );
    assert!(
        xml.contains(&format!("<Agency>{BBFC_AGENCY}</Agency>")),
        "{xml}"
    );
    assert!(
        xml.contains(&format!("<Label>{BBFC_LABEL}</Label>")),
        "{xml}"
    );
    assert!(
        !xml.contains(MPAA_AGENCY),
        "the ratings are replaced: {xml}"
    );
    assert!(
        xml.contains(&format!("<LabelText>{CONTENT_VERSION}</LabelText>")),
        "{xml}"
    );
}

struct NoProgress;

impl ProgressSink for NoProgress {
    fn stage(&self, _percent: u32, _message: &str) {}
    fn cancelled(&self) -> bool {
        false
    }
}

#[derive(Default)]
struct RecordedProgress {
    percents: Mutex<Vec<u32>>,
    frames_seen: AtomicBool,
}

impl ProgressSink for RecordedProgress {
    fn stage(&self, percent: u32, _message: &str) {
        self.percents.lock().unwrap().push(percent);
    }
    fn cancelled(&self) -> bool {
        false
    }
    fn frames(&self, _done: u64, _total: u64) {
        self.frames_seen.store(true, Ordering::Relaxed);
    }
}

#[test]
fn inspect_reads_the_raster_reels_and_ratings() {
    let root = tempfile::tempdir().unwrap();
    let source = make_source(root.path());

    let read = inspect(&source).expect("inspect the source");

    assert_eq!(read.title, SOURCE_TITLE);
    assert_eq!(read.standard, dcpwizard_core::Standard::Smpte);
    assert_eq!((read.stored_width, read.stored_height), (WIDTH, HEIGHT));
    assert_eq!(
        (read.active_width, read.active_height),
        (FLAT_ACTIVE_WIDTH, HEIGHT)
    );
    assert_eq!(
        (read.edit_rate_numerator, read.edit_rate_denominator),
        (FPS, 1)
    );
    assert_eq!(read.reel_count, 1);
    assert_eq!(read.total_frames, FRAMES as u64);
    assert_eq!(read.content_kind, TRAILER_CONTENT_KIND);
    assert_eq!(
        read.ratings,
        [Rating {
            agency: MPAA_AGENCY.into(),
            label: MPAA_LABEL.into(),
        }]
    );
}

#[test]
fn nothing_asking_for_a_new_picture_copies_the_source_picture() {
    let root = tempfile::tempdir().unwrap();
    let source = make_source(root.path());
    let source_reel = only_reel(&source);
    let output_parent = root.path().join("rebuilt");
    std::fs::create_dir_all(&output_parent).unwrap();
    let progress = RecordedProgress::default();

    let package = rebuild_with_progress(&rebuild_config(&source, &output_parent), &progress)
        .expect("rebuild");

    assert_eq!(package, output_parent.join(REBUILT_TITLE));
    assert_eq!(entries(&output_parent), [REBUILT_TITLE]);
    let reel = only_reel(&package);
    assert_eq!(
        hash_file(Path::new(&reel.picture_file)).unwrap(),
        hash_file(Path::new(&source_reel.picture_file)).unwrap(),
        "the picture is copied, never re-encoded"
    );
    assert!(
        !progress.frames_seen.load(Ordering::Relaxed),
        "no frame was encoded"
    );
    assert_eq!(progress.percents.lock().unwrap().last(), Some(&100));
    assert_rebuilt_identity(&cpl_xml(&package));
    assert!(
        Path::new(&source_reel.picture_file).is_file(),
        "the source keeps its picture"
    );
}

#[test]
fn a_new_container_re_encodes_the_picture_and_leaves_the_source_alone() {
    let root = tempfile::tempdir().unwrap();
    let source = make_source(root.path());
    let source_reel = only_reel(&source);
    let source_hashes = file_hashes(&source);
    let output_parent = root.path().join("rebuilt");
    std::fs::create_dir_all(&output_parent).unwrap();
    let progress = RecordedProgress::default();
    let config = RebuildConfig {
        container: Some(SCOPE),
        ..rebuild_config(&source, &output_parent)
    };

    let package = rebuild_with_progress(&config, &progress).expect("rebuild into scope");

    assert_eq!(entries(&output_parent), [REBUILT_TITLE]);
    let xml = cpl_xml(&package);
    assert_eq!(declared_area(&xml, "MainPictureStoredArea"), SCOPE);
    assert_eq!(declared_area(&xml, "MainPictureActiveArea"), SCOPE);
    assert_rebuilt_identity(&xml);
    let reel = only_reel(&package);
    assert_eq!(picture_raster(&reel.picture_file), SCOPE);
    assert_ne!(reel.picture_asset_id, source_reel.picture_asset_id);
    assert!(
        progress.frames_seen.load(Ordering::Relaxed),
        "the picture was re-encoded"
    );
    let percents = progress.percents.lock().unwrap().clone();
    assert!(percents.is_sorted(), "progress only rises: {percents:?}");
    assert!(
        percents
            .iter()
            .any(|percent| (1..REENCODE_END_PERCENT).contains(percent)),
        "the encoded frames move the percent: {percents:?}"
    );
    assert_eq!(percents.last(), Some(&100));
    assert_eq!(
        file_hashes(&source),
        source_hashes,
        "the source is untouched"
    );

    let result = dcpwizard_core::verify::verify_dcp(&package);
    assert!(result.valid, "dcpdoctor errors: {:?}", result.errors);
}

#[derive(Default)]
struct CancelOnFirstMove {
    moving: AtomicBool,
}

impl ProgressSink for CancelOnFirstMove {
    fn stage(&self, _percent: u32, message: &str) {
        if message.starts_with(MOVING_STAGE_PREFIX) {
            self.moving.store(true, Ordering::Relaxed);
        }
    }
    fn cancelled(&self) -> bool {
        self.moving.load(Ordering::Relaxed)
    }
}

#[test]
fn a_cancel_leaves_neither_the_package_nor_the_intermediate() {
    let root = tempfile::tempdir().unwrap();
    let source = make_source(root.path());
    let source_hashes = file_hashes(&source);
    let output_parent = root.path().join("rebuilt");
    std::fs::create_dir_all(&output_parent).unwrap();
    let config = RebuildConfig {
        container: Some(SCOPE),
        ..rebuild_config(&source, &output_parent)
    };

    let error = rebuild_with_progress(&config, &CancelOnFirstMove::default())
        .expect_err("a cancelled rebuild fails");

    assert!(error.contains("cancelled"), "{error}");
    assert_eq!(entries(&output_parent), Vec::<String>::new());
    assert_eq!(
        file_hashes(&source),
        source_hashes,
        "the source is untouched"
    );
}

#[test]
fn an_existing_package_folder_is_refused_and_kept() {
    let root = tempfile::tempdir().unwrap();
    let source = make_source(root.path());
    let output_parent = root.path().join("rebuilt");
    let existing = output_parent.join(REBUILT_TITLE);
    std::fs::create_dir_all(&existing).unwrap();

    let error = rebuild_with_progress(&rebuild_config(&source, &output_parent), &NoProgress)
        .expect_err("the folder exists");

    assert!(error.contains("already exists"), "{error}");
    assert!(existing.is_dir(), "a folder this run did not make stays");
}

#[test]
fn a_source_with_closed_captions_is_refused_rather_than_losing_them() {
    let root = tempfile::tempdir().unwrap();
    let captions = root.path().join("captions.srt");
    std::fs::write(&captions, CAPTION_SRT).unwrap();
    let source = make_source_with_captions(root.path(), Some(captions));

    let error = inspect(&source).expect_err("captions are not carried");

    assert!(error.contains("closed captions"), "{error}");
}
