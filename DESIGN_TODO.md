# Planned

Remaining gaps after the 2026-07 audit fixes. Paths: CORE = rust/crates/dcpwizard-core/src, CLI = rust/crates/dcpwizard-cli/src/main.rs, PK = extern/postkit (postkit submodule; bump the pin when postkit changes).

## Fixed in the 2026-07 pass (for reference)

- Encryption is real: CSPRNG content keys, AES-128 at wrap time, hashes from the
  encrypted files, KeyIds in the CPL, keys persisted to KEYS.json, KDM bound to
  the real keys via `kdm --keys`. (encrypt.rs, dcp.rs, mxf_wrap, PK packaging/certificate)
- KDM docs/examples include the required signer flags; `--valid-duration` and
  forensic-marking flags removed from docs; GUI KDM panel collects signer cert/key + keys.
- GUI: OS drag-drop via Tauri webview event; encode honours selected fps;
  decorative GPU/threads/colour/channels/3D controls removed.
- cli_flags_test.sh invokes each GUI command line and fails on clap parse errors.
- Dedup onto postkit: escape_xml, CPL/PKL/ASSETMAP writers, SRT parsing.
- dcpdoctor-core git rev bumped to ce050e5.
- KDM MessageType fixed: always the standard `#kdm-key-type` URI. The old
  `--formulation` dci-any/dci-specific emitted invented `#kdm-key-type-dci-*`
  URIs that compliant gear does not recognise as a KDM (PK/certificate.rs).
- Batch KDM gained `--cert-dir` (glob a directory of cinema certs); real
  end-to-end test generates one signed KDM per recipient, KeyId-bound and
  xmlsec1-verified (CORE/kdm.rs).
- VF (supplemental) creation is real and wired (CLI `create-vf`, CORE/vf.rs).
  Unchanged reels reference the OV's asset ids; replacement essence is wrapped
  (or a pre-wrapped MXF copied) and registered under its real embedded id in
  CPL/PKL/ASSETMAP; an `<OriginalPackagingList>` marker makes dcpdoctor detect
  the VF. Verified end-to-end: `dcpdoctor validate --ov` resolves cross-refs,
  alone gives the supplemental warning (not cross_ref_broken).

## Planned features

- multi-CPL timelines (multi_cpl.rs create_multi_cpl) still unbuilt.
- Encrypted DCP derivatives: accept an external KDM plus recipient certificate
  and private key, decrypt the source, then permit supported transcode and
  burn-in workflows. Keep key material out of logs and temporary output.
- Subtitle authoring controls: supplied SMPTE subtitle XML keeps its placement
  and styling, but SRT always becomes centred bottom text. Add SRT placement
  controls and closed-caption packaging. 3D subtitle depth remains separate.
- Input video range: expose an explicit full-range or legal-range override, or
  verify the decoded range before conversion. The current ffmpeg raw-RGB path
  leaves users no way to correct wrong or absent source metadata.
- DTS:X: BLOCKED. postkit declined a generic DCData wrap because the DTS:X
  DataEssenceCoding UL could not be confirmed. dcpwizard used to map DTS:X onto
  the Atmos (IAB) essence UL, which is wrong. The `MxfType::DtsX` variant is
  removed so DTS:X now fails loud as unsupported. Unblock: confirm the DTS:X UL,
  add a postkit essence type + CPL AuxData DataType, then wire a `--dtsx` flag
  mirroring `--atmos`.

Built in the 3D/Atmos/MCA/Interop-KDM pass (obsolete asdcp-wrap scaffolding
modules stereo3d.rs/atmos.rs/audio.rs/dtsx.rs deleted; the real wiring lives in
mxf_wrap.rs, dcp.rs, cpl.rs):
- Stereoscopic 3D: `create --right-eye`; ST 429-10 MainStereoscopicPicture in
  the CPL (429-10/2008 namespace, prefixed form so children stay in the CPL's
  429-7 namespace and validate against the clairmeta 429-10 XSD; EditRate =
  composition rate, FrameRate doubled). postkit `wrap_stereoscopic`. Verified
  end-to-end: dcpdoctor validate passes with 0 errors and now runs real ST
  429-10 checks (prefixed form, FrameRate = 2x EditRate, Jpeg2000Stereo
  essence), proven in the dci-ctp corpus. The stereo element validates against
  the 429-10 XSD via xmllint.
