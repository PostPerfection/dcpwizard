use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Subtitle format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SubtitleFormat {
    #[default]
    SmpteXml,
    InteropXml,
    Srt,
}

/// Subtitle configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubtitleConfig {
    pub input_file: PathBuf,
    pub output_file: PathBuf,
    pub format: SubtitleFormat,
    pub language: String,
    pub font_size: u32,
    pub font_color: String,
    /// Edit rate for the SMPTE timecode / EditRate (frames per second).
    pub fps: u32,
    /// Bottom-line Vposition as a percentage from the bottom of the screen
    /// (Valign="bottom"). Zero falls back to the default 8%.
    pub vposition: f64,
}

/// Default bottom-line position: 8% up from the bottom of the screen.
const DEFAULT_VPOSITION: f64 = 8.0;
/// Vertical gap between stacked subtitle lines, in percent of screen height.
const LINE_SPACING: f64 = 7.0;

/// Vposition (percent from the bottom, Valign="bottom") for line `j` of a cue
/// with `line_count` lines: the last line sits at `base`, earlier lines stack
/// upward at LINE_SPACING each.
fn line_vposition(base: f64, line_count: usize, j: usize) -> f64 {
    base + (line_count - 1 - j) as f64 * LINE_SPACING
}

/// Import subtitles from SRT format and convert to TTML/XML for DCP packaging.
pub fn import_subtitles(config: &SubtitleConfig) -> i32 {
    let srt_content = match std::fs::read_to_string(&config.input_file) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to read SRT file: {e}");
            return -1;
        }
    };

    let entries = parse_srt(&srt_content);
    if entries.is_empty() {
        tracing::error!(
            "No subtitle entries found in {}",
            config.input_file.display()
        );
        return -1;
    }

    let lang = if config.language.is_empty() {
        "en"
    } else {
        &config.language
    };

    let font_size = if config.font_size == 0 {
        42
    } else {
        config.font_size
    };
    let font_color = if config.font_color.is_empty() {
        "FFFFFFFF"
    } else {
        &config.font_color
    };
    let fps = if config.fps == 0 { 24 } else { config.fps };
    let vposition = if config.vposition <= 0.0 {
        DEFAULT_VPOSITION
    } else {
        config.vposition
    };

    let xml = match config.format {
        SubtitleFormat::SmpteXml | SubtitleFormat::Srt => {
            generate_smpte_ttml(&entries, lang, font_size, font_color, fps, vposition)
        }
        SubtitleFormat::InteropXml => {
            generate_interop_xml(&entries, lang, font_size, font_color, vposition)
        }
    };

    match std::fs::write(&config.output_file, xml) {
        Ok(()) => {
            tracing::info!(
                "Converted {} subtitle entries to {}",
                entries.len(),
                config.output_file.display()
            );
            0
        }
        Err(e) => {
            tracing::error!("Failed to write subtitle XML: {e}");
            -1
        }
    }
}

/// Burn subtitles into video frames using ffmpeg drawtext/subtitles filter.
pub fn burnin_subtitles(input_video: &Path, subtitle_file: &Path, output_video: &Path) -> i32 {
    let sub_path = subtitle_file.to_string_lossy();
    let filter = format!("subtitles='{}'", sub_path.replace('\'', "\\'"));

    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(input_video)
        .arg("-vf")
        .arg(&filter)
        .arg("-c:a")
        .arg("copy")
        .arg(output_video)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("Burned subtitles into {}", output_video.display());
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg subtitle burn-in failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            -1
        }
        Err(e) => {
            tracing::error!("Failed to run ffmpeg: {e}");
            -1
        }
    }
}

struct SrtEntry {
    start_ms: u64,
    end_ms: u64,
    text: String,
}

/// Parse SRT via the shared postkit parser, keeping raw millisecond timing so
/// each format below can render it at its own timecode rate.
fn parse_srt(content: &str) -> Vec<SrtEntry> {
    postkit::subtitle_retime::parse_srt(content)
        .into_iter()
        .filter(|c| !c.text.is_empty())
        .map(|c| SrtEntry {
            start_ms: c.start_ms,
            end_ms: c.end_ms,
            text: c.text,
        })
        .collect()
}

/// A subtitle cue in whole picture frames, used by reel splitting to filter and
/// rebase cues onto per-reel timelines.
#[derive(Debug, Clone)]
pub struct SubCue {
    pub start_frame: u64,
    pub end_frame: u64,
    pub text: String,
}

/// Frame count to ST 428-7 timecode "HH:MM:SS:FF" at `fps` frames/sec.
fn frames_to_dcst(total_frames: u64, fps: u32) -> String {
    let fps = fps.max(1) as u64;
    let frames = total_frames % fps;
    let secs = total_frames / fps;
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60,
        frames
    )
}

