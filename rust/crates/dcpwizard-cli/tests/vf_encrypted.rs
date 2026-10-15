use assert_cmd::Command;
use dcpwizard_core::dcp::{DcpConfig, create_dcp};
use dcpwizard_core::encrypt::KeyBundle;
use dcpwizard_core::package_signature::PackageSigner;
use postkit::certificate::{CertOptions, CertType, generate_certificate, generate_chain};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const FPS: u32 = 24;
const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const FRAMES: usize = 24;
const SUBTITLE_TEXT: &str = "Hello";
const VF_TITLE: &str = "Secret VF";
const OV_MARKERS: [&str; 3] = ["FFOC=1", "FFEC=12", "LFOC=23"];
const OV_RATING_AGENCY: &str = "http://www.mpaa.org/2003-ratings";
const OV_RATING_LABEL: &str = "PG-13";
const TRACK_ELEMENTS: [&str; 5] = [
    "MainPicture",
    "MainSound",
    "MainSubtitle",
    "ClosedCaption",
    "AuxData",
];
// 16 MB covers one 2K J2K frame
const MAX_FRAME_BUFFER: usize = 16 * 1024 * 1024;

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    chain: PathBuf,
    source_codestream: Vec<u8>,
}

impl Fixture {
    fn build() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        let chain = root.join("chain");
        assert_eq!(generate_chain("Studio", &chain), 0, "signer chain");
        let source_codestream = make_frames(&root.join("frames"));
        make_wav(&root.join("audio.wav"));
        make_atmos(&root.join("atmos"));
        std::fs::write(
            root.join("subs.srt"),
            format!("1\n00:00:00,100 --> 00:00:00,900\n{SUBTITLE_TEXT}\n\n"),
        )
        .unwrap();
        Self {
            _temp: temp,
            root,
            chain,
            source_codestream,
        }
    }

    fn signer(&self) -> PackageSigner {
        PackageSigner {
            signer_cert: self.chain.join("signer.pem"),
            signer_key: self.chain.join("signer.key"),
            signer_chain: vec![
                self.chain.join("intermediate.pem"),
                self.chain.join("root.pem"),
            ],
        }
    }

    // the create --encrypt path, with an Atmos track, markers and a rating
    fn make_ov(&self, encrypted: bool) -> (PathBuf, PathBuf) {
        let ov = self.root.join("ov");
        let ov_keys = self.root.join("ov_keys.json");
        let config = DcpConfig {
            title: "Secret".into(),
            standard: dcpwizard_core::Standard::Smpte,
            resolution: dcpwizard_core::Resolution::TwoK,
            content_type: dcpwizard_core::ContentType::Feature,
            frame_rate_num: FPS,
            frame_rate_den: 1,
            output_dir: ov.clone(),
            j2k_dir: Some(self.root.join("frames")),
            audio_path: Some(self.root.join("audio.wav")),
            atmos_path: Some(self.root.join("atmos")),
            markers: OV_MARKERS.iter().map(|m| m.to_string()).collect(),
            ratings: vec![dcpwizard_core::isdcf_name::Rating {
                agency: OV_RATING_AGENCY.into(),
                label: OV_RATING_LABEL.into(),
            }],
            encrypt: encrypted,
            key_out: encrypted.then(|| ov_keys.clone()),
            signer: Some(self.signer()),
            ..Default::default()
        };
        assert_eq!(create_dcp(&config), 0, "create the OV");
        (ov, ov_keys)
    }

    fn recipient(&self, name: &str) -> (PathBuf, PathBuf) {
        let cert = self.root.join(format!("{name}.pem"));
        let key = self.root.join(format!("{name}.key"));
        let options = CertOptions {
            cert_type: CertType::Leaf,
            common_name: name.into(),
            organization: "Cinema".into(),
            output_cert: cert.clone(),
            output_key: key.clone(),
            issuer_cert: self.chain.join("root.pem"),
            issuer_key: self.chain.join("root.key"),
            ..Default::default()
        };
        assert_eq!(generate_certificate(&options), 0, "recipient cert {name}");
        (cert, key)
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("dcpwizard").unwrap();
        command
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_DATA_HOME", self.root.join("data"));
        command
    }

    fn signed(&self, command: &mut Command) {
        command
            .arg("--signer-cert")
            .arg(self.chain.join("signer.pem"))
            .arg("--signer-key")
            .arg(self.chain.join("signer.key"))
            .arg("--signer-chain")
            .arg(self.chain.join("intermediate.pem"))
            .arg("--signer-chain")
            .arg(self.chain.join("root.pem"));
    }

    fn create_vf(&self, ov: &Path, extra: &[&std::ffi::OsStr]) -> Command {
        let mut command = self.create_unsigned_vf(ov, extra);
        self.signed(&mut command);
        command
    }

    fn create_unsigned_vf(&self, ov: &Path, extra: &[&std::ffi::OsStr]) -> Command {
        let mut command = self.command();
        command
            .arg("create-vf")
            .arg("--ov")
            .arg(ov)
            .arg("--output")
            .arg(self.root.join("vf"))
            .args(["--title", VF_TITLE])
            .arg("--add-subtitle")
            .arg(format!("1={}", self.root.join("subs.srt").display()))
            .arg("--subtitle-font")
            .arg(subtitle_font())
            .args(extra);
        command
    }

    fn vf_written(&self) -> bool {
        let Ok(entries) = std::fs::read_dir(self.root.join("vf")) else {
            return false;
        };
        entries
            .flatten()
            .any(|e| e.path().join("ASSETMAP.xml").exists())
    }

    fn vf_dir(&self) -> PathBuf {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(self.root.join("vf"))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        assert_eq!(dirs.len(), 1, "one VF package: {dirs:?}");
        dirs.remove(0)
    }

    fn kdm_from_keys_file(&self, cpl_id: &str, keys_file: &Path, recipient: &Path, output: &Path) {
        let mut command = self.command();
        command
            .args(["kdm", "--cpl-id", cpl_id, "--content-title", VF_TITLE])
            .args(["--valid-from", &tomorrow(), "--valid-to", "2 weeks"])
            .arg("--cert")
            .arg(recipient)
            .arg("--keys")
            .arg(keys_file)
            .arg("--output")
            .arg(output);
        self.signed(&mut command);
        command.assert().success();
    }
}

