use std::path::Path;

pub use postkit::certificate::{CertInfo, CertOptions, CertType, TrustedDevice};

/// Generate a single X.509 certificate.
pub fn generate_certificate(opts: &CertOptions) -> i32 {
    postkit::certificate::generate_certificate(opts)
}

/// Generate a full certificate chain (root → intermediate → signer).
pub fn generate_chain(organization: &str, output_dir: &Path) -> i32 {
    postkit::certificate::generate_chain(organization, output_dir)
}

/// Read and display certificate info from a PEM file.
pub fn read_certificate(cert_path: &Path) -> CertInfo {
    postkit::certificate::read_certificate(cert_path)
}