/// Parse an SRT file into frame-based cues at `fps` (for reel splitting).
pub fn parse_srt_frames(path: &Path, fps: u32) -> Result<Vec<SubCue>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let fps64 = fps.max(1) as u64;
    Ok(parse_srt(&content)
        .into_iter()
        .map(|e| SubCue {
            start_frame: e.start_ms * fps64 / 1000,
            end_frame: e.end_ms * fps64 / 1000,
            text: e.text,
        })
        .collect())
}

/// Write a reel's DCST from frame-based cues (already rebased to reel-local 0).
pub fn write_dcst_frames(cues: &[SubCue], lang: &str, fps: u32, out: &Path) -> Result<(), String> {
    let xml = render_dcst_frames(cues, lang, 42, "FFFFFFFF", fps, DEFAULT_VPOSITION);
    std::fs::write(out, xml).map_err(|e| e.to_string())
}

/// Milliseconds to Interop "HH:MM:SS.mmm".
fn ms_to_interop(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1000;
    let millis = ms % 1000;
    format!("{h:02}:{m:02}:{s:02}.{millis:03}")
}

fn generate_smpte_ttml(
    entries: &[SrtEntry],
    lang: &str,
    font_size: u32,
    font_color: &str,
    fps: u32,
    vposition: f64,
) -> String {
    let fps64 = fps.max(1) as u64;
    let cues: Vec<SubCue> = entries
        .iter()
        .map(|e| SubCue {
            start_frame: e.start_ms * fps64 / 1000,
            end_frame: e.end_ms * fps64 / 1000,
            text: e.text.clone(),
        })
        .collect();
    render_dcst_frames(&cues, lang, font_size, font_color, fps, vposition)
}

/// Shared ST 428-7 DCST renderer working from frame-based cues. Both the
/// single-reel path (via [`generate_smpte_ttml`]) and reel splitting use it.
fn render_dcst_frames(
    cues: &[SubCue],
    lang: &str,
    font_size: u32,
    font_color: &str,
    fps: u32,
    vposition: f64,
) -> String {
    let sub_id = uuid::Uuid::new_v4();
    // ~1/12 s fade, expressed in frames like the rest of the timecodes
    let fade = format!("00:00:00:{:02}", (fps as f64 / 12.0).round() as u64);
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<dcst:SubtitleReel xmlns:dcst=\"http://www.smpte-ra.org/schemas/428-7/2010/DCST\" xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n");
    xml.push_str(&format!("  <dcst:Id>urn:uuid:{sub_id}</dcst:Id>\n"));
    xml.push_str("  <dcst:ContentTitleText>Subtitles</dcst:ContentTitleText>\n");
    xml.push_str("  <dcst:AnnotationText>Subtitles</dcst:AnnotationText>\n");
    xml.push_str(&format!(
        "  <dcst:IssueDate>{}</dcst:IssueDate>\n",
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S+00:00")
    ));
    xml.push_str("  <dcst:ReelNumber>1</dcst:ReelNumber>\n");
    xml.push_str(&format!("  <dcst:Language>{lang}</dcst:Language>\n"));
    xml.push_str(&format!("  <dcst:EditRate>{fps} 1</dcst:EditRate>\n"));
    xml.push_str(&format!("  <dcst:TimeCodeRate>{fps}</dcst:TimeCodeRate>\n"));
    xml.push_str("  <dcst:SubtitleList>\n");
    xml.push_str(&format!(
        "    <dcst:Font ID=\"font1\" Color=\"{font_color}\" Size=\"{font_size}\" Effect=\"shadow\" EffectColor=\"FF000000\">\n"
    ));

    for (i, cue) in cues.iter().enumerate() {
        xml.push_str(&format!(
            "      <dcst:Subtitle SpotNumber=\"{}\" TimeIn=\"{}\" TimeOut=\"{}\" FadeUpTime=\"{fade}\" FadeDownTime=\"{fade}\">\n",
            i + 1,
            frames_to_dcst(cue.start_frame, fps),
            frames_to_dcst(cue.end_frame, fps),
        ));
        let lines: Vec<&str> = cue.text.split('\n').collect();
        for (j, line) in lines.iter().enumerate() {
            let vpos = line_vposition(vposition, lines.len(), j);
            xml.push_str(&format!(
                "        <dcst:Text Vposition=\"{vpos:.1}\" Valign=\"bottom\" Halign=\"center\">{}</dcst:Text>\n",
                postkit::packaging::escape_xml(line)
            ));
        }
        xml.push_str("      </dcst:Subtitle>\n");
    }

    xml.push_str("    </dcst:Font>\n");
    xml.push_str("  </dcst:SubtitleList>\n");
    xml.push_str("</dcst:SubtitleReel>\n");
    xml
}

