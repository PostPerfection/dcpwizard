pub mod aaf_import;
pub mod accessibility;
pub mod assemble;
pub mod assetmap;
pub mod audio_adjust;
pub mod audio_fallback;
pub mod audio_map;
pub mod audio_route;
pub mod burnin;
pub mod cert_fetch;
pub mod certificate;
pub mod cinema;
pub mod combine;
pub mod conform;
pub mod copy_drive;
pub mod cpl;
pub mod cpl_annotation;
pub mod dashboard;
pub mod dcdm;
pub mod dcp;
pub mod decrypt;
pub mod disk;
pub mod dolby_vision;
pub mod edit;
pub mod edl_import;
pub mod email;
pub mod encode;
pub mod encode_qol;
pub mod encrypt;
pub mod export;
pub mod flm;
pub mod grok;
pub mod hash;
pub mod hdr;
pub mod hfr;
pub mod hints;
pub mod import;
pub mod info;
pub mod ingest_package;
pub mod intermediates;
pub mod isdcf_name;
pub mod isdcf_title;
pub mod j2k_transcode;
pub mod job_queue;
pub mod kdm;
pub mod kdm_log;
pub mod kdm_template;
pub mod library;
pub mod library_reel;
pub mod loudness;
pub mod markers;
pub mod mca;
pub mod multi_cpl;
pub mod mxf_wrap;
pub mod otioz_import;
pub mod overlapped_picture;
pub mod package_signature;
pub mod pad;
pub mod pkl;
pub mod preferences;
pub mod preflight;
pub mod preview;
pub mod probe;
pub mod profiles;
pub mod qc;
pub mod reel;
pub mod report;
pub mod rest_api;
pub mod sign_language;
pub mod source_picture;
pub mod store;
pub mod subtitle;
pub mod subtitle_edit;
pub mod subtitle_extract;
pub mod subtitle_preview;
pub mod subtitle_retime;
#[cfg(test)]
pub(crate) mod test_wav;
pub mod tms_upload;
pub mod trailer;
pub mod transcode;
pub mod trim;
pub mod verify;
pub mod version_tracker;
pub mod versions;
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

    /// The family a coded raster belongs to: anything past the 2K frame is 4K.
    pub fn for_raster(width: u32, height: u32) -> Resolution {
        if width > Resolution::TwoK.width() || height > Resolution::TwoK.height() {
            Resolution::FourK
        } else {
            Resolution::TwoK
        }
    }
}

/// DCP content type (SMPTE 429-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ContentType {
    /// Feature
    #[default]
    Feature,
    /// Short
    Short,
    /// Trailer
    Trailer,
    /// Test
    Test,
    /// Transitional (pre-show)
    Transitional,
    /// Rating
    Rating,
    /// Teaser
    Teaser,
    /// Policy
    Policy,
    /// Public service announcement
    PublicServiceAnnouncement,
    /// Advertisement
    Advertisement,
    /// Episode (episodic content)
    Episode,
}

const CONTENT_TYPE_ABBREVIATIONS: [(&str, ContentType); 11] = [
    ("FTR", ContentType::Feature),
    ("SHR", ContentType::Short),
    ("TLR", ContentType::Trailer),
    ("TST", ContentType::Test),
    ("XSN", ContentType::Transitional),
    ("RTG", ContentType::Rating),
    ("TSR", ContentType::Teaser),
    ("POL", ContentType::Policy),
    ("PSA", ContentType::PublicServiceAnnouncement),
    ("ADV", ContentType::Advertisement),
    ("EPS", ContentType::Episode),
];

impl ContentType {
    /// Parse from common abbreviation string.
    pub fn from_abbrev(s: &str) -> Option<Self> {
        CONTENT_TYPE_ABBREVIATIONS
            .iter()
            .find(|(abbreviation, _)| abbreviation.eq_ignore_ascii_case(s))
            .map(|(_, content_type)| *content_type)
    }

    pub fn parse_abbrev(s: &str) -> Result<Self, String> {
        Self::from_abbrev(s).ok_or_else(|| {
            let abbreviations: Vec<&str> = CONTENT_TYPE_ABBREVIATIONS
                .iter()
                .map(|(abbreviation, _)| *abbreviation)
                .collect();
            format!(
                "unknown content type '{s}' (use {})",
                abbreviations.join(", ")
            )
        })
    }

    /// SMPTE content kind string for CPL.
    pub fn as_cpl_kind(&self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Short => "short",
            Self::Trailer => "trailer",
            Self::Test => "test",
            Self::Transitional => "transitional",
            Self::Rating => "rating",
            Self::Teaser => "teaser",
            Self::Policy => "policy",
            Self::PublicServiceAnnouncement => "psa",
            Self::Advertisement => "advertisement",
            Self::Episode => "episode",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_from_abbrev() {
        assert_eq!(ContentType::from_abbrev("FTR"), Some(ContentType::Feature));
        assert_eq!(ContentType::from_abbrev("SHR"), Some(ContentType::Short));
        assert_eq!(ContentType::from_abbrev("TLR"), Some(ContentType::Trailer));
        assert_eq!(ContentType::from_abbrev("TST"), Some(ContentType::Test));
        assert_eq!(
            ContentType::from_abbrev("XSN"),
            Some(ContentType::Transitional)
        );
        assert_eq!(ContentType::from_abbrev("RTG"), Some(ContentType::Rating));
        assert_eq!(ContentType::from_abbrev("TSR"), Some(ContentType::Teaser));
        assert_eq!(ContentType::from_abbrev("POL"), Some(ContentType::Policy));
        assert_eq!(
            ContentType::from_abbrev("PSA"),
            Some(ContentType::PublicServiceAnnouncement)
        );
        assert_eq!(
            ContentType::from_abbrev("ADV"),
            Some(ContentType::Advertisement)
        );
    }

    #[test]
    fn test_content_type_case_insensitive() {
        assert_eq!(ContentType::from_abbrev("ftr"), Some(ContentType::Feature));
        assert_eq!(ContentType::from_abbrev("Tlr"), Some(ContentType::Trailer));
    }

    #[test]
    fn test_content_type_invalid() {
        assert_eq!(ContentType::from_abbrev("XYZ"), None);
        assert_eq!(ContentType::from_abbrev(""), None);
    }

    #[test]
    fn an_unknown_content_type_is_refused_with_every_valid_abbreviation() {
        assert_eq!(ContentType::parse_abbrev("tlr"), Ok(ContentType::Trailer));
        let refusal = ContentType::parse_abbrev("TRL").unwrap_err();
        assert!(refusal.contains("'TRL'"), "{refusal}");
        for (abbreviation, _) in CONTENT_TYPE_ABBREVIATIONS {
            assert!(refusal.contains(abbreviation), "{refusal}");
        }
    }

    #[test]
    fn test_content_type_cpl_kind() {
        assert_eq!(ContentType::Feature.as_cpl_kind(), "feature");
        assert_eq!(ContentType::Trailer.as_cpl_kind(), "trailer");
        assert_eq!(ContentType::PublicServiceAnnouncement.as_cpl_kind(), "psa");
    }

    #[test]
    fn test_resolution_dimensions() {
        assert_eq!(Resolution::TwoK.width(), 2048);
        assert_eq!(Resolution::TwoK.height(), 1080);
        assert_eq!(Resolution::FourK.width(), 4096);
        assert_eq!(Resolution::FourK.height(), 2160);
    }
}