- MCA labeling: every PCM wrap carries ST 429-12 labels by channel count
  (2.0/5.1/7.1); `--hi-channel`/`--vi-channel` add HI/VI-N standalone labels.
  Verified via asdcplib mca_labels (6ch -> 1 soundfield group + 6 channel labels
  + ChannelAssignment UL). dcpdoctor now reads the ST 429-12 subdescriptors from
  the MXF, so its `sound_invalid_channel_count` INFO clears on labeled packages
  (verified in the dci-ctp corpus: labeled 5.1 clean, unlabeled mono flagged).
- Dolby Atmos: `create --atmos <file-or-dir>`; ST 429-18 AuxData in the CPL
  (Dolby 2012/AD namespace, IAB UL 060e2b34.04010105.0e090604.00000000),
  registered in PKL/ASSETMAP. postkit `EssenceType::Atmos`. One input file = one
  aux frame; create now refuses when the frame count differs from the picture
  duration (a short aux track is a broken DCP, caught by ClairMeta
  check_cpl_reel_duration_picture_aux). Verified with a synthetic per-frame
  DCData payload: dcpdoctor validate (and validate --atmos) pass with 0 errors.
  Real-essence conformance needs real Atmos material.
- Interop KDM: `kdm --format interop` / `kdm-batch --format interop`, threaded to
  postkit KdmConfig.format. Verified: digicine namespace present, xmlsec1
  verifies. Legacy output, needs real-gear validation before production use.

Done in the earlier pass:
- J2K transcoder is real and wired (CLI `transcode-dcp`, j2k_transcode.rs): reads
  an existing DCP, extracts each reel's J2K frames from the picture MXF (asdcplib
  jp2k reader), decodes with grk_decompress (raw XYZ components), re-encodes with
  grok at the bandwidth-derived ratio (apply_xyz_transform off, so the round trip
  preserves the stored components), rewraps and emits a fresh CPL/PKL/ASSETMAP.
  Audio/subtitle tracks are copied verbatim (asset ids preserved). Encrypted input
  is rejected (CPL KeyId + MXF writer_info). get_timeline now also parses the
  MainSubtitle track so it survives the repackage. Verified end-to-end: a 400 Mbps
  2K DCP transcoded to 50 Mbps keeps 48 frames / 24 1 edit rate, the picture MXF
  drops 43.5 MB -> 16.7 MB, and dcpdoctor passes with 0 errors (same one ISDCF
  naming warning the source already has). Optional `--width`/`--height` rescale via
  ffmpeg.
- OpenJPEG removed: grok is the only encoder. The `create --encoder` flag and the
  openjpeg branch are gone; postkit is built with `grok-ffi` (was `openjpeg`). The
  bandwidth->ratio math (width*height*36) is now the shared
  `encode::bandwidth_to_ratio`, used by `create` and `transcode-dcp`.
- Reel splitting is real: `create --reel-length <minutes>` (reel.rs) splits the
  picture (per-reel slice of the sorted frame list), audio (sample-accurate WAV
  slice at frame edges, padded to the reel frame count), and subtitles (DCST
  re-split by reel and rebased to reel-local time) into one MXF set per reel and
  emits a multi-reel CPL over a shared PKL/ASSETMAP. Every reel >= 1s; a sub-second
  remainder merges into the previous reel. Encryption stays coherent (per-reel
  keys, all reels encrypted). The single-reel path (flag absent) is untouched.
  postkit has no frame-subrange wrap option, so picture is wrapped from an explicit
  per-reel file list (mxf_wrap::wrap_mxf_files) and audio/subtitle from temp files.
  Verified end-to-end: 2-reel DCP with audio and per-reel subtitles passes dcpdoctor
  with 0 errors and no reel-coherence note; encrypted variant too.
- HFR validation is real and wired (hfr.rs): legal SMPTE rates 24/25/30/48/50/60/
  96/100/120 (Interop 24/25/30/48); 4K capped at 30 fps (48+ needs 2K, per the DCI
  HFR addendum / SMPTE ST 428-11:2013). `create`/pipeline reject illegal fps/resolution
  combos loudly before encoding; the edit rate threads through CPL and MXF (verified
  a 48fps 2K DCP validates). GUI frame-rate menus offer exactly the legal set.
- 23.976 source delivery: `create` maps 24000/1001 to the 24 fps DCP edit rate
  and applies a 1000/1001 audio pull-up to supplied or demuxed audio before PCM
  wrapping.
