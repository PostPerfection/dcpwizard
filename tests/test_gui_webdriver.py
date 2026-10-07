import html
import json
import os
import re
import subprocess
import tomllib
import unicodedata
import xml.etree.ElementTree as ElementTree
from datetime import datetime, timezone
from pathlib import Path

import pytest

from tauri_webdriver import SELECT_ALL_CHORD, Window, visible_windows, wait_until

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
PROJECT_FILE_VERSION = 2

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

# crossing the first reel takes longer than the reel lasts when decoding lags
REEL_CROSSING_TIMEOUT_MULTIPLE = 3

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
NO_PICTURE_FOR_MARKER = "Put a video on the first reel to set a marker from the player"

VERIFY_CHORD = "ctrl+3"
VERIFY_VIEW = "view-verify"
VERIFY_STATUS_VERDICTS = {"Verification passed": "PASSED", "Verification failed": "FAILED"}
VERIFY_TIMEOUT_SECONDS = 300
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


# create refuses a .mov for the sound, so the two sources are written apart
def write_media(directory):
    picture = directory / "source.mov"
    sound = directory / "sound.wav"
    run_ffmpeg(
        "-f", "lavfi",
        "-i", f"testsrc2=size={FIXTURE_SIZE}:rate={FIXTURE_FPS}:duration={FIXTURE_SECONDS}",
        "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p",
        str(picture),
    )
    run_ffmpeg(
        "-f", "lavfi",
        "-i", f"sine=frequency=440:sample_rate=48000:duration={FIXTURE_SECONDS}",
        "-ac", str(FIXTURE_CHANNELS), "-c:a", "pcm_s24le",
        str(sound),
    )
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

    first_reel_frames, first_reel_fps = reels[0]
    frame_seconds = 1 / first_reel_fps
    wait_until(
        "playback never reached the second reel",
        lambda: segment_is_active(session, "picture", 1),
        first_reel_frames / first_reel_fps * REEL_CROSSING_TIMEOUT_MULTIPLE,
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
    wait_until(
        "the Preview button never came back at the end of the composition",
        lambda: session.property("#btn-preview", "disabled") is False,
        composition_seconds(reels) * REEL_CROSSING_TIMEOUT_MULTIPLE,
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
    window.click("#btn-recent-projects")
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
    window.press("ctrl+7")
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
    window.press("ctrl+7")
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
    save_in_dialog(window, "#btn-new-project", second_path)
    wait_for_status(session, f"Saved {second_path}", REACTION_TIMEOUT_SECONDS)
    assert session.property("#prop-title", "value") == SECOND_PROJECT_TITLE
    assert session.execute(ASSET_PATHS) == []
    assert saved_asset_paths(saved_project(second_path)) == []
    assert session.execute(DETECTED_FIELDS) == []
    assert session.property(DETECTED_HINT, "hidden") is True

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
    assert session.text(".track-sound .track-info") == f"{CHANNEL_SET_NAME}, 2 channels"
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
