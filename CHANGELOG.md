# Changelog

## [Unreleased]

### Added
- **CompositionMetadataAsset (Bv2.1)** — SMPTE CPLs now carry a ST 429-16 `CompositionMetadataAsset` in the first reel with `MainSoundConfiguration` (e.g. `51/L,R,C,LFE,Ls,Rs` with `-` padding for the silent fill channels), `MainSoundSampleRate`, `MainPictureStoredArea`/`ActiveArea`, `FullContentTitleText` and the Bv2.1 `ExtensionMetadata` marker. The sound configuration is derived from the packaged channel count, so validators no longer flag "0 channels but sound assets have N". Interop CPLs are unchanged. Verified: the asset validates against the ST 429-16 XSD and the whole CPL against ST 429-7 via xmllint
- **`ingest-package` subcommand** — metadata-only repackaging: scans a directory, reads each MXF's embedded asset UUID (asdcplib), and regenerates ASSETMAP/PKL/VOLINDEX covering every asset file present, reusing hashes from the old PKL where available. Fixes exported OV/VF folders (e.g. a Sony server's VF export) whose ASSETMAP/PKL omit hardlinked assets referenced by the CPL. Essence passes through untouched; the old ASSETMAP/PKL are replaced. Both SMPTE and Interop naming
- **Configurable subtitle position** — `subtitle-convert --vposition <percent>` (default 8) sets the bottom line's distance from the bottom of the screen
- **Stereoscopic 3D** — `create --right-eye <input>` (main input is the left eye) encodes both eyes at the same settings, wraps them into one ST 429-10 stereoscopic picture MXF (postkit `wrap_stereoscopic`) and emits the CPL `MainStereoscopicPicture` element (429-10/2008 namespace, EditRate at the composition rate, FrameRate doubled for interleaved L/R). Eye frame counts must match. Verified: a short 3D DCP passes `dcpdoctor validate` with 0 errors and the stereoscopic element validates against the clairmeta 429-10 XSD via xmllint
- **MCA channel labeling** — every PCM sound wrap now carries ST 429-12 / 377-4 MCA labels derived from the probed channel count (2.0 / 5.1 / 7.1). `create --hi-channel <n>` / `--vi-channel <n>` label a channel index as the standalone HI / VI-N accessibility track. Verified by reading the labels back out of the MXF with asdcplib (`mca_labels`)
- **Dolby Atmos aux track** — `create --atmos <file-or-dir>` wraps a Dolby Atmos / DCData bitstream (postkit `EssenceType::Atmos`) and registers it as a ST 429-18 `AuxData` element (Dolby `2012/AD` namespace, IAB data-essence UL) in the CPL/PKL/ASSETMAP. Verified: an Atmos DCP with a synthetic payload passes `dcpdoctor validate`; real-essence conformance needs real Atmos material
- **Interop KDM** — `kdm --format smpte|interop` (default smpte) and `kdm-batch --format`. Interop uses postkit's digicine ETM (134-byte key block). Verified: an Interop KDM carries the digicine namespace and verifies with xmlsec1. Legacy output, mark for real-gear validation before production use
- **`transcode-dcp` subcommand** — re-encode an existing DCP's picture essence to a lower bandwidth (`--video-bit-rate`, optional `--width`/`--height` rescale). Extracts each J2K frame from the picture MXF (asdcplib), decodes with grk_decompress, re-encodes with grok at the target ratio, rewraps, and emits a fresh CPL/PKL/ASSETMAP with correct hashes. Audio and subtitle tracks are copied unchanged; encrypted input is rejected
- **Reel splitting**: `create --reel-length <minutes>` splits the picture, audio (sample-accurate at the frame edge) and subtitles (DCST re-split and rebased per reel) into one MXF set per reel and emits a multi-reel CPL over a shared PKL/ASSETMAP; every reel is at least 1 second, a sub-second remainder merges into the previous reel. Absent the flag the single-reel path is unchanged
- **HFR validation**: legal DCP rates are 24/25/30/48/50/60/96/100/120 (SMPTE; Interop stays 24/25/30/48); 4K is capped at 30 fps. Illegal fps/resolution combos are rejected loudly before encoding; the GUI frame-rate menus offer exactly the legal set
- **Subtitle packaging** — `create --subtitle <srt> [--subtitle-language]` converts SRT to ST 428-7 DCST XML, wraps it as a timed-text MXF and registers the track in the reel/CPL/PKL/ASSETMAP
- **`export` subcommand** — ProRes/H.264/H.265/DNxHR/image-sequence export of a picture MXF (previously only reachable via the batch daemon)
- **`transcode --format` / `--bit-depth`** — wired the GUI transcode controls to the real backend
- **`watch --webhook-url`** — POST a JSON notification when a new DCP is detected

### Changed
- **Encode bandwidth** — the GUI now maps the target Mbps to the J2K compression ratio (was hardcoded)
- **`colour --target xyz`** — routes through the real DCDM Rec.709/P3/Rec.2020 → X'Y'Z' transform (fails loud on an unsupported source)
- **PCM wrap** — rejects non-DCP audio sample rates (48/96 kHz only) instead of mislabeling
- **KDM validity** — duration-based end times keep the start's UTC offset
- **GUI resolution** — scope/flat/full map to the correct container dimensions; content kind `episode` maps to a real ContentKind
- **Subtitle timecodes** — SMPTE DCST now emits frame-based `HH:MM:SS:FF` (schema-valid), not `HH:MM:SS.mmm`

