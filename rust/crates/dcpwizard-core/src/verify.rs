use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub use dcpdoctor_core::{VerifyProgress, VerifyStage};

/// Result of a DCP verification pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifyResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

/// CLI options for verification.
#[derive(Debug, Clone, Default)]
pub struct VerifyCliOptions {
    pub skip_hash_check: bool,
    pub skip_picture_check: bool,
    pub skip_bitrate_measurement: bool,
    pub strict: bool,
    /// Read every frame's codestream, which is what the QC report's forensics
    /// line is measured from.
    pub scan_every_frame: bool,
    pub ov_dir: Option<PathBuf>,
}

/// Verify a DCP by delegating to dcpdoctor-core.
pub fn verify_dcp(dcp_dir: &Path) -> VerifyResult {
    verify_dcp_with_options(dcp_dir, &VerifyCliOptions::default())
}

/// Verify for the QC report, which reads the picture essence itself.
///
/// The frame-by-frame scan is the expensive part of a verify, so the report pays
/// for it and a plain verify stays fast.
pub fn verify_dcp_for_report(dcp_dir: &Path) -> VerifyResult {
    verify_dcp_with_options(
        dcp_dir,
        &VerifyCliOptions {
            scan_every_frame: true,
            ..VerifyCliOptions::default()
        },
    )
}

/// Verify a DCP with the specified options.
pub fn verify_dcp_with_options(dcp_dir: &Path, options: &VerifyCliOptions) -> VerifyResult {
    verify_dcp_with_progress(dcp_dir, options, &mut |_| {})
}

pub fn verify_dcp_with_progress(
    dcp_dir: &Path,
    options: &VerifyCliOptions,
    progress: &mut dyn FnMut(VerifyProgress),
) -> VerifyResult {
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

    let opts = if options.strict {
        let mut o = dcpdoctor_core::VerifyOptions::strict();
        if options.skip_hash_check {
            o.check_hashes = false;
        }
        if options.skip_picture_check {
            o.check_picture_details = false;
        }
        o.scan_every_frame = options.scan_every_frame;
        o.skip_bitrate_measurement = options.skip_bitrate_measurement;
        o.ov = options.ov_dir.clone();
        o
    } else {
        dcpdoctor_core::VerifyOptions {
            check_hashes: !options.skip_hash_check,
            check_signatures: true,
            check_picture_details: !options.skip_picture_check,
            scan_every_frame: options.scan_every_frame,
            skip_bitrate_measurement: options.skip_bitrate_measurement,
            strict_smpte: false,
            ov: options.ov_dir.clone(),
            kdm: None,
            recipient_key: None,
            // a DCP gets no IMF pass
            photon: None,
        }
    };

    let report = dcpdoctor_core::verify_with_progress(dcp_dir, &opts, progress);
    verify_result_from_notes(&report.notes)
}

pub fn check_bv21_profile(dcp_dir: &Path) -> VerifyResult {
    let standard = dcpdoctor_core::dcp::detect_standard(dcp_dir);
    verify_result_from_notes(&dcpdoctor_core::advanced::check_bv21_compliance(
        dcp_dir, standard,
    ))
}

fn verify_result_from_notes(notes: &[dcpdoctor_core::Note]) -> VerifyResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    for note in notes {
        match note.severity {
            dcpdoctor_core::Severity::Error => errors.push(note.to_string()),
            dcpdoctor_core::Severity::Warning => warnings.push(note.to_string()),
            dcpdoctor_core::Severity::Info => info.push(note.to_string()),
        }
    }

    VerifyResult {
        valid: errors.is_empty(),
        errors,
        warnings,
        info,
    }
}

