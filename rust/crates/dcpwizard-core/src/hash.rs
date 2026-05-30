use std::path::Path;

/// Compute SHA-1 hash of a file, returned as base64.
pub fn hash_file(path: &Path) -> Result<String, String> {
    postkit::hash::hash_file(path, postkit::hash::HashAlgorithm::Sha1)
        .map(|r| r.base64)
        .map_err(|e| format!("Failed to hash {}: {e}", path.display()))
}
