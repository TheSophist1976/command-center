#!/bin/sh
# Install command-center (`task`; older releases also shipped `task-tui`) from the public releases repo.
#   curl -fsSL https://raw.githubusercontent.com/TheSophist1976/command-center-releases/main/install.sh | sh
# Env: INSTALL_DIR (default ~/.local/bin). Test hooks: RELEASE_TAG, RELEASE_BASE_URL.
set -eu

REPO="${RELEASES_REPO:-TheSophist1976/command-center-releases}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

err() { printf 'error: %s\n' "$1" >&2; exit 1; }
info() { printf '▸ %s\n' "$1"; }

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "$os/$arch" in
        Darwin/arm64) echo aarch64-apple-darwin ;;
        Darwin/x86_64) echo x86_64-apple-darwin ;;
        Linux/x86_64) echo x86_64-unknown-linux-gnu ;;
        Linux/aarch64 | Linux/arm64) echo aarch64-unknown-linux-gnu ;;
        *) err "unsupported platform: $os $arch (supported: macOS arm64/x64, Linux x64/arm64)" ;;
    esac
}

latest_tag() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1
}

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

# verify <archive-file-in-cwd> <sums-file>
verify() {
    expected=$(awk -v f="$1" '$2 == f || $2 == "*" f { print $1 }' "$2")
    [ -n "$expected" ] || err "no checksum for $1 in SHA256SUMS"
    actual=$(sha256_of "$1")
    [ "$expected" = "$actual" ] || err "checksum mismatch for $1 — aborting"
}

profile_file() {
    case "${SHELL:-}" in
        */zsh) echo "$HOME/.zshrc" ;;
        */bash) if [ "$(uname -s)" = Darwin ]; then echo "$HOME/.bash_profile"; else echo "$HOME/.bashrc"; fi ;;
        *) echo "$HOME/.profile" ;;
    esac
}

offer_path() {
    case ":$PATH:" in *":$INSTALL_DIR:"*) return 0 ;; esac
    profile=$(profile_file)
    line="export PATH=\"$INSTALL_DIR:\$PATH\""
    if [ -f "$profile" ] && grep -qF "$INSTALL_DIR" "$profile"; then
        info "$INSTALL_DIR is already referenced in $profile (restart your shell)"
        return 0
    fi
    if ( : < /dev/tty ) 2>/dev/null; then
        printf '▸ Add %s to your PATH in %s? [Y/n] ' "$INSTALL_DIR" "$profile"
        read -r answer < /dev/tty || answer=""
        case "$answer" in [Nn]*) info "Skipped. Add it yourself: $line"; return 0 ;; esac
        printf '\n# added by command-center installer\n%s\n' "$line" >> "$profile"
        info "Added to $profile (restart your shell)"
    else
        info "Add $INSTALL_DIR to your PATH: $line"
    fi
}

main() {
    target=$(detect_target)
    tag="${RELEASE_TAG:-$(latest_tag)}"
    [ -n "$tag" ] || err "could not determine the latest release of $REPO"
    base="${RELEASE_BASE_URL:-https://github.com/$REPO/releases/download/$tag}"
    archive="task-$tag-$target.tar.gz"

    tmp=$(mktemp -d "${TMPDIR:-/tmp}/task-install.XXXXXX")
    trap 'rm -rf "$tmp"' EXIT
    info "Downloading $archive"
    curl -fsSL "$base/$archive" -o "$tmp/$archive" || err "download failed: $base/$archive"
    curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS" || err "download failed: $base/SHA256SUMS"
    (cd "$tmp" && verify "$archive" SHA256SUMS)

    tar -xzf "$tmp/$archive" -C "$tmp"
    mkdir -p "$INSTALL_DIR"
    for bin in task task-tui; do
        if [ -f "$tmp/$bin" ]; then install -m 755 "$tmp/$bin" "$INSTALL_DIR/$bin"; fi
    done
    info "Installed to $INSTALL_DIR"
    offer_path
    echo
    echo "Next: run \`task setup\`"
}

[ "${INSTALL_SH_LIB:-}" = 1 ] || main "$@"
