//! Subtitle / text burn-in via ffmpeg drawtext.
//!
//! Delegates to [`postkit::burnin`], which also handles subtitle-file burn-in
//! (SRT/ASS/SMPTE) in addition to plain text overlays.

pub use postkit::burnin::{BurninOptions, burnin};
