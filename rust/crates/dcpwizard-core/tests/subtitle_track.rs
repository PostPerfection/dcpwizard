//! End-to-end subtitle track: SRT -> ST 428-7 DCST XML (schema-checked) ->
//! timed-text MXF -> CPL registration.

use dcpwizard_core::cpl::{CplConfig, CplReel, generate_cpl};
use dcpwizard_core::mxf_wrap::{MxfType, MxfWrapConfig, wrap_mxf_result};
use dcpwizard_core::subtitle::convert_srt_to_dcp_xml;
use std::path::Path;

const SRT: &str = "1\n00:00:01,000 --> 00:00:04,000\nHello world\n\n2\n00:00:05,500 --> 00:00:08,000\nSecond line\nwith two rows\n";

fn xmllint_available() -> bool {
    std::process::Command::new("xmllint")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn srt_wraps_into_a_registered_timed_text_track() {
    let dir = tempfile::tempdir().unwrap();
    let srt = dir.path().join("in.srt");
    std::fs::write(&srt, SRT).unwrap();

    // 1. SRT -> ST 428-7 DCST XML at 24 fps.
    let dcst = dir.path().join("sub.xml");
    convert_srt_to_dcp_xml(&srt, &dcst, "de", 24).expect("srt->dcst");
    let xml = std::fs::read_to_string(&dcst).unwrap();
    // frame-based timecodes, not the illegal dot-millisecond form
    assert!(
        xml.contains("TimeOut=\"00:00:08:00\""),
        "frame timecode: {xml}"
    );

    // 2. Validate against the vendored ST 428-7 schema.
    if xmllint_available() {
        let xsd = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/schemas/DCDMSubtitle-2010.xsd");
        let ok = std::process::Command::new("xmllint")
            .args(["--noout", "--schema"])
            .arg(&xsd)
            .arg(&dcst)
            .output()
            .expect("run xmllint")
            .status
            .success();
        assert!(ok, "DCST XML must validate against ST 428-7 XSD");
    }

    // 3. Wrap the DCST XML into a timed-text MXF (real asdcplib).
    let mxf = dir.path().join("sub.mxf");
    let track = wrap_mxf_result(&MxfWrapConfig {
        input_path: dcst.clone(),
        output_mxf: mxf.clone(),
        mxf_type: MxfType::TimedText,
        frame_rate: 24,
        ..Default::default()
    })
    .expect("timed-text wrap");
    assert!(mxf.exists(), "MXF written");
    // 8.000 s out at 24 fps = 192 frames
    assert_eq!(track.duration, 192, "duration from the subtitle timing");

    // 4. Register the track in a CPL and confirm MainSubtitle is present.
    let cpl_path = dir.path().join("CPL.xml");
    let reel = CplReel {
        reel_id: "11111111-1111-1111-1111-111111111111".into(),
        picture_id: "22222222-2222-2222-2222-222222222222".into(),
        picture_width: 1998,
        picture_height: 1080,
        picture_edit_rate_num: 24,
        picture_edit_rate_den: 1,
        picture_duration: 192,
        subtitle_id: Some("33333333-3333-3333-3333-333333333333".into()),
        subtitle_edit_rate_num: 24,
        subtitle_edit_rate_den: 1,
        subtitle_duration: track.duration,
        subtitle_language: Some("de".into()),
        ..Default::default()
    };
    let config = CplConfig {
        title: "Sub Test".into(),
        content_kind: "feature".into(),
        reels: vec![reel],
        ..Default::default()
    };
    assert_eq!(
        generate_cpl(&config, "44444444-4444-4444-4444-444444444444", &cpl_path),
        0
    );
    let cpl = std::fs::read_to_string(&cpl_path).unwrap();
    assert!(
        cpl.contains("<MainSubtitle>"),
        "CPL registers the subtitle track"
    );
    assert!(
        cpl.contains("<Id>urn:uuid:33333333-3333-3333-3333-333333333333</Id>"),
        "MainSubtitle references the wrapped asset id"
    );
    assert!(cpl.contains("<Language>de</Language>"), "subtitle language");
}
