use dcpwizard_core::isdcf_name::Rating;
use dcpwizard_core::library::Library;
use dcpwizard_core::rebuild::{RebuildConfig, RebuildSource};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

use crate::cancellable_run::CancellableRun;

const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(250);
const PROGRESS_EVENT: &str = "rebuild-progress";
const ALREADY_RUNNING: &str = "a rebuild is already running";
const KEEP_CONTAINER: &str = "keep";
const CUSTOM_CONTAINER: &str = "custom";
// the bounds transcode-dcp holds a custom size to
const CUSTOM_CONTAINER_UP_TO_4K: bool = true;

#[derive(Default)]
pub struct RebuildState(CancellableRun);

#[derive(Deserialize)]
pub struct RebuildRequest {
    source: String,
    output_parent: String,
    title: String,
    head_items: Vec<String>,
    tail_items: Vec<String>,
    ratings: Vec<Rating>,
    container: String,
    container_dims: Option<String>,
    video_bit_rate_mbps: Option<u32>,
}

#[derive(Clone, Default, Serialize)]
struct RebuildProgressEvent {
    percent: u32,
    message: String,
    frame: u64,
    total_frames: u64,
}

impl RebuildRequest {
    fn into_config(
        self,
        library: &Library,
        encode_threads: u32,
        detect_picture_findings: bool,
    ) -> Result<RebuildConfig, String> {
        Ok(RebuildConfig {
            container: container_of(&self.container, self.container_dims.as_deref())?,
            head_items: dcpwizard_core::library_reel::attach_by_name(library, &self.head_items)?,
            tail_items: dcpwizard_core::library_reel::attach_by_name(library, &self.tail_items)?,
            source: PathBuf::from(self.source),
            output_parent: PathBuf::from(self.output_parent),
            title: self.title,
            ratings: self.ratings,
            video_bit_rate_mbps: self.video_bit_rate_mbps,
            encode_threads,
            detect_picture_findings,
        })
    }
}

fn container_of(container: &str, dims: Option<&str>) -> Result<Option<(u32, u32)>, String> {
    match container {
        KEEP_CONTAINER => Ok(None),
        CUSTOM_CONTAINER => {
            let dims = dims
                .filter(|dims| !dims.is_empty())
                .ok_or("a custom container needs its size as WxH")?;
            dcpwizard_core::dcp::resolve_container(None, Some(dims), CUSTOM_CONTAINER_UP_TO_4K)
                .map(Some)
        }
        name => dcpwizard_core::dcp::resolve_container(Some(name), None, CUSTOM_CONTAINER_UP_TO_4K)
            .map(Some),
    }
}

struct RebuildProgress {
    app: AppHandle,
    cancel: Arc<AtomicBool>,
    shown: Mutex<ShownProgress>,
}

#[derive(Default)]
struct ShownProgress {
    event: RebuildProgressEvent,
    last_emit: Option<Instant>,
}

impl RebuildProgress {
    fn emit(&self, shown: &mut ShownProgress, now_due: bool) {
        let now = Instant::now();
        let due = shown
            .last_emit
            .is_none_or(|last| now.duration_since(last) >= PROGRESS_EMIT_INTERVAL);
        if !due && !now_due {
            return;
        }
        shown.last_emit = Some(now);
        let _ = self.app.emit(PROGRESS_EVENT, shown.event.clone());
    }
}

impl dcpwizard_core::dcp::ProgressSink for RebuildProgress {
    fn stage(&self, percent: u32, message: &str) {
        let mut shown = self.shown.lock().expect("rebuild progress lock");
        let new_stage = shown.event.message != message;
        shown.event.percent = percent;
        shown.event.message = message.to_string();
        self.emit(&mut shown, new_stage);
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }

    fn frames(&self, done: u64, total: u64) {
        let mut shown = self.shown.lock().expect("rebuild progress lock");
        shown.event.frame = done;
        shown.event.total_frames = total;
        self.emit(&mut shown, done >= total);
    }
}

