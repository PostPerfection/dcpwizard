import base64
import os
import sys
import tempfile
import time
from pathlib import Path

from tauri_webdriver import CENTRE_OF_ELEMENT, INPUT_SETTLE_SECONDS, Window, wait_until, xdotool
from test_gui_webdriver import (
    CLIP_TITLES,
    FIXTURE_SIZE,
    NEUTRAL_TARGET,
    NEW_PROJECT_CHORD,
    NEXT_FIELD_KEY,
    OPEN_TIMEOUT_SECONDS,
    PAGE_TIMEOUT_SECONDS,
    PROJECT_WIZARD,
    REACTION_TIMEOUT_SECONDS,
    RECENT_LIST_STORED,
    REPOSITORY_ROOT,
    TIMELINE_CLIP,
    TIMELINE_IN_FIELD,
    TIMELINE_SOUND_LABEL,
    WINDOW_TITLE,
    application_environment,
    choose_in_dialog,
    gui_binary,
    replace_field_text,
    save_in_dialog_by_chord,
    wait_for_status,
    write_picture,
    write_sound,
)

MANUAL_DIRECTORY = REPOSITORY_ROOT / "docs/manual"
DEVICE_SCALE = 2
WINDOW_CSS_SIZE = (1200, 860)
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"

PROJECT_TITLE = "My Film"
MOVIE_SECONDS = 4
TRAILER_SECONDS = 3
TRAILER_IN = "00:00:01:00"
SOUND_SECONDS = 6


class ScaledWindow(Window):
    # xdotool takes device pixels and the page reports css pixels
    def click(self, css):
        spot = self.session.execute(CENTRE_OF_ELEMENT, css)
        if spot is None:
            raise AssertionError(f"{css} has no box on the page to click")
        x, y = (self.origin[axis] + round(spot[axis] * DEVICE_SCALE) for axis in (0, 1))
        xdotool("mousemove", str(x), str(y), "click", "1")
        time.sleep(INPUT_SETTLE_SECONDS)


def viewport_size_and_scale(session):
    return session.execute("return [window.innerWidth, window.innerHeight, window.devicePixelRatio]")


def open_scaled_window(root):
    environment = application_environment(root)
    environment["GDK_SCALE"] = str(DEVICE_SCALE)
    window = ScaledWindow(gui_binary(), environment, root / "driver.log", WINDOW_TITLE)
    try:
        wait_until(
            "the page never loaded",
            lambda: window.session.find("#theme-toggle"),
            PAGE_TIMEOUT_SECONDS,
        )
        window.take_focus(NEUTRAL_TARGET)
        width, height = (side * DEVICE_SCALE for side in WINDOW_CSS_SIZE)
        xdotool("windowsize", "--sync", window.window_id, str(width), str(height))
        wait_until(
            "the window never took the manual's size",
            lambda: viewport_size_and_scale(window.session) == [*WINDOW_CSS_SIZE, DEVICE_SCALE],
            REACTION_TIMEOUT_SECONDS,
        )
    except Exception:
        print(window.output(), file=sys.stderr)
        window.close()
        raise
    return window


def stage_project_view(window, root):
    session = window.session
    movie = write_picture(root / "movie.mov", FIXTURE_SIZE, MOVIE_SECONDS)
    trailer = write_picture(root / "trailer.mov", FIXTURE_SIZE, TRAILER_SECONDS)
    sound = write_sound(root / "mix.wav", SOUND_SECONDS)
    project_path = root / f"{PROJECT_TITLE}.{PROJECT_WIZARD}"

    wait_until(
        "the project file handling never started",
        lambda: session.execute(RECENT_LIST_STORED),
        PAGE_TIMEOUT_SECONDS,
    )
    save_in_dialog_by_chord(window, NEW_PROJECT_CHORD, project_path)
    wait_for_status(session, f"Saved {project_path}", REACTION_TIMEOUT_SECONDS)
    choose_in_dialog(window, "#import-video", movie)
    choose_in_dialog(window, "#import-video", trailer)
    choose_in_dialog(window, "#import-audio", sound)
    wait_until(
        "the two clips never reached the timeline",
        lambda: len(session.execute(CLIP_TITLES)) == 2,
        OPEN_TIMEOUT_SECONDS,
    )
    wait_until(
        "the sound never reached the timeline",
        lambda: session.find(TIMELINE_SOUND_LABEL),
        OPEN_TIMEOUT_SECONDS,
    )
    window.click(TIMELINE_CLIP.format(1))
    replace_field_text(window, TIMELINE_IN_FIELD, TRAILER_IN)
    window.press(NEXT_FIELD_KEY)
    wait_until(
        "the trailer never took its In point",
        lambda: TRAILER_IN in session.execute(CLIP_TITLES)[1],
        REACTION_TIMEOUT_SECONDS,
    )


MANUAL_SHOTS = {"project.png": stage_project_view}


def save_screenshot(session, path):
    png = base64.b64decode(session.call("GET", "/screenshot"))
    assert png.startswith(PNG_SIGNATURE), "the driver returned no PNG"
    path.write_bytes(png)


def main():
    assert os.environ.get("DISPLAY"), "no DISPLAY: run under xvfb-run"
    for name, stage in MANUAL_SHOTS.items():
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            window = open_scaled_window(root)
            try:
                stage(window, root)
                save_screenshot(window.session, MANUAL_DIRECTORY / name)
            except Exception:
                print(window.output(), file=sys.stderr)
                raise
            finally:
                window.close()
        print(MANUAL_DIRECTORY / name)


if __name__ == "__main__":
    main()
