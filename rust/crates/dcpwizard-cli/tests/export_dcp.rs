// `export` over a DCP built from one flat colour: the screener has to come back the colour of
// the master, carry the raster and frame count of the DCP, and hold PCM under ProRes.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tempfile::TempDir;

const SOURCE_COLOUR: &str = "0x5a8f3c";
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const FRAME_RATE: u32 = 24;
const DURATION_SECONDS: f64 = 0.5;
const FRAMES: u32 = 12;

// how far the export may sit from the DCP picture it was made from, the only drift the export
// itself owns: a matrix or transfer mistake moves a channel by 4 percent or more
const PICTURE_TOLERANCE: f64 = 0.03;

// the master to X'Y'Z' to J2K trip costs 3.9 percent per channel on this fixture before the
// export runs at all, so the whole chain gets the wider bound
const MASTER_TOLERANCE: f64 = 0.06;

const SIXTEEN_BIT_PER_EIGHT_BIT: f64 = 257.0;

struct Fixture {
    directory: TempDir,
    config_home: TempDir,
    master: PathBuf,
    tone: PathBuf,
    picture_mxf: PathBuf,
    sound_mxf: PathBuf,
}

fn ffmpeg() -> std::process::Command {
    let mut command = std::process::Command::new("ffmpeg");
    command.args(["-hide_banner", "-v", "error", "-y"]);
    command
}

fn run(command: &mut std::process::Command, what: &str) {
    let output = command.output().unwrap_or_else(|e| panic!("{what}: {e}"));
    assert!(
        output.status.success(),
        "{what}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn only_file_starting_with(directory: &Path, prefix: &str) -> PathBuf {
    let mut found: Vec<PathBuf> = std::fs::read_dir(directory)
        .expect("the package directory has to be readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix))
        })
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "one {prefix}* in {}", directory.display());
    found.remove(0)
}

fn dcp_fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(build_dcp)
}

fn build_dcp() -> Fixture {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    let master = directory.path().join("master.mkv");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "color=c={SOURCE_COLOUR}:size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}:duration={DURATION_SECONDS}"
            ))
            .args(["-c:v", "ffv1", "-pix_fmt", "gbrp"])
            .arg(&master),
        "ffmpeg must be installed to write the flat colour master",
    );

    let wav = directory.path().join("tone.wav");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:duration={DURATION_SECONDS}:sample_rate=48000"
            ))
            .args(["-ac", "2", "-c:a", "pcm_s24le"])
            .arg(&wav),
        "the tone has to be written",
    );

    let package = create_dcp(
        config_home.path(),
        &master,
        &wav,
        &directory.path().join("dcp"),
        "Export Colour",
        &[],
    );
    let picture_mxf = only_file_starting_with(&package, "picture_");
    let sound_mxf = only_file_starting_with(&package, "sound_");
    Fixture {
        directory,
        config_home,
        master,
        tone: wav,
        picture_mxf,
        sound_mxf,
    }
}

fn create_dcp(
    config_home: &Path,
    master: &Path,
    wav: &Path,
    output: &Path,
    title: &str,
    extra: &[&str],
) -> PathBuf {
    Command::cargo_bin("dcpwizard")
        .unwrap()
        .env("XDG_CONFIG_HOME", config_home)
        .args(["create", "--title", title, "--video"])
        .arg(master)
        .arg("--audio")
        .arg(wav)
        .arg("-o")
        .arg(output)
        // the export is compared pixel for pixel with the master
        .args(["--container-dims", &format!("{WIDTH}x{HEIGHT}")])
        .args(extra)
        .assert()
        .success();
    output.join(title)
}

fn export_command(config_home: &Path, input: &Path, output: &Path) -> Command {
    let mut command = Command::cargo_bin("dcpwizard").unwrap();
    command
        .env("XDG_CONFIG_HOME", config_home)
        .arg("export")
        .arg("--input")
        .arg(input)
        .arg("-o")
        .arg(output);
    command
}

fn export(fixture: &Fixture, format: &str, output: &Path) {
    export_command(fixture.config_home.path(), &fixture.picture_mxf, output)
        .arg("--audio")
        .arg(&fixture.sound_mxf)
        .args(["--format", format])
        .assert()
        .success();
}

