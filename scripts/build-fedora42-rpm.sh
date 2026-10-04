#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GROK_SOURCE="${1:-}"
CUDA_ARCH="${2:-}"

if [[ -z "$GROK_SOURCE" || ! "$CUDA_ARCH" =~ ^[0-9]+$ ]]; then
    echo "usage: $0 /path/to/grok/source <cuda arch, for example 86>" >&2
    exit 2
fi

GROK_SOURCE="$(cd "$GROK_SOURCE" && pwd)"
if [[ ! -f "$GROK_SOURCE/extern/grok-gpu-plugin/CMakeLists.txt" ]]; then
    echo "$GROK_SOURCE has no extern/grok-gpu-plugin, run git submodule update --init extern/grok-gpu-plugin there" >&2
    exit 1
fi

IMAGE=postperfection-fedora42-build
FEDORA_IMAGE=registry.fedoraproject.org/fedora:42
CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/postperfection/fedora42"
GROK_BUILD="$CACHE_ROOT/grok-build-sm$CUDA_ARCH"
GROK_INSTALL="$CACHE_ROOT/grok-install-sm$CUDA_ARCH"
CARGO_CACHE="$CACHE_ROOT/cargo"
PNPM_STORE="$CACHE_ROOT/pnpm-store"
WORK="$CACHE_ROOT/work-dcpwizard"
OUTPUT_DIRECTORY="$ROOT/gui/src-tauri/target/release/bundle/rpm"
RPM_RELEASE="1.sm$CUDA_ARCH.fc42"

mkdir -p "$GROK_BUILD" "$GROK_INSTALL" "$CARGO_CACHE" "$PNPM_STORE" "$WORK" "$OUTPUT_DIRECTORY"

podman build --tag "$IMAGE" --file "$ROOT/scripts/fedora-rpm.Containerfile" "$ROOT/scripts"

# :z would relabel every file in the grok and wizard trees for selinux
run_in_image() {
    podman run --rm -i --userns=keep-id --security-opt label=disable \
        -e HOME=/tmp -e CUDA_ARCH="$CUDA_ARCH" "$@"
}

run_in_image \
    -v "$GROK_SOURCE:/grok:ro" \
    -v "$GROK_BUILD:/grok-build" \
    -v "$GROK_INSTALL:/grok-install" \
    "$IMAGE" bash -euo pipefail <<'EOF'
cmake -S /grok -B /grok-build -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/grok-install \
    -DBUILD_SHARED_LIBS=ON \
    -DGRK_BUILD_PLUGIN_LOADER=ON \
    -DGRK_CUDA_ARCH="$CUDA_ARCH" \
    -DCMAKE_CUDA_ARCHITECTURES="$CUDA_ARCH" \
    -DGPUP_ENABLE_AUTH=ON \
    -DGPUP_USE_LEGACY_AUTH=OFF \
    -DGRK_BUILD_CORE_SWIG_BINDINGS=OFF \
    -DGRK_BUILD_JPEG=OFF
cmake --build /grok-build
cmake --install /grok-build
EOF

# --delete leaves the container's own target and node_modules in place
rsync -a --delete \
    --exclude .git \
    --exclude target/ \
    --exclude node_modules/ \
    --exclude /gui/dist/ \
    "$ROOT/" "$WORK/"

run_in_image \
    -v "$WORK:/wizard" \
    -v "$GROK_INSTALL:/grok-install:ro" \
    -v "$CARGO_CACHE:/cache/cargo" \
    -v "$PNPM_STORE:/cache/pnpm-store" \
    -e CARGO_HOME=/cache/cargo \
    -e RPM_RELEASE="$RPM_RELEASE" \
    "$IMAGE" bash -euo pipefail <<'EOF'
for candidate in /grok-install/lib64 /grok-install/lib; do
    if [[ -f "$candidate/libgrokj2k.so.1" && -f "$candidate/libgrokj2k_plugin.so" ]]; then
        GROK_LIBRARY_DIRECTORY="$candidate"
        break
    fi
done

if [[ -z "${GROK_LIBRARY_DIRECTORY:-}" ]]; then
    echo "no Grok core library and GPU plugin found under /grok-install" >&2
    exit 1
fi

