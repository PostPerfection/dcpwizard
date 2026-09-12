import os
import subprocess
import xml.etree.ElementTree as ElementTree
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
# a click or a key press is answered in the page, not over the network
REACTION_TIMEOUT_SECONDS = 15
CREATE_TIMEOUT_SECONDS = 900

REELS_CHORD = "ctrl+2"
REELS_VIEW = "view-reels"

PREVIEW_DEFAULT_TITLE = "Preview"

FIXTURE_TITLE = "Two Reel Test"
FIXTURE_SIZE = "1998x1080"
FIXTURE_FPS = 24
FIXTURE_SECONDS = 4
FIXTURE_SPLIT_AT = "00:00:02"
FIXTURE_CHANNELS = 6

# crossing the first reel takes longer than the reel lasts when decoding lags
REEL_CROSSING_TIMEOUT_MULTIPLE = 3

# the tick to seek to, as how far along the ruler it has to sit at least
SEEK_TICK_LEFT_PERCENT = 20
SEEK_TICK_ID = "test-seek-tick"

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
    def __init__(self, directory, cpl_path):
        self.directory = directory
        self.cpl_path = cpl_path
        self.reels = reel_pictures(cpl_path)


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
    cpls = sorted(output.glob("CPL_*.xml"))
    assert len(cpls) == 1, cpls
    return TwoReelPackage(output, cpls[0])


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


def preview_title(session):
    return session.property("#preview-title", "textContent")


# what the scrubber says the player holds, in seconds
def reported_duration(session):
    return float(session.attribute("#timeline-duration", "data-raw"))


@pytest.fixture
def window(tmp_path):
    opened = open_window(application_environment(tmp_path), tmp_path / "driver.log")
    yield opened
    opened.close()


def test_the_reels_view_lists_the_reels_and_follows_playback(window, two_reel_dcp):
    session = window.session
    reels = two_reel_dcp.reels
    assert len(reels) == 2, reels

    choose_in_dialog(window, "#btn-open-project", two_reel_dcp.directory)
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
