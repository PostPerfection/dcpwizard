use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

// ─── Progress / Events ─────────────────────────────────────────────────────

#[derive(Clone, Serialize)]
pub struct PipelineProgress {
    pub job_id: u64,
    pub stage: String,
    pub message: String,
    pub frame: u64,
    pub total_frames: u64,
    pub fps: f64,
    pub elapsed_secs: f64,
    pub percent: f64,
}

#[derive(Clone, Serialize)]
pub struct JobInfo {
    pub id: u64,
    pub title: String,
    pub status: String,
    pub percent: f64,
}

// ─── Job types ─────────────────────────────────────────────────────────────

#[derive(Clone)]
#[allow(dead_code)]
struct JobConfig {
    id: u64,
    video_path: PathBuf,
    title: String,
    output_dir: PathBuf,
    audio_path: Option<String>,
    validate: bool,
    standard: String,
    resolution: String,
    framerate: String,
    bandwidth: u32,
    colour: String,
    content_kind: String,
    encrypt: bool,
    key_out: Option<String>,
    stereo_3d: bool,
    channels: String,
}

// ─── Queue state (managed by Tauri) ────────────────────────────────────────

pub struct JobQueue {
    queue: Mutex<VecDeque<JobConfig>>,
    next_id: AtomicU64,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    current_id: AtomicU64,
    current_title: Mutex<String>,
    current_status: Mutex<String>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            next_id: AtomicU64::new(1),
            cancel: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            current_id: AtomicU64::new(0),
            current_title: Mutex::new(String::new()),
            current_status: Mutex::new(String::new()),
        }
    }
}

// ─── Tauri commands ────────────────────────────────────────────────────────

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn submit_job(
    app: AppHandle,
    video_path: String,
    title: String,
    output_dir: String,
    audio_path: Option<String>,
    validate: Option<bool>,
    standard: Option<String>,
    resolution: Option<String>,
    framerate: Option<String>,
    bandwidth: Option<u32>,
    colour: Option<String>,
    content_kind: Option<String>,
    encrypt: Option<bool>,
    key_out: Option<String>,
    stereo_3d: Option<bool>,
    channels: Option<String>,
) -> Result<u64, String> {
    let queue = app.state::<JobQueue>();
    let id = queue.next_id.fetch_add(1, Ordering::Relaxed);

    // Never encrypt without an explicit key destination.
    if encrypt.unwrap_or(false) && key_out.as_deref().unwrap_or("").is_empty() {
        return Err("Key Output File is required when encrypting".into());
    }

    let job = JobConfig {
        id,
        video_path: PathBuf::from(&video_path),
        title: title.clone(),
        output_dir: PathBuf::from(&output_dir),
        audio_path,
        validate: validate.unwrap_or(false),
        standard: standard.unwrap_or_else(|| "smpte".into()),
        resolution: resolution.unwrap_or_else(|| "2k-full".into()),
        framerate: framerate.unwrap_or_else(|| "24".into()),
        bandwidth: bandwidth.unwrap_or(250),
        colour: colour.unwrap_or_else(|| "xyz".into()),
        content_kind: content_kind.unwrap_or_else(|| "feature".into()),
        encrypt: encrypt.unwrap_or(false),
        key_out: key_out.filter(|k| !k.is_empty()),
        stereo_3d: stereo_3d.unwrap_or(false),
        channels: channels.unwrap_or_else(|| "5.1".into()),
    };

    {
        let mut q = queue.queue.lock().unwrap();
        q.push_back(job);
    }

    if queue.current_id.load(Ordering::Relaxed) == 0 {
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            run_queue_worker(app2).await;
        });
    }

    Ok(id)
}

#[tauri::command]
pub async fn cancel_job(app: AppHandle, job_id: u64) -> Result<(), String> {
    let queue = app.state::<JobQueue>();
    if queue.current_id.load(Ordering::Relaxed) == job_id {
        queue.cancel.store(true, Ordering::Relaxed);
        return Ok(());
    }
    let mut q = queue.queue.lock().unwrap();
    q.retain(|j| j.id != job_id);
    Ok(())
}

