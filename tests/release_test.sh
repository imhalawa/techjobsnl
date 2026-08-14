#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

sh -n scripts/check-release-version.sh
sh -n scripts/install.sh
sh scripts/check-release-version.sh v0.1.0 >/dev/null
if sh scripts/check-release-version.sh v9.9.9 >/dev/null 2>&1; then
    printf '%s\n' 'release version check accepted the wrong tag' >&2
    exit 1
fi

temporary_dir=$(mktemp -d "${TMPDIR:-/tmp}/techjobsnl-release-test.XXXXXX")
trap 'rm -rf "$temporary_dir"' EXIT HUP INT TERM

case "$(uname -s)" in
    Linux) os=unknown-linux-gnu ;;
    Darwin) os=apple-darwin ;;
    *) exit 0 ;;
esac
case "$(uname -m)" in
    x86_64|amd64) arch=x86_64 ;;
    arm64|aarch64) arch=aarch64 ;;
    *) exit 0 ;;
esac

asset="techjobsnl-$arch-$os.tar.gz"
mkdir -p "$temporary_dir/package" "$temporary_dir/release"
printf '%s\n' '#!/bin/sh' 'printf "%s\n" installed' > "$temporary_dir/package/techjobsnl"
chmod 755 "$temporary_dir/package/techjobsnl"
tar -czf "$temporary_dir/release/$asset" -C "$temporary_dir/package" techjobsnl
if command -v sha256sum >/dev/null 2>&1; then
    hash=$(sha256sum "$temporary_dir/release/$asset" | awk '{ print $1 }')
else
    hash=$(shasum -a 256 "$temporary_dir/release/$asset" | awk '{ print $1 }')
fi
printf '%s  %s\n' "$hash" "$asset" > "$temporary_dir/release/SHA256SUMS"

TECHJOBSNL_DOWNLOAD_BASE="file://$temporary_dir/release" \
TECHJOBSNL_INSTALL_DIR="$temporary_dir/bin" \
    sh scripts/install.sh >/dev/null

test "$("$temporary_dir/bin/techjobsnl")" = installed
test "$(grep -c 'target:' .github/workflows/release.yml)" -eq 6
grep -q 'aarch64-pc-windows-msvc' .github/workflows/release.yml
grep -q 'Download checksum verification failed' scripts/install.ps1
