# Planned

Remaining gaps after the 2026-07 audit fixes. Paths: CORE = rust/crates/dcpwizard-core/src, CLI = rust/crates/dcpwizard-cli/src/main.rs, PK = ../../postkit (canonical postkit, path dep).

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

## Not implemented, de-advertised (build only if wanted)

Dead modules, now honest in README/docs. Wire or delete:

- VF creation (vf.rs), reel splitting (reel.rs), multi-CPL timelines (multi_cpl.rs),
  stereoscopic 3D (stereo3d.rs), HFR validation table (hfr.rs), subtitle packaging
  into a DCP timed-text track (no --subtitle flag, MxfType::TimedText unused),
  Atmos/DTS:X (atmos.rs/dtsx.rs), channel mapping (mca.rs/audio.rs), J2K transcoder
  (j2k_transcode.rs). The PCM wrapper hardcodes 6ch/24-bit/48k regardless of input.

## KDM (minor)

- Interop KDM: only SMPTE ETM namespaces exist.
- `--formulation` dci-any/dci-specific map to invented MessageType URIs.
- Duration-based end times computed in the start's offset but labelled +00:00.
- AnnotationText hardcoded; no Trusted Device List / DeviceList written.

## Encode / colour / audio

- Encode bandwidth is decorative: StreamEncodeOptions compression_ratio hardcoded
  10.0 (PK/pipeline.rs); only fps is honoured now.
- Colour: no P3-D65; `colour --target xyz` maps to bt709/linear (real transforms
  only in the dcdm path).
- Markers: only FFOC/LFOC emitted; the other eight are defined but never placed.

## HDR, ingest, conform (mostly postkit-side)

- hdr10-inject writes a container tag, not ST 2086/CTA 861.3 SEI (PK/dolby_vision.rs).
- HDR10/HLG/SDR conversion exists in postkit but nothing calls it.
- Camera raw: ARRIRAW/R3D/BRAW detected but ffmpeg can't decode them; Sony not detected.
- ingest has no --lut flag (apply_lut hardcoded false).
- conform parses and prints only (no reel assembly); only CMX3600 EDL and FCP7 xmeml work.

## Export, automation, misc

- Export to ProRes/H.264/H.265/DNxHR: real ffmpeg code but only reachable via the
  batch daemon job type (no subcommand, no GUI).
- Watch folder only detects finished DCPs; webhooks never triggered by the daemon.
- Job queue progress jumps 0→100; cancel only affects pending jobs; `serve` uses a
  separate in-process queue.
- copy-to-drive verify reads back without fsync/O_DIRECT; verify --output .pdf falls
  back to plain text.
- Trailer output is mp4 (no DCP/CPL); accessibility check is substring matching.

## GUI (remaining)

- "Show in Files" likely broken (plugin:shell open with bare path vs URL-only validation).
- Encode/transcode panels: resolution/framerate selects, transcode format/bit-depth,
  copy "verify after copy" checkbox are still decorative.
- scope/flat/full collapses to bare 2K/4K; content kind "episode" becomes Feature.

## Dedup (remaining)

- dcp_diff.rs ~75% similar to imfwizard imp_diff.rs; candidate shared postkit module.
- Dead unadvertised modules to delete or wire: dcp_diff, plugin, preferences,
  geometry (PillarBox == Letterbox), prores re-exports.

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
- .github/workflows/release.yml, gui-release.yml — copies across dcpwizard,
  imfwizard, dcpdoctor differing by binary/artifact names + per-app build deps.
  Separate git repos, so no shared reusable-workflow without a central repo. Keep aligned by hand.
- tests/cli_flags_test.sh — NOT the same harness as imfwizard's anymore (this one
  runs the binary and checks clap parse errors; imf parses main.js). Different CLIs,
  leave separate.
