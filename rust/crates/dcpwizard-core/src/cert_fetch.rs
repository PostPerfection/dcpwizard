//! download a projector/server recipient certificate by vendor + serial
//! (dom#2705, dom#2706). only vendors whose public endpoint was verified live
//! are implemented; others return one clear line telling the user to get the
//! cert from the vendor. downloads are validated as parseable X.509 before being
//! written. ftp/sftp fetching is delegated to the system `curl` binary (same
//! transport dcp-o-matic uses via libcurl); the serial is validated first and
//! passed as a plain argument, never through a shell.
//!
//! verified endpoints (anonymous ftp, directory listing confirmed 2026-07):
//!   dolby/doremi: ftp://ftp.cinema.dolby.com/Certificates/<first3>xxx/
//!   qube:         ftp://certificates.qubecinema.com/SMPTE-<type>/

use crate::store;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Vendor {
    /// dolby / doremi digit-serial servers (DCP2000/IMB/IMS): sha256 leaf PEM.
    Dolby,
    /// qube cinema servers, keyed by device type (e.g. QXPD).
    Qube,
}

const DOLBY_BASE: &str = "ftp://ftp.cinema.dolby.com/Certificates";
const QUBE_BASE: &str = "ftp://certificates.qubecinema.com";

/// map a vendor name to a supported downloader, or a clear "get it from the
/// vendor" error for vendors without an automated public endpoint.
pub fn parse_vendor(name: &str) -> Result<Vendor, String> {
    match name.to_ascii_lowercase().as_str() {
        "dolby" | "doremi" => Ok(Vendor::Dolby),
        "qube" => Ok(Vendor::Qube),
        v @ ("christie" | "gdc" | "barco" | "sony" | "nec" | "gdc-tech") => Err(format!(
            "automated download for {v} is not available; obtain the certificate from the vendor"
        )),
        other => Err(format!(
            "unknown vendor '{other}'; obtain the certificate from the vendor"
        )),
    }
}

/// reject anything but plain serial characters so nothing odd reaches curl or a
/// url path.
pub fn validate_serial(serial: &str) -> Result<(), String> {
    if serial.is_empty()
        || !serial
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(format!(
            "invalid serial '{serial}': use letters, digits and '-' only"
        ));
    }
    Ok(())
}

fn dolby_dir_url(serial: &str) -> Result<String, String> {
    let digits: String = serial.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 3 {
        return Err(format!(
            "dolby serial '{serial}' must start with at least 3 digits"
        ));
    }
    Ok(format!("{DOLBY_BASE}/{}xxx/", &digits[..3]))
}

/// among a directory listing, pick the sha256 leaf cert for this serial.
fn dolby_pick(entries: &[String], serial: &str) -> Option<String> {
    let want = format!("-{serial}.cert.sha256.pem");
    entries.iter().find(|e| e.ends_with(&want)).cloned()
}

fn qube_dir_url(qtype: &str) -> String {
    format!("{QUBE_BASE}/SMPTE-{qtype}/")
}

fn qube_pad(serial: &str) -> String {
    if serial.chars().all(|c| c.is_ascii_digit()) && serial.len() < 5 {
        format!("{serial:0>5}")
    } else {
        serial.to_string()
    }
}

/// pick the sha256 cert whose name starts with "<type>-<padded serial>-".
fn qube_pick(entries: &[String], qtype: &str, serial: &str) -> Option<String> {
    let prefix = format!("{qtype}-{}-", qube_pad(serial));
    let mut matches: Vec<&String> = entries
        .iter()
        .filter(|e| e.starts_with(&prefix) && e.contains("sha256"))
        .collect();
    matches.sort();
    matches.last().map(|s| (*s).clone())
}

/// run `curl` for one url, returning stdout bytes. distinguishes an unreachable
/// host from a missing file so callers can report a useful message (dom#2705).
fn curl(url: &str, list: bool) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new("curl");
    cmd.arg("-s")
        .args(["--connect-timeout", "15"])
        .args(["--max-time", "45"]);
    if list {
        // "-l" name-only listing for an ftp directory url.
        cmd.arg("-l");
    }
    cmd.arg(url);
    let out = cmd
        .output()
        .map_err(|e| format!("cannot run curl (is it installed?): {e}"))?;
    if out.status.success() {
        return Ok(out.stdout);
    }
    let code = out.status.code().unwrap_or(-1);
    // curl exit codes: 6 resolve, 7 connect, 28 timeout -> network; else likely
    // a missing path.
    let msg = match code {
        6 => "could not resolve host",
        7 => "could not connect to host",
        28 => "connection timed out",
        9 | 19 | 78 => "file not found on server",
        _ => "download failed",
    };
    Err(format!("{msg} (curl exit {code})"))
}

