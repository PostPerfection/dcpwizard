# DCP Wizard

[![CI](https://github.com/PostPerfection/dcpwizard/actions/workflows/ci.yml/badge.svg)](https://github.com/PostPerfection/dcpwizard/actions/workflows/ci.yml)

[Documentation](https://postperfection.github.io/dcpwizard/)

Digital Cinema Package (DCP) creator — CLI tool and desktop GUI.

## Overview

DCP Wizard creates valid DCPs (Digital Cinema Packages) from video sources,
image sequences, and WAV audio, conforming to SMPTE ST 429 (Bv2.1) and Interop standards.

Free and open-source alternative to easyDCP Creator+ (€2,998).

## Features

### DCP Creation & Packaging
- **Original Version (OV) DCP** creation from J2K + WAV
- **Version File (VF)** creation referencing original reels
- **Multi-CPL timeline** — multiple compositions sharing track assets
- **SMPTE Bv2.1 & Interop** standard support
- **2K and 4K** resolution (2048×1080, 4096×2160)
- **High Frame Rate (HFR)** — 24, 25, 30, 48, 60, 96, 120 fps
- **High Bitrate (HBR)** — up to 500 Mbps for demanding content
- **Automatic reel splitting** by duration or file size
- **CPL / PKL / ASSETMAP / VOLINDEX** generation
- **SHA-1 hashing** for integrity verification

### Encoding & Transcoding
- **JPEG 2000 encoding** via grok (GPU and CPU)
- **Video file import** — QuickTime (.mov), MP4, MXF, AVI, MKV, MJ2
- **Video transcoding** — ProRes, H.264, H.265, DNxHR → image sequence → J2K (via ffmpeg)
- **Image sequence input** — DPX, TIFF, EXR, PNG, BMP, JPEG
- **J2K transcoder** — re-encode existing JPEG 2000 at a different bitrate
- **Scale / Crop / Letterbox / Pillarbox** — target resolution adaptation
- **Colour conversion** — Rec.709, P3-D65, P3-DCI → XYZ with gamma 2.6
- **Stereoscopic 3D** — frame interleaving from separate L/R eye sequences

### Encryption & KDM
- **AES-128 content encryption** (SMPTE compliant)
- **KDM generation** — Interop & SMPTE, including Dolby Atmos
- **Batch KDM** — generate for multiple screens in one pass
- **DKDM support** — generate KDMs from Distribution KDMs
- **Time zone support** in KDM validity periods
- **Annotation scheme** — customizable KDM annotation patterns
- **Trusted Device List** support

### Subtitles & Captions
- **SMPTE 428-7 XML subtitles** (CineCanvas) packaging
- **Interop XML subtitles** packaging
- **SRT → SMPTE subtitle** conversion
- **Multilingual subtitles** with RFC 5646 language tags
- **Subtitle burn-in** — permanently render into video frames (for festivals)

### Audio
- **Multi-channel audio** — mono, stereo, 5.1, 7.1
- **Dolby Atmos (IAB)** immersive audio packaging
- **DTS:X** audio packaging
- **Loudness measurement** — EBU R128 / ATSC A/85
- **Channel mapping** configuration
- **WAV and QuickTime audio** input

### Quality Control
- **Integrated QC** via dcpdoctor (SMPTE Bv2.1 compliance checking)
- **HTML QC report** generation
- **DCP verification** — validate structure, hashes, XML schemas
- **Markers** — FFOC, LFOC, FFTC, LFTC, FFOI, LFOI, FFEC, LFEC, FFMC, LFMC

### Export & Playback
- **Export DCP** to ProRes, H.264, H.265, DNxHR, or image sequence
- **Frame extraction** — extract individual frames as images (thumbnails/preview)

### Delivery & Automation
- **Copy to drive** with post-copy hash verification (USB/CRU)
- **Watch folder** mode — automated DCP creation on file arrival
- **Job queue** with progress tracking and cancellation
- **REST API** for headless/batch operation
- **Docker container** for CI/CD pipelines
- **CLI scriptable** — all features accessible from command line

### Mastering & Compliance
- **DCDM creation** — Digital Cinema Distribution Master (X'Y'Z' 12/16-bit) intermediate
- **Forensic watermarking** — NexGuard, Civolution, or internal spatial watermark
- **Trailer packaging** — ratings cards (MPAA/BBFC/FSK), green/red band, countdown leaders
- **Content version tracker** — SQLite database of which version delivered where and when
- **Accessibility compliance** — verify AD/HI/SL tracks against CVAA, EAA, AODA, Ofcom standards

## Installation

```bash
cd rust
cargo build --release
cargo test
```

The Rust workspace uses [postkit](https://github.com/PostPerfection/postkit), [dcpdoctor-core](https://github.com/PostPerfection/dcpdoctor), and [asdcplib-rs](https://github.com/PostPerfection/asdcplib-rs) as dependencies.

### Optional runtime dependencies

| Dependency | Notes |
|-----------|-------|
| ffmpeg | Video transcoding and import |
| grok | JPEG 2000 encoding (GPU and CPU) |

### Docker

```bash
docker build -t dcpwizard .
docker run -v /path/to/media:/data dcpwizard create \
    --title "My Film" --video /data/j2k --audio /data/audio.wav --output /data/dcp
```

### Desktop GUI

The GUI uses [Tauri 2](https://tauri.app/) (Rust backend + web frontend).

```bash
cd gui
npm install
npm run tauri dev      # development mode
npm run tauri build    # production build
```

## CLI Usage

```bash
# Create a DCP
dcpwizard create --title "My Feature Film" --video ./j2k --audio ./audio.wav --output ./dcp

# Create with encryption
dcpwizard create --title "My Film" --video ./j2k --audio ./audio.wav --output ./dcp --encrypt

# Create Interop DCP
dcpwizard create --title "My Film" --video ./j2k --output ./dcp --standard interop

# Encode images to JPEG 2000
dcpwizard encode --input ./dpx --output ./j2k --bandwidth 250 --encoder grok

# Transcode video to image sequence
dcpwizard transcode --input movie.mov --output ./sequence

# Verify an existing DCP
dcpwizard verify ./my_dcp

# Inspect DCP metadata
dcpwizard info ./my_dcp

# Generate KDM
dcpwizard kdm --dcp ./my_dcp --cert recipient.pem --output kdm.xml

# Copy to cinema drive
dcpwizard copy --src ./my_dcp --dst /mnt/cru_drive

# Measure audio loudness
dcpwizard loudness audio.wav

# Generate QC report
dcpwizard report --dcp ./my_dcp --output report.html

# Start REST API server
dcpwizard serve --port 8080

# Watch folder for auto-DCP creation
dcpwizard watch ./incoming

# Shell completion
dcpwizard completion bash >> ~/.bashrc
dcpwizard completion zsh >> ~/.zshrc
dcpwizard completion fish > ~/.config/fish/completions/dcpwizard.fish
```

## REST API

Start the server:
```bash
dcpwizard serve --port 8080
```

Or via Docker:
```bash
docker run -p 8080:8080 -v /path/to/media:/data dcpwizard serve --port 8080
```

## Comparison with easyDCP Creator+

| Feature | DCP Wizard | easyDCP Creator+ |
|---------|-----------|------------------|
| SMPTE Bv2.1 & Interop | ✅ | ✅ |
| Version Files (VF) | ✅ | ✅ |
| Multi-CPL timeline | ✅ | ✅ |
| CLI scriptable | ✅ | ✅ |
| Up to 4K, 2D & 3D | ✅ | ✅ |
| HFR (up to 120fps) | ✅ | ✅ |
| High Bitrate (500 Mbps) | ✅ | ✅ |
| DPX/TIFF/PNG/BMP/J2K/QuickTime | ✅ | ✅ |
| Scale/Crop/Letterbox | ✅ | ✅ |
| J2K Transcoder | ✅ | ✅ |
| Audio 2.0–7.1, Atmos, DTS:X | ✅ | ✅ |
| SRT→SMPTE subtitles | ✅ | ✅ |
| Multilingual CineCanvas | ✅ | ✅ |
| Subtitle burn-in | ✅ | ✅ |
| Integrated QC | ✅ (dcpdoctor) | ✅ (Fraunhofer) |
| KDM (Interop + SMPTE + Atmos) | ✅ | ✅ |
| DKDM, Time Zones, Trusted Devices | ✅ | ✅ |
| DCP Export/Playback | ✅ | ✅ |
| Desktop GUI | ✅ (Tauri) | ✅ (native) |
| REST API / Docker | ✅ | ❌ |
| Watch folder automation | ✅ | ❌ |
| Forensic watermarking | ✅ | ❌ |
| DCDM intermediate format | ✅ | ❌ |
| Trailer packaging (ratings/leaders) | ✅ | ❌ |
| Content version tracking | ✅ | ❌ |
| Accessibility compliance (CVAA/EAA) | ✅ | ❌ |
| Open source | ✅ (GPL-3.0) | ❌ |
| **Price** | **Free** | **€2,998 or €138/mo** |

## Architecture

```
dcpwizard/
├── rust/                # Rust workspace
│   ├── crates/
│   │   ├── dcpwizard-core/  # Core library — DCP creation, encoding, encryption, KDM, QC
│   │   └── dcpwizard-cli/   # CLI binary (dcpwizard)
│   └── Cargo.toml
├── gui/                 # Tauri 2 desktop application
│   ├── src/             # Frontend (Vite + vanilla JS)
│   └── src-tauri/       # Rust backend (plugin shell)
└── docs/                # GitHub Pages site
```

DCP Wizard shares common functionality with [IMF Wizard](https://github.com/DcpDoctor/imfwizard)
via the [postkit](https://github.com/DcpDoctor/postkit) library (encoding, transcoding, hashing,
job queue, preferences, REST API, watch folders, and more).

## License

GPL-3.0 — see [LICENSE](LICENSE).