#[tauri::command]
pub fn rebuild_inspect(dcp_dir: String) -> Result<RebuildSource, String> {
    dcpwizard_core::rebuild::inspect(Path::new(&dcp_dir))
}

#[tauri::command]
pub async fn rebuild_dcp(
    app: AppHandle,
    state: State<'_, RebuildState>,
    request: RebuildRequest,
) -> Result<String, String> {
    let preferences = dcpwizard_core::preferences::load_preferences()
        .map_err(|e| format!("Cannot load the preferences: {e}"))?;
    let config = request.into_config(
        &Library::open(),
        preferences.encode_threads,
        preferences.detect_picture_findings,
    )?;
    let _running = state.0.claim(ALREADY_RUNNING)?;
    let progress = RebuildProgress {
        app,
        cancel: state.0.cancel_flag(),
        shown: Mutex::default(),
    };
    let package = tokio::task::spawn_blocking(move || {
        dcpwizard_core::rebuild::rebuild_with_progress(&config, &progress)
    })
    .await
    .unwrap_or_else(|e| Err(format!("rebuild task panicked: {e}")))?;
    Ok(package.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn rebuild_cancel(state: State<'_, RebuildState>) {
    state.0.cancel();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(container: &str, container_dims: Option<&str>) -> RebuildRequest {
        RebuildRequest {
            source: "/films/Film_FTR".into(),
            output_parent: "/films/rebuilt".into(),
            title: "Film UK".into(),
            head_items: Vec::new(),
            tail_items: Vec::new(),
            ratings: vec![Rating {
                agency: "http://www.bbfc.co.uk".into(),
                label: "12A".into(),
            }],
            container: container.into(),
            container_dims: container_dims.map(str::to_string),
            video_bit_rate_mbps: None,
        }
    }

    fn config_of(request: RebuildRequest) -> Result<RebuildConfig, String> {
        let library = tempfile::tempdir().unwrap();
        request.into_config(&Library::open_at(library.path().to_path_buf()), 4, true)
    }

    #[test]
    fn keep_leaves_the_container_alone() {
        let config = config_of(request(KEEP_CONTAINER, None)).unwrap();
        assert_eq!(config.container, None);
        assert_eq!(config.source, PathBuf::from("/films/Film_FTR"));
        assert_eq!(config.output_parent, PathBuf::from("/films/rebuilt"));
        assert_eq!(config.title, "Film UK");
        assert_eq!(config.ratings.len(), 1);
        assert_eq!(config.encode_threads, 4);
        assert!(config.detect_picture_findings);
    }

    #[test]
    fn every_named_container_reaches_the_config_as_its_raster() {
        for (name, raster) in dcpwizard_core::dcp::NAMED_CONTAINERS {
            let config = config_of(request(name, None)).unwrap();
            assert_eq!(config.container, Some(raster), "{name}");
        }
        let refused = config_of(request("8k-full", None)).err().unwrap();
        assert!(refused.contains("8k-full"), "{refused}");
    }

    #[test]
    fn a_custom_container_takes_a_size_up_to_4k() {
        let config = config_of(request(CUSTOM_CONTAINER, Some("3840x2160"))).unwrap();
        assert_eq!(config.container, Some((3840, 2160)));
        assert!(config_of(request(CUSTOM_CONTAINER, None)).is_err());
        assert!(config_of(request(CUSTOM_CONTAINER, Some(""))).is_err());
        assert!(config_of(request(CUSTOM_CONTAINER, Some("5000x2160"))).is_err());
    }

    #[test]
    fn an_item_missing_from_the_library_is_refused() {
        let refused = config_of(RebuildRequest {
            head_items: vec!["Studio Ident".into()],
            ..request(KEEP_CONTAINER, None)
        });
        assert!(refused.is_err());
    }
}
