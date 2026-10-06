//! The job daemon: a TCP IPC listener in front of a postkit job queue whose jobs
//! run dcpwizard's own operations (create/verify/export/import DCP).

use postkit::job_queue::{JobInfo, JobState, QueueJob};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

/// What a job is failed with when its worker thread ended without sending a
/// result back.
pub const WORKER_LOST_MESSAGE: &str = "the job thread stopped without reporting a result";

const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(500);
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(200);
const CANCELLED_MESSAGE: &str = "Cancelled";
const COMPLETED_MESSAGE: &str = "Completed successfully";

/// Job type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    CreateDcp,
    VerifyDcp,
    ExportDcp,
    ImportVideo,
    EncodeJ2k,
    WrapMxf,
    CopyToDrive,
}

const ALL_JOB_TYPES: [JobType; 7] = [
    JobType::CreateDcp,
    JobType::VerifyDcp,
    JobType::ExportDcp,
    JobType::ImportVideo,
    JobType::EncodeJ2k,
    JobType::WrapMxf,
    JobType::CopyToDrive,
];

impl JobType {
    pub fn name(self) -> &'static str {
        match self {
            JobType::CreateDcp => "create-dcp",
            JobType::VerifyDcp => "verify-dcp",
            JobType::ExportDcp => "export-dcp",
            JobType::ImportVideo => "import-video",
            JobType::EncodeJ2k => "encode-j2k",
            JobType::WrapMxf => "wrap-mxf",
            JobType::CopyToDrive => "copy-to-drive",
        }
    }

    pub fn from_name(name: &str) -> Option<JobType> {
        ALL_JOB_TYPES
            .into_iter()
            .find(|job_type| job_type.name() == name)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DaemonJob {
    pub id: u64,
    pub job_type: JobType,
    pub params: String,
}

impl QueueJob for DaemonJob {
    fn id(&self) -> u64 {
        self.id
    }

    fn title(&self) -> &str {
        self.job_type.name()
    }

    fn output_dir(&self) -> Option<&Path> {
        None
    }
}

pub type JobQueue = postkit::job_queue::JobQueue<DaemonJob>;

/// IPC request sent from CLI client to daemon.
#[derive(Debug, Serialize, Deserialize)]
pub enum IpcRequest {
    List,
    Submit { job_type: JobType, params: String },
    Cancel { id: u64 },
    Status { id: u64 },
    Move { id: u64, before: Option<u64> },
}

/// IPC response sent from daemon to CLI client.
#[derive(Serialize, Deserialize)]
pub enum IpcResponse {
    Jobs(Vec<JobInfo>),
    Submitted { id: u64 },
    Cancelled(bool),
    JobStatus(Option<JobInfo>),
    Moved(bool),
    Error(String),
}

fn submit(queue: &JobQueue, job_type: JobType, params: String) -> u64 {
    let id = queue.reserve_job_id();
    queue.submit(DaemonJob {
        id,
        job_type,
        params,
    });
    tracing::info!("Submitted job {id}");
    id
}

fn answer(queue: &JobQueue, request: IpcRequest) -> IpcResponse {
    match request {
        IpcRequest::List => IpcResponse::Jobs(queue.snapshot()),
        IpcRequest::Submit { job_type, params } => IpcResponse::Submitted {
            id: submit(queue, job_type, params),
        },
        IpcRequest::Cancel { id } => IpcResponse::Cancelled(queue.cancel(id)),
        IpcRequest::Status { id } => IpcResponse::JobStatus(queue.get(id)),
        IpcRequest::Move { id, before } => IpcResponse::Moved(queue.move_before(id, before)),
    }
}

/// Start the job queue processor in a background thread.
pub fn start_job_queue(queue: Arc<JobQueue>, encode_threads: u32) {
    std::thread::spawn(move || {
        tracing::info!("Job queue processor started");
        loop {
            let Some(job) = queue.take_next() else {
                queue.clear_current();
                std::thread::sleep(IDLE_POLL_INTERVAL);
                continue;
            };
            queue.start(&job);
            tracing::info!("Processing job {} ({:?})", job.id, job.job_type);

            // run the job on its own thread so the loop can watch the cancel
            // flag and finalise the job even if the operation is still running
            let control = JobControl {
                queue: queue.clone(),
                cancel: Arc::new(AtomicBool::new(false)),
            };
            let job_cancel = control.cancel.clone();
            let (sender, receiver) = mpsc::channel();
            let worker_job = job.clone();
            std::thread::spawn(move || {
                let outcome = process_job(&worker_job, &control, encode_threads);
                let _ = sender.send(outcome);
            });

            let outcome = loop {
                match receiver.recv_timeout(CANCEL_POLL_INTERVAL) {
                    Ok(outcome) => break Some(outcome),
                    Err(RecvTimeoutError::Timeout) => {
                        if queue.is_cancelled() {
                            // the queue's flag resets when the next job starts
                            job_cancel.store(true, Ordering::Relaxed);
                            break None;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break None,
                }
            };
            finish_job(&queue, &job, outcome);
        }
    });
}

fn finish_job(queue: &JobQueue, job: &DaemonJob, outcome: Option<Result<(), String>>) {
    let (state, message) = match outcome {
        _ if queue.is_cancelled() => (JobState::Cancelled, CANCELLED_MESSAGE.to_string()),
        Some(Ok(())) => (JobState::Completed, COMPLETED_MESSAGE.to_string()),
        Some(Err(cause)) => {
            tracing::error!("job {} failed: {cause}", job.id);
            (JobState::Failed, cause)
        }
        None => (JobState::Failed, WORKER_LOST_MESSAGE.to_string()),
    };
    queue.finish(job, state, &message);
    queue.clear_current();
}

/// Progress and cancel bridge handed to a running operation. Forwards stage
/// updates to the queue and exposes the job's own cancel flag.
struct JobControl {
    queue: Arc<JobQueue>,
    cancel: Arc<AtomicBool>,
}

impl crate::dcp::ProgressSink for JobControl {
    fn stage(&self, percent: u32, message: &str) {
        // a detached worker would overwrite the next job's progress
        if self.cancelled() {
            return;
        }
        self.queue.set_progress(f64::from(percent), message);
    }
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(params: &str, job_type: &str) -> Result<T, String> {
    serde_json::from_str(params).map_err(|e| format!("invalid {job_type} params: {e}"))
}

/// An operation that reports only an exit code leaves the cause in the daemon
/// log, so say which operation failed and with what.
fn from_exit_code(code: i32, operation: &str) -> Result<(), String> {
    if code == 0 {
        return Ok(());
    }
    Err(format!(
        "{operation} failed with code {code}, the daemon log holds the cause"
    ))
}

fn process_job(job: &DaemonJob, control: &JobControl, encode_threads: u32) -> Result<(), String> {
    match job.job_type {
        JobType::CreateDcp => {
            let mut config = crate::dcp::DcpConfig {
                encode_threads,
                ..parse_params(&job.params, "CreateDcp")?
            };
            config.output_dir =
                crate::package_dir::new_package_dir(&config.output_dir, &config.title)?;
            crate::dcp::create_dcp_with_progress(&config, control)
        }
        JobType::VerifyDcp => {
            let path = std::path::PathBuf::from(&job.params);
            let result = crate::verify::verify_dcp(&path);
            if result.valid {
                return Ok(());
            }
            let cause = result.errors.join("; ");
            Err(if cause.is_empty() {
                "the DCP did not verify".to_string()
            } else {
                cause
            })
        }
        JobType::ExportDcp => {
            let config = parse_params::<crate::export::ExportConfig>(&job.params, "ExportDcp")?;
            crate::export::export_dcp(&config, &control.cancel, &mut |frame, total_frames| {
                crate::dcp::ProgressSink::stage(
                    control,
                    (frame * 100 / total_frames) as u32,
                    &format!("{frame}/{total_frames} frames"),
                );
            })
        }
        JobType::ImportVideo => {
            let config = parse_params::<crate::import::ImportConfig>(&job.params, "ImportVideo")?;
            from_exit_code(crate::import::import_video(&config), "importing the video")
        }
        JobType::EncodeJ2k => {
            let encode = crate::encode::ImageSequenceEncode {
                encode_threads,
                ..parse_params(&job.params, "EncodeJ2k")?
            };
            let report = |progress: &postkit::pipeline::PipelineProgress| {
                crate::dcp::ProgressSink::stage(
                    control,
                    progress.percent as u32,
                    &format!("{}/{} frames", progress.frame, progress.total_frames),
                );
            };
            crate::encode::encode_image_sequence(&encode, &control.cancel, report).map(|_| ())
        }
        JobType::WrapMxf => {
            let config = parse_params::<crate::mxf_wrap::MxfWrapConfig>(&job.params, "WrapMxf")?;
            from_exit_code(crate::mxf_wrap::wrap_mxf(&config), "wrapping the MXF")
        }
        JobType::CopyToDrive => {
            // params is JSON {"source": "...", "target": "..."}
            let map = parse_params::<HashMap<String, String>>(&job.params, "CopyToDrive")?;
            let src = std::path::Path::new(map.get("source").map(|s| s.as_str()).unwrap_or(""));
            let dst = std::path::Path::new(map.get("target").map(|s| s.as_str()).unwrap_or(""));
            from_exit_code(
                crate::copy_drive::copy_to_drive(src, dst),
                "copying to the drive",
            )
        }
    }
}

/// Get the daemon address.
/// Uses TCP localhost on a fixed port for cross-platform compatibility.
pub fn daemon_addr() -> String {
    std::env::var("DCPWIZARD_DAEMON_ADDR").unwrap_or_else(|_| "127.0.0.1:9457".to_string())
}

/// Start the daemon IPC listener.
/// Binds a TCP listener on localhost and processes client requests.
/// This blocks the current thread.
pub fn start_daemon_ipc(queue: Arc<JobQueue>, encode_threads: u32) -> i32 {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    let addr = daemon_addr();

    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("failed to bind {addr}: {e}");
            return -1;
        }
    };

    tracing::info!("Daemon listening on {addr}");

    queue.load_jobs_file();

    // Start the job processor thread
    start_job_queue(queue.clone(), encode_threads);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let queue = queue.clone();
                std::thread::spawn(move || {
                    let reader = BufReader::new(match stream.try_clone() {
                        Ok(s) => s,
                        Err(_) => return,
                    });

                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break,
                        };

                        let request: IpcRequest = match serde_json::from_str(&line) {
                            Ok(r) => r,
                            Err(e) => {
                                let resp = IpcResponse::Error(format!("invalid request: {e}"));
                                let _ = writeln!(
                                    stream,
                                    "{}",
                                    serde_json::to_string(&resp).unwrap_or_default()
                                );
                                continue;
                            }
                        };

                        let response = answer(&queue, request);

                        let json = serde_json::to_string(&response).unwrap_or_default();
                        if writeln!(stream, "{json}").is_err() {
                            break;
                        }
                    }
                });
            }
            Err(e) => {
                tracing::error!("accept error: {e}");
            }
        }
    }

    0
}

