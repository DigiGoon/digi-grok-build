#!/usr/bin/env bash
#
# digi Grok (dgrok) installer — same shape as https://x.ai/cli/install.sh
#
# Official Grok downloads prebuilts from x.ai/GCS.
# digi downloads prebuilts from GitHub Releases (DigiGoon/digi-grok-build),
# then falls back to a source build only if no release asset exists yet.
#
# Usage (mirrors x.ai):
#   curl -fsSL https://raw.githubusercontent.com/DigiGoon/digi-grok-build/main/scripts/install.sh | bash
#   curl -fsSL …/install.sh | bash -s 0.1.0          # pin version (tag v0.1.0)
#   curl -fsSL …/install.sh | bash -s v0.1.0
#
# Env:
#   DGROK_BIN_DIR / GROK_BIN_DIR   install dir (default: ~/.grok/bin)
#   DGROK_REPO_SLUG                owner/repo (default: DigiGoon/digi-grok-build)
#   DGROK_CHANNEL                  unused alias for parity; always "latest" release
#   DGROK_FROM_SOURCE=1            force cargo build (dev)
#   DGROK_NO_SOURCE=1              fail if release download fails
#   GITHUB_TOKEN                   optional; higher API rate limits

set -e

REPO_SLUG="${DGROK_REPO_SLUG:-DigiGoon/digi-grok-build}"
DOWNLOAD_DIR="${DGROK_DOWNLOAD_DIR:-$HOME/.grok/downloads}"
BIN_DIR="${DGROK_BIN_DIR:-${GROK_BIN_DIR:-$HOME/.grok/bin}}"
FROM_SOURCE="${DGROK_FROM_SOURCE:-0}"
NO_SOURCE="${DGROK_NO_SOURCE:-0}"
REPO_REF="${DGROK_REF:-main}"
SRC_DIR="${DGROK_SRC:-$HOME/.grok/src/digi-grok-build}"
REPO_URL="${DGROK_REPO:-https://github.com/${REPO_SLUG}.git}"

# Official: first arg is version X.Y.Z. We accept that or a v-prefixed tag.
TARGET="${1:-${DGROK_VERSION:-}}"
if [ -n "$TARGET" ]; then
    case "$TARGET" in
        --from-source) FROM_SOURCE=1; TARGET="" ;;
        --help|-h)
            sed -n '2,28p' "$0" 2>/dev/null || true
            exit 0
            ;;
        --*)
            echo "Unknown flag: $TARGET" >&2
            exit 1
            ;;
        *)
            # strip optional leading v for display; keep tag with v for GitHub
            ;;
    esac
fi

VERSION_TAG=""
if [ -n "$TARGET" ]; then
    if [[ "$TARGET" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9._]+)?$ ]]; then
        VERSION_TAG="v${TARGET#v}"
    else
        echo "Invalid version format: $TARGET (expected X.Y.Z or vX.Y.Z)" >&2
        exit 1
    fi
fi

DOWNLOADER=""
if command -v curl >/dev/null 2>&1; then
    DOWNLOADER="curl"
elif command -v wget >/dev/null 2>&1; then
    DOWNLOADER="wget"
else
    echo "Either curl or wget is required but neither is installed" >&2
    exit 1
fi

download_file() {
    local url="$1" output="$2"
    if [ "$DOWNLOADER" = "curl" ]; then
        if [ -n "$output" ]; then
            curl -fsSL -o "$output" "$url"
        else
            curl -fsSL "$url"
        fi
    else
        if [ -n "$output" ]; then
            wget -q -O "$output" "$url"
        else
            wget -q -O - "$url"
        fi
    fi
}

api_get() {
    local url="$1"
    if [ "$DOWNLOADER" = "curl" ]; then
        curl -fsSL -H "Accept: application/vnd.github+json" \
            -H "User-Agent: digi-grok-install" \
            ${GITHUB_TOKEN:+-H "Authorization: Bearer ${GITHUB_TOKEN}"} \
            "$url"
    else
        wget -q -O - --header="Accept: application/vnd.github+json" \
            --header="User-Agent: digi-grok-install" "$url"
    fi
}

# OS/arch → release asset name (must match .github/workflows/release.yml)
detect_platform() {
    local os arch
    case "$(uname -s)" in
        Linux*)  os=linux ;;
        Darwin*) os=darwin ;;
        MINGW*|MSYS*|CYGWIN*) os=windows ;;
        *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
        x86_64|amd64) arch=x86_64 ;;
        aarch64|arm64) arch=aarch64 ;;
        *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
    esac
    PLATFORM="${os}-${arch}"
    if [ "$os" = "windows" ]; then
        ASSET="dgrok-windows-${arch}.exe"
    else
        ASSET="dgrok-${os}-${arch}"
    fi
}

path_has_dir() {
    case ":$PATH:" in *":$1:"*) return 0 ;; *) return 1 ;; esac
}

