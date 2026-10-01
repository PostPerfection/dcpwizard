use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

const LOG_EXTENSION: &str = "log";
const LOG_TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const BITS_PER_MEGABIT: f64 = 1_000_000.0;

// dcpdoctor flags any file inside the package that the ASSETMAP does not list
pub fn job_log_path(output: &Path) -> Result<PathBuf, String> {
    let Some(folder_name) = output.file_name() else {
        return Err(format!(
            "Cannot name a job log beside the output folder {}: it has no folder name",
            output.display()
        ));
    };
    let mut log_name = folder_name.to_os_string();
    log_name.push(".");
    log_name.push(LOG_EXTENSION);
    Ok(output.with_file_name(log_name))
}

// the three states the GUI's job log names the accelerator by, so one log reads
// like the other
pub fn accelerator_status(requested: bool, active: bool, error: Option<&str>) -> String {
    match (requested, active, error) {
        (false, _, _) => "off".to_string(),
        (true, true, _) => "requested, active".to_string(),
        (true, false, Some(error)) => format!("requested, inactive: {error}"),
        (true, false, None) => "requested, inactive".to_string(),
    }
}

pub fn encode_threads_status(encode_threads: u32) -> String {
    let count = postkit::grok_encoder::encode_thread_count(encode_threads);
    if encode_threads == crate::preferences::AUTOMATIC_ENCODE_THREADS {
        return format!("{count} (automatic)");
    }
    count.to_string()
}

pub fn picture_findings_status(detect_picture_findings: bool) -> &'static str {
    if detect_picture_findings { "on" } else { "off" }
}

pub fn log_timestamp() -> String {
    chrono::Local::now()
        .format(LOG_TIMESTAMP_FORMAT)
        .to_string()
}

pub fn source_line(source: &postkit::probe::VideoInfo) -> String {
    let bit_rate = source.bit_rate.map_or("unknown".to_string(), |bits| {
        format!("{:.1}", bits as f64 / BITS_PER_MEGABIT)
    });
    format!(
        "Source: {} {}x{} {}/{} {} frames, {}, colour {} {} {} {}, {bit_rate} Mbit/s",
        source.codec_name,
        source.width,
        source.height,
        source.fps_num,
        source.fps_den,
        source.total_frames,
        source.pix_fmt,
        source.color_space,
        source.color_range,
        source.color_transfer,
        source.color_primaries,
    )
}

pub fn settings_line(settings: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string(settings)
        .map(|json| format!("Settings: {json}"))
        .map_err(|e| format!("Cannot write the job settings to the job log: {e}"))
}

pub enum JobOutcome<'a> {
    Done,
    Failed(&'a str),
    Cancelled,
}

pub fn finished_line(timestamp: &str, outcome: &JobOutcome) -> String {
    match outcome {
        JobOutcome::Done => format!("Finished: {timestamp}, done"),
        JobOutcome::Failed(reason) => format!("Finished: {timestamp}, failed: {reason}"),
        JobOutcome::Cancelled => format!("Finished: {timestamp}, cancelled"),
    }
}

pub fn device_frames_lines(
    device_frames: u64,
    frames_encoded: u64,
    accelerator_requested: bool,
) -> Vec<String> {
    let mut lines = vec![format!(
        "[ENCODE] Frames on the device: {device_frames} of {frames_encoded}"
    )];
    if device_frames == 0 && accelerator_requested {
        lines.push(
            "[ENCODE] WARNING: the GPU was requested and no frame ran on the device".to_string(),
        );
    }
    lines
}

pub struct JobLog {
    file: File,
    _running_job_log: RunningJobLog,
}

impl JobLog {
    pub fn create(output: &Path) -> Result<Self, String> {
        let path = job_log_path(output)?;
        std::fs::create_dir_all(output)
            .map_err(|e| format!("Cannot create the output folder {}: {e}", output.display()))?;
        let file = File::create(&path)
            .map_err(|e| format!("Cannot create the job log {}: {e}", path.display()))?;
        let running_job_log = RunningJobLog::open(&file).map_err(|e| {
            format!(
                "Cannot open the job log {} for crash lines: {e}",
                path.display()
            )
        })?;
        Ok(Self {
            file,
            _running_job_log: running_job_log,
        })
    }

    pub fn line(&self, text: &str) {
        let _ = writeln!(&self.file, "{text}");
    }
}