fn generate_interop_xml(
    entries: &[SrtEntry],
    lang: &str,
    font_size: u32,
    font_color: &str,
    vposition: f64,
) -> String {
    let sub_id = uuid::Uuid::new_v4();
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<DCSubtitle Version=\"1.0\">\n");
    xml.push_str(&format!("  <SubtitleID>{sub_id}</SubtitleID>\n"));
    xml.push_str("  <MovieTitle>Subtitles</MovieTitle>\n");
    xml.push_str("  <ReelNumber>1</ReelNumber>\n");
    xml.push_str(&format!("  <Language>{lang}</Language>\n"));
    xml.push_str(&format!(
        "  <Font Id=\"font1\" Color=\"{font_color}\" Size=\"{font_size}\" Effect=\"shadow\" EffectColor=\"000000\">\n"
    ));

    for (i, entry) in entries.iter().enumerate() {
        xml.push_str(&format!(
            "    <Subtitle SpotNumber=\"{}\" TimeIn=\"{}\" TimeOut=\"{}\" FadeUpTime=\"2\" FadeDownTime=\"2\">\n",
            i + 1,
            ms_to_interop(entry.start_ms),
            ms_to_interop(entry.end_ms),
        ));
        let lines: Vec<&str> = entry.text.split('\n').collect();
        for (j, line) in lines.iter().enumerate() {
            let vpos = line_vposition(vposition, lines.len(), j);
            xml.push_str(&format!(
                "      <Text Vposition=\"{vpos:.1}\" VAlign=\"bottom\" HAlign=\"center\">{}</Text>\n",
                postkit::packaging::escape_xml(line)
            ));
        }
        xml.push_str("    </Subtitle>\n");
    }

    xml.push_str("  </Font>\n");
    xml.push_str("</DCSubtitle>\n");
    xml
}

/// High-level convenience: convert an SRT file to DCP SMPTE XML. `vposition` is
/// the bottom-line percentage from the bottom of the screen (0 uses the default).
pub fn convert_srt_to_dcp_xml(
    input: &Path,
    output: &Path,
    language: &str,
    fps: u32,
    vposition: f64,
) -> Result<(), String> {
    let config = SubtitleConfig {
        input_file: input.to_path_buf(),
        output_file: output.to_path_buf(),
        format: SubtitleFormat::SmpteXml,
        language: language.to_string(),
        font_size: 42,
        font_color: "FFFFFFFF".to_string(),
        fps,
        vposition,
    };
    let code = import_subtitles(&config);
    if code == 0 {
        Ok(())
    } else {
        Err("Subtitle conversion failed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_line_cue_anchors_at_bottom() {
        let entries = [SrtEntry {
            start_ms: 1000,
            end_ms: 4000,
            text: "line one\nline two".to_string(),
        }];
        let xml = generate_smpte_ttml(&entries, "en", 42, "FFFFFFFF", 24, DEFAULT_VPOSITION);
        // last line at 8%, the line above it at 15%, both anchored to the bottom
        assert!(
            xml.contains("Vposition=\"15.0\" Valign=\"bottom\""),
            "top line at 15%: {xml}"
        );
        assert!(
            xml.contains("Vposition=\"8.0\" Valign=\"bottom\""),
            "bottom line at 8%: {xml}"
        );
        assert!(
            !xml.contains("Vposition=\"85.0\""),
            "old top-anchored value gone"
        );
    }

    #[test]
    fn interop_two_line_cue_anchors_at_bottom() {
        let entries = [SrtEntry {
            start_ms: 1000,
            end_ms: 4000,
            text: "line one\nline two".to_string(),
        }];
        let xml = generate_interop_xml(&entries, "en", 42, "FFFFFFFF", DEFAULT_VPOSITION);
        assert!(
            xml.contains("Vposition=\"15.0\" VAlign=\"bottom\""),
            "{xml}"
        );
        assert!(xml.contains("Vposition=\"8.0\" VAlign=\"bottom\""), "{xml}");
    }

    #[test]
    fn custom_vposition_shifts_the_block() {
        let entries = [SrtEntry {
            start_ms: 0,
            end_ms: 1000,
            text: "solo".to_string(),
        }];
        let xml = generate_smpte_ttml(&entries, "en", 42, "FFFFFFFF", 24, 12.0);
        assert!(xml.contains("Vposition=\"12.0\""), "{xml}");
    }
}