fn subtitle_font() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../dcpwizard-core/tests/fixtures/LiberationSans-Regular.ttf")
}

// a KDM starting on the day its signer certificate does is refused
fn tomorrow() -> String {
    (chrono::Utc::now() + chrono::Duration::days(1))
        .format("%Y-%m-%dT%H:%M:%S+00:00")
        .to_string()
}

fn make_frames(dir: &Path) -> Vec<u8> {
    std::fs::create_dir_all(dir).unwrap();
    let seed = dir.join("seed.j2c");
    dcpwizard_core::pad::generate_black_frame(WIDTH, HEIGHT, FPS, &seed).unwrap();
    let codestream = std::fs::read(&seed).unwrap();
    for index in 0..FRAMES {
        std::fs::copy(&seed, dir.join(format!("frame_{index:05}.j2c"))).unwrap();
    }
    std::fs::remove_file(&seed).unwrap();
    codestream
}

fn make_wav(path: &Path) {
    let sample_rate = 48_000;
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for _ in 0..FRAMES * (sample_rate / FPS) as usize * 2 {
        writer.write_sample(0).unwrap();
    }
    writer.finalize().unwrap();
}

// one Atmos input file per picture frame, which the aux-data wrap expects
fn make_atmos(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    for index in 0..FRAMES {
        std::fs::write(dir.join(format!("atmos_{index:05}.bin")), [index as u8; 64]).unwrap();
    }
}

fn cpl_path(dir: &Path) -> PathBuf {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("CPL_"))
        })
        .expect("CPL")
}

