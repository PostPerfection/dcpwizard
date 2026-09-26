#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GROK_INSTALL="${1:-}"

if [[ -z "$GROK_INSTALL" ]]; then
    echo "usage: $0 /path/to/grok/install" >&2
    exit 2
fi

for candidate in "$GROK_INSTALL/lib64" "$GROK_INSTALL/lib"; do
    if [[ -f "$candidate/libgrokj2k.so.1" && -f "$candidate/libgrokj2k_plugin.so" ]]; then
        GROK_LIBRARY_DIRECTORY="$candidate"
        break
    fi
done

if [[ -z "${GROK_LIBRARY_DIRECTORY:-}" ]]; then
    echo "no Grok core library and GPU plugin found under $GROK_INSTALL" >&2
    exit 1
fi

export PKG_CONFIG_PATH="$GROK_LIBRARY_DIRECTORY/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export LD_LIBRARY_PATH="$GROK_LIBRARY_DIRECTORY${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

cargo build --release -p dcpwizard-cli --manifest-path "$ROOT/rust/Cargo.toml"
"$ROOT/scripts/setup-tauri-bin.sh"
cp -L "$GROK_LIBRARY_DIRECTORY/libgrokj2k.so.1" "$ROOT/gui/src-tauri/libgrokj2k.so.1"
cp -L "$GROK_LIBRARY_DIRECTORY/libgrokj2k_plugin.so" "$ROOT/gui/src-tauri/libgrokj2k_plugin.so"

cd "$ROOT/gui"
pnpm install --frozen-lockfile
# the plugin is private, the committed config leaves it out
PLUGIN_FILES='{"bundle":{"linux":{"rpm":{"files":{"/usr/lib/dcpwizard/libgrokj2k.so.1":"libgrokj2k.so.1","/usr/lib/dcpwizard/libgrokj2k_plugin.so":"libgrokj2k_plugin.so"}}}}}'
pnpm tauri build --bundles rpm --config "$PLUGIN_FILES"
