# DCP Wizard

[![CI](https://github.com/PostPerfection/dcpwizard/actions/workflows/ci.yml/badge.svg)](https://github.com/PostPerfection/dcpwizard/actions/workflows/ci.yml)

[Documentation](https://postperfection.github.io/dcpwizard/)

Digital Cinema Package (DCP) creator, CLI tool and desktop GUI.

Version 1.1 creates consistent CPL, PKL, and ASSETMAP identities for SMPTE and Interop packages. OpenJPEG is the default encoder, and Grok remains optional.

## Overview

DCP Wizard creates valid DCPs (Digital Cinema Packages) from video sources,
image sequences, and WAV audio, conforming to SMPTE ST 429 (Bv2.1) and Interop standards.

Free and open-source alternative to easyDCP Creator+ (€2,998).

## Features

### DCP Creation & Packaging
- **Original Version (OV) DCP** creation from J2K + WAV
- **SMPTE & Interop** standard support
- **2K and 4K** resolution (2048×1080, 4096×2160)
- **Frame rates** 24, 25, 30, 48, 60 fps
- **High Bitrate (HBR)**, up to 500 Mbps for demanding content
- **CPL / PKL / ASSETMAP / VOLINDEX** generation
- **SHA-1 hashing** for integrity verification

### Encoding & Transcoding
- **JPEG 2000 encoding** via OpenJPEG (create video path) or Grok (streaming pipeline)
- **Video file import**, QuickTime (.mov), MP4, MXF, AVI, MKV
- **Video transcoding**, ProRes, H.264, H.265, DNxHR → image sequence → J2K (via ffmpeg)
- **Image sequence input**, DPX, TIFF, EXR, PNG
- **Scale / Crop / Letterbox**, target resolution adaptation
- **Colour conversion** to XYZ (DCI, gamma 2.6)

### Encryption & KDM
- **AES-128 essence encryption**, content keys generated with a CSPRNG, encrypted at wrap time
- **Signed SMPTE KDM** (ST 430-1 / 430-3) carrying the DCP's image and audio content keys
- **Batch KDM**, generate for multiple screens in one pass
- **DKDM re-wrap**, re-issue KDMs from a Distribution KDM
- **Certificate generation**, X.509 cert chain (root → intermediate → signer)
- **Certificate inspection**, display subject, issuer, validity, thumbprint, CA status

### Subtitles & Captions
- **SRT → SMPTE / Interop subtitle XML** conversion
- **Multilingual subtitles** with RFC 5646 language tags
- **Subtitle burn-in**, permanently render into video frames (for festivals)

### Audio
- **PCM audio wrapping** (48 kHz)
- **Loudness measurement**, EBU R128 / ATSC A/85
- **WAV audio** input

### Quality Control
- **Integrated QC** via dcpdoctor (SMPTE Bv2.1 compliance checking)
- **HTML QC report** generation
- **DCP verification**, validate structure, hashes, XML schemas
- **Markers**, FFOC, LFOC, FFTC, LFTC, FFOI, LFOI, FFEC, LFEC, FFMC, LFMC

### HDR & Dolby Vision
- **HDR detection**, auto-detect SDR, HDR10, HDR10+, Dolby Vision, HLG from source
- **Dolby Vision RPU injection** via dovi_tool
- **HDR10 static metadata** injection (SMPTE ST 2086 + CTA 861.3)
- **HDR format conversion**, HDR10 ↔ HLG ↔ SDR tone mapping

### Camera Ingest
- **Camera raw detection**, ARRIRAW, RED R3D, Blackmagic BRAW, Sony, Canon
- **Media scanning**, auto-detect resolution, frame rate, codec, reel names
- **Transcode to intermediate**, DPX, TIFF, EXR, ProRes 4444 (via ffmpeg)
- **3D LUT application** during ingest
- **Timeline conform**, assemble reels from EDL (CMX 3600), AAF, FCP XML, OTIO

### Export & Playback
- **Export DCP** to ProRes, H.264, H.265, DNxHR, or image sequence
- **Frame extraction**, extract individual frames as images (thumbnails/preview)
- **Frame-accurate preview** with display colourspace selection

### Delivery & Automation
- **Copy to drive** with post-copy hash verification (USB/CRU)
- **Watch folder** mode, automated DCP creation on file arrival
- **Job queue** with progress tracking and cancellation
- **REST API** for headless/batch operation
- **Prometheus metrics endpoint** (`GET /metrics`), job counts, daemon status for monitoring
- **Docker container** for CI/CD pipelines
- **CLI scriptable**, all features accessible from command line
- **Version dashboard**, OV/VF management, territory tracking, distribution matrix export
- **Webhook notifications**, HTTP callbacks on job completion/failure

### Mastering & Compliance
- **DCDM creation**, Digital Cinema Distribution Master (X'Y'Z' 12/16-bit) intermediate
- **Visible watermarking**, burned-in text mark (distributor ID/serial) across image frames
- **Trailer packaging**, ratings cards (MPAA/BBFC/FSK), green/red band, countdown leaders
- **Content version tracker**, SQLite database of which version delivered where and when
- **Accessibility compliance**, verify AD/HI/SL tracks against CVAA, EAA, AODA, Ofcom standards

## Installation

### Pre-built binaries (recommended)

Download from the [GitHub Releases](https://github.com/PostPerfection/dcpwizard/releases/latest) page:

| Platform | CLI | Desktop GUI |
|----------|-----|-------------|
| **Linux** (x86_64) | `dcpwizard-linux-x86_64.tar.gz` | `.deb`, `.AppImage` |
| **macOS** (Apple Silicon) | `dcpwizard-macos-aarch64.tar.gz` | `.dmg` |
| **Windows** (x86_64) | `dcpwizard-windows-x86_64.zip` | `.msi` |

The CLI binary is fully self-contained (OpenSSL and OpenJPEG are statically linked). Extract and run, no dependencies required.

### Install from source

#### Linux (Ubuntu/Debian)

```bash
sudo apt-get install -y pkg-config libxml2-dev libssl-dev libxerces-c-dev
# For GUI: also install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev

cd rust
cargo build --release
# Binary at rust/target/release/dcpwizard
```

#### macOS

```bash
brew install pkg-config libxml2 openssl@3 xerces-c

export OPENSSL_DIR=$(brew --prefix openssl@3)
export PKG_CONFIG_PATH="$(brew --prefix openssl@3)/lib/pkgconfig:$(brew --prefix libxml2)/lib/pkgconfig:$(brew --prefix xerces-c)/lib/pkgconfig"

cd rust
cargo build --release
```

#### Windows

```powershell
# Using vcpkg (recommended)
vcpkg install libxml2 openssl xerces-c --triplet x64-windows

$env:VCPKG_ROOT = "$env:VCPKG_INSTALLATION_ROOT"

cd rust
cargo build --release
```

### Optional runtime dependencies

| Dependency | Purpose | Install |
|-----------|---------|---------|
| `ffmpeg` | Video transcoding and import | `apt install ffmpeg` / `brew install ffmpeg` / [ffmpeg.org](https://ffmpeg.org/download.html) |
| `mpv` | GUI preview player | `apt install mpv` / `brew install mpv` / [mpv.io](https://mpv.io/installation/) |

### Docker

```bash
docker build -t dcpwizard .
docker run -v /path/to/media:/data dcpwizard create \
    --title "My Film" --video /data/j2k --audio /data/audio.wav --output /data/dcp
```

### Desktop GUI

The GUI uses [Tauri 2](https://tauri.app/) (Rust backend + web frontend) with a single-window layout inspired by professional NLEs.

**GUI features:**
- Drag & drop file import (video, audio, subtitle)
- Keyboard shortcuts (Ctrl+N/O/B/P/I, Ctrl+1–7 for views)
- Recent projects quick-access list
- Right-click context menus on assets (Preview, Remove, Show in Files)
- Asset filter / search
- Auto-detect framerate and resolution from imported video (via ffprobe)
- Progress in title bar (visible in taskbar during builds)
- Desktop notifications on build complete/fail
- Conditional button enabling (Build disabled until ready)
- Built-in mpv preview player with timeline scrubber (click-to-seek, drag-to-scrub, timecode display)
- Multi-CPL composition tabs, switch, add, remove compositions
- SRT → SMPTE subtitle conversion panel
- Subtitle burn-in panel
- Target resolution conversion panel (2K/4K scope/flat/full)
- GPU encoding toggle (grok acceleration)

```bash
cd gui
pnpm install
pnpm tauri dev
pnpm tauri build
```

## CLI Usage

```bash
# Create a DCP
dcpwizard create --title "My Feature Film" --video ./j2k --audio ./audio.wav --output ./dcp

# Create from video file (full pipeline: decode → J2K encode → MXF wrap → DCP)
dcpwizard create --title "My Film" --video movie.mov --output ./dcp --encoder openjpeg

# Create with encryption. Content keys are generated with a CSPRNG and the
# essence is AES-128 encrypted at wrap time. --key-out is required: it is the
# only place the keys are written (never next to the DCP). That file holds the
# plaintext keys, keep it secret and outside the DCP. Feed it to `kdm --keys`.
dcpwizard create --title "My Film" --video ./j2k --audio ./audio.wav --output ./dcp \
    --encrypt --key-out ./secret/my_film.keys.json

# Create Interop DCP
dcpwizard create --title "My Film" --video ./j2k --output ./dcp --standard interop

# Create with content type and resolution
dcpwizard create --title "My Trailer" --video trailer.mov --output ./dcp \
    --content-type TLR --fourk --video-bit-rate 500

# Create with frame rate override
dcpwizard create --title "My Film" --video ./j2k --output ./dcp --frame-rate 25

# Full pipeline: video → J2K → DCP in one pass (no intermediate files)
dcpwizard pipeline -i movie.mov -t "My Film" -o ./dcp --audio mix.wav

# Encode images to JPEG 2000
dcpwizard encode --input ./dpx --output ./j2k --bandwidth 250

# Transcode video to image sequence
dcpwizard transcode --input movie.mov --output ./sequence

# Create DCDM (Digital Cinema Distribution Master)
dcpwizard dcdm -i ./frames -o ./dcdm --colour-space rec709

# Colour space conversion
dcpwizard colour -i ./rec709_frames -o ./p3_frames --source rec709 --target p3

# Verify an existing DCP
dcpwizard verify ./my_dcp

# Verify with options (skip slow hash check, output report)
dcpwizard verify ./my_dcp --no-hash-check --output report.html

# Strict SMPTE Bv2.1 verification
dcpwizard verify ./my_dcp --strict --quiet

# Inspect DCP metadata
dcpwizard info ./my_dcp

# Generate KDM
dcpwizard kdm --cpl-id <uuid> --content-title "My Film" --cert recipient.pem \
    --signer-cert signer.pem --signer-key signer.key \
    --keys ./secret/my_film.keys.json --output kdm.xml

# KDM with validity period (--valid-to accepts a relative duration)
dcpwizard kdm --cpl-id <uuid> --content-title "My Film" --cert recipient.pem \
    --signer-cert signer.pem --signer-key signer.key --keys ./secret/my_film.keys.json \
    --output kdm.xml --valid-from now --valid-to "2 weeks"

# KDM with specific dates
dcpwizard kdm --cpl-id <uuid> --content-title "My Film" --cert recipient.pem \
    --signer-cert signer.pem --signer-key signer.key --keys ./secret/my_film.keys.json \
    --output kdm.xml --valid-from 2024-06-01T00:00:00+00:00 \
    --valid-to 2024-06-30T23:59:59+00:00

# Copy to cinema drive
dcpwizard copy --src ./my_dcp --dst /mnt/cru_drive

# Measure audio loudness
dcpwizard loudness audio.wav

# Generate QC report
dcpwizard report --dcp ./my_dcp --output report.html

# Start REST API server
dcpwizard serve --bind 127.0.0.1:8080

# Watch folder for auto-DCP creation
dcpwizard watch ./incoming

# Job queue daemon
dcpwizard daemon

# Manage job queue
dcpwizard batch list
dcpwizard batch add -T create-dcp -p '{"title":"My Film","video":"./j2k","output":"./dcp"}'
dcpwizard batch cancel <job-id>

# Shell completion
dcpwizard completion bash >> ~/.bashrc
dcpwizard completion zsh >> ~/.zshrc
dcpwizard completion fish > ~/.config/fish/completions/dcpwizard.fish

# Convert SRT subtitles to SMPTE DCP XML
dcpwizard subtitle-convert --input subs.srt --output subs.xml --language en

# Burn subtitles into video
dcpwizard burn-in --input movie.mov --subtitles subs.srt --output movie_burned.mov

# Scale/crop video to DCI target resolution
dcpwizard convert --input movie.mov --output movie_2k_scope.mov --target 2k-scope
# Targets: 2k-scope (2048×858), 2k-flat (1998×1080), 2k-full (2048×1080),
#          4k-scope (4096×1716), 4k-flat (3996×2160), 4k-full (4096×2160)

# Import EDL/AAF/XML timeline for conforming
dcpwizard conform -i timeline.edl --json

# Camera raw ingest
dcpwizard ingest -s /mnt/camera_card -o ./dpx_frames -f dpx --colour-space ACES

# Extract a single frame from MXF/video
dcpwizard frame-extract -i video.mxf -f 100 -o frame100.png

# Inject Dolby Vision RPU into HEVC
dcpwizard dv-inject -i input.hevc -r metadata.bin -o output.hevc

# Inject HDR10 static metadata
dcpwizard hdr10-inject -i input.mov -o output.mov --max-cll 1000 --max-fall 400

# Burn a visible watermark into a video/image file
dcpwizard watermark -i movie.mov -o movie_wm.mov -p "DIST-001-SERIAL"

# Batch KDM: one KDM per recipient certificate in a single pass.
# List certs with repeated --cert, or point --cert-dir at a directory
# of cinema certs (every *.pem/*.crt/*.cer gets a KDM).
dcpwizard kdm-batch --cpl-id <uuid> --content-title "My Film" \
    --cert-dir ./cinema-certs \
    --signer-cert signer.pem --signer-key signer.key \
    --signer-chain intermediate.pem --signer-chain root.pem \
    --keys ./secret/my_film.keys.json --output-dir ./kdms

# Package a trailer (ratings card + countdown leader + content)
dcpwizard trailer -c trailer.mov -o ./trailer_pkg --title "My Film" \
    --rating "PG-13" --rating-system mpaa --band green --countdown 8

# Generate DCP markers for a composition
dcpwizard markers --frames 172800        # FFOC/LFOC list
dcpwizard markers --frames 172800 --xml  # XML MarkerList

# Check accessibility compliance
dcpwizard accessibility ./my_dcp --standard cvaa   # cvaa|eaa|aoda|ofcom

# Send a webhook notification
dcpwizard webhook --url https://example.com/hook --event job.completed \
    --job-id 42 --payload '{"status":"ok"}'

# Content version / delivery tracking (SQLite)
dcpwizard version record --db deliveries.db --package-uuid <uuid> \
    --title "My Film" --version OV --destination "AMC" --method hard_drive --verified
dcpwizard version list --db deliveries.db
dcpwizard version export --db deliveries.db --output deliveries.csv

# OV/VF version dashboard
dcpwizard dashboard register --uuid <uuid> --title "My Film" \
    --version-type OV --territory US --status released
dcpwizard dashboard list
dcpwizard dashboard matrix --output distribution.csv
dcpwizard dashboard serve --port 9090
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
| SMPTE & Interop | ✅ | ✅ |
| Version Files (VF) | ❌ | ✅ |
| Multi-CPL timeline | ❌ | ✅ |
| CLI scriptable | ✅ | ✅ |
| Up to 4K | ✅ | ✅ |
| Stereoscopic 3D | ❌ | ✅ |
| Frame rates 24–60 fps | ✅ | ✅ |
| High Bitrate (500 Mbps) | ✅ | ✅ |
| DPX/TIFF/PNG/QuickTime input | ✅ | ✅ |
| Scale/Crop/Letterbox | ✅ | ✅ |
| J2K Transcoder | ❌ | ✅ |
| Audio (PCM 5.1) | ✅ | ✅ |
| Immersive audio (Atmos, DTS:X) | ❌ | ✅ |
| SRT→SMPTE subtitles | ✅ | ✅ |
| Subtitle packaging into DCP | ❌ | ✅ |
| Subtitle burn-in | ✅ | ✅ |
| Integrated QC | ✅ (dcpdoctor) | ✅ (Fraunhofer) |
| KDM (SMPTE) | ✅ | ✅ |
| AES-128 essence encryption | ✅ | ✅ |
| DKDM re-wrap | ✅ | ✅ |
| Desktop GUI | ✅ (Tauri) | ✅ (native) |
| REST API / Docker | ✅ | ❌ |
| Watch folder automation | ✅ | ❌ |
| Visible watermarking | ✅ | ❌ |
| DCDM intermediate format | ✅ | ❌ |
| Trailer packaging (ratings/leaders) | ✅ | ❌ |
| Content version tracking | ✅ | ❌ |
| Accessibility compliance (CVAA/EAA) | ✅ | ❌ |
| Dolby Vision / HDR10 / HLG | ✅ | ❌ |
| Camera ingest (ARRI/RED/BRAW) | ✅ | ❌ |
| Timeline conform (EDL/AAF/OTIO) | ✅ | ❌ |
| Version dashboard & distribution matrix | ✅ | ❌ |
| Open source | ✅ (GPL-3.0) | ❌ |
| **Price** | **Free** | **€2,998 or €138/mo** |

## Architecture

```
dcpwizard/
├── rust/                # Rust workspace
│   ├── crates/
│   │   ├── dcpwizard-core/  # Core library, 59 modules, DCP creation, encoding, encryption, KDM, QC
│   │   └── dcpwizard-cli/   # CLI binary (dcpwizard)
│   └── Cargo.toml
├── gui/                 # Tauri 2 desktop application
│   ├── src/             # Frontend (Vite + vanilla JS)
│   └── src-tauri/       # Rust backend (plugin shell)
└── docs/                # GitHub Pages site
```

DCP Wizard shares common functionality with [IMF Wizard](https://github.com/PostPerfection/imfwizard)
via the [postkit](https://github.com/PostPerfection/postkit) library (encoding, transcoding, hashing,
job queue, preferences, REST API, watch folders, and more).

## License

GPL-3.0, see [LICENSE](LICENSE).
