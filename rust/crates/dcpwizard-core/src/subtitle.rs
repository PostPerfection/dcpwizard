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

    let xml = match config.format {
        SubtitleFormat::SmpteXml | SubtitleFormat::Srt => {
            generate_smpte_ttml(&entries, lang, font_size, font_color)
        }
        SubtitleFormat::InteropXml => generate_interop_xml(&entries, lang, font_size, font_color),
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
    start: String,
    end: String,
    text: String,
}

fn parse_srt(content: &str) -> Vec<SrtEntry> {
    let mut entries = Vec::new();
    let mut lines = content.lines().peekable();

    while lines.peek().is_some() {
        // Skip blank lines and sequence numbers
        while let Some(line) = lines.peek() {
            if line.trim().is_empty() || line.trim().parse::<u32>().is_ok() {
                lines.next();
            } else {
                break;
            }
        }

        // Timestamp line: 00:00:01,000 --> 00:00:04,000
        let ts_line = match lines.next() {
            Some(l) if l.contains("-->") => l,
            _ => continue,
        };

        let parts: Vec<&str> = ts_line.split("-->").collect();
        if parts.len() != 2 {
            continue;
        }

        let start = srt_time_to_ttml(parts[0].trim());
        let end = srt_time_to_ttml(parts[1].trim());

        let mut text = String::new();
        while let Some(line) = lines.peek() {
            if line.trim().is_empty() {
                break;
            }
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(lines.next().unwrap().trim());
        }

        if !text.is_empty() {
            entries.push(SrtEntry { start, end, text });
        }
    }

    entries
}

fn srt_time_to_ttml(srt_time: &str) -> String {
    // Convert "00:00:01,000" to "00:00:01.000"
    srt_time.replace(',', ".")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn generate_smpte_ttml(
    entries: &[SrtEntry],
    lang: &str,
    font_size: u32,
    font_color: &str,
) -> String {
    let sub_id = uuid::Uuid::new_v4();
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<dcst:SubtitleReel xmlns:dcst=\"http://www.smpte-ra.org/schemas/428-7/2010/DCST\" xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n");
    xml.push_str(&format!("  <dcst:Id>urn:uuid:{sub_id}</dcst:Id>\n"));
    xml.push_str("  <dcst:ContentTitleText>Subtitles</dcst:ContentTitleText>\n");
    xml.push_str("  <dcst:AnnotationText>Subtitles</dcst:AnnotationText>\n");
    xml.push_str("  <dcst:IssueDate>2024-01-01T00:00:00+00:00</dcst:IssueDate>\n");
    xml.push_str("  <dcst:ReelNumber>1</dcst:ReelNumber>\n");
    xml.push_str(&format!("  <dcst:Language>{lang}</dcst:Language>\n"));
    xml.push_str("  <dcst:EditRate>24 1</dcst:EditRate>\n");
    xml.push_str("  <dcst:TimeCodeRate>24</dcst:TimeCodeRate>\n");
    xml.push_str("  <dcst:SubtitleList>\n");
    xml.push_str(&format!(
        "    <dcst:Font ID=\"font1\" Color=\"{font_color}\" Size=\"{font_size}\" Effect=\"shadow\" EffectColor=\"FF000000\">\n"
    ));

    for (i, entry) in entries.iter().enumerate() {
        xml.push_str(&format!(
            "      <dcst:Subtitle SpotNumber=\"{}\" TimeIn=\"{}\" TimeOut=\"{}\" FadeUpTime=\"00:00:00.083\" FadeDownTime=\"00:00:00.083\">\n",
            i + 1,
            entry.start,
            entry.end
        ));
        for (j, line) in entry.text.split('\n').enumerate() {
            let vpos = 85.0 - (j as f64 * 7.0);
            xml.push_str(&format!(
                "        <dcst:Text Vposition=\"{vpos:.1}\" Valign=\"bottom\" Halign=\"center\">{}</dcst:Text>\n",
                escape_xml(line)
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
            entry.start,
            entry.end
        ));
        for (j, line) in entry.text.split('\n').enumerate() {
            let vpos = 85.0 - (j as f64 * 7.0);
            xml.push_str(&format!(
                "      <Text Vposition=\"{vpos:.1}\" VAlign=\"bottom\" HAlign=\"center\">{}</Text>\n",
                escape_xml(line)
            ));
        }
        xml.push_str("    </Subtitle>\n");
    }

    xml.push_str("  </Font>\n");
    xml.push_str("</DCSubtitle>\n");
    xml
}
