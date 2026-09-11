#!/usr/bin/env python3
# release GUI against a grok build with the plugin, launched so grok finds it
import os
import shutil
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parent
cli_manifest = root / "rust" / "Cargo.toml"
gui_dir = root / "gui"
gui_manifest = gui_dir / "src-tauri" / "Cargo.toml"


def prepend_path(name, directory):
    directory = str(directory)
    current = os.environ.get(name)
    os.environ[name] = directory if not current else directory + os.pathsep + current


def platform_layout():
    if sys.platform.startswith("linux"):
        return {
            "plugin_names": ["libgrokj2k_plugin.so"],
            "lib_dirs": ["lib64", "lib"],
            "loader_var": "LD_LIBRARY_PATH",
            "kernels": None,
        }
    if sys.platform == "darwin":
        return {
            "plugin_names": ["libgrokj2k_plugin.dylib"],
            "lib_dirs": ["lib", "lib64"],
            "loader_var": "DYLD_LIBRARY_PATH",
            "kernels": "grok_kernels.metallib",
        }
    if sys.platform == "win32":
        return {
            "plugin_names": ["grokj2k_plugin.dll", "libgrokj2k_plugin.dll"],
            "lib_dirs": ["bin", "lib", "lib64"],
            "loader_var": "PATH",
            "kernels": None,
        }
    sys.exit(f"no GPU GUI launch for {sys.platform}")


def find_lib_dir(grok_root, layout):
    searched = []
    candidates = [grok_root / name for name in layout["lib_dirs"]]
    candidates.append(grok_root)
    for directory in candidates:
        searched.append(directory)
        if any((directory / plugin).is_file() for plugin in layout["plugin_names"]):
            return directory
    names = " or ".join(layout["plugin_names"])
    places = ", ".join(str(path) for path in searched)
    sys.exit(f"no {names} under {grok_root} (looked in {places})")


def pkgconfig_dir(grok_root, lib_dir):
    for directory in (
        lib_dir / "pkgconfig",
        grok_root / "lib" / "pkgconfig",
        grok_root / "lib64" / "pkgconfig",
    ):
        if directory.is_dir():
            return directory
    return None


def host_triple():
    for line in subprocess.check_output(["rustc", "-vV"], text=True).splitlines():
        if line.startswith("host:"):
            return line.split()[1]
    sys.exit("rustc did not print a host triple")


def run(*command, cwd=root):
    subprocess.run(command, cwd=cwd, check=True)


def install_sidecar():
    exe = ".exe" if sys.platform == "win32" else ""
    src = root / "rust" / "target" / "release" / f"dcpwizard{exe}"
    dest_dir = gui_dir / "src-tauri"
    dest_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest_dir / f"dcpwizard-{host_triple()}{exe}")


if len(sys.argv) < 2:
    sys.exit("usage: run-gpu-gui.py <grok install root> [gui args]")

layout = platform_layout()
grok_root = Path(sys.argv[1]).expanduser()
lib_dir = find_lib_dir(grok_root, layout)
if layout["kernels"] and not (lib_dir / layout["kernels"]).is_file():
    sys.exit(f"no {layout['kernels']} in {lib_dir}")

pkgconfig = pkgconfig_dir(grok_root, lib_dir)
if pkgconfig:
    prepend_path("PKG_CONFIG_PATH", pkgconfig)
prepend_path(layout["loader_var"], lib_dir)
os.environ["GRK_PLUGIN_PATH"] = str(lib_dir)

# grokj2k-sys caches the grok it last linked, so a CPU-only build would be kept
run("cargo", "clean", "-q", "-p", "grokj2k-sys", "--manifest-path", cli_manifest)
run("cargo", "build", "-q", "--release", "-p", "dcpwizard-cli", "--manifest-path", cli_manifest)
if sys.platform == "win32":
    install_sidecar()
else:
    run(root / "scripts" / "setup-tauri-bin.sh")
run("cargo", "clean", "-q", "-p", "grokj2k-sys", "--manifest-path", gui_manifest)
run("pnpm", "tauri", "build", "--no-bundle", "--ignore-version-mismatches", cwd=gui_dir)

gui_bin = gui_dir / "src-tauri" / "target" / "release" / (
    "dcpwizard-gui.exe" if sys.platform == "win32" else "dcpwizard-gui"
)
os.execv(gui_bin, [str(gui_bin), *sys.argv[2:]])
