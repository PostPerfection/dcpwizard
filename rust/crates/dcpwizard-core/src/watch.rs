//! DCP watch-folder polling.
//!
//! [`postkit::watch`] is a generic `notify`-based file watcher emitting raw
//! fs events. This is higher-level domain logic: it polls for directories that
//! contain an ASSETMAP (i.e. complete DCPs) and fires once per new package.

use std::path::{Path, PathBuf};

/// Watch a directory for new DCP deliveries by polling for directories
/// containing an ASSETMAP file.
///
/// Calls `on_dcp_found` for each newly detected DCP. Polls at the given
/// interval. Runs until `should_stop` returns true.
pub fn watch_directory<F>(
    watch_dir: &Path,
    interval: std::time::Duration,
    should_stop: &dyn Fn() -> bool,
    on_dcp_found: F,
) where
    F: Fn(&Path),
{
    if !watch_dir.exists() {
        tracing::error!("Watch directory does not exist: {}", watch_dir.display());
        return;
    }

    tracing::info!(
        "Watching {} for new DCPs (polling every {:?})",
        watch_dir.display(),
        interval
    );

    let mut known_dcps: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    // Initial scan to populate known set
    for entry in scan_for_dcps(watch_dir) {
        known_dcps.insert(entry);
    }

    loop {
        if should_stop() {
            tracing::info!("Watch directory stopping");
            break;
        }

        std::thread::sleep(interval);

        let current_dcps = scan_for_dcps(watch_dir);

        for dcp_dir in &current_dcps {
            if !known_dcps.contains(dcp_dir) {
                tracing::info!("New DCP detected: {}", dcp_dir.display());
                on_dcp_found(dcp_dir);
                known_dcps.insert(dcp_dir.clone());
            }
        }
    }
}

fn scan_for_dcps(dir: &Path) -> Vec<PathBuf> {
    let mut dcps = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return dcps,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let has_assetmap = path.join("ASSETMAP").exists() || path.join("ASSETMAP.xml").exists();
            if has_assetmap {
                dcps.push(path);
            }
        }
    }

    dcps
}
