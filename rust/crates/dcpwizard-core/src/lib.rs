pub mod accessibility;
pub mod assetmap;
pub mod atmos;
pub mod audio;
pub mod burnin;
pub mod colour;
pub mod conform;
pub mod copy_drive;
pub mod cpl;
pub mod cpl_annotation;
pub mod dashboard;
pub mod dcdm;
pub mod dcp;
pub mod dcp_diff;
pub mod dolby_vision;
pub mod dtsx;
pub mod edl_import;
pub mod encode;
pub mod encrypt;
pub mod export;
pub mod geometry;
pub mod grok;
pub mod hash;
pub mod hfr;
pub mod import;
pub mod info;
pub mod ingest;
pub mod j2k_transcode;
pub mod job_queue;
pub mod kdm;
pub mod kdm_advanced;
pub mod loudness;
pub mod markers;
pub mod mca;
pub mod multi_cpl;
pub mod mxf_wrap;
pub mod otioz_import;
pub mod pkl;
pub mod plugin;
pub mod preferences;
pub mod preview;
pub mod probe;
pub mod profiles;
pub mod prores;
pub mod qc;
pub mod reel;
pub mod report;
pub mod rest_api;
pub mod shell_completion;
pub mod stereo3d;
pub mod subtitle;
pub mod subtitle_retime;
pub mod trailer;
pub mod transcode;
pub mod verify;
pub mod version_tracker;
pub mod vf;
pub mod watch;
pub mod watermark;
pub mod webhook;

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
