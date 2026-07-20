//! OV/VF version management dashboard and distribution tracking.
//!
//! Delegates to [`postkit::dashboard`] for version registration, territory
//! tracking, status management, and distribution matrix export.

pub use postkit::dashboard::{
    DashboardOptions, TerritoryInfo, VersionEntry, export_distribution_matrix, init_database,
    list_territories, list_versions, register_version, serve_dashboard, update_status,
};

use std::path::PathBuf;

/// Path of the shared version database that postkit's default-path dashboard
/// functions (register/list/status/matrix) operate on. Mirrors postkit's
/// `default_db_path` (which is private) so the CLI can create the schema before
/// use. Keep in sync with postkit if it ever changes.
pub fn default_db_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("postkit");
    let _ = std::fs::create_dir_all(&config_dir);
    config_dir.join("versions.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("versions.db");
        assert_eq!(init_database(&db), 0);
        // Verify the file was created
        assert!(db.exists());
    }

    #[test]
    fn test_version_entry_default() {
        let entry = VersionEntry::default();
        assert!(entry.uuid.is_empty());
        assert!(entry.title.is_empty());
        assert!(entry.territory.is_empty());
        assert_eq!(entry.kdm_recipients.len(), 0);
    }

    #[test]
    fn test_dashboard_options_default() {
        let opts = DashboardOptions::default();
        assert_eq!(opts.http_port, 9090);
        assert_eq!(opts.bind_address, "127.0.0.1");
    }
}