ensure_path() {
    local dest="$1"
    if ! path_has_dir "$BIN_DIR"; then
        for candidate in "$HOME/.local/bin" /usr/local/bin; do
            if [ -d "$candidate" ] && path_has_dir "$candidate" && [ -w "$candidate" ]; then
                ln -sf "$dest" "$candidate/dgrok" 2>/dev/null && {
                    echo "  Symlinked $candidate/dgrok -> $dest" >&2
                    break
                }
            fi
        done
    fi

    local user_shell config_file new_block
    user_shell="$(basename "${SHELL:-bash}")"
    config_file=""
    case "$user_shell" in
        bash)
            if [ "$(uname -s)" = "Darwin" ]; then config_file="$HOME/.bash_profile"
            else config_file="$HOME/.bashrc"; fi
            ;;
        zsh)  config_file="$HOME/.zshrc" ;;
        fish) config_file="$HOME/.config/fish/config.fish" ;;
    esac
    [ -z "$config_file" ] && return 0
    mkdir -p "$(dirname "$config_file")"
    if [ "$user_shell" = "fish" ]; then
        new_block="# >>> digi dgrok installer >>>
fish_add_path $BIN_DIR
# <<< digi dgrok installer <<<"
    else
        new_block="# >>> digi dgrok installer >>>
export PATH=\"$BIN_DIR:\$PATH\"
# <<< digi dgrok installer <<<"
    fi
    if [ -f "$config_file" ] && grep -qs "digi dgrok installer\|grok installer" "$config_file" 2>/dev/null; then
        local tmp="$config_file.tmp.$$"
        awk '
            /# >>> digi dgrok installer >>>/ || /# >>> grok installer >>>/ { skip=1; next }
            /# <<< digi dgrok installer <<</ || /# <<< grok installer <<</ { skip=0; next }
            !skip { print }
        ' "$config_file" > "$tmp" && mv "$tmp" "$config_file"
    fi
    printf '\n%s\n' "$new_block" >> "$config_file"
    echo "  Updated $BIN_DIR in PATH in $config_file." >&2
}

install_bin() {
    local src="$1"
    local name="dgrok"
    case "$src" in *.exe) name="dgrok.exe" ;; esac
    mkdir -p "$BIN_DIR"
    local dest="$BIN_DIR/$name"
    # Windows-style locked exe: rename aside then copy (mirrors x.ai installer)
    if [ -f "$dest" ] && ! cp -f "$src" "$dest" 2>/dev/null; then
        mv -f "$dest" "$dest.old" 2>/dev/null || true
        cp -f "$src" "$dest"
    else
        cp -f "$src" "${dest}.tmp.$$"
        chmod +x "${dest}.tmp.$$"
        mv -f "${dest}.tmp.$$" "$dest"
    fi
    chmod +x "$dest" 2>/dev/null || true
    ln -sfn "$name" "$BIN_DIR/agent" 2>/dev/null || true
    echo "  Binary installed to $dest." >&2
}

finish() {
    local dest="$BIN_DIR/dgrok"
    [ -x "$BIN_DIR/dgrok.exe" ] && dest="$BIN_DIR/dgrok.exe"
    echo "" >&2
    "$dest" --version 2>/dev/null || true

    # Completions (best-effort) — same idea as official install.sh
    mkdir -p "$HOME/.grok/completions/bash" "$HOME/.grok/completions/zsh"
    "$dest" completions bash >"$HOME/.grok/completions/bash/dgrok.bash" 2>/dev/null || true
    "$dest" completions zsh  >"$HOME/.grok/completions/zsh/_dgrok" 2>/dev/null || true
    if mkdir -p "$HOME/.config/fish/completions" 2>/dev/null; then
        "$dest" completions fish >"$HOME/.config/fish/completions/dgrok.fish" 2>/dev/null || true
    fi

    ensure_path "$dest"
    echo "" >&2
    if path_has_dir "$BIN_DIR"; then
        echo "Run 'dgrok' to get started!" >&2
        echo "In the TUI: /provider add <name> <url> …" >&2
    else
        echo "Restart your terminal, then run 'dgrok' to get started!" >&2
        echo "  export PATH=\"$BIN_DIR:\$PATH\"" >&2
    fi
}

