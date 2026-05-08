"""CLI integration tests for dcpwizard."""

import subprocess
import sys

def run(args):
    result = subprocess.run(
        ["./dcpwizard"] + args,
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
