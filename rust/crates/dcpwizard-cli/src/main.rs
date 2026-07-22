use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "dcpwizard",
    version,
    about = "DCP Wizard — Digital Cinema Package creator"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new DCP
    Create {
        /// DCP title
        #[arg(short, long)]
        title: String,
        /// Video file (mp4/mov/mkv) or J2K/image sequence directory
        #[arg(long)]
        video: String,
        /// Audio WAV file
        #[arg(long)]
        audio: Option<String>,
        /// Six-channel WAV order: dcp (L,R,C,LFE,Ls,Rs) or lrc-ls-rs-lfe
        #[arg(long, default_value = "dcp")]
        audio_input_order: String,
        /// HDR-to-DCI 3D LUT. Required for HDR source video unless generic tone mapping is enabled.
        #[arg(long)]
        hdr_to_dci_lut: Option<String>,
        /// Allow generic FFmpeg HDR tone mapping. It is not a delivery transform.
        #[arg(long)]
        allow_generic_hdr_tonemap: bool,
        /// SRT file to convert, or supplied SMPTE subtitle XML to package unchanged
        #[arg(long)]
        subtitle: Option<String>,
        /// Subtitle language code (e.g. "en", "fr")
        #[arg(long, default_value = "en")]
        subtitle_language: String,
        /// Output directory
        #[arg(short, long)]
        output: String,
        /// DCP standard (smpte|interop)
        #[arg(long, default_value = "smpte")]
        standard: String,
        /// Delivery profile
        #[arg(long)]
        profile: Option<String>,
        /// Encrypt the DCP
        #[arg(long)]
        encrypt: bool,
        /// Where to write the content keys (required with --encrypt). Holds the
        /// plaintext AES keys: point it outside the DCP and keep it secret.
        #[arg(long, required_if_eq("encrypt", "true"))]
        key_out: Option<String>,
        /// Content type: FTR, SHR, TLR, TST, XSN, RTG, TSR, POL, PSA, ADV
        #[arg(long)]
        content_type: Option<String>,
        /// DCP frame rate (auto-detected from source if not specified)
        #[arg(long)]
        frame_rate: Option<u32>,
        /// Force 2K resolution
        #[arg(long)]
        twok: bool,
        /// Force 4K resolution
        #[arg(long)]
        fourk: bool,
        /// Picture container: 2k-scope, 2k-flat, 2k-full, 4k-scope, 4k-flat, or 4k-full
        #[arg(long)]
        container: Option<String>,
        /// Number of encoding threads (default: auto-detect CPU count)
        #[arg(short = 'j', long)]
        threads: Option<u32>,
        /// J2K bandwidth in Mbit/s (default: 250 for 2K, 500 for 4K)
        #[arg(long)]
        video_bit_rate: Option<u32>,
        /// Split into reels of at most N minutes each (default: single reel)
        #[arg(long)]
        reel_length: Option<u32>,
        /// Right-eye video/J2K for a stereoscopic 3D DCP (main input is left eye)
        #[arg(long)]
        right_eye: Option<String>,
        /// Dolby Atmos / DCData bitstream to wrap as a ST 429-18 auxiliary track
        #[arg(long)]
        atmos: Option<String>,
        /// Sound channel index (0-based) carrying the Hearing Impaired (HI) track
        #[arg(long)]
        hi_channel: Option<u32>,
        /// Sound channel index (0-based) carrying the Visually Impaired (VI-N) track
        #[arg(long)]
        vi_channel: Option<u32>,
    },
    /// Rebuild ASSETMAP and PKL to cover every asset file present (metadata-only
    /// repackaging; no re-wrap or re-encode). For re-ingesting exported OV/VF
    /// folders whose ASSETMAP/PKL omit hardlinked assets.
    IngestPackage {
        /// DCP package directory to repackage in place
        dir: String,
    },
    /// Create a supplemental Version File (VF) DCP against an Original Version
    CreateVf {
        /// Original Version (OV) DCP directory
        #[arg(long)]
        ov: String,
        /// Output VF directory
        #[arg(short, long)]
        output: String,
        /// VF title (defaults to "<OV title>_VF")
        #[arg(short, long, default_value = "")]
        title: String,
        /// Replace a reel's picture essence: --replace-picture REEL=PATH (repeatable)
        #[arg(long = "replace-picture", value_name = "REEL=PATH")]
        replace_picture: Vec<String>,
        /// Replace a reel's sound essence: --replace-sound REEL=PATH (repeatable)
        #[arg(long = "replace-sound", value_name = "REEL=PATH")]
        replace_sound: Vec<String>,
    },
    /// Encode images to JPEG 2000
    Encode {
        /// Input image directory
        #[arg(short, long)]
        input: String,
        /// Output J2K directory
        #[arg(short, long)]
        output: String,
        /// Target bitrate (Mbps)
        #[arg(long, default_value = "250")]
        bandwidth: u32,
    },
    /// Full pipeline: video → J2K → DCP (streaming, no intermediate files)
    Pipeline {
        /// Input video file (or image/J2K directory)
        #[arg(short, long)]
        input: String,
        /// DCP title
        #[arg(short, long)]
        title: String,
        /// Output directory
        #[arg(short, long)]
        output: String,
        /// Audio WAV file
        #[arg(long)]
        audio: Option<String>,
        /// Compression ratio (default: 10)
        #[arg(long, default_value = "10")]
        ratio: f64,
        /// Frame rate (default: 24)
        #[arg(long, default_value = "24")]
        fps: u32,
    },
    /// Transcode video to image sequence
    Transcode {
        /// Input video file
        #[arg(short, long)]
        input: String,
        /// Output directory
        #[arg(short, long)]
        output: String,
        /// Image format: tiff (default), dpx, exr, png
        #[arg(long, default_value = "tiff")]
        format: String,
        /// Bit depth: 16 (default), 10, or 8
        #[arg(long, default_value = "16")]
        bit_depth: u32,
    },
    /// Re-encode an existing DCP's picture essence to a lower bandwidth
    TranscodeDcp {
        /// Input DCP directory
        #[arg(short, long)]
        input: String,
        /// Output DCP directory (must differ from input)
        #[arg(short, long)]
        output: String,
        /// Target picture bandwidth in Mbit/s
        #[arg(long)]
        video_bit_rate: u32,
        /// Optional target width (with --height, rescales the picture)
        #[arg(long)]
        width: Option<u32>,
        /// Optional target height (with --width, rescales the picture)
        #[arg(long)]
        height: Option<u32>,
    },
    /// Verify an existing DCP
    Verify {
        /// DCP directory
        dcp_dir: String,
        /// Skip asset hash verification
        #[arg(long)]
        no_hash_check: bool,
        /// Skip picture bitstream checks (faster)
        #[arg(long)]
        no_picture_check: bool,
        /// Require strict SMPTE Bv2.1 compliance
        #[arg(long)]
        strict: bool,
        /// Write report to file (.txt or .html)
        #[arg(short, long)]
        output: Option<String>,
        /// Quiet mode (exit code only, no output)
        #[arg(short, long)]
        quiet: bool,
    },
    /// Show DCP metadata
    Info {
        /// DCP directory
        dcp_dir: String,
    },
    /// Generate KDM for encrypted DCP
    Kdm {
        /// CPL ID
        #[arg(long)]
        cpl_id: String,
        /// Content title
        #[arg(long)]
        content_title: String,
        /// Recipient certificate file
        #[arg(long)]
        cert: String,
        /// Signer leaf certificate file
        #[arg(long)]
        signer_cert: String,
        /// Signer private key file
        #[arg(long)]
        signer_key: String,
        /// Signer CA certificate above the leaf (repeatable: intermediate(s) then root)
        #[arg(long)]
        signer_chain: Vec<String>,
        /// Output KDM file
        #[arg(short, long)]
        output: String,
        /// Valid from (ISO 8601, e.g. "2024-06-01T00:00:00+00:00") or "now"
        #[arg(short = 'f', long, default_value = "now")]
        valid_from: String,
        /// Valid to: ISO 8601 or a relative duration (e.g. "2 weeks", "30 days")
        #[arg(short = 't', long, default_value = "2 weeks")]
        valid_to: String,
        /// DCP keys file (KEYS.json from `create --encrypt`) whose content keys
        /// this KDM should carry. Required to unlock an encrypted DCP.
        #[arg(long)]
        keys: Option<String>,
        /// KDM format: smpte (default) or interop (legacy, needs real-gear validation)
        #[arg(long, default_value = "smpte")]
        format: String,
    },
    /// Re-wrap a DKDM to a new recipient
    KdmRewrap {
        /// Source DKDM file
        #[arg(long)]
        dkdm: String,
        /// DKDM recipient's private key (decrypts the source key blocks)
        #[arg(long)]
        dkdm_key: String,
        /// New recipient certificate file
        #[arg(long)]
        cert: String,
        /// Signer leaf certificate file
        #[arg(long)]
        signer_cert: String,
        /// Signer private key file
        #[arg(long)]
        signer_key: String,
        /// Signer CA certificate above the leaf (repeatable: intermediate(s) then root)
        #[arg(long)]
        signer_chain: Vec<String>,
        /// Valid from: ISO 8601 or "now"; empty preserves the DKDM window
        #[arg(short = 'f', long, default_value = "")]
        valid_from: String,
        /// Valid to: ISO 8601 or relative duration; empty preserves the DKDM window
        #[arg(short = 't', long, default_value = "")]
        valid_to: String,
        /// Output KDM file
        #[arg(short, long)]
        output: String,
    },
    /// Copy DCP to drive
    Copy {
        /// DCP directory
        #[arg(long)]
        src: String,
        /// Destination drive/directory
        #[arg(long)]
        dst: String,
    },
    /// Measure audio loudness
    Loudness {
        /// Audio file
        audio_file: String,
    },
    /// Generate QC report
    Report {
        /// DCP directory
        #[arg(long)]
        dcp: String,
        /// Output HTML file
        #[arg(short, long)]
        output: String,
    },
    /// Start REST API server
    Serve {
        /// Listen address (host:port)
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        bind: String,
    },
    /// Watch directory for auto-DCP creation
    Watch {
        /// Directory to watch
        dir: String,
        /// POST a JSON notification to this URL when a new DCP is detected
        #[arg(long)]
        webhook_url: Option<String>,
    },
    /// Export a DCP picture MXF to a delivery format via ffmpeg
    Export {
        /// Input picture MXF
        #[arg(long)]
        input: String,
        /// Output file (or directory for image-sequence)
        #[arg(short, long)]
        output: String,
        /// Format: prores, h264, h265, dnxhr, image-sequence
        #[arg(long, default_value = "h264")]
        format: String,
        /// Quality CRF for h264/h265 (lower is better; default 18)
        #[arg(long, default_value = "18")]
        crf: u32,
        /// Optional sound MXF to mux into the output
        #[arg(long)]
        audio: Option<String>,
    },
    /// Generate shell completion
    Completion {
        /// Shell (bash|zsh|fish)
        #[arg(default_value = "bash")]
        shell: String,
    },
    /// Start job queue daemon
    Daemon,
    /// Manage job queue
    Batch {
        #[command(subcommand)]
        action: BatchAction,
    },
    /// Convert SRT subtitles to DCP XML (SMPTE Timed Text)
    SubtitleConvert {
        /// Input SRT file
        #[arg(short, long)]
        input: String,
        /// Output XML file
        #[arg(short, long)]
        output: String,
        /// Language code (e.g. "en", "fr", "de")
        #[arg(short, long, default_value = "en")]
        language: String,
        /// Frame rate for timecode conversion (24, 25, 30, 48)
        #[arg(long, default_value = "24")]
        fps: u32,
        /// Bottom-line position as a percentage up from the bottom of the screen
        #[arg(long, default_value = "8.0")]
        vposition: f64,
    },
    /// Burn subtitles into video
    #[command(alias = "burn-in")]
    Burnin {
        /// Input video file
        #[arg(short, long)]
        input: String,
        /// Subtitle file (SRT, ASS, or SMPTE XML)
        #[arg(short, long)]
        subtitles: String,
        /// Output video file
        #[arg(short, long)]
        output: String,
        /// Font size for burn-in (default: 24)
        #[arg(long, default_value = "24")]
        font_size: u32,
    },
    /// Convert video to a target DCI container (scale/crop/letterbox)
    Convert {
        /// Input video file
        #[arg(short, long)]
        input: String,
        /// Output video file
        #[arg(short, long)]
        output: String,
        /// Target container: 2k-scope, 2k-flat, 2k-full, 4k-scope, 4k-flat, 4k-full
        #[arg(short, long)]
        target: String,
        /// Method: letterbox, crop, or scale
        #[arg(short, long, default_value = "letterbox")]
        method: String,
    },

    /// Create DCDM (Digital Cinema Distribution Master) X'Y'Z' sequence
    Dcdm {
        /// Input image sequence directory
        #[arg(short, long)]
        input: String,

        /// Output DCDM TIFF directory
        #[arg(short, long)]
        output: String,

        /// Source colour space (rec709, p3, aces, logc)
        #[arg(short, long, default_value = "rec709")]
        colour_space: String,

        /// Optional 3D LUT for colour transform
        #[arg(long)]
        lut: Option<String>,
    },

    /// Convert colour space of images/video
    Colour {
        /// Input file or directory
        #[arg(short, long)]
        input: String,

        /// Output file or directory
        #[arg(short, long)]
        output: String,

        /// Source colour space (rec709, p3, xyz, rec2020, aces, acescg, logc)
        #[arg(short, long)]
        source: String,

        /// Target colour space
        #[arg(short, long)]
        target: String,

        /// Optional 3D LUT file for custom transform
        #[arg(long)]
        lut: Option<String>,
    },

    /// Import EDL/AAF/XML timeline for conforming
    Conform {
        /// Input timeline file (EDL, AAF, FCP XML, OTIO)
        #[arg(short, long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Ingest camera raw media
    Ingest {
        /// Camera card/media directory
        #[arg(short, long)]
        source: String,

        /// Output directory
        #[arg(short, long)]
        output: String,

        /// Output format (dpx, tiff, exr, prores)
        #[arg(short, long, default_value = "dpx")]
        format: String,

        /// Colour space (ACES, Rec.709, P3, LogC)
        #[arg(short, long, default_value = "ACES")]
        colour_space: String,
    },

    /// Extract a frame from video/MXF as image
    #[command(name = "frame-extract")]
    FrameExtract {
        /// Input video/MXF file
        #[arg(short, long)]
        input: String,

        /// Frame number to extract
        #[arg(short, long, default_value = "0")]
        frame: u32,

        /// Output image file (png, jpg, tiff)
        #[arg(short, long)]
        output: String,
    },

    /// Inject Dolby Vision RPU into HEVC stream
    #[command(name = "dv-inject")]
    DvInject {
        /// Input HEVC file
        #[arg(short, long)]
        input: String,

        /// RPU file (.bin)
        #[arg(short, long)]
        rpu: String,

        /// Output file
        #[arg(short, long)]
        output: String,
    },

    /// Inject HDR10 static metadata
    #[command(name = "hdr10-inject")]
    Hdr10Inject {
        /// Input video file
        #[arg(short, long)]
        input: String,

        /// Output video file
        #[arg(short, long)]
        output: String,

        /// Max content light level (MaxCLL)
        #[arg(long, default_value = "1000")]
        max_cll: u16,

        /// Max frame average light level (MaxFALL)
        #[arg(long, default_value = "400")]
        max_fall: u16,
    },

    /// Burn a visible watermark into a video/image file
    Watermark {
        /// Input video/image file
        #[arg(short, long)]
        input: String,

        /// Output video/image file
        #[arg(short, long)]
        output: String,

        /// Watermark payload (distributor ID, serial, etc.) rendered visibly
        #[arg(short, long)]
        payload: String,
    },

    /// Generate or inspect X.509 certificates for DCP encryption
    #[command(alias = "cert")]
    Certificate {
        #[command(subcommand)]
        action: CertAction,
    },

    /// Generate KDMs for multiple recipients in one pass
    #[command(name = "kdm-batch")]
    KdmBatch {
        /// CPL ID
        #[arg(long)]
        cpl_id: String,
        /// Content title
        #[arg(long)]
        content_title: String,
        /// Recipient certificate file (repeatable, one KDM generated per cert)
        #[arg(long = "cert")]
        certs: Vec<String>,
        /// Directory of recipient certificates; every *.pem/*.crt/*.cer in it
        /// gets a KDM. Combined with any --cert values.
        #[arg(long)]
        cert_dir: Option<String>,
        /// Signer leaf certificate file
        #[arg(long)]
        signer_cert: String,
        /// Signer private key file
        #[arg(long)]
        signer_key: String,
        /// Signer CA certificate above the leaf (repeatable)
        #[arg(long)]
        signer_chain: Vec<String>,
        /// Output directory for generated KDMs
        #[arg(short, long)]
        output_dir: String,
        /// Valid from (ISO 8601 or "now")
        #[arg(short = 'f', long, default_value = "now")]
        valid_from: String,
        /// Valid to (ISO 8601 or relative duration, e.g. "2 weeks")
        #[arg(short = 't', long, default_value = "2 weeks")]
        valid_to: String,
        /// DCP keys file (KEYS.json from `create --encrypt`) whose content keys
        /// every generated KDM should carry.
        #[arg(long)]
        keys: Option<String>,
        /// KDM format: smpte (default) or interop (legacy, needs real-gear validation)
        #[arg(long, default_value = "smpte")]
        format: String,
    },

    /// Package a trailer (ratings card + countdown leader + content)
    Trailer {
        /// Trailer content video file
        #[arg(short, long)]
        content: String,
        /// Output directory
        #[arg(short, long)]
        output: String,
        /// Trailer title (rendered on the ratings card)
        #[arg(long, default_value = "")]
        title: String,
        /// Rating text (e.g. "PG-13", "15")
        #[arg(long, default_value = "")]
        rating: String,
        /// Rating system: mpaa, bbfc, fsk, custom
        #[arg(long, default_value = "mpaa")]
        rating_system: String,
        /// Band colour: green, red, yellow
        #[arg(long, default_value = "green")]
        band: String,
        /// Countdown leader length in seconds
        #[arg(long, default_value = "8")]
        countdown: u32,
        /// Frame rate
        #[arg(long, default_value = "24")]
        fps: u32,
    },

    /// Generate DCP markers (FFOC/LFOC) for a composition
    Markers {
        /// Composition length in frames
        #[arg(short, long)]
        frames: u64,
        /// Emit an XML MarkerList instead of a plain list
        #[arg(long)]
        xml: bool,
    },

    /// Check accessibility compliance of a DCP
    Accessibility {
        /// DCP directory
        dcp_dir: String,
        /// Standard: cvaa, eaa, aoda, ofcom
        #[arg(short, long, default_value = "cvaa")]
        standard: String,
    },

    /// Send a webhook notification (HTTP POST via curl)
    Webhook {
        /// Target URL
        #[arg(short, long)]
        url: String,
        /// Event type
        #[arg(long, default_value = "ping")]
        event: String,
        /// Job ID
        #[arg(long, default_value = "")]
        job_id: String,
        /// Shared secret (sent as X-Webhook-Secret)
        #[arg(long, default_value = "")]
        secret: String,
        /// JSON payload (defaults to a test ping body)
        #[arg(long, default_value = "")]
        payload: String,
    },

    /// Content version / delivery history tracker (SQLite)
    Version {
        #[command(subcommand)]
        action: VersionAction,
    },

    /// OV/VF version dashboard and distribution tracking
    Dashboard {
        #[command(subcommand)]
        action: DashboardAction,
    },
}

