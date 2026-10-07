"""CLI integration tests for dcpwizard."""

import json
import os
import subprocess
import sys


def _find_exe():
    """Locate the dcpwizard executable in the build tree."""
    name = "dcpwizard.exe" if os.name == "nt" else "dcpwizard"
    # Multi-config generators (Visual Studio) put binaries under Release/Debug
    for subdir in [".", "Release", "Debug", "RelWithDebInfo", "MinSizeRel"]:
        candidate = os.path.join(subdir, name)
        if os.path.isfile(candidate):
            return candidate
    # Fallback — let the OS resolve it
    return name


def run(args, env=None):
    result = subprocess.run(
        [_find_exe()] + args,
        capture_output=True, text=True, timeout=30, env=env
    )
    return result

def test_help():
    r = run(["--help"])
    assert r.returncode == 0
    assert "DCP Wizard" in r.stdout

def test_version_flag():
    r = run(["--help"])
    assert "create" in r.stdout
    assert "verify" in r.stdout
    assert "encode" in r.stdout

def test_create_missing_args():
    r = run(["create"])
    assert r.returncode != 0

PRESET_NAME = "Festival"
PRESETS_FILE_VERSION = 1
# create refuses a PSNR target over 80 dB by name, so the value shows where it came from
REFUSED_QUALITY_PSNR = "99"
QUALITY_PSNR_REFUSAL = f"--quality-psnr {REFUSED_QUALITY_PSNR} is outside the range"


def preset_environment(config_home, form):
    presets_folder = config_home / "dcpwizard"
    presets_folder.mkdir(parents=True)
    presets = {
        "version": PRESETS_FILE_VERSION,
        "presets": [{"name": PRESET_NAME, "form": form, "audioMap": None}],
    }
    (presets_folder / "presets.json").write_text(json.dumps(presets))
    return dict(os.environ, XDG_CONFIG_HOME=str(config_home))


# the pre-build check stops before anything is encoded
def checked_create(tmp_path, *arguments):
    return ["create", "--check", "--title", "Film", "--video", str(tmp_path / "missing.mov"),
            "--output", str(tmp_path / "out"), *arguments]


def test_create_takes_a_saved_preset_value(tmp_path):
    env = preset_environment(tmp_path / "config", {"qualityPsnr": REFUSED_QUALITY_PSNR})

    r = run(checked_create(tmp_path, "--preset", PRESET_NAME), env)

    assert r.returncode != 0
    assert QUALITY_PSNR_REFUSAL in r.stdout + r.stderr


def test_a_flag_given_wins_over_the_preset(tmp_path):
    env = preset_environment(tmp_path / "config", {"qualityPsnr": REFUSED_QUALITY_PSNR})

    r = run(checked_create(tmp_path, "--preset", PRESET_NAME, "--quality-psnr", "40"), env)

    assert r.returncode == 0, r.stdout + r.stderr
    assert "Pre-build check passed" in r.stdout


def test_a_preset_key_create_has_no_option_for_is_named_on_stderr(tmp_path):
    env = preset_environment(tmp_path / "config", {"someLaterField": "3"})

    r = run(checked_create(tmp_path, "--preset", PRESET_NAME), env)

    assert f"warning: preset {PRESET_NAME}: someLaterField has no create option" in r.stderr


def test_an_unknown_preset_lists_the_saved_ones(tmp_path):
    env = preset_environment(tmp_path / "config", {})

    r = run(checked_create(tmp_path, "--preset", "Other"), env)

    assert r.returncode != 0
    assert f"unknown preset 'Other'. Saved: {PRESET_NAME}" in r.stderr


def test_a_preset_and_a_profile_are_refused_together(tmp_path):
    env = preset_environment(tmp_path / "config", {})

    r = run(checked_create(tmp_path, "--preset", PRESET_NAME, "--profile", "cinema_2k"), env)

    assert r.returncode != 0
    assert "cannot be used with" in r.stderr


if __name__ == "__main__":
    test_help()
    test_version_flag()
    test_create_missing_args()
    print("All CLI tests passed")
    sys.exit(0)
