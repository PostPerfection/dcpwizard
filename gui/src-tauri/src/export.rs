use dcpwizard_core::export::{ExportConfig, ExportFormat};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

use crate::cancellable_run::CancellableRun;

const DEFAULT_CRF: u32 = 18;
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(250);
const ALREADY_RUNNING: &str = "an export is already running";

const FORMAT_WORDS: [(&str, ExportFormat); 5] = [
    ("prores", ExportFormat::ProRes),
    ("h264", ExportFormat::H264),
    ("h265", ExportFormat::H265),
    ("dnxhr", ExportFormat::DnxHr),
    ("image-sequence", ExportFormat::ImageSequence),
];

#[derive(Deserialize)]
pub struct ExportRequest {
    input: String,
    output: String,
    format: String,
    crf: Option<u32>,
    audio: Option<String>,
    kdm: Option<String>,
    recipient_key: Option<String>,
    keys: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct ExportProgress {
    frame: u64,
    total_frames: u64,
    fps: f64,
    elapsed_secs: f64,
}

#[derive(Default)]
pub struct ExportState(CancellableRun);

fn optional_path(path: Option<String>) -> Option<PathBuf> {
    path.filter(|path| !path.is_empty()).map(PathBuf::from)
}

impl ExportRequest {
    fn into_config(self) -> Result<ExportConfig, String> {
        let format = FORMAT_WORDS
            .iter()
            .find(|(word, _)| *word == self.format)
            .map(|(_, format)| *format)
            .ok_or_else(|| format!("unknown export format '{}'", self.format))?;
        Ok(ExportConfig {
            input: PathBuf::from(self.input),
            output_path: PathBuf::from(self.output),
            format,
            quality_crf: self.crf.unwrap_or(DEFAULT_CRF),
            audio_mxf: optional_path(self.audio),
            kdm: optional_path(self.kdm),
            recipient_key: optional_path(self.recipient_key),
            keys: optional_path(self.keys),
        })
    }
}

#[tauri::command]
pub async fn export_dcp(
    app: AppHandle,
    state: State<'_, ExportState>,
    request: ExportRequest,
) -> Result<(), String> {
    let config = request.into_config()?;
    let _running = state.0.claim(ALREADY_RUNNING)?;
    let cancel = state.0.cancel_flag();
    let progress_app = app.clone();
    let exported = tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let mut last_emit: Option<Instant> = None;
        dcpwizard_core::export::export_dcp(&config, &cancel, &mut |frame, total_frames| {
            let now = Instant::now();
            let due =
                last_emit.is_none_or(|last| now.duration_since(last) >= PROGRESS_EMIT_INTERVAL);
            if !due && frame < total_frames {
                return;
            }
            last_emit = Some(now);
            let elapsed_secs = now.duration_since(started).as_secs_f64();
            let fps = if elapsed_secs > 0.0 {
                frame as f64 / elapsed_secs
            } else {
                0.0
            };
            let _ = progress_app.emit(
                "export-progress",
                ExportProgress {
                    frame,
                    total_frames,
                    fps,
                    elapsed_secs,
                },
            );
        })
    })
    .await
    .unwrap_or_else(|e| Err(format!("export task panicked: {e}")));

    exported
}

#[tauri::command]
pub fn export_cancel(state: State<'_, ExportState>) {
    state.0.cancel();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    fn request(format: &str) -> ExportRequest {
        ExportRequest {
            input: "/films/dcp".into(),
            output: "/exports/film.mov".into(),
            format: format.into(),
            crf: None,
            audio: None,
            kdm: None,
            recipient_key: None,
            keys: None,
        }
    }

    #[test]
    fn every_cli_format_word_maps_to_its_format() {
        for (word, format) in FORMAT_WORDS {
            assert_eq!(
                request(word).into_config().unwrap().format,
                format,
                "{word}"
            );
        }
    }

    #[test]
    fn an_unknown_format_word_is_refused_by_name() {
        let refused = request("avi").into_config().unwrap_err();
        assert!(refused.contains("'avi'"), "{refused}");
    }

    #[test]
    fn a_request_without_a_crf_gets_the_default() {
        assert_eq!(
            request("h264").into_config().unwrap().quality_crf,
            DEFAULT_CRF
        );
        let chosen = ExportRequest {
            crf: Some(23),
            ..request("h264")
        };
        assert_eq!(chosen.into_config().unwrap().quality_crf, 23);
    }

    #[test]
    fn the_paths_reach_the_config_and_empty_optional_paths_are_none() {
        let config = ExportRequest {
            audio: Some(String::new()),
            kdm: Some("/keys/film.kdm.xml".into()),
            recipient_key: Some(String::new()),
            keys: None,
            ..request("prores")
        }
        .into_config()
        .unwrap();
        assert_eq!(config.input, PathBuf::from("/films/dcp"));
        assert_eq!(config.output_path, PathBuf::from("/exports/film.mov"));
        assert_eq!(config.audio_mxf, None);
        assert_eq!(config.kdm, Some(PathBuf::from("/keys/film.kdm.xml")));
        assert_eq!(config.recipient_key, None);
        assert_eq!(config.keys, None);
    }

    #[test]
    fn a_second_export_is_refused_while_one_runs_and_allowed_after() {
        let state = ExportState::default();
        let running = state.0.claim(ALREADY_RUNNING).unwrap();
        assert_eq!(
            state.0.claim(ALREADY_RUNNING).err(),
            Some(ALREADY_RUNNING.to_string())
        );
        drop(running);
        assert!(state.0.claim(ALREADY_RUNNING).is_ok());
    }

    #[test]
    fn a_new_export_starts_with_the_cancel_flag_clear() {
        let state = ExportState::default();
        state.0.cancel();
        let _running = state.0.claim(ALREADY_RUNNING).unwrap();
        assert!(!state.0.cancel_flag().load(Ordering::Acquire));
    }
}
