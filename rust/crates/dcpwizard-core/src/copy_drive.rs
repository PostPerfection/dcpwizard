use sha1::Digest;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const COPY_BUFFER_BYTES: usize = 1 << 20;

// a cinema server reads the drive as a user other than the one who copied it
const COPIED_DIRECTORY_MODE: u32 = 0o755;
const COPIED_FILE_MODE: u32 = 0o644;

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

fn create_readable_directories(root: &Path, relative_directory: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(root)?;
    set_mode(root, COPIED_DIRECTORY_MODE)?;
    let mut directory = root.to_path_buf();
    for component in relative_directory.components() {
        directory.push(component);
        std::fs::create_dir_all(&directory)?;
        set_mode(&directory, COPIED_DIRECTORY_MODE)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn warn_unless_cinema_server_filesystem(target_dir: &Path) {
    let Ok(output) = std::process::Command::new("findmnt")
        .args(["--noheadings", "--output", "FSTYPE", "--target"])
        .arg(target_dir)
        .output()
    else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let fstype = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if crate::disk::is_cinema_server_filesystem(&fstype) {
        return;
    }
    tracing::warn!(
        "{} is on {fstype}. Cinema servers reliably read only ext2 and ext3.",
        target_dir.display()
    );
}

#[cfg(not(target_os = "linux"))]
fn warn_unless_cinema_server_filesystem(_target_dir: &Path) {}

/// Evict a file's pages from the page cache so a following read hits the device.
/// Without this the read-back below just returns the bytes we cached on write and
/// verifies nothing about what actually landed on the drive.
#[cfg(target_os = "linux")]
fn drop_page_cache(file: &File) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED);
    }
}

// macos has no posix_fadvise; the read-back there may be served from cache
#[cfg(not(target_os = "linux"))]
fn drop_page_cache(_file: &File) {}

/// Copy a DCP to a target drive with SHA-1 hash verification.
pub fn copy_to_drive(dcp_dir: &Path, target_dir: &Path) -> i32 {
    if !dcp_dir.exists() {
        tracing::error!("Source DCP directory not found: {}", dcp_dir.display());
        return -1;
    }

    let dest = target_dir.join(
        dcp_dir
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("DCP")),
    );

    if let Err(e) = create_readable_directories(&dest, Path::new("")) {
        tracing::error!("Failed to create target directory: {e}");
        return -1;
    }
    warn_unless_cinema_server_filesystem(target_dir);

    let files = collect_files(dcp_dir);
    let total = files.len();

    // Fail early if the DCP won't fit on the destination (DoM bug 3150).
    let required: u64 = files
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum();
    if let Err(e) = postkit::free_space::check_destination_space(&dest, required) {
        tracing::error!("{e}");
        return -1;
    }

    tracing::info!(
        "Copying {total} files from {} to {}",
        dcp_dir.display(),
        dest.display()
    );

    for (i, src_path) in files.iter().enumerate() {
        let rel = src_path.strip_prefix(dcp_dir).unwrap_or(src_path);
        let dst_path = dest.join(rel);

        if let Some(relative_directory) = rel.parent()
            && let Err(e) = create_readable_directories(&dest, relative_directory)
        {
            tracing::error!(
                "Failed to create directory {}: {e}",
                dest.join(relative_directory).display()
            );
            return -1;
        }

        let src_hash = match copy_hashing(src_path, &dst_path) {
            Ok(hash) => hash,
            Err(e) => {
                tracing::error!(
                    "Failed to copy {} to {}: {e}",
                    src_path.display(),
                    dst_path.display()
                );
                return -1;
            }
        };

        let dst_hash = match postkit::hash::hash_file(&dst_path, postkit::hash::HashAlgorithm::Sha1)
        {
            Ok(result) => result.hex,
            Err(e) => {
                tracing::error!("Failed to read back {}: {e}", dst_path.display());
                return -1;
            }
        };

        if src_hash != dst_hash {
            tracing::error!(
                "Hash mismatch for {}: src={} dst={}",
                rel.display(),
                src_hash,
                dst_hash
            );
            return -1;
        }

        tracing::info!(
            "[{}/{}] Verified: {} ({})",
            i + 1,
            total,
            rel.display(),
            src_hash
        );
    }

    tracing::info!("Successfully copied and verified {total} files");
    0
}

