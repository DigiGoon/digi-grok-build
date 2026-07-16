#!/usr/bin/env bash
#
# digi Grok (dgrok) enterprise installer — same as install.sh
# (GitHub Release binary, else source build).
#
#   curl -fsSL https://raw.githubusercontent.com/DigiGoon/digi-grok-build/main/scripts/install-enterprise.sh | bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd)" || SCRIPT_DIR=""

if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/install.sh" ]; then
    exec bash "$SCRIPT_DIR/install.sh" "$@"
fi

REF="${DGROK_REF:-main}"
URL="https://raw.githubusercontent.com/DigiGoon/digi-grok-build/${REF}/scripts/install.sh"
echo "Fetching digi install.sh from $URL …" >&2
exec bash -c "$(curl -fsSL "$URL")" bash "$@"