const ENCRYPTED_TITLE: &str = "Export Encrypted";

struct EncryptedFixture {
    _directory: TempDir,
    chain: PathBuf,
    package: PathBuf,
    keys: PathBuf,
}

fn encrypted_fixture() -> &'static EncryptedFixture {
    static FIXTURE: OnceLock<EncryptedFixture> = OnceLock::new();
    FIXTURE.get_or_init(build_encrypted_dcp)
}

// the same master and tone as the cleartext fixture
fn build_encrypted_dcp() -> EncryptedFixture {
    let fixture = dcp_fixture();
    let directory = TempDir::new().unwrap();
    // an encrypted package is only conformant with a signed CPL and PKL
    let chain = directory.path().join("chain");
    assert_eq!(
        postkit::certificate::generate_chain("Export Test", &chain),
        0,
        "signer chain"
    );
    let keys = directory.path().join("KEYS.json");
    let package = create_dcp(
        fixture.config_home.path(),
        &fixture.master,
        &fixture.tone,
        &directory.path().join("dcp"),
        ENCRYPTED_TITLE,
        &[
            "--encrypt",
            "--key-out",
            keys.to_str().unwrap(),
            "--signer-cert",
            chain.join("signer.pem").to_str().unwrap(),
            "--signer-key",
            chain.join("signer.key").to_str().unwrap(),
            "--signer-chain",
            chain.join("intermediate.pem").to_str().unwrap(),
            "--signer-chain",
            chain.join("root.pem").to_str().unwrap(),
        ],
    );
    let mut picture = asdcplib::jp2k::MxfReader::new();
    picture
        .open_read(&only_file_starting_with(&package, "picture_").to_string_lossy())
        .unwrap();
    let mut sound = asdcplib::pcm::MxfReader::new();
    sound
        .open_read(&only_file_starting_with(&package, "sound_").to_string_lossy())
        .unwrap();
    assert!(
        picture.writer_info().unwrap().encrypted_essence
            && sound.writer_info().unwrap().encrypted_essence,
        "the picture and sound have to be encrypted for the key path to mean anything"
    );
    EncryptedFixture {
        _directory: directory,
        chain,
        package,
        keys,
    }
}

fn only_cpl(package: &Path) -> (String, PathBuf) {
    let cpls = dcpwizard_core::multi_cpl::list_cpls(package);
    assert_eq!(cpls.len(), 1, "one CPL in {}", package.display());
    (cpls[0].id.clone(), package.join(&cpls[0].file_path))
}

// a KDM starting on the day its signer certificate does is refused
fn tomorrow() -> String {
    (chrono::Utc::now() + chrono::Duration::days(1))
        .format("%Y-%m-%dT%H:%M:%S+00:00")
        .to_string()
}

fn assert_prores_export_holds_the_fixture(output: &Path, what: &str) {
    let fixture = dcp_fixture();
    assert_eq!(
        probe(output, "v:0", "nb_frames"),
        [FRAMES.to_string()],
        "{what} has to carry every frame of the DCP"
    );
    assert_eq!(probe(output, "a:0", "codec_name"), ["pcm_s24le"]);
    let exported = mean_rgb(output);
    assert_colour_within(
        exported,
        mean_rgb(&fixture.picture_mxf),
        PICTURE_TOLERANCE,
        &format!("{what} against the DCP picture"),
    );
    assert_colour_within(
        exported,
        mean_rgb(&fixture.master),
        MASTER_TOLERANCE,
        &format!("{what} against the master"),
    );
}

