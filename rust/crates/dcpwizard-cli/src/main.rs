use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "dcpwizard",
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
        /// J2K encoder: "grok" (default) or "openjpeg"
        #[arg(long, default_value = "grok")]
        encoder: String,
        /// Content type: FTR, SHR, TLR, TST, XSN, RTG, TSR, POL, PSA, ADV
        #[arg(long)]
        content_type: Option<String>,
        /// Container ratio: flat (185), scope (239), full (133)
        #[arg(long)]
        container_ratio: Option<String>,
        /// DCP frame rate (auto-detected from source if not specified)
        #[arg(long)]
        frame_rate: Option<u32>,
        /// Force 2K resolution
        #[arg(long)]
        twok: bool,
        /// Force 4K resolution
        #[arg(long)]
        fourk: bool,
        /// Number of encoding threads (default: auto-detect CPU count)
        #[arg(short = 'j', long)]
        threads: Option<u32>,
        /// J2K bandwidth in Mbit/s (default: 250 for 2K, 500 for 4K)
        #[arg(long)]
        video_bit_rate: Option<u32>,
        /// Number of audio channels
        #[arg(long)]
        audio_channels: Option<u32>,
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
        /// Write report to file (.txt, .html, or .pdf)
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
        /// Output KDM file
        #[arg(short, long)]
        output: String,
        /// Valid from (ISO 8601, e.g. "2024-06-01T00:00:00+00:00") or "now"
        #[arg(short = 'f', long)]
        valid_from: Option<String>,
        /// Valid to (ISO 8601, e.g. "2024-12-31T23:59:59+00:00")
        #[arg(short = 't', long)]
        valid_to: Option<String>,
        /// Valid duration (e.g. "2 weeks", "30 days", "4 hours")
        #[arg(short = 'd', long)]
        valid_duration: Option<String>,
        /// KDM formulation: modified-transitional-1 (default), dci-any, dci-specific
        #[arg(long, default_value = "modified-transitional-1")]
        formulation: String,
        /// Disable forensic marking of picture
        #[arg(long)]
        disable_forensic_marking_picture: bool,
        /// Disable forensic marking of audio
        #[arg(long)]
        disable_forensic_marking_audio: bool,
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

fn main() {
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
            output,
            standard,
            encrypt,
            encoder,
            content_type,
            container_ratio: _container_ratio,
            frame_rate,
            twok,
            fourk,
            threads,
            video_bit_rate,
            audio_channels: _,
            profile: _,
        } => {
            let video_path = PathBuf::from(&video);
            let output_dir = PathBuf::from(&output);
            let std_val = if standard == "interop" {
                dcpwizard_core::Standard::Interop
            } else {
                dcpwizard_core::Standard::Smpte
            };

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

                tracing::info!("Detected video file input — using {} encoder", encoder);

                // Probe video for frame rate and resolution
                let video_info = dcpwizard_core::probe::probe_video(&video_path);
                let fps = frame_rate.unwrap_or_else(|| {
                    video_info
                        .as_ref()
                        .map(|v| v.fps_num / v.fps_den.max(1))
                        .unwrap_or(24)
                });
                let (mut width, mut height, total_frames) = video_info
                    .as_ref()
                    .map(|v| (v.width, v.height, v.total_frames))
                    .unwrap_or((2048, 1080, 0));

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
                let compression_ratio = if let Some(mbps) = video_bit_rate {
                    // DCI J2K: raw = width*height*36 bits/frame
                    // ratio = raw_bits_per_frame / target_bits_per_frame
                    let raw_bits = width as f64 * height as f64 * 36.0;
                    let target_bits = (mbps as f64 * 1_000_000.0) / fps as f64;
                    (raw_bits / target_bits).max(1.0)
                } else {
                    10.0
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

                let result = if encoder == "openjpeg" {
                    postkit::openjpeg_encoder::encode_video_pipeline_opj(
                        &video_path,
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
                    )
                } else {
                    grok_encoder::encode_video_pipeline(
                        &video_path,
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
                    )
                };
                eprintln!();

                if !result.success {
                    tracing::error!("Encode failed: {}", result.error);
                    std::process::exit(1);
                }
                tracing::info!("Encoded {} frames", result.frames_encoded);

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
                    output_dir: output_dir.clone(),
                    frame_rate_num: fps,
                    frame_rate_den: 1,
                    resolution,
                    content_type: ct,
                    max_bitrate_mbps: video_bit_rate.unwrap_or(0),
                    j2k_dir: Some(j2k_dir.clone()),
                    audio_path: audio_path.clone(),
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
                    output_dir,
                    frame_rate_num: frame_rate.unwrap_or(24),
                    frame_rate_den: 1,
                    resolution,
                    content_type: ct,
                    max_bitrate_mbps: video_bit_rate.unwrap_or(0),
                    j2k_dir: Some(video_path),
                    audio_path: audio.map(PathBuf::from),
                    ..Default::default()
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

        Commands::Transcode { input, output } => {
            let config = dcpwizard_core::transcode::TranscodeConfig {
                input_file: PathBuf::from(input),
                output_dir: PathBuf::from(output),
                ..Default::default()
            };
            dcpwizard_core::transcode::transcode_to_sequence(&config)
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
            output,
            valid_from,
            valid_to,
            valid_duration,
            formulation,
            disable_forensic_marking_picture,
            disable_forensic_marking_audio,
        } => {
            let (not_valid_before, not_valid_after) = dcpwizard_core::kdm::resolve_validity_period(
                valid_from.as_deref(),
                valid_to.as_deref(),
                valid_duration.as_deref(),
            );

            let config = dcpwizard_core::kdm::KdmConfig {
                cpl_id,
                content_title,
                recipient_cert_file: PathBuf::from(cert),
                output_file: PathBuf::from(output),
                not_valid_before,
                not_valid_after,
                formulation: dcpwizard_core::kdm::KdmFormulation::parse(&formulation),
                disable_forensic_marking_picture,
                disable_forensic_marking_audio,
                ..Default::default()
            };
            dcpwizard_core::kdm::generate_kdm(&config)
        }

        Commands::Copy { src, dst } => {
            dcpwizard_core::copy_drive::copy_to_drive(&PathBuf::from(src), &PathBuf::from(dst))
        }

        Commands::Loudness { audio_file } => {
            match dcpwizard_core::loudness::measure_loudness(&PathBuf::from(audio_file)) {
                Ok(result) => {
                    tracing::info!("Integrated: {:.1} LUFS", result.integrated_lufs);
                    tracing::info!("True Peak: {:.1} dBTP", result.true_peak_dbtp);
                    tracing::info!("LRA: {:.1} LU", result.loudness_range_lu);
                    0
                }
                Err(e) => {
                    tracing::error!("{e}");
                    1
                }
            }
        }

        Commands::Report { dcp, output } => {
            dcpwizard_core::report::generate_report(&PathBuf::from(dcp), &PathBuf::from(output))
        }

        Commands::Serve { bind } => dcpwizard_core::rest_api::start_rest_api(&bind),

        Commands::Watch { dir } => {
            dcpwizard_core::watch::watch_directory(
                &PathBuf::from(dir),
                std::time::Duration::from_secs(5),
                &|| false,
                |p| {
                    tracing::info!("New DCP detected: {}", p.display());
                },
            );
            0
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
    };

    postkit::grok_encoder::deinitialize();
    std::process::exit(code);
}