fn cpl_id(dir: &Path) -> String {
    dcpwizard_core::multi_cpl::list_cpls(dir)[0].id.clone()
}

struct TrackAsset {
    element: String,
    id: String,
    key_id: Option<String>,
}

fn bare(text: &str) -> String {
    text.trim().trim_start_matches("urn:uuid:").to_string()
}

fn track_assets(dir: &Path) -> Vec<TrackAsset> {
    let xml = std::fs::read_to_string(cpl_path(dir)).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let child = |node: roxmltree::Node, name: &str| {
        node.children()
            .find(|c| c.has_tag_name(name))
            .and_then(|c| c.text())
            .map(bare)
    };
    doc.descendants()
        .filter(|n| TRACK_ELEMENTS.iter().any(|name| n.has_tag_name(*name)))
        .map(|n| TrackAsset {
            element: n.tag_name().name().to_string(),
            id: child(n, "Id").unwrap(),
            key_id: child(n, "KeyId"),
        })
        .collect()
}

fn key_ids_by_asset(dir: &Path) -> BTreeMap<String, String> {
    track_assets(dir)
        .into_iter()
        .filter_map(|asset| Some((asset.id, asset.key_id?)))
        .collect()
}

fn subtitle_mxf(dir: &Path) -> PathBuf {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("subtitle") && n.ends_with(".mxf"))
        })
        .expect("subtitle MXF")
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// returns the KeyId of the new subtitle
fn assert_vf_keys_ov_references(fixture: &Fixture, ov: &Path, vf: &Path) -> String {
    let ov_key_ids = key_ids_by_asset(ov);
    let vf_assets = track_assets(vf);
    for element in ["MainPicture", "MainSound", "AuxData"] {
        let asset = vf_assets
            .iter()
            .find(|a| a.element == element)
            .unwrap_or_else(|| panic!("the VF lists the OV {element}"));
        assert_eq!(
            asset.key_id.as_ref(),
            Some(&ov_key_ids[&asset.id]),
            "the VF {element} carries the OV's KeyId"
        );
    }
    let subtitle = vf_assets
        .iter()
        .find(|a| a.element == "MainSubtitle")
        .expect("the VF lists its subtitle");
    assert!(!ov_key_ids.contains_key(&subtitle.id), "a new subtitle");
    let subtitle_key_id = subtitle.key_id.clone().expect("the subtitle has a KeyId");

    let mut reader = asdcplib::timed_text::MxfReader::new();
    reader
        .open_read(&subtitle_mxf(vf).to_string_lossy())
        .unwrap();
    let info = reader.writer_info().unwrap();
    assert!(info.encrypted_essence, "the subtitle MXF is encrypted");
    assert_eq!(
        uuid::Uuid::from_bytes(info.cryptographic_key_id).to_string(),
        subtitle_key_id,
        "the MXF is encrypted under the KeyId the CPL lists"
    );

    let cpl = std::fs::read_to_string(cpl_path(vf)).unwrap();
    postkit::xmldsig::verify_document_enveloped(&cpl, Some(&fixture.signer().signer_cert))
        .expect("the VF CPL is signed");
    subtitle_key_id
}

fn assert_keys_file_matches_cpl(vf: &Path, keys_file: &Path) {
    let bundle = KeyBundle::read(keys_file).unwrap();
    assert_eq!(bundle.cpl_id, cpl_id(vf), "the keys file names the VF CPL");
    let held: BTreeSet<String> = bundle.keys.iter().map(|k| k.key_id.clone()).collect();
    let listed: BTreeSet<String> = key_ids_by_asset(vf).into_values().collect();
    assert_eq!(held, listed, "one key for every KeyId in the VF CPL");
    assert_eq!(bundle.keys.len(), held.len(), "no KeyId twice");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(keys_file).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "the keys file is owner-only: {mode:o}");
    }
}