fn probe(file: &Path, stream: &str, entries: &str) -> Vec<String> {
    let output = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", stream])
        .arg("-show_entries")
        .arg(format!("stream={entries}"))
        .args(["-of", "default=nk=1:nw=1"])
        .arg(file)
        .output()
        .expect("ffprobe must be installed");
    assert!(
        output.status.success(),
        "ffprobe failed on {}: {}",
        file.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    // ffprobe 9 on macos prints the stream section twice, so keep one value per entry
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .take(entries.split(',').count())
        .collect()
}

// the mean of the first frame in RGB. A player reads an HD file as Rec.709, so the readback
// forces that matrix instead of trusting swscale's unspecified-means-601 default, and the
// accurate flags keep swscale's fast 8-bit path from moving red by 3 codes on its own.
fn mean_rgb(file: &Path) -> [f64; 3] {
    let output = ffmpeg()
        .arg("-i")
        .arg(file)
        .args(["-frames:v", "1"])
        .args(["-sws_flags", "+accurate_rnd+full_chroma_int"])
        .args(["-vf", "scale=in_color_matrix=bt709,format=rgb48le"])
        .args(["-f", "rawvideo", "-"])
        .output()
        .expect("ffmpeg must be installed");
    assert!(
        output.status.success(),
        "could not decode {}: {}",
        file.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let samples: Vec<u16> = output
        .stdout
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    assert_eq!(
        samples.len(),
        (WIDTH * HEIGHT * 3) as usize,
        "a full {WIDTH}x{HEIGHT} frame has to come back from {}",
        file.display()
    );
    let pixels = (samples.len() / 3) as f64;
    [0, 1, 2].map(|channel| {
        samples
            .iter()
            .skip(channel)
            .step_by(3)
            .map(|&s| s as f64)
            .sum::<f64>()
            / pixels
            / SIXTEEN_BIT_PER_EIGHT_BIT
    })
}

fn assert_colour_within(measured: [f64; 3], reference: [f64; 3], tolerance: f64, what: &str) {
    for channel in 0..3 {
        let drift = (measured[channel] - reference[channel]).abs() / reference[channel];
        assert!(
            drift <= tolerance,
            "{what}: channel {channel} reads {:.2} against {:.2}, {:.1} percent off, over the {:.1} percent allowed. \
             Measured RGB {measured:.2?} against {reference:.2?}",
            measured[channel],
            reference[channel],
            drift * 100.0,
            tolerance * 100.0
        );
    }
}

fn assert_rec709_tags(file: &Path) {
    assert_eq!(
        probe(
            file,
            "v:0",
            "color_range,color_space,color_transfer,color_primaries"
        ),
        ["tv", "bt709", "bt709", "bt709"],
        "the export has to name the colour a player must assume, or it reads the picture as BT.601"
    );
}

#[test]
fn a_prores_export_holds_the_dcp_colour_and_pcm_audio() {
    let fixture = dcp_fixture();
    let output = fixture.directory.path().join("screener.mov");
    export(fixture, "prores", &output);

    assert_eq!(
        probe(&output, "v:0", "codec_name,profile"),
        ["prores", "HQ"]
    );
    assert_eq!(
        probe(&output, "v:0", "width,height,nb_frames"),
        [WIDTH.to_string(), HEIGHT.to_string(), FRAMES.to_string()]
    );
    // 422 HQ is a 4:2:2 codec, so a 4:4:4 frame under the apch tag is not a ProRes a grade reads
    assert_eq!(probe(&output, "v:0", "pix_fmt"), ["yuv422p10le"]);
    assert_eq!(probe(&output, "a:0", "codec_name"), ["pcm_s24le"]);
    assert_rec709_tags(&output);

    let exported = mean_rgb(&output);
    assert_colour_within(
        exported,
        mean_rgb(&fixture.picture_mxf),
        PICTURE_TOLERANCE,
        "the ProRes export against the DCP picture",
    );
    assert_colour_within(
        exported,
        mean_rgb(&fixture.master),
        MASTER_TOLERANCE,
        "the ProRes export against the master",
    );
}

#[test]
fn an_h264_export_holds_the_dcp_colour_and_aac_audio() {
    let fixture = dcp_fixture();
    let output = fixture.directory.path().join("screener.mp4");
    export(fixture, "h264", &output);

    assert_eq!(
        probe(&output, "v:0", "codec_name,profile"),
        ["h264", "High"]
    );
    assert_eq!(
        probe(&output, "v:0", "width,height,nb_frames"),
        [WIDTH.to_string(), HEIGHT.to_string(), FRAMES.to_string()]
    );
    // 8-bit 4:2:0 plays everywhere a screener is opened, High 4:4:4 10-bit does not
    assert_eq!(probe(&output, "v:0", "pix_fmt"), ["yuv420p"]);
    assert_eq!(probe(&output, "a:0", "codec_name"), ["aac"]);
    assert_rec709_tags(&output);

    let exported = mean_rgb(&output);
    assert_colour_within(
        exported,
        mean_rgb(&fixture.picture_mxf),
        PICTURE_TOLERANCE,
        "the H.264 export against the DCP picture",
    );
    assert_colour_within(
        exported,
        mean_rgb(&fixture.master),
        MASTER_TOLERANCE,
        "the H.264 export against the master",
    );
}

#[test]
fn an_h265_export_holds_the_dcp_colour_and_aac_audio() {
    let fixture = dcp_fixture();
    let output = fixture.directory.path().join("screener_h265.mp4");
    export(fixture, "h265", &output);

    assert_eq!(probe(&output, "v:0", "codec_name"), ["hevc"]);
    assert_eq!(
        probe(&output, "v:0", "width,height,nb_frames"),
        [WIDTH.to_string(), HEIGHT.to_string(), FRAMES.to_string()]
    );
    assert_eq!(probe(&output, "v:0", "pix_fmt"), ["yuv420p"]);
    assert_eq!(probe(&output, "a:0", "codec_name"), ["aac"]);
    assert_rec709_tags(&output);

    let exported = mean_rgb(&output);
    assert_colour_within(
        exported,
        mean_rgb(&fixture.picture_mxf),
        PICTURE_TOLERANCE,
        "the H.265 export against the DCP picture",
    );
    assert_colour_within(
        exported,
        mean_rgb(&fixture.master),
        MASTER_TOLERANCE,
        "the H.265 export against the master",
    );
}

#[test]
fn a_dnxhr_export_holds_the_dcp_colour_and_pcm_audio() {
    let fixture = dcp_fixture();
    let output = fixture.directory.path().join("screener.mxf");
    export(fixture, "dnxhr", &output);

    assert_eq!(probe(&output, "v:0", "codec_name"), ["dnxhd"]);
    assert_eq!(
        probe(&output, "v:0", "width,height"),
        [WIDTH.to_string(), HEIGHT.to_string()]
    );
    // DNxHR HQ is 4:2:2 8-bit, which is the layout a master for approval carries
    assert_eq!(probe(&output, "v:0", "pix_fmt"), ["yuv422p"]);
    assert_eq!(probe(&output, "a:0", "codec_name"), ["pcm_s24le"]);
    assert_rec709_tags(&output);

    let exported = mean_rgb(&output);
    assert_colour_within(
        exported,
        mean_rgb(&fixture.picture_mxf),
        PICTURE_TOLERANCE,
        "the DNxHR export against the DCP picture",
    );
    assert_colour_within(
        exported,
        mean_rgb(&fixture.master),
        MASTER_TOLERANCE,
        "the DNxHR export against the master",
    );
}

#[test]
fn an_image_sequence_export_writes_a_readable_png_per_frame() {
    let fixture = dcp_fixture();
    let output = fixture.directory.path().join("stills");
    export(fixture, "image-sequence", &output);

    let mut frames: Vec<PathBuf> = std::fs::read_dir(&output)
        .expect("the sequence directory has to be readable")
        .flatten()
        .map(|entry| entry.path())
        .collect();
    frames.sort();
    assert_eq!(
        frames.len(),
        FRAMES as usize,
        "one still per packaged frame in {}",
        output.display()
    );

    let first = &frames[0];
    assert_eq!(probe(first, "v:0", "codec_name"), ["png"]);
    assert_eq!(probe(first, "v:0", "pix_fmt"), ["rgb48be"]);
    assert_eq!(
        probe(first, "v:0", "width,height"),
        [WIDTH.to_string(), HEIGHT.to_string()]
    );
    assert_colour_within(
        mean_rgb(first),
        mean_rgb(&fixture.picture_mxf),
        PICTURE_TOLERANCE,
        "the first still against the DCP picture",
    );
}

#[test]
fn exporting_something_that_is_not_a_track_file_says_so() {
    let directory = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let not_a_dcp = directory.path().join("holiday_photos");
    std::fs::create_dir(&not_a_dcp).unwrap();

    Command::cargo_bin("dcpwizard")
        .unwrap()
        .env("XDG_CONFIG_HOME", config_home.path())
        .args([
            "export",
            "--input",
            not_a_dcp.to_str().unwrap(),
            "-o",
            directory.path().join("out.mp4").to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("holiday_photos"));
}

// the decrypted sound sits beside the output only while the export runs
fn assert_only_entries(directory: &Path, expected: &[&Path]) {
    let mut found: Vec<PathBuf> = std::fs::read_dir(directory)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    found.sort();
    let mut expected: Vec<PathBuf> = expected.iter().map(|path| path.to_path_buf()).collect();
    expected.sort();
    assert_eq!(
        found,
        expected,
        "what the export left in {}",
        directory.display()
    );
}

#[test]
fn an_encrypted_dcp_exports_under_its_keys_file() {
    let encrypted = encrypted_fixture();
    let directory = TempDir::new().unwrap();
    let output = directory.path().join("keys_screener.mov");
    export_command(
        dcp_fixture().config_home.path(),
        &encrypted.package,
        &output,
    )
    .arg("--keys")
    .arg(&encrypted.keys)
    .args(["--format", "prores"])
    .assert()
    .success();

    assert_prores_export_holds_the_fixture(&output, "the ProRes export of the encrypted DCP");
    assert_only_entries(directory.path(), &[&output]);
}

// one 24 fps edit unit of the stereo tone is 12000 bytes
const PCM_FRAME_BUFFER_BYTES: usize = 1 << 20;

fn pcm_frames(sound_mxf: &Path) -> Vec<Vec<u8>> {
    let mut reader = asdcplib::pcm::MxfReader::new();
    reader.open_read(&sound_mxf.to_string_lossy()).unwrap();
    let frame_count = reader.audio_descriptor().unwrap().container_duration;
    let mut buffer = vec![0u8; PCM_FRAME_BUFFER_BYTES];
    (0..frame_count)
        .map(|frame| {
            let length = reader.read_frame(frame, &mut buffer, None, None).unwrap();
            buffer[..length].to_vec()
        })
        .collect()
}

#[test]
fn a_decrypted_sound_mxf_holds_the_cleartext_pcm() {
    let encrypted = encrypted_fixture();
    let directory = TempDir::new().unwrap();
    let output = directory.path().join("decrypted");
    Command::cargo_bin("dcpwizard")
        .unwrap()
        .env("XDG_CONFIG_HOME", dcp_fixture().config_home.path())
        .arg("decrypt")
        .arg("--input")
        .arg(&encrypted.package)
        .arg("--output")
        .arg(&output)
        .arg("--keys")
        .arg(&encrypted.keys)
        .assert()
        .success();

    let cleartext = pcm_frames(&dcp_fixture().sound_mxf);
    assert_eq!(cleartext.len(), FRAMES as usize);
    assert!(
        pcm_frames(&only_file_starting_with(&output, "sound_")) == cleartext,
        "the decrypted sound has to hold the cleartext package's PCM frame for frame"
    );
}

#[test]
fn an_encrypted_dcp_exports_under_a_kdm() {
    let encrypted = encrypted_fixture();
    let directory = TempDir::new().unwrap();
    let recipient = directory.path().join("screen.pem");
    let recipient_key = directory.path().join("screen.key");
    let options = postkit::certificate::CertOptions {
        cert_type: postkit::certificate::CertType::Leaf,
        common_name: "screen".into(),
        organization: "Cinema".into(),
        output_cert: recipient.clone(),
        output_key: recipient_key.clone(),
        issuer_cert: encrypted.chain.join("root.pem"),
        issuer_key: encrypted.chain.join("root.key"),
        ..Default::default()
    };
    assert_eq!(
        postkit::certificate::generate_certificate(&options),
        0,
        "recipient certificate"
    );

    let (cpl_id, _) = only_cpl(&encrypted.package);
    let kdm = directory.path().join("kdm.xml");
    let history = directory.path().join("kdm-history.log");
    Command::cargo_bin("dcpwizard")
        .unwrap()
        .env("XDG_CONFIG_HOME", directory.path().join("config"))
        .env("XDG_DATA_HOME", directory.path().join("data"))
        .args([
            "kdm",
            "--history-file",
            history.to_str().unwrap(),
            "--cpl-id",
            &cpl_id,
            "--content-title",
            ENCRYPTED_TITLE,
        ])
        .args(["--valid-from", &tomorrow(), "--valid-to", "2 weeks"])
        .arg("--cert")
        .arg(&recipient)
        .arg("--signer-cert")
        .arg(encrypted.chain.join("signer.pem"))
        .arg("--signer-key")
        .arg(encrypted.chain.join("signer.key"))
        .arg("--signer-chain")
        .arg(encrypted.chain.join("intermediate.pem"))
        .arg("--signer-chain")
        .arg(encrypted.chain.join("root.pem"))
        .arg("--keys")
        .arg(&encrypted.keys)
        .arg("--output")
        .arg(&kdm)
        .assert()
        .success();

    let output = directory.path().join("kdm_screener.mov");
    export_command(
        dcp_fixture().config_home.path(),
        &encrypted.package,
        &output,
    )
    .arg("--kdm")
    .arg(&kdm)
    .arg("--recipient-key")
    .arg(&recipient_key)
    .args(["--format", "prores"])
    .assert()
    .success();

    assert_prores_export_holds_the_fixture(&output, "the ProRes export under a KDM");
    assert_only_entries(
        directory.path(),
        &[&recipient, &recipient_key, &kdm, &output, &history],
    );
}

#[test]
fn an_encrypted_dcp_without_keys_is_refused_before_anything_is_written() {
    let encrypted = encrypted_fixture();
    let directory = TempDir::new().unwrap();
    let output = directory.path().join("screener.mov");
    let picture = only_file_starting_with(&encrypted.package, "picture_");

    export_command(
        dcp_fixture().config_home.path(),
        &encrypted.package,
        &output,
    )
    .args(["--format", "prores"])
    .assert()
    .failure()
    .stderr(predicate::str::contains(format!(
        "{} is encrypted, pass --kdm with --recipient-key or --keys",
        picture.display()
    )));

    assert_only_entries(directory.path(), &[]);
}

#[test]
fn a_cpl_exports_its_picture_and_sound() {
    let fixture = dcp_fixture();
    let package = fixture.picture_mxf.parent().unwrap();
    let (_, cpl) = only_cpl(package);
    let output = fixture.directory.path().join("cpl_screener.mov");

    export_command(fixture.config_home.path(), &cpl, &output)
        .args(["--format", "prores"])
        .assert()
        .success();

    assert_prores_export_holds_the_fixture(&output, "the ProRes export of the CPL");
}

fn write_flat_master(directory: &Path, seconds: u32) -> (PathBuf, PathBuf) {
    let master = directory.join("flat.mkv");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "color=c={SOURCE_COLOUR}:size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}:duration={seconds}"
            ))
            .args(["-c:v", "ffv1", "-pix_fmt", "gbrp"])
            .arg(&master),
        "the flat master has to be written",
    );
    let wav = directory.join("tone.wav");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:duration={seconds}:sample_rate=48000"
            ))
            .args(["-ac", "2", "-c:a", "pcm_s24le"])
            .arg(&wav),
        "the tone has to be written",
    );
    (master, wav)
}

