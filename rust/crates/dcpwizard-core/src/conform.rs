//! Timeline conformance — assemble reels from EDL/AAF/XML edit decisions.
//!
//! Delegates to [`postkit::conform`] for parsing timelines and conforming
//! media into the reel structure needed for DCP creation.

pub use postkit::conform::{
    ConformOptions, EditEvent, Timeline, TimelineFormat, conform, detect_timeline_format,
    find_missing_reels, parse_timeline,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_detect_format() {
        assert_eq!(
            detect_timeline_format(Path::new("edit.edl")),
            TimelineFormat::EdlCmx3600
        );
        assert_eq!(
            detect_timeline_format(Path::new("project.aaf")),
            TimelineFormat::Aaf
        );
        assert_eq!(
            detect_timeline_format(Path::new("timeline.otio")),
            TimelineFormat::Otio
        );
    }

    #[test]
    fn test_parse_edl() {
        let dir = tempfile::tempdir().unwrap();
        let edl = dir.path().join("test.edl");
        std::fs::write(
            &edl,
            "TITLE: Test\nFCM: NON-DROP FRAME\n\n001  REEL001  V  C        01:00:00:00 01:00:05:00 01:00:00:00 01:00:05:00\n",
        ).unwrap();
        let tl = parse_timeline(&edl);
        assert_eq!(tl.title, "Test");
        assert_eq!(tl.events.len(), 1);
        assert_eq!(tl.events[0].reel_name, "REEL001");
    }

    #[test]
    fn test_find_missing_reels() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("REEL001.mxf"), "").unwrap();
        let tl = Timeline {
            events: vec![
                EditEvent {
                    reel_name: "REEL001".into(),
                    ..Default::default()
                },
                EditEvent {
                    reel_name: "REEL002".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let missing = find_missing_reels(&tl, dir.path());
        assert_eq!(missing, vec!["REEL002"]);
    }
}