/// Send an IPC request to the running daemon and return the response.
pub fn send_ipc_request(request: &IpcRequest) -> Result<IpcResponse, String> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    let addr = daemon_addr();
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| format!("cannot connect to daemon at {addr}: {e} (is the daemon running?)"))?;

    let json = serde_json::to_string(request).map_err(|e| format!("serialize error: {e}"))?;
    writeln!(stream, "{json}").map_err(|e| format!("write error: {e}"))?;

    let reader = BufReader::new(stream);
    let line = reader
        .lines()
        .next()
        .ok_or_else(|| "no response from daemon".to_string())?
        .map_err(|e| format!("read error: {e}"))?;

    serde_json::from_str(&line).map_err(|e| format!("invalid response: {e}"))
}

/// Check if the daemon is running by attempting a connection.
pub fn is_daemon_running() -> bool {
    use std::net::TcpStream;
    let addr = daemon_addr();
    TcpStream::connect_timeout(
        &addr
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:9457".parse().unwrap()),
        std::time::Duration::from_millis(500),
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use postkit::job_queue::INTERRUPTED_MESSAGE;
    use std::path::PathBuf;

    fn state_of(queue: &JobQueue, id: u64) -> JobState {
        queue.get(id).expect("the submitted job").state
    }

    #[test]
    fn reload_keeps_queued_and_fails_running() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state").join("jobs.jsonl");

        let queue = JobQueue::new(path.clone());
        let running = submit(&queue, JobType::VerifyDcp, "/dcp/one".into());
        let queued = submit(&queue, JobType::VerifyDcp, "/dcp/two".into());
        let started = queue.take_next().unwrap();
        assert_eq!(started.id, running);
        queue.start(&started);
        queue.set_progress(40.0, "Processing...");

        let reloaded = JobQueue::new(path.clone());
        assert_eq!(reloaded.load_jobs_file(), 0);

        assert_eq!(state_of(&reloaded, queued), JobState::Queued);
        let interrupted = reloaded.get(running).unwrap();
        assert_eq!(interrupted.state, JobState::Failed);
        assert_eq!(interrupted.message, INTERRUPTED_MESSAGE);

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2);
    }

    /// How long the queue processor gets to pick up and finish one job.
    const FAILURE_POLL_LIMIT: std::time::Duration = std::time::Duration::from_secs(10);

    fn wait_until_failed(queue: &JobQueue, id: u64) -> JobInfo {
        let deadline = std::time::Instant::now() + FAILURE_POLL_LIMIT;
        loop {
            let job = queue.get(id).expect("the submitted job");
            if job.state == JobState::Failed {
                return job;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "job stayed {:?} for {FAILURE_POLL_LIMIT:?}",
                job.state
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn a_failed_job_carries_the_runners_own_error_text() {
        let dir = tempfile::tempdir().unwrap();
        let missing_dcp = dir.path().join("no_such_dcp");

        let queue = Arc::new(JobQueue::new(dir.path().join("jobs.jsonl")));
        let id = submit(
            &queue,
            JobType::VerifyDcp,
            missing_dcp.to_str().unwrap().into(),
        );
        start_job_queue(queue.clone(), crate::preferences::AUTOMATIC_ENCODE_THREADS);

        let failed = wait_until_failed(&queue, id);
        assert!(
            failed.message.contains(missing_dcp.to_str().unwrap()),
            "message hid the cause: {}",
            failed.message
        );
    }

    #[test]
    fn a_failed_create_carries_the_missing_source_path() {
        let dir = tempfile::tempdir().unwrap();
        let missing_j2k = dir.path().join("no_such_frames");
        let config = crate::dcp::DcpConfig {
            title: "Test Film".into(),
            output_dir: dir.path().join("out"),
            j2k_dir: Some(missing_j2k.clone()),
            frame_rate_num: 24,
            frame_rate_den: 1,
            ..Default::default()
        };
        let params = serde_json::to_string(&config).unwrap();

        let queue = Arc::new(JobQueue::new(dir.path().join("jobs.jsonl")));
        let id = submit(&queue, JobType::CreateDcp, params);
        start_job_queue(queue.clone(), crate::preferences::AUTOMATIC_ENCODE_THREADS);

        let failed = wait_until_failed(&queue, id);
        assert!(
            failed.message.contains(missing_j2k.to_str().unwrap()),
            "message hid the cause: {}",
            failed.message
        );
    }

    const BLOCKING_FRAMES: usize = 48;
    const BLOCKING_FRAME_BYTES: usize = 4 * 1024 * 1024;
    const BLOCKING_FRAME_WIDTH: u32 = 2048;
    const BLOCKING_FRAME_HEIGHT: u32 = 1080;
    const BLOCKING_FRAME_RATE: u32 = 24;
    const END_OF_CODESTREAM_BYTES: usize = 2;
    const CANCELLED_WATCH_FACTOR: u32 = 2;
    const STATE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(20);
    // a create over the blocking frames takes seconds on a loaded box
    const CREATE_RUN_LIMIT: std::time::Duration = std::time::Duration::from_secs(180);

    // the wrapper reads only the codestream header
    fn blocking_codestream_directory(root: &Path) -> PathBuf {
        let frames = root.join("j2k");
        std::fs::create_dir_all(&frames).unwrap();
        let seed = root.join("seed.j2c");
        crate::pad::generate_black_frame(
            BLOCKING_FRAME_WIDTH,
            BLOCKING_FRAME_HEIGHT,
            BLOCKING_FRAME_RATE,
            &seed,
        )
        .unwrap();
        let mut codestream = std::fs::read(&seed).unwrap();
        let end_of_codestream = codestream.split_off(codestream.len() - END_OF_CODESTREAM_BYTES);
        codestream.resize(BLOCKING_FRAME_BYTES, 0);
        codestream.extend_from_slice(&end_of_codestream);
        for frame in 0..BLOCKING_FRAMES {
            std::fs::write(frames.join(format!("frame_{frame:05}.j2c")), &codestream).unwrap();
        }
        frames
    }

    fn create_params(frames: &Path, output: &Path) -> String {
        serde_json::to_string(&crate::dcp::DcpConfig {
            title: "Cancel Test".into(),
            output_dir: output.to_path_buf(),
            j2k_dir: Some(frames.to_path_buf()),
            frame_rate_num: BLOCKING_FRAME_RATE,
            frame_rate_den: 1,
            ..Default::default()
        })
        .unwrap()
    }

    fn wait_for_state(queue: &JobQueue, id: u64, wanted: JobState, limit: std::time::Duration) {
        let deadline = std::time::Instant::now() + limit;
        loop {
            let job = queue.get(id).expect("the submitted job");
            if job.state == wanted {
                return;
            }
            assert!(
                matches!(job.state, JobState::Queued | JobState::Running),
                "job ended {:?} ({}) while waiting for {wanted:?}",
                job.state,
                job.message
            );
            assert!(
                std::time::Instant::now() < deadline,
                "job stayed {:?} for {limit:?}",
                job.state
            );
            std::thread::sleep(STATE_POLL_INTERVAL);
        }
    }

    fn holds_assetmap(dir: &Path) -> bool {
        std::fs::read_dir(dir).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().starts_with("ASSETMAP"))
        })
    }

    #[test]
    fn a_create_cancelled_while_it_runs_stays_cancelled_and_writes_no_package() {
        let dir = tempfile::tempdir().unwrap();
        let frames = blocking_codestream_directory(dir.path());
        let queue = Arc::new(JobQueue::new(dir.path().join("jobs.jsonl")));
        start_job_queue(queue.clone(), crate::preferences::AUTOMATIC_ENCODE_THREADS);

        let finished_output = dir.path().join("finished");
        let started = std::time::Instant::now();
        let finished = submit(
            &queue,
            JobType::CreateDcp,
            create_params(&frames, &finished_output),
        );
        wait_for_state(&queue, finished, JobState::Completed, CREATE_RUN_LIMIT);
        let uncancelled_run_time = started.elapsed();
        assert!(
            holds_assetmap(&finished_output.join("Cancel Test")),
            "the uncancelled create wrote no ASSETMAP under the title folder"
        );

        let cancelled_output = dir.path().join("cancelled");
        let cancelled = submit(
            &queue,
            JobType::CreateDcp,
            create_params(&frames, &cancelled_output),
        );
        // starting the next job clears the queue's cancel flag under the detached worker
        submit(
            &queue,
            JobType::CreateDcp,
            create_params(&frames, &dir.path().join("following")),
        );
        wait_for_state(&queue, cancelled, JobState::Running, FAILURE_POLL_LIMIT);
        assert!(queue.cancel(cancelled));
        wait_for_state(&queue, cancelled, JobState::Cancelled, FAILURE_POLL_LIMIT);

        let watch_until = std::time::Instant::now() + uncancelled_run_time * CANCELLED_WATCH_FACTOR;
        while std::time::Instant::now() < watch_until {
            let job = queue.get(cancelled).expect("the submitted job");
            assert_eq!(job.state, JobState::Cancelled, "{}", job.message);
            std::thread::sleep(STATE_POLL_INTERVAL);
        }
        assert!(
            !holds_assetmap(&cancelled_output.join("Cancel Test")),
            "the cancelled create finished its package"
        );
    }

    #[test]
    fn a_stage_reported_after_a_cancel_leaves_the_job_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let queue = Arc::new(JobQueue::new(dir.path().join("jobs.jsonl")));
        let id = submit(&queue, JobType::EncodeJ2k, "{}".into());
        let job = queue.take_next().unwrap();
        queue.start(&job);
        let control = JobControl {
            queue: queue.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
        };

        assert!(queue.cancel(id));
        crate::dcp::ProgressSink::stage(&control, 50, "24/48 frames");
        finish_job(&queue, &job, Some(Ok(())));

        assert_eq!(state_of(&queue, id), JobState::Cancelled);
    }

    fn queue_with_three_jobs(dir: &Path) -> JobQueue {
        let queue = JobQueue::new(dir.join("jobs.jsonl"));
        for dcp in ["/dcp/one", "/dcp/two", "/dcp/three"] {
            submit(&queue, JobType::VerifyDcp, dcp.into());
        }
        queue
    }

    fn listed_ids(queue: &JobQueue) -> Vec<u64> {
        let IpcResponse::Jobs(jobs) = answer(queue, IpcRequest::List) else {
            panic!("List did not answer with jobs");
        };
        jobs.iter().map(|job| job.id).collect()
    }

    fn moved(queue: &JobQueue, id: u64, before: Option<u64>) -> bool {
        let IpcResponse::Moved(moved) = answer(queue, IpcRequest::Move { id, before }) else {
            panic!("Move did not answer with Moved");
        };
        moved
    }

    #[test]
    fn a_move_with_no_target_runs_the_job_next() {
        let dir = tempfile::tempdir().unwrap();
        let queue = queue_with_three_jobs(dir.path());
        assert!(moved(&queue, 3, None));
        assert_eq!(listed_ids(&queue), vec![3, 1, 2]);
    }

    #[test]
    fn a_move_before_a_job_puts_it_just_ahead_of_that_job() {
        let dir = tempfile::tempdir().unwrap();
        let queue = queue_with_three_jobs(dir.path());
        assert!(moved(&queue, 3, Some(2)));
        assert_eq!(listed_ids(&queue), vec![1, 3, 2]);
    }

    #[test]
    fn a_move_of_an_unknown_job_answers_false_and_keeps_the_order() {
        let dir = tempfile::tempdir().unwrap();
        let queue = queue_with_three_jobs(dir.path());
        assert!(!moved(&queue, 9, None));
        assert_eq!(listed_ids(&queue), vec![1, 2, 3]);
    }

    #[test]
    fn corrupt_line_is_skipped_and_counted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.jsonl");

        let queue = JobQueue::new(path.clone());
        let id = submit(&queue, JobType::VerifyDcp, "/dcp/one".into());
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("{not json}\n");
        std::fs::write(&path, text).unwrap();

        let reloaded = JobQueue::new(path.clone());
        assert_eq!(reloaded.load_jobs_file(), 1);
        assert_eq!(reloaded.snapshot().len(), 1);
        assert_eq!(state_of(&reloaded, id), JobState::Queued);
    }
}