// decrypt reads one folder, as a server holding the OV and the VF does
fn ov_and_vf_in_one_folder(ov: &Path, vf: &Path, out: &Path) {
    std::fs::create_dir_all(out).unwrap();
    let mut entries = Vec::new();
    for (dir, take_packing) in [(vf, true), (ov, false)] {
        let assetmap = std::fs::read_to_string(dir.join("ASSETMAP.xml")).unwrap();
        let doc = roxmltree::Document::parse(&assetmap).unwrap();
        for asset in doc.descendants().filter(|n| n.has_tag_name("Asset")) {
            let text = |name: &str| {
                asset
                    .descendants()
                    .find(|c| c.has_tag_name(name))
                    .and_then(|c| c.text())
                    .map(|t| t.trim().to_string())
            };
            let path = text("Path").unwrap();
            let packing_list = text("PackingList").as_deref() == Some("true");
            if !path.ends_with(".mxf") && !take_packing {
                continue;
            }
            std::fs::copy(dir.join(&path), out.join(&path)).unwrap();
            entries.push(dcpwizard_core::assetmap::AssetMapEntry {
                id: bare(&text("Id").unwrap()),
                path,
                packing_list,
            });
        }
    }
    assert_eq!(
        dcpwizard_core::assetmap::generate_assetmap(
            &entries,
            out,
            dcpwizard_core::Standard::Smpte,
            None
        ),
        0
    );
}

// a KDM built from the VF keys file opens every track the VF composition plays
fn assert_kdm_from_vf_keys_decrypts(fixture: &Fixture, ov: &Path, vf: &Path, keys_file: &Path) {
    let (screen, screen_key) = fixture.recipient("screen");
    let kdm = fixture.root.join("vf.kdm.xml");
    fixture.kdm_from_keys_file(&cpl_id(vf), keys_file, &screen, &kdm);

    let together = fixture.root.join("together");
    ov_and_vf_in_one_folder(ov, vf, &together);
    let clear = fixture.root.join("clear");
    fixture
        .command()
        .arg("decrypt")
        .arg("--input")
        .arg(&together)
        .arg("--output")
        .arg(&clear)
        .arg("--kdm")
        .arg(&kdm)
        .arg("--recipient-key")
        .arg(&screen_key)
        .assert()
        .success();

    let picture = std::fs::read_dir(&clear)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("picture") && n.ends_with(".mxf"))
        })
        .expect("decrypted picture");
    let mut reader = asdcplib::jp2k::MxfReader::new();
    reader.open_read(&picture.to_string_lossy()).unwrap();
    let mut buffer = vec![0u8; MAX_FRAME_BUFFER];
    let length = reader.read_frame(0, &mut buffer, None, None).unwrap();
    assert!(
        buffer[..length] == fixture.source_codestream[..],
        "the OV picture decrypts with the VF's KDM"
    );

    let cues = dcpwizard_core::subtitle_extract::extract_track_cues(
        &clear,
        dcpwizard_core::subtitle_extract::PackagedTrack::Subtitle,
    )
    .unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].text, SUBTITLE_TEXT, "the VF subtitle decrypts");
}

#[test]
fn an_encrypted_vf_keeps_the_ov_key_ids_and_its_kdm_opens_ov_and_vf() {
    let fixture = Fixture::build();
    let (ov, ov_keys) = fixture.make_ov(true);
    let vf_keys = fixture.root.join("vf_keys.json");

    fixture
        .create_vf(
            &ov,
            &[
                "--ov-keys".as_ref(),
                ov_keys.as_os_str(),
                "--key-out".as_ref(),
                vf_keys.as_os_str(),
            ],
        )
        .assert()
        .success();
    let vf = fixture.vf_dir();

    assert_vf_keys_ov_references(&fixture, &ov, &vf);
    assert_keys_file_matches_cpl(&vf, &vf_keys);
    assert_kdm_from_vf_keys_decrypts(&fixture, &ov, &vf, &vf_keys);
}