static RUNNING_JOB_LOG: Mutex<Option<File>> = Mutex::new(None);

#[cfg(unix)]
const NO_JOB_LOG_FD: i32 = -1;
// the signal handler cannot take the lock on RUNNING_JOB_LOG
#[cfg(unix)]
static RUNNING_JOB_LOG_FD: std::sync::atomic::AtomicI32 =
    std::sync::atomic::AtomicI32::new(NO_JOB_LOG_FD);

#[cfg(unix)]
const CRASH_SIGNALS: [(libc::c_int, &[u8]); 4] = [
    (libc::SIGSEGV, b"[CRASH] signal SIGSEGV\n"),
    (libc::SIGBUS, b"[CRASH] signal SIGBUS\n"),
    (libc::SIGABRT, b"[CRASH] signal SIGABRT\n"),
    (libc::SIGILL, b"[CRASH] signal SIGILL\n"),
];

pub struct RunningJobLog;

impl RunningJobLog {
    pub fn open(log: &File) -> std::io::Result<Self> {
        let crash_copy = log.try_clone()?;
        #[cfg(unix)]
        RUNNING_JOB_LOG_FD.store(
            std::os::fd::AsRawFd::as_raw_fd(&crash_copy),
            std::sync::atomic::Ordering::SeqCst,
        );
        *running_job_log() = Some(crash_copy);
        Ok(Self)
    }
}

impl Drop for RunningJobLog {
    fn drop(&mut self) {
        // whoever catches the panic writes the Finished line
        if std::thread::panicking() {
            return;
        }
        close_running_job_log();
    }
}

fn running_job_log() -> std::sync::MutexGuard<'static, Option<File>> {
    RUNNING_JOB_LOG
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn close_running_job_log() -> Option<File> {
    #[cfg(unix)]
    RUNNING_JOB_LOG_FD.store(NO_JOB_LOG_FD, std::sync::atomic::Ordering::SeqCst);
    running_job_log().take()
}

// for an exit that has no JobLog in reach
pub fn finish_running_job(outcome: &JobOutcome) {
    if let Some(log) = close_running_job_log() {
        let _ = writeln!(&log, "{}", finished_line(&log_timestamp(), outcome));
    }
}

pub fn write_panic_line(info: &std::panic::PanicHookInfo) {
    if let Some(mut log) = running_job_log().as_ref() {
        let _ = writeln!(log, "{}", panic_line(info));
    }
}

fn panic_line(info: &std::panic::PanicHookInfo) -> String {
    let location = info.location().map_or_else(
        || "an unknown location".to_string(),
        |location| format!("{}:{}", location.file(), location.line()),
    );
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("a payload that is not text");
    format!("[CRASH] panicked at {location}: {message}")
}

// rust's own SIGSEGV handler prints the stack overflow message
#[cfg(unix)]
static PREVIOUS_CRASH_ACTIONS: std::sync::OnceLock<[libc::sigaction; CRASH_SIGNALS.len()]> =
    std::sync::OnceLock::new();

#[cfg(unix)]
pub fn install_crash_signal_handlers() {
    PREVIOUS_CRASH_ACTIONS
        .get_or_init(|| CRASH_SIGNALS.map(|(signal, _)| install_crash_handler(signal)));
}

#[cfg(unix)]
fn install_crash_handler(signal: libc::c_int) -> libc::sigaction {
    let handler: extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) =
        write_crash_line_and_reraise;
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handler as libc::sighandler_t;
        // a stack overflow leaves no room on the thread's own stack
        action.sa_flags = libc::SA_ONSTACK | libc::SA_SIGINFO;
        libc::sigemptyset(&mut action.sa_mask);
        let mut previous: libc::sigaction = std::mem::zeroed();
        libc::sigaction(signal, &action, &mut previous);
        previous
    }
}

