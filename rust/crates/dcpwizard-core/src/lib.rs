pub mod assetmap;
pub mod atmos;
pub mod audio;
pub mod burnin;
pub mod colour;
pub mod copy_drive;
pub mod cpl;
pub mod dcp;
pub mod dtsx;
pub mod encode;
pub mod encrypt;
pub mod export;
pub mod geometry;
pub mod hash;
pub mod hfr;
pub mod import;
pub mod info;
pub mod j2k_transcode;
pub mod job_queue;
pub mod kdm;
pub mod kdm_advanced;
pub mod loudness;
pub mod markers;
pub mod multi_cpl;
pub mod mxf_wrap;
pub mod pkl;
pub mod preferences;
pub mod profiles;
pub mod qc;
pub mod reel;
pub mod report;
pub mod rest_api;
pub mod shell_completion;
pub mod stereo3d;
pub mod subtitle;
pub mod transcode;
pub mod verify;
pub mod vf;
pub mod watch;

use serde::{Deserialize, Serialize};

/// DCP standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Standard {
    #[default]
    Smpte,
    Interop,
}

/// Resolution preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Resolution {
    #[default]
    TwoK,
    FourK,
}

impl Resolution {
    pub fn width(&self) -> u32 {
        match self {
            Resolution::TwoK => 2048,
            Resolution::FourK => 4096,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Resolution::TwoK => 1080,
            Resolution::FourK => 2160,
        }
    }
}
