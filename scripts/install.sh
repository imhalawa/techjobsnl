#!/bin/sh
set -eu

repository=${TECHJOBSNL_REPOSITORY:-${JOB_WATCH_REPOSITORY:-imhalawa/techjobsnl}}
version=${TECHJOBSNL_VERSION:-${JOB_WATCH_VERSION:-latest}}
install_dir=${TECHJOBSNL_INSTALL_DIR:-${JOB_WATCH_INSTALL_DIR:-"$HOME/.local/bin"}}

case "$(uname -s)" in
    Linux) os=unknown-linux-gnu ;;
    Darwin) os=apple-darwin ;;
    *)
        printf '%s\n' 'unsupported operating system; use the Windows installer on Windows' >&2
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64|amd64) arch=x86_64 ;;
    arm64|aarch64) arch=aarch64 ;;
    *)
        printf 'unsupported CPU architecture: %s\n' "$(uname -m)" >&2
        exit 1
        ;;
esac

asset="techjobsnl-$arch-$os.tar.gz"
if [ -n "${TECHJOBSNL_DOWNLOAD_BASE:-${JOB_WATCH_DOWNLOAD_BASE:-}}" ]; then
    download_base=${TECHJOBSNL_DOWNLOAD_BASE:-$JOB_WATCH_DOWNLOAD_BASE}
elif [ "$version" = latest ]; then
    download_base="https://github.com/$repository/releases/latest/download"
else
    download_base="https://github.com/$repository/releases/download/$version"
fi

temporary_dir=$(mktemp -d "${TMPDIR:-/tmp}/techjobsnl-install.XXXXXX")
trap 'rm -rf "$temporary_dir"' EXIT HUP INT TERM

download() {
    source_url=$1
    destination=$2
    if command -v curl >/dev/null 2>&1; then
        case "$source_url" in
            file://*) curl -fsSL "$source_url" -o "$destination" ;;
            *) curl --proto '=https' --tlsv1.2 -fsSL "$source_url" -o "$destination" ;;
        esac
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$source_url" -O "$destination"
    else
        printf '%s\n' 'installation requires curl or wget' >&2
        exit 1
    fi
}

download "$download_base/$asset" "$temporary_dir/$asset"
download "$download_base/SHA256SUMS" "$temporary_dir/SHA256SUMS"

expected=$(awk -v asset="$asset" '$2 == asset || $2 == "*" asset { print $1; exit }' "$temporary_dir/SHA256SUMS")
if [ -z "$expected" ]; then
    printf 'SHA256SUMS has no entry for %s\n' "$asset" >&2
    exit 1
fi
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$temporary_dir/$asset" | awk '{ print $1 }')
elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$temporary_dir/$asset" | awk '{ print $1 }')
else
    printf '%s\n' 'installation requires sha256sum or shasum' >&2
    exit 1
fi
if [ "$actual" != "$expected" ]; then
    printf '%s\n' 'download checksum verification failed' >&2
    exit 1
fi

tar -xzf "$temporary_dir/$asset" -C "$temporary_dir" techjobsnl
mkdir -p "$install_dir"
temporary_binary="$install_dir/.techjobsnl.$$"
cp "$temporary_dir/techjobsnl" "$temporary_binary"
chmod 755 "$temporary_binary"
mv "$temporary_binary" "$install_dir/techjobsnl"

printf 'Installed techjobsnl to %s/techjobsnl\n' "$install_dir"
case ":$PATH:" in
    *":$install_dir:"*) ;;
    *) printf 'Add %s to PATH before running techjobsnl.\n' "$install_dir" ;;
esac
