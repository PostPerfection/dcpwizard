use std::io::Write;
use std::path::{Path, PathBuf};

const LOG_EXTENSION: &str = "log";

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

pub struct JobLog(std::fs::File);

impl JobLog {
    pub fn create(output: &Path) -> Result<Self, String> {
        let path = job_log_path(output)?;
        std::fs::create_dir_all(output)
            .map_err(|e| format!("Cannot create the output folder {}: {e}", output.display()))?;
        std::fs::File::create(&path)
            .map(Self)
            .map_err(|e| format!("Cannot create the job log {}: {e}", path.display()))
    }

    pub fn line(&mut self, text: &str) {
        let _ = writeln!(self.0, "{text}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("dcp");
        let mut log = JobLog::create(&output).expect("the log has to be created");
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