#[derive(Subcommand)]
enum VersionAction {
    /// Record a delivery
    Record {
        /// Tracker database file
        #[arg(long, default_value = "deliveries.db")]
        db: String,
        /// Package UUID
        #[arg(long)]
        package_uuid: String,
        /// Title
        #[arg(long, default_value = "")]
        title: String,
        /// Version label (e.g. OV, VF)
        #[arg(long, default_value = "")]
        version: String,
        /// Destination
        #[arg(long, default_value = "")]
        destination: String,
        /// Delivery method (e.g. hard_drive, satellite)
        #[arg(long, default_value = "")]
        method: String,
        /// Mark as verified
        #[arg(long)]
        verified: bool,
    },
    /// List recorded deliveries
    List {
        /// Tracker database file
        #[arg(long, default_value = "deliveries.db")]
        db: String,
        /// Filter by package UUID
        #[arg(long)]
        package_uuid: Option<String>,
        /// Filter by destination
        #[arg(long)]
        destination: Option<String>,
    },
    /// Export delivery history (format by extension: .json or .csv)
    Export {
        /// Tracker database file
        #[arg(long, default_value = "deliveries.db")]
        db: String,
        /// Output file (.json or .csv)
        #[arg(short, long)]
        output: String,
    },
}

#[derive(Subcommand)]
enum DashboardAction {
    /// Register a DCP version (OV or VF)
    Register {
        /// Version UUID
        #[arg(long)]
        uuid: String,
        /// Title
        #[arg(long)]
        title: String,
        /// Version type: OV or VF
        #[arg(long, default_value = "OV")]
        version_type: String,
        /// Territory (ISO 3166-1 alpha-2)
        #[arg(long, default_value = "")]
        territory: String,
        /// Language (RFC 5646)
        #[arg(long, default_value = "")]
        language: String,
        /// Standard: SMPTE or Interop
        #[arg(long, default_value = "SMPTE")]
        standard: String,
        /// DCP path
        #[arg(long, default_value = "")]
        dcp_path: String,
        /// Status: draft, released, archived
        #[arg(long, default_value = "draft")]
        status: String,
        /// KDM recipient theatre (repeatable)
        #[arg(long = "kdm-recipient")]
        kdm_recipients: Vec<String>,
    },
    /// List registered versions
    List {
        /// Filter by territory
        #[arg(long)]
        territory: Option<String>,
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },
    /// Update a version's status
    Status {
        /// Version UUID
        #[arg(long)]
        uuid: String,
        /// New status
        #[arg(long)]
        status: String,
    },
    /// Export the distribution matrix as CSV
    Matrix {
        /// Output CSV file
        #[arg(short, long)]
        output: String,
    },
    /// Start the dashboard HTTP server
    Serve {
        /// Listen port
        #[arg(short, long, default_value = "9090")]
        port: u32,
        /// Bind address
        #[arg(short, long, default_value = "127.0.0.1")]
        bind: String,
    },
}

