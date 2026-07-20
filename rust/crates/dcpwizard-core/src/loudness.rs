//! EBU R128 loudness measurement.
//!
//! Delegates to [`postkit::loudness`]. postkit parses ffmpeg's `loudnorm` JSON
//! output (integrated, LRA, true peak) rather than the older `ebur128` summary.

pub use postkit::loudness::{LoudnessResult, measure_loudness};
