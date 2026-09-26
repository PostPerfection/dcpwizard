//! TIFF loading for the in-process JPEG 2000 encoder.

pub use postkit::grok::{TiffFrame, load_tiff};

#[cfg(target_os = "linux")]
const PACKAGED_GPU_PLUGIN_FILE_NAME: &str = "libgrokj2k_plugin.so";

#[cfg(target_os = "linux")]
fn packaged_gpu_plugin_directory(executable: &std::path::Path) -> Option<std::path::PathBuf> {
    Some(executable.parent()?.join("../lib/dcpwizard"))
}

pub fn set_packaged_gpu_plugin_path() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("GRK_PLUGIN_PATH").is_some() {
            return;
        }
        let Ok(executable) = std::env::current_exe() else {
            return;
        };
        let Some(plugin_directory) = packaged_gpu_plugin_directory(&executable) else {
            return;
        };
        if !plugin_directory
            .join(PACKAGED_GPU_PLUGIN_FILE_NAME)
            .is_file()
        {
            return;
        }
        unsafe {
            std::env::set_var("GRK_PLUGIN_PATH", plugin_directory);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_tiff_missing_file() {
        let result = load_tiff(std::path::Path::new("/nonexistent.tif"));
        assert!(result.is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn packaged_gpu_plugin_is_beside_installed_binaries() {
        let directory = packaged_gpu_plugin_directory(std::path::Path::new("/usr/bin/dcpwizard"));
        assert_eq!(
            directory,
            Some(std::path::PathBuf::from("/usr/bin/../lib/dcpwizard"))
        );
    }
}