/// Write verification report to a file. Supports .txt and .html extensions.
pub fn write_verify_report(
    result: &VerifyResult,
    bv21_profile: Option<&VerifyResult>,
    output: &Path,
) -> Result<(), String> {
    let ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt")
        .to_lowercase();

    let content = match ext.as_str() {
        "html" | "htm" => format_report_html(result, bv21_profile),
        _ => format_report_text(result, bv21_profile),
    };

    std::fs::write(output, content)
        .map_err(|e| format!("Failed to write report to {}: {e}", output.display()))
}

fn verdict(passed: bool) -> &'static str {
    if passed { "PASSED" } else { "FAILED" }
}

fn report_passed(result: &VerifyResult, bv21_profile: Option<&VerifyResult>) -> bool {
    result.valid && bv21_profile.is_none_or(|profile| profile.valid)
}

fn format_report_text(result: &VerifyResult, bv21_profile: Option<&VerifyResult>) -> String {
    let mut out = format!(
        "DCP Verification: {}\n\n",
        verdict(report_passed(result, bv21_profile))
    );
    push_text_findings(&mut out, result);
    if let Some(profile) = bv21_profile {
        out.push_str(&format!(
            "\nBv2.1 profile check: {}\n\n",
            verdict(profile.valid)
        ));
        push_text_findings(&mut out, profile);
    }
    out
}

fn push_text_findings(out: &mut String, result: &VerifyResult) {
    if !result.errors.is_empty() {
        out.push_str("ERRORS:\n");
        for e in &result.errors {
            out.push_str(&format!("  [ERROR] {e}\n"));
        }
        out.push('\n');
    }
    if !result.warnings.is_empty() {
        out.push_str("WARNINGS:\n");
        for w in &result.warnings {
            out.push_str(&format!("  [WARN] {w}\n"));
        }
        out.push('\n');
    }
    if !result.info.is_empty() {
        out.push_str("INFO:\n");
        for i in &result.info {
            out.push_str(&format!("  [INFO] {i}\n"));
        }
    }
}

fn format_report_html(result: &VerifyResult, bv21_profile: Option<&VerifyResult>) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">\n");
    out.push_str("<title>DCP Verification Report</title>\n");
    out.push_str("<style>body{font-family:sans-serif;margin:2em}");
    out.push_str(".error{color:#c00}.warn{color:#a60}.info{color:#060}");
    out.push_str("h1{margin-bottom:0.5em}</style></head><body>\n");
    out.push_str(&format!(
        "<h1>DCP Verification: {}</h1>\n",
        verdict(report_passed(result, bv21_profile))
    ));
    push_html_findings(&mut out, result, "h2");
    if let Some(profile) = bv21_profile {
        out.push_str(&format!(
            "<h2>Bv2.1 profile check: {}</h2>\n",
            verdict(profile.valid)
        ));
        push_html_findings(&mut out, profile, "h3");
    }
    out.push_str("</body></html>\n");
    out
}

