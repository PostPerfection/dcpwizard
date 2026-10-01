import json
import os
import subprocess
import tomllib
import xml.etree.ElementTree as ElementTree
from datetime import datetime, timezone
from pathlib import Path

import pytest

from tauri_webdriver import Window, visible_windows, wait_until

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
PROJECT_FILE_VERSION = 1

PROJECT_TITLE = "Film"
SECOND_PROJECT_TITLE = "Second"
NEW_PROJECT_CHORD = "ctrl+n"
SAVE_PROJECT_CHORD = "ctrl+s"
BUILD_COMPLETE_STATUS = "Build complete"
FINISHED_BUILD_STAGES = {"Done", "Error", "Cancelled"}
WINDOW_TITLE_SEPARATOR = " - "
# the probe writes this into the video's asset row
FIXTURE_ASSET_SIZE = FIXTURE_SIZE.replace("x", "\u00d7")

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

# the composition metadata fields, what each is given, and the CPL element it lands in
COMPOSITION_METADATA = [
    ("#prop-version-number", "versionNumber", "3", "VersionNumber"),
    ("#prop-chain", "chain", "Odeon", "Chain"),
    ("#prop-distributor", "distributor", "Film Distributors", "Distributor"),
    ("#prop-facility-name", "facilityName", "Post House", "Facility"),
    ("#prop-luminance", "luminance", "48", "Luminance"),
]
LUMINANCE_UNITS_SELECT = "#prop-luminance-units"
# typing this into the select picks the unit after it
LUMINANCE_UNITS_TYPED = "candela"
LUMINANCE_UNITS = "candela-per-square-metre"

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


# one DCP for the whole session, the encode is the slow part of this suite
@pytest.fixture(scope="session")
def two_reel_dcp(tmp_path_factory):
    root = tmp_path_factory.mktemp("two-reel")
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
            "--split-at", FIXTURE_SPLIT_AT,
        ),
        capture_output=True,
        text=True,
        timeout=CREATE_TIMEOUT_SECONDS,
    )
    assert created.returncode == 0, created.stderr[-4000:]
    # create writes the package into <output>/<title>
    cpls = sorted(output.glob("*/CPL_*.xml"))
    assert len(cpls) == 1, cpls
    package = cpls[0].parent
    return TwoReelPackage(package, cpls[0], write_project_beside(package))


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
    window.click("#recent-header")
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
