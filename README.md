# DCP Wizard

Digital Cinema Package (DCP) creator — CLI tool and desktop GUI.

## Overview

DCP Wizard creates valid DCPs (Digital Cinema Packages) from video sources,
image sequences, and WAV audio, conforming to SMPTE ST 429 and Interop standards.

## Features

### DCP Creation
- **Original Version (OV)** DCP creation from J2K + WAV
- **Version File (VF)** creation referencing original reels
- **SMPTE & Interop** standard support
- **2K and 4K** resolution
- **Multiple frame rates** — 24, 25, 30, 48 fps
- **Automatic reel splitting** by duration or file size
- **CPL/PKL/ASSETMAP** generation
- **SHA-1 hashing** for integrity verification

### Encoding & Transcoding
- **JPEG 2000 encoding** via grok or OpenJPEG
- **Video transcoding** — ProRes, H.264, H.265 → image sequence → J2K (via ffmpeg)
- **Colour conversion** — Rec.709 / P3 → XYZ with gamma 2.6
- **Stereoscopic 3D** frame interleaving

### Encryption & KDM
- **AES-128 content encryption** (SMPTE compliant)
- **KDM generation** for individual recipients
- **Batch KDM** generation for multiple screens

### Subtitles & Captions
- **SMPTE XML subtitles** packaging
- **Interop XML subtitles** packaging
- **SRT import** and conversion
- **Subtitle burn-in** for festival DCPs

### Audio
- **Multi-channel audio** — mono, stereo, 5.1, 7.1
- **Dolby Atmos (IAB)** immersive audio support
- **Loudness measurement** — EBU R128 / ATSC A/85
- **Channel mapping** configuration

### Quality Control
- **DCP verification** via dcpdoctor integration
- **HTML QC report** generation
- **Markers** — FFOC, LFOC, title credits, intermission, end credits

### Delivery
- **Copy to drive** with hash verification (USB/CRU)
- **Watch folder** mode for automated DCP creation
- **Job queue** with progress tracking
- **REST API** for headless/batch operation
- **Docker container** for CI/CD pipelines

## Building

### Prerequisites

- CMake ≥ 3.25
- C++23 compiler (GCC 13+, Clang 17+, MSVC 2022+)
- libxml2, OpenSSL, Xerces-C

### Build

```bash
git clone --recurse-submodules https://github.com/yourorg/dcpwizard.git
cd dcpwizard
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel
```

### Test

```bash
cd build
ctest --output-on-failure
```

### GUI

The desktop GUI uses [Tauri](https://tauri.app/).

```bash
cd gui
npm install
npm run tauri dev
```

## Docker

```bash
docker build -t dcpwizard .
docker run -v /path/to/media:/data dcpwizard create \
  --title "My Film" --video /data/j2k --audio /data/audio.wav --output /data/dcp
```

REST API mode:

```bash
docker run -p 8080:8080 -v /path/to/media:/data dcpwizard serve --port 8080
```

## CLI Usage

```
dcpwizard create --title "My Film" --video ./j2k --audio ./audio.wav --output ./dcp
dcpwizard encode --input ./dpx --output ./j2k --bandwidth 250
dcpwizard transcode --input movie.mov --output ./sequence
dcpwizard verify ./my_dcp
dcpwizard info ./my_dcp
dcpwizard kdm --dcp ./my_dcp --cert recipient.pem --output kdm.xml
dcpwizard copy --src ./my_dcp --dst /mnt/cru_drive
dcpwizard loudness audio.wav
dcpwizard report --dcp ./my_dcp --output report.html
dcpwizard serve --port 8080
dcpwizard watch ./incoming
dcpwizard completion bash
```

## License

GPL-3.0 — see [LICENSE](LICENSE).
