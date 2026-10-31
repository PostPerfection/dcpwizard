#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GROK_SOURCE="${1:-}"
CUDA_ARCH="${2:-}"

if [[ -z "$GROK_SOURCE" || ! "$CUDA_ARCH" =~ ^[0-9]+$ ]]; then
    echo "usage: [UBUNTU_VERSION=26.04] GPUP_KERNEL_KEY_FILE=<hex key file> $0 /path/to/grok/source <cuda arch, for example 86>" >&2
    exit 2
fi

if [[ ! -f "${GPUP_KERNEL_KEY_FILE:-}" ]]; then
    echo "set GPUP_KERNEL_KEY_FILE to the file holding the key the plugin's encode kernels are encrypted with" >&2
    exit 2
fi

UBUNTU_VERSION="${UBUNTU_VERSION:-24.04}"
# NVIDIA's ubuntu2604 repo starts at CUDA 13.3
case "$UBUNTU_VERSION" in
    24.04) CUDA_IMAGE=docker.io/nvidia/cuda:13.2.1-devel-ubuntu24.04 ;;
    26.04) CUDA_IMAGE=docker.io/nvidia/cuda:13.4.3-devel-ubuntu26.04 ;;
    *) echo "no CUDA image for Ubuntu $UBUNTU_VERSION, use 24.04 or 26.04" >&2; exit 2 ;;
esac

GROK_SOURCE="$(cd "$GROK_SOURCE" && pwd)"
if [[ ! -f "$GROK_SOURCE/extern/grok-gpu-plugin/CMakeLists.txt" ]]; then
    echo "$GROK_SOURCE has no extern/grok-gpu-plugin, run git submodule update --init extern/grok-gpu-plugin there" >&2
    exit 1
fi

IMAGE="postperfection-ubuntu${UBUNTU_VERSION%%.*}-build"
UBUNTU_IMAGE="docker.io/library/ubuntu:$UBUNTU_VERSION"
CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/postperfection/ubuntu${UBUNTU_VERSION%%.*}"
GROK_BUILD="$CACHE_ROOT/grok-build-sm$CUDA_ARCH"
GROK_INSTALL="$CACHE_ROOT/grok-install-sm$CUDA_ARCH"
CARGO_CACHE="$CACHE_ROOT/cargo"
PNPM_STORE="$CACHE_ROOT/pnpm-store"
WORK="$CACHE_ROOT/work-dcpwizard"
OUTPUT_DIRECTORY="$ROOT/gui/src-tauri/target/release/bundle/deb"

mkdir -p "$GROK_BUILD" "$GROK_INSTALL" "$CARGO_CACHE" "$PNPM_STORE" "$WORK" "$OUTPUT_DIRECTORY"

podman build --tag "$IMAGE" --build-arg CUDA_IMAGE="$CUDA_IMAGE" --file "$ROOT/scripts/ubuntu-deb.Containerfile" "$ROOT/scripts"

# :z would relabel every file in the grok and wizard trees for selinux
run_in_image() {
    podman run --rm -i --userns=keep-id --security-opt label=disable \
        -e HOME=/tmp -e CUDA_ARCH="$CUDA_ARCH" "$@"
}

run_in_image \
    -v "$GROK_SOURCE:/grok:ro" \
    -v "$GROK_BUILD:/grok-build" \
    -v "$GROK_INSTALL:/grok-install" \
    -v "$GPUP_KERNEL_KEY_FILE:/kernel-key.hex:ro" \
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
    -DGPUP_ALLOW_DEV_LICENSE_KEY=OFF \
    -DGPUP_KERNEL_KEY_FILE=/kernel-key.hex \
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
# without it ffmpeg-sys-next can link the system FFmpeg
export FFMPEG_DIR="$FFMPEG_MPV_DIR"

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

cd gui
pnpm install --frozen-lockfile --store-dir /cache/pnpm-store
# --config replaces arrays instead of appending
DEB_CONFIG="$(../scripts/ffmpeg-mpv-bundle-config.sh "$FFMPEG_MPV_DIR" Linux | jq -c '{bundle: {linux: {deb: {
    depends: (.bundle.linux.deb.depends + ["libtiff6", "libcurl4t64"]),
    files: (.bundle.linux.deb.files + {
        "/usr/lib/dcpwizard/libgrokj2k.so.1": "libgrokj2k.so.1",
        "/usr/lib/dcpwizard/libgrokj2k_plugin.so": "libgrokj2k_plugin.so"
    })
}}}}')"
pnpm tauri build --bundles deb --config "$DEB_CONFIG"
EOF

read_tauri_config() {
    run_in_image -v "$WORK:/wizard:ro" "$IMAGE" jq -r "$1" /wizard/gui/src-tauri/tauri.conf.json
}
PRODUCT_NAME="$(read_tauri_config .productName)"
VERSION="$(read_tauri_config .version)"
BUILT_DEB="$WORK/gui/src-tauri/target/release/bundle/deb/${PRODUCT_NAME}_${VERSION}_amd64.deb"
DEB_NAME="${PRODUCT_NAME// /-}_${VERSION}-sm${CUDA_ARCH}.ubuntu${UBUNTU_VERSION}_amd64.deb"
cp "$BUILT_DEB" "$OUTPUT_DIRECTORY/$DEB_NAME"
# the image has dpkg-deb, the host may not
run_in_image \
    -v "$ROOT/scripts:/scripts:ro" \
    -v "$OUTPUT_DIRECTORY/$DEB_NAME:/debs/$DEB_NAME:ro" \
    "$IMAGE" /scripts/check-linux-package-payload.sh "/debs/$DEB_NAME"
echo "wrote $OUTPUT_DIRECTORY/$DEB_NAME"

podman run --rm -i --security-opt label=disable \
    -v "$OUTPUT_DIRECTORY/$DEB_NAME:/debs/$DEB_NAME:ro" \
    -e DEB_NAME="$DEB_NAME" \
    -e DEBIAN_FRONTEND=noninteractive \
    "$UBUNTU_IMAGE" bash -euo pipefail <<'EOF'
apt-get update
apt-get install -y --no-install-recommends "/debs/$DEB_NAME"
command -v ffmpeg
dcpwizard --version

if [[ "$(ldd /usr/bin/dcpwizard-gui | grep -c "not found" || true)" != 0 ]]; then
    ldd /usr/bin/dcpwizard-gui
    echo "dcpwizard-gui has an unresolved library" >&2
    exit 1
fi

# libcuda.so.1 comes with the NVIDIA driver, which the container does not have
if ldd /usr/lib/dcpwizard/libgrokj2k_plugin.so | grep "not found" | grep -v "^\s*libcuda\.so\.1 "; then
    echo "libgrokj2k_plugin.so has an unresolved library" >&2
    exit 1
fi

GRK_PLUGIN_PATH=/usr/lib/dcpwizard dcpwizard --version
EOF
echo "deb installs on ubuntu:$UBUNTU_VERSION"
