#!/usr/bin/env bash

set -euo pipefail

PACKAGE="${1:-}"
if [[ -z "$PACKAGE" ]]; then
    echo "usage: $0 <rpm or deb>" >&2
    exit 2
fi

PAYLOAD="$(mktemp -d)"
trap 'rm -rf "$PAYLOAD"' EXIT
case "$PACKAGE" in
    *.rpm) rpm2cpio "$PACKAGE" | cpio -idm --quiet -D "$PAYLOAD" ;;
    *.deb) dpkg-deb -x "$PACKAGE" "$PAYLOAD" ;;
    *) echo "$PACKAGE is neither an rpm nor a deb" >&2; exit 2 ;;
esac

LIBRARY_DIRECTORY="$PAYLOAD/usr/lib/dcpwizard"
failed=0
for binary in "$PAYLOAD/usr/bin/dcpwizard" "$PAYLOAD/usr/bin/dcpwizard-gui"; do
    if ! readelf -d "$binary" | grep -F RUNPATH | grep -qF '$ORIGIN/../lib/dcpwizard'; then
        echo "${binary#$PAYLOAD} has no runpath to /usr/lib/dcpwizard" >&2
        failed=1
    fi
    # a system FFmpeg found first at link time leaves a soname the bundle does not ship
    for soname in $(readelf -d "$binary" | sed -n 's/.*(NEEDED).*\[\(lib\(av\|sw\|grok\)[^]]*\)\]/\1/p'); do
        if [[ ! -f "$LIBRARY_DIRECTORY/$soname" ]]; then
            echo "${binary#$PAYLOAD} needs $soname, which /usr/lib/dcpwizard does not ship" >&2
            failed=1
        fi
    done
done
exit $failed