- Alpha input: `create` rejects alpha-bearing pixel formats before RGB encoding
  and tells the user to composite an opaque background. DCP picture essence has
  no alpha channel.
- Subtitle packaging is real: `create --subtitle` converts SRT to ST 428-7 DCST XML
  and wraps supplied SMPTE XML unchanged, preserving authored placement and styling.
  3D subtitle depth is unsupported.
- PCM wrap rejects non-DCP sample rates (48/96 kHz only). Six-channel input is
  padded to 16 PCM channels; `--audio-input-order lrc-ls-rs-lfe` explicitly remaps
  that source order to L/R/C/LFE/Ls/Rs before wrapping.
- Deleted (zero callers): dcp_diff, plugin, preferences, geometry, prores shim.

## KDM (minor)

- Interop (pre-SMPTE) KDM: now built via `kdm --format interop` (postkit
  KdmConfig.format = Interop; digicine 134-byte block). See note below.
- `--formulation`: removed from CLI/config/docs (was inert; MessageType is fixed
  and postkit emits no formulation-gated extensions).
- Duration-based end times now keep the start's UTC offset (resolved on our side
  in kdm.rs before postkit's duration path, which mislabels them +00:00).
- AnnotationText: postkit derives it from content_title ("<title> KDM for
  <recipient>"), not hardcoded. A separate `--annotation` override needs a
  postkit KdmConfig.annotation field (postkit is frozen here), so not added.
  No Trusted Device List / DeviceList written.

### Interop KDM (built, legacy)

Implemented: postkit gained `KdmConfig.format` (Smpte/Interop); dcpwizard threads
it via `kdm --format` and `kdm-batch --format` (default smpte). Interop uses the
digicine `PROTO-ASDCP-KDM-20040311#` namespace and the 134-byte key block (drops
the 4-byte KeyType field vs SMPTE 138). Verified: the digicine namespace is
present and xmlsec1 verifies the signature; postkit's own test asserts the
134-byte block. Caveat unchanged: no reference Interop KDM exists in the suite to
diff against and no compliant reference library *generates* Interop (libdcp only
reads it). Treat Interop output as legacy and validate against real legacy gear
before production use.

## Encode / colour / audio

- Encode bandwidth: the GUI now maps the target Mbps to the J2K compression ratio
  via run_encode_with_ratio (raw = w*h*36); CLI create/pipeline already did. The
  CLI create path and transcode-dcp share `encode::bandwidth_to_ratio` for this.
- Encoder: grok only (openjpeg removed). Build needs libgrokj2k on the pkg-config
  path (grok-ffi links it); runtime needs it on the loader path (LD_LIBRARY_PATH).
- Colour: `colour --target xyz` now routes through the real DCDM transform (fails
  loud on an unsupported source). P3-D65 still not added (would need a postkit
  change, which is frozen here).
- Markers: only FFOC/LFOC emitted; the other eight are defined but never placed.

## HDR, ingest, conform (mostly postkit-side)

- HDR source creation requires `--hdr-to-dci-lut`; the LUT runs through postkit's
  colour path before J2K encoding. `--allow-generic-hdr-tonemap` is opt-in and warns
  because FFmpeg tone mapping is not a delivery transform.
- Camera raw: ARRIRAW/R3D/BRAW detected but ffmpeg can't decode them; Sony not detected.
- ingest has no --lut flag (apply_lut hardcoded false).
- conform parses and prints only (no reel assembly); only CMX3600 EDL and FCP7 xmeml work.

## Export, automation, misc

- Export to ProRes/H.264/H.265/DNxHR: now exposed as `dcpwizard export` (was only
  reachable via the batch daemon job type).
- Watch folder: `watch --webhook-url` POSTs a JSON notification when a DCP is
  detected (was configured but never fired).
- Job queue progress jumps 0→100; cancel only affects pending jobs; `serve` uses a
  separate in-process queue.
- copy-to-drive verify now flushes to the device (sync_all) and drops the page
  cache (posix_fadvise DONTNEED) before reading back, so verification is real.
  `verify --output .pdf` removed (it silently produced plain text).
- Trailer output is mp4 (no DCP/CPL); accessibility check is substring matching.

## GUI (remaining)

- "Show in Files" fixed: uses tauri-plugin-opener `revealItemInDir` (the shell
  `open` call only accepted URLs).
- Transcode format/bit-depth now wired (CLI transcode gained --format/--bit-depth);
  the copy "verify after copy" checkbox was removed (copy always hash-verifies).
- scope/flat/full now map to the correct container dimensions; content kind
  "episode" maps to a real ContentKind (ContentType::Episode added).
- KDM panel gained a Format select (smpte/interop), wired straight into the
  `kdm --format` CLI arg (the panel builds args directly, so this is cheap).
- Create-panel 3D/Atmos pickers NOT wired: the create panel goes through the
  tauri `submit_job` -> JobConfig -> run_job job queue, not the CLI, so adding
  `--right-eye`/`--atmos` needs plumbing through the command signature, JobConfig
  and run_job (structural). Deferred. The CLI path (`create --right-eye/--atmos`)
  is the way to build 3D/Atmos DCPs for now.
- Added the asdcplib `[patch]` to gui/src-tauri/Cargo.toml (mirrors rust/Cargo.toml).
  The postkit MCA bump calls asdcplib::open_write_mca, absent from the git-pinned
  asdcplib; without the patch the GUI's postkit no longer compiles.

## Dedup (remaining)

- multi_cpl.rs kept: list_cpls/get_timeline are load-bearing for VF (vf.rs) and the
  GUI timeline panel (gui timeline.rs). Its create_multi_cpl (multi-CPL timeline
  feature) is still unbuilt.
- Deleted (zero callers): dcp_diff, plugin, preferences, geometry, prores shim.

## Keep in sync with imfwizard (deliberately duplicated, no clean shared home)

The final dedup pass (2026-07-20) confirmed the shared *logic* already lives in
postkit (mpv::MpvPlayer, packaging writers, escape_xml, parse_srt,
pipeline::run_encode). What remains duplicated is app/framework glue with no
clean cross-repo home, so it is left as copies. If you edit one side, mirror the
other:

- gui/src-tauri/src/preview_server.rs — near-identical (only the MpvPlayer app
  name differs). NOT moved to postkit: it is all `#[tauri::command]` wrappers and
  postkit has no tauri dep (also used by the CLI and wasm), so hosting it there
  would force tauri onto the core lib. The reusable part (MpvPlayer) is already in
  postkit::mpv. dcpwizard also keeps a windows preview_server_stub the imf side lacks.
- gui/src/preview.js, gui/vite.config.js — frontend files (differ only by var
  order / dev port); the GUIs don't consume JS from the postkit crate, so no home.
- gui/src-tauri/src/lib.rs, gui/src-tauri/src/pipeline.rs — app-specific tauri
  setup and build orchestration; they already delegate the encode to
  postkit::pipeline. Diverged enough (lib.rs module names + terminal guard,
  pipeline.rs 467 vs 382 lines) that unifying would need config flags per divergence.
  2026-07 honesty pass edited pipeline.rs (bandwidth->ratio via
  run_encode_with_ratio, scope/flat/full container dims, episode ContentKind) and
  lib.rs (register tauri_plugin_opener). imfwizard already wires bandwidth->ratio;
  mirror the opener plugin + any container/ContentKind logic that applies there.
- .github/workflows/ci.yml, release.yml, gui-release.yml — copies across dcpwizard,
  imfwizard, dcpdoctor differing by binary/artifact names + per-app build deps.
  Separate git repos, so no shared reusable-workflow without a central repo. Keep
  aligned by hand. Grok CI addressed 2026-07-21: every job that compiles the rust
  workspace (ci build + gui, release build + deb, gui-release build-gui) has a
  cached "Setup grok" step that builds grok v20.3.6 from source with cmake, installs
  to $GITHUB_WORKSPACE/grok-install, and exports PKG_CONFIG_PATH/LD_LIBRARY_PATH
  (+ DYLD + bin on PATH) via $GITHUB_ENV. actions/cache keyed on grok tag + runner
  os, so the cmake build runs once per tag bump. Linux + macOS only; windows legs
  are continue-on-error since the grok source build there is not wired up. No
  openjpeg system deps were present to drop. imfwizard mirrors the same step but
  only in ci (it runs grk_compress at runtime, does not link grok-ffi); dcpdoctor
  needs no grok (postkit dep has no grok-ffi, no grok usage).
- tests/cli_flags_test.sh — NOT the same harness as imfwizard's anymore (this one
  runs the binary and checks clap parse errors; imf parses main.js). Different CLIs,
  leave separate.
