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

if __name__ == "__main__":
    test_help()
    test_version_flag()
    test_create_missing_args()
    print("All CLI tests passed")
    sys.exit(0)
