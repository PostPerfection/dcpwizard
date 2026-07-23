//! Conform assembly end-to-end: a tiny 2-reel CMX3600 EDL over synthetic media is
//! driven to a finished multi-reel DCP (per-reel encode + wrap + assembly). Fast
//! (a few frames per reel) but exercises the real grok encode + create + assemble
//! path, then verifies the output with dcpdoctor.

use dcpwizard_core::conform::{assemble_dcp, build_reel_plan, parse_timeline};
use std::path::Path;
use std::process::Command;

fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A short 2048x1080 24fps clip via ffmpeg testsrc.
fn make_clip(path: &Path, frames: u32) {
    let ok = Command::new("ffmpeg")
        .args(["-y", "-f", "lavfi", "-i", "testsrc=size=2048x1080:rate=24"])
        .args(["-frames:v", &frames.to_string(), "-pix_fmt", "yuv420p"])
        .arg(path)
        .output()
        .expect("run ffmpeg")
        .status
        .success();
    assert!(ok, "ffmpeg testsrc generation failed");
}

#[test]
fn two_reel_edl_conforms_to_a_dcp() {
    if !ffmpeg_available() {
        eprintln!("ffmpeg not available, skipping conform assembly test");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let media = root.path().join("media");
    std::fs::create_dir_all(&media).unwrap();
    make_clip(&media.join("REEL001.mov"), 8);
    make_clip(&media.join("REEL002.mov"), 8);

    // two video reels, each trimmed to source frames 0..6
    let edl = root.path().join("cut.edl");
    std::fs::write(
        &edl,
        "TITLE: Conform Test\nFCM: NON-DROP FRAME\n\n\
         001  REEL001  V  C        00:00:00:00 00:00:00:06 00:00:00:00 00:00:00:06\n\
         002  REEL002  V  C        00:00:00:00 00:00:00:06 00:00:00:06 00:00:00:12\n",
    )
    .unwrap();

    let timeline = parse_timeline(&edl).expect("parse edl");
    let plan = build_reel_plan(&timeline, &media).expect("resolve reels");
    assert_eq!(plan.reels.len(), 2, "two resolved reels");

    let out = root.path().join("dcp");
    assert_eq!(assemble_dcp(&plan, &out), 0, "conform assembly");

    // the reel plan artifact is kept next to the assembled DCP
    // (written by the CLI, not assemble_dcp; assert the DCP structure instead)
    let cpls: Vec<String> = std::fs::read_dir(&out)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("CPL_") && n.ends_with(".xml"))
        })
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    assert_eq!(cpls.len(), 1, "one assembled CPL");
    assert_eq!(
        cpls[0].matches("<Reel>").count(),
        2,
        "assembled CPL has two reels"
    );

    // and the assembled OV is verify-clean
    let result = dcpwizard_core::verify::verify_dcp(&out);
    assert!(result.valid, "dcpdoctor errors: {:?}", result.errors);
}
