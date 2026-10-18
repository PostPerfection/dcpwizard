import hashlib
import html
import json
import os
import re
import shutil
import subprocess
import time
import tomllib
import unicodedata
import xml.etree.ElementTree as ElementTree
from datetime import datetime, timezone
from pathlib import Path

import pytest

from tauri_webdriver import (
    INPUT_SETTLE_SECONDS,
    POLL_SECONDS,
    SELECT_ALL_CHORD,
    Window,
    visible_windows,
    wait_until,
    xdotool,
)

REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_GUI_BINARY = REPOSITORY_ROOT / "gui/src-tauri/target/release/dcpwizard-gui"
DEFAULT_CLI_BINARY = REPOSITORY_ROOT / "rust/target/release/dcpwizard"
WINDOW_TITLE = "DCP Wizard"

# the heading has no click handler, a click there only focuses the webview
NEUTRAL_TARGET = ".toolbar-left h1"

PAGE_TIMEOUT_SECONDS = 60
OPEN_TIMEOUT_SECONDS = 60
PREVIEW_TIMEOUT_SECONDS = 60
PLAYHEAD_TIMEOUT_SECONDS = 60
STATUS_TIMEOUT_SECONDS = 60
# a click or a key press is answered in the page, not over the network
REACTION_TIMEOUT_SECONDS = 15
CREATE_TIMEOUT_SECONDS = 900
EXPORT_TIMEOUT_SECONDS = 300

REELS_CHORD = "ctrl+2"
REELS_VIEW = "view-reels"
PROJECT_CHORD = "ctrl+1"
PROJECT_VIEW = "view-project"

CANCEL_KEY = "Escape"
TEXT_DIALOG = ".text-dialog"

PREVIEW_DEFAULT_TITLE = "Preview"
GPU_UNAVAILABLE_PREFIX = "GPU encoding unavailable"
UNAVAILABLE_VERSION = "unavailable"

FIXTURE_TITLE = "Two Reel Test"
FIXTURE_SIZE = "1998x1080"
FIXTURE_FPS = 24
FIXTURE_SECONDS = 4
FIXTURE_SPLIT_AT = "00:00:02"
FIXTURE_CHANNELS = 6

PROJECT_WIZARD = "dcpwizard"
PROJECT_FILE_VERSION = 3

PROJECT_TITLE = "Film"
SECOND_PROJECT_TITLE = "Second"
CHANNEL_SET_NAME = "mix"
AUDIO_GAIN_FIELD = "#prop-audio-gain"
AUDIO_GAIN_TYPED = "-6"
AUDIO_GAIN_TYPED_DB = -6
AUDIO_GAIN_TOLERANCE = 0.005
LOUDNESS_TOLERANCE_LU = 0.1
LOUDNESS_TARGET_FIELD = "#prop-loudness"
LOUDNESS_TARGET = "lufs=-20"
LOUDNESS_TARGET_LUFS = -20
TARGET_TOLERANCE_LU = 0.2
SET_GAIN_BUTTON = "#prop-gain-to-target"
MEASURE_BUTTON = "#prop-measure-sound"
MEASURE_SOURCE = "#prop-loudness-source"
MEASURE_DELIVERED = "#prop-loudness-delivered"
MEASURE_STEPS = "#prop-loudness-steps"
LOUDNESS_CHART_CURVE = "#prop-loudness-chart svg.loudness-chart path.loudness-chart-curve"
SOURCE_PREFIX = "Source: Integrated "
DELIVERED_PREFIX = "Delivered: Integrated "
ROUTED_STEP = "Routed the channel set by filename"
GAIN_STEP = "Applied gain/fades"
UNAPPLIED_TARGET_STEP = f"Loudness target {LOUDNESS_TARGET} not applied: the gain sets the level"
PICTURE_SCALE_FIELD = "#prop-picture-scale"
PICTURE_OFFSET_X_FIELD = "#prop-picture-offset-x"
PICTURE_PLAN = "#prop-crop-plan"
# the fixture fills the flat container, so half scale leaves a 998x540 picture centred on it
HALF_SCALE_PLAN = (
    "crop 0/0/0/0 to 1998x1080, rotate none, scale to 998x540 at 50%, pad to 1998x1080 at (500,270)"
)
MOVED_HALF_SCALE_PLAN = (
    "crop 0/0/0/0 to 1998x1080, rotate none, scale to 998x540 at 50%, "
    "pad to 1998x1080 at (600,270), offset (100,0)"
)
CONTAINER_VIEW_TOGGLE = "#prop-preview-in-container"
# a source off the 2K flat raster, so the filtered frame differs from it
CONTAINER_VIEW_SOURCE_SIZE = [1920, 1080]
CONTAINER_VIEW_RASTER = [1998, 1080]
CONTAINER_VIEW_AUTO_RESOLUTION = "Auto (1998\u00d71080)"
# long enough to still be playing when the last filter change lands
CONTAINER_VIEW_CLIP_SECONDS = 60
NEW_PROJECT_CHORD = "ctrl+n"
SAVE_PROJECT_CHORD = "ctrl+s"
BUILD_COMPLETE_STATUS = "Build complete"
FINISHED_BUILD_STAGES = {"Done", "Error", "Cancelled"}
WINDOW_TITLE_SEPARATOR = " - "
# the probe writes this into the video's asset row
FIXTURE_ASSET_SIZE = FIXTURE_SIZE.replace("x", "\u00d7")
# the fixture's x264 stream carries no colour tags
DETECTED_HINT_PREFIX = f"From the source: {FIXTURE_SIZE}, {FIXTURE_FPS} fps, colour space not declared, "
# the bit rate between the two varies by encoder build
DETECTED_HINT_SUFFIX = " Mbit/s. Edit any field to override."
DETECTED_HINT = "#prop-detected-hint"
AUTO_RESOLUTION_OPTION = '#prop-resolution option[value="auto"]'
FIXTURE_AUTO_RESOLUTION = f"Auto ({FIXTURE_ASSET_SIZE})"

# the tick to seek to, as how far along the ruler it has to sit at least
SEEK_TICK_LEFT_PERCENT = 20
SEEK_TICK_ID = "test-seek-tick"

# ST 429-10 defines these ten, in this order
MARKER_LABELS = ["FFOC", "LFOC", "FFTC", "LFTC", "FFOI", "LFOI", "FFEC", "LFEC", "FFMC", "LFMC"]
PICKED_MARKER = "FFEC"
PICKED_MARKER_POSITION = "00:00:03:00"
MARKER_POSITION_INPUT = "#prop-markers .marker-row .marker-position"
MARKER_LABEL_SELECT = "#prop-markers .marker-row .marker-label"
# shift+tab lands on the select without opening its popup, where typing picks an option
PREVIOUS_FIELD_CHORD = "shift+Tab"
NEXT_FIELD_KEY = "Tab"
MARKER_FROM_PLAYER_BUTTON = "#prop-markers .marker-row .marker-from-player"
NO_PICTURE_FOR_MARKER = "Put a video on the timeline to set a marker from the player"

VERIFY_CHORD = "ctrl+3"
VERIFY_VIEW_BUTTON = '.sidebar-btn[data-view="verify"]'
VERIFY_VIEW = "view-verify"
NO_VERIFY_PACKAGE_TEXT = "No DCP selected"
VERIFY_STATUS_VERDICTS = {"Verification passed": "PASSED", "Verification failed": "FAILED"}
VERIFY_TIMEOUT_SECONDS = 300
VERIFY_CANCELLED_TEXT = "Verification cancelled"
HASH_PROGRESS_PREFIX = "Checking hashes "
# the hash check reads this for about 10 s on a laptop
SPARSE_PICTURE_BYTES = 20 * 1024**3
PDF_TIMEOUT_SECONDS = 60
SAVE_REPORT_BUTTON = "#verify-save-report"
SAVE_PDF_BUTTON = "#verify-save-pdf"
SAVED_REPORT_PREFIX = "Saved the report to "
REPORT_VERDICT_PREFIX = "DCP Verification: "
BV21_SECTION_HEADING = "<h2>Bv2.1 profile check: "
REPORT_FINDING = re.compile(r'<li class="\w+">(.*?)</li>')
PDF_MAGIC = b"%PDF-"

TOOLS_CHORD = "ctrl+5"
TOOLS_VIEW = "view-tools"
EXPORT_FINISHED_PREFIX = "Exported to "
EXPORT_RUNNING_TEXT = "Exporting\u2026"

REBUILD_VIEW_BUTTON = '.sidebar-btn[data-view="rebuild"]'
REBUILD_VIEW = "view-rebuild"
REBUILD_TIMEOUT_SECONDS = 300
REBUILT_PREFIX = "Rebuilt into "
REBUILD_RUNNING_TEXT = "Rebuilding\u2026"
REBUILT_TITLE = "Film Rebuilt"
RATING_AGENCY = "http://www.bbfc.co.uk"
SOURCE_RATING_LABEL = "12A"
REBUILT_RATING_LABEL = "15"
KEEP_CONTAINER = "keep"
HASH_CHUNK_BYTES = 1024 * 1024

# the composition metadata fields, what each is given, and the CPL element it lands in
COMPOSITION_METADATA = [
    ("#prop-version-number", "versionNumber", "3", "VersionNumber"),
    ("#prop-chain", "chain", "Odeon", "Chain"),
    ("#prop-distributor", "distributor", "Film Distributors", "Distributor"),
    ("#prop-facility-name", "facilityName", "Post House", "Facility"),
    ("#prop-luminance", "luminance", "48", "Luminance"),
]
LUMINANCE_UNITS_SELECT = "#prop-luminance-units"
TOOLBAR_PROJECT_LABEL = "#project-name"
# typing this into the select picks the unit after it
LUMINANCE_UNITS_TYPED = "candela"
LUMINANCE_UNITS = "candela-per-square-metre"

PROFILE_SELECT = "#prop-profile"
# a click on the label focuses the select without opening its popup
PROFILE_LABEL = 'label[for="prop-profile"]'
PROFILE_HINT = "#prop-profile-hint"
SAVE_PRESET_BUTTON = "#prop-preset-save"
PRESET_NAME = "Festival"
PRESET_FIELD = "#prop-studio"
PRESET_FIELD_KEY = "studio"
PRESET_VALUE = "ABCD"
EDITED_VALUE = "WXYZ"
PROFILE_DRIVEN = "profile-driven"
NO_PROFILE_KEY = "Home"
# End picks the last option, the newest saved preset
LAST_SAVED_PRESET_KEY = "End"
DELETE_PRESET_BUTTON = "#prop-preset-delete"
TEXT_DIALOG_OK = f"{TEXT_DIALOG} button.primary"