#[derive(Subcommand)]
enum CertAction {
    /// Generate a full certificate chain (root → intermediate → signer)
    Chain {
        /// Organization name for the certificates
        #[arg(long)]
        organization: String,
        /// Output directory for generated certificates
        #[arg(short, long)]
        output: String,
    },
    /// Generate a single certificate
    Generate {
        /// Certificate type: root, intermediate, leaf, signer
        #[arg(short = 't', long, default_value = "signer")]
        cert_type: String,
        /// Common Name (CN)
        #[arg(long)]
        cn: String,
        /// Organization
        #[arg(long, default_value = "")]
        organization: String,
        /// Output certificate file
        #[arg(long)]
        output_cert: String,
        /// Output private key file
        #[arg(long)]
        output_key: String,
        /// Issuer certificate (required for non-root)
        #[arg(long)]
        issuer_cert: Option<String>,
        /// Issuer private key (required for non-root)
        #[arg(long)]
        issuer_key: Option<String>,
        /// Key size in bits
        #[arg(long, default_value = "2048")]
        key_bits: u32,
        /// Validity in days
        #[arg(long, default_value = "3650")]
        validity_days: u32,
    },
    /// Inspect a certificate file and show its details
    Inspect {
        /// Path to PEM certificate file
        cert_file: String,
    },
}

#[derive(Subcommand)]
enum BatchAction {
    /// List all jobs
    List,
    /// Submit a new job
    Add {
        /// Job type (create-dcp|verify-dcp|export-dcp|import-video|encode-j2k|wrap-mxf|copy-to-drive)
        #[arg(short = 'T', long)]
        r#type: String,
        /// Job parameters (JSON string)
        #[arg(short, long)]
        params: String,
    },
    /// Cancel a job
    Cancel {
        /// Job ID to cancel
        id: String,
    },
}

fn parse_colour_space(s: &str) -> postkit::colour::ColourSpace {
    match s.to_lowercase().as_str() {
        "rec709" | "bt709" => postkit::colour::ColourSpace::Rec709,
        "p3" | "dcip3" | "dci-p3" => postkit::colour::ColourSpace::P3,
        "xyz" | "ciexyz" => postkit::colour::ColourSpace::Xyz,
        "rec2020" | "bt2020" => postkit::colour::ColourSpace::Rec2020,
        "aces" => postkit::colour::ColourSpace::Aces,
        "acescg" => postkit::colour::ColourSpace::AcesCg,
        "logc" | "arrilogc" => postkit::colour::ColourSpace::LogC,
        _ => {
            tracing::warn!("Unknown colour space '{s}', defaulting to Rec709");
            postkit::colour::ColourSpace::Rec709
        }
    }
}

fn main() {
    // Windows debug builds overflow the default 1MB stack due to large clap
    // derive enum (102 args across 34 subcommands). Spawn with 8MB stack.
    let thread = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(run)
        .expect("failed to spawn main thread");
    thread.join().unwrap();
}

