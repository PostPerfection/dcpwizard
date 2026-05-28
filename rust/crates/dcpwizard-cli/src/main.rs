use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
        /// Recipient certificate
        #[arg(long)]
        cert: String,
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
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let code = match cli.command {
        Commands::Create {
            title,
            video,
            audio,
            output,
            standard,
            encrypt,
            ..
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
                use postkit::encode::{StreamEncodeOptions, find_compressor, stream_encode};
                use std::sync::Arc;
                use std::sync::atomic::AtomicBool;

                let (compressor_path, lib_dir) = match find_compressor() {
                    Some(c) => c,
                    None => {
                        tracing::error!("grk_compress not found (required for video encoding)");
                        std::process::exit(1);
                    }
                };

                let _ = std::fs::create_dir_all(&output_dir);
                let j2k_dir = output_dir.join("j2k");
                let _ = std::fs::create_dir_all(&j2k_dir);

                tracing::info!("Detected video file input — running full pipeline");
                tracing::info!("Compressor: {}", compressor_path.display());

                // Probe video for frame rate and resolution
                let video_info = dcpwizard_core::probe::probe_video(&video_path);
                let fps = video_info
                    .as_ref()
                    .map(|v| v.fps_num / v.fps_den.max(1))
                    .unwrap_or(24);
                if let Some(ref info) = video_info {
                    tracing::info!(
                        "Input: {}x{} @ {}/{} fps",
                        info.width,
                        info.height,
                        info.fps_num,
                        info.fps_den
                    );
                }
                let opts = StreamEncodeOptions {
                    input: video_path.clone(),
                    output_dir: j2k_dir.clone(),
                    compression_ratio: 10.0,
                    num_resolutions: 6,
                    codeblock_size: 32,
                    progression: "CPRL".to_string(),
                    fps,
                    compressor_path,
                    lib_dir,
                };

                let cancel = Arc::new(AtomicBool::new(false));
                let pause = Arc::new(AtomicBool::new(false));
                let cancel_clone = cancel.clone();
                let _ = ctrlc::set_handler(move || {
                    cancel_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                });

                let result = stream_encode(&opts, &cancel, &pause, |p| {
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

                let config = dcpwizard_core::dcp::DcpConfig {
                    title,
                    standard: std_val,
                    encrypt,
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
            } else {
                // Input is a J2K directory or image sequence
                let config = dcpwizard_core::dcp::DcpConfig {
                    title,
                    standard: std_val,
                    encrypt,
                    output_dir,
                    frame_rate_num: 24,
                    frame_rate_den: 1,
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
            use postkit::encode::{StreamEncodeOptions, find_compressor, stream_encode};
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

            let (compressor_path, lib_dir) = match find_compressor() {
                Some(c) => c,
                None => {
                    tracing::error!("grk_compress not found");
                    std::process::exit(1);
                }
            };

            tracing::info!("Pipeline: {} -> {}", input, output);
            tracing::info!("Compressor: {}", compressor_path.display());

            let opts = StreamEncodeOptions {
                input: input_path.clone(),
                output_dir: j2k_dir.clone(),
                compression_ratio: ratio,
                num_resolutions: 6,
                codeblock_size: 32,
                progression: "CPRL".to_string(),
                fps,
                compressor_path,
                lib_dir,
            };

            let cancel = Arc::new(AtomicBool::new(false));
            let pause = Arc::new(AtomicBool::new(false));

            // Handle Ctrl+C
            let cancel_clone = cancel.clone();
            let _ = ctrlc::set_handler(move || {
                cancel_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            });

            let result = stream_encode(&opts, &cancel, &pause, |p| {
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

        Commands::Verify { dcp_dir } => {
            let result = dcpwizard_core::verify::verify_dcp(&PathBuf::from(dcp_dir));
            if result.valid {
                tracing::info!("DCP verification PASSED");
                0
            } else {
                for e in &result.errors {
                    tracing::error!("{e}");
                }
                for w in &result.warnings {
                    tracing::warn!("{w}");
                }
                1
            }
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
        } => {
            let config = dcpwizard_core::kdm::KdmConfig {
                cpl_id,
                content_title,
                recipient_cert_file: PathBuf::from(cert),
                output_file: PathBuf::from(output),
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

    std::process::exit(code);
}