#[tauri::command]
pub async fn pause_job(app: AppHandle) -> Result<(), String> {
    let queue = app.state::<JobQueue>();
    queue.pause.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn resume_job(app: AppHandle) -> Result<(), String> {
    let queue = app.state::<JobQueue>();
    queue.pause.store(false, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn list_jobs(app: AppHandle) -> Vec<JobInfo> {
    let queue = app.state::<JobQueue>();
    let mut jobs = Vec::new();

    let current_id = queue.current_id.load(Ordering::Relaxed);
    if current_id > 0 {
        let title = queue.current_title.lock().unwrap().clone();
        let status = queue.current_status.lock().unwrap().clone();
        jobs.push(JobInfo {
            id: current_id,
            title,
            status,
            percent: 0.0,
        });
    }

    let q = queue.queue.lock().unwrap();
    for job in q.iter() {
        jobs.push(JobInfo {
            id: job.id,
            title: job.title.clone(),
            status: "queued".to_string(),
            percent: 0.0,
        });
    }
    jobs
}

// ─── Version File (supplemental DCP) ───────────────────────────────────────

// One reel replacement from the GUI. Empty strings mean "reference the OV".
#[derive(Deserialize)]
pub struct VfReplacementInput {
    reel_number: u32,
    picture: Option<String>,
    sound: Option<String>,
}

#[tauri::command]
pub async fn create_vf(
    ov_dir: String,
    output_dir: String,
    title: Option<String>,
    replacements: Vec<VfReplacementInput>,
) -> Result<String, String> {
    let path_opt = |s: Option<String>| s.filter(|p| !p.is_empty()).map(PathBuf::from);
    let replacement_reels: Vec<dcpwizard_core::vf::ReplacementReel> = replacements
        .into_iter()
        .map(|r| dcpwizard_core::vf::ReplacementReel {
            reel_number: r.reel_number,
            picture: path_opt(r.picture),
            sound: path_opt(r.sound),
            subtitle: None,
        })
        .collect();

    if !replacement_reels
        .iter()
        .any(|r| r.picture.is_some() || r.sound.is_some())
    {
        return Err("Add at least one replacement reel with a picture or sound".into());
    }

    let config = dcpwizard_core::vf::VfConfig {
        ov_dir: PathBuf::from(&ov_dir),
        vf_dir: PathBuf::from(&output_dir),
        title: title.unwrap_or_default(),
        replacement_reels,
        subtitle_language: String::new(),
    };

    // create_vf does blocking IO (mxf wrap, hashing), keep it off the async runtime.
    let code = tokio::task::spawn_blocking(move || dcpwizard_core::vf::create_vf(&config))
        .await
        .map_err(|e| format!("VF task panicked: {e}"))?;

    if code == 0 {
        Ok(format!("Created Version File DCP at {output_dir}"))
    } else {
        Err(format!(
            "VF creation failed (rc={code}); see log for details"
        ))
    }
}

// ─── Queue worker ──────────────────────────────────────────────────────────

async fn run_queue_worker(app: AppHandle) {
    loop {
        let job = {
            let queue = app.state::<JobQueue>();
            let mut q = queue.queue.lock().unwrap();
            q.pop_front()
        };

        let Some(job) = job else {
            let queue = app.state::<JobQueue>();
            queue.current_id.store(0, Ordering::Relaxed);
            break;
        };

        {
            let queue = app.state::<JobQueue>();
            queue.current_id.store(job.id, Ordering::Relaxed);
            *queue.current_title.lock().unwrap() = job.title.clone();
            *queue.current_status.lock().unwrap() = "running".to_string();
            queue.cancel.store(false, Ordering::Relaxed);
            queue.pause.store(false, Ordering::Relaxed);
        }

        let result = tokio::task::spawn_blocking({
            let app = app.clone();
            let job = job.clone();
            move || run_job(&app, &job)
        })
        .await;

        let queue = app.state::<JobQueue>();
        match result {
            Ok(Ok(_)) => {
                *queue.current_status.lock().unwrap() = "done".to_string();
                emit_progress(&app, job.id, "done", "Complete", 0, 0, 0.0, 0.0, 100.0);
            }
            Ok(Err(e)) => {
                let status = if queue.cancel.load(Ordering::Relaxed) {
                    "cancelled".to_string()
                } else {
                    format!("failed: {e}")
                };
                *queue.current_status.lock().unwrap() = status;
                emit_progress(&app, job.id, "error", &e, 0, 0, 0.0, 0.0, 0.0);
            }
            Err(e) => {
                *queue.current_status.lock().unwrap() = format!("panic: {e}");
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

// ─── Job execution ─────────────────────────────────────────────────────────

fn log_to(log_file: &Arc<Mutex<Option<std::fs::File>>>, msg: &str) {
    eprintln!("[pipeline] {msg}");
    if let Some(f) = log_file.lock().unwrap().as_mut() {
        let _ = writeln!(f, "{msg}");
    }
}

fn run_job(app: &AppHandle, job: &JobConfig) -> Result<String, String> {
    let queue = app.state::<JobQueue>();
    let cancel = queue.cancel.clone();
    let pause = queue.pause.clone();

    let output = &job.output_dir;
    let log_path = output.join("dcpwizard.log");
    let log_file: Arc<Mutex<Option<std::fs::File>>> =
        Arc::new(Mutex::new(std::fs::File::create(&log_path).ok()));

    log_to(&log_file, "=== DCP Wizard Pipeline ===");
    log_to(&log_file, &format!("Job ID: {}", job.id));
    log_to(&log_file, &format!("Title: {}", job.title));
    log_to(&log_file, &format!("Input: {}", job.video_path.display()));
    log_to(&log_file, &format!("Output: {}", output.display()));
    log_to(
        &log_file,
        &format!(
            "Started: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
    );

    // Frame rate drives both the J2K encode (video demux rate) and the CPL.
    let (fps_num, fps_den) = match job.framerate.as_str() {
        "25" => (25, 1),
        "30" => (30, 1),
        "48" => (48, 1),
        "50" => (50, 1),
        "60" => (60, 1),
        "96" => (96, 1),
        "100" => (100, 1),
        "120" => (120, 1),
        _ => (24, 1),
    };

    // Map the target bandwidth (Mbps) to a J2K compression ratio, matching the
    // CLI convention (raw = w*h*36 bits/frame). Only honoured for video input;
    // image/J2K sequences fall back to the encoder default.
    let compression_ratio = dcpwizard_core::probe::probe_video(&job.video_path)
        .map(|info| {
            let fps = (fps_num as f64).max(1.0);
            let raw_bits = info.width as f64 * info.height as f64 * 36.0;
            let target_bits = (job.bandwidth as f64 * 1_000_000.0) / fps;
            (raw_bits / target_bits).max(1.0)
        })
        .unwrap_or(10.0);

    // Encode using shared pipeline
    let job_id = job.id;
    let app_ref = app.clone();
    let log_ref = log_file.clone();
    let encode_result = postkit::pipeline::run_encode_with_ratio(
        &job.video_path,
        output,
        compression_ratio,
        fps_num,
        &cancel,
        &pause,
        |p| {
            emit_progress(
                &app_ref,
                job_id,
                &p.stage,
                &p.message,
                p.frame,
                p.total_frames,
                p.fps,
                p.elapsed_secs,
                p.percent,
            );
        },
        |msg| log_to(&log_ref, msg),
    )?;

    // Package DCP
    emit_progress(
        app,
        job.id,
        "package",
        "Creating DCP...",
        0,
        0,
        0.0,
        0.0,
        99.0,
    );
    log_to(&log_file, "[PACKAGE] Creating DCP...");

    let standard = match job.standard.as_str() {
        "interop" => dcpwizard_core::Standard::Interop,
        _ => dcpwizard_core::Standard::Smpte,
    };

    let resolution = if job.resolution.contains("4k") {
        dcpwizard_core::Resolution::FourK
    } else {
        dcpwizard_core::Resolution::TwoK
    };
    // scope/flat/full are distinct containers, not just 2K vs 4K
    let (container_width, container_height) = match job.resolution.as_str() {
        "2k-scope" => (2048, 858),
        "2k-flat" => (1998, 1080),
        "2k-full" => (2048, 1080),
        "4k-scope" => (4096, 1716),
        "4k-flat" => (3996, 2160),
        "4k-full" => (4096, 2160),
        _ => (0, 0),
    };

    let content_type = match job.content_kind.as_str() {
        "trailer" => dcpwizard_core::ContentType::Trailer,
        "test" => dcpwizard_core::ContentType::Test,
        "short" => dcpwizard_core::ContentType::Short,
        "advertisement" => dcpwizard_core::ContentType::Advertisement,
        "episode" => dcpwizard_core::ContentType::Episode,
        _ => dcpwizard_core::ContentType::Feature,
    };

    let config = dcpwizard_core::dcp::DcpConfig {
        title: job.title.clone(),
        standard,
        resolution,
        container_width,
        container_height,
        content_type,
        output_dir: job.output_dir.clone(),
        frame_rate_num: fps_num,
        frame_rate_den: fps_den,
        max_bitrate_mbps: job.bandwidth,
        encrypt: job.encrypt,
        key_out: job.key_out.as_ref().map(std::path::PathBuf::from),
        stereo_3d: job.stereo_3d,
        j2k_dir: Some(encode_result.j2k_dir.clone()),
        audio_path: job
            .audio_path
            .as_ref()
            .filter(|a| !a.is_empty())
            .map(std::path::PathBuf::from),
        // subtitles are packaged via the CLI create --subtitle path, not the batch job
        ..Default::default()
    };

    let rc = dcpwizard_core::dcp::create_dcp(&config);
    if rc != 0 {
        log_to(&log_file, &format!("[PACKAGE] FAILED (rc={rc})"));
        return Err(format!("DCP packaging failed (rc={rc})"));
    }
    log_to(&log_file, "[PACKAGE] Done");

    // Optional validation
    if job.validate {
        emit_progress(
            app,
            job.id,
            "validate",
            "Validating DCP...",
            0,
            0,
            0.0,
            0.0,
            99.5,
        );
        log_to(&log_file, "[VALIDATE] Running validation...");

        let result = dcpwizard_core::verify::verify_dcp(&job.output_dir);

        for err in &result.errors {
            log_to(&log_file, &format!("[VALIDATE] ERROR: {err}"));
        }
        for warn in &result.warnings {
            log_to(&log_file, &format!("[VALIDATE] WARNING: {warn}"));
        }

        let _ = app.emit(
            "validation-result",
            serde_json::json!({
                "job_id": job.id,
                "valid": result.valid,
                "errors": result.errors,
                "warnings": result.warnings,
                "info": result.info,
            }),
        );

        let summary = if result.valid {
            "DCP is valid ✓".to_string()
        } else {
            format!(
                "Validation: {} error(s), {} warning(s)",
                result.errors.len(),
                result.warnings.len()
            )
        };
        log_to(&log_file, &format!("[VALIDATE] {summary}"));
        emit_progress(app, job.id, "validate", &summary, 0, 0, 0.0, 0.0, 100.0);
    }

    log_to(
        &log_file,
        &format!(
            "=== Pipeline finished in {:.1}s ===",
            encode_result.elapsed_secs
        ),
    );
    Ok(format!("DCP created in {:.1}s", encode_result.elapsed_secs))
}

// ─── Helpers ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn emit_progress(
    app: &AppHandle,
    job_id: u64,
    stage: &str,
    message: &str,
    frame: u64,
    total_frames: u64,
    fps: f64,
    elapsed_secs: f64,
    percent: f64,
) {
    let _ = app.emit(
        "pipeline-progress",
        PipelineProgress {
            job_id,
            stage: stage.to_string(),
            message: message.to_string(),
            frame,
            total_frames,
            fps,
            elapsed_secs,
            percent,
        },
    );
}
