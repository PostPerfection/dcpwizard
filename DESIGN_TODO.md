# Planned

Open todos. Paths: CORE = rust/crates/dcpwizard-core/src, CLI = rust/crates/dcpwizard-cli/src/main.rs, PK = extern/postkit (postkit submodule; bump the pin when postkit changes).

## Done 2026-07-23 (app-side batch)

- Credentialed vendor cert download (dom#2705/2706): christie/gdc/barco added to
  cert-fetch with `--user`/`--password`. Path builders (christie 12-digit zero-pad
  + F-IMB->IMB-S2 fallback, gdc /SHA256, barco 10-char + first7-xxx dir) are
  unit-tested; credentials go to curl via a stdin config (`-K -`), never argv or
  logs. Anonymous dolby/qube paths unchanged. No config-file storage exists for
  cert-fetch, so creds are CLI-only.
- Closed-caption (CCAP) packaging: `create --ccap <file>` wraps ST 428-10/429-12
  timed text with a MainClosedCaption CPL role (CORE/cpl.rs splice + CORE/dcp.rs
  single-reel path + PKL/ASSETMAP). Same input formats as subtitle. The multi-reel
  (reel.rs) / vf / versions CPL paths carry CCAP too as of batch 8b (below).
- Markers: `markers --marker LABEL=timecode` (repeatable) places any of the ten
  defined markers, validating the label and the offset (frame or HH:MM:SS:FF)
  against the composition length. Default set stays FFOC/LFOC.
- ingest `--lut`: threads a 3D LUT through postkit ingest (apply_lut/lut_path,
  applied via ffmpeg lut3d). Was hardcoded false.
- Trailer: `trailer` now encodes the packaged mp4 to J2K and builds a real trailer
  DCP (ContentKind=trailer) in `<output>/dcp` via the grok encode + create_dcp path.
- Job queue: create jobs report coarse stage progress (dcp::ProgressSink) instead
  of 0->100; cancel now affects running jobs (per-job AtomicBool checked in the job
  loop + between create_dcp stages, worker runs on its own thread); `serve` proxies
  every job route to the shared daemon queue over IPC (one queue) and returns 503
  when the daemon is down. Note: the queue's create path wraps pre-encoded J2K and
  never runs postkit::pipeline, so progress is stage-based, not per-frame.
- DTS:X: the mxf_wrap essence-type comment and the docs now point DTS:X at the IAB
  (`--atmos`, ST 429-18) path. There is no separate DTS:X CLI surface to fail loud
  on (the DtsX variant was already removed), so this is a doc/comment pointer only.
- conform: `conform --media-dir <dir> --output <dir>` resolves every EDL/xmeml reel
  to media, fails loud on unresolved reels, and writes a reel/asset plan
  (conform_plan.json) + the postkit conform manifest. Per-reel encode + MXF wrap +
  multi-reel CPL assembly into a finished DCP was completed in batch 8b (below).

## Done 2026-07-23 (app-side batch 8b)

- DCI HDR Addendum (dom#2374/2799): `create --hdr-dci` now authors an HDR DCP.
  The picture MXF is wrapped through asdcplib `jp2k::open_write_hdr`
  (CORE/mxf_wrap.rs `wrap_j2k_hdr_files`), setting TransferCharacteristic=ST 2084
  (PQ) + ColorPrimaries=P3-D65 on the essence descriptor. `--hdr-dci` still
  validates the flag combo (needs --hdr-to-dci-lut or --hdr-already-pq) and the
  raised per-codestream cap up front. Not supported with 3D or reel splitting
  (fails loud). Roundtrip test reads the descriptor back and asserts both ULs.
- KDM `--annotation`: CLI flag -> postkit `KdmConfig.annotation` (CORE/kdm.rs,
  CLI Kdm). None keeps the derived "<title> KDM for <recipient>" text. Test
  asserts the override lands, escaped, in the KDM XML.
- Colour `--target p3-d65`: routes through postkit `DcdmTarget::P3D65` alongside
  the xyz branch (CLI `parse_dcdm_target`). Unit test covers the mapping.
- combine.rs: dropped the local `inject_annotation` string-splice; the merged
  PKL/ASSETMAP now carry AnnotationText via the postkit packaging fields
  (`generate_pkl`/`generate_assetmap` gained an `annotation` arg). Output stays
  byte-identical (combine tests pass).
- CCAP in multi-reel / vf / versions: reel.rs, vf.rs and versions.rs now carry
  and emit MainClosedCaption tracks wherever they handle MainSubtitle (VF gains
  `--add-ccap`/`--replace-ccap`; versions manifest gains a `ccap` field). Tests
  mirror the subtitle-path tests.
- conform full assembly: `conform --media-dir --output` now drives the resolved
  reel plan to a finished multi-reel DCP (per-reel grok encode + MXF wrap via
  create_dcp, then assemble.rs multi-reel CPL assembly). The conform_plan.json
  stays as an artifact. Gated e2e test builds a 2-reel DCP from a tiny EDL over
  synthetic media and verifies it with dcpdoctor.

## DoM tracker gaps

Feature requests from the DCP-o-matic Mantis tracker (dom#N =
https://dcpomatic.com/bugs/view.php?id=N) that dcpwizard lacks. Shared DSP/parsers
belong in postkit (see postkit DESIGN_TODO); the user-facing surface is here.

- Certificate download for credentialed vendors (dom#2705, dom#2706): DONE
  2026-07-23 (see Done section). christie/gdc/barco wired with --user/--password.
- Distributed encoding across machines (dom#155, dom#1635, dom#2605). Job queue is
  single-machine.
- DCI HDR Addendum DCPs (dom#2374, dom#2799): DONE 2026-07-23 (see Done batch 8b).
  `--hdr-dci` authors an HDR DCP with ST 2084 / P3-D65 on the picture descriptor.
- Closed-caption (CCAP) packaging: DONE 2026-07-23 for every CPL path (single-reel,
  multi-reel, vf, versions; see Done sections).
- DTS:X: DONE 2026-07-23 (doc/comment pointer to the `--atmos` IAB path; see Done
  section). No public DTS:X DataEssenceCoding UL exists (SMPTE registers have only
  DTS private nodes; asdcplib/libdcp carry nothing). Since ST 429-18/-19 (2019),
  DTS:X is delivered as a standard IAB track per ST 2098-2 ("DTS:X for IAB").

## KDM

- AnnotationText override: DONE 2026-07-23 (`kdm --annotation`; see Done batch 8b).
  No Trusted Device List / DeviceList is written.
- Interop KDM (`kdm --format interop`) is legacy: no reference library generates
  Interop (libdcp only reads it) and the suite has no reference Interop KDM to diff
  against. Validate against real legacy gear before production use.

## Encode / colour / audio

- Colour `--target p3-d65`: DONE 2026-07-23 (see Done batch 8b). `colour --target
  xyz` and `p3-d65` both route through the real DCDM transform.
- grok/postkit RGB->X'Y'Z' harmonization: done in postkit 32838ea ("fix rec709
  to xyz transform, align dcdm gamma"); colour.rs tests assert agreement with
  grok's [2817,2183,870].

## HDR, ingest, conform (mostly postkit-side)

- Camera raw: ARRIRAW/R3D/BRAW/Canon and now Sony RAW / X-OCN are all detected-but-
  undecodable (ffmpeg can't decode them). Sony landed 2026-07-23: postkit's
  `detect_format` reads the .mxf header partition pack + header-metadata region and
  matches Sony's private essence ULs -> `SonyRaw`, rejected loud as "Sony RAW (X-OCN
  family)". Caveat (postkit DESIGN_TODO has the full note): the ULs are reverse-
  engineered from MediaInfo, NOT SMPTE-registered (bmx and ffmpeg have no Sony RAW
  essence at all), and mark the Sony RAW family without distinguishing X-OCN ST/LT/XT
  tiers. A match only produces a clearer detected-but-undecodable error, so the
  family-level granularity is fine. Non-Sony .mxf still resolves to DNxHR.
- ingest `--lut`: DONE 2026-07-23 (see Done section).
- conform: DONE 2026-07-23 (plan + full per-reel encode/wrap/assembly; see Done
  batch 8b). Remaining: only CMX3600 EDL and FCP7 xmeml parse (no AAF/OTIO yet).

## Automation, misc

- Job queue progress / cancel / shared `serve` queue: DONE 2026-07-23 (see Done
  section). Remaining caveat: progress is stage-based (the queue path does not run
  the encode pipeline); distributed encoding is still single-machine (above).
- Trailer: DONE 2026-07-23 (see Done section). Remaining caveat: the accessibility
  check is still substring matching.

## GUI (remaining)

- Create-panel pickers (3D/Atmos/loudness/subtitle flags/etc) NOT wired: the create
  panel goes through the tauri `submit_job` -> JobConfig -> run_job job queue, not
  the CLI, so adding create flags needs plumbing through the command signature,
  JobConfig and run_job (structural). Deferred. The CLI path is the way to build
  these DCPs for now.

## Dedup

- combine.rs AnnotationText: DONE 2026-07-23 (see Done batch 8b). The local
  string-splice is gone; the merged PKL/ASSETMAP use the postkit packaging
  AnnotationText fields. Nothing remaining here.

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