fn push_html_findings(out: &mut String, result: &VerifyResult, heading: &str) {
    let sections = [
        ("Errors", "error", &result.errors),
        ("Warnings", "warn", &result.warnings),
        ("Info", "info", &result.info),
    ];
    for (title, class, findings) in sections {
        if findings.is_empty() {
            continue;
        }
        out.push_str(&format!("<{heading}>{title}</{heading}><ul>\n"));
        for finding in findings {
            out.push_str(&format!(
                "<li class=\"{class}\">{}</li>\n",
                html_escape(finding)
            ));
        }
        out.push_str("</ul>\n");
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_verify_nonexistent_directory() {
        let result = verify_dcp(Path::new("/tmp/nonexistent_dcp_xyz"));
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].contains("not found"));
    }

    #[test]
    fn test_verify_missing_assetmap() {
        let tmp = tempfile::tempdir().unwrap();
        let result = verify_dcp(tmp.path());
        assert!(!result.valid);
        assert!(result.errors[0].contains("ASSETMAP"));
    }

    #[test]
    fn test_verify_with_options_skip_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let options = VerifyCliOptions {
            skip_hash_check: true,
            ..VerifyCliOptions::default()
        };
        let result = verify_dcp_with_options(tmp.path(), &options);
        // Should still fail (no ASSETMAP) but exercising the code path
        assert!(!result.valid);
    }

    #[test]
    fn test_write_verify_report_text() {
        let tmp = tempfile::NamedTempFile::with_suffix(".txt").unwrap();
        let result = VerifyResult {
            valid: false,
            errors: vec!["Missing CPL".into()],
            warnings: vec!["Unusual frame rate".into()],
            info: vec!["SMPTE standard detected".into()],
        };
        write_verify_report(&result, None, tmp.path()).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("FAILED"));
        assert!(content.contains("Missing CPL"));
        assert!(content.contains("Unusual frame rate"));
        assert!(content.contains("SMPTE standard detected"));
    }

    #[test]
    fn test_write_verify_report_html() {
        let tmp = tempfile::NamedTempFile::with_suffix(".html").unwrap();
        let result = VerifyResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
            info: vec!["All good".into()],
        };
        write_verify_report(&result, None, tmp.path()).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("PASSED"));
        assert!(content.contains("All good"));
    }

    fn bv21_profile_with_findings() -> VerifyResult {
        VerifyResult {
            valid: false,
            errors: vec!["Picture bitrate exceeds the Bv2.1 limit".into()],
            warnings: vec!["Sound track has no MCA labels".into()],
            info: vec![],
        }
    }

    fn passed_verification() -> VerifyResult {
        VerifyResult {
            valid: true,
            info: vec!["All good".into()],
            ..VerifyResult::default()
        }
    }

    #[test]
    fn the_text_report_holds_the_bv21_findings_and_their_verdict() {
        let tmp = tempfile::NamedTempFile::with_suffix(".txt").unwrap();
        write_verify_report(
            &passed_verification(),
            Some(&bv21_profile_with_findings()),
            tmp.path(),
        )
        .unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(content.starts_with("DCP Verification: FAILED"));
        assert!(content.contains("Bv2.1 profile check: FAILED"));
        assert!(content.contains("[ERROR] Picture bitrate exceeds the Bv2.1 limit"));
        assert!(content.contains("[WARN] Sound track has no MCA labels"));
        assert!(content.contains("[INFO] All good"));
    }

    #[test]
    fn the_html_report_holds_the_bv21_findings_and_their_verdict() {
        let tmp = tempfile::NamedTempFile::with_suffix(".html").unwrap();
        write_verify_report(
            &passed_verification(),
            Some(&bv21_profile_with_findings()),
            tmp.path(),
        )
        .unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("<h1>DCP Verification: FAILED</h1>"));
        assert!(content.contains("<h2>Bv2.1 profile check: FAILED</h2>"));
        assert!(
            content.contains("<li class=\"error\">Picture bitrate exceeds the Bv2.1 limit</li>")
        );
        assert!(content.contains("<li class=\"warn\">Sound track has no MCA labels</li>"));
        assert!(content.contains("<li class=\"info\">All good</li>"));
    }

    #[test]
    fn a_passed_bv21_profile_keeps_the_report_passed() {
        let tmp = tempfile::NamedTempFile::with_suffix(".html").unwrap();
        let profile = VerifyResult {
            valid: true,
            ..VerifyResult::default()
        };
        write_verify_report(&passed_verification(), Some(&profile), tmp.path()).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("<h1>DCP Verification: PASSED</h1>"));
        assert!(content.contains("<h2>Bv2.1 profile check: PASSED</h2>"));
    }

    #[test]
    fn test_html_escape_in_report() {
        let tmp = tempfile::NamedTempFile::with_suffix(".html").unwrap();
        let result = VerifyResult {
            valid: false,
            errors: vec!["Problem with <file> & stuff".into()],
            warnings: vec![],
            info: vec![],
        };
        write_verify_report(&result, None, tmp.path()).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("&lt;file&gt;"));
        assert!(content.contains("&amp;"));
    }
}
