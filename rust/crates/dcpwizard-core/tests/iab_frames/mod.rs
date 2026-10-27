use std::io::Read;
use std::path::Path;

// 48 frames at 24 fps that postkit's IAB checker rates valid under every DCP constraint set
pub fn validation_stream_1_frames() -> Vec<Vec<u8>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../extern/postkit/tests/fixtures/iab/IABValidationStream_1.iab.zip");
    let file =
        std::fs::File::open(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut stream = Vec::new();
    archive
        .by_index(0)
        .unwrap()
        .read_to_end(&mut stream)
        .unwrap();
    let frames: Vec<Vec<u8>> = postkit::iab::split_frames(&stream)
        .unwrap()
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect();
    let neighbours_differ =
        (0..frames.len()).all(|index| frames[index] != frames[(index + 1) % frames.len()]);
    assert!(
        frames.len() > 1 && neighbours_differ,
        "no frame equals the next one, the last and first included"
    );
    frames
}