# ── Resolve release JSON + asset URL (GitHub Releases = our CDN) ──
resolve_asset_url() {
    local asset="$1"
    local api_url json url
    if [ -n "$VERSION_TAG" ]; then
        api_url="https://api.github.com/repos/${REPO_SLUG}/releases/tags/${VERSION_TAG}"
        echo "Fetching digi dgrok ${VERSION_TAG#v}…" >&2
    else
        api_url="https://api.github.com/repos/${REPO_SLUG}/releases/latest"
        echo "Fetching latest digi dgrok release…" >&2
    fi
    if ! json="$(api_get "$api_url" 2>/dev/null)"; then
        return 1
    fi
    if command -v python3 >/dev/null 2>&1; then
        # stdout: download URL; also sets RESOLVED_TAG via a side file
        url="$(printf '%s' "$json" | python3 -c '
import json,sys
asset=sys.argv[1]
out_tag=sys.argv[2]
data=json.load(sys.stdin)
tag=data.get("tag_name") or ""
open(out_tag,"w").write(tag)
for a in data.get("assets") or []:
    if a.get("name")==asset:
        print(a["browser_download_url"])
        sys.exit(0)
sys.exit(2)
' "$asset" "$DOWNLOAD_DIR/.tag.$$")" || {
            rm -f "$DOWNLOAD_DIR/.tag.$$"
            return 1
        }
        RESOLVED_TAG="$(tr -d '[:space:]' <"$DOWNLOAD_DIR/.tag.$$" 2>/dev/null || true)"
        rm -f "$DOWNLOAD_DIR/.tag.$$"
        printf '%s' "$url"
        return 0
    fi
    # minimal fallback without python
    RESOLVED_TAG="$(printf '%s' "$json" | tr ',' '\n' | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
    url="$(printf '%s' "$json" | tr ',' '\n' | grep browser_download_url | grep -F "$asset" | head -1 \
        | sed -E 's/.*"browser_download_url": *"([^"]+)".*/\1/')"
    [ -n "$url" ] || return 1
    printf '%s' "$url"
}

try_prebuilt() {
    detect_platform
    mkdir -p "$DOWNLOAD_DIR" "$BIN_DIR"
    local url binary_tmp
    url="$(resolve_asset_url "$ASSET")" || return 1
    [ -n "$url" ] || return 1

    local ver_label="${RESOLVED_TAG:-$VERSION_TAG}"
    ver_label="${ver_label#v}"
    echo "Installing digi dgrok ${ver_label:-latest} ($PLATFORM)…" >&2
    echo "  Downloading $ASSET…" >&2

    binary_tmp="$DOWNLOAD_DIR/${ASSET}.tmp.$$"
    if ! download_file "$url" "$binary_tmp"; then
        rm -f "$binary_tmp"
        return 1
    fi
    chmod +x "$binary_tmp"
    if ! "$binary_tmp" --version </dev/null >/dev/null 2>&1; then
        echo "Error: downloaded dgrok failed to run; keeping any existing install." >&2
        rm -f "$binary_tmp"
        return 1
    fi
    # Keep a stable name in downloads/ (like official grok-$platform)
    mv -f "$binary_tmp" "$DOWNLOAD_DIR/$ASSET"
    install_bin "$DOWNLOAD_DIR/$ASSET"
    finish
    return 0
}

build_from_source() {
    local profile="${DGROK_PROFILE:-release}"
    local repo_root="" cargo_flags=()
    echo "No prebuilt release for this platform — building from source ($profile)…" >&2

    if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
        local here
        here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        if [ -f "$here/../crates/codegen/xai-grok-shell/src/agent/provider_add.rs" ]; then
            repo_root="$(cd "$here/.." && pwd)"
        fi
    fi
    if [ -z "$repo_root" ]; then
        command -v git >/dev/null 2>&1 || { echo "git required for source install" >&2; exit 1; }
        mkdir -p "$(dirname "$SRC_DIR")"
        if [ -d "$SRC_DIR/.git" ]; then
            git -C "$SRC_DIR" fetch --depth 1 origin "$REPO_REF" 2>/dev/null || true
            git -C "$SRC_DIR" checkout -q "$REPO_REF" 2>/dev/null || true
            git -C "$SRC_DIR" pull --ff-only origin "$REPO_REF" 2>/dev/null || true
        else
            rm -rf "$SRC_DIR"
            git clone --depth 1 --branch "$REPO_REF" "$REPO_URL" "$SRC_DIR" \
                || git clone --depth 1 "$REPO_URL" "$SRC_DIR"
        fi
        repo_root="$SRC_DIR"
    fi
    if [ ! -f "$repo_root/crates/codegen/xai-grok-shell/src/agent/provider_add.rs" ]; then
        echo "Error: tree is not digi-grok-build (missing /provider support)." >&2
        exit 1
    fi
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Installing rustup…" >&2
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck disable=SC1091
        [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
    fi
    [ "$profile" = "release" ] && cargo_flags+=(--release)
    (cd "$repo_root" && cargo build -p xai-grok-pager-bin "${cargo_flags[@]}")
    local bin="$repo_root/target/$profile/dgrok"
    [ -x "${bin}.exe" ] && bin="${bin}.exe"
    [ -x "$bin" ] || { echo "build produced no dgrok binary" >&2; exit 1; }
    install_bin "$bin"
    finish
}

# ── main (same user journey as x.ai/cli/install.sh) ───────────────
echo "digi Grok CLI installer" >&2

if [ "$FROM_SOURCE" = "1" ]; then
    build_from_source
    exit 0
fi

if try_prebuilt; then
    exit 0
fi

if [ "$NO_SOURCE" = "1" ]; then
    echo "Error: no GitHub Release binary for your system." >&2
    echo "Publish a release (git tag vX.Y.Z && git push --tags) or unset DGROK_NO_SOURCE." >&2
    exit 1
fi

build_from_source
