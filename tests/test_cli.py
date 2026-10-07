"""CLI integration tests for dcpwizard."""

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


def run(args):
    result = subprocess.run(
        [_find_exe()] + args,
        capture_output=True, text=True, timeout=30
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

def test_create_takes_a_picture_scale_and_offset():
    r = run(["create", "--help"])
    assert "--picture-scale" in r.stdout
    assert "--picture-offset" in r.stdout

    r = run(["create", "--picture-scale", "500", "--picture-offset", "10,10"])
    assert r.returncode != 0
    assert "outside 25% to 400%" in r.stderr

    r = run(["create", "--picture-scale", "50", "--picture-offset", "10"])
    assert r.returncode != 0
    assert "'10' is not X,Y" in r.stderr

    # both values parse, so what is refused is the missing input
    r = run(["create", "--picture-scale", "87.5", "--picture-offset", "-20,10"])
    assert r.returncode != 0
    assert "invalid value" not in r.stderr
    assert "required arguments were not provided" in r.stderr

if __name__ == "__main__":
    test_help()
    test_version_flag()
    test_create_missing_args()
    test_create_takes_a_picture_scale_and_offset()
    print("All CLI tests passed")
    sys.exit(0)