#[test]
fn a_two_reel_dcp_is_refused_naming_its_cpl() {
    let fixture = dcp_fixture();
    let directory = TempDir::new().unwrap();
    let (master, wav) = write_flat_master(directory.path(), 2);
    let package = create_dcp(
        fixture.config_home.path(),
        &master,
        &wav,
        &directory.path().join("dcp"),
        "Export Reels",
        &["--split-at", "00:00:01"],
    );
    let (_, cpl) = only_cpl(&package);
    let output = directory.path().join("screener.mov");

    export_command(fixture.config_home.path(), &package, &output)
        .args(["--format", "prores"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "{} has 2 reels, export takes a single-reel CPL",
            cpl.display()
        )));
    assert!(
        !output.exists(),
        "the refused export wrote {}",
        output.display()
    );
}

#[test]
fn a_stereoscopic_picture_mxf_is_refused_by_name() {
    let directory = TempDir::new().unwrap();
    let codestream = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../extern/postkit/tests/fixtures/cinema2k_64x64.j2c"),
    )
    .expect("the postkit submodule holds the cinema 2K codestream");
    let eye_files = |eye: &str| -> Vec<PathBuf> {
        (0..FRAMES)
            .map(|index| {
                let path = directory.path().join(format!("{eye}_{index}.j2c"));
                std::fs::write(&path, &codestream).unwrap();
                path
            })
            .collect()
    };
    let mxf = directory.path().join("stereoscopic.mxf");
    let track = postkit::mxf_wrap::wrap_stereoscopic(&postkit::mxf_wrap::StereoscopicWrapOptions {
        left_files: eye_files("left"),
        right_files: eye_files("right"),
        output: mxf.clone(),
        fps_num: FRAME_RATE,
        fps_den: 1,
        encryption: None,
        asset_uuid: None,
    });
    assert!(track.success, "wrap failed: {}", track.error);
    let output = directory.path().join("screener.mov");

    export_command(dcp_fixture().config_home.path(), &mxf, &output)
        .args(["--format", "prores"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "{} is a stereoscopic picture MXF",
            mxf.display()
        )));
    assert!(
        !output.exists(),
        "the refused export wrote {}",
        output.display()
    );
}

