# Changelog

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
