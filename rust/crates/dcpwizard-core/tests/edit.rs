//! Edit DCP metadata end-to-end (dom#1127): build a real DCP, rewrite its CPL
//! metadata with a new CPL id, and validate. Plus the encrypted-refusal case.

use dcpwizard_core::dcp::{DcpConfig, create_dcp};
use dcpwizard_core::edit::{EditConfig, edit_dcp};
use std::path::{Path, PathBuf};

const W: u32 = 2048;
const H: u32 = 1080;
const FPS: u32 = 24;

fn make_content_frames(dir: &Path, count: usize) {
    std::fs::create_dir_all(dir).unwrap();
    let seed = dir.join("seed.j2c");
    dcpwizard_core::pad::generate_black_frame(W, H, FPS, &seed).expect("encode content frame");
    for i in 0..count {
        std::fs::copy(&seed, dir.join(format!("frame_{i:05}.j2c"))).unwrap();
    }
    std::fs::remove_file(&seed).unwrap();
}

fn make_dcp(root: &Path, name: &str, title: &str, encrypt: bool) -> PathBuf {
    let content = root.join(format!("{name}_j2k"));
    make_content_frames(&content, 48);
    let out = root.join(name);
    let config = DcpConfig {
        title: title.into(),
        standard: dcpwizard_core::Standard::Smpte,
        resolution: dcpwizard_core::Resolution::TwoK,
        content_type: dcpwizard_core::ContentType::Feature,
        frame_rate_num: FPS,
        frame_rate_den: 1,
        output_dir: out.clone(),
        j2k_dir: Some(content),
        encrypt,
        key_out: encrypt.then(|| root.join(format!("{name}_keys.json"))),
        ..Default::default()
    };
    assert_eq!(create_dcp(&config), 0);
    out
}

fn cpl_path(dir: &Path) -> PathBuf {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("CPL_"))
        })
        .unwrap()
}

fn tag(text: &str, t: &str) -> String {
    let open = format!("<{t}>");
    let start = text.find(&open).unwrap() + open.len();
    let end = text[start..].find(&format!("</{t}>")).unwrap();
    text[start..start + end].to_string()
}

#[test]
fn edits_metadata_and_new_cpl_id_and_validates() {
    let root = tempfile::tempdir().unwrap();
    let dcp = make_dcp(root.path(), "src", "Old Title", false);

    let old_cpl = cpl_path(&dcp);
    let old_xml = std::fs::read_to_string(&old_cpl).unwrap();
    let old_id = tag(&old_xml, "Id");
    let out = root.path().join("edited");

    let config = EditConfig {
        input: dcp.clone(),
        output: Some(out.clone()),
        title: Some("New Title".into()),
        annotation: Some("Reissue 2026".into()),
        content_kind: Some("TLR".into()),
        issuer: Some("Acme Post".into()),
    };
    assert_eq!(edit_dcp(&config), 0);

    let new_cpl = cpl_path(&out);
    let xml = std::fs::read_to_string(&new_cpl).unwrap();
    assert_ne!(tag(&xml, "Id"), old_id, "CPL gets a new composition id");
    assert_eq!(tag(&xml, "ContentTitleText"), "New Title");
    assert_eq!(tag(&xml, "ContentKind"), "trailer");
    assert_eq!(tag(&xml, "Issuer"), "Acme Post");
    assert_eq!(tag(&xml, "AnnotationText"), "Reissue 2026");

    // source is left untouched
    let src_xml = std::fs::read_to_string(&old_cpl).unwrap();
    assert_eq!(tag(&src_xml, "ContentTitleText"), "Old Title");

    let result = dcpwizard_core::verify::verify_dcp(&out);
    assert!(result.valid, "dcpdoctor errors: {:?}", result.errors);
}

#[test]
fn edits_in_place_and_validates() {
    let root = tempfile::tempdir().unwrap();
    let dcp = make_dcp(root.path(), "src", "Old", false);

    let config = EditConfig {
        input: dcp.clone(),
        output: None,
        title: Some("Renamed".into()),
        ..Default::default()
    };
    assert_eq!(edit_dcp(&config), 0);

    let xml = std::fs::read_to_string(cpl_path(&dcp)).unwrap();
    assert_eq!(tag(&xml, "ContentTitleText"), "Renamed");
    let result = dcpwizard_core::verify::verify_dcp(&dcp);
    assert!(result.valid, "dcpdoctor errors: {:?}", result.errors);
}

#[test]
fn encrypted_dcp_refused() {
    let root = tempfile::tempdir().unwrap();
    let dcp = make_dcp(root.path(), "enc", "Secret", true);

    let config = EditConfig {
        input: dcp,
        output: Some(root.path().join("out")),
        title: Some("Nope".into()),
        ..Default::default()
    };
    assert_eq!(edit_dcp(&config), -1, "encrypted DCP must be refused");
}