const SPAN_SOURCE_SECONDS: u32 = 3;
const SPAN_ENTRY_POINT: u32 = FRAME_RATE;

// every <element> in the CPL
fn set_every_element(xml: &str, element: &str, value: u32) -> String {
    let open = format!("<{element}>");
    let close = format!("</{element}>");
    assert!(xml.contains(&open), "the CPL has to carry {open}");
    let mut rewritten = String::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let value_start = start + open.len();
        let value_end = value_start + rest[value_start..].find(&close).unwrap();
        rewritten.push_str(&rest[..value_start]);
        rewritten.push_str(&value.to_string());
        rest = &rest[value_end..];
    }
    rewritten.push_str(rest);
    rewritten
}

fn set_reel_span(cpl: &Path, entry_point: u32, duration: u32) {
    let xml = std::fs::read_to_string(cpl).unwrap();
    let xml = set_every_element(&xml, "EntryPoint", entry_point);
    std::fs::write(cpl, set_every_element(&xml, "Duration", duration)).unwrap();
}

#[test]
fn a_cpl_exports_only_the_span_its_reel_plays() {
    let fixture = dcp_fixture();
    let directory = TempDir::new().unwrap();
    let (master, wav) = write_flat_master(directory.path(), SPAN_SOURCE_SECONDS);
    let package = create_dcp(
        fixture.config_home.path(),
        &master,
        &wav,
        &directory.path().join("dcp"),
        "Export Span",
        &[],
    );
    let (_, cpl) = only_cpl(&package);
    let source_frames = SPAN_SOURCE_SECONDS * FRAME_RATE;
    let played = source_frames - 2 * SPAN_ENTRY_POINT;
    set_reel_span(&cpl, SPAN_ENTRY_POINT, played);
    let output = directory.path().join("span.mov");

    export_command(fixture.config_home.path(), &cpl, &output)
        .args(["--format", "prores"])
        .assert()
        .success();

    assert_eq!(probe(&output, "v:0", "nb_frames"), [played.to_string()]);
    let sound_seconds: f64 = probe(&output, "a:0", "duration")[0].parse().unwrap();
    let played_seconds = f64::from(played) / f64::from(FRAME_RATE);
    assert!(
        (sound_seconds - played_seconds).abs() <= 1.0 / f64::from(FRAME_RATE),
        "the sound runs {sound_seconds} s against the {played_seconds} s the reel plays"
    );

    let past_the_end = directory.path().join("past_the_end.mov");
    set_reel_span(&cpl, SPAN_ENTRY_POINT, source_frames);
    export_command(fixture.config_home.path(), &cpl, &past_the_end)
        .args(["--format", "prores"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "{} plays frames {SPAN_ENTRY_POINT}..{} of {}, which holds {source_frames}",
            cpl.display(),
            SPAN_ENTRY_POINT + source_frames,
            only_file_starting_with(&package, "picture_").display()
        )));
    assert!(
        !past_the_end.exists(),
        "the refused export wrote {}",
        past_the_end.display()
    );
}

