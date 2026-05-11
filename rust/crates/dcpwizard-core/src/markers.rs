use serde::{Deserialize, Serialize};

/// DCP marker types per SMPTE 429-10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Marker {
    /// First Frame of Composition
    Ffoc,
    /// Last Frame of Composition
    Lfoc,
    /// First Frame of Title Credits
    Fftc,
    /// Last Frame of Title Credits
    Lftc,
    /// First Frame of Intermission
    Ffoi,
    /// Last Frame of Intermission
    Lfoi,
    /// First Frame of End Credits
    Ffec,
    /// Last Frame of End Credits
    Lfec,
    /// First Frame of Moving Credits
    Ffmc,
    /// Last Frame of Moving Credits
    Lfmc,
}

impl Marker {
    pub fn label(&self) -> &'static str {
        match self {
            Marker::Ffoc => "FFOC",
            Marker::Lfoc => "LFOC",
            Marker::Fftc => "FFTC",
            Marker::Lftc => "LFTC",
            Marker::Ffoi => "FFOI",
            Marker::Lfoi => "LFOI",
            Marker::Ffec => "FFEC",
            Marker::Lfec => "LFEC",
            Marker::Ffmc => "FFMC",
            Marker::Lfmc => "LFMC",
        }
    }

    pub fn scope(&self) -> &'static str {
        "http://www.smpte-ra.org/schemas/429-10/2008/Main-Stereo-Picture-CPL#"
    }
}

/// A marker placed at a specific frame offset within a reel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerEntry {
    pub marker: Marker,
    pub frame: u64,
}

impl MarkerEntry {
    pub fn new(marker: Marker, frame: u64) -> Self {
        Self { marker, frame }
    }

    /// Generate the XML element for this marker entry.
    pub fn to_xml(&self) -> String {
        format!(
            "<Marker>\n  <Label Scope=\"{}\">{}</Label>\n  <Offset>{}</Offset>\n</Marker>",
            self.marker.scope(),
            self.marker.label(),
            self.frame
        )
    }
}

/// Generate default markers for a composition of the given frame count.
/// Places FFOC at frame 0 and LFOC at the last frame.
pub fn default_markers(total_frames: u64) -> Vec<MarkerEntry> {
    if total_frames == 0 {
        return Vec::new();
    }
    vec![
        MarkerEntry::new(Marker::Ffoc, 0),
        MarkerEntry::new(Marker::Lfoc, total_frames.saturating_sub(1)),
    ]
}

/// Generate the XML MarkerList block for a set of markers.
pub fn markers_to_xml(markers: &[MarkerEntry]) -> String {
    let mut xml = String::new();
    xml.push_str("<MarkerList>\n");
    for entry in markers {
        xml.push_str("  ");
        xml.push_str(&entry.to_xml());
        xml.push('\n');
    }
    xml.push_str("</MarkerList>");
    xml
}