XDG_DIRECTORIES = {
    "XDG_CONFIG_HOME": "config",
    "XDG_DATA_HOME": "data",
    "XDG_CACHE_HOME": "cache",
}

SEGMENTS = """
return [...document.querySelectorAll(arguments[0] + " .timeline-segment")].map(
  (segment) => ({
    reelIndex: segment.dataset.reelIndex,
    type: segment.dataset.type,
    duration: segment.querySelector(".segment-duration").textContent,
    active: segment.classList.contains("active"),
  }),
);
"""

COMPONENT_VERSIONS = """
return [...document.querySelectorAll("#component-versions .component-version")].map(
  (row) => [row.querySelector("span").textContent, row.querySelector("output").textContent],
);
"""

MARKER_ROWS = """
return [...document.querySelectorAll("#prop-markers .marker-row")].map((row) => ({
  labels: [...row.querySelectorAll(".marker-label option")].map((option) => option.value),
  label: row.querySelector(".marker-label").value,
  position: row.querySelector(".marker-position").value,
}));
"""

ASSET_PATHS = """
return [...document.querySelectorAll("#asset-list .asset-item")].map(
  (item) => item.querySelector(".asset-name").title,
);
"""

ASSET_METAS = """
return [...document.querySelectorAll("#asset-list .asset-meta")].map((meta) => meta.textContent);
"""

CHANNEL_SET_ROWS = """
return [...document.querySelectorAll("#asset-list .asset-item.channel-set")].map((item) => ({
  name: item.querySelector(".asset-name").textContent,
  files: [...item.querySelectorAll(".asset-files > div")].map((line) => line.textContent),
}));
"""

AUDIO_MAP_ROWS = """
return [...document.querySelectorAll("#prop-audio-map tbody tr")].map((row) => ({
  name: row.querySelector("th").title,
  autoRouted: [...row.querySelectorAll("input.auto-routed")].map((cell) => cell.dataset.lane),
}));
"""

DETECTED_FIELDS = """
return [...document.querySelectorAll("#view-project .detected")].map((field) => field.id);
"""

# guikit stores the recent list only once it knows the form a New would discard
RECENT_LIST_STORED = """
return localStorage.getItem("dcpwizard-recent-projects") !== null;
"""

SHOWN_FRAME_SIZE = """
return window.__TAURI_INTERNALS__.invoke("preview_get_metadata").then((text) => {
  const metadata = JSON.parse(text);
  return [metadata.shown_frame_width, metadata.shown_frame_height];
});
"""

RECENT_ROWS = """
return [...document.querySelectorAll("#recent-list .recent-item")].map((item) => ({
  path: item.dataset.path,
  retitle: item.querySelector(".recent-retitle") !== null,
}));
"""

PLAYHEAD_LEFT = """
const playhead = document.getElementById("ruler-playhead");
return playhead ? parseFloat(playhead.style.left) : null;
"""
PLAYBACK_STALL_SECONDS = 10


# a busy runner decodes slower than real time, so only a stopped playhead fails
def wait_while_playing(describe, check, session):
    began = time.monotonic()
    playhead = None
    moved_at = began
    while True:
        try:
            result = check()
            current = session.execute(PLAYHEAD_LEFT)
        except OSError:
            # the driver drops a connection now and then, and a poll can retry
            result, current = None, playhead
        if result:
            return result
        now = time.monotonic()
        if current != playhead:
            playhead, moved_at = current, now
        elif now - moved_at >= PLAYBACK_STALL_SECONDS:
            raise AssertionError(
                f"{describe}, the playhead stopped at {playhead}% "
                f"{now - began:.1f}s after the wait began"
            )
        time.sleep(POLL_SECONDS)

# css cannot pick a tick out by where it sits, so the one to click is given an id
MARK_TICK = """
const ticks = [...document.querySelectorAll("#timeline-ruler .ruler-tick")];
const wanted = ticks.find((tick) => parseFloat(tick.style.left) >= arguments[0]);
if (!wanted) return null;
wanted.id = arguments[1];
return wanted.dataset.time;
"""


def gui_binary():
    binary = Path(os.environ.get("DCPWIZARD_GUI", DEFAULT_GUI_BINARY))
    assert binary.is_file(), f"GUI binary not found at {binary}"
    return binary


def cli_binary():
    binary = Path(os.environ.get("DCPWIZARD_CLI", DEFAULT_CLI_BINARY))
    assert binary.is_file(), f"CLI binary not found at {binary}"
    return binary


def application_environment(root):
    environment = dict(os.environ)
    # the app runs under XWayland on a wayland desktop, and under Xvfb in CI
    environment["GDK_BACKEND"] = "x11"
    for name, directory in XDG_DIRECTORIES.items():
        (root / directory).mkdir(parents=True, exist_ok=True)
        environment[name] = str(root / directory)
    # with a session bus the dialog goes to the desktop's portal, which has no
    # location bar to type a path into
    runtime = root / "runtime"
    runtime.mkdir(mode=0o700, exist_ok=True)
    environment.pop("DBUS_SESSION_BUS_ADDRESS", None)
    environment["XDG_RUNTIME_DIR"] = str(runtime)
    return environment


def open_window(environment, log_path):
    assert os.environ.get("DISPLAY"), "no DISPLAY: run under xvfb-run or a desktop"
    window = Window(gui_binary(), environment, log_path, WINDOW_TITLE)
    try:
        wait_until(
            "the page never loaded",
            lambda: window.session.find("#theme-toggle"),
            PAGE_TIMEOUT_SECONDS,
        )
        window.take_focus(NEUTRAL_TARGET)
    except Exception:
        window.close()
        raise
    return window


def run_ffmpeg(*arguments):
    made = subprocess.run(
        ("ffmpeg", "-y", "-v", "error", *arguments), capture_output=True, text=True
    )
    assert made.returncode == 0, made.stderr


def write_picture(path, size, seconds):
    run_ffmpeg(
        "-f", "lavfi",
        "-i", f"testsrc2=size={size}:rate={FIXTURE_FPS}:duration={seconds}",
        "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p",
        str(path),
    )
    return path


def write_sound(path, seconds):
    run_ffmpeg(
        "-f", "lavfi",
        "-i", f"sine=frequency=440:sample_rate=48000:duration={seconds}",
        "-ac", str(FIXTURE_CHANNELS), "-c:a", "pcm_s24le",
        str(path),
    )
    return path


# create refuses a .mov for the sound, so the two sources are written apart
def write_media(directory):
    picture = write_picture(directory / "source.mov", FIXTURE_SIZE, FIXTURE_SECONDS)
    sound = write_sound(directory / "sound.wav", FIXTURE_SECONDS)
    return picture, sound


class TwoReelPackage:
    def __init__(self, directory, cpl_path, project_path):
        self.directory = directory
        self.cpl_path = cpl_path
        self.project_path = project_path
        self.reels = reel_pictures(cpl_path)


# Recent offers Retitle only for a project file with its package beside it
def write_project_beside(package_directory):
    project_path = package_directory.with_name(f"{package_directory.name}.{PROJECT_WIZARD}")
    project = {
        "wizard": PROJECT_WIZARD,
        "version": PROJECT_FILE_VERSION,
        "saved": datetime.now(timezone.utc).isoformat(),
        "form": {"title": FIXTURE_TITLE},
    }
    project_path.write_text(json.dumps(project))
    return project_path


# the CPL of a package built by the CLI from the fixture media
def create_package(root, *create_arguments):
    picture, sound = write_media(root)
    output = root / "dcp"
    created = subprocess.run(
        (
            str(cli_binary()),
            "create",
            "--title", FIXTURE_TITLE,
            "--video", str(picture),
            "--audio", str(sound),
            "--output", str(output),
            *create_arguments,
        ),
        capture_output=True,
        text=True,
        timeout=CREATE_TIMEOUT_SECONDS,
    )
    assert created.returncode == 0, created.stderr[-4000:]
    # create writes the package into <output>/<title>
    cpls = sorted(output.glob("*/CPL_*.xml"))
    assert len(cpls) == 1, cpls
    return cpls[0]


# one DCP for the whole session, the encode is the slow part of this suite
@pytest.fixture(scope="session")
def two_reel_dcp(tmp_path_factory):
    cpl_path = create_package(tmp_path_factory.mktemp("two-reel"), "--split-at", FIXTURE_SPLIT_AT)
    package = cpl_path.parent
    return TwoReelPackage(package, cpl_path, write_project_beside(package))


# export refuses a CPL with more than one reel
@pytest.fixture(scope="session")
def one_reel_cpl(tmp_path_factory):
    return create_package(tmp_path_factory.mktemp("one-reel"))


def named(parent, name):
    return [node for node in parent.iter() if node.tag.rsplit("}", 1)[-1] == name]


def only_text(parent, name):
    found = named(parent, name)
    assert len(found) == 1, f"{len(found)} {name} elements in {parent.tag}"
    return found[0].text.strip()


# what each reel's picture says it holds, as (frames, frames per second)
def reel_pictures(cpl_path):
    pictures = []
    for reel in named(ElementTree.parse(cpl_path).getroot(), "Reel"):
        picture = named(reel, "MainPicture")[0]
        numerator, denominator = only_text(picture, "EditRate").split()
        pictures.append(
            (
                int(only_text(picture, "IntrinsicDuration")),
                round(int(numerator) / int(denominator)),
            )
        )
    return pictures


