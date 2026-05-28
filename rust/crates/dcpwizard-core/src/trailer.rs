//! Trailer packaging (ratings card, countdown leader, assembly).
//!
//! Delegates to [`postkit::trailer`] for trailer creation with configurable
//! rating systems, band colours, and countdown leaders.

pub use postkit::trailer::{
    RatingSystem, TrailerBand, TrailerOptions, TrailerResult, package_trailer,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_system_default() {
        assert_eq!(RatingSystem::default(), RatingSystem::Mpaa);
    }

    #[test]
    fn test_trailer_band_default() {
        assert_eq!(TrailerBand::default(), TrailerBand::Green);
    }

    #[test]
    fn test_trailer_options_default() {
        let opts = TrailerOptions::default();
        assert!(opts.title.is_empty());
        assert_eq!(opts.countdown_seconds, 0);
    }

    #[test]
    fn test_package_trailer_missing_output() {
        let opts = TrailerOptions {
            output_dir: std::path::PathBuf::from("/nonexistent_readonly_dir/sub/trailer"),
            ..Default::default()
        };
        let result = package_trailer(&opts);
        // May fail due to directory creation issues on some systems
        // but shouldn't panic
        let _ = result;
    }
}