### Removed
- **DTS:X** — was silently mapped to the Atmos (IAB) essence UL, which is wrong for DTS:X. Removed the `MxfType::DtsX` variant and dropped DTS:X from the docs; blocked on a confirmed DataEssenceCoding UL (see DESIGN_TODO)
- **Dead scaffolding modules** — atmos.rs, audio.rs, stereo3d.rs, dtsx.rs (shelled out to a nonexistent `asdcp-wrap`, zero callers; superseded by the real postkit-based 3D/Atmos/MCA wiring)
- **OpenJPEG encoder** — grok is now the only J2K encoder. Dropped the `create --encoder` flag and the openjpeg branch; the postkit dependency enables `grok-ffi` (not `openjpeg`). libgrokj2k must be discoverable by pkg-config at build time and loadable at runtime
- **KDM `--formulation`** — was inert (MessageType is fixed by ST 430-1); removed from CLI/config/docs
- **`verify --output .pdf`** — silently produced plain text; option dropped
- **Dead modules** — dcp_diff, plugin, preferences, geometry, and the prores re-export shim (zero callers)

### Fixed
- **Subtitle vertical position** — SMPTE and Interop subtitle generators anchored the block at the top (Vposition 85 with Valign="bottom") so subtitles rendered near the top of the screen. The bottom line now sits at 8% from the bottom with lines stacked upward (a two-line cue renders at 15% and 8%)
- **GUI "Show in Files"** — uses the tauri opener plugin (`revealItemInDir`); the shell `open` call only accepted URLs
- **Copy-to-drive verify** — flushes to the device and drops the page cache before reading back, so verification is real

## [1.1.0] — 2026-05-28

### Added
- **Timeline view** — Visual DCP timeline with multi-reel navigation, duration bars, and asset type indicators
- **CLI flag consistency test** — `tests/cli_flags_test.sh` verifies GUI invocations match actual CLI flags
- **CI: CLI flag check** — Automated verification in GitHub Actions pipeline

### Fixed
- **Properties panel → build pipeline** — All properties (standard, resolution, framerate, bandwidth, colour, content_kind, encrypt, stereo_3d, channels) now correctly passed to the DCP creation pipeline
- **Verify flags** — `--check-mxf` → `--no-picture-check`, `--skip-hashes` → `--no-hash-check` (correct inverted logic)
- **Encode flags** — Removed non-existent `--resolution`/`--framerate`, added `-i` input flag
- **Transcode flags** — Removed non-existent `--format`/`--bit-depth`, uses `-i` flag
- **Copy flags** — Positional args replaced with `--src`/`--dst` named args
- **Report flags** — Uses `--dcp` and `-o` instead of non-existent `--format`
- **Subtitle conversion** — `Command.create` → `Command.sidecar`, `--framerate` → `--fps`, added `-i`/`-l` flags
- **Burn-in** — `Command.create` → `Command.sidecar`, correct `-i`/`-s`/`-o` flags
- **Target conversion** — `Command.create` → `Command.sidecar`, correct `-i`/`-t`/`-m` flags
- **KDM** — Added CPL ID, content title, and output path fields; uses correct `--cpl-id`/`--content-title`/`--cert`/`-o` flags
- **Encrypt standalone** — Shows informational message (standalone re-encryption not supported by CLI)

## [1.0.0] — 2025-01-20

### Added
- **CLI: Create subcommand** — Full DCP creation from JPEG 2000 + audio MXF inputs
  - `--content-type` (feature, trailer, advertisement, etc.)
  - `--frame-rate` (24, 25, 30, 48, 60)
  - `--twok` / `--fourk` resolution selection
  - `--video-bit-rate` for encoding control
  - `--threads` for parallel encoding
  - `--container-ratio` (flat, scope, full)
  - `--audio-channels` configuration
- **CLI: Verify subcommand** — Full DCP verification with options
  - `--no-hash-check` to skip hash verification
  - `--no-picture-check` to skip picture essence inspection
  - `--strict` for SMPTE-strict mode
  - `--output` for writing reports to file (text or HTML)
  - `--quiet` to suppress progress output
- **CLI: KDM subcommand** — Key Delivery Message generation
  - `--valid-from` / `--valid-to` for validity periods (`--valid-to` accepts a
    relative duration: "2 weeks", "30 days", "4 hours")
  - `--keys` to carry the DCP's content keys (from `create --encrypt`)
  - `--signer-cert` / `--signer-key` (required) for the ETM signature
  - `--formulation` (modified-transitional-1, dci-any, dci-specific)
- **Panic hook** — User-friendly crash messages with issue tracker link
- **CLI integration tests** — 10 end-to-end tests using assert_cmd
- **Release CI** — GitHub Actions workflow for building release binaries on tag push
- **GUI Release CI** — Tauri build workflow producing .deb, .AppImage, .dmg, .msi

### Changed
- Version unified to 0.5.0 across all workspace crates
- Git dependencies pinned to v0.5.0 tags (asdcplib-rs, dcpdoctor, postkit)
- ContentType, Resolution, Standard enums now in dcpwizard-core::lib

### Fixed
- OpenJPEG thread-safety issues resolved via Grok JPEG 2000 encoder
- Clippy warnings cleaned up across entire workspace
