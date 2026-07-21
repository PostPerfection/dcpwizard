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

- OV creation from video files or J2K dirs + WAV: demux, J2K encode (grok in-process FFI), real asdcplib MXF wrapping, CPL/PKL/ASSETMAP/VOLINDEX generation, SHA-1 hashes. `create --container` selects 2K/4K scope, flat, or full CPL dimensions.
- Reel splitting (`create --reel-length <minutes>`, reel.rs): splits the encoded frame sequence, audio and subtitles into per-reel ranges and wraps one picture/sound/subtitle MXF set per reel, then emits a multi-reel CPL over a shared PKL/ASSETMAP. Boundaries are in whole frames at the edit rate; audio is sliced sample-accurately (frames * sample_rate/fps) and padded to the reel frame count; DCST cues are re-split by reel and rebased to reel-local time. Every reel is at least 1s (a sub-second remainder merges back). postkit's wrapper consumes every input file, so picture is wrapped from an explicit per-reel file list and audio/subtitle from temp files. The single-reel path (flag absent) is unchanged.
- HFR frame rates (hfr.rs): legal SMPTE rates 24/25/30/48/50/60/96/100/120 (Interop 24/25/30/48); 4K is capped at 30 fps per the DCI HFR addendum. `create_dcp` and the CLI/GUI pipelines reject an illegal fps/resolution combo loudly before encoding; the edit rate threads through both CPL and MXF.
- Supplemental Version File (VF) creation (`create-vf`): references the OV's real asset ids for unchanged reels, wraps replacement essence (or copies an already-wrapped MXF) and registers it under its real embedded asset id in CPL/PKL/ASSETMAP, carries an `<OriginalPackagingList>` marker so dcpdoctor detects a supplemental. Validating the VF with `--ov` resolves cross-refs; alone it yields the supplemental-OV-not-provided warning, not a hard cross-ref error.
- Encryption: content keys generated with the OS CSPRNG, essence AES-128 encrypted at wrap time (`postkit::mxf_wrap`, HMAC per SMPTE), PKL/ASSETMAP hashes taken from the final encrypted files, KeyIds written into the CPL. `create --encrypt` requires `--key-out <path>` (clap-enforced); the plaintext keys are written only there, never next to the DCP. The GUI mirrors this: encrypting requires a Key Output File.
- SMPTE (and Interop) KDM generation, batch, DKDM rewrap (real signed ETM). `kdm --keys <key-out file>` binds the KDM to the DCP's actual image (MDIK) and audio (MDAK) content keys. `kdm --format smpte|interop` (also on `kdm-batch`) selects the ETM: Interop uses postkit's digicine 134-byte block, legacy output that needs real-gear validation. `kdm-batch` takes repeated `--cert` or a `--cert-dir` of cinema certs and writes one signed KDM per recipient. Certificate chain generation and inspection.
- Stereoscopic 3D (`create --right-eye`): both eyes encoded at the same settings, wrapped into one ST 429-10 picture MXF (postkit `wrap_stereoscopic`), CPL carries the `MainStereoscopicPicture` element (429-10/2008 namespace, EditRate = composition rate, FrameRate doubled). Validated against the clairmeta 429-10 XSD.
- MCA channel labeling: 5.1 input is expanded to 16 PCM channels, with L/R/C/LFE/Ls/Rs in the first six positions and ten silent channels. `create --audio-input-order lrc-ls-rs-lfe` explicitly remaps that common source order; it never guesses. ST 429-12 labels cover the canonical 5.1 channels, and `create --hi-channel/--vi-channel` add standalone HI/VI-N labels.
- Dolby Atmos (`create --atmos`): wraps a Dolby Atmos / DCData bitstream (`EssenceType::Atmos`) and registers a ST 429-18 `AuxData` element (Dolby `2012/AD` namespace, IAB UL) in CPL/PKL/ASSETMAP. Structure validates; real conformance needs real Atmos essence.
- verify/info/report via dcpdoctor-core; SRT to subtitle-XML conversion; supplied SMPTE subtitle XML is wrapped unchanged, preserving authored placement and styling. Subtitle/watermark burn-in is separate.
- HDR source video requires `create --hdr-to-dci-lut <lut>`. `--allow-generic-hdr-tonemap` enables FFmpeg tone mapping with a warning, for explicitly accepted non-delivery use.
- Transcode/import via ffmpeg, frame extraction, dv-inject via dovi_tool, loudness via loudnorm.
- SQLite version tracker and dashboard (including serve), daemon/batch IPC, shell completion.
- GUI: shortcuts, recents, filter, preview, timeline view, sidecar setup; KDM panel collects signer cert/key + keys file; encode honours the selected frame rate; OS file drop uses the Tauri webview drag-drop event.

## Not implemented (de-advertised)

These have no working path and are not advertised: multi-CPL timelines, and DTS:X (blocked on a confirmed DataEssenceCoding UL, see DESIGN_TODO; the `MxfType::DtsX` variant was removed so it fails loud rather than emitting an Atmos UL). Atmos KDM (essence-encrypted aux track in a KDM) is not implemented. `cli_flags_test.sh` invokes each GUI command line and fails on clap parse errors.

Subtitle packaging into a DCP timed-text track is real: `create --subtitle <srt>` converts SRT to ST 428-7 DCST XML, and supplied SMPTE XML is wrapped unchanged. Both paths register the timed-text MXF in the reel/CPL/PKL/ASSETMAP. 3D subtitle depth is unsupported.

Deleted (dead, zero callers): dcp_diff, plugin, preferences, geometry, and the prores re-export shim.
