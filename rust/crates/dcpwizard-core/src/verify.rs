use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Result of a DCP verification pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifyResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

/// Verify a DCP by delegating to dcpdoctor-core.
pub fn verify_dcp(dcp_dir: &Path) -> VerifyResult {
    if !dcp_dir.exists() {
        return VerifyResult {
            valid: false,
            errors: vec![format!("DCP directory not found: {}", dcp_dir.display())],
            ..Default::default()
        };
    }

    let assetmap = find_assetmap(dcp_dir);
    if assetmap.is_none() {
        return VerifyResult {
            valid: false,
            errors: vec!["No ASSETMAP found in DCP directory".into()],
            ..Default::default()
        };
    }

    let opts = dcpdoctor_core::VerifyOptions::standard();
    let report = dcpdoctor_core::verify(dcp_dir, &opts);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    for note in &report.notes {
        match note.severity {
            dcpdoctor_core::Severity::Error => errors.push(note.to_string()),
            dcpdoctor_core::Severity::Warning => warnings.push(note.to_string()),
            dcpdoctor_core::Severity::Info => info.push(note.to_string()),
        }
    }

    VerifyResult {
        valid: report.ok(),
        errors,
        warnings,
        info,
    }
}

fn find_assetmap(dir: &Path) -> Option<PathBuf> {
    for name in &["ASSETMAP", "ASSETMAP.xml"] {
        let path = dir.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}
