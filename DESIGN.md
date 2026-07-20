# Design

DCP creation tool. Rust core with CLI and Tauri GUI.

## Layout

- `rust/crates/dcpwizard-core`: DCP assembly (CPL/PKL/ASSETMAP writers, MXF wrapping via postkit/asdcplib), encryption, KDM, export, job queue.
- `rust/crates/dcpwizard-cli`: clap CLI (create, encode, pipeline, kdm, certificate, verify, batch/daemon, serve, dashboard, ...).
- `gui/`: Tauri app; builds run through postkit::pipeline with the CLI as sidecar.
- Shares code via postkit (path dep at `extern/postkit`), asdcplib-rs (git), and dcpdoctor-core (git) for verify/info/report.
- CPL/PKL/ASSETMAP/VOLINDEX XML, `escape_xml`, and SRT parsing come from `postkit::packaging` / `postkit::subtitle_retime` (the app maps its configs onto those writers; no hand-rolled XML remains in cpl/pkl/assetmap).
- `~15` core modules are thin `pub use postkit::...` re-export shims.

## What is implemented and wired

- OV creation from video files or J2K dirs + WAV: demux, J2K encode (grok subprocess or in-process; OpenJPEG on the create video path), real asdcplib MXF wrapping, CPL/PKL/ASSETMAP/VOLINDEX generation, SHA-1 hashes.
- Supplemental Version File (VF) creation (`create-vf`): references the OV's real asset ids for unchanged reels, wraps replacement essence (or copies an already-wrapped MXF) and registers it under its real embedded asset id in CPL/PKL/ASSETMAP, carries an `<OriginalPackagingList>` marker so dcpdoctor detects a supplemental. Validating the VF with `--ov` resolves cross-refs; alone it yields the supplemental-OV-not-provided warning, not a hard cross-ref error.
- Encryption: content keys generated with the OS CSPRNG, essence AES-128 encrypted at wrap time (`postkit::mxf_wrap`, HMAC per SMPTE), PKL/ASSETMAP hashes taken from the final encrypted files, KeyIds written into the CPL. `create --encrypt` requires `--key-out <path>` (clap-enforced); the plaintext keys are written only there, never next to the DCP. The GUI mirrors this: encrypting requires a Key Output File.
- SMPTE KDM generation, batch, DKDM rewrap (real signed ETM). `kdm --keys <key-out file>` binds the KDM to the DCP's actual image (MDIK) and audio (MDAK) content keys. `kdm-batch` takes repeated `--cert` or a `--cert-dir` of cinema certs and writes one signed KDM per recipient. Certificate chain generation and inspection.
- verify/info/report via dcpdoctor-core; SRT to subtitle-XML conversion; subtitle/watermark burn-in.
- Transcode/import via ffmpeg, frame extraction, dv-inject via dovi_tool, loudness via loudnorm.
- SQLite version tracker and dashboard (including serve), daemon/batch IPC, shell completion.
- GUI: shortcuts, recents, filter, preview, timeline view, sidecar setup; KDM panel collects signer cert/key + keys file; encode honours the selected frame rate; OS file drop uses the Tauri webview drag-drop event.

## Not implemented (de-advertised)

These have no working path and are no longer advertised in README/docs: reel splitting, multi-CPL timelines, stereoscopic 3D, HFR validation table, subtitle packaging into a DCP timed-text track, Dolby Atmos (IAB) / DTS:X, channel mapping, J2K transcoder. The dead modules remain in the tree but are unreferenced. Interop and Atmos KDM variants are not implemented (SMPTE only). `cli_flags_test.sh` now actually invokes each GUI command line and fails on clap parse errors.
