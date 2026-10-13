use crate::assemble::{AssembleConfig, assemble_with_progress, parse_edit_rate, parse_input};
use crate::dcp::ProgressSink;
use crate::isdcf_name::Rating;
use crate::j2k_transcode::{
    DcpTranscodeConfig, check_picture_changes, transcode_dcp_with_progress,
};
use crate::library::AttachedItem;
use serde::Serialize;
use std::path::{Path, PathBuf};

// a dot keeps the re-encoded intermediate out of folder listings
const INTERMEDIATE_DIR_PREFIX: &str = ".rebuild_";
const TRANSCODE_END_PERCENT: u32 = 90;
const WHOLE_PERCENT: u32 = 100;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RebuildSource {
    pub title: String,
    pub standard: crate::Standard,
    pub stored_width: u32,
    pub stored_height: u32,
    pub active_width: u32,
    pub active_height: u32,
    pub edit_rate_numerator: u32,
    pub edit_rate_denominator: u32,
    pub reel_count: usize,
    pub total_frames: u64,
    pub content_kind: String,
    pub ratings: Vec<Rating>,
}

#[derive(Debug, Clone, Default)]
pub struct RebuildConfig {
    pub source: PathBuf,
    pub output_parent: PathBuf,
    pub title: String,
    pub head_items: Vec<AttachedItem>,
    pub tail_items: Vec<AttachedItem>,
    pub ratings: Vec<Rating>,
    pub container: Option<(u32, u32)>,
    pub video_bit_rate_mbps: Option<u32>,
    pub encode_threads: u32,
    pub detect_picture_findings: bool,
}

pub fn inspect(dcp_dir: &Path) -> Result<RebuildSource, String> {
    let input = parse_input(dcp_dir).map_err(|e| format!("{}: {e}", dcp_dir.display()))?;
    let (edit_rate_numerator, edit_rate_denominator) = parse_edit_rate(&input.edit_rate);
    Ok(RebuildSource {
        title: input.identity.title,
        standard: input.standard,
        stored_width: input.geometry.stored_width,
        stored_height: input.geometry.stored_height,
        active_width: input.geometry.active_width,
        active_height: input.geometry.active_height,
        edit_rate_numerator,
        edit_rate_denominator,
        reel_count: input.reels.len(),
        total_frames: input.reels.iter().map(|reel| reel.duration_frames).sum(),
        content_kind: input.identity.content_kind,
        ratings: input.identity.ratings,
    })
}

pub fn rebuild_with_progress(
    config: &RebuildConfig,
    progress: &dyn ProgressSink,
) -> Result<PathBuf, String> {
    inspect(&config.source)?;
    let package_dir = crate::package_dir::package_dir(&config.output_parent, &config.title)?;
    if package_dir.exists() {
        return Err(format!(
            "{} already exists. Use a new title or output folder.",
            package_dir.display()
        ));
    }
    let transcode = DcpTranscodeConfig {
        input_dir: config.source.clone(),
        // inside the output folder, so assemble moves its tracks by rename
        output_dir: config
            .output_parent
            .join(format!("{INTERMEDIATE_DIR_PREFIX}{}", uuid::Uuid::new_v4())),
        target_bitrate_mbps: config.video_bit_rate_mbps,
        container: config.container,
        encode_threads: config.encode_threads,
        ..Default::default()
    };
    let rebuilt = if check_picture_changes(&transcode).is_ok() {
        reencode_then_assemble(config, &transcode, &package_dir, progress)
    } else {
        assemble_into(config, &config.source, &package_dir, false, progress)
    };
    match rebuilt {
        Ok(()) => Ok(package_dir),
        Err(error) => Err(remove_created(
            error,
            &[&transcode.output_dir, &package_dir],
        )),
    }
}

fn reencode_then_assemble(
    config: &RebuildConfig,
    transcode: &DcpTranscodeConfig,
    package_dir: &Path,
    progress: &dyn ProgressSink,
) -> Result<(), String> {
    transcode_dcp_with_progress(
        transcode,
        &PhaseProgress {
            sink: progress,
            start_percent: 0,
            end_percent: TRANSCODE_END_PERCENT,
        },
    )?;
    assemble_into(
        config,
        &transcode.output_dir,
        package_dir,
        true,
        &PhaseProgress {
            sink: progress,
            start_percent: TRANSCODE_END_PERCENT,
            end_percent: WHOLE_PERCENT,
        },
    )?;
    std::fs::remove_dir_all(&transcode.output_dir).map_err(|e| {
        format!(
            "cannot remove the re-encoded intermediate {}: {e}",
            transcode.output_dir.display()
        )
    })
}

fn assemble_into(
    config: &RebuildConfig,
    input: &Path,
    package_dir: &Path,
    move_track_files: bool,
    progress: &dyn ProgressSink,
) -> Result<(), String> {
    assemble_with_progress(
        &AssembleConfig {
            inputs: vec![input.to_path_buf()],
            output_dir: package_dir.to_path_buf(),
            title: config.title.clone(),
            signer: None,
            head_items: config.head_items.clone(),
            tail_items: config.tail_items.clone(),
            ratings: Some(config.ratings.clone()),
            item_video_bit_rate_mbps: config.video_bit_rate_mbps,
            move_track_files,
            encode_threads: config.encode_threads,
            detect_picture_findings: config.detect_picture_findings,
        },
        progress,
    )
}

fn remove_created(error: String, dirs: &[&Path]) -> String {
    dirs.iter()
        .filter(|dir| dir.exists())
        .fold(error, |error, dir| match std::fs::remove_dir_all(dir) {
            Ok(()) => error,
            Err(e) => format!("{error}. Cannot remove {}: {e}", dir.display()),
        })
}

struct PhaseProgress<'a> {
    sink: &'a dyn ProgressSink,
    start_percent: u32,
    end_percent: u32,
}

impl ProgressSink for PhaseProgress<'_> {
    fn stage(&self, percent: u32, message: &str) {
        let span = self.end_percent - self.start_percent;
        let scaled = percent.min(WHOLE_PERCENT) * span / WHOLE_PERCENT;
        self.sink.stage(self.start_percent + scaled, message);
    }
    fn cancelled(&self) -> bool {
        self.sink.cancelled()
    }
    fn frames(&self, done: u64, total: u64) {
        self.sink.frames(done, total);
    }
}
