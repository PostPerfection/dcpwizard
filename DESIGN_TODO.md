# DESIGN_TODO

Paths: CORE = rust/crates/dcpwizard-core/src, CLI = rust/crates/dcpwizard-cli/src/main.rs,
PK = extern/postkit (postkit submodule; bump the pin when postkit changes).
DoM refs (dom#N = https://dcpomatic.com/bugs/view.php?id=N) are DCP-o-matic tracker
feature requests. Shared DSP/parsers belong in postkit (see its DESIGN_TODO); the
user-facing surface is here.

## Open

- GUI re-verification owed. None of it has been clicked through in a running
  window: the QC overlays drawn at and across end of file, without freezing and
  without a frame-rate hit (watch the HUD decoder fps), the playlist behaviour
  when rows are cleared (the preview stops or clears when the queue owns it, one
  advance per end of file), the transport bar tracking during playback and its
  skip and frame-step buttons, the decode-resolution menu and HUD, the crop
  overlay and the subtitle/CC render toggles. Everything in guikit's preview
  header is owed the same pass in imfwizard.
- Hints not ported from DCP-o-matic's list, each for a reason. Signing certificate
  checks (utf8 subject strings, a chain valid for more than 15 years) are about
  the configured signer rather than the job, and our signer is held to ST 430-2
  at sign time instead. MPEG2 and VOB inputs do not exist here. Mixed encryption
  cannot happen: `--encrypt` is all or nothing. 3D content in a 2D DCP has no
  equivalent, since a right eye is named per job rather than carried by content.
  The size limits on text assets (an Interop font over 640 kB, a SMPTE reel over
  4096 PNG subtitle resources, a caption XML over 256 kB, a subtitle MXF over
  115 MB) are refusals here or in postkit's font subsetter rather than advice,
  and the ones that are not would need the DCST rendered to measure, which the
  hints pass deliberately does not do.
- Checks left in the front ends on purpose, because each names a flag or a panel
  control and the two spell them differently: every spelling parser (`--rotate`,
  `--flip`, `--upmix`, `--container`, `--marker`, the appearance flags), and the
  pairings that refuse two ways of saying one thing (`--audio-map` beside
  `--upmix`, a channel WAV directory or `--audio-input-order lrc-ls-rs-lfe`;
  `--still-length` with a video; a trim on a still; an appearance flag with no
  track to style; the reel-split sources). `preflight` takes the rules whose
  message names the content, not the control.
- A marker past the composition length is refused after the encode
  (`markers::markers_for_composition`, from `create_dcp`). The plan-time pass
  hints at a marker sitting at or past the picture length rather than refusing
  one, because the frame count it works from is the source's and the packaged
  length also depends on padding the packager applies. Moving the refusal forward
  needs the padded length to be settled in the plan.

- ISDCF naming takes free-text studio codes and territories, where it could pull
  the current ISDCF registry instead of naming from whatever the user typed.
  CORE isdcf_title.rs, GUI pipeline.rs.
- The windows embedded-preview host has not run on real hardware. All three
  hosts are implemented in guikit and CI compiles every platform; macos has had
  a hand pass (the layer-backed GL view needs the same Y flip as linux, or the
  picture is upside down). Windows is still owed that pass.
- Windows release builds are unproven until the next tag run. Watch for grok's
  msvc install dropping more dlls that grokj2k.dll depends on, in which case
  release.yml and gui-release.yml should copy bin/*.dll instead of the one file.
  A local windows tauri build fails at bundle time unless the dll is staged at
  gui/src-tauri/grokj2k.dll.
- TMS upload over sftp is untested past the transport boundary. Every test hands
  `tms` a fake transport, so PK tms.rs `SftpTransport::connect` (the ssh2
  session, host key check, password and key login) is never entered. Closing it
  needs an sftp server in the test.
- TMS vendor certificate fetch (CORE cert_fetch.rs) needs vendor credentials
  to exercise, so the download path has no test and no recorded run.
- Distributed encoding across machines (dom#155, dom#1635, dom#2605). Out of scope
  (user-excluded). The job queue is single-machine and its create path wraps
  pre-encoded J2K rather than running postkit::pipeline, so job progress is
  stage-based, not per-frame.
- Playlist / SPL playback (DCP-o-matic ships this as a separate dcpomatic2_playlist
  tool feeding dcpomatic2_player). Sequence several packages into one list the
  player walks in order. The embedded preview is otherwise at parity with that
  player: postkit `preview` resolves a CPL by uuid, decrypts encrypted picture
  essence with the content key, colour-manages and can drive a GPU decoder. What it
  lacks is the list, since `PlaybackOptions` names one `input` and one `cpl_uuid`,
  so this needs a queue above it plus GUI ordering. Nothing about package
  correctness depends on it.
- 4K playback is not real time. PK grok_player's pool runs one grok thread per
  core and sustains 48 fps at 2048x1080 on 16 cores, and a 4096x2160 frame is
  four times the samples, so 4K wants a GPU decoder rather than more CPU workers.
  Stereoscopic J2K stays on libmpv, since `GrokPlayer::accepts` refuses it and
  nothing there pairs the two eyes. The pool reads ahead two frames per worker,
  32 on this machine, a frame count with nothing bounding the bytes those frames
  hold, so the cache costs four times as much on 4K as on 2K. GUI.
- `report --scan-picture` decodes through ffmpeg's filters, so it is bound by that
  same few frames a second. Moving it to grok at `reduce` 2 would be around fifty
  times faster, at the cost of writing the black and frozen tests in Rust rather
  than reading blackdetect and freezedetect.
- The GUI's create step has no watermark field: `create --watermark` and the
  `watermark` command that marks a finished DCP are CLI only.
- The GUI's Jobs panel lists both queues but they stay separate: the GUI runs its own
  queue in tauri state (`postkit::gui_job_queue`) and only `serve` proxies to the daemon, so
  two queues exist on one machine and neither can take the other's jobs. The daemon
  cannot take a GUI job as things stand: its `CreateDcp` runs
  `create_dcp_with_progress` over a `DcpConfig` of already-encoded J2K, while a GUI
  job encodes through postkit and reports per-frame progress. One queue for both
  needs a job type that carries a GUI build, or the GUI build path moved into CORE
  behind the existing IPC. GUI + CORE.
- DCP-o-matic allows fully custom colour conversions (user chromaticities,
  white point, gamma), a flexibility we do not have anywhere.
- conform gaps (the formats themselves are in DESIGN.md): AAF video is
  code-complete but untested against a real file, since libaaf's public test
  corpus has video tracks but no video clips. AAF pan and gain automation are
  surfaced in the timeline's skipped list but not applied, deliberate scope.

### Batch E (easyDCP parity, surveyed 2026-08-16)

From en.easydcp.com easyDCP Plus and IMF Studio (both now EUR 3567.62 permanent or
EUR 164.22/month, so the README's "EUR 2,998" line is stale). Most of what those
pages advertise is already here. These two are not.

- HD-SDI monitoring output. easyDCP Player+ and IMF Player both drive Blackmagic
  hardware. This is a port rather than new work: imfwizard already has `sdi-preview`,
  which runs a GStreamer decklink pipeline and probes for the plugin first
  (imfwizard-core `tools.rs`, `has_gst_decklink`). The open question is whether the
  embedded preview should feed it or it stays a separate command.
- Atmos KDM. easyDCP's KDM Generator+ advertises "SMPTE (incl. Dolby Atmos)". Not a
  new item: it is the tail of the encrypted timed text and Atmos bullet above, since
  a KDM can only carry an Atmos key once `wrap_atmos` encrypts the essence.

Two claims from those pages we could not judge and should read the specs for before
calling them gaps: "Dolby Vision 4.0 packaging" (imfwizard converts RPU profiles 8.1
and 8.4, unclear whether that is what 4.0 means) and the IMF "Extended" application
alongside App2 and ProRes.

### Transkoder survey (2026-08-17)

From colorfront.com/software/transkoder plus the NAB 2026 press release. Colorfront
publishes no spec sheet, release notes, manual or price; the most detailed feature
list is a reseller page pinned to Transkoder 2022, so their column mixes "verified
current" with "true in 2022, probably still". Confirmed absent or undocumented on
their side: TMS upload, Interop DCPs, watch folders, published pricing, and any
named scope types. Their clear leads are GPU J2K speed with 8K SDI output, camera
RAW ingest, Dolby Vision cinema authoring (DV2, eCMU, licensed), IMF App4/App5/RDD45
breadth, QC detectors, and the render-farm/cloud story. Items worth landing here:

- Side-by-side / wipe / difference compare in the preview. `frame_compare` has the
  metrics (PSNR/SSIM/VMAF); nothing shows two compositions ganged. guikit, so both
  wizards. Transkoder 2026 adds semantic composition diffing on top; metrics plus a
  visual compare is the part worth matching.
- Waveform and vectorscope in the preview. Transkoder implies scopes ("HDR
  analyzer") but never enumerates them. guikit, both wizards.

### GPU rate: where the frames go (measured 2026-09-29)

The encode threads setting landed in grok 20.4.14 (`grk_plugin_init_info::num_threads`,
automatic count from the affinity mask), the plugin, postkit, guikit, dcpwizard and
imfwizard. What the traces said, and what is still owed:

- The laptop is now device bound. An Nsight Systems trace of the 20 s Toms clip at
  40 fps has a kernel running for 94% of the encode span, `mqcoder_rc` summing
  16.9 s of kernel time in 9 s of wall time, two launches overlapping for half of
  it. btop shows the CPUs below full and the GPU at full. Nsight Compute showed
  each `mqcoder_rc` launch at 5 to 15% SM busy: one thread per code block, 219
  blocks of 32 threads a launch, 0.46 waves on 30 SMs, warps stalled on latency.
  So the kernel's occupancy is the next lever. Before the host cuts the device
  idled 55% of the mqcoder time.
- One MQ launch for all three channels: at 4K the plugin launched the MQ coder once
  per channel (`splitChannelsAcrossBPC_MQKernelRuns` above `threshold_2k_4k`) so the
  channels could share one context stream, which is sized for 31 bit planes, 250 MB
  per 4K channel. On CUDA the plugin now takes one launch when the three channel
  stream fits a surface's 65536 rows and two frames of it fit the card. The 656
  block launch takes 12 ms where a 219 block launch took 10.5. Steady state on the
  60 s Toms clip went from 48.3 to 49.7 fps to 50.2 to 51.4, output byte-identical.
  Two frames in flight run as fast as three or six, in either mode. OpenCL, HIP and
  Metal keep the size threshold, since latke has no image height limit to check
  and Metal textures stop at 16384 rows.
- With the single launch, frames run one at a time on the device: MQ launches
  overlap 3% of the time and the device idles 11%. Each frame's upload waits on a
  CUDA host callback, and CUDA runs every host callback on one thread. That thread
  copies the 28 MB input into pinned memory (4 to 13 ms) and the used code stream
  off the device buffer (8 ms), so it was busy back to back. Splitting both copies
  over four threads cut the second to 3.9 ms, but the upload callbacks then waited
  9.4 ms on average for a filled frame, and steady state fell 2 fps from the extra
  threads competing for the laptop's 8 cores. So the laptop is bound by frame
  supply (ffmpeg, the pipe, the submit copy) with the device close behind, and
  device side work needs a benchmark that feeds frames from memory to measure.
- Sizing the context stream by `precision + GPUP_BIBO_EXTRA_BITS` bit planes, as
  the decoder's output buffer already is, would cut it by about 40%, but only if
  the bit plane coder can never exceed that count, which is unchecked.
- Host cuts, all committed (grok 84ee560c and 1fd7053b, plugin a78da57), output
  byte-identical: 30.1 to 40.3 fps on the 20 s Toms clip (4096x1716, 250 Mbit/s)
  and from 180 CPU seconds for the encoder process plus 45 for ffmpeg to 122
  for both together (101 user, 21 sys).
  Rate control searches the slope on per-slope tables of body bytes and hull passes
  and verifies two thresholds instead of bisecting with full simulations. Precincts
  hold their code blocks in one array and allocate the 4 KB code slot only for
  grok's own T1. The bit writer packs header fields a byte at a time. Codecs are
  reused across batch frames. The plugin copies only each block's coded bytes and
  pass entries off the device and writes its final pass records into grok's pass
  storage, so grok takes them without a copy. Its memory estimate uses the real
  buffers and keeps a quarter of the card free while taking up to 12 frames in
  flight. Black and frozen detection is opt-in, so ffmpeg's graph is only the pad.
- Levers left, by share of perf samples in one run (121 CPU seconds, 41.6 fps under
  perf): ffmpeg 30% (DNxHR decode 19% over 16 threads, the raw pipe write 7%, the
  pad on the filter thread 3%). In the encoder process: the plugin to grok pass
  handoff 12% (the plugin's per-block pass sync 5.4%, grok's convex hull 5% with
  its `log`, the tile wrapper update and grok's per-block synch 1.5%), grok's packet
  header coder 13% (three header passes a frame plus the real write), grok's slope
  search 9%, libc memmove 7% (the plugin's two host copies of every incoming frame
  by the earlier trace, callers not resolved in this one), the used code stream
  copy off the device 2.4%, and kernel time 15%, spread over pipe reads, page
  faults and syscalls. A 1 MB pipe buffer, jemalloc, mimalloc, tcmalloc and 8 or
  12 encoder threads changed nothing.
- The plugin running grok's convex hull on each block as it writes the passes, so
  grok skips its own hull, was tried: 40.5 against 40.1 fps over three alternating
  pairs, 1 to 3 user CPU seconds less, picture essence byte-identical.
- Pass records and hull on the device: a kernel after the last MQ launch reads
  each block's `PassInfo` entries and its coded bytes and writes grok's pass
  records (pass count, bit planes, the 0xFF trimmed rate, length, cumulative
  distortion in double with the band's MSE weight, no FMA so it matches the host
  bit for bit), then the convex hull. grok then skips `compress_synch_with_plugin`'s
  pass work and `RateControl::convexHull` for plugin tiles. The hull's `slopeToLog`
  calls `log`, and CUDA's double `log` is not correctly rounded, so a slope that
  lands on a 16-bit step can differ from glibc's. Either compare every frame's
  records against the host path in a debug run, or return the double slope and
  keep `slopeToLog` on the host. Kernel pattern: the plugin's MQ wrapper and its
  CUDA launch (`MQ_RC_CUDA`).
- Device compaction of the tile buffer: pack the used code bytes and pass entries
  on the device before the copy, so the host copy is one `memcpy` instead of the
  per-block loop in `copyUsedCodeStream`.
- Decode in process: link libavcodec instead of piping raw frames from ffmpeg,
  which removes the pipe write and read (9% together) and one copy of each frame.
- With codec reuse, a `GRK_PLUGIN_STATE_DEBUG` run whose CPU T1 writes passes
  writes into the plugin's pass array after frame 1. Normal runs are unaffected.
- Profiling needs root for GPU counters on this Fedora (`RmProfilingAdminOnly: 1`):
  `sudo /tmp/bench/ncu_mq.sh`, or `options nvidia NVreg_RestrictProfilingToAdminUsers=0`
  in modprobe.d and a reboot. build-gpu's `libgrokj2k.so.1` symlink pointed at the
  20.4.13 build until ninja relinked the core, so a run can load a stale core library
  after a version bump: check `readlink bin/libgrokj2k.so.1`.
- spain-docker is shared: `/proc/loadavg` in the container is host-wide and swung the
  same encode between 22 and 44 fps. No timing from it counts without the load beside
  it. `~/bench/quiet_matrix.sh` there waits for a quiet minute and runs the matrix.
- Uncompiled: the plugin's OpenCL and Metal encoder and decoder constructors gained
  the thread count without a build, and the affinity helper's Windows branch was only
  compiled under mingw.
- imfwizard's settings field, `set_gpu` round trip and `Encode threads:` log line have
  tests but no desktop run. dcpwizard's field was looked at in a headless render only.
- postkit's `grokj2k-sys` git tag stays at v20.4.3 (bindgen reads the installed header),
  and the settings page's Resolution preference is read by nothing.

### Pad and picture findings off ffmpeg's filter thread (specced 2026-09-29, parked)

A video encode with picture findings on runs ffmpeg with `pad,split[picture][detect];[detect]blackdetect,
freezedetect,nullsink;[picture]null` (PK/src/picture_processing.rs builds the pad,
`with_detection_branch` in PK/src/picture_findings.rs adds the detectors). libavfilter
runs one graph on one thread and pad and freezedetect have no slice threading, so that
thread copies the whole 4K frame once and reads it twice per frame. On spain-docker
(EPYC, 4x4090) ffmpeg alone delivered 15 fps with the graph and 52 without; on the
6900HX laptop with the 3060 it delivers 70 fps with the graph and the GPU is the bound at
30 fps, so nothing below is a win there. It matters only where the card outruns the
decode, which the EPYC box does and the 5070 tester's machine does not (see the entry
above). Measure ffmpeg alone
with the exact graph into `cat` before starting any of it.

- CPU path splits before the pad: the detectors judge the source, and the pad runs only
  on the picture branch. The detectors stay at full scale: an 8x8 area downscale cut 5
  of 108 CPU seconds on the laptop but averages grain away, so a grainy static shot
  reads as frozen sooner. Baseline to beat on the CPU path: decoder wait 6 s of 19 s on
  the box.
- grok API and CUDA plugin: `grk_plugin_batch_memory_info` grows canvas size, pad
  offset and an optional per-frame stats callback (black fraction, ceil(w/8) x
  ceil(h/8) box-averaged luma thumbnail); the preprocess kernel writes the canvas and
  the black value outside the frame rectangle in one pass and accumulates the stats
  with the code blocks, no extra copy. Capability probed by symbol
  (`gpup_batch_memory_canvas_supported`), so a host can plan the graph before the
  batch. Tests in the plugin's `batch` target, configured standalone with auth off.
- postkit device path: build.rs probes grok.h for the symbol and sets a cfg; with the
  accelerator active the plan drops the pad and the pipe carries scaled frames;
  `Batch::begin` fills the canvas fields and the stats callback; the freeze compare
  runs on the host from thumbnails (encoder threads submit out of index order, so
  each is kept until both neighbours are compared) with blackdetect's 0.98 and
  freezedetect's 0.001 thresholds and the 2 s minimum, into the same
  `PictureFindings`. CI's `grok-ref` pins move to the tag that carries the API.
- Metal parity for the RGB48 path behind the same symbol; until then the symbol is
  false on Metal and ffmpeg keeps the pad there.

## Keep in sync with imfwizard (deliberately duplicated, no clean shared home)

The shared *logic* lives in postkit (mpv::MpvPlayer, packaging writers, escape_xml,
parse_srt, pipeline::run_encode) and the shared GUI glue in guikit. What remains
duplicated is app glue with no clean cross-repo home, left as copies. If you edit
one side, mirror the other:

- gui/vite.config.js: per-app, only partially aligned. The dev port differs,
  and consuming guikit needs a `server.fs.allow` plus a `resolve.dedupe` for the
  bare @tauri-apps imports, since guikit sources sit outside the vite root. Mirror
  any other change.
- extern/guikit/src/shortcuts.js: both wizards import it from guikit. dcpdoctor is
  not a submodule consumer and keeps a vendored copy synced by plain cp, so a guikit
  change to this file still needs a manual copy into dcpdoctor. App-agnostic by
  design: all app specifics enter through initShortcuts, never a per-repo edit.
- gui/src/timeline.js: per-app deliberately, a genuine domain difference rather
  than drift, so it is not a guikit candidate. It is a thin renderer over disjoint
  backend structs: dcpwizard's TimelineEntry reels against imfwizard's SegmentEntry
  segments. Unifying would mean a field-mapping layer larger than the duplication.
- gui/src-tauri/src/lib.rs, gui/src-tauri/src/pipeline.rs: app-specific tauri setup
  and build orchestration. They delegate the encode to postkit::pipeline but have
  diverged enough that unifying would need per-divergence config flags. Everything
  the create panel adds sits on dcpwizard-core (sign_language, pad, audio_route,
  reel, profiles, versions) or is dcpwizard-only by format (atmos, stereoscopic 3D,
  the DCI HDR addendum), so there is nothing to mirror. What is shared arrives
  through postkit instead: upmix would port to imfwizard's panel unchanged, and the
  source colour path and the codestream cap reach it by bumping its postkit pin.
- .github/workflows/ci.yml, release.yml, gui-release.yml: copies across dcpwizard,
  imfwizard, dcpdoctor differing by binary/artifact names + per-app build deps.
  Separate git repos, so no shared reusable-workflow without a central repo. Keep
  aligned by hand. Every job that compiles the rust workspace sets grok up through
  PostPerfection/setup-grok@v1 at grok-ref v20.4.4, windows on a separate msvc
  build of the same tag. All three platforms are required in ci, none are
  continue-on-error.
  imfwizard sets grok up in ci, release and gui-release the same way, and links
  grok-ffi as this workspace does, so a postkit change that needs grok reaches
  both wizards.
  dcpdoctor links no grok and sets it up in none of its workflows.
- run-gpu-gui.py: identical apart from the app name, copy it across with sed
  rather than editing one side.
- scripts/relink-macos-grok.sh: identical apart from the app name, copy it across
  with sed rather than editing one side. It runs as beforeBundleCommand and points
  the GUI and the CLI sidecar at the libgrokj2k copy the macOS bundler puts in
  Contents/Frameworks, which that bundler never relinks.
- gui/src-tauri/tauri.macos.conf.json, gui/src-tauri/tauri.linux.conf.json: the
  bundle.macOS.frameworks entry, the beforeBundleCommand and the deb and rpm
  files maps differ only by the app name, which is also the /usr/lib directory
  the GUI build.rs rpath names, so all three move together.
- tests/tauri_webdriver.py: identical apart from the process name it kills leftover
  windows by, copy it across with sed rather than editing one side. Every fix to how
  a native dialog is answered or a window is focused belongs on both sides at once.
- tests/test_gui_webdriver.py: per-app, the suites drive different panels. What has
  to stay aligned is the setup around them: the XDG directories, the dropped session
  bus, the neutral click target and the tauri-driver steps in ci.yml.
- tests/cli_flags_test.sh: NOT the same harness as imfwizard's (this one runs the
  binary and checks clap parse errors, imf parses main.js). Different CLIs, leave
  separate.
