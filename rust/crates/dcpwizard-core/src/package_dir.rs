use std::path::{Path, PathBuf};

// the windows set, which covers the path separators of every platform
const CHARACTERS_NOT_ALLOWED_IN_FOLDER_NAMES: [char; 9] =
    ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const REPLACEMENT: char = '_';

pub fn package_folder_name(title: &str) -> Result<String, String> {
    let replaced: String = title
        .chars()
        .map(|character| {
            if character.is_control() || CHARACTERS_NOT_ALLOWED_IN_FOLDER_NAMES.contains(&character)
            {
                REPLACEMENT
            } else {
                character
            }
        })
        .collect();
    // windows drops trailing dots and spaces from a folder name
    let trimmed = replaced.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return Err(format!(
            "the title {title:?} leaves nothing to name the package folder with"
        ));
    }
    Ok(trimmed.to_string())
}

// every package is written to a folder named after its title inside the chosen
// output folder, the way DCP-o-matic writes <film directory>/<dcp name>
pub fn package_dir(output: &Path, title: &str) -> Result<PathBuf, String> {
    Ok(output.join(package_folder_name(title)?))
}

// the SMPTE and Interop names of the files at a finished package's root
const DCP_ROOT_FILES: [&str; 4] = ["ASSETMAP.xml", "VOLINDEX.xml", "ASSETMAP", "VOLINDEX"];

pub fn holds_dcp(dir: &Path) -> bool {
    DCP_ROOT_FILES.iter().any(|name| dir.join(name).is_file())
}

// a reused title would land in the old package
pub fn new_package_dir(output: &Path, title: &str) -> Result<PathBuf, String> {
    let dir = package_dir(output, title)?;
    if holds_dcp(&dir) {
        return Err(format!(
            "A DCP already exists at {}. Use a new title or output folder, or delete the old package first.",
            dir.display()
        ));
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_title_names_the_folder_as_is() {
        assert_eq!(
            package_dir(Path::new("/out"), "My Film").unwrap(),
            PathBuf::from("/out/My Film")
        );
    }

    #[test]
    fn an_isdcf_name_names_the_folder_as_is() {
        assert_eq!(
            package_folder_name("MyFilm_FTR-1_F_EN-XX_MOS_2K_20260930_WRK_SMPTE_OV").unwrap(),
            "MyFilm_FTR-1_F_EN-XX_MOS_2K_20260930_WRK_SMPTE_OV"
        );
    }

    #[test]
    fn separators_and_windows_reserved_characters_are_replaced() {
        assert_eq!(
            package_folder_name("Part 1/2: \"Return\" <of> the|Film?*").unwrap(),
            "Part 1_2_ _Return_ _of_ the_Film__"
        );
    }

    #[test]
    fn trailing_dots_and_spaces_are_dropped() {
        assert_eq!(package_folder_name("  Film v2. . ").unwrap(), "Film v2");
    }

    #[test]
    fn only_a_folder_holding_a_dcp_counts_as_one() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!holds_dcp(dir.path()));
        std::fs::write(dir.path().join("movie.mov"), b"x").unwrap();
        assert!(!holds_dcp(dir.path()));
        std::fs::write(dir.path().join("ASSETMAP.xml"), b"x").unwrap();
        assert!(holds_dcp(dir.path()));
    }

    #[test]
    fn an_interop_package_counts_as_a_dcp() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("VOLINDEX"), b"x").unwrap();
        assert!(holds_dcp(dir.path()));
    }

    #[test]
    fn a_new_package_is_refused_over_an_existing_one_and_allowed_into_an_empty_folder() {
        let output = tempfile::tempdir().unwrap();
        let dir = output.path().join("My Film");
        std::fs::create_dir(&dir).unwrap();
        assert_eq!(new_package_dir(output.path(), "My Film").unwrap(), dir);
        std::fs::write(dir.join("ASSETMAP.xml"), b"x").unwrap();
        let error = new_package_dir(output.path(), "My Film").unwrap_err();
        assert_eq!(
            error,
            format!(
                "A DCP already exists at {}. Use a new title or output folder, or delete the old package first.",
                dir.display()
            )
        );
    }

    #[test]
    fn a_title_with_nothing_usable_is_refused() {
        assert!(package_folder_name("   ").is_err());
        assert!(package_folder_name("..").is_err());
    }
}
