use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Job type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    CreateDcp,
    VerifyDcp,
    ExportDcp,
    ImportVideo,
    EncodeJ2k,
    WrapMxf,
    CopyToDrive,
}

/// Job state.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum JobState {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A queued job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub job_type: JobType,
    pub state: JobState,
    pub progress_percent: u32,
    pub message: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub params: String,
}

/// Thread-safe in-memory job queue.
#[derive(Clone)]
pub struct JobQueue {
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    running: Arc<Mutex<bool>>,
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Submit a new job to the queue.
    pub fn submit(&self, job_type: JobType, params: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let now = current_epoch_secs();

        let job = Job {
            id: id.clone(),
            job_type,
            state: JobState::Pending,
            progress_percent: 0,
            message: String::new(),
            created_at: now,
            updated_at: now,
            params: params.to_string(),
        };

        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.insert(id.clone(), job);
        }

        tracing::info!("Submitted job {id}");
        id
    }

    /// Cancel a job by ID.
    pub fn cancel(&self, id: &str) -> bool {
        if let Ok(mut jobs) = self.jobs.lock()
            && let Some(job) = jobs.get_mut(id)
                && (job.state == JobState::Pending || job.state == JobState::Running) {
                    job.state = JobState::Cancelled;
                    job.updated_at = current_epoch_secs();
                    tracing::info!("Cancelled job {id}");
                    return true;
                }
        false
    }

    /// Get a job by ID.
    pub fn get(&self, id: &str) -> Option<Job> {
        self.jobs.lock().ok()?.get(id).cloned()
    }

    /// List all jobs.
    pub fn list(&self) -> Vec<Job> {
        match self.jobs.lock() {
            Ok(jobs) => {
                let mut result: Vec<Job> = jobs.values().cloned().collect();
                result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                result
            }
            Err(_) => Vec::new(),
        }
    }

    /// Update a job's state and progress.
    pub fn update_job(&self, id: &str, state: JobState, progress: u32, message: &str) {
        if let Ok(mut jobs) = self.jobs.lock()
            && let Some(job) = jobs.get_mut(id) {
                job.state = state;
                job.progress_percent = progress;
                job.message = message.to_string();
                job.updated_at = current_epoch_secs();
            }
    }
}

/// Start the job queue processor in a background thread.
pub fn start_job_queue(queue: &JobQueue) {
    if let Ok(mut running) = queue.running.lock() {
        if *running {
            tracing::warn!("Job queue is already running");
            return;
        }
        *running = true;
    }

    let queue_clone = queue.clone();
    std::thread::spawn(move || {
        tracing::info!("Job queue processor started");
        loop {
            let is_running = queue_clone.running.lock().map(|r| *r).unwrap_or(false);
            if !is_running {
                break;
            }

            // Find next pending job
            let next_job = {
                let jobs = match queue_clone.jobs.lock() {
                    Ok(j) => j,
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        continue;
                    }
                };
                jobs.values()
                    .filter(|j| j.state == JobState::Pending)
                    .min_by_key(|j| j.created_at)
                    .cloned()
            };

            if let Some(job) = next_job {
                queue_clone.update_job(&job.id, JobState::Running, 0, "Processing...");
                tracing::info!("Processing job {} ({:?})", job.id, job.job_type);

                let result = process_job(&job);

                if result == 0 {
                    queue_clone.update_job(
                        &job.id,
                        JobState::Completed,
                        100,
                        "Completed successfully",
                    );
                } else {
                    queue_clone.update_job(&job.id, JobState::Failed, 0, "Job failed");
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        tracing::info!("Job queue processor stopped");
    });
}

/// Stop the job queue processor.
pub fn stop_job_queue(queue: &JobQueue) {
    if let Ok(mut running) = queue.running.lock() {
        *running = false;
    }
    tracing::info!("Job queue stop requested");
}

fn process_job(job: &Job) -> i32 {
    match job.job_type {
        JobType::CreateDcp => match serde_json::from_str::<crate::dcp::DcpConfig>(&job.params) {
            Ok(config) => crate::dcp::create_dcp(&config),
            Err(e) => {
                tracing::error!("Invalid CreateDcp params: {e}");
                -1
            }
        },
        JobType::VerifyDcp => {
            let path = std::path::PathBuf::from(&job.params);
            let result = crate::verify::verify_dcp(&path);
            if result.valid { 0 } else { -1 }
        }
        JobType::ExportDcp => {
            match serde_json::from_str::<crate::export::ExportConfig>(&job.params) {
                Ok(config) => crate::export::export_dcp(&config),
                Err(e) => {
                    tracing::error!("Invalid ExportDcp params: {e}");
                    -1
                }
            }
        }
        JobType::ImportVideo => {
            match serde_json::from_str::<crate::import::ImportConfig>(&job.params) {
                Ok(config) => crate::import::import_video(&config),
                Err(e) => {
                    tracing::error!("Invalid ImportVideo params: {e}");
                    -1
                }
            }
        }
        JobType::EncodeJ2k => {
            match serde_json::from_str::<crate::encode::EncodeConfig>(&job.params) {
                Ok(config) => crate::encode::encode_j2k(&config),
                Err(e) => {
                    tracing::error!("Invalid EncodeJ2k params: {e}");
                    -1
                }
            }
        }
        JobType::WrapMxf => {
            match serde_json::from_str::<crate::mxf_wrap::MxfWrapConfig>(&job.params) {
                Ok(config) => crate::mxf_wrap::wrap_mxf(&config),
                Err(e) => {
                    tracing::error!("Invalid WrapMxf params: {e}");
                    -1
                }
            }
        }
        JobType::CopyToDrive => {
            // params is JSON {"source": "...", "target": "..."}
            let parsed: Result<HashMap<String, String>, _> = serde_json::from_str(&job.params);
            match parsed {
                Ok(map) => {
                    let src =
                        std::path::Path::new(map.get("source").map(|s| s.as_str()).unwrap_or(""));
                    let dst =
                        std::path::Path::new(map.get("target").map(|s| s.as_str()).unwrap_or(""));
                    crate::copy_drive::copy_to_drive(src, dst)
                }
                Err(e) => {
                    tracing::error!("Invalid CopyToDrive params: {e}");
                    -1
                }
            }
        }
    }
}

fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
