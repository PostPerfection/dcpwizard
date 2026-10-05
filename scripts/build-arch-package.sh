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

IMAGE=postperfection-arch-build
ARCH_IMAGE=docker.io/library/archlinux:base
CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/postperfection/arch"
GROK_BUILD="$CACHE_ROOT/grok-build-sm$CUDA_ARCH"
GROK_INSTALL="$CACHE_ROOT/grok-install-sm$CUDA_ARCH"
CARGO_CACHE="$CACHE_ROOT/cargo"
PNPM_STORE="$CACHE_ROOT/pnpm-store"
WORK="$CACHE_ROOT/work-dcpwizard"
OUTPUT_DIRECTORY="$ROOT/gui/src-tauri/target/release/bundle/arch"

mkdir -p "$GROK_BUILD" "$GROK_INSTALL" "$CARGO_CACHE" "$PNPM_STORE" "$WORK" "$OUTPUT_DIRECTORY"

podman build --tag "$IMAGE" --file "$ROOT/scripts/arch-package.Containerfile" "$ROOT/scripts"

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

cd gui
pnpm install --frozen-lockfile --store-dir /cache/pnpm-store
# tauri has no pacman bundle
DEB_CONFIG="$(../scripts/ffmpeg-mpv-bundle-config.sh "$FFMPEG_MPV_DIR" Linux | jq -c '{bundle: {linux: {deb: {
    files: (.bundle.linux.deb.files + {
        "/usr/lib/dcpwizard/libgrokj2k.so.1": "libgrokj2k.so.1",
        "/usr/lib/dcpwizard/libgrokj2k_plugin.so": "libgrokj2k_plugin.so"
    })
}}}}')"
pnpm tauri build --bundles deb --config "$DEB_CONFIG"

PRODUCT_NAME="$(jq -r .productName src-tauri/tauri.conf.json)"
VERSION="$(jq -r .version src-tauri/tauri.conf.json)"
BUILT_DEB="src-tauri/target/release/bundle/deb/${PRODUCT_NAME}_${VERSION}_amd64.deb"
PACKAGE_DIRECTORY=/wizard/arch-package
STAGE="$PACKAGE_DIRECTORY/root"
rm -rf "$PACKAGE_DIRECTORY"
mkdir -p "$STAGE"
bsdtar -xOf "$BUILT_DEB" 'data.tar.*' | bsdtar -xf - -C "$STAGE"

BUNDLED_SONAMES="$(find "$STAGE/usr/lib/dcpwizard" -maxdepth 1 -name '*.so*' -printf '%f\n')"
NEEDED_SONAMES="$(find "$STAGE" -type f -exec sh -c 'readelf -h "$1" >/dev/null 2>&1 && readelf -d "$1"' _ {} \; \
    | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p' | sort -u | grep -vxF -f <(echo "$BUNDLED_SONAMES"))"
SONAME_PACKAGES="$(for soname in $NEEDED_SONAMES; do pacman -Qqo "/usr/lib/$soname"; done | sort -u)"
# programs the app runs, and the driver's libcuda that no NEEDED entry names
RUNTIME_PACKAGES="ffmpeg xmlsec libxml2 curl nvidia-utils"
DEPENDS="$(printf '%s\n' $RUNTIME_PACKAGES $SONAME_PACKAGES | sort -u | sed "s/.*/'&'/" | tr '\n' ' ')"

cat > "$PACKAGE_DIRECTORY/PKGBUILD" <<PKGBUILD
pkgname=dcp-wizard
pkgver=$VERSION
pkgrel=1
pkgdesc='$PRODUCT_NAME with the Grok CUDA plugin for sm_$CUDA_ARCH'
arch=('x86_64')
license=('AGPL-3.0-or-later')
depends=($DEPENDS)
options=('!strip' '!debug')

package() {
    cp -a "\$startdir/root/." "\$pkgdir/"
}
PKGBUILD

cd "$PACKAGE_DIRECTORY"
# ffmpeg and the driver are not in this image
makepkg --nodeps --force
EOF

read_tauri_config() {
    run_in_image -v "$WORK:/wizard:ro" "$IMAGE" jq -r "$1" /wizard/gui/src-tauri/tauri.conf.json
}
PRODUCT_NAME="$(read_tauri_config .productName)"
VERSION="$(read_tauri_config .version)"
BUILT_PACKAGE="$WORK/arch-package/dcp-wizard-$VERSION-1-x86_64.pkg.tar.zst"
PACKAGE_NAME="${PRODUCT_NAME// /-}-$VERSION-1-sm$CUDA_ARCH-x86_64.pkg.tar.zst"
cp "$BUILT_PACKAGE" "$OUTPUT_DIRECTORY/$PACKAGE_NAME"
echo "wrote $OUTPUT_DIRECTORY/$PACKAGE_NAME"

podman run --rm -i --security-opt label=disable \
    -v "$OUTPUT_DIRECTORY/$PACKAGE_NAME:/packages/$PACKAGE_NAME:ro" \
    -e PACKAGE_NAME="$PACKAGE_NAME" \
    "$ARCH_IMAGE" bash -euo pipefail <<'EOF'
pacman -Syu --noconfirm binutils
pacman -U --noconfirm "/packages/$PACKAGE_NAME"
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
echo "package installs on archlinux:base"
