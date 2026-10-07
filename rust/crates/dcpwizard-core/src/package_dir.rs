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

pub enum PackageName<'a> {
    Title(&'a str),
    Isdcf(crate::isdcf_title::IsdcfNaming<'a>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub title: String,
    pub dir: PathBuf,
}

// the folder takes the content title, which ISDCF naming replaces with the
// ISDCF name, the way DCP-o-matic names it with Film::dcp_name
pub fn new_package(output: &Path, name: &PackageName, resume: bool) -> Result<Package, String> {
    let title = match name {
        PackageName::Title(title) => title.to_string(),
        PackageName::Isdcf(naming) => naming.name()?,
    };
    if let PackageName::Isdcf(naming) = name
        && resume
        && !package_dir(output, &title)?.exists()
    {
        return resumed_isdcf_package(output, naming, &title);
    }
    Ok(Package {
        dir: new_package_dir(output, &title)?,
        title,
    })
}

// the name carries the day it was made, so a resume on a later day looks for
// the same name with another date
fn resumed_isdcf_package(
    output: &Path,
    naming: &crate::isdcf_title::IsdcfNaming,
    title: &str,
) -> Result<Package, String> {
    let undated_folder_name = package_folder_name(&naming.name_with_date(None)?)?;
    let mut candidates: Vec<(PathBuf, crate::isdcf_name::IsdcfDate)> = std::fs::read_dir(output)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let dir = entry.ok()?.path();
            crate::encode_qol::EncodeState::load(&dir)?;
            let date = crate::isdcf_title::date_apart_from(
                dir.file_name()?.to_str()?,
                &undated_folder_name,
            )?;
            Some((dir, date))
        })
        .collect();
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    match candidates.as_slice() {
        [(dir, date)] => Ok(Package {
            title: naming.name_with_date(Some(*date))?,
            dir: dir.clone(),
        }),
        [] => Err(format!(
            "--resume found no interrupted encode in {} named {} or that name with another date",
            output.display(),
            package_folder_name(title)?
        )),
        several => Err(format!(
            "--resume found {} interrupted encodes in {} whose names differ only by date: {}. \
             Pass --isdcf-date with the date of the one to resume",
            several.len(),
            output.display(),
            several
                .iter()
                .map(|(dir, _)| dir.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
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

    fn naming_config() -> crate::dcp::DcpConfig {
        crate::dcp::DcpConfig {
            title: "My Film".into(),
            frame_rate_num: 24,
            frame_rate_den: 1,
            container_width: 1998,
            container_height: 1080,
            ..Default::default()
        }
    }

    fn dated(day: u32) -> crate::isdcf_title::IsdcfNamingOptions {
        crate::isdcf_title::IsdcfNamingOptions {
            date: Some(crate::isdcf_name::IsdcfDate {
                year: 2026,
                month: 9,
                day,
            }),
            ..Default::default()
        }
    }

    fn isdcf<'a>(
        config: &'a crate::dcp::DcpConfig,
        options: &'a crate::isdcf_title::IsdcfNamingOptions,
    ) -> PackageName<'a> {
        PackageName::Isdcf(crate::isdcf_title::IsdcfNaming {
            config,
            options,
            sound: crate::isdcf_title::SoundtrackSource {
                audio: None,
                channel_files: None,
                picture: None,
                audio_map: None,
                upmix: false,
                hi_channel: None,
                vi_channel: None,
            },
            burnt_in_subtitle: false,
        })
    }

    fn interrupted_encode(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        crate::encode_qol::EncodeState {
            source: "source.mov".into(),
            total_frames: 24,
            fps: 24,
            width: 1998,
            height: 1080,
            bitrate_mbps: 250,
        }
        .save(dir)
        .unwrap();
    }

    const FIRST_DAY: &str = "MyFilm_FTR-1_F_XX-XX_MOS_2K_20260901_SMPTE_OV";
    const SECOND_DAY: &str = "MyFilm_FTR-1_F_XX-XX_MOS_2K_20260902_SMPTE_OV";

    #[test]
    fn without_isdcf_naming_the_title_names_the_folder() {
        let output = tempfile::tempdir().unwrap();
        let package = new_package(output.path(), &PackageName::Title("My Film"), false).unwrap();
        assert_eq!(package.title, "My Film");
        assert_eq!(package.dir, output.path().join("My Film"));
    }

    #[test]
    fn with_isdcf_naming_the_isdcf_name_names_the_folder() {
        let output = tempfile::tempdir().unwrap();
        let (config, options) = (naming_config(), dated(1));
        let package = new_package(output.path(), &isdcf(&config, &options), false).unwrap();
        assert_eq!(package.title, FIRST_DAY);
        assert_eq!(package.dir, output.path().join(FIRST_DAY));
    }

    #[test]
    fn an_existing_isdcf_named_package_is_refused() {
        let output = tempfile::tempdir().unwrap();
        let (config, options) = (naming_config(), dated(1));
        let dir = output.path().join(FIRST_DAY);
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("ASSETMAP.xml"), b"x").unwrap();
        let error = new_package(output.path(), &isdcf(&config, &options), false).unwrap_err();
        assert!(
            error.starts_with(&format!("A DCP already exists at {}", dir.display())),
            "{error}"
        );
    }

    #[test]
    fn a_resume_on_a_later_day_finds_the_earlier_days_folder_and_name() {
        let output = tempfile::tempdir().unwrap();
        interrupted_encode(&output.path().join(FIRST_DAY));
        let (config, options) = (naming_config(), dated(2));
        let package = new_package(output.path(), &isdcf(&config, &options), true).unwrap();
        assert_eq!(package.title, FIRST_DAY);
        assert_eq!(package.dir, output.path().join(FIRST_DAY));
    }

    #[test]
    fn a_resume_uses_the_computed_folder_when_it_exists() {
        let output = tempfile::tempdir().unwrap();
        interrupted_encode(&output.path().join(FIRST_DAY));
        interrupted_encode(&output.path().join(SECOND_DAY));
        let (config, options) = (naming_config(), dated(2));
        let package = new_package(output.path(), &isdcf(&config, &options), true).unwrap();
        assert_eq!(package.dir, output.path().join(SECOND_DAY));
    }

    #[test]
    fn a_resume_with_no_interrupted_encode_of_that_name_is_refused() {
        let output = tempfile::tempdir().unwrap();
        // a folder of that name with no resume state, and another package's encode
        std::fs::create_dir(output.path().join(FIRST_DAY)).unwrap();
        interrupted_encode(
            &output
                .path()
                .join("Other_FTR-1_F_XX-XX_MOS_2K_20260901_SMPTE_OV"),
        );
        let (config, options) = (naming_config(), dated(3));
        let error = new_package(output.path(), &isdcf(&config, &options), true).unwrap_err();
        assert!(error.contains("found no interrupted encode"), "{error}");
        assert!(
            error.contains("MyFilm_FTR-1_F_XX-XX_MOS_2K_20260903_SMPTE_OV"),
            "{error}"
        );
    }

    #[test]
    fn a_resume_with_several_dates_to_choose_from_is_refused_naming_them() {
        let output = tempfile::tempdir().unwrap();
        interrupted_encode(&output.path().join(FIRST_DAY));
        interrupted_encode(&output.path().join(SECOND_DAY));
        let (config, options) = (naming_config(), dated(3));
        let error = new_package(output.path(), &isdcf(&config, &options), true).unwrap_err();
        assert!(error.contains("found 2 interrupted encodes"), "{error}");
        assert!(
            error.contains(FIRST_DAY) && error.contains(SECOND_DAY),
            "{error}"
        );
    }
}