// async-signal-safe calls only: no allocation, locks or formatting
#[cfg(unix)]
extern "C" fn write_crash_line_and_reraise(
    signal: libc::c_int,
    info: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    let fd = RUNNING_JOB_LOG_FD.load(std::sync::atomic::Ordering::SeqCst);
    let crash_signal_index = CRASH_SIGNALS
        .iter()
        .position(|(crash_signal, _)| *crash_signal == signal);
    unsafe {
        if let Some(index) = crash_signal_index.filter(|_| fd != NO_JOB_LOG_FD) {
            let line = CRASH_SIGNALS[index].1;
            libc::write(fd, line.as_ptr().cast(), line.len());
        }
        let previous = crash_signal_index
            .zip(PREVIOUS_CRASH_ACTIONS.get())
            .map(|(index, previous_actions)| &previous_actions[index]);
        if let Some(previous) = previous {
            call_previous_crash_handler(previous, signal, info, context);
        }
        // rust's handler returns unless the fault was a stack overflow
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}

#[cfg(unix)]
unsafe fn call_previous_crash_handler(
    previous: &libc::sigaction,
    signal: libc::c_int,
    info: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    let handler = previous.sa_sigaction;
    if handler == libc::SIG_DFL || handler == libc::SIG_IGN {
        return;
    }
    unsafe {
        if previous.sa_flags & libc::SA_SIGINFO != 0 {
            let previous_handler: extern "C" fn(
                libc::c_int,
                *mut libc::siginfo_t,
                *mut libc::c_void,
            ) = std::mem::transmute(handler);
            previous_handler(signal, info, context);
        } else {
            let previous_handler: extern "C" fn(libc::c_int) = std::mem::transmute(handler);
            previous_handler(signal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // tests that open a JobLog take turns on the one running job log
    static RUNNING_JOB_LOG_TURN: Mutex<()> = Mutex::new(());

    fn take_turn() -> std::sync::MutexGuard<'static, ()> {
        RUNNING_JOB_LOG_TURN
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    #[test]
    fn the_status_names_what_was_asked_for_and_why_it_failed() {
        assert_eq!(accelerator_status(false, false, None), "off");
        assert_eq!(accelerator_status(true, true, None), "requested, active");
        assert_eq!(
            accelerator_status(true, false, Some("the plugin did not initialise")),
            "requested, inactive: the plugin did not initialise"
        );
        assert_eq!(accelerator_status(true, false, None), "requested, inactive");
    }

    #[test]
    fn the_thread_count_says_when_it_was_chosen_automatically() {
        assert_eq!(encode_threads_status(2), "2");
        let automatic = encode_threads_status(crate::preferences::AUTOMATIC_ENCODE_THREADS);
        assert!(automatic.ends_with(" (automatic)"), "{automatic}");
        assert_ne!(automatic, "0 (automatic)");
    }

    fn probed_source() -> postkit::probe::VideoInfo {
        postkit::probe::VideoInfo {
            codec_name: "prores".to_string(),
            width: 3840,
            height: 2160,
            fps_num: 24000,
            fps_den: 1001,
            has_audio: true,
            total_frames: 1442,
            pix_fmt: "yuv422p10le".to_string(),
            color_space: "bt709".to_string(),
            color_range: "tv".to_string(),
            color_transfer: "bt709".to_string(),
            color_primaries: "bt709".to_string(),
            bit_rate: Some(707_000_000),
        }
    }

    #[test]
    fn the_source_line_carries_everything_the_probe_read() {
        assert_eq!(
            source_line(&probed_source()),
            "Source: prores 3840x2160 24000/1001 1442 frames, yuv422p10le, colour bt709 tv \
             bt709 bt709, 707.0 Mbit/s"
        );
    }

    #[test]
    fn a_source_with_no_bit_rate_says_unknown() {
        let source = postkit::probe::VideoInfo {
            bit_rate: None,
            ..probed_source()
        };
        assert!(
            source_line(&source).ends_with(", unknown Mbit/s"),
            "{}",
            source_line(&source)
        );
    }

    #[test]
    fn the_settings_line_is_the_whole_config_as_one_json_object() {
        #[derive(serde::Serialize)]
        struct Settings {
            title: &'static str,
            bandwidth: u32,
        }
        let line = settings_line(&Settings {
            title: "Sunrise",
            bandwidth: 230,
        })
        .unwrap();
        assert_eq!(line, r#"Settings: {"title":"Sunrise","bandwidth":230}"#);
    }

    #[test]
    fn the_finished_line_names_how_the_job_ended() {
        let timestamp = "2026-09-30 12:00:00";
        assert_eq!(
            finished_line(timestamp, &JobOutcome::Done),
            "Finished: 2026-09-30 12:00:00, done"
        );
        assert_eq!(
            finished_line(timestamp, &JobOutcome::Failed("ffmpeg exited with 1")),
            "Finished: 2026-09-30 12:00:00, failed: ffmpeg exited with 1"
        );
        assert_eq!(
            finished_line(timestamp, &JobOutcome::Cancelled),
            "Finished: 2026-09-30 12:00:00, cancelled"
        );
    }

    #[test]
    fn the_device_frames_line_warns_when_the_gpu_ran_nothing() {
        assert_eq!(
            device_frames_lines(90, 90, true),
            ["[ENCODE] Frames on the device: 90 of 90"]
        );
        assert_eq!(
            device_frames_lines(0, 90, true),
            [
                "[ENCODE] Frames on the device: 0 of 90",
                "[ENCODE] WARNING: the GPU was requested and no frame ran on the device",
            ]
        );
        assert_eq!(
            device_frames_lines(0, 90, false),
            ["[ENCODE] Frames on the device: 0 of 90"]
        );
    }

    #[test]
    fn a_job_finished_from_outside_its_log_ends_the_log_once() {
        let _turn = take_turn();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("dcp");
        let log = JobLog::create(&output).unwrap();
        log.line("[ENCODE] frame=1/2");
        finish_running_job(&JobOutcome::Failed("Encode failed: ffmpeg exited"));
        finish_running_job(&JobOutcome::Done);
        drop(log);
        let written = std::fs::read_to_string(directory.path().join("dcp.log")).unwrap();
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(lines.len(), 2, "{written}");
        assert!(
            lines[1].starts_with("Finished: ")
                && lines[1].ends_with(", failed: Encode failed: ffmpeg exited"),
            "{written}"
        );
    }

    #[test]
    fn a_job_log_dropped_by_a_panic_stays_open_for_the_finished_line() {
        let _turn = take_turn();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("dcp");
        let panicked = std::panic::catch_unwind(|| {
            let _log = JobLog::create(&output).unwrap();
            panic!("the encoder gave up");
        });
        assert!(panicked.is_err());
        finish_running_job(&JobOutcome::Failed("panicked"));
        let written = std::fs::read_to_string(directory.path().join("dcp.log")).unwrap();
        assert!(
            written.starts_with("Finished: ") && written.ends_with(", failed: panicked\n"),
            "{written}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_fatal_signal_writes_its_line_and_still_ends_the_process_with_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("dcp.log");
        let log = File::create(&path).unwrap();
        let fd = std::os::fd::AsRawFd::as_raw_fd(&log);

        let child = unsafe { libc::fork() };
        assert!(child >= 0, "fork failed");
        if child == 0 {
            // no core file from the child
            unsafe {
                let no_core = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                libc::setrlimit(libc::RLIMIT_CORE, &no_core);
                RUNNING_JOB_LOG_FD.store(fd, std::sync::atomic::Ordering::SeqCst);
                install_crash_signal_handlers();
                libc::raise(libc::SIGSEGV);
                libc::_exit(0);
            }
        }
        let mut status = 0;
        unsafe { libc::waitpid(child, &mut status, 0) };
        assert!(
            libc::WIFSIGNALED(status) && libc::WTERMSIG(status) == libc::SIGSEGV,
            "the child ended with status {status} instead of SIGSEGV"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[CRASH] signal SIGSEGV\n"
        );
    }

    #[test]
    fn the_log_is_named_after_the_package_folder_beside_it() {
        assert_eq!(
            job_log_path(Path::new("/x/my_dcp")).unwrap(),
            Path::new("/x/my_dcp.log")
        );
        assert_eq!(
            job_log_path(Path::new("/x/my.dcp/")).unwrap(),
            Path::new("/x/my.dcp.log")
        );
        assert_eq!(
            job_log_path(Path::new("my_dcp")).unwrap(),
            Path::new("my_dcp.log")
        );
    }

    #[test]
    fn an_output_with_no_folder_name_is_refused_naming_it() {
        let error = job_log_path(Path::new("/x/..")).unwrap_err();
        assert!(error.contains("/x/.."), "the error names the path: {error}");
    }

    #[test]
    fn the_log_is_created_beside_an_output_folder_that_does_not_exist_yet() {
        let _turn = take_turn();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("dcp");
        let log = JobLog::create(&output).expect("the log has to be created");
        log.line("Accelerator: off");
        assert!(output.is_dir(), "the package folder is created too");
        assert_eq!(
            std::fs::read_dir(&output).unwrap().count(),
            0,
            "nothing is written inside the package folder"
        );
        assert_eq!(
            std::fs::read_to_string(directory.path().join("dcp.log")).unwrap(),
            "Accelerator: off\n"
        );
    }
}
