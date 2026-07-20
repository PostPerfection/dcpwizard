//! Content version and delivery tracking (SQLite-backed).
//!
//! Delegates to [`postkit::version_tracker`] for recording deliveries,
//! querying version history, and exporting delivery records.

pub use postkit::version_tracker::{DeliveryRecord, VersionQuery, VersionTracker};

/// Current UTC time as an RFC 3339 timestamp, for delivery records.
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_tracker_open() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tracker.db");
        let mut tracker = VersionTracker::new();
        assert!(tracker.open(&db));
    }

    #[test]
    fn test_record_and_query() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tracker.db");
        let mut tracker = VersionTracker::new();
        tracker.open(&db);

        let record = DeliveryRecord {
            package_uuid: "dcp-uuid-1".into(),
            title: "Test DCP".into(),
            version: "OV".into(),
            destination: "Cinema Chain A".into(),
            delivery_method: "hard_drive".into(),
            timestamp: "2024-06-01T12:00:00Z".into(),
            verified: true,
        };
        assert!(tracker.record(&record));

        let results = tracker.versions_of("dcp-uuid-1");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Test DCP");
    }

    #[test]
    fn test_export_json() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tracker.db");
        let mut tracker = VersionTracker::new();
        tracker.open(&db);

        tracker.record(&DeliveryRecord {
            package_uuid: "u1".into(),
            title: "Film".into(),
            ..Default::default()
        });

        let json_path = dir.path().join("out.json");
        assert!(tracker.export_json(&json_path));
        let content = std::fs::read_to_string(&json_path).unwrap();
        assert!(content.contains("Film"));
    }
}