// silent for the first second, a tone after
const LATE_TONE: &str = "aevalsrc=if(gte(t\\,1)\\,sin(2*PI*440*t)\\,0):s=48000:d=3";
const SOUND_SPAN_FRAMES: u32 = 2 * FRAME_RATE;
const PROBED_SECONDS: &str = "0.5";
const TONE_MEAN_VOLUME_FLOOR_DB: f64 = -30.0;
const SILENCE_MEAN_VOLUME_CEILING_DB: f64 = -60.0;

fn set_asset_entry_point(xml: &str, asset: &str, entry_point: u32) -> String {
    let start = xml
        .find(&format!("<{asset}>"))
        .unwrap_or_else(|| panic!("the CPL has to carry <{asset}>"));
    let end = start + xml[start..].find(&format!("</{asset}>")).unwrap();
    format!(
        "{}{}{}",
        &xml[..start],
        set_every_element(&xml[start..end], "EntryPoint", entry_point),
        &xml[end..]
    )
}

// ffmpeg's mean volume over the first PROBED_SECONDS of the sound
fn opening_mean_volume_db(file: &Path) -> f64 {
    let output = ffmpeg()
        .args(["-v", "info", "-t", PROBED_SECONDS])
        .arg("-i")
        .arg(file)
        .args(["-vn", "-af", "volumedetect", "-f", "null", "-"])
        .output()
        .expect("ffmpeg must be installed");
    let log = String::from_utf8_lossy(&output.stderr);
    let value = log
        .lines()
        .find_map(|line| line.split("mean_volume:").nth(1))
        .unwrap_or_else(|| panic!("volumedetect printed no mean volume: {log}"));
    value.trim().trim_end_matches("dB").trim().parse().unwrap()
}

