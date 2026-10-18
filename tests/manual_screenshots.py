import base64
import os
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from tauri_webdriver import (
    CENTRE_OF_ELEMENT,
    INPUT_SETTLE_SECONDS,
    Window,
    visible_windows,
    wait_until,
    xdotool,
)
from test_gui_webdriver import (
    CLIP_TITLES,
    CREATE_TIMEOUT_SECONDS,
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
    STATUS_TIMEOUT_SECONDS,
    TEXT_DIALOG_OK,
    TIMELINE_CLIP,
    TIMELINE_IN_FIELD,
    TIMELINE_SOUND_LABEL,
    WINDOW_TITLE,
    application_environment,
    choose_in_dialog,
    cli_binary,
    gui_binary,
    replace_field_text,
    save_in_dialog,
    save_in_dialog_by_chord,
    text_dialog_open,
    wait_for_status,
    wait_for_view,
    write_picture,
    write_sound,
)

MANUAL_DIRECTORY = REPOSITORY_ROOT / "docs/manual"
DEVICE_SCALE = 2
WINDOW_CSS_SIZE = (1200, 860)
# both KDM panels and the hints above them fit without scrolling at this height
KDM_WINDOW_CSS_SIZE = (1200, 1025)
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"

PROJECT_TITLE = "My Film"
MOVIE_SECONDS = 4
TRAILER_SECONDS = 3
TRAILER_IN = "00:00:01:00"
SOUND_SECONDS = 6

# the settings button sits below the bottom of the screen in the taller KDM window
SETTINGS_CHORD = "ctrl+9"
SECURITY_VIEW_BUTTON = '.sidebar-btn[data-view="security"]'
CERTIFICATES_FIELDSET = "fieldset:has(#set-signing-cert)"
OPTIONS_FIELDSET = "fieldset:has(#prop-encrypt)"
SIGNING_ORGANIZATION = "Studio"
SCREEN_NAME = "screen-1"
KEYS_FILE_NAME = "my-film.keys.json"
DKDM_FILE_NAME = "my-film.dkdm.xml"
DKDM_VALID_TO = "2 weeks"
ENCRYPTED_PACKAGE_SECONDS = 1
# file names the app adds to the folder picked for each KDM output
MAKE_KDM_OUTPUT_NAME = "dkdm.xml"
REWRAP_OUTPUT_NAME = "kdm.xml"

SCROLL_INTO_VIEW = "document.querySelector(arguments[0]).scrollIntoView({ block: arguments[1] })"
VIEW_OVERFLOW = """
const view = document.querySelector(arguments[0])
return view.scrollHeight - view.clientHeight
"""


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


