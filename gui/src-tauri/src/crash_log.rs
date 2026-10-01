pub use dcpwizard_core::job_log::RunningJobLog;

// tests that open a RunningJobLog take turns on the one static
#[cfg(test)]
pub static RUNNING_JOB_LOG_TEST_TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn install() {
    install_panic_hook();
    #[cfg(unix)]
    dcpwizard_core::job_log::install_crash_signal_handlers();
    // TODO: windows writes no crash line for an access violation, it needs SetUnhandledExceptionFilter
}

fn install_panic_hook() {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        dcpwizard_core::job_log::write_panic_line(info);
        previous_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_is_written_to_the_running_job_log_until_the_log_closes() {
        let _turn = RUNNING_JOB_LOG_TEST_TURN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        install_panic_hook();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("dcp.log");
        let log = std::fs::File::create(&path).unwrap();

        let running_job_log = RunningJobLog::open(&log).unwrap();
        let panicked = std::panic::catch_unwind(|| panic!("the encoder gave up"));
        assert!(panicked.is_err());
        drop(running_job_log);
        let after_the_panic = std::fs::read_to_string(&path).unwrap();
        assert!(
            after_the_panic.starts_with("[CRASH] panicked at ")
                && after_the_panic.ends_with(": the encoder gave up\n"),
            "{after_the_panic}"
        );

        let panicked = std::panic::catch_unwind(|| panic!("a later job's panic"));
        assert!(panicked.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), after_the_panic);
    }
}