#[test]
fn the_sound_plays_from_its_own_entry_point() {
    let fixture = dcp_fixture();
    let directory = TempDir::new().unwrap();
    let (master, _) = write_flat_master(directory.path(), SPAN_SOURCE_SECONDS);
    let late_tone = directory.path().join("late_tone.wav");
    run(
        ffmpeg()
            .args(["-f", "lavfi", "-i", LATE_TONE])
            .args(["-ac", "2", "-c:a", "pcm_s24le"])
            .arg(&late_tone),
        "the late tone has to be written",
    );
    let package = create_dcp(
        fixture.config_home.path(),
        &master,
        &late_tone,
        &directory.path().join("dcp"),
        "Export Sound Entry",
        &[],
    );
    let (_, cpl) = only_cpl(&package);
    let original = std::fs::read_to_string(&cpl).unwrap();
    let export_with_entry_points = |picture_entry: u32, sound_entry: u32, name: &str| {
        let xml = set_every_element(&original, "Duration", SOUND_SPAN_FRAMES);
        let xml = set_asset_entry_point(&xml, "MainPicture", picture_entry);
        std::fs::write(&cpl, set_asset_entry_point(&xml, "MainSound", sound_entry)).unwrap();
        let output = directory.path().join(name);
        export_command(fixture.config_home.path(), &cpl, &output)
            .args(["--format", "prores"])
            .assert()
            .success();
        assert_eq!(
            probe(&output, "v:0", "nb_frames"),
            [SOUND_SPAN_FRAMES.to_string()]
        );
        opening_mean_volume_db(&output)
    };

    let late_sound = export_with_entry_points(0, FRAME_RATE, "sound_entry.mov");
    assert!(
        late_sound > TONE_MEAN_VOLUME_FLOOR_DB,
        "the sound seeked one second in opens with the tone, measured {late_sound} dB"
    );
    let late_picture = export_with_entry_points(FRAME_RATE, 0, "picture_entry.mov");
    assert!(
        late_picture < SILENCE_MEAN_VOLUME_CEILING_DB,
        "the sound read from its start opens silent, measured {late_picture} dB"
    );
}
