"""DCP Wizard — Python bindings for creating Digital Cinema Packages.

Provides a subprocess-based API that wraps the dcpwizard CLI.
Requires the `dcpwizard` binary to be on PATH or specified via DCPWIZARD_BIN.
"""

import json
import os
import shutil
import subprocess
from pathlib import Path

__version__ = "1.0.0"


def _find_binary():
    """Locate the dcpwizard binary."""
    env_bin = os.environ.get("DCPWIZARD_BIN")
    if env_bin and os.path.isfile(env_bin):
        return env_bin
    found = shutil.which("dcpwizard")
    if found:
        return found
    raise FileNotFoundError(
        "dcpwizard binary not found. Set DCPWIZARD_BIN or add it to PATH."
    )


def _run(args, check=True):
    """Run dcpwizard with given arguments."""
    bin_path = _find_binary()
    result = subprocess.run(
        [bin_path] + args,
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        raise RuntimeError(
            f"dcpwizard failed (exit {result.returncode}): {result.stderr}"
        )
    return result


def create(title, video, output, audio=None, subtitle=None, encoder="grok"):
    """Create a DCP from source media."""
    args = ["create", "--title", title, "--video", str(video), "--output", str(output)]
    if audio:
        args.extend(["--audio", str(audio)])
    if subtitle:
        args.extend(["--subtitle", str(subtitle)])
    args.extend(["--encoder", encoder])
    _run(args)
    return Path(output)


def verify(dcp_dir, strict=False):
    """Verify a DCP. Returns dict with valid/errors/warnings."""
    args = ["verify", str(dcp_dir), "--json"]
    if strict:
        args.append("--strict")
    result = _run(args, check=False)
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"valid": result.returncode == 0, "errors": [], "warnings": []}


def encode(input_dir, output_dir, bandwidth=250, encoder="grok"):
    """Encode image sequence to JPEG 2000."""
    _run(["encode", "--input", str(input_dir), "--output", str(output_dir),
          "--bandwidth", str(bandwidth), "--encoder", encoder])
    return Path(output_dir)


def kdm(cpl_id, content_title, cert, output, valid_from="now", valid_duration="2 weeks"):
    """Generate a KDM."""
    _run(["kdm", "--cpl-id", cpl_id, "--content-title", content_title,
          "--cert", str(cert), "--output", str(output),
          "--valid-from", valid_from, "--valid-duration", valid_duration])
    return Path(output)


def copy(src, dst):
    """Copy DCP to destination with hash verification."""
    _run(["copy", "--src", str(src), "--dst", str(dst)])
    return Path(dst)


def loudness(audio_path):
    """Measure audio loudness (EBU R128)."""
    result = _run(["loudness", str(audio_path)])
    return result.stdout


def subtitle_convert(input_path, output_path, language="en"):
    """Convert SRT to SMPTE DCP XML subtitles."""
    _run(["subtitle-convert", "--input", str(input_path),
          "--output", str(output_path), "--language", language])
    return Path(output_path)

