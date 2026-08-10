#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <path-to-binary>"
  exit 1
fi

BINARY_PATH="$1"

if [[ ! -f "$BINARY_PATH" ]]; then
  echo "error: binary not found: $BINARY_PATH"
  exit 1
fi

chmod +x "$BINARY_PATH"
if [[ ! -x "$BINARY_PATH" ]]; then
  echo "error: failed to make binary executable: $BINARY_PATH"
  exit 1
fi

# Remove Gatekeeper quarantine attribute added to downloaded files.
# This is needed for unsigned/notarized local development binaries.
/usr/bin/xattr -dr com.apple.quarantine "$BINARY_PATH" || true

echo "ok: executable bit ensured (+x)"
echo "ok: quarantine attribute removed (if present)"
echo "try now: \"$BINARY_PATH\" --help"
