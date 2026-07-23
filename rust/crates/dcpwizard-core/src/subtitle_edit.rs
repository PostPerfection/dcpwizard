//! CLI subtitle editing (dom#828, dom#2071): list cues, shift/retime, and edit
//! cue text on a standalone subtitle file, writing SRT back out. It round-trips
//! any parsable input to SRT; it never edits subtitles inside a finished DCP.

use std::path::Path;

/// One editable cue: millisecond timing plus plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Load any supported subtitle format into editable cues (styling flattened to
/// plain text). A supplied SMPTE DCST XML is rejected: this edits source files,
/// not finished timed-text tracks.
pub fn load(path: &Path, fps: u32) -> Result<Vec<EditCue>, String> {
    let cues = crate::subtitle::load_styled_cues(path, fps)?;
    Ok(cues
        .iter()
        .map(|c| EditCue {
            start_ms: c.start_ms,
            end_ms: c.end_ms,
            text: c.plain_text(),
        })
        .collect())
}

/// Shift every cue by `delta_ms` (negative moves earlier, clamped at 0).
pub fn shift_all(cues: &mut [EditCue], delta_ms: i64) {
    for c in cues {
        c.start_ms = c.start_ms.saturating_add_signed(delta_ms);
        c.end_ms = c.end_ms.saturating_add_signed(delta_ms);
    }
}

/// Replace the text of the 1-based cue at `index`.
pub fn set_text(cues: &mut [EditCue], index: usize, text: &str) -> Result<(), String> {
    let cue = at_mut(cues, index)?;
    cue.text = text.to_string();
    Ok(())
}

/// Retime the 1-based cue at `index` to `[start_ms, end_ms)`.
pub fn set_timing(
    cues: &mut [EditCue],
    index: usize,
    start_ms: u64,
    end_ms: u64,
) -> Result<(), String> {
    if end_ms <= start_ms {
        return Err(format!(
            "cue {index}: end {end_ms} must be after start {start_ms}"
        ));
    }
    let cue = at_mut(cues, index)?;
    cue.start_ms = start_ms;
    cue.end_ms = end_ms;
    Ok(())
}

fn at_mut(cues: &mut [EditCue], index: usize) -> Result<&mut EditCue, String> {
    if index == 0 || index > cues.len() {
        return Err(format!(
            "cue index {index} out of range (1..={})",
            cues.len()
        ));
    }
    Ok(&mut cues[index - 1])
}

/// Render cues as SRT text (1-based indices, "HH:MM:SS,mmm" timing).
pub fn format_srt(cues: &[EditCue]) -> String {
    let mut out = String::new();
    for (i, c) in cues.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_time(c.start_ms),
            srt_time(c.end_ms),
            c.text
        ));
    }
    out
}

/// A one-line human summary of a cue for `--list`.
pub fn summary_line(index: usize, c: &EditCue) -> String {
    format!(
        "{index}\t{} --> {}\t{}",
        srt_time(c.start_ms),
        srt_time(c.end_ms),
        c.text.replace('\n', " / ")
    )
}

fn srt_time(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1000;
    let milli = ms % 1000;
    format!("{h:02}:{m:02}:{s:02},{milli:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cues() -> Vec<EditCue> {
        vec![
            EditCue {
                start_ms: 1000,
                end_ms: 4000,
                text: "hello".into(),
            },
            EditCue {
                start_ms: 5000,
                end_ms: 8000,
                text: "world\ntwo".into(),
            },
        ]
    }

    #[test]
    fn shift_moves_all_cues_and_round_trips_srt() {
        let mut c = cues();
        shift_all(&mut c, 500);
        assert_eq!(c[0].start_ms, 1500);
        assert_eq!(c[1].end_ms, 8500);
        let srt = format_srt(&c);
        // re-parse via the shared SRT parser to confirm a valid round-trip
        let back = postkit::subtitle_retime::parse_srt(&srt);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].start_ms, 1500);
        assert_eq!(back[1].text, "world\ntwo");
    }

    #[test]
    fn shift_negative_clamps_at_zero() {
        let mut c = cues();
        shift_all(&mut c, -2000);
        assert_eq!(c[0].start_ms, 0, "1000 - 2000 clamps to 0");
        assert_eq!(c[0].end_ms, 2000);
    }

    #[test]
    fn set_text_edits_the_indexed_cue_only() {
        let mut c = cues();
        set_text(&mut c, 2, "new line").unwrap();
        assert_eq!(c[1].text, "new line");
        assert_eq!(c[0].text, "hello");
        assert!(set_text(&mut c, 3, "x").is_err(), "out of range");
        assert!(set_text(&mut c, 0, "x").is_err(), "1-based, 0 invalid");
    }

    #[test]
    fn set_timing_rejects_inverted_window() {
        let mut c = cues();
        set_timing(&mut c, 1, 2000, 3000).unwrap();
        assert_eq!(c[0].start_ms, 2000);
        assert_eq!(c[0].end_ms, 3000);
        assert!(set_timing(&mut c, 1, 3000, 3000).is_err());
    }
}