# these win over the host grok paths in the committed cargo [env] config
export PKG_CONFIG_PATH="$FFMPEG_MPV_DIR/lib/pkgconfig:$GROK_LIBRARY_DIRECTORY/pkgconfig"
export LD_LIBRARY_PATH="$FFMPEG_MPV_DIR/lib:$GROK_LIBRARY_DIRECTORY"

cd /wizard
cargo build --release -p dcpwizard-cli --manifest-path rust/Cargo.toml
scripts/setup-tauri-bin.sh
cp -L "$GROK_LIBRARY_DIRECTORY/libgrokj2k.so.1" gui/src-tauri/libgrokj2k.so.1
cp -L "$GROK_LIBRARY_DIRECTORY/libgrokj2k_plugin.so" gui/src-tauri/libgrokj2k_plugin.so

CUBIN_ARCH="$(cuobjdump --list-elf gui/src-tauri/libgrokj2k_plugin.so | grep -oE 'sm_[0-9]+' | head -1 || true)"
if [[ "$CUBIN_ARCH" != "sm_$CUDA_ARCH" ]]; then
    echo "libgrokj2k_plugin.so carries ${CUBIN_ARCH:-no cubin}, expected sm_$CUDA_ARCH" >&2
    exit 1
fi

FFMPEG_MPV_FILES="$(scripts/ffmpeg-mpv-bundle-config.sh "$FFMPEG_MPV_DIR" Linux)"

cd gui
pnpm install --frozen-lockfile --store-dir /cache/pnpm-store
# the plugin is private, the committed config leaves it out
PLUGIN_FILES='{"bundle":{"linux":{"rpm":{"release":"'"$RPM_RELEASE"'","files":{"/usr/lib/dcpwizard/libgrokj2k.so.1":"libgrokj2k.so.1","/usr/lib/dcpwizard/libgrokj2k_plugin.so":"libgrokj2k_plugin.so"}}}}}'
pnpm tauri build --bundles rpm --config "$FFMPEG_MPV_FILES" --config "$PLUGIN_FILES"
EOF

read_tauri_config() {
    run_in_image -v "$WORK:/wizard:ro" "$IMAGE" jq -r "$1" /wizard/gui/src-tauri/tauri.conf.json
}
PRODUCT_NAME="$(read_tauri_config .productName)"
VERSION="$(read_tauri_config .version)"
BUILT_RPM="$WORK/gui/src-tauri/target/release/bundle/rpm/$PRODUCT_NAME-$VERSION-$RPM_RELEASE.x86_64.rpm"
RPM_NAME="${PRODUCT_NAME// /-}-$VERSION-$RPM_RELEASE.x86_64.rpm"
cp "$BUILT_RPM" "$OUTPUT_DIRECTORY/$RPM_NAME"
echo "wrote $OUTPUT_DIRECTORY/$RPM_NAME"

podman run --rm -i --security-opt label=disable \
    -v "$OUTPUT_DIRECTORY/$RPM_NAME:/rpms/$RPM_NAME:ro" \
    -e RPM_NAME="$RPM_NAME" \
    "$FEDORA_IMAGE" bash -euo pipefail <<'EOF'
# ffmpeg comes from RPM Fusion
dnf install -y "https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm"
dnf install -y binutils "/rpms/$RPM_NAME"
command -v ffmpeg
dcpwizard --version

if [[ "$(ldd /usr/bin/dcpwizard-gui | grep -c "not found" || true)" != 0 ]]; then
    ldd /usr/bin/dcpwizard-gui
    echo "dcpwizard-gui has an unresolved library" >&2
    exit 1
fi

if ldd /usr/lib/dcpwizard/libgrokj2k_plugin.so | grep "not found"; then
    echo "libgrokj2k_plugin.so has an unresolved library" >&2
    exit 1
fi

if ! readelf -d /usr/bin/dcpwizard | grep -F 'RUNPATH' | grep -qF '$ORIGIN/../lib/dcpwizard'; then
    readelf -d /usr/bin/dcpwizard
    echo "dcpwizard has no runpath to /usr/lib/dcpwizard" >&2
    exit 1
fi

GRK_PLUGIN_PATH=/usr/lib/dcpwizard dcpwizard --version
EOF
echo "rpm installs on fedora:42"
