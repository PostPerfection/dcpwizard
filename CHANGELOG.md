# Changelog

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
  - `--valid-from` / `--valid-to` / `--valid-duration` for validity periods
  - Duration parsing ("2 weeks", "30 days", "4 hours")
  - `--formulation` (modified-transitional-1, dci-any, dci-specific)
  - `--disable-forensic-marking-picture` / `--disable-forensic-marking-audio`
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
