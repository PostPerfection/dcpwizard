#!/usr/bin/env bash
# Verify the GUI's CLI invocations parse against the real binary.
# Run from the project root: bash tests/cli_flags_test.sh [path-to-binary]
#
# Two checks:
#  1. Every flag the GUI passes exists in that subcommand's --help.
#  2. Each GUI command line is actually invoked (with dummy values) and must
#     not fail clap parsing. This catches missing required flags and unknown
#     flags, which a --help grep alone misses (e.g. the KDM panel that used to
#     omit --signer-cert/--signer-key and failed every run).

set -uo pipefail

BINARY="${1:-./build/dcpwizard}"
JS_FILE="gui/src/main.js"
FAILURES=0
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [[ ! -x "$BINARY" ]]; then
  echo "ERROR: Binary not found at $BINARY"
  echo "Usage: $0 [path-to-dcpwizard-binary]"
  exit 1
fi
if [[ ! -f "$JS_FILE" ]]; then
  echo "ERROR: $JS_FILE not found. Run from project root."
  exit 1
fi

# clap prints these to stderr and exits 2 when parsing fails. Runtime errors
# (missing files, etc.) are fine: they mean parsing succeeded.
PARSE_ERR='unexpected argument|required arguments were not provided|a value is required for|unrecognized subcommand|invalid value'

echo "=== DCPWizard CLI Flag Verification ==="
echo "Binary: $BINARY"
echo ""

# ── Check 1: every flag the GUI sends exists in --help ──────────────────────
JS_SUBCMDS=$(grep -oP 'args\s*=\s*\["([a-z][-a-z]*)"|Command\.(?:sidecar|create)\("dcpwizard",\s*\["([a-z][-a-z]*)' "$JS_FILE" \
  | grep -oP '\["[a-z][-a-z]*' | tr -d '["' | sort -u)

for subcmd in $JS_SUBCMDS; do
  if ! "$BINARY" "$subcmd" --help &>/dev/null; then
    echo "FAIL: subcommand '$subcmd' does not exist in binary"
    FAILURES=$((FAILURES + 1))
    continue
  fi
  help_text=$("$BINARY" "$subcmd" --help 2>&1 || true)
  FLAGS=$(grep -P "\[\"$subcmd\"" "$JS_FILE" \
    | grep -oP '"--[a-z][-a-z0-9]*"|"-[a-z]"' | tr -d '"' | sort -u || true)
  for flag in $FLAGS; do
    if ! echo "$help_text" | grep -qF -- "$flag"; then
      echo "FAIL: '$subcmd $flag' not found in '$subcmd --help'"
      FAILURES=$((FAILURES + 1))
    fi
  done
done

# ── Check 2: invoke each GUI command line, reject clap parse errors ──────────
# Dummy paths that don't need to exist; we only care that parsing succeeds.
C="$TMP/f"                # generic file/dir placeholder
UUID="00000000-0000-0000-0000-000000000000"

smoke() {
  local label="$1"; shift
  local out
  out=$("$BINARY" "$@" 2>&1 || true)
  if echo "$out" | grep -qiE "$PARSE_ERR"; then
    echo "FAIL: '$label' rejected at parse time:"
    echo "$out" | grep -iE "$PARSE_ERR" | sed 's/^/    /'
    FAILURES=$((FAILURES + 1))
  else
    echo "ok: $label"
  fi
}

# Mirror the exact flag sets gui/src/main.js builds.
smoke "verify"          verify "$C" --strict --no-picture-check --no-hash-check
smoke "kdm"             kdm --cpl-id "$UUID" --content-title x --cert "$C" \
                            --signer-cert "$C" --signer-key "$C" --keys "$C" \
                            -o "$C" -f now -t "2 weeks"
smoke "batch list"      batch list
smoke "batch cancel"    batch cancel 1
smoke "encode"          encode -i "$C" -o "$C" --bandwidth 250
smoke "transcode"       transcode -i "$C" -o "$C"
smoke "loudness"        loudness "$C"
smoke "copy"            copy --src "$C" --dst "$C"
smoke "report"          report --dcp "$C" -o "$C/report.html"
smoke "subtitle-convert" subtitle-convert -i "$C" -l en --fps 24 -o "$C"
smoke "burnin"          burnin -i "$C" -s "$C" -o "$C"
smoke "convert"         convert -i "$C" -t mov -m fast -o "$C"
smoke "create+encrypt"  create --title x --video "$C" --output "$C" --encrypt --key-out "$C"

# --encrypt must require --key-out: this call MUST be rejected at parse time.
reject() {
  local label="$1"; shift
  local out
  out=$("$BINARY" "$@" 2>&1 || true)
  if echo "$out" | grep -qiE "$PARSE_ERR"; then
    echo "ok (rejected): $label"
  else
    echo "FAIL: '$label' should have been rejected but parsed"
    FAILURES=$((FAILURES + 1))
  fi
}
reject "create --encrypt without --key-out" \
       create --title x --video "$C" --output "$C" --encrypt

echo ""
echo "=== Summary ==="
if [[ $FAILURES -eq 0 ]]; then
  echo "All CLI invocations parse successfully."
  exit 0
else
  echo "$FAILURES check(s) failed."
  exit 1
fi
