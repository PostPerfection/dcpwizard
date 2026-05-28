//! OV/VF version management dashboard and distribution tracking.
//!
//! Delegates to [`postkit::dashboard`] for version registration, territory
//! tracking, status management, and distribution matrix export.

pub use postkit::dashboard::{
    DashboardOptions, TerritoryInfo, VersionEntry, export_distribution_matrix, init_database,
    list_territories, list_versions, register_version, update_status,
};

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