fn run() {
    // User-friendly panic handler
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unexpected error".to_string()
        };
        let location = info
            .location()
            .map(|l| format!(" ({}:{})", l.file(), l.line()))
            .unwrap_or_default();
        eprintln!("\nerror: dcpwizard crashed: {payload}{location}");
        eprintln!(
            "This is a bug. Please report it at https://github.com/PostPerfection/dcpwizard/issues"
        );
        eprintln!("Include the command you ran and any input files if possible.");
        if std::env::var("RUST_BACKTRACE").is_ok() {
            eprintln!(
                "\nBacktrace:\n{:?}",
                std::backtrace::Backtrace::force_capture()
            );
        } else {
            eprintln!("Set RUST_BACKTRACE=1 for a detailed backtrace.");
        }
    }));

    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    postkit::grok_encoder::initialize(0);

    let code = match cli.command {
        Commands::Create {
            title,
            video,
            audio,
            audio_input_order,
            hdr_to_dci_lut,
            allow_generic_hdr_tonemap,
            subtitle,
            subtitle_language,
            output,
            standard,
            encrypt,
            key_out,
            content_type,
            frame_rate,
            twok,
            fourk,
            container,
            threads,
            video_bit_rate,
            reel_length,
            profile,
            right_eye,
            atmos,
            hi_channel,
            vi_channel,
        } => {
            let video_path = PathBuf::from(&video);
            let output_dir = PathBuf::from(&output);
            let std_val = if standard == "interop" {
                dcpwizard_core::Standard::Interop
            } else {
                dcpwizard_core::Standard::Smpte
            };
            let audio_input_order = match audio_input_order.as_str() {
                "dcp" => dcpwizard_core::mxf_wrap::AudioInputOrder::Canonical51,
                "lrc-ls-rs-lfe" => dcpwizard_core::mxf_wrap::AudioInputOrder::LrcLsRsLfe,
                value => {
                    tracing::error!("Unknown audio input order: {value}");
                    return;
                }
            };

            // Resolve delivery profile and apply its presets as defaults; explicit
            // flags still win.
            let profile = match profile.as_deref() {
                Some(name) => match dcpwizard_core::profiles::get_profile(name) {
                    Some(p) => {
                        tracing::info!("Using profile '{}': {}", p.name, p.description);
                        Some(p)
                    }
                    None => {
                        let names: Vec<String> = dcpwizard_core::profiles::all_profiles()
                            .into_iter()
                            .map(|p| p.name)
                            .collect();
                        tracing::error!(
                            "Unknown profile '{name}'. Available: {}",
                            names.join(", ")
                        );
                        std::process::exit(1);
                    }
                },
                None => None,
            };
            let fourk = fourk
                || (!twok
                    && profile
                        .as_ref()
                        .map(|p| p.resolution_width >= 4096)
                        .unwrap_or(false));
            let frame_rate = frame_rate.or_else(|| profile.as_ref().map(|p| p.frame_rate));
            let video_bit_rate =
                video_bit_rate.or_else(|| profile.as_ref().map(|p| p.bitrate_mbps));

            // Detect if input is a video file (not a J2K directory)
            let is_video_file = video_path.is_file()
                && video_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| {
                        matches!(
                            e.to_lowercase().as_str(),
                            "mp4"
                                | "mov"
                                | "mkv"
                                | "avi"
                                | "mxf"
                                | "ts"
                                | "m2ts"
                                | "mpg"
                                | "mpeg"
                                | "webm"
                        )
                    })
                    .unwrap_or(false);

            if is_video_file {
                // Full pipeline: video → J2K encode → MXF wrap → DCP
                use postkit::grok_encoder::{self, CompressParams, EncodeProgress};
                use std::sync::Arc;
                use std::sync::atomic::AtomicBool;

                let _ = std::fs::create_dir_all(&output_dir);
                let j2k_dir = output_dir.join("j2k");
                let _ = std::fs::create_dir_all(&j2k_dir);

                let mut encode_video_path = video_path.clone();
                let hdr_type = dcpwizard_core::dolby_vision::detect_hdr_type(&video_path);
                if hdr_type != postkit::dolby_vision::HdrType::Sdr {
                    let converted = output_dir.join("hdr_to_dci_source.mov");
                    if let Some(lut) = hdr_to_dci_lut.as_ref() {
                        let lut = PathBuf::from(lut);
                        if !lut.is_file() {
                            tracing::error!("HDR-to-DCI LUT not found: {}", lut.display());
                            return;
                        }
                        let opts = postkit::colour::ColourConvertOptions {
                            input: video_path.clone(),
                            output: converted.clone(),
                            source_space: postkit::colour::ColourSpace::Rec2020,
                            target_space: postkit::colour::ColourSpace::Xyz,
                            lut_path: Some(lut),
                        };
                        if let Err(e) = postkit::colour::convert_colour(&opts) {
                            tracing::error!("HDR-to-DCI LUT conversion failed: {e}");
                            return;
                        }
                    } else if allow_generic_hdr_tonemap {
                        tracing::warn!(
                            "Using generic FFmpeg HDR tone mapping. It is not suitable as a default delivery transform."
                        );
                        if dcpwizard_core::dolby_vision::convert_hdr(
                            &video_path,
                            postkit::dolby_vision::HdrType::Sdr,
                            &converted,
                        ) != 0
                        {
                            return;
                        }
                    } else {
                        tracing::error!(
                            "HDR source requires --hdr-to-dci-lut. Use --allow-generic-hdr-tonemap only for an explicitly accepted generic transform."
                        );
                        return;
                    }
                    encode_video_path = converted;
                }

                tracing::info!("Detected video file input — using grok encoder");

                // Probe video for frame rate and resolution
                let video_info = dcpwizard_core::probe::probe_video(&encode_video_path);
                match dcpwizard_core::probe::video_has_alpha(&encode_video_path) {
                    Ok(true) => {
                        tracing::error!(
                            "Input video has alpha. Composite it over an opaque background before creating a DCP."
                        );
                        std::process::exit(1);
                    }
                    Ok(false) => {}
                    Err(error) => {
                        tracing::error!("Cannot determine whether the input has alpha: {error}");
                        std::process::exit(1);
                    }
                }
                let (source_fps, source_needs_audio_pull_up) = video_info
                    .as_ref()
                    .map(|v| dcpwizard_core::hfr::source_rate_to_dcp(v.fps_num, v.fps_den))
                    .unwrap_or((24, false));
                let fps = frame_rate.unwrap_or(source_fps);
                let needs_audio_pull_up = source_needs_audio_pull_up && fps == 24;
                let (mut width, mut height, total_frames) = video_info
                    .as_ref()
                    .map(|v| (v.width, v.height, v.total_frames))
                    .unwrap_or((2048, 1080, 0));

                // reject an illegal fps/resolution combo before the encode runs
                if let Err(e) = dcpwizard_core::hfr::validate_fps_resolution(
                    fps,
                    fourk,
                    std_val == dcpwizard_core::Standard::Smpte,
                ) {
                    tracing::error!("{e}");
                    std::process::exit(1);
                }

                // Apply resolution override
                if fourk {
                    width = 4096;
                    height = 2160;
                } else if twok {
                    width = 2048;
                    height = 1080;
                }

                if let Some(ref info) = video_info {
                    tracing::info!(
                        "Input: {}x{} @ {}/{} fps, ~{} frames",
                        info.width,
                        info.height,
                        info.fps_num,
                        info.fps_den,
                        info.total_frames,
                    );
                }

                // Compute compression ratio from bitrate if specified
                let compression_ratio = match video_bit_rate {
                    Some(mbps) => {
                        dcpwizard_core::encode::bandwidth_to_ratio(width, height, fps, mbps)
                    }
                    None => 10.0,
                };

                let _num_threads = threads.unwrap_or(0); // reserved for future use

                let params = CompressParams {
                    compression_ratio,
                    frame_rate: fps as u16,
                    ..CompressParams::default()
                };

                let cancel = Arc::new(AtomicBool::new(false));
                let cancel_clone = cancel.clone();
                let _ = ctrlc::set_handler(move || {
                    cancel_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                });

                let result = grok_encoder::encode_video_pipeline(
                    &encode_video_path,
                    &j2k_dir,
                    &params,
                    total_frames as u64,
                    width,
                    height,
                    &cancel,
                    |p: EncodeProgress| {
                        let percent = if p.total_frames > 0 {
                            (p.frames_encoded as f64 / p.total_frames as f64) * 100.0
                        } else {
                            0.0
                        };
                        eprint!(
                            "\r[encode] {}/{} frames ({:.0}%) {:.1} fps   ",
                            p.frames_encoded, p.total_frames, percent, p.fps
                        );
                    },
                );
                eprintln!();

                if !result.success {
                    tracing::error!("Encode failed: {}", result.error);
                    std::process::exit(1);
                }
                tracing::info!("Encoded {} frames", result.frames_encoded);

                // Stereoscopic: encode the right eye into its own dir at the same
                // settings (main input is the left eye).
                let right_eye_dir = if let Some(ref re) = right_eye {
                    let re_path = PathBuf::from(re);
                    let j2k_right = output_dir.join("j2k_right");
                    let _ = std::fs::create_dir_all(&j2k_right);
                    tracing::info!("Encoding right eye: {}", re_path.display());
                    let re_result = grok_encoder::encode_video_pipeline(
                        &re_path,
                        &j2k_right,
                        &params,
                        total_frames as u64,
                        width,
                        height,
                        &cancel,
                        |_p: EncodeProgress| {},
                    );
                    if !re_result.success {
                        tracing::error!("Right-eye encode failed: {}", re_result.error);
                        std::process::exit(1);
                    }
                    Some(j2k_right)
                } else {
                    None
                };

                // Auto-demux audio from video if --audio not provided
                let audio_path = if let Some(a) = audio {
                    Some(PathBuf::from(a))
                } else {
                    let wav_out = output_dir.join("audio_demux.wav");
                    let demux = std::process::Command::new("ffmpeg")
                        .arg("-y")
                        .arg("-i")
                        .arg(&video_path)
                        .arg("-vn")
                        .arg("-acodec")
                        .arg("pcm_s24le")
                        .arg("-ar")
                        .arg("48000")
                        .arg(&wav_out)
                        .output();
                    match demux {
                        Ok(o) if o.status.success() => {
                            tracing::info!("Demuxed audio: {}", wav_out.display());
                            Some(wav_out)
                        }
                        Ok(_) => {
                            tracing::warn!("No audio stream found in input (or demux failed)");
                            None
                        }
                        Err(e) => {
                            tracing::warn!("ffmpeg not available for audio demux: {e}");
                            None
                        }
                    }
                };
                let audio_path = if needs_audio_pull_up {
                    audio_path.map(|input| {
                        let output = output_dir.join("audio_pullup.wav");
                        let result = std::process::Command::new("ffmpeg")
                            .arg("-y")
                            .arg("-i")
                            .arg(&input)
                            .arg("-af")
                            .arg("asetrate=48048,aresample=48000")
                            .arg("-c:a")
                            .arg("pcm_s24le")
                            .arg(&output)
                            .output();
                        match result {
                            Ok(result) if result.status.success() => {
                                tracing::info!("Applied 23.976-to-24 audio pull-up");
                                output
                            }
                            Ok(result) => {
                                tracing::error!(
                                    "23.976-to-24 audio pull-up failed: {}",
                                    String::from_utf8_lossy(&result.stderr)
                                );
                                std::process::exit(1);
                            }
                            Err(error) => {
                                tracing::error!("failed to run ffmpeg for audio pull-up: {error}");
                                std::process::exit(1);
                            }
                        }
                    })
                } else {
                    audio_path
                };

                let resolution = if fourk {
                    dcpwizard_core::Resolution::FourK
                } else {
                    dcpwizard_core::Resolution::TwoK
                };
                let ct = content_type
                    .as_deref()
                    .and_then(dcpwizard_core::ContentType::from_abbrev)
                    .unwrap_or_default();

                let (container_width, container_height) = match container.as_deref() {
                    Some("2k-scope") => (2048, 858),
                    Some("2k-flat") => (1998, 1080),
                    Some("2k-full") => (2048, 1080),
                    Some("4k-scope") => (4096, 1716),
                    Some("4k-flat") => (3996, 2160),
                    Some("4k-full") => (4096, 2160),
                    Some(value) => {
                        tracing::error!("Unknown container: {value}");
                        return;
                    }
                    None => (0, 0),
                };

                let config = dcpwizard_core::dcp::DcpConfig {
                    title,
                    standard: std_val,
                    encrypt,
                    key_out: key_out.map(PathBuf::from),
                    output_dir: output_dir.clone(),
                    frame_rate_num: fps,
                    frame_rate_den: 1,
                    resolution,
                    content_type: ct,
                    container_width,
                    container_height,
                    max_bitrate_mbps: video_bit_rate.unwrap_or(0),
                    j2k_dir: Some(j2k_dir.clone()),
                    audio_path: audio_path.clone(),
                    audio_input_order,
                    subtitle_path: subtitle.clone().map(PathBuf::from),
                    subtitle_language: subtitle_language.clone(),
                    reel_length_minutes: reel_length.unwrap_or(0),
                    right_eye_dir: right_eye_dir.clone(),
                    atmos_path: atmos.clone().map(PathBuf::from),
                    hi_channel,
                    vi_channel,
                    stereo_3d: right_eye_dir.is_some(),
                };
                let code = dcpwizard_core::dcp::create_dcp(&config);

                // Clean up intermediate files
                let _ = std::fs::remove_dir_all(&j2k_dir);
                if let Some(ref d) = right_eye_dir {
                    let _ = std::fs::remove_dir_all(d);
                }
                if let Some(ref wav) = audio_path
                    && matches!(
                        wav.file_name().and_then(|f| f.to_str()),
                        Some("audio_demux.wav" | "audio_pullup.wav")
                    )
                {
                    let _ = std::fs::remove_file(wav);
                }
                code
            } else {
                // Input is a J2K directory or image sequence
                let resolution = if fourk {
                    dcpwizard_core::Resolution::FourK
                } else {
                    dcpwizard_core::Resolution::TwoK
                };
                let ct = content_type
                    .as_deref()
                    .and_then(dcpwizard_core::ContentType::from_abbrev)
                    .unwrap_or_default();

                let config = dcpwizard_core::dcp::DcpConfig {
                    title,
                    standard: std_val,
                    encrypt,
                    key_out: key_out.map(PathBuf::from),
                    output_dir,
                    frame_rate_num: frame_rate.unwrap_or(24),
                    frame_rate_den: 1,
                    resolution,
                    content_type: ct,
                    container_width: match container.as_deref() {
                        Some("2k-scope") => 2048,
                        Some("2k-flat") => 1998,
                        Some("2k-full") => 2048,
                        Some("4k-scope") => 4096,
                        Some("4k-flat") => 3996,
                        Some("4k-full") => 4096,
                        Some(value) => {
                            tracing::error!("Unknown container: {value}");
                            return;
                        }
                        None => 0,
                    },
                    container_height: match container.as_deref() {
                        Some("2k-scope") => 858,
                        Some("2k-flat") | Some("2k-full") => 1080,
                        Some("4k-scope") => 1716,
                        Some("4k-flat") | Some("4k-full") => 2160,
                        Some(value) => {
                            tracing::error!("Unknown container: {value}");
                            return;
                        }
                        None => 0,
                    },
                    max_bitrate_mbps: video_bit_rate.unwrap_or(0),
                    j2k_dir: Some(video_path),
                    audio_path: audio.map(PathBuf::from),
                    audio_input_order,
                    subtitle_path: subtitle.map(PathBuf::from),
                    subtitle_language,
                    reel_length_minutes: reel_length.unwrap_or(0),
                    stereo_3d: right_eye.is_some(),
                    right_eye_dir: right_eye.map(PathBuf::from),
                    atmos_path: atmos.map(PathBuf::from),
                    hi_channel,
                    vi_channel,
                };
                dcpwizard_core::dcp::create_dcp(&config)
            }
        }

        Commands::Encode {
            input,
            output,
            bandwidth,
        } => {
            let config = dcpwizard_core::encode::EncodeConfig {
                input_dir: PathBuf::from(input),
                output_dir: PathBuf::from(output),
                bandwidth_mbps: bandwidth,
                ..Default::default()
            };
            dcpwizard_core::encode::encode_j2k(&config)
        }

        Commands::Pipeline {
            input,
            title,
            output,
            audio,
            ratio,
            fps,
        } => {
            use postkit::encode::{StreamEncodeOptions, stream_encode_subprocess};
            use std::sync::Arc;
            use std::sync::atomic::AtomicBool;

            let input_path = PathBuf::from(&input);
            let output_dir = PathBuf::from(&output);

            if !input_path.exists() {
                tracing::error!("Input not found: {input}");
                std::process::exit(1);
            }

            let _ = std::fs::create_dir_all(&output_dir);
            let j2k_dir = output_dir.join("j2k");
            let _ = std::fs::create_dir_all(&j2k_dir);

            tracing::info!("Pipeline (subprocess Grok): {} -> {}", input, output);

            let grk_bin = std::env::var("GRK_COMPRESS_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_default();
                    PathBuf::from(home).join("bin/grok/bin/grk_compress")
                });

            let opts = StreamEncodeOptions {
                input: input_path.clone(),
                output_dir: j2k_dir.clone(),
                compression_ratio: ratio,
                num_resolutions: 6,
                codeblock_size: 32,
                progression: "CPRL".to_string(),
                fps,
                compressor_path: grk_bin,
                lib_dir: None,
            };

            let cancel = Arc::new(AtomicBool::new(false));

            // Handle Ctrl+C
            let cancel_clone = cancel.clone();
            let _ = ctrlc::set_handler(move || {
                cancel_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            });

            let result = stream_encode_subprocess(&opts, &cancel, |p| {
                let percent = if p.total_frames > 0 {
                    (p.frame as f64 / p.total_frames as f64) * 100.0
                } else {
                    0.0
                };
                eprint!(
                    "\r[encode] {}/{} frames ({:.0}%) {:.1} fps   ",
                    p.frame, p.total_frames, percent, p.fps
                );
            });
            eprintln!();

            if !result.success {
                tracing::error!("Encode failed: {}", result.error);
                1
            } else {
                tracing::info!("Encoded {} frames", result.frames_encoded);

                // Auto-demux audio from video if --audio not provided
                let audio_path = if let Some(a) = audio {
                    Some(PathBuf::from(a))
                } else {
                    let wav_out = output_dir.join("audio_demux.wav");
                    let demux = std::process::Command::new("ffmpeg")
                        .arg("-y")
                        .arg("-i")
                        .arg(&input_path)
                        .arg("-vn")
                        .arg("-acodec")
                        .arg("pcm_s24le")
                        .arg("-ar")
                        .arg("48000")
                        .arg(&wav_out)
                        .output();
                    match demux {
                        Ok(o) if o.status.success() => {
                            tracing::info!("Demuxed audio: {}", wav_out.display());
                            Some(wav_out)
                        }
                        Ok(_) => {
                            tracing::warn!("No audio stream in input (or demux failed)");
                            None
                        }
                        Err(e) => {
                            tracing::warn!("ffmpeg not available for audio demux: {e}");
                            None
                        }
                    }
                };

                // Package
                let config = dcpwizard_core::dcp::DcpConfig {
                    title,
                    standard: dcpwizard_core::Standard::Smpte,
                    output_dir: output_dir.clone(),
                    frame_rate_num: fps,
                    frame_rate_den: 1,
                    j2k_dir: Some(j2k_dir.clone()),
                    audio_path: audio_path.clone(),
                    audio_input_order: dcpwizard_core::mxf_wrap::AudioInputOrder::Canonical51,
                    ..Default::default()
                };
                let code = dcpwizard_core::dcp::create_dcp(&config);

                // Clean up intermediate files
                let _ = std::fs::remove_dir_all(&j2k_dir);
                if let Some(ref wav) = audio_path
                    && wav.file_name().and_then(|f| f.to_str()) == Some("audio_demux.wav")
                {
                    let _ = std::fs::remove_file(wav);
                }
                code
            }
        }

        Commands::Transcode {
            input,
            output,
            format,
            bit_depth,
        } => {
            // 8/10/16-bit packed RGB pixel formats ffmpeg understands for these codecs
            let pixel_format = match bit_depth {
                8 => "rgb24",
                10 | 16 => "rgb48le",
                other => {
                    tracing::error!("unsupported bit depth {other}; use 8, 10 or 16");
                    std::process::exit(1);
                }
            };
            let config = dcpwizard_core::transcode::TranscodeConfig {
                input_file: PathBuf::from(input),
                output_dir: PathBuf::from(output),
                image_format: format,
                pixel_format: pixel_format.to_string(),
                ..Default::default()
            };
            dcpwizard_core::transcode::transcode_to_sequence(&config)
        }

        Commands::TranscodeDcp {
            input,
            output,
            video_bit_rate,
            width,
            height,
        } => {
            let config = dcpwizard_core::j2k_transcode::DcpTranscodeConfig {
                input_dir: PathBuf::from(input),
                output_dir: PathBuf::from(output),
                target_bitrate_mbps: video_bit_rate,
                target_width: width.unwrap_or(0),
                target_height: height.unwrap_or(0),
            };
            dcpwizard_core::j2k_transcode::transcode_dcp(&config)
        }

        Commands::Verify {
            dcp_dir,
            no_hash_check,
            no_picture_check,
            strict,
            output,
            quiet,
        } => {
            let result = dcpwizard_core::verify::verify_dcp_with_options(
                &PathBuf::from(&dcp_dir),
                &dcpwizard_core::verify::VerifyCliOptions {
                    skip_hash_check: no_hash_check,
                    skip_picture_check: no_picture_check,
                    strict,
                },
            );

            if let Some(ref out_path) = output
                && let Err(e) =
                    dcpwizard_core::verify::write_verify_report(&result, Path::new(out_path))
            {
                tracing::error!("Failed to write report: {e}");
                std::process::exit(1);
            }

            if !quiet {
                if result.valid {
                    tracing::info!("DCP verification PASSED");
                } else {
                    for e in &result.errors {
                        tracing::error!("{e}");
                    }
                }
                for w in &result.warnings {
                    tracing::warn!("{w}");
                }
                for i in &result.info {
                    tracing::info!("{i}");
                }
            }

            if result.valid { 0 } else { 1 }
        }

        Commands::Info { dcp_dir } => {
            match dcpwizard_core::info::inspect_dcp(&PathBuf::from(dcp_dir)) {
                Ok(info) => {
                    tracing::info!("Title: {}", info.title);
                    tracing::info!("Standard: {}", info.standard);
                    tracing::info!("Frame rate: {}", info.frame_rate);
                    tracing::info!("Duration: {} frames", info.duration_frames);
                    tracing::info!("Reels: {}", info.reel_count);
                    tracing::info!("Encrypted: {}", if info.encrypted { "yes" } else { "no" });
                    0
                }
                Err(e) => {
                    tracing::error!("{e}");
                    1
                }
            }
        }

        Commands::Kdm {
            cpl_id,
            content_title,
            cert,
            signer_cert,
            signer_key,
            signer_chain,
            output,
            valid_from,
            valid_to,
            keys,
            format,
        } => {
            let format = match dcpwizard_core::kdm::parse_format(&format) {
                Ok(f) => f,
                Err(e) => {
                    tracing::error!("{e}");
                    std::process::exit(1);
                }
            };
            let content_keys = match keys {
                Some(path) => {
                    match dcpwizard_core::kdm::load_content_keys(&PathBuf::from(path), &cpl_id) {
                        Ok(k) => k,
                        Err(e) => {
                            tracing::error!("{e}");
                            std::process::exit(1);
                        }
                    }
                }
                None => Vec::new(),
            };
            dcpwizard_core::kdm::generate_kdm(
                cpl_id,
                content_title,
                PathBuf::from(cert),
                PathBuf::from(signer_cert),
                PathBuf::from(signer_key),
                signer_chain.into_iter().map(PathBuf::from).collect(),
                valid_from,
                valid_to,
                content_keys,
                PathBuf::from(output),
                format,
            )
        }

        Commands::KdmRewrap {
            dkdm,
            dkdm_key,
            cert,
            signer_cert,
            signer_key,
            signer_chain,
            valid_from,
            valid_to,
            output,
        } => dcpwizard_core::kdm::rewrap_dkdm(
            PathBuf::from(dkdm),
            PathBuf::from(dkdm_key),
            PathBuf::from(cert),
            PathBuf::from(signer_cert),
            PathBuf::from(signer_key),
            signer_chain.into_iter().map(PathBuf::from).collect(),
            valid_from,
            valid_to,
            PathBuf::from(output),
        ),

        Commands::Copy { src, dst } => {
            dcpwizard_core::copy_drive::copy_to_drive(&PathBuf::from(src), &PathBuf::from(dst))
        }

        Commands::Loudness { audio_file } => {
            let result = dcpwizard_core::loudness::measure_loudness(&PathBuf::from(audio_file));
            if result.success {
                tracing::info!("Integrated: {:.1} LUFS", result.integrated_lufs);
                tracing::info!("True Peak: {:.1} dBTP", result.true_peak_dbtp);
                tracing::info!("LRA: {:.1} LU", result.range_lu);
                0
            } else {
                tracing::error!("{}", result.error);
                1
            }
        }

        Commands::Report { dcp, output } => {
            dcpwizard_core::report::generate_report(&PathBuf::from(dcp), &PathBuf::from(output))
        }

        Commands::Serve { bind } => dcpwizard_core::rest_api::start_rest_api(&bind),

        Commands::Watch { dir, webhook_url } => {
            let webhook = webhook_url.map(|url| postkit::webhook::WebhookConfig {
                url,
                ..Default::default()
            });
            dcpwizard_core::watch::watch_directory(
                &PathBuf::from(dir),
                std::time::Duration::from_secs(5),
                &|| false,
                |p| {
                    tracing::info!("New DCP detected: {}", p.display());
                    if let Some(ref cfg) = webhook {
                        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                        let evt = postkit::webhook::WebhookEvent {
                            event_type: "dcp.detected".into(),
                            job_id: name.to_string(),
                            payload_json: postkit::webhook::build_job_completed_payload(
                                name, p, 0.0,
                            ),
                            timestamp: String::new(),
                        };
                        let res = postkit::webhook::send_webhook(cfg, &evt);
                        if !res.success {
                            tracing::warn!("webhook delivery failed: {}", res.error);
                        }
                    }
                },
            );
            0
        }

        Commands::Export {
            input,
            output,
            format,
            crf,
            audio,
        } => {
            use dcpwizard_core::export::{ExportConfig, ExportFormat, export_dcp};
            let fmt = match format.to_lowercase().as_str() {
                "prores" => ExportFormat::ProRes,
                "h264" | "x264" | "avc" => ExportFormat::H264,
                "h265" | "hevc" | "x265" => ExportFormat::H265,
                "dnxhr" | "dnxhd" => ExportFormat::DnxHr,
                "image-sequence" | "images" | "png" => ExportFormat::ImageSequence,
                other => {
                    tracing::error!(
                        "unknown export format '{other}'; use prores, h264, h265, dnxhr or image-sequence"
                    );
                    std::process::exit(1);
                }
            };
            let config = ExportConfig {
                input_mxf: PathBuf::from(input),
                output_path: PathBuf::from(output),
                format: fmt,
                quality_crf: crf,
                audio_mxf: audio.map(PathBuf::from),
            };
            export_dcp(&config)
        }

        Commands::Completion { shell } => {
            print!(
                "{}",
                dcpwizard_core::shell_completion::generate_completion(&shell, "dcpwizard")
            );
            0
        }

        Commands::Daemon => {
            let addr = dcpwizard_core::job_queue::daemon_addr();
            println!("Starting dcpwizard daemon on {addr}...");
            let queue = dcpwizard_core::job_queue::JobQueue::new();
            dcpwizard_core::job_queue::start_daemon_ipc(&queue)
        }

        Commands::SubtitleConvert {
            input,
            output,
            language,
            fps,
            vposition,
        } => {
            let input_path = PathBuf::from(&input);
            let output_path = PathBuf::from(&output);
            if !input_path.exists() {
                tracing::error!("Input file not found: {input}");
                std::process::exit(1);
            }
            match dcpwizard_core::subtitle::convert_srt_to_dcp_xml(
                &input_path,
                &output_path,
                &language,
                fps,
                vposition,
            ) {
                Ok(()) => {
                    tracing::info!(
                        "Converted {} -> {} (lang={}, fps={}, vposition={})",
                        input,
                        output,
                        language,
                        fps,
                        vposition
                    );
                    0
                }
                Err(e) => {
                    tracing::error!("Subtitle conversion failed: {e}");
                    1
                }
            }
        }

        Commands::Burnin {
            input,
            subtitles,
            output,
            font_size,
        } => {
            let opts = dcpwizard_core::burnin::BurninOptions {
                input: PathBuf::from(&input),
                output: PathBuf::from(&output),
                subtitle_file: Some(PathBuf::from(&subtitles)),
                font_size,
                position: "bottom".to_string(),
                ..Default::default()
            };
            match dcpwizard_core::burnin::burnin(&opts) {
                Ok(()) => {
                    tracing::info!("Burned subtitles into: {output}");
                    0
                }
                Err(e) => {
                    tracing::error!("Burn-in failed: {e}");
                    1
                }
            }
        }

        Commands::Convert {
            input,
            output,
            target,
            method,
        } => {
            // Parse target resolution
            let (tw, th) = match target.to_lowercase().as_str() {
                "2k-scope" => (2048, 858),
                "2k-flat" => (1998, 1080),
                "2k-full" => (2048, 1080),
                "4k-scope" => (4096, 1716),
                "4k-flat" => (3996, 2160),
                "4k-full" => (4096, 2160),
                _ => {
                    tracing::error!(
                        "Unknown target: {target}. Use: 2k-scope, 2k-flat, 2k-full, 4k-scope, 4k-flat, 4k-full"
                    );
                    std::process::exit(1);
                }
            };

            let vf = match method.to_lowercase().as_str() {
                "letterbox" => format!(
                    "scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2"
                ),
                "crop" => {
                    format!("scale={tw}:{th}:force_original_aspect_ratio=increase,crop={tw}:{th}")
                }
                "scale" => format!("scale={tw}:{th}"),
                _ => {
                    tracing::error!("Unknown method: {method}. Use: letterbox, crop, or scale");
                    std::process::exit(1);
                }
            };

            let status = std::process::Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(&input)
                .arg("-vf")
                .arg(&vf)
                .arg("-c:a")
                .arg("copy")
                .arg(&output)
                .status();
            match status {
                Ok(s) if s.success() => {
                    tracing::info!("Converted to {target} ({method}): {output} ({}x{})", tw, th);
                    0
                }
                Ok(s) => {
                    tracing::error!("ffmpeg exited with code {}", s.code().unwrap_or(-1));
                    1
                }
                Err(e) => {
                    tracing::error!("Failed to run ffmpeg: {e}");
                    1
                }
            }
        }

        Commands::Dcdm {
            input,
            output,
            colour_space,
            lut,
        } => {
            let cs = parse_colour_space(&colour_space);
            let opts = postkit::dcdm::DcdmOptions {
                input_dir: std::path::PathBuf::from(&input),
                output_dir: std::path::PathBuf::from(&output),
                encoding: postkit::dcdm::DcdmColourEncoding::Xyz12Bit,
                width: 0,
                height: 0,
                fps_num: 24,
                fps_den: 1,
                colour_space: format!("{cs:?}"),
                lut_path: lut.map(std::path::PathBuf::from).unwrap_or_default(),
            };
            let result = postkit::dcdm::create_dcdm(&opts);
            if result.success {
                tracing::info!("DCDM created: {} frames written", result.frames_written);
                0
            } else {
                tracing::error!("DCDM creation failed: {}", result.error);
                1
            }
        }

        Commands::Colour {
            input,
            output,
            source,
            target,
            lut,
        } => {
            // X'Y'Z' (DCDM) is not an ffmpeg colorspace-filter target; route it
            // through the real Rec.709/P3/Rec.2020 -> DCI X'Y'Z' transform in the
            // dcdm module (fails loud on an unsupported source there).
            if parse_colour_space(&target) == postkit::colour::ColourSpace::Xyz {
                let opts = postkit::dcdm::DcdmOptions {
                    input_dir: std::path::PathBuf::from(&input),
                    output_dir: std::path::PathBuf::from(&output),
                    encoding: postkit::dcdm::DcdmColourEncoding::Xyz12Bit,
                    width: 0,
                    height: 0,
                    fps_num: 24,
                    fps_den: 1,
                    colour_space: source.clone(),
                    lut_path: lut.map(std::path::PathBuf::from).unwrap_or_default(),
                };
                let result = postkit::dcdm::create_dcdm(&opts);
                if result.success {
                    tracing::info!(
                        "Colour converted {source} -> xyz (DCDM): {} frames written",
                        result.frames_written
                    );
                    0
                } else {
                    tracing::error!("Colour conversion failed: {}", result.error);
                    1
                }
            } else {
                let opts = postkit::colour::ColourConvertOptions {
                    input: std::path::PathBuf::from(&input),
                    output: std::path::PathBuf::from(&output),
                    source_space: parse_colour_space(&source),
                    target_space: parse_colour_space(&target),
                    lut_path: lut.map(std::path::PathBuf::from),
                };
                match postkit::colour::convert_colour(&opts) {
                    Ok(()) => {
                        tracing::info!("Colour converted {source} -> {target}: {output}");
                        0
                    }
                    Err(e) => {
                        tracing::error!("Colour conversion failed: {e}");
                        1
                    }
                }
            }
        }

        Commands::Conform { input, json } => {
            match postkit::conform::parse_timeline(std::path::Path::new(&input)) {
                Ok(timeline) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&timeline).unwrap());
                    } else {
                        println!("Timeline: {}", timeline.title);
                        println!("Format: {:?}", timeline.format);
                        println!("Frame rate: {}", timeline.frame_rate);
                        println!("Events: {}", timeline.events.len());
                        for (i, evt) in timeline.events.iter().enumerate() {
                            println!("  [{i}] {} -> {}", evt.source_in, evt.source_out);
                        }
                    }
                    0
                }
                Err(e) => {
                    tracing::error!("Timeline parse failed: {e}");
                    1
                }
            }
        }

        Commands::Ingest {
            source,
            output,
            format,
            colour_space,
        } => {
            let opts = postkit::ingest::IngestOptions {
                source: std::path::PathBuf::from(&source),
                output_dir: std::path::PathBuf::from(&output),
                output_format: format,
                colour_space,
                debayer_quality: 3,
                apply_lut: false,
                lut_path: std::path::PathBuf::new(),
                gpu_device: -1,
            };
            postkit::ingest::ingest(&opts)
        }

        Commands::FrameExtract {
            input,
            frame,
            output,
        } => postkit::preview::extract_frame(
            std::path::Path::new(&input),
            frame,
            std::path::Path::new(&output),
        ),

        Commands::DvInject { input, rpu, output } => {
            let opts = postkit::dolby_vision::DolbyVisionOptions {
                input: std::path::PathBuf::from(&input),
                rpu_file: std::path::PathBuf::from(&rpu),
                profile: postkit::dolby_vision::DolbyVisionProfile::Profile8,
                output: std::path::PathBuf::from(&output),
                embed_rpu: true,
            };
            postkit::dolby_vision::inject_dolby_vision(&opts)
        }

        Commands::Hdr10Inject {
            input,
            output,
            max_cll,
            max_fall,
        } => {
            let opts = postkit::dolby_vision::HdrMetadataOptions {
                input: std::path::PathBuf::from(&input),
                hdr_type: postkit::dolby_vision::HdrType::Hdr10,
                hdr10: postkit::dolby_vision::Hdr10Metadata {
                    display_primaries_rx: 13250,
                    display_primaries_ry: 34500,
                    display_primaries_gx: 7500,
                    display_primaries_gy: 3000,
                    display_primaries_bx: 34000,
                    display_primaries_by: 16000,
                    white_point_x: 15635,
                    white_point_y: 16450,
                    max_luminance: 10000000,
                    min_luminance: 1,
                    max_cll,
                    max_fall,
                },
                dolby_vision_xml: std::path::PathBuf::new(),
                output: std::path::PathBuf::from(&output),
            };
            postkit::dolby_vision::inject_hdr10_metadata(&opts)
        }

        Commands::Watermark {
            input,
            output,
            payload,
        } => {
            match dcpwizard_core::watermark::embed_watermark(
                PathBuf::from(&input),
                PathBuf::from(&output),
                &payload,
            ) {
                Ok(()) => {
                    tracing::info!("Visible watermark burned into: {output}");
                    0
                }
                Err(e) => {
                    tracing::error!("Watermark failed: {e}");
                    1
                }
            }
        }

        Commands::Certificate { action } => match action {
            CertAction::Chain {
                organization,
                output,
            } => {
                let output_dir = PathBuf::from(&output);
                dcpwizard_core::certificate::generate_chain(&organization, &output_dir)
            }
            CertAction::Generate {
                cert_type,
                cn,
                organization,
                output_cert,
                output_key,
                issuer_cert,
                issuer_key,
                key_bits,
                validity_days,
            } => {
                let ct = match cert_type.to_lowercase().as_str() {
                    "root" => dcpwizard_core::certificate::CertType::Root,
                    "intermediate" => dcpwizard_core::certificate::CertType::Intermediate,
                    "leaf" => dcpwizard_core::certificate::CertType::Leaf,
                    _ => dcpwizard_core::certificate::CertType::Signer,
                };
                let opts = dcpwizard_core::certificate::CertOptions {
                    cert_type: ct,
                    common_name: cn,
                    organization,
                    output_cert: PathBuf::from(&output_cert),
                    output_key: PathBuf::from(&output_key),
                    issuer_cert: issuer_cert.map(PathBuf::from).unwrap_or_default(),
                    issuer_key: issuer_key.map(PathBuf::from).unwrap_or_default(),
                    key_bits,
                    validity_days,
                    ..Default::default()
                };
                dcpwizard_core::certificate::generate_certificate(&opts)
            }
            CertAction::Inspect { cert_file } => {
                let info = dcpwizard_core::certificate::read_certificate(Path::new(&cert_file));
                println!("Subject CN:  {}", info.subject_cn);
                println!("Issuer CN:   {}", info.issuer_cn);
                println!("Serial:      {}", info.serial);
                println!("Not Before:  {}", info.not_before);
                println!("Not After:   {}", info.not_after);
                println!("Key Size:    {} bits", info.key_bits);
                println!("Is CA:       {}", info.is_ca);
                println!("Expired:     {}", info.is_expired);
                println!("Thumbprint:  {}", info.thumbprint_sha1);
                0
            }
        },

        Commands::Batch { action } => {
            use dcpwizard_core::job_queue::{IpcRequest, IpcResponse, send_ipc_request};

            match action {
                BatchAction::List => match send_ipc_request(&IpcRequest::List) {
                    Ok(IpcResponse::Jobs(jobs)) => {
                        if jobs.is_empty() {
                            println!("No jobs in queue");
                        } else {
                            println!(
                                "{:<38} {:<12} {:<10} {:<14} Message",
                                "ID", "State", "Progress", "Type"
                            );
                            for j in &jobs {
                                println!(
                                    "{:<38} {:?} {:<10}% {:?} {}",
                                    j.id, j.state, j.progress_percent, j.job_type, j.message
                                );
                            }
                        }
                        0
                    }
                    Ok(IpcResponse::Error(e)) => {
                        tracing::error!("{e}");
                        1
                    }
                    Err(e) => {
                        tracing::error!("{e}");
                        1
                    }
                    _ => 1,
                },
                BatchAction::Add { r#type, params } => {
                    if !dcpwizard_core::job_queue::is_daemon_running() {
                        tracing::error!("Daemon is not running. Start it with: dcpwizard daemon");
                        std::process::exit(1);
                    }
                    let job_type = match r#type.as_str() {
                        "create-dcp" => dcpwizard_core::job_queue::JobType::CreateDcp,
                        "verify-dcp" => dcpwizard_core::job_queue::JobType::VerifyDcp,
                        "export-dcp" => dcpwizard_core::job_queue::JobType::ExportDcp,
                        "import-video" => dcpwizard_core::job_queue::JobType::ImportVideo,
                        "encode-j2k" => dcpwizard_core::job_queue::JobType::EncodeJ2k,
                        "wrap-mxf" => dcpwizard_core::job_queue::JobType::WrapMxf,
                        "copy-to-drive" => dcpwizard_core::job_queue::JobType::CopyToDrive,
                        other => {
                            tracing::error!("Unknown job type: {other}");
                            std::process::exit(1);
                        }
                    };
                    match send_ipc_request(&IpcRequest::Submit { job_type, params }) {
                        Ok(IpcResponse::Submitted { id }) => {
                            println!("Submitted job {id}");
                            0
                        }
                        Ok(IpcResponse::Error(e)) => {
                            tracing::error!("{e}");
                            1
                        }
                        Err(e) => {
                            tracing::error!("{e}");
                            1
                        }
                        _ => 1,
                    }
                }
                BatchAction::Cancel { id } => {
                    match send_ipc_request(&IpcRequest::Cancel { id: id.clone() }) {
                        Ok(IpcResponse::Cancelled(true)) => {
                            println!("Cancelled job {id}");
                            0
                        }
                        Ok(IpcResponse::Cancelled(false)) => {
                            println!("Could not cancel job {id}");
                            1
                        }
                        Ok(IpcResponse::Error(e)) => {
                            tracing::error!("{e}");
                            1
                        }
                        Err(e) => {
                            tracing::error!("{e}");
                            1
                        }
                        _ => 1,
                    }
                }
            }
        }

        Commands::KdmBatch {
            cpl_id,
            content_title,
            certs,
            cert_dir,
            signer_cert,
            signer_key,
            signer_chain,
            output_dir,
            valid_from,
            valid_to,
            keys,
            format,
        } => {
            let format = match dcpwizard_core::kdm::parse_format(&format) {
                Ok(f) => f,
                Err(e) => {
                    tracing::error!("{e}");
                    std::process::exit(1);
                }
            };
            let mut certs: Vec<String> = certs;
            if let Some(dir) = cert_dir {
                match dcpwizard_core::kdm::certs_in_dir(&PathBuf::from(&dir)) {
                    Ok(found) => certs.extend(found),
                    Err(e) => {
                        tracing::error!("{e}");
                        std::process::exit(1);
                    }
                }
            }
            if certs.is_empty() {
                tracing::error!(
                    "No recipient certificates provided (use --cert and/or --cert-dir)"
                );
                std::process::exit(1);
            }
            let content_keys = match keys {
                Some(path) => {
                    match dcpwizard_core::kdm::load_content_keys(&PathBuf::from(path), &cpl_id) {
                        Ok(k) => k,
                        Err(e) => {
                            tracing::error!("{e}");
                            std::process::exit(1);
                        }
                    }
                }
                None => Vec::new(),
            };
            dcpwizard_core::kdm::generate_kdm_batch(
                cpl_id,
                content_title,
                certs.into_iter().map(PathBuf::from).collect(),
                PathBuf::from(signer_cert),
                PathBuf::from(signer_key),
                signer_chain.into_iter().map(PathBuf::from).collect(),
                valid_from,
                valid_to,
                content_keys,
                PathBuf::from(output_dir),
                format,
            )
        }

        Commands::Trailer {
            content,
            output,
            title,
            rating,
            rating_system,
            band,
            countdown,
            fps,
        } => {
            let opts = postkit::trailer::TrailerOptions {
                content_dir: PathBuf::from(&content),
                audio_file: PathBuf::new(),
                output_dir: PathBuf::from(&output),
                title,
                rating,
                rating_system: match rating_system.to_lowercase().as_str() {
                    "bbfc" => postkit::trailer::RatingSystem::Bbfc,
                    "fsk" => postkit::trailer::RatingSystem::Fsk,
                    "custom" => postkit::trailer::RatingSystem::Custom,
                    _ => postkit::trailer::RatingSystem::Mpaa,
                },
                band: match band.to_lowercase().as_str() {
                    "red" => postkit::trailer::TrailerBand::Red,
                    "yellow" => postkit::trailer::TrailerBand::Yellow,
                    _ => postkit::trailer::TrailerBand::Green,
                },
                countdown_seconds: countdown,
                fps_num: fps,
                fps_den: 1,
            };
            let result = postkit::trailer::package_trailer(&opts);
            if result.success {
                tracing::info!(
                    "Trailer packaged: {} ({})",
                    result.output_dir.display(),
                    result.output_file.display()
                );
                0
            } else {
                tracing::error!("Trailer packaging failed: {}", result.error);
                1
            }
        }

        Commands::Markers { frames, xml } => {
            let markers = dcpwizard_core::markers::default_markers(frames);
            if xml {
                println!("{}", dcpwizard_core::markers::markers_to_xml(&markers));
            } else if markers.is_empty() {
                println!("No markers (composition length is 0 frames)");
            } else {
                for m in &markers {
                    println!("{}\t{}", m.marker.label(), m.frame);
                }
            }
            0
        }

        Commands::Accessibility { dcp_dir, standard } => {
            let std_val = match standard.to_lowercase().as_str() {
                "eaa" => postkit::accessibility::AccessibilityStandard::Eaa,
                "aoda" => postkit::accessibility::AccessibilityStandard::Aoda,
                "ofcom" => postkit::accessibility::AccessibilityStandard::Ofcom,
                _ => postkit::accessibility::AccessibilityStandard::Cvaa,
            };
            let result =
                dcpwizard_core::accessibility::check_accessibility(Path::new(&dcp_dir), std_val);
            println!("Standard:  {:?}", result.standard);
            println!("Compliant: {}", result.compliant);
            println!("Errors:    {}", result.errors);
            println!("Warnings:  {}", result.warnings);
            for f in &result.findings {
                println!(
                    "  [{:?}] {} ({:?}): {}",
                    f.severity, f.rule_id, f.track_type, f.description
                );
            }
            if result.compliant { 0 } else { 1 }
        }

        Commands::Webhook {
            url,
            event,
            job_id,
            secret,
            payload,
        } => {
            let config = postkit::webhook::WebhookConfig {
                url,
                secret,
                ..Default::default()
            };
            let result = if payload.is_empty() && event == "ping" {
                postkit::webhook::test_webhook(&config)
            } else {
                let evt = postkit::webhook::WebhookEvent {
                    event_type: event,
                    job_id,
                    payload_json: payload,
                    timestamp: String::new(),
                };
                postkit::webhook::send_webhook(&config, &evt)
            };
            if result.success {
                tracing::info!(
                    "Webhook delivered (HTTP {}, {} attempt(s))",
                    result.http_status,
                    result.attempts
                );
                0
            } else {
                tracing::error!(
                    "Webhook failed after {} attempt(s): {}",
                    result.attempts,
                    result.error
                );
                1
            }
        }

        Commands::Version { action } => match action {
            VersionAction::Record {
                db,
                package_uuid,
                title,
                version,
                destination,
                method,
                verified,
            } => {
                let mut tracker = dcpwizard_core::version_tracker::VersionTracker::new();
                if !tracker.open(Path::new(&db)) {
                    tracing::error!("Failed to open tracker database: {db}");
                    std::process::exit(1);
                }
                let record = dcpwizard_core::version_tracker::DeliveryRecord {
                    package_uuid,
                    title,
                    version,
                    destination,
                    delivery_method: method,
                    timestamp: dcpwizard_core::version_tracker::now_iso(),
                    verified,
                };
                if tracker.record(&record) {
                    println!("Recorded delivery of {}", record.package_uuid);
                    0
                } else {
                    tracing::error!("Failed to record delivery");
                    1
                }
            }
            VersionAction::List {
                db,
                package_uuid,
                destination,
            } => {
                let mut tracker = dcpwizard_core::version_tracker::VersionTracker::new();
                if !tracker.open(Path::new(&db)) {
                    tracing::error!("Failed to open tracker database: {db}");
                    std::process::exit(1);
                }
                let query = dcpwizard_core::version_tracker::VersionQuery {
                    package_uuid,
                    destination,
                    ..Default::default()
                };
                let records = tracker.query(&query);
                if records.is_empty() {
                    println!("No deliveries recorded");
                } else {
                    for r in &records {
                        println!(
                            "{}  {}  {}  -> {}  ({}, verified={})",
                            r.timestamp,
                            r.package_uuid,
                            r.title,
                            r.destination,
                            r.delivery_method,
                            r.verified
                        );
                    }
                }
                0
            }
            VersionAction::Export { db, output } => {
                let mut tracker = dcpwizard_core::version_tracker::VersionTracker::new();
                if !tracker.open(Path::new(&db)) {
                    tracing::error!("Failed to open tracker database: {db}");
                    std::process::exit(1);
                }
                let out = PathBuf::from(&output);
                let ok = if output.to_lowercase().ends_with(".csv") {
                    tracker.export_csv(&out)
                } else {
                    tracker.export_json(&out)
                };
                if ok {
                    println!("Exported delivery history to {output}");
                    0
                } else {
                    tracing::error!("Failed to export delivery history");
                    1
                }
            }
        },

        Commands::Dashboard { action } => {
            // register/list/status/matrix operate on postkit's default database;
            // ensure its schema exists first.
            let db_path = dcpwizard_core::dashboard::default_db_path();
            if dcpwizard_core::dashboard::init_database(&db_path) != 0 {
                tracing::error!("Failed to initialise dashboard database");
                std::process::exit(1);
            }
            match action {
                DashboardAction::Register {
                    uuid,
                    title,
                    version_type,
                    territory,
                    language,
                    standard,
                    dcp_path,
                    status,
                    kdm_recipients,
                } => {
                    let entry = dcpwizard_core::dashboard::VersionEntry {
                        uuid,
                        title,
                        version_type,
                        territory,
                        language,
                        standard,
                        dcp_path: PathBuf::from(dcp_path),
                        ov_uuid: String::new(),
                        created_date: dcpwizard_core::version_tracker::now_iso(),
                        status,
                        kdm_recipients,
                    };
                    if dcpwizard_core::dashboard::register_version(&entry) == 0 {
                        println!("Registered version {}", entry.uuid);
                        0
                    } else {
                        1
                    }
                }
                DashboardAction::List { territory, status } => {
                    let versions = dcpwizard_core::dashboard::list_versions(
                        territory.as_deref(),
                        status.as_deref(),
                    );
                    if versions.is_empty() {
                        println!("No versions registered");
                    } else {
                        for v in &versions {
                            println!(
                                "{}  {}  {}  {}  [{}]",
                                v.uuid, v.title, v.version_type, v.territory, v.status
                            );
                        }
                    }
                    0
                }
                DashboardAction::Status { uuid, status } => {
                    if dcpwizard_core::dashboard::update_status(&uuid, &status) == 0 {
                        println!("Updated {uuid} -> {status}");
                        0
                    } else {
                        tracing::error!("Failed to update status (unknown UUID?)");
                        1
                    }
                }
                DashboardAction::Matrix { output } => {
                    if dcpwizard_core::dashboard::export_distribution_matrix(Path::new(&output))
                        == 0
                    {
                        println!("Exported distribution matrix to {output}");
                        0
                    } else {
                        tracing::error!("Failed to export distribution matrix");
                        1
                    }
                }
                DashboardAction::Serve { port, bind } => {
                    let opts = dcpwizard_core::dashboard::DashboardOptions {
                        database_path: db_path,
                        http_port: port,
                        bind_address: bind,
                    };
                    dcpwizard_core::dashboard::serve_dashboard(&opts)
                }
            }
        }

        Commands::IngestPackage { dir } => {
            let code = dcpwizard_core::ingest_package::ingest_package(&PathBuf::from(&dir));
            if code == 0 {
                println!("Repackaged {dir} (regenerated ASSETMAP and PKL)");
            }
            code
        }

        Commands::CreateVf {
            ov,
            output,
            title,
            replace_picture,
            replace_sound,
        } => {
            // Parse REEL=PATH into a per-reel map, picture and sound sharing reels.
            let mut reels: std::collections::BTreeMap<u32, dcpwizard_core::vf::ReplacementReel> =
                std::collections::BTreeMap::new();
            let mut parse_ok = true;
            for (specs, is_picture) in [(&replace_picture, true), (&replace_sound, false)] {
                for spec in specs {
                    let Some((reel_str, path)) = spec.split_once('=') else {
                        tracing::error!("bad --replace spec '{spec}', expected REEL=PATH");
                        parse_ok = false;
                        continue;
                    };
                    let Ok(reel_number) = reel_str.trim().parse::<u32>() else {
                        tracing::error!("bad reel number in '{spec}'");
                        parse_ok = false;
                        continue;
                    };
                    let entry =
                        reels
                            .entry(reel_number)
                            .or_insert(dcpwizard_core::vf::ReplacementReel {
                                reel_number,
                                ..Default::default()
                            });
                    let p = Some(PathBuf::from(path.trim()));
                    if is_picture {
                        entry.picture = p;
                    } else {
                        entry.sound = p;
                    }
                }
            }

            if !parse_ok {
                1
            } else {
                let config = dcpwizard_core::vf::VfConfig {
                    ov_dir: PathBuf::from(&ov),
                    vf_dir: PathBuf::from(&output),
                    title,
                    replacement_reels: reels.into_values().collect(),
                };
                let code = dcpwizard_core::vf::create_vf(&config);
                if code == 0 {
                    println!("Created VF DCP at {output}");
                }
                code
            }
        }
    };

    postkit::grok_encoder::deinitialize();
    std::process::exit(code);
}