fn collect_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files);
    files.sort();
    files
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, files);
        } else {
            files.push(path);
        }
    }
}

// a feature's picture MXF is tens of GB
fn copy_hashing(src_path: &Path, dst_path: &Path) -> std::io::Result<String> {
    let mut source = File::open(src_path)?;
    let mut sink = File::create(dst_path)?;
    set_mode(dst_path, COPIED_FILE_MODE)?;
    let mut hasher = sha1::Sha1::new();
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        sink.write_all(&buffer[..read])?;
    }
    sink.sync_all()?;
    drop_page_cache(&sink);
    Ok(hex_encode(&hasher.finalize()))
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_and_verifies_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("MyDCP");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("ASSETMAP.xml"), b"a").unwrap();
        std::fs::write(src.join("sub/picture.mxf"), vec![7u8; 4096]).unwrap();

        let target = dir.path().join("drive");
        assert_eq!(copy_to_drive(&src, &target), 0);

        let dst = target.join("MyDCP");
        assert_eq!(std::fs::read(dst.join("ASSETMAP.xml")).unwrap(), b"a");
        assert_eq!(
            std::fs::read(dst.join("sub/picture.mxf")).unwrap(),
            vec![7u8; 4096]
        );
    }

    #[cfg(unix)]
    #[test]
    fn copied_files_and_folders_are_readable_by_everyone_under_a_private_umask() {
        use std::os::unix::fs::PermissionsExt;
        const PRIVATE_UMASK: libc::mode_t = 0o077;
        const PERMISSION_BITS: u32 = 0o777;

        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("MyDCP");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("ASSETMAP.xml"), b"a").unwrap();
        std::fs::write(src.join("sub/picture.mxf"), b"p").unwrap();
        let target = dir.path().join("drive");

        let previous_umask = unsafe { libc::umask(PRIVATE_UMASK) };
        let status = copy_to_drive(&src, &target);
        unsafe { libc::umask(previous_umask) };
        assert_eq!(status, 0);

        let mode =
            |path: PathBuf| std::fs::metadata(path).unwrap().permissions().mode() & PERMISSION_BITS;
        let dst = target.join("MyDCP");
        assert_eq!(mode(dst.clone()), COPIED_DIRECTORY_MODE);
        assert_eq!(mode(dst.join("sub")), COPIED_DIRECTORY_MODE);
        assert_eq!(mode(dst.join("ASSETMAP.xml")), COPIED_FILE_MODE);
        assert_eq!(mode(dst.join("sub/picture.mxf")), COPIED_FILE_MODE);
    }

    #[test]
    fn free_space_check_refuses_a_requirement_no_volume_can_hold() {
        // u64::MAX rendered by the check's own byte formatter
        const REQUIRED_TEXT: &str = "need 16777216.0 TiB";

        let dir = tempfile::tempdir().unwrap();
        let err = postkit::free_space::check_destination_space(dir.path(), u64::MAX).unwrap_err();
        assert!(err.contains(&dir.path().display().to_string()), "{err}");
        assert!(err.contains(REQUIRED_TEXT), "{err}");
        assert!(err.contains("but only ") && err.ends_with(" free"), "{err}");
    }

    #[test]
    fn free_space_check_allows_a_zero_requirement() {
        let dir = tempfile::tempdir().unwrap();
        assert!(postkit::free_space::check_destination_space(dir.path(), 0).is_ok());
    }

    // a sparse file counts its apparent size while staying a few bytes on disk
    #[cfg(unix)]
    #[test]
    fn refuses_a_dcp_larger_than_the_free_space() {
        // 8 TiB: past any runner's free space and under ext4's 16 TiB file limit
        const SPARSE_PICTURE_BYTES: u64 = 8 << 40;

        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("MyDCP");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("ASSETMAP.xml"), b"a").unwrap();
        File::create(src.join("picture.mxf"))
            .unwrap()
            .set_len(SPARSE_PICTURE_BYTES)
            .unwrap();

        let target = dir.path().join("drive");
        assert_eq!(copy_to_drive(&src, &target), -1);

        let copied = std::fs::read_dir(target.join("MyDCP"))
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(copied, 0, "nothing copied");
    }
}