/// download, validate and write a recipient cert. returns "subject / serial" on
/// success. `qube_type` is required for qube and ignored for dolby.
pub fn fetch(
    vendor: Vendor,
    serial: &str,
    qube_type: Option<&str>,
    out: &Path,
) -> Result<String, String> {
    validate_serial(serial)?;

    let (dir_url, file) = match vendor {
        Vendor::Dolby => {
            let dir = dolby_dir_url(serial)?;
            let listing = curl(&dir, true)?;
            let entries: Vec<String> = String::from_utf8_lossy(&listing)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            let file = dolby_pick(&entries, serial)
                .ok_or_else(|| format!("no certificate for dolby serial '{serial}' at {dir}"))?;
            (dir, file)
        }
        Vendor::Qube => {
            let qtype = qube_type.ok_or("qube requires --type (the device type, e.g. QXPD)")?;
            let dir = qube_dir_url(qtype);
            let listing = curl(&dir, true)?;
            let entries: Vec<String> = String::from_utf8_lossy(&listing)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            let file = qube_pick(&entries, qtype, serial).ok_or_else(|| {
                format!("no certificate for qube {qtype} serial '{serial}' at {dir}")
            })?;
            (dir, file)
        }
    };

    let bytes = curl(&format!("{dir_url}{file}"), false)?;
    let pem = String::from_utf8_lossy(&bytes).to_string();
    // untrusted download: must parse as X.509 before we store it.
    let info = store::cert_info_from_pem(&pem)?;
    store::atomic_write(out, pem.as_bytes())?;
    Ok(format!("{} / {}", info.subject_cn, info.serial))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_parsing_routes_correctly() {
        assert_eq!(parse_vendor("dolby").unwrap(), Vendor::Dolby);
        assert_eq!(parse_vendor("Doremi").unwrap(), Vendor::Dolby);
        assert_eq!(parse_vendor("qube").unwrap(), Vendor::Qube);
        // known-but-unsupported vendors tell the user to get it from the vendor
        let e = parse_vendor("christie").unwrap_err();
        assert!(e.contains("obtain the certificate from the vendor"));
        assert!(parse_vendor("acme").is_err());
    }

    #[test]
    fn serial_validation_blocks_odd_input() {
        assert!(validate_serial("218281828").is_ok());
        assert!(validate_serial("H123-45").is_ok());
        assert!(validate_serial("").is_err());
        assert!(validate_serial("../etc/passwd").is_err());
        assert!(validate_serial("a b").is_err());
    }

    #[test]
    fn dolby_dir_and_pick() {
        assert_eq!(
            dolby_dir_url("218281828").unwrap(),
            "ftp://ftp.cinema.dolby.com/Certificates/218xxx/"
        );
        assert!(dolby_dir_url("ab").is_err());
        // matches the real dolby naming (Dolby-DCP2000-<serial>.cert.sha256.pem)
        let entries = vec![
            "Dolby-DCP2000-218281828.cert.mpeg.pem".to_string(),
            "Dolby-DCP2000-218281828.cert.sha256.pem".to_string(),
            "Dolby-DCP2000-218281828.dcicerts.zip".to_string(),
        ];
        assert_eq!(
            dolby_pick(&entries, "218281828").unwrap(),
            "Dolby-DCP2000-218281828.cert.sha256.pem"
        );
        assert!(dolby_pick(&entries, "999999999").is_none());
    }

    #[test]
    fn qube_pad_and_pick() {
        assert_eq!(qube_pad("54"), "00054");
        assert_eq!(qube_pad("00054"), "00054");
        // matches real qube naming (QXPD-00054-10-14-sha256.pem.crt)
        let entries = vec![
            "QXPD-00054-10-14-sha256.pem.crt".to_string(),
            "QXPD-00097-08-14-sha256.pem.crt".to_string(),
        ];
        assert_eq!(
            qube_pick(&entries, "QXPD", "54").unwrap(),
            "QXPD-00054-10-14-sha256.pem.crt"
        );
        assert!(qube_pick(&entries, "QXPD", "12345").is_none());
    }
}
