//! Delegates to [`postkit::loudness`], which meters a WAV or a PCM MXF.

pub use postkit::loudness::{
    AdjustError, DEFAULT_TRUE_PEAK_CEILING_DBTP, GainPlan, LoudnessResult, LoudnessTarget,
    adjust_loudness, loudness_target_spec, measure_loudness, measure_true_peak_dbtp,
    parse_loudness_target,
};