def open_scaled_window(root, css_size):
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
        width, height = (side * DEVICE_SCALE for side in css_size)
        xdotool("windowsize", "--sync", window.window_id, str(width), str(height))
        wait_until(
            "the window never took the manual's size",
            lambda: viewport_size_and_scale(window.session) == [*css_size, DEVICE_SCALE],
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


def wait_for_field(session, css, value):
    wait_until(
        f"{css} never read {value!r}",
        lambda: session.property(css, "value") == value,
        REACTION_TIMEOUT_SECONDS,
    )


def choose_into_field(window, button, path, field, value=None):
    choose_in_dialog(window, button, path)
    wait_for_field(window.session, field, value or str(path))


# the folder picker opens only once the name asked for is confirmed
def create_in_settings(window, button, typed_name, directory):
    session = window.session
    window.click(button)
    wait_until(
        "the name was never asked for",
        lambda: text_dialog_open(session) is True,
        REACTION_TIMEOUT_SECONDS,
    )
    if typed_name:
        window.type_text(typed_name)
    windows_before = visible_windows()
    window.click(TEXT_DIALOG_OK)
    window.answer_file_dialog(directory, windows_before)


def fill_certificate_settings(window, root):
    session = window.session
    certificates = root / "dcp-certs"
    certificates.mkdir()
    window.press(SETTINGS_CHORD)
    wait_for_view(session, "view-settings")
    create_in_settings(window, "#set-create-signing-chain", SIGNING_ORGANIZATION, certificates)
    wait_for_status(session, f"Created a signing chain in {certificates}", STATUS_TIMEOUT_SECONDS)
    create_in_settings(window, "#set-create-recipient", None, certificates)
    wait_for_status(
        session, f"Created a recipient certificate in {certificates}", STATUS_TIMEOUT_SECONDS
    )
    return certificates


def stage_settings_view(window, root):
    fill_certificate_settings(window, root)
    # drops the focus ring the name dialog leaves on the button that opened it
    window.click(NEUTRAL_TARGET)
    window.session.execute(SCROLL_INTO_VIEW, CERTIFICATES_FIELDSET, "start")


def stage_encrypt_options(window, root):
    stage_project_view(window, root)
    keys_directory = root / "keys"
    keys_directory.mkdir()
    window.click("#prop-encrypt")
    save_in_dialog(window, "#prop-browse-key-out", keys_directory / KEYS_FILE_NAME)
    wait_for_field(window.session, "#prop-key-out", str(keys_directory / KEYS_FILE_NAME))
    assert window.session.property("#prop-encrypt", "checked") is True
    window.session.execute(SCROLL_INTO_VIEW, OPTIONS_FIELDSET, "center")


def run_cli(environment, *arguments):
    done = subprocess.run(
        (str(cli_binary()), *arguments),
        env=environment,
        capture_output=True,
        text=True,
        timeout=CREATE_TIMEOUT_SECONDS,
    )
    assert done.returncode == 0, (done.stdout + done.stderr)[-4000:]


def signer_arguments(certificates):
    return (
        "--signer-cert", str(certificates / "signer.pem"),
        "--signer-key", str(certificates / "signer.key"),
        "--signer-chain", str(certificates / "intermediate.pem"),
        "--signer-chain", str(certificates / "root.pem"),
    )


def create_encrypted_package(environment, root, certificates, keys_file):
    picture = write_picture(root / "movie.mov", FIXTURE_SIZE, ENCRYPTED_PACKAGE_SECONDS)
    sound = write_sound(root / "mix.wav", ENCRYPTED_PACKAGE_SECONDS)
    output = root / "dcp"
    run_cli(
        environment,
        "create",
        "--title", PROJECT_TITLE,
        "--video", str(picture),
        "--audio", str(sound),
        "--output", str(output),
        "--encrypt",
        "--key-out", str(keys_file),
        *signer_arguments(certificates),
    )
    [cpl] = output.glob("*/CPL_*.xml")
    return cpl


def write_screen_certificate(environment, root, certificates):
    screens = root / "screens"
    screens.mkdir()
    certificate = screens / f"{SCREEN_NAME}.pem"
    run_cli(
        environment,
        "certificate", "generate", "-t", "leaf",
        "--cn", SCREEN_NAME,
        "--issuer-cert", str(certificates / "intermediate.pem"),
        "--issuer-key", str(certificates / "intermediate.key"),
        "--output-cert", str(certificate),
        "--output-key", str(screens / f"{SCREEN_NAME}.key"),
    )
    return certificate


# a KDM may not start on the day its signer certificate starts
def start_of_tomorrow():
    tomorrow = datetime.now(timezone.utc) + timedelta(days=1)
    return tomorrow.replace(hour=0, minute=0, second=0, microsecond=0).isoformat()


def write_dkdm(environment, cpl, certificates, keys_file, dkdm):
    cpl_id = cpl.stem.removeprefix("CPL_")
    run_cli(
        environment,
        "kdm",
        "--cpl-id", cpl_id,
        "--content-title", PROJECT_TITLE,
        "--cert", str(certificates / "recipient.pem"),
        *signer_arguments(certificates),
        "--keys", str(keys_file),
        "--valid-from", start_of_tomorrow(),
        "--valid-to", DKDM_VALID_TO,
        "--output", str(dkdm),
    )


def stage_kdm_view(window, root):
    session = window.session
    certificates = fill_certificate_settings(window, root)
    environment = application_environment(root)
    keys_directory = root / "keys"
    keys_directory.mkdir()
    keys_file = keys_directory / KEYS_FILE_NAME
    cpl = create_encrypted_package(environment, root, certificates, keys_file)
    screen_certificate = write_screen_certificate(environment, root, certificates)
    kdms = root / "kdms"
    kdms.mkdir()
    dkdm = kdms / DKDM_FILE_NAME
    write_dkdm(environment, cpl, certificates, keys_file, dkdm)

    window.click(SECURITY_VIEW_BUTTON)
    wait_for_view(session, "view-security")
    choose_into_field(window, "#kdm-browse-dcp", cpl.parent, "#kdm-dcp")
    wait_for_field(session, "#kdm-content-title", PROJECT_TITLE)
    assert session.property("#kdm-cpl-id", "value"), "the CPL ID never filled in"
    window.click("#kdm-for-this-machine")
    wait_for_field(session, "#kdm-cert", str(certificates / "recipient.pem"))
    choose_into_field(window, "#kdm-browse-keys", keys_file, "#kdm-keys")
    choose_into_field(
        window, "#kdm-browse-output", kdms, "#kdm-output", str(kdms / MAKE_KDM_OUTPUT_NAME)
    )

    choose_into_field(window, "#dkdm-browse", dkdm, "#dkdm-file")
    window.click("#dkdm-use-own-key")
    wait_for_field(session, "#dkdm-key", str(certificates / "recipient.key"))
    choose_into_field(window, "#dkdm-browse-cert", screen_certificate, "#dkdm-cert")
    choose_into_field(
        window, "#dkdm-browse-output", kdms, "#dkdm-output", str(kdms / REWRAP_OUTPUT_NAME)
    )
    for date in ("#kdm-from", "#kdm-to", "#dkdm-from", "#dkdm-to"):
        assert session.property(date, "value") == "", f"{date} is not empty"
    overflow = session.execute(VIEW_OVERFLOW, "#view-security")
    assert overflow <= 0, f"the KDM view needs {overflow} more css pixels of window height"


MANUAL_SHOTS = {
    "project.png": (stage_project_view, WINDOW_CSS_SIZE),
    "settings.png": (stage_settings_view, WINDOW_CSS_SIZE),
    "encrypt.png": (stage_encrypt_options, WINDOW_CSS_SIZE),
    "kdm.png": (stage_kdm_view, KDM_WINDOW_CSS_SIZE),
}


def save_screenshot(session, path):
    png = base64.b64decode(session.call("GET", "/screenshot"))
    assert png.startswith(PNG_SIGNATURE), "the driver returned no PNG"
    path.write_bytes(png)


def main():
    assert os.environ.get("DISPLAY"), "no DISPLAY: run under xvfb-run"
    for name, (stage, css_size) in MANUAL_SHOTS.items():
        # WebKit's helper processes can still be writing here after the app is killed
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as directory:
            root = Path(directory)
            window = open_scaled_window(root, css_size)
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
