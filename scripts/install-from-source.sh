#!/usr/bin/env bash
#
# Build digi-grok-build from source and install the CLI as `dgrok`.
#
# Usage (from repo root):
#   ./scripts/install-from-source.sh
#   ./scripts/install-from-source.sh --release
#   DGROK_BIN_DIR=~/.local/bin ./scripts/install-from-source.sh
#
# Installs to ${DGROK_BIN_DIR:-$HOME/.grok/bin}/dgrok and ensures that
# directory is on PATH when possible.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_DIR="${DGROK_BIN_DIR:-${GROK_BIN_DIR:-$HOME/.grok/bin}}"
PROFILE="debug"
CARGO_FLAGS=()

for arg in "$@"; do
    case "$arg" in
        --release) PROFILE="release"; CARGO_FLAGS+=(--release) ;;
        -h|--help)
            sed -n '2,12p' "$0"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            exit 1
            ;;
    esac
done

cd "$REPO_ROOT"
echo "Building dgrok from source ($PROFILE)…" >&2
cargo build -p xai-grok-pager-bin "${CARGO_FLAGS[@]}"

SRC="$REPO_ROOT/target/$PROFILE/dgrok"
if [ ! -x "$SRC" ]; then
    echo "error: expected binary not found at $SRC" >&2
    exit 1
fi

mkdir -p "$BIN_DIR"
# Atomic-ish install: copy then rename.
cp -f "$SRC" "$BIN_DIR/dgrok.tmp.$$"
chmod +x "$BIN_DIR/dgrok.tmp.$$"
mv -f "$BIN_DIR/dgrok.tmp.$$" "$BIN_DIR/dgrok"

# Optional compat alias used by some docs/scripts.
ln -sfn "dgrok" "$BIN_DIR/agent" 2>/dev/null || true

echo "Installed: $BIN_DIR/dgrok" >&2
"$BIN_DIR/dgrok" --version 2>/dev/null || true

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        echo "" >&2
        echo "Add to PATH for this shell:" >&2
        echo "  export PATH=\"$BIN_DIR:\$PATH\"" >&2
        ;;
esac

echo "Run: dgrok" >&2