# the label timeline.js writes into .segment-duration
def timecode_short(seconds):
    if seconds <= 0:
        return "0:00"
    hours = int(seconds // 3600)
    minutes = int(seconds % 3600 // 60)
    whole_seconds = int(seconds % 60)
    if hours > 0:
        return f"{hours}:{minutes:02d}:{whole_seconds:02d}"
    return f"{minutes}:{whole_seconds:02d}"


def expected_segments(reels, track):
    return [
        {
            "reelIndex": str(index),
            "type": track,
            "duration": timecode_short(frames / fps),
        }
        for index, (frames, fps) in enumerate(reels)
    ]


def segments(session, track):
    return session.execute(SEGMENTS, f"#timeline-{track}")


def listed_segments(session, track, reels):
    found = wait_until(
        f"the {track} track never listed {len(reels)} reels",
        lambda: segments(session, track) or None,
        OPEN_TIMEOUT_SECONDS,
    )
    return [
        {key: value for key, value in segment.items() if key != "active"}
        for segment in found
    ]


def active_view(session):
    return session.execute("return document.querySelector('.view.active')?.id")


def wait_for_view(session, view):
    wait_until(
        f"{view} never became the active view",
        lambda: active_view(session) == view,
        REACTION_TIMEOUT_SECONDS,
    )


# the real GTK file picker, driven through its location bar
def choose_in_dialog(window, button, path):
    windows_before = visible_windows()
    window.click(button)
    window.answer_file_dialog(path, windows_before)


def segment_is_active(session, track, reel_index):
    return any(
        segment["active"] and segment["reelIndex"] == str(reel_index)
        for segment in segments(session, track)
    )


def composition_seconds(reels):
    return sum(frames / fps for frames, fps in reels)


# guikit builds the dialog on the first ask, so before that there is no element
def text_dialog_open(session):
    return session.find(TEXT_DIALOG) is not None and session.property(
        TEXT_DIALOG, "open"
    )


def preview_title(session):
    return session.property("#preview-title", "textContent")


def status_text(session):
    return session.property("#status-text", "textContent")


# what the scrubber says the player holds, in seconds
def reported_duration(session):
    return float(session.attribute("#timeline-duration", "data-raw"))


def save_in_dialog_by_chord(window, chord, path):
    windows_before = visible_windows()
    window.press(chord)
    window.answer_save_dialog(path, windows_before)


def save_in_dialog(window, button, path):
    windows_before = visible_windows()
    window.click(button)
    window.answer_save_dialog(path, windows_before)


def window_title(session):
    return session.execute("return document.title")


def wait_for_status(session, status, timeout_seconds):
    wait_until(
        f"the status never read {status!r}",
        lambda: status_text(session) == status,
        timeout_seconds,
    )


def saved_project(path):
    return json.loads(path.read_text())


def saved_asset_paths(project):
    return [asset["path"] for asset in project["form"]["project"]["assets"]]


@pytest.fixture
def window(tmp_path):
    opened = open_window(application_environment(tmp_path), tmp_path / "driver.log")
    yield opened
    # pytest shows this only when the test failed
    print(opened.output())
    opened.close()


def test_the_reels_view_lists_the_reels_and_follows_playback(window, two_reel_dcp):
    session = window.session
    reels = two_reel_dcp.reels
    assert len(reels) == 2, reels

    # opening a project clears the preview selection
    window.click("#btn-project-menu")
    choose_in_dialog(window, "#btn-project-open", two_reel_dcp.project_path)
    wait_until(
        "the opened project never reached the Recent list",
        lambda: session.find(".recent-retitle"),
        REACTION_TIMEOUT_SECONDS,
    )
    choose_in_dialog(window, "#btn-open-dcp", two_reel_dcp.directory)
    window.press(REELS_CHORD)
    wait_for_view(session, REELS_VIEW)

    assert listed_segments(session, "picture", reels) == expected_segments(
        reels, "picture"
    )
    assert listed_segments(session, "sound", reels) == expected_segments(reels, "sound")

    assert session.property("#timeline-play-btn", "disabled") is True
    # the panel is hidden until the first load, and a hidden element renders no text
    assert preview_title(session) == PREVIEW_DEFAULT_TITLE

    window.click("#btn-preview")
    wait_until(
        "the preview panel stayed hidden",
        lambda: session.property("#preview-panel", "hidden") is False,
        PREVIEW_TIMEOUT_SECONDS,
    )
    wait_until(
        "the preview never reported a duration",
        lambda: reported_duration(session) > 0,
        PREVIEW_TIMEOUT_SECONDS,
    )
    wait_until(
        "the Preview button never took the package as what the panel shows",
        lambda: session.property("#btn-preview", "disabled") is True,
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property("#timeline-play-btn", "disabled") is False
    assert preview_title(session).endswith(two_reel_dcp.directory.name)

    started = session.execute(PLAYHEAD_LEFT)
    assert started is not None, "the ruler has no playhead"
    wait_until(
        "the playhead never moved",
        lambda: session.execute(PLAYHEAD_LEFT) > started,
        PLAYHEAD_TIMEOUT_SECONDS,
    )

    _, first_reel_fps = reels[0]
    frame_seconds = 1 / first_reel_fps
    wait_while_playing(
        "playback never reached the second reel",
        lambda: segment_is_active(session, "picture", 1),
        session,
    )
    # one reel's picture file loaded behind the panel would halve this duration
    assert reported_duration(session) == pytest.approx(
        composition_seconds(reels), abs=frame_seconds
    )
    assert session.property("#btn-preview", "disabled") is True

    tick = session.execute(MARK_TICK, SEEK_TICK_LEFT_PERCENT, SEEK_TICK_ID)
    assert tick is not None, "the ruler has no tick to seek to"
    window.click(f"#{SEEK_TICK_ID}")
    wait_until(
        f"the seek to {tick} never moved playback back into the first reel",
        lambda: segment_is_active(session, "picture", 0),
        REACTION_TIMEOUT_SECONDS,
    )
    assert reported_duration(session) == pytest.approx(
        composition_seconds(reels), abs=frame_seconds
    )

    # once playback runs out the panel holds nothing that is previewing, so the
    # button offers the package again
    wait_while_playing(
        "the Preview button never came back at the end of the composition",
        lambda: session.property("#btn-preview", "disabled") is False,
        session,
    )
    assert reported_duration(session) == pytest.approx(
        composition_seconds(reels), abs=frame_seconds
    )
    # the play button is how the package is played again from the start
    assert session.property("#timeline-play-btn", "disabled") is False

    # the retitle box is the app's own dialog, which the preview surface would
    # cover if it were the webview's prompt
    window.press(PROJECT_CHORD)
    wait_for_view(session, PROJECT_VIEW)
    window.click("#btn-project-menu")
    window.click(".recent-retitle")
    wait_until(
        "the retitle dialog never opened",
        lambda: text_dialog_open(session) is True,
        REACTION_TIMEOUT_SECONDS,
    )
    assert (
        session.property(f"{TEXT_DIALOG} input", "value") == two_reel_dcp.directory.name
    )

    window.press(CANCEL_KEY)
    wait_until(
        "the retitle dialog stayed open",
        lambda: text_dialog_open(session) is False,
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property("#timeline-play-btn", "disabled") is False


# the success path needs grok's plugin and a licence, hand tested
def test_saving_the_gpu_setting_reports_the_missing_plugin_and_stays_off(window, tmp_path):
    session = window.session
    window.press("ctrl+9")
    wait_for_view(session, "view-settings")

    window.click("#set-gpu")
    window.click("#settings-form button[type='submit']")
    status = wait_until(
        "the status never mentioned the GPU",
        lambda: status_text(session).startswith(GPU_UNAVAILABLE_PREFIX)
        and status_text(session),
        STATUS_TIMEOUT_SECONDS,
    )
    assert GPU_UNAVAILABLE_PREFIX in status
    assert session.property("#set-gpu", "checked") is False

    preferences_file = (
        tmp_path
        / XDG_DIRECTORIES["XDG_CONFIG_HOME"]
        / "dcpwizard/preferences.json"
    )
    wait_until(
        "the preferences were never written",
        preferences_file.is_file,
        STATUS_TIMEOUT_SECONDS,
    )
    assert json.loads(preferences_file.read_text())["gpu"] is False


def field_classes(session, css):
    return session.property(css, "className").split()


def type_into_field(window, css, text):
    window.click(css)
    window.press(SELECT_ALL_CHORD)
    window.type_text(text)


def pick_profile(window, key, value):
    window.click(PROFILE_LABEL)
    window.press(key)
    wait_until(
        f"the profile select never took {value!r}",
        lambda: window.session.property(PROFILE_SELECT, "value") == value,
        REACTION_TIMEOUT_SECONDS,
    )


def test_a_saved_preset_puts_back_the_values_it_holds(window, tmp_path):
    session = window.session
    type_into_field(window, PRESET_FIELD, PRESET_VALUE)
    window.click(SAVE_PRESET_BUTTON)
    wait_until(
        "the preset name was never asked for",
        lambda: text_dialog_open(session) is True,
        REACTION_TIMEOUT_SECONDS,
    )
    window.type_text(PRESET_NAME)
    window.click(TEXT_DIALOG_OK)
    wait_for_status(session, f"Saved preset {PRESET_NAME}", STATUS_TIMEOUT_SECONDS)

    presets_file = tmp_path / XDG_DIRECTORIES["XDG_CONFIG_HOME"] / "dcpwizard/presets.json"
    [saved] = json.loads(presets_file.read_text())["presets"]
    assert saved["name"] == PRESET_NAME
    assert saved["form"][PRESET_FIELD_KEY] == PRESET_VALUE
    assert "title" not in saved["form"]

    type_into_field(window, PRESET_FIELD, EDITED_VALUE)
    assert session.property(PRESET_FIELD, "value") == EDITED_VALUE
    assert PROFILE_DRIVEN not in field_classes(session, PRESET_FIELD)
    pick_profile(window, NO_PROFILE_KEY, "")
    assert session.property(PROFILE_HINT, "textContent") == ""

    pick_profile(window, LAST_SAVED_PRESET_KEY, PRESET_NAME)
    wait_until(
        "the preset never put its value back",
        lambda: session.property(PRESET_FIELD, "value") == PRESET_VALUE,
        REACTION_TIMEOUT_SECONDS,
    )
    assert PROFILE_DRIVEN in field_classes(session, PRESET_FIELD)
    setting_count = len(saved["form"])
    assert session.property(PROFILE_HINT, "textContent") == (
        f"Set by {PRESET_NAME}: {setting_count} settings. Edit any field to override."
    )
    assert session.property(DELETE_PRESET_BUTTON, "disabled") is False


def test_the_settings_page_lists_the_component_versions(window):
    session = window.session
    window.press("ctrl+9")
    wait_for_view(session, "view-settings")

    rows = wait_until(
        "the component versions were never listed",
        lambda: session.execute(COMPONENT_VERSIONS),
        STATUS_TIMEOUT_SECONDS,
    )
    names = [name for name, _ in rows]
    versions = dict(rows)
    assert names == ["DCP Wizard", "PostKit", "Grok", "Grok plugin", "FFmpeg", "mpv"]
    assert versions["DCP Wizard"] == package_version(REPOSITORY_ROOT / "gui/src-tauri/Cargo.toml")
    assert versions["PostKit"] == package_version(REPOSITORY_ROOT / "extern/postkit/Cargo.toml")
    for name, version in rows:
        assert version and version != UNAVAILABLE_VERSION, f"{name} has no version: {version!r}"


def test_a_marker_row_offers_the_ten_labels_and_takes_a_position(window):
    session = window.session
    assert session.execute(MARKER_ROWS) == []

    window.click("#prop-add-marker")
    wait_until(
        "the marker row never appeared",
        lambda: session.find(MARKER_POSITION_INPUT),
        REACTION_TIMEOUT_SECONDS,
    )
    window.click(MARKER_POSITION_INPUT)
    window.type_text(PICKED_MARKER_POSITION)
    window.press(PREVIOUS_FIELD_CHORD)
    window.type_text(PICKED_MARKER)
    wait_until(
        f"the marker select never took {PICKED_MARKER}",
        lambda: session.property(MARKER_LABEL_SELECT, "value") == PICKED_MARKER,
        REACTION_TIMEOUT_SECONDS,
    )

    assert session.execute(MARKER_ROWS) == [
        {"labels": MARKER_LABELS, "label": PICKED_MARKER, "position": PICKED_MARKER_POSITION}
    ]
    assert session.property(MARKER_FROM_PLAYER_BUTTON, "disabled") is True
    assert session.property(MARKER_FROM_PLAYER_BUTTON, "title") == NO_PICTURE_FOR_MARKER


def fill_composition_metadata(window):
    for field, _, value, _ in COMPOSITION_METADATA:
        window.click(field)
        window.type_text(value)
    window.press(NEXT_FIELD_KEY)
    window.type_text(LUMINANCE_UNITS_TYPED)
    wait_until(
        f"the luminance unit never took {LUMINANCE_UNITS}",
        lambda: window.session.property(LUMINANCE_UNITS_SELECT, "value") == LUMINANCE_UNITS,
        REACTION_TIMEOUT_SECONDS,
    )
    # shortcuts stay off while a field has focus
    window.click(TOOLBAR_PROJECT_LABEL)


def composition_metadata_in(cpl_path):
    asset = named(ElementTree.parse(cpl_path).getroot(), "CompositionMetadataAsset")[0]
    found = {element: only_text(asset, element) for *_, element in COMPOSITION_METADATA}
    found["units"] = named(asset, "Luminance")[0].get("units")
    return found


def package_version(manifest):
    return tomllib.loads(manifest.read_text())["package"]["version"]


def test_a_project_is_created_saved_built_and_opened_again(window, tmp_path):
    session = window.session
    media = tmp_path / "media"
    media.mkdir()
    picture, sound = write_media(media)
    project_path = tmp_path / f"{PROJECT_TITLE}.{PROJECT_WIZARD}"
    package = tmp_path / PROJECT_TITLE
    project_window_title = f"{WINDOW_TITLE}{WINDOW_TITLE_SEPARATOR}{project_path.name}"

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    save_in_dialog_by_chord(window, NEW_PROJECT_CHORD, project_path)
    wait_for_status(session, f"Saved {project_path}", REACTION_TIMEOUT_SECONDS)
    assert session.property("#prop-title", "value") == PROJECT_TITLE
    assert session.text("#project-name") == project_path.stem
    assert session.property("#prop-output", "value") == str(tmp_path)
    assert window_title(session) == project_window_title
    created = saved_project(project_path)
    assert (created["wizard"], created["version"]) == (PROJECT_WIZARD, PROJECT_FILE_VERSION)
    assert created["form"]["title"] == PROJECT_TITLE
    assert created["form"]["outputDir"] == str(tmp_path)

    choose_in_dialog(window, "#import-video", picture)
    wait_until(
        "the video's size was never probed",
        lambda: any(FIXTURE_ASSET_SIZE in meta for meta in session.execute(ASSET_METAS)),
        OPEN_TIMEOUT_SECONDS,
    )
    wait_until(
        "auto resolution never named the fixture's container",
        lambda: session.property(AUTO_RESOLUTION_OPTION, "textContent") == FIXTURE_AUTO_RESOLUTION,
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.execute(DETECTED_FIELDS) == ["prop-resolution", "prop-framerate"]
    assert session.property("#prop-framerate", "value") == str(FIXTURE_FPS)
    detected_hint = session.property(DETECTED_HINT, "textContent")
    assert detected_hint.startswith(DETECTED_HINT_PREFIX), detected_hint
    assert detected_hint.endswith(DETECTED_HINT_SUFFIX), detected_hint
    choose_in_dialog(window, "#import-audio", sound)
    wait_until(
        "the sound never reached the asset list",
        lambda: session.execute(ASSET_PATHS) == [str(picture), str(sound)],
        REACTION_TIMEOUT_SECONDS,
    )
    fill_composition_metadata(window)

    window.press(SAVE_PROJECT_CHORD)
    wait_for_status(session, f"Saved {project_path}", REACTION_TIMEOUT_SECONDS)
    saved = saved_project(project_path)
    assert saved["form"]["title"] == PROJECT_TITLE
    assert saved_asset_paths(saved) == [str(picture), str(sound)]
    for _, key, value, _ in COMPOSITION_METADATA:
        assert saved["form"][key] == value, key
    assert saved["form"]["luminanceUnits"] == LUMINANCE_UNITS

    wait_until(
        "the Build button stayed disabled",
        lambda: session.property("#btn-build", "disabled") is False,
        REACTION_TIMEOUT_SECONDS,
    )
    window.click("#btn-build")
    wait_until(
        "the hints dialog never opened",
        lambda: session.property("#hints-dialog", "hidden") is False,
        STATUS_TIMEOUT_SECONDS,
    )
    window.click("#hints-build")
    wait_until(
        "the build never finished",
        lambda: session.property("#progress-stage", "textContent") in FINISHED_BUILD_STAGES,
        CREATE_TIMEOUT_SECONDS,
    )
    assert status_text(session) == BUILD_COMPLETE_STATUS

    cpls = list(package.glob("CPL_*.xml"))
    assert len(cpls) == 1, sorted(package.iterdir())
    assert composition_metadata_in(cpls[0]) == {
        **{element: value for _, _, value, element in COMPOSITION_METADATA},
        "units": LUMINANCE_UNITS,
    }
    assert (tmp_path / f"{PROJECT_TITLE}.log").is_file()
    built = saved_project(project_path)
    assert datetime.fromisoformat(built["saved"]) > datetime.fromisoformat(saved["saved"])
    assert saved_asset_paths(built) == [str(picture), str(sound)]
    wait_until(
        "Recent never offered Retitle for the built project",
        lambda: {"path": str(project_path), "retitle": True} in session.execute(RECENT_ROWS),
        REACTION_TIMEOUT_SECONDS,
    )

    second_path = tmp_path / f"{SECOND_PROJECT_TITLE}.{PROJECT_WIZARD}"
    window.click("#btn-project-menu")
    save_in_dialog(window, "#btn-new-project", second_path)
    wait_for_status(session, f"Saved {second_path}", REACTION_TIMEOUT_SECONDS)
    assert session.property("#prop-title", "value") == SECOND_PROJECT_TITLE
    assert session.execute(ASSET_PATHS) == []
    assert saved_asset_paths(saved_project(second_path)) == []
    assert session.execute(DETECTED_FIELDS) == []
    assert session.property(DETECTED_HINT, "hidden") is True

    window.click("#btn-project-menu")
    choose_in_dialog(window, "#btn-project-open", project_path)
    wait_for_status(session, f"Opened {project_path.name}", REACTION_TIMEOUT_SECONDS)
    assert session.property("#prop-title", "value") == PROJECT_TITLE
    assert session.text("#project-name") == project_path.stem
    assert session.property("#prop-output", "value") == str(tmp_path)
    assert session.execute(ASSET_PATHS) == [str(picture), str(sound)]
    assert window_title(session) == project_window_title
    for field, _, value, _ in COMPOSITION_METADATA:
        assert session.property(field, "value") == value, field
    assert session.property(LUMINANCE_UNITS_SELECT, "value") == LUMINANCE_UNITS


TIMELINE_SOUND_LABEL = '#timeline-editor [data-track="sound"] .timeline-editor-span-label'
TIMELINE_CLIP = '#timeline-editor .timeline-editor-clip[data-clip-index="{}"]'
TIMELINE_IN_FIELD = '#timeline-editor input[data-field="in"]'
TIMELINE_OUT_FIELD = '#timeline-editor input[data-field="out"]'
SPLIT_REEL_BUTTON = '#timeline-editor [data-action="split-reel"]'
# a second size, so the second clip is fitted onto the first clip's container
SECOND_CLIP_SIZE = "1920x1080"
EACH_CLIP_SECONDS = 2
EACH_CLIP_FRAMES = EACH_CLIP_SECONDS * FIXTURE_FPS
SECOND_CLIP_IN = "00:00:00:12"
SECOND_CLIP_OUT = "00:00:01:18"
SECOND_CLIP_IN_FRAME = 12
SECOND_CLIP_OUT_FRAME = 42
KEPT_FRAMES = [EACH_CLIP_FRAMES, SECOND_CLIP_OUT_FRAME - SECOND_CLIP_IN_FRAME]
TIMELINE_SECONDS = sum(KEPT_FRAMES) / FIXTURE_FPS

CLIP_TITLES = """
return [...document.querySelectorAll("#timeline-editor .timeline-editor-clip")].map((clip) => clip.title);
"""

# the middle of an element's first frame, which the editor reads as the clip's first frame
FIRST_FRAME_OF_CLIP = """
const clip = document.querySelector(arguments[0]);
clip.scrollIntoView({ block: "center" });
const box = clip.getBoundingClientRect();
return [box.left + box.width / arguments[1] / 2, box.top + box.height / 2];
"""


def click_first_frame_of_clip(window, css, kept_frames):
    x, y = window.session.execute(FIRST_FRAME_OF_CLIP, css, kept_frames)
    xdotool(
        "mousemove", str(window.origin[0] + round(x)), str(window.origin[1] + round(y)), "click", "1"
    )
    time.sleep(INPUT_SETTLE_SECONDS)


def clip_title(name, in_timecode, out_timecode):
    return f"{name}, {in_timecode} to {out_timecode}"


# each reel's picture duration, as the CPL plays it
def reel_durations(cpl_path):
    durations = []
    for reel in named(ElementTree.parse(cpl_path).getroot(), "Reel"):
        picture = named(reel, "MainPicture")[0]
        played = named(picture, "Duration") or named(picture, "IntrinsicDuration")
        durations.append(int(played[0].text.strip()))
    return durations


def test_two_clips_with_trims_and_a_reel_start_build_a_two_reel_dcp(window, tmp_path):
    session = window.session
    media = tmp_path / "media"
    media.mkdir()
    first = write_picture(media / "first.mov", FIXTURE_SIZE, EACH_CLIP_SECONDS)
    second = write_picture(media / "second.mov", SECOND_CLIP_SIZE, EACH_CLIP_SECONDS)
    sound = write_sound(media / "timeline.wav", TIMELINE_SECONDS)
    project_path = tmp_path / f"{PROJECT_TITLE}.{PROJECT_WIZARD}"
    package = tmp_path / PROJECT_TITLE
    whole_clip = ("00:00:00:00", f"00:00:{EACH_CLIP_SECONDS:02d}:00")

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    save_in_dialog_by_chord(window, NEW_PROJECT_CHORD, project_path)
    wait_for_status(session, f"Saved {project_path}", REACTION_TIMEOUT_SECONDS)
    choose_in_dialog(window, "#import-video", first)
    choose_in_dialog(window, "#import-video", second)
    choose_in_dialog(window, "#import-audio", sound)
    wait_until(
        "the two clips never took their sources' lengths",
        lambda: session.execute(CLIP_TITLES)
        == [clip_title(first.name, *whole_clip), clip_title(second.name, *whole_clip)],
        OPEN_TIMEOUT_SECONDS,
    )
    assert session.property(TIMELINE_SOUND_LABEL, "textContent") == sound.name
    assert session.property("#prop-split-at", "disabled") is True
    assert session.property("#prop-reel-split-hint", "hidden") is False

    window.click(TIMELINE_CLIP.format(1))
    replace_field_text(window, TIMELINE_IN_FIELD, SECOND_CLIP_IN)
    window.press(NEXT_FIELD_KEY)
    replace_field_text(window, TIMELINE_OUT_FIELD, SECOND_CLIP_OUT)
    window.press(NEXT_FIELD_KEY)
    wait_until(
        "the second clip never took its in and out points",
        lambda: session.execute(CLIP_TITLES)[1] == clip_title(second.name, SECOND_CLIP_IN, SECOND_CLIP_OUT),
        REACTION_TIMEOUT_SECONDS,
    )
    click_first_frame_of_clip(window, TIMELINE_CLIP.format(1), KEPT_FRAMES[1])
    window.click(SPLIT_REEL_BUTTON)

    wait_until(
        "the Build button stayed disabled",
        lambda: session.property("#btn-build", "disabled") is False,
        REACTION_TIMEOUT_SECONDS,
    )
    window.click("#btn-build")
    wait_until(
        "the hints dialog never opened",
        lambda: session.property("#hints-dialog", "hidden") is False,
        STATUS_TIMEOUT_SECONDS,
    )
    window.click("#hints-build")
    wait_until(
        "the build never finished",
        lambda: session.property("#progress-stage", "textContent") in FINISHED_BUILD_STAGES,
        CREATE_TIMEOUT_SECONDS,
    )
    assert status_text(session) == BUILD_COMPLETE_STATUS

    cpls = list(package.glob("CPL_*.xml"))
    assert len(cpls) == 1, sorted(package.iterdir())
    assert reel_durations(cpls[0]) == KEPT_FRAMES
    built = saved_project(project_path)
    assert built["version"] == PROJECT_FILE_VERSION
    timeline = built["form"]["project"]["compositions"][0]["timeline"]
    assert [(clip["inFrame"], clip["outFrame"]) for clip in timeline["clips"]] == [
        (0, EACH_CLIP_FRAMES),
        (SECOND_CLIP_IN_FRAME, SECOND_CLIP_OUT_FRAME),
    ]
    assert timeline["reelStarts"] == [EACH_CLIP_FRAMES]


def write_mono_channel(path):
    run_ffmpeg(
        "-f", "lavfi",
        "-i", f"sine=frequency=440:sample_rate=48000:duration={FIXTURE_SECONDS}",
        "-ac", "1", "-c:a", "pcm_s24le",
        str(path),
    )


def packaged_sound_channels_of(mxf):
    probed = subprocess.run(
        (
            "ffprobe", "-v", "error",
            "-select_streams", "a",
            "-show_entries", "stream=channels",
            "-of", "csv=p=0",
            str(mxf),
        ),
        capture_output=True,
        text=True,
    )
    assert probed.returncode == 0, probed.stderr
    return [int(line) for line in probed.stdout.split()]


# the channel count of every sound track in the package, read off its MXF
def packaged_sound_channels(package):
    return [
        count for mxf in sorted(package.glob("*.mxf")) for count in packaged_sound_channels_of(mxf)
    ]


# the largest sample of every sound track in a file, read back as PCM
def sound_peak(path):
    decoded = subprocess.run(
        ("ffmpeg", "-v", "error", "-i", str(path), "-map", "0:a", "-f", "s32le", "-c:a", "pcm_s32le", "-"),
        capture_output=True,
    )
    assert decoded.returncode == 0, decoded.stderr.decode()
    samples = memoryview(decoded.stdout).cast("i")
    return max(abs(sample) for sample in samples)


def packaged_sound_peak(package):
    sound = [mxf for mxf in sorted(package.glob("*.mxf")) if packaged_sound_channels_of(mxf)]
    assert len(sound) == 1, sorted(package.iterdir())
    return sound_peak(sound[0])


def integrated_lufs_after(session, css, prefix):
    return float(session.property(css, "textContent").removeprefix(prefix).split()[0])


# the source and delivered integrated loudness of one press of Measure
def measure_integrated_lufs(window, expected_steps):
    session = window.session
    window.click(MEASURE_BUTTON)
    wait_until(
        "the sound measurement never came back",
        lambda: session.property(MEASURE_SOURCE, "textContent").startswith(SOURCE_PREFIX)
        and session.property(MEASURE_DELIVERED, "textContent").startswith(DELIVERED_PREFIX),
        STATUS_TIMEOUT_SECONDS,
    )
    assert session.property(MEASURE_STEPS, "textContent").split(", ") == expected_steps
    curve = session.attribute(LOUDNESS_CHART_CURVE, "d")
    assert curve.startswith("M") and "L" in curve, curve
    return (
        integrated_lufs_after(session, MEASURE_SOURCE, SOURCE_PREFIX),
        integrated_lufs_after(session, MEASURE_DELIVERED, DELIVERED_PREFIX),
    )


INSIDE_PROPERTIES_PANEL = """
const panel = document.querySelector("#properties").getBoundingClientRect();
const element = document.querySelector(arguments[0]);
element.scrollIntoView({ block: "center" });
const box = element.getBoundingClientRect();
return box.left >= panel.left && box.right <= panel.right && box.width > 0;
"""


def test_mono_channel_wavs_become_one_channel_set_that_builds(window, tmp_path):
    session = window.session
    media = tmp_path / "media"
    media.mkdir()
    picture, _ = write_media(media)
    left = media / f"{CHANNEL_SET_NAME}_L.wav"
    right = media / f"{CHANNEL_SET_NAME}_R.wav"
    write_mono_channel(left)
    write_mono_channel(right)
    project_path = tmp_path / f"{PROJECT_TITLE}.{PROJECT_WIZARD}"
    package = tmp_path / PROJECT_TITLE

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    save_in_dialog_by_chord(window, NEW_PROJECT_CHORD, project_path)
    wait_for_status(session, f"Saved {project_path}", REACTION_TIMEOUT_SECONDS)

    choose_in_dialog(window, "#import-video", picture)
    wait_until(
        "the video's size was never probed",
        lambda: any(FIXTURE_ASSET_SIZE in meta for meta in session.execute(ASSET_METAS)),
        OPEN_TIMEOUT_SECONDS,
    )
    choose_in_dialog(window, "#import-audio", left)
    wait_until(
        "the left channel never reached the asset list",
        lambda: session.execute(ASSET_PATHS) == [str(picture), str(left)],
        REACTION_TIMEOUT_SECONDS,
    )
    choose_in_dialog(window, "#import-audio", right)
    wait_until(
        "the two channels never became one set",
        lambda: session.execute(ASSET_PATHS) == [str(picture), CHANNEL_SET_NAME],
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.execute(CHANNEL_SET_ROWS) == [
        {"name": CHANNEL_SET_NAME, "files": [f"L {left.name}", f"R {right.name}"]}
    ]
    assert session.property(TIMELINE_SOUND_LABEL, "textContent") == f"{CHANNEL_SET_NAME}, 2 channels"
    wait_until(
        "the mapping matrix never named the set's files",
        lambda: session.execute(AUDIO_MAP_ROWS)
        == [
            {"name": left.name, "autoRouted": ["L"]},
            {"name": right.name, "autoRouted": ["R"]},
        ],
        REACTION_TIMEOUT_SECONDS,
    )

    assert session.execute(INSIDE_PROPERTIES_PANEL, MEASURE_BUTTON)
    source, delivered = measure_integrated_lufs(window, [ROUTED_STEP])
    assert source == delivered
    window.click(AUDIO_GAIN_FIELD)
    window.type_text(AUDIO_GAIN_TYPED)
    window.click(TOOLBAR_PROJECT_LABEL)
    _, quieter = measure_integrated_lufs(window, [ROUTED_STEP, GAIN_STEP])
    assert abs(source + AUDIO_GAIN_TYPED_DB - quieter) < LOUDNESS_TOLERANCE_LU, (source, quieter)

    window.click(LOUDNESS_TARGET_FIELD)
    window.type_text(LOUDNESS_TARGET)
    window.click(TOOLBAR_PROJECT_LABEL)
    wait_until(
        "Set gain to reach target stayed disabled",
        lambda: session.property(SET_GAIN_BUTTON, "disabled") is False,
        REACTION_TIMEOUT_SECONDS,
    )
    window.click(SET_GAIN_BUTTON)
    gain_db = float(session.property(AUDIO_GAIN_FIELD, "value"))
    _, on_target = measure_integrated_lufs(
        window, [ROUTED_STEP, UNAPPLIED_TARGET_STEP, GAIN_STEP]
    )
    assert abs(on_target - LOUDNESS_TARGET_LUFS) < TARGET_TOLERANCE_LU, (gain_db, on_target)

    wait_until(
        "the Build button stayed disabled",
        lambda: session.property("#btn-build", "disabled") is False,
        REACTION_TIMEOUT_SECONDS,
    )
    window.click("#btn-build")
    wait_until(
        "the hints dialog never opened",
        lambda: session.property("#hints-dialog", "hidden") is False,
        STATUS_TIMEOUT_SECONDS,
    )
    window.click("#hints-build")
    wait_until(
        "the build never finished",
        lambda: session.property("#progress-stage", "textContent") in FINISHED_BUILD_STAGES,
        CREATE_TIMEOUT_SECONDS,
    )
    assert status_text(session) == BUILD_COMPLETE_STATUS
    assert packaged_sound_channels(package) == [2]
    ratio = packaged_sound_peak(package) / sound_peak(left)
    assert abs(ratio - 10 ** (gain_db / 20)) < AUDIO_GAIN_TOLERANCE, (ratio, gain_db)


def test_a_channel_set_sounds_a_timeline_of_two_clips(window, tmp_path):
    session = window.session
    media = tmp_path / "media"
    media.mkdir()
    first = write_picture(media / "first.mov", FIXTURE_SIZE, EACH_CLIP_SECONDS)
    second = write_picture(media / "second.mov", FIXTURE_SIZE, EACH_CLIP_SECONDS)
    left = media / f"{CHANNEL_SET_NAME}_L.wav"
    right = media / f"{CHANNEL_SET_NAME}_R.wav"
    write_mono_channel(left)
    write_mono_channel(right)
    project_path = tmp_path / f"{PROJECT_TITLE}.{PROJECT_WIZARD}"
    package = tmp_path / PROJECT_TITLE
    whole_clip = ("00:00:00:00", f"00:00:{EACH_CLIP_SECONDS:02d}:00")

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    save_in_dialog_by_chord(window, NEW_PROJECT_CHORD, project_path)
    wait_for_status(session, f"Saved {project_path}", REACTION_TIMEOUT_SECONDS)
    choose_in_dialog(window, "#import-video", first)
    choose_in_dialog(window, "#import-video", second)
    wait_until(
        "the two clips never took their sources' lengths",
        lambda: session.execute(CLIP_TITLES)
        == [clip_title(first.name, *whole_clip), clip_title(second.name, *whole_clip)],
        OPEN_TIMEOUT_SECONDS,
    )
    choose_in_dialog(window, "#import-audio", left)
    choose_in_dialog(window, "#import-audio", right)
    wait_until(
        "the two channels never became one set",
        lambda: session.execute(ASSET_PATHS) == [str(first), str(second), CHANNEL_SET_NAME],
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property(TIMELINE_SOUND_LABEL, "textContent") == f"{CHANNEL_SET_NAME}, 2 channels"

    wait_until(
        "the Build button stayed disabled",
        lambda: session.property("#btn-build", "disabled") is False,
        REACTION_TIMEOUT_SECONDS,
    )
    window.click("#btn-build")
    wait_until(
        "the hints dialog never opened",
        lambda: session.property("#hints-dialog", "hidden") is False,
        STATUS_TIMEOUT_SECONDS,
    )
    window.click("#hints-build")
    wait_until(
        "the build never finished",
        lambda: session.property("#progress-stage", "textContent") in FINISHED_BUILD_STAGES,
        CREATE_TIMEOUT_SECONDS,
    )
    assert status_text(session) == BUILD_COMPLETE_STATUS
    assert packaged_sound_channels(package) == [2]
    cpls = list(package.glob("CPL_*.xml"))
    assert len(cpls) == 1, sorted(package.iterdir())
    assert reel_durations(cpls[0]) == [2 * EACH_CLIP_FRAMES]


def replace_field_text(window, css, text):
    window.click(css)
    window.press(SELECT_ALL_CHORD)
    window.type_text(text)


def test_the_picture_scale_and_offset_move_the_planned_picture(window, tmp_path):
    session = window.session
    media = tmp_path / "media"
    media.mkdir()
    picture, _ = write_media(media)
    project_path = tmp_path / f"{PROJECT_TITLE}.{PROJECT_WIZARD}"

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    save_in_dialog_by_chord(window, NEW_PROJECT_CHORD, project_path)
    wait_for_status(session, f"Saved {project_path}", REACTION_TIMEOUT_SECONDS)
    choose_in_dialog(window, "#import-video", picture)
    wait_until(
        "the video's size was never probed",
        lambda: any(FIXTURE_ASSET_SIZE in meta for meta in session.execute(ASSET_METAS)),
        OPEN_TIMEOUT_SECONDS,
    )

    assert session.execute(INSIDE_PROPERTIES_PANEL, PICTURE_SCALE_FIELD)
    replace_field_text(window, PICTURE_SCALE_FIELD, "50")
    wait_until(
        "the picture plan never showed the half scale",
        lambda: session.text(PICTURE_PLAN) == HALF_SCALE_PLAN,
        REACTION_TIMEOUT_SECONDS,
    )
    replace_field_text(window, PICTURE_OFFSET_X_FIELD, "100")
    wait_until(
        "the picture plan never moved with the offset",
        lambda: session.text(PICTURE_PLAN) == MOVED_HALF_SCALE_PLAN,
        REACTION_TIMEOUT_SECONDS,
    )


def wait_for_shown_frame_size(session, size, what):
    wait_until(what, lambda: session.execute(SHOWN_FRAME_SIZE) == size, PREVIEW_TIMEOUT_SECONDS)


def test_the_preview_in_container_shows_the_frame_the_build_writes(window, tmp_path):
    session = window.session
    picture = tmp_path / "source.mov"
    width, height = CONTAINER_VIEW_SOURCE_SIZE
    run_ffmpeg(
        "-f", "lavfi",
        "-i", f"testsrc2=size={width}x{height}:rate={FIXTURE_FPS}:duration={CONTAINER_VIEW_CLIP_SECONDS}",
        "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p",
        str(picture),
    )

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    choose_in_dialog(window, "#import-video", picture)
    wait_until(
        "the Auto resolution never named the container the build resolves to",
        lambda: session.property(AUTO_RESOLUTION_OPTION, "textContent") == CONTAINER_VIEW_AUTO_RESOLUTION,
        OPEN_TIMEOUT_SECONDS,
    )
    window.click("#btn-preview")
    wait_for_shown_frame_size(session, CONTAINER_VIEW_SOURCE_SIZE, "the preview never showed the source")

    assert session.execute(INSIDE_PROPERTIES_PANEL, CONTAINER_VIEW_TOGGLE)
    assert session.property(CONTAINER_VIEW_TOGGLE, "disabled") is False
    window.click(CONTAINER_VIEW_TOGGLE)
    wait_for_shown_frame_size(session, CONTAINER_VIEW_RASTER, "the container view never showed the raster")
    replace_field_text(window, PICTURE_SCALE_FIELD, "50")
    wait_until(
        "the picture plan never took the half scale",
        lambda: "at 50%" in session.text(PICTURE_PLAN),
        REACTION_TIMEOUT_SECONDS,
    )
    wait_for_shown_frame_size(session, CONTAINER_VIEW_RASTER, "the half scale left the raster")

    assert session.execute(INSIDE_PROPERTIES_PANEL, CONTAINER_VIEW_TOGGLE)
    window.click(CONTAINER_VIEW_TOGGLE)
    wait_for_shown_frame_size(session, CONTAINER_VIEW_SOURCE_SIZE, "turning the view off never brought the source back")


TIMELINE_CLIP_SECONDS = 10
TIMELINE_SEEK_SECONDS = 5
TIMELINE_HALFWAY = "50%"
REELS_VIEW_BUTTON = '.sidebar-btn[data-view="reels"]'
PROJECT_VIEW_BUTTON = '.sidebar-btn[data-view="project"]'

PREVIEW_DURATION = """
return window.__TAURI_INTERNALS__.invoke("preview_get_metadata").then((text) => JSON.parse(text).duration);
"""

PREVIEW_PAUSED = """
return window.__TAURI_INTERNALS__.invoke("preview_get_metadata").then((text) => JSON.parse(text).paused);
"""

PREVIEW_PLAY_PAUSE = """
return window.__TAURI_INTERNALS__.invoke("preview_play_pause").then(() => true);
"""

# what the preview's scrubber sends while it is dragged
PREVIEW_SEEK = """
return window.__TAURI_INTERNALS__.invoke("preview_seek_absolute", {seconds: arguments[0]}).then(() => true);
"""

RULER_PLAYHEAD_LEFT = """
return document.getElementById("ruler-playhead")?.style.left ?? null;
"""
RULER_PLAYHEAD_AT_START = "0%"
TRIM_START_FIELD = "#prop-trim-start"
TRIM_START = "12f"
TRIM_START_TIMECODE = "00:00:00:12"


def test_scrubbing_the_preview_moves_the_project_timeline_playhead(window, tmp_path):
    session = window.session
    picture = tmp_path / "source.mov"
    run_ffmpeg(
        "-f", "lavfi",
        "-i", f"testsrc2=size=640x360:rate={FIXTURE_FPS}:duration={TIMELINE_CLIP_SECONDS}",
        "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p",
        str(picture),
    )

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    choose_in_dialog(window, "#import-video", picture)
    window.click(REELS_VIEW_BUTTON)
    wait_until(
        "the project timeline never drew its playhead",
        lambda: session.execute(RULER_PLAYHEAD_LEFT) is not None,
        OPEN_TIMEOUT_SECONDS,
    )
    window.click("#btn-preview")
    wait_until(
        "the preview never reported a duration",
        lambda: (session.execute(PREVIEW_DURATION) or 0) > 0,
        PREVIEW_TIMEOUT_SECONDS,
    )
    # mpv plays on load, so the playhead only passes through halfway
    session.execute(PREVIEW_PLAY_PAUSE)
    wait_until(
        "the preview never paused",
        lambda: session.execute(PREVIEW_PAUSED),
        REACTION_TIMEOUT_SECONDS,
    )

    session.execute(PREVIEW_SEEK, TIMELINE_SEEK_SECONDS)
    wait_until(
        "the timeline playhead never followed the preview",
        lambda: session.execute(RULER_PLAYHEAD_LEFT) == TIMELINE_HALFWAY,
        REACTION_TIMEOUT_SECONDS,
    )

    window.click(PROJECT_VIEW_BUTTON)
    replace_field_text(window, TRIM_START_FIELD, TRIM_START)
    window.press(NEXT_FIELD_KEY)
    wait_until(
        "Trim off the head never moved the clip's in point, which Play timeline plays from",
        lambda: session.execute(CLIP_TITLES)
        == [clip_title(picture.name, TRIM_START_TIMECODE, f"00:00:{TIMELINE_CLIP_SECONDS:02d}:00")],
        REACTION_TIMEOUT_SECONDS,
    )


PLAY_TIMELINE_BUTTON = '#timeline-editor [data-action="play-timeline"]'
PLAYED_CLIP_SIZE = "640x360"
FRAME_SECONDS = 1 / FIXTURE_FPS

PREVIEW_METADATA = """
return window.__TAURI_INTERNALS__.invoke("preview_get_metadata").then((text) => JSON.parse(text));
"""

EDITOR_PLAYHEAD_LEFT = """
return document.querySelector("#timeline-editor .timeline-editor-playhead")?.style.left ?? null;
"""

SEGMENT_LABELS = """
return [...document.querySelectorAll(arguments[0] + " .segment-label")].map((label) => label.textContent);
"""


def test_play_timeline_plays_the_trimmed_clips_back_to_back(window, tmp_path):
    session = window.session
    media = tmp_path / "media"
    media.mkdir()
    first = write_picture(media / "first.mov", PLAYED_CLIP_SIZE, EACH_CLIP_SECONDS)
    second = write_picture(media / "second.mov", PLAYED_CLIP_SIZE, EACH_CLIP_SECONDS)
    sound = write_sound(media / "timeline.wav", TIMELINE_SECONDS)
    whole_clip = ("00:00:00:00", f"00:00:{EACH_CLIP_SECONDS:02d}:00")

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    choose_in_dialog(window, "#import-video", first)
    choose_in_dialog(window, "#import-video", second)
    choose_in_dialog(window, "#import-audio", sound)
    wait_until(
        "the two clips never took their sources' lengths",
        lambda: session.execute(CLIP_TITLES)
        == [clip_title(first.name, *whole_clip), clip_title(second.name, *whole_clip)],
        OPEN_TIMEOUT_SECONDS,
    )
    window.click(TIMELINE_CLIP.format(1))
    replace_field_text(window, TIMELINE_IN_FIELD, SECOND_CLIP_IN)
    window.press(NEXT_FIELD_KEY)
    replace_field_text(window, TIMELINE_OUT_FIELD, SECOND_CLIP_OUT)
    window.press(NEXT_FIELD_KEY)
    wait_until(
        "the second clip never took its in and out points",
        lambda: session.execute(CLIP_TITLES)[1] == clip_title(second.name, SECOND_CLIP_IN, SECOND_CLIP_OUT),
        REACTION_TIMEOUT_SECONDS,
    )
    playhead_before = session.execute(EDITOR_PLAYHEAD_LEFT)

    window.click(PLAY_TIMELINE_BUTTON)
    wait_until(
        "the preview never reported the timeline's duration",
        lambda: reported_duration(session) > 0,
        PREVIEW_TIMEOUT_SECONDS,
    )
    assert reported_duration(session) == pytest.approx(TIMELINE_SECONDS)
    assert session.execute(PREVIEW_METADATA)["duration"] == pytest.approx(TIMELINE_SECONDS, abs=FRAME_SECONDS)
    wait_until(
        "the editor playhead never moved with the timeline",
        lambda: session.execute(EDITOR_PLAYHEAD_LEFT) != playhead_before,
        PLAYHEAD_TIMEOUT_SECONDS,
    )
    wait_until(
        "the timeline never played into the second clip",
        lambda: session.execute(PREVIEW_METADATA).get("timeline_clip_index") == 1,
        PLAYHEAD_TIMEOUT_SECONDS,
    )
    window.click(REELS_VIEW_BUTTON)
    wait_until(
        "the reels view playhead never followed the playing timeline",
        lambda: session.execute(RULER_PLAYHEAD_LEFT) not in (None, RULER_PLAYHEAD_AT_START),
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.execute(SEGMENT_LABELS, "#timeline-picture") == [f"R1 {first.name}, {second.name}"]



SCROLLED_PREVIEW_SIZE = [640, 360]
SCROLLED_PREVIEW_SECONDS = 10
# at the test window size the docked preview scrolls with the project view's left column
PREVIEW_SCROLL_CONTAINER = "#view-project .workspace-main"
# a surface top above the window is what GTK margins could not place
SURFACE_ABOVE_WINDOW_PIXELS = 40
GTK_ALLOCATION_WARNING = "adjust its size allocation"
LAYOUT_SETTLE_SECONDS = 1

SCROLL_SURFACE_ABOVE_WINDOW = """
const container = document.querySelector(arguments[0]);
const surface = document.getElementById("preview-surface");
container.scrollTop += surface.getBoundingClientRect().top + arguments[1];
const surfaceBox = surface.getBoundingClientRect();
const containerBox = container.getBoundingClientRect();
return {
  surfaceTop: surfaceBox.top,
  surfaceHeight: surfaceBox.height,
  visibleTop: Math.max(surfaceBox.top, containerBox.top, 0),
  visibleBottom: Math.min(surfaceBox.bottom, containerBox.bottom, window.innerHeight),
  centreX: surfaceBox.left + surfaceBox.width / 2,
};
"""

WATCH_POINTER_OVER_SURFACE = """
window.pointerReachedSurface = false;
document.getElementById("preview-surface").addEventListener("pointermove", () => {
  window.pointerReachedSurface = true;
});
"""

POINTER_REACHED_SURFACE = "return window.pointerReachedSurface;"


def test_a_preview_scrolled_partly_out_of_its_column_stays_inside_it(window, tmp_path):
    session = window.session
    width, height = SCROLLED_PREVIEW_SIZE
    picture = write_picture(tmp_path / "source.mov", f"{width}x{height}", SCROLLED_PREVIEW_SECONDS)
    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    choose_in_dialog(window, "#import-video", picture)
    window.click("#btn-preview")
    wait_for_shown_frame_size(session, SCROLLED_PREVIEW_SIZE, "the preview never showed the source")

    scrolled = session.execute(SCROLL_SURFACE_ABOVE_WINDOW, PREVIEW_SCROLL_CONTAINER, SURFACE_ABOVE_WINDOW_PIXELS)
    assert scrolled["surfaceTop"] == pytest.approx(-SURFACE_ABOVE_WINDOW_PIXELS, abs=1), scrolled
    visible_height = scrolled["visibleBottom"] - scrolled["visibleTop"]
    assert 0 < visible_height < scrolled["surfaceHeight"], scrolled
    time.sleep(LAYOUT_SETTLE_SECONDS)

    session.execute(WATCH_POINTER_OVER_SURFACE)
    xdotool(
        "mousemove",
        str(window.origin[0] + round(scrolled["centreX"])),
        str(window.origin[1] + round((scrolled["visibleTop"] + scrolled["visibleBottom"]) / 2)),
    )
    wait_until(
        "the page never saw the pointer over the picture",
        lambda: session.execute(POINTER_REACHED_SURFACE),
        REACTION_TIMEOUT_SECONDS,
    )
    app_output = window.log_path.read_text(errors="replace")
    assert GTK_ALLOCATION_WARNING not in app_output, "GTK could not place the scrolled picture"

def counted_video_frames(movie):
    probed = subprocess.run(
        (
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-count_packets",
            "-show_entries", "stream=nb_read_packets",
            "-of", "csv=p=0",
            str(movie),
        ),
        capture_output=True,
        text=True,
    )
    assert probed.returncode == 0, probed.stderr
    return int(probed.stdout.strip())


def export_result(session):
    return session.property("#export-result-text", "textContent")


def test_the_export_tool_writes_a_prores_from_a_dcp(window, one_reel_cpl, tmp_path):
    session = window.session
    output = tmp_path / "export.mov"
    package = one_reel_cpl.parent
    [(frames, _)] = reel_pictures(one_reel_cpl)

    window.press(TOOLS_CHORD)
    wait_for_view(session, TOOLS_VIEW)
    choose_in_dialog(window, "#export-browse-folder", package)
    wait_until(
        "the chosen package never reached the input field",
        lambda: session.property("#export-input", "value") == str(package),
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property("#export-format", "value") == "prores"
    save_in_dialog(window, "#export-browse-output", output)
    wait_until(
        "the chosen movie never reached the output field",
        lambda: session.property("#export-output", "value") == str(output),
        REACTION_TIMEOUT_SECONDS,
    )

    window.click("#export-start")
    result = wait_until(
        "the export never finished",
        lambda: export_result(session) not in ("", EXPORT_RUNNING_TEXT) and export_result(session),
        EXPORT_TIMEOUT_SECONDS,
    )
    assert result == f"{EXPORT_FINISHED_PREFIX}{output}"
    assert session.property("#export-reveal", "hidden") is False
    wait_until(
        "the progress line never reached the last frame",
        lambda: session.property("#export-progress-text", "textContent").startswith(
            f"{frames} / {frames}, "
        ),
        REACTION_TIMEOUT_SECONDS,
    )
    assert counted_video_frames(output) == frames


REBUILD_RATING_ROWS = """
return [...document.querySelectorAll("#rebuild-rating-list .rating-row")].map((row) => [
  row.querySelector(".rating-agency").value,
  row.querySelector(".rating-label").value,
]);
"""


def rebuild_result(session):
    return session.property("#rebuild-result-text", "textContent")


# the CLI's assemble gives the one-reel package a rating to change
def rated_package(package, root):
    output = root / "rated"
    assembled = subprocess.run(
        (
            str(cli_binary()),
            "assemble",
            "--input", str(package),
            "--output", str(output),
            "--title", FIXTURE_TITLE,
            "--rating", f"{RATING_AGENCY}={SOURCE_RATING_LABEL}",
        ),
        capture_output=True,
        text=True,
        timeout=CREATE_TIMEOUT_SECONDS,
    )
    assert assembled.returncode == 0, assembled.stderr[-4000:]
    return output / FIXTURE_TITLE


def only_cpl(package):
    cpls = list(package.glob("CPL_*.xml"))
    assert len(cpls) == 1, sorted(package.iterdir())
    return cpls[0]


def picture_track_hash(package):
    picture_id = only_text(named(ElementTree.parse(only_cpl(package)).getroot(), "MainPicture")[0], "Id")
    assets = named(ElementTree.parse(package / "ASSETMAP.xml").getroot(), "Asset")
    [path] = [only_text(asset, "Path") for asset in assets if only_text(asset, "Id") == picture_id]
    digest = hashlib.sha1()
    with open(package / path, "rb") as track:
        while chunk := track.read(HASH_CHUNK_BYTES):
            digest.update(chunk)
    return digest.hexdigest()


def cpl_ratings(package):
    root = ElementTree.parse(only_cpl(package)).getroot()
    return [(only_text(rating, "Agency"), only_text(rating, "Label")) for rating in named(root, "Rating")]


def test_a_dcp_is_rebuilt_with_a_new_title_and_rating(window, one_reel_cpl, tmp_path):
    session = window.session
    source = rated_package(one_reel_cpl.parent, tmp_path)
    output = tmp_path / "rebuilt"
    output.mkdir()

    window.click(REBUILD_VIEW_BUTTON)
    wait_for_view(session, REBUILD_VIEW)
    assert session.property("#rebuild-start", "disabled") is True
    choose_in_dialog(window, "#rebuild-browse-source", source)
    wait_until(
        "the source's title never reached the title field",
        lambda: session.property("#rebuild-title", "value") == FIXTURE_TITLE,
        REACTION_TIMEOUT_SECONDS,
    )
    summary = session.property("#rebuild-source-summary", "textContent")
    assert "Reels: 1," in summary, summary
    assert session.execute(REBUILD_RATING_ROWS) == [[RATING_AGENCY, SOURCE_RATING_LABEL]]
    assert session.property("#rebuild-container", "value") == KEEP_CONTAINER

    type_into_field(window, "#rebuild-title", REBUILT_TITLE)
    type_into_field(window, "#rebuild-rating-list .rating-label", REBUILT_RATING_LABEL)
    choose_in_dialog(window, "#rebuild-browse-output", output)
    wait_until(
        "the Rebuild button stayed disabled",
        lambda: session.property("#rebuild-start", "disabled") is False,
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property("#rebuild-output", "value") == str(output)

    window.click("#rebuild-start")
    result = wait_until(
        "the rebuild never finished",
        lambda: rebuild_result(session) not in ("", REBUILD_RUNNING_TEXT) and rebuild_result(session),
        REBUILD_TIMEOUT_SECONDS,
    )
    package = output / REBUILT_TITLE
    assert result == f"{REBUILT_PREFIX}{package}"
    assert sorted(path.name for path in output.iterdir()) == [REBUILT_TITLE]
    assert only_text(ElementTree.parse(only_cpl(package)).getroot(), "ContentTitleText") == REBUILT_TITLE
    assert cpl_ratings(package) == [(RATING_AGENCY, REBUILT_RATING_LABEL)]
    assert picture_track_hash(package) == picture_track_hash(source)
    assert session.property("#rebuild-reveal", "hidden") is False


def verify_status(session):
    return status_text(session) in VERIFY_STATUS_VERDICTS and status_text(session)


def first_bv21_finding(report_html):
    _, bv21_section = report_html.split(BV21_SECTION_HEADING, 1)
    return html.unescape(REPORT_FINDING.search(bv21_section).group(1))


# pdftotext splits wrapped lines, drops the hyphen it wrapped at and keeps the font's fi ligature
def comparable_text(text):
    return re.sub(r"[\s-]", "", unicodedata.normalize("NFKC", text))


def pdf_text(pdf):
    extracted = subprocess.run(
        ("pdftotext", str(pdf), "-"), capture_output=True, text=True, check=True
    )
    return comparable_text(extracted.stdout)


def test_the_verify_view_checks_the_opened_package(window, one_reel_cpl):
    session = window.session
    package = one_reel_cpl.parent

    choose_in_dialog(window, "#btn-open-dcp", package)
    window.press(VERIFY_CHORD)
    wait_for_view(session, VERIFY_VIEW)
    wait_until(
        "the opened package never became the one to validate",
        lambda: session.property("#verify-path", "textContent") == str(package),
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property("#verify-run", "disabled") is False


def test_the_verify_view_checks_the_package_played_from_recent(window, two_reel_dcp):
    session = window.session
    window.click("#btn-project-menu")
    choose_in_dialog(window, "#btn-project-open", two_reel_dcp.project_path)
    wait_until(
        "the opened project never reached the Recent list",
        lambda: session.find(".recent-queue"),
        REACTION_TIMEOUT_SECONDS,
    )
    window.click("#btn-project-menu")
    window.click(".recent-queue")
    window.click("#playlist .playlist-title")
    wait_until(
        "the queued package never reached the preview",
        lambda: preview_title(session).endswith(two_reel_dcp.directory.name),
        PREVIEW_TIMEOUT_SECONDS,
    )

    window.press(VERIFY_CHORD)
    wait_for_view(session, VERIFY_VIEW)
    wait_until(
        "the package in the preview never became the one to validate",
        lambda: session.property("#verify-path", "textContent") == str(two_reel_dcp.directory),
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property("#verify-run", "disabled") is False


# the row's own buttons queue, retitle or delete the package instead of opening it
def open_from_recent(window, project_path):
    window.click("#btn-project-menu")
    window.click(f'.recent-item[data-path="{project_path}"] .recent-title')
    wait_until(
        "the project menu stayed open after a recent row was clicked",
        lambda: window.session.property("#project-menu", "hidden") is True,
        REACTION_TIMEOUT_SECONDS,
    )


def wait_for_verify_path(session, path):
    wait_until(
        f"the Verify view never read {path}",
        lambda: session.property("#verify-path", "textContent") == path,
        REACTION_TIMEOUT_SECONDS,
    )


def test_the_verify_view_checks_the_package_of_a_project_opened_from_recent(window, two_reel_dcp):
    session = window.session
    window.click("#btn-project-menu")
    choose_in_dialog(window, "#btn-project-open", two_reel_dcp.project_path)
    wait_until(
        "the opened project never reached the Recent list",
        lambda: session.find(".recent-item"),
        REACTION_TIMEOUT_SECONDS,
    )
    open_from_recent(window, two_reel_dcp.project_path)

    window.click(VERIFY_VIEW_BUTTON)
    wait_for_view(session, VERIFY_VIEW)
    wait_for_verify_path(session, str(two_reel_dcp.directory))
    assert session.property("#verify-run", "disabled") is False


def test_a_project_opened_from_recent_refreshes_the_verify_view_in_place(window, two_reel_dcp, tmp_path):
    session = window.session
    window.click("#btn-project-menu")
    choose_in_dialog(window, "#btn-project-open", two_reel_dcp.project_path)
    wait_until(
        "the opened project never reached the Recent list",
        lambda: session.find(".recent-item"),
        REACTION_TIMEOUT_SECONDS,
    )
    window.press(VERIFY_CHORD)
    wait_for_view(session, VERIFY_VIEW)
    wait_for_verify_path(session, str(two_reel_dcp.directory))

    new_project_path = tmp_path / f"{SECOND_PROJECT_TITLE}.{PROJECT_WIZARD}"
    save_in_dialog_by_chord(window, NEW_PROJECT_CHORD, new_project_path)
    wait_for_status(session, f"Saved {new_project_path}", REACTION_TIMEOUT_SECONDS)
    wait_for_verify_path(session, NO_VERIFY_PACKAGE_TEXT)
    assert session.property("#verify-run", "disabled") is True

    open_from_recent(window, two_reel_dcp.project_path)
    wait_for_verify_path(session, str(two_reel_dcp.directory))
    assert session.property("#verify-run", "disabled") is False
    assert active_view(session) == VERIFY_VIEW


def test_a_validated_package_saves_its_report_as_html_and_pdf(window, one_reel_cpl, tmp_path):
    session = window.session
    package = one_reel_cpl.parent
    report = tmp_path / "report.html"
    pdf = tmp_path / "report.pdf"

    window.press(VERIFY_CHORD)
    wait_for_view(session, VERIFY_VIEW)
    assert session.property(SAVE_REPORT_BUTTON, "disabled") is True
    assert session.property(SAVE_PDF_BUTTON, "hidden") is False
    choose_in_dialog(window, "#verify-browse", package)
    wait_until(
        "the chosen package never reached the Verify view",
        lambda: session.property("#verify-path", "textContent") == str(package),
        REACTION_TIMEOUT_SECONDS,
    )
    window.click("#verify-run")
    status = wait_until("the validation never finished", lambda: verify_status(session), VERIFY_TIMEOUT_SECONDS)
    verdict_line = f"{REPORT_VERDICT_PREFIX}{VERIFY_STATUS_VERDICTS[status]}"
    assert session.property(SAVE_REPORT_BUTTON, "disabled") is False

    save_in_dialog(window, SAVE_REPORT_BUTTON, report)
    wait_for_status(session, f"{SAVED_REPORT_PREFIX}{report}", REACTION_TIMEOUT_SECONDS)
    report_html = report.read_text()
    assert f"<h1>{verdict_line}</h1>" in report_html
    finding = first_bv21_finding(report_html)
    assert finding in session.property("#verify-results", "textContent")

    save_in_dialog(window, SAVE_PDF_BUTTON, pdf)
    wait_for_status(session, f"{SAVED_REPORT_PREFIX}{pdf}", PDF_TIMEOUT_SECONDS)
    assert pdf.read_bytes().startswith(PDF_MAGIC)
    printed = pdf_text(pdf)
    assert comparable_text(verdict_line) in printed, printed
    assert comparable_text(finding) in printed, printed


# a picture MXF grown into a sparse file keeps the hash check reading for seconds
def package_with_sparse_picture(package, tmp_path):
    copy = tmp_path / package.name
    shutil.copytree(package, copy)
    os.truncate(sorted(copy.glob("picture_*.mxf"))[0], SPARSE_PICTURE_BYTES)
    return copy


def processes_reading(path):
    found = []
    for cmdline in Path("/proc").glob("[0-9]*/cmdline"):
        try:
            arguments = cmdline.read_bytes().split(b"\0")
        except (FileNotFoundError, ProcessLookupError):
            continue
        if os.fsencode(path) in arguments:
            found.append(cmdline.parent.name)
    return found


def hash_progress_shown(session):
    value = session.attribute("#verify-progress", "value")
    text = session.property("#verify-progress-text", "textContent")
    return value is not None and float(value) > 0 and text.startswith(HASH_PROGRESS_PREFIX)


def test_cancel_stops_a_running_validation(window, two_reel_dcp, tmp_path):
    session = window.session
    package = package_with_sparse_picture(two_reel_dcp.directory, tmp_path)

    window.click(VERIFY_VIEW_BUTTON)
    wait_for_view(session, VERIFY_VIEW)
    assert session.property("#verify-cancel", "hidden") is True
    assert session.property("#verify-progress-section", "hidden") is True
    choose_in_dialog(window, "#verify-browse", package)
    wait_for_verify_path(session, str(package))

    window.click("#verify-run")
    wait_until(
        "the hash check never showed its progress",
        lambda: hash_progress_shown(session),
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property("#verify-run", "disabled") is True
    assert processes_reading(package), "no verify process was reading the package"

    window.click("#verify-cancel")
    wait_until(
        "the results never said the validation was cancelled",
        lambda: session.property("#verify-results", "textContent") == VERIFY_CANCELLED_TEXT,
        REACTION_TIMEOUT_SECONDS,
    )
    wait_until(
        "the verify process outlived the cancel",
        lambda: not processes_reading(package),
        REACTION_TIMEOUT_SECONDS,
    )
    assert session.property("#verify-results", "textContent") == VERIFY_CANCELLED_TEXT
    assert status_text(session) == VERIFY_CANCELLED_TEXT
    assert session.property("#verify-run", "disabled") is False
    assert session.property("#verify-cancel", "hidden") is True
    assert session.property("#verify-progress-section", "hidden") is True
    assert session.property(SAVE_REPORT_BUTTON, "disabled") is True
