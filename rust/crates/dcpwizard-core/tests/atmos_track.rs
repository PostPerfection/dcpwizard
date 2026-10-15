use dcpwizard_core::dcp::{DcpConfig, create_dcp};
use postkit::certificate::generate_chain;
use std::path::{Path, PathBuf};

const FPS: u32 = 24;
const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1080;
const FRAMES: usize = 60;
// reels of 24 and 36 frames, both at least the one second a reel needs
const SPLIT_AT_FRAME: u64 = 24;
const LARGEST_TEST_FRAME: usize = 64 * 1024;

fn make_picture_frames(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let seed = dir.join("seed.j2c");
    dcpwizard_core::pad::generate_black_frame(WIDTH, HEIGHT, FPS, &seed).expect("encode frame");
    for i in 0..FRAMES {
        std::fs::copy(&seed, dir.join(format!("frame_{i:05}.j2c"))).unwrap();
    }
    std::fs::remove_file(&seed).unwrap();
    dir.to_path_buf()
}

// every frame has its own length and bytes, so a dropped, repeated or reordered frame shows
fn atmos_frame(index: usize) -> Vec<u8> {
    (0..100 + index * 7)
        .map(|byte| (index * 31 + byte) as u8)
        .collect()
}

fn make_atmos_track_file(
    root: &Path,
    fps: u32,
    encryption: Option<postkit::mxf_wrap::MxfEncryption>,
) -> PathBuf {
    let frames_dir = root.join("atmos_frames");
    std::fs::create_dir_all(&frames_dir).unwrap();
    let files: Vec<PathBuf> = (0..FRAMES)
        .map(|index| {
            let path = frames_dir.join(format!("atmos_{index:05}.bin"));
            std::fs::write(&path, atmos_frame(index)).unwrap();
            path
        })
        .collect();
    let track_file = root.join("source_atmos.mxf");
    dcpwizard_core::mxf_wrap::wrap_mxf_files(
        files,
        &track_file,
        dcpwizard_core::mxf_wrap::MxfType::Atmos,
        fps,
        encryption,
        None,
        None,
    )
    .expect("wrap the source Atmos track file");
    track_file
}

fn read_descriptor(path: &Path) -> asdcplib::atmos::AtmosDescriptor {
    let mut reader = asdcplib::atmos::MxfReader::new();
    reader.open_read(&path.to_string_lossy()).unwrap();
    reader.atmos_descriptor().unwrap()
}

fn split_config(root: &Path, out: &Path, atmos: PathBuf) -> DcpConfig {
    DcpConfig {
        title: "Atmos Reels".into(),
        standard: dcpwizard_core::Standard::Smpte,
        resolution: dcpwizard_core::Resolution::TwoK,
        content_type: dcpwizard_core::ContentType::Test,
        frame_rate_num: FPS,
        frame_rate_den: 1,
        output_dir: out.to_path_buf(),
        j2k_dir: Some(make_picture_frames(&root.join("frames"))),
        atmos_path: Some(atmos),
        reel_split_frames: vec![SPLIT_AT_FRAME],
        ..Default::default()
    }
}

struct ReelAux {
    picture_duration: u64,
    aux_id: String,
    aux_duration: u64,
    aux_key_id: Option<String>,
}

fn child_text<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.children()
        .find(|child| child.tag_name().name() == name)
        .and_then(|child| child.text())
}

fn bare_uuid(urn: &str) -> String {
    urn.trim_start_matches("urn:uuid:").to_string()
}

fn cpl_reel_aux(out: &Path) -> Vec<ReelAux> {
    let cpl = std::fs::read_dir(out)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("CPL_"))
        })
        .expect("a CPL");
    let text = std::fs::read_to_string(cpl).unwrap();
    let document = roxmltree::Document::parse(&text).expect("parse CPL");
    document
        .descendants()
        .filter(|node| node.tag_name().name() == "Reel")
        .map(|reel| {
            let asset = |name: &str| {
                reel.descendants()
                    .find(|node| node.tag_name().name() == name)
                    .unwrap_or_else(|| panic!("reel has a {name}"))
            };
            let picture = asset("MainPicture");
            let aux = asset("AuxData");
            ReelAux {
                picture_duration: child_text(picture, "Duration").unwrap().parse().unwrap(),
                aux_id: bare_uuid(child_text(aux, "Id").unwrap()),
                aux_duration: child_text(aux, "Duration").unwrap().parse().unwrap(),
                aux_key_id: child_text(aux, "KeyId").map(bare_uuid),
            }
        })
        .collect()
}

fn hex_key(hex: &str) -> [u8; 16] {
    let mut key = [0u8; 16];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap();
    }
    key
}

fn read_frames(path: &Path, content_key: Option<[u8; 16]>) -> Vec<Vec<u8>> {
    let mut reader = asdcplib::atmos::MxfReader::new();
    reader.open_read(&path.to_string_lossy()).unwrap();
    let frame_count = reader.atmos_descriptor().unwrap().container_duration;
    let mut contexts = content_key.map(|key| {
        let mut cipher = asdcplib::crypto::AesDecContext::new();
        cipher.init_key(&key).unwrap();
        let mut hmac = asdcplib::crypto::HmacContext::new();
        hmac.init_key(&key, asdcplib::LabelSet::Smpte).unwrap();
        (cipher, hmac)
    });
    let mut buffer = vec![0u8; LARGEST_TEST_FRAME];
    (0..frame_count)
        .map(|frame_number| {
            let (cipher, hmac) = match contexts.as_mut() {
                Some((cipher, hmac)) => (Some(cipher), Some(hmac)),
                None => (None, None),
            };
            let length = reader
                .read_frame(frame_number, &mut buffer, cipher, hmac)
                .unwrap();
            buffer[..length].to_vec()
        })
        .collect()
}