#[test]
fn ov_keys_can_come_from_an_ov_kdm_and_its_private_key() {
    let fixture = Fixture::build();
    let (ov, ov_keys) = fixture.make_ov(true);
    let (facility, facility_key) = fixture.recipient("facility");
    let ov_kdm = fixture.root.join("ov.kdm.xml");
    fixture.kdm_from_keys_file(&cpl_id(&ov), &ov_keys, &facility, &ov_kdm);
    let vf_keys = fixture.root.join("vf_keys.json");

    fixture
        .create_vf(
            &ov,
            &[
                "--ov-kdm".as_ref(),
                ov_kdm.as_os_str(),
                "--ov-key".as_ref(),
                facility_key.as_os_str(),
                "--key-out".as_ref(),
                vf_keys.as_os_str(),
            ],
        )
        .assert()
        .success();
    let vf = fixture.vf_dir();

    assert_vf_keys_ov_references(&fixture, &ov, &vf);
    assert_keys_file_matches_cpl(&vf, &vf_keys);
    assert_kdm_from_vf_keys_decrypts(&fixture, &ov, &vf, &vf_keys);
}

#[test]
fn an_ov_kdm_for_another_recipient_or_another_cpl_is_refused() {
    let fixture = Fixture::build();
    let (ov, ov_keys) = fixture.make_ov(true);
    let (facility, facility_key) = fixture.recipient("facility");
    let (_, other_key) = fixture.recipient("other");
    let ov_kdm = fixture.root.join("ov.kdm.xml");
    fixture.kdm_from_keys_file(&cpl_id(&ov), &ov_keys, &facility, &ov_kdm);

    let other_cpl_kdm = fixture.root.join("other_cpl.kdm.xml");
    let other_cpl_id = uuid::Uuid::new_v4().to_string();
    let ov_content_keys = dcpwizard_core::kdm::load_content_keys(&ov_keys, &cpl_id(&ov)).unwrap();
    assert_eq!(
        dcpwizard_core::kdm::generate_kdm(
            other_cpl_id.clone(),
            "Other".into(),
            facility,
            fixture.chain.join("signer.pem"),
            fixture.chain.join("signer.key"),
            vec![
                fixture.chain.join("intermediate.pem"),
                fixture.chain.join("root.pem"),
            ],
            tomorrow(),
            "2 weeks".into(),
            ov_content_keys,
            other_cpl_kdm.clone(),
            None,
            None,
            Vec::new(),
            Default::default(),
        ),
        0
    );

    let vf_keys = fixture.root.join("vf_keys.json");
    for (kdm, private_key, reason) in [
        (&ov_kdm, &other_key, "does not decrypt with the private key"),
        (&other_cpl_kdm, &facility_key, "is for CPL"),
    ] {
        let output = fixture
            .create_vf(
                &ov,
                &[
                    "--ov-kdm".as_ref(),
                    kdm.as_os_str(),
                    "--ov-key".as_ref(),
                    private_key.as_os_str(),
                    "--key-out".as_ref(),
                    vf_keys.as_os_str(),
                ],
            )
            .output()
            .unwrap();
        let text = output_text(&output);
        assert!(!output.status.success(), "{text}");
        assert!(text.contains(reason), "names why: {text}");
        assert!(!vf_keys.exists(), "no keys file is written");
        assert!(!fixture.vf_written(), "no VF is written");
    }
}

#[test]
fn without_ov_keys_the_vf_keys_file_holds_its_new_key_and_names_the_missing_ones() {
    let fixture = Fixture::build();
    let (ov, _) = fixture.make_ov(true);
    let vf_keys = fixture.root.join("vf_keys.json");

    let output = fixture
        .create_vf(&ov, &["--key-out".as_ref(), vf_keys.as_os_str()])
        .output()
        .unwrap();
    let text = output_text(&output);
    assert!(output.status.success(), "{text}");
    let vf = fixture.vf_dir();

    let subtitle_key_id = assert_vf_keys_ov_references(&fixture, &ov, &vf);
    let bundle = KeyBundle::read(&vf_keys).unwrap();
    let held: Vec<&str> = bundle.keys.iter().map(|k| k.key_id.as_str()).collect();
    assert_eq!(held, [subtitle_key_id.as_str()], "only the new key");
    for ov_key_id in key_ids_by_asset(&ov).values() {
        assert!(
            text.contains(ov_key_id.as_str()),
            "names {ov_key_id}: {text}"
        );
    }
}

