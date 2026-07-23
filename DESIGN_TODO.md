# Planned

Open todos. Paths: CORE = rust/crates/dcpwizard-core/src, CLI = rust/crates/dcpwizard-cli/src/main.rs, PK = extern/postkit (postkit submodule; bump the pin when postkit changes).

## DoM tracker gaps

Feature requests from the DCP-o-matic Mantis tracker (dom#N =
https://dcpomatic.com/bugs/view.php?id=N) that dcpwizard lacks. Shared DSP/parsers
belong in postkit (see postkit DESIGN_TODO); the user-facing surface is here.

- Certificate download for credentialed vendors (dom#2705, dom#2706): cert-fetch
  covers only the credential-free public endpoints (dolby/doremi, qube). Christie
  (`ftp://certificates.christiedigital.com`), GDC (`ftp://ftp.gdc-tech.com`) and
  Barco (`sftp://certificates.barco.com`) have known URL patterns and reachable
  hosts but need vendor-account credentials threaded through before their
  endpoints can be used; they currently error telling the user to get the cert
  from the vendor.
- Distributed encoding across machines (dom#155, dom#1635, dom#2605). Job queue is
  single-machine.
- DCI HDR Addendum DCPs (dom#2374, dom#2799): BLOCKED. `create --hdr-dci` validates
  the flag combo and the per-codestream byte cap (floor(56,250,000/fps) bytes/frame),
  then refuses: the asdcplib-rs jp2k writer exposes no setter for the picture
  TransferCharacteristic=ST 2084 UL, so a compliant HDR claim cannot be written over
  the essence. Unblock: add the UL setter upstream, then let `--hdr-dci` wrap instead
  of exiting.
- Closed-caption (CCAP) packaging: accessibility CCAP track, distinct from the open
  subtitles already wired (placement/RTL/wrap/font/3D-Z + ASS/PAC/MKS/FCPXML/
  Interop-PNG input).
- DTS:X: BLOCKED. postkit declined a generic DCData wrap because the DTS:X
  DataEssenceCoding UL could not be confirmed. dcpwizard used to map DTS:X onto
  the Atmos (IAB) essence UL, which is wrong. The `MxfType::DtsX` variant is
  removed so DTS:X now fails loud as unsupported. Unblock: confirm the DTS:X UL,
  add a postkit essence type + CPL AuxData DataType, then wire a `--dtsx` flag
  mirroring `--atmos`.

## KDM

- AnnotationText override: postkit derives AnnotationText from content_title
  ("<title> KDM for <recipient>"). A separate `--annotation` override needs a
  postkit KdmConfig.annotation field (postkit frozen here), so not added. No
  Trusted Device List / DeviceList is written.
- Interop KDM (`kdm --format interop`) is legacy: no reference library generates
  Interop (libdcp only reads it) and the suite has no reference Interop KDM to diff
  against. Validate against real legacy gear before production use.

## Encode / colour / audio

- Colour: P3-D65 target not added (needs a postkit change, frozen here). `colour
  --target xyz` routes through the real DCDM transform.
- grok's and postkit's RGB->X'Y'Z' transforms differ slightly (grok red
  [2817,2183,870] vs postkit [2914,2258,898], different linearization); both are
  valid X'Y'Z', harmonizing them is open.
- Markers: only FFOC/LFOC emitted; the other eight are defined but never placed.

## HDR, ingest, conform (mostly postkit-side)

- Camera raw: ARRIRAW/R3D/BRAW detected but ffmpeg can't decode them; Sony not detected.
- ingest has no --lut flag (apply_lut hardcoded false).
- conform parses and prints only (no reel assembly); only CMX3600 EDL and FCP7 xmeml work.

## Automation, misc

- Job queue progress jumps 0->100; cancel only affects pending jobs; `serve` uses a
  separate in-process queue.
- Trailer output is mp4 (no DCP/CPL); accessibility check is substring matching.

## GUI (remaining)

- Create-panel pickers (3D/Atmos/loudness/subtitle flags/etc) NOT wired: the create
  panel goes through the tauri `submit_job` -> JobConfig -> run_job job queue, not
  the CLI, so adding create flags needs plumbing through the command signature,
  JobConfig and run_job (structural). Deferred. The CLI path is the way to build
  these DCPs for now.

## Dedup (remaining, postkit-side)

- combine.rs injects the AnnotationText element after `<Id>` via a local helper
  because the postkit AssetMap/PackingList writers have no AnnotationText field; add
  the field to postkit and drop the injection.

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