fn signer(root: &Path) -> dcpwizard_core::package_signature::PackageSigner {
    let chain = root.join("chain");
    assert_eq!(generate_chain("Acme", &chain), 0, "chain gen");
    dcpwizard_core::package_signature::PackageSigner {
        signer_cert: chain.join("signer.pem"),
        signer_key: chain.join("signer.key"),
        signer_chain: vec![chain.join("intermediate.pem"), chain.join("root.pem")],
    }
}

fn check_split_atmos_track_file(encrypt: bool) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let out = root.join("dcp");
    let keys_file = root.join("KEYS.json");
    let source = make_atmos_track_file(root, FPS, None);
    let source_descriptor = read_descriptor(&source);
    let config = DcpConfig {
        encrypt,
        key_out: encrypt.then(|| keys_file.clone()),
        signer: encrypt.then(|| signer(root)),
        ..split_config(root, &out, source)
    };
    assert_eq!(
        create_dcp(&config),
        0,
        "a split DCP with an Atmos track file"
    );

    let reels = cpl_reel_aux(&out);
    assert_eq!(reels.len(), 2, "two reels");
    let atmos_files = std::fs::read_dir(&out)
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("atmos_"))
        .count();
    assert_eq!(atmos_files, 2, "one Atmos MXF per reel");
    let keys = encrypt.then(|| dcpwizard_core::encrypt::KeyBundle::read(&keys_file).unwrap());

    let mut next_source_frame = 0;
    for (index, reel) in reels.iter().enumerate() {
        assert_eq!(
            reel.aux_duration, reel.picture_duration,
            "reel {index} AuxData duration"
        );
        let track_file = out.join(format!("atmos_{}.mxf", reel.aux_id));
        let descriptor = read_descriptor(&track_file);
        assert_eq!(
            u64::from(descriptor.container_duration),
            reel.picture_duration,
            "reel {index} Atmos frame count"
        );
        assert_eq!(descriptor.atmos_id, source_descriptor.atmos_id);
        assert_eq!(descriptor.atmos_version, source_descriptor.atmos_version);
        assert_eq!(
            descriptor.max_channel_count,
            source_descriptor.max_channel_count
        );
        assert_eq!(
            descriptor.max_object_count,
            source_descriptor.max_object_count
        );
        let mut reader = asdcplib::atmos::MxfReader::new();
        reader.open_read(&track_file.to_string_lossy()).unwrap();
        assert_eq!(
            *uuid::Uuid::parse_str(&reel.aux_id).unwrap().as_bytes(),
            reader.writer_info().unwrap().asset_uuid
        );

        let content_key = keys.as_ref().map(|bundle| {
            let key_id = reel
                .aux_key_id
                .as_ref()
                .unwrap_or_else(|| panic!("reel {index} AuxData has a KeyId"));
            let key = bundle
                .keys
                .iter()
                .find(|key| &key.key_id == key_id)
                .unwrap_or_else(|| panic!("the keys file covers reel {index}'s Atmos KeyId"));
            assert_eq!(key.asset_uuid, reel.aux_id);
            hex_key(&key.content_key_hex)
        });
        if !encrypt {
            assert_eq!(reel.aux_key_id, None, "a cleartext reel has no KeyId");
        }
        for frame in read_frames(&track_file, content_key) {
            assert_eq!(
                frame,
                atmos_frame(next_source_frame),
                "reel {index} carries source frame {next_source_frame}"
            );
            next_source_frame += 1;
        }
    }
    assert_eq!(
        next_source_frame, FRAMES,
        "every source frame is packaged once"
    );
    if let Some(keys) = keys {
        let reel_key_ids: Vec<_> = reels.iter().map(|reel| reel.aux_key_id.clone()).collect();
        assert_ne!(
            reel_key_ids[0], reel_key_ids[1],
            "each reel has its own key"
        );
        assert!(keys.keys.len() >= 2);
    }

    let result = dcpwizard_core::verify::verify_dcp(&out);
    assert!(result.valid, "dcpdoctor errors: {:?}", result.errors);
}

#[test]
fn an_atmos_track_file_is_rewrapped_into_one_track_per_reel() {
    check_split_atmos_track_file(false);
}

#[test]
fn an_encrypted_split_dcp_keys_each_reels_atmos_track() {
    check_split_atmos_track_file(true);
}

#[test]
fn an_atmos_frame_directory_splits_into_one_track_per_reel() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let out = root.join("dcp");
    let frames_dir = root.join("atmos_frames");
    std::fs::create_dir_all(&frames_dir).unwrap();
    for index in 0..FRAMES {
        std::fs::write(
            frames_dir.join(format!("atmos_{index:05}.bin")),
            atmos_frame(index),
        )
        .unwrap();
    }
    assert_eq!(create_dcp(&split_config(root, &out, frames_dir)), 0);

    let reels = cpl_reel_aux(&out);
    assert_eq!(reels.len(), 2, "two reels");
    let mut next_source_frame = 0;
    for (index, reel) in reels.iter().enumerate() {
        assert_eq!(reel.aux_duration, reel.picture_duration, "reel {index}");
        for frame in read_frames(&out.join(format!("atmos_{}.mxf", reel.aux_id)), None) {
            assert_eq!(frame, atmos_frame(next_source_frame));
            next_source_frame += 1;
        }
    }
    assert_eq!(next_source_frame, FRAMES);
}
