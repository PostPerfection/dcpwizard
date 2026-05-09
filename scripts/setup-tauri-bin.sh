#!/usr/bin/env bash
# Build the dcpwizard CLI and copy it into the Tauri binaries folder
# so that `npx tauri dev` / `npx tauri build` can find it.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="${ROOT}/build"
BIN_DIR="${ROOT}/gui/src-tauri"
TARGET_TRIPLE="$(rustc -vV | awk '/^host:/ {print $2}')"

# Build the C++ CLI if needed
if [[ ! -f "${BUILD_DIR}/dcpwizard" ]]; then
    echo "Building dcpwizard CLI …"
    cmake --build "${BUILD_DIR}" --target dcpwizard --parallel
fi

mkdir -p "${BIN_DIR}"
cp "${BUILD_DIR}/dcpwizard" "${BIN_DIR}/dcpwizard-${TARGET_TRIPLE}"
echo "Installed: ${BIN_DIR}/dcpwizard-${TARGET_TRIPLE}"
