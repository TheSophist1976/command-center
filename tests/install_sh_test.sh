#!/bin/sh
# Tests for install.sh. Run: sh tests/install_sh_test.sh
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
INSTALL_SH_LIB=1
export INSTALL_SH_LIB
# shellcheck source=../install.sh
. "$HERE/install.sh"

failures=0
check() { # name expected actual
    if [ "$2" = "$3" ]; then printf 'ok   %s\n' "$1"; else printf 'FAIL %s: expected [%s] got [%s]\n' "$1" "$2" "$3"; failures=$((failures + 1)); fi
}

# --- detect_target, with uname faked ---
# shellcheck disable=SC2317  # called indirectly: install.sh's detect_target runs `uname`
uname() { if [ "$1" = "-s" ]; then echo "$FAKE_OS"; else echo "$FAKE_ARCH"; fi; }
check "darwin arm64" aarch64-apple-darwin "$(FAKE_OS=Darwin FAKE_ARCH=arm64 detect_target)"
check "darwin x64" x86_64-apple-darwin "$(FAKE_OS=Darwin FAKE_ARCH=x86_64 detect_target)"
check "linux x64" x86_64-unknown-linux-gnu "$(FAKE_OS=Linux FAKE_ARCH=x86_64 detect_target)"
check "linux arm64" aarch64-unknown-linux-gnu "$(FAKE_OS=Linux FAKE_ARCH=aarch64 detect_target)"
unsupported=$( (FAKE_OS=Windows_NT FAKE_ARCH=x86_64 detect_target) 2>&1 || true)
case "$unsupported" in *unsupported*) check "unsupported platform message" ok ok ;; *) check "unsupported platform message" ok "$unsupported" ;; esac
unset -f uname

# --- verify ---
tmp=$(mktemp -d "${TMPDIR:-/tmp}/task-install.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
printf 'hello' > "$tmp/a.tar.gz"
good=$(sha256_of "$tmp/a.tar.gz")
printf '%s  a.tar.gz\n' "$good" > "$tmp/SHA256SUMS"
( cd "$tmp" && verify a.tar.gz SHA256SUMS ) && check "verify accepts match" ok ok
printf '%s  a.tar.gz\n' "0000000000000000000000000000000000000000000000000000000000000000" > "$tmp/SHA256SUMS"
out=$( (cd "$tmp" && verify a.tar.gz SHA256SUMS) 2>&1 || true)
case "$out" in *mismatch*) check "verify rejects mismatch" ok ok ;; *) check "verify rejects mismatch" ok "$out" ;; esac
printf '%s  other.tar.gz\n' "$good" > "$tmp/SHA256SUMS"
out=$( (cd "$tmp" && verify a.tar.gz SHA256SUMS) 2>&1 || true)
case "$out" in *"no checksum"*) check "verify rejects missing entry" ok ok ;; *) check "verify rejects missing entry" ok "$out" ;; esac

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all passed"