#[test]
fn the_vf_references_the_ov_atmos_track_with_its_key_id() {
    let fixture = Fixture::build();
    let (ov, ov_keys) = fixture.make_ov(true);
    let vf_keys = fixture.root.join("vf_keys.json");

    fixture
        .create_vf(
            &ov,
            &[
                "--ov-keys".as_ref(),
                ov_keys.as_os_str(),
                "--key-out".as_ref(),
                vf_keys.as_os_str(),
            ],
        )
        .assert()
        .success();
    let vf = fixture.vf_dir();

    let aux = |dir: &Path| {
        track_assets(dir)
            .into_iter()
            .find(|a| a.element == "AuxData")
            .expect("an AuxData track")
    };
    let ov_aux = aux(&ov);
    let vf_aux = aux(&vf);
    assert_eq!(vf_aux.id, ov_aux.id, "the VF points at the OV's Atmos");
    assert!(ov_aux.key_id.is_some(), "the OV Atmos is encrypted");
    assert_eq!(vf_aux.key_id, ov_aux.key_id, "under the OV's KeyId");
    let atmos_key = KeyBundle::read(&vf_keys)
        .unwrap()
        .keys
        .into_iter()
        .find(|k| Some(&k.key_id) == ov_aux.key_id.as_ref())
        .expect("the VF keys file holds the Atmos key");
    assert_eq!(atmos_key.key_type, dcpwizard_core::encrypt::KeyType::Mdek);
}

#[test]
fn the_vf_keeps_the_ov_markers_and_rating() {
    let fixture = Fixture::build();
    let (ov, _) = fixture.make_ov(false);

    fixture.create_unsigned_vf(&ov, &[]).assert().success();
    let vf = fixture.vf_dir();

    let markers = |dir: &Path| -> Vec<(String, u64)> {
        let xml = std::fs::read_to_string(cpl_path(dir)).unwrap();
        dcpwizard_core::markers::markers_from_cpl(&xml)
            .into_iter()
            .flatten()
            .map(|m| (m.marker.label().to_string(), m.frame))
            .collect()
    };
    let ov_markers = markers(&ov);
    assert_eq!(
        ov_markers.len(),
        OV_MARKERS.len(),
        "the OV carries its markers"
    );
    assert_eq!(markers(&vf), ov_markers, "the VF carries the same markers");

    let vf_cpl = std::fs::read_to_string(cpl_path(&vf)).unwrap();
    assert!(
        vf_cpl.contains(&format!("<Agency>{OV_RATING_AGENCY}</Agency>"))
            && vf_cpl.contains(&format!("<Label>{OV_RATING_LABEL}</Label>")),
        "the VF carries the OV's rating:\n{vf_cpl}"
    );
}

#[test]
fn an_encrypted_vf_needs_a_signer() {
    let fixture = Fixture::build();
    let (ov, ov_keys) = fixture.make_ov(true);

    let vf_keys = fixture.root.join("vf_keys.json");
    let output = fixture
        .create_unsigned_vf(
            &ov,
            &[
                "--ov-keys".as_ref(),
                ov_keys.as_os_str(),
                "--key-out".as_ref(),
                vf_keys.as_os_str(),
            ],
        )
        .output()
        .unwrap();
    let text = output_text(&output);
    assert!(!output.status.success(), "{text}");
    assert!(
        text.contains(dcpwizard_core::preflight::ENCRYPTED_PACKAGE_NEEDS_SIGNER),
        "{text}"
    );
    assert!(!fixture.vf_written(), "no VF is written");
}
