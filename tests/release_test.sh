#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

sh -n scripts/check-release-version.sh
sh -n scripts/install.sh
sh -n scripts/uninstall.sh
sh -n docs/install
grep -Fq 'https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.sh' docs/install
for documentation in README.md docs/index.html
do
    grep -Fq 'curl -fsSL https://imhalawa.github.io/techjobsnl/install | sh' "$documentation"
    grep -Fq 'irm https://imhalawa.github.io/techjobsnl/install.ps1 | iex' "$documentation"
done
sh scripts/check-release-version.sh v0.1.0 >/dev/null
if sh scripts/check-release-version.sh v9.9.9 >/dev/null 2>&1; then
    printf '%s\n' 'release version check accepted the wrong tag' >&2
    exit 1
fi

temporary_dir=$(mktemp -d "${TMPDIR:-/tmp}/techjobsnl-release-test.XXXXXX")
trap 'rm -rf "$temporary_dir"' EXIT HUP INT TERM
temporary_home="$temporary_dir/home"
mkdir -p "$temporary_home"

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

default_output=$(
    HOME="$temporary_home" XDG_CONFIG_HOME= SHELL=/bin/zsh PATH=/usr/bin:/bin \
    TECHJOBSNL_DOWNLOAD_BASE="file://$temporary_dir/release" \
        sh scripts/install.sh
)
printf '%s\n' "$default_output" | grep -q '^\[1/6\] Detected '
printf '%s\n' "$default_output" | grep -q '^\[2/6\] Downloading '
printf '%s\n' "$default_output" | grep -q '^\[3/6\] Verifying SHA-256 checksum$'
printf '%s\n' "$default_output" | grep -q '^\[4/6\] Installing to '
printf '%s\n' "$default_output" | grep -q '^\[5/6\] Configuring command discovery$'
printf '%s\n' "$default_output" | grep -q '^\[6/6\] Installation complete$'
printf '%s\n' "$default_output" | grep -Fqx "Added PATH entry to $temporary_home/.zshrc."
case "$(uname -s)" in
    Linux) expected_config="$temporary_home/.config/techjobsnl" ;;
    Darwin) expected_config="$temporary_home/Library/Application Support/techjobsnl" ;;
esac
printf '%s\n' "$default_output" | grep -Fqx "Config on first launch: $expected_config"
test -x "$temporary_home/.local/bin/techjobsnl"

second_output=$(
    HOME="$temporary_home" XDG_CONFIG_HOME= SHELL=/bin/zsh PATH=/usr/bin:/bin \
    TECHJOBSNL_DOWNLOAD_BASE="file://$temporary_dir/release" \
        sh scripts/install.sh
)
test "$(grep -Fxc 'export PATH="$HOME/.local/bin:$PATH"' "$temporary_home/.zshrc")" -eq 1
printf '%s\n' "$second_output" | grep -q 'already available'

unterminated_home="$temporary_dir/unterminated-home"
mkdir -p "$unterminated_home"
printf '%s' 'export TECHJOBSNL_TEST=1' > "$unterminated_home/.zshrc"
unterminated_output=$(
    HOME="$unterminated_home" XDG_CONFIG_HOME= SHELL=/bin/zsh PATH=/usr/bin:/bin \
    TECHJOBSNL_DOWNLOAD_BASE="file://$temporary_dir/release" \
        sh scripts/install.sh
)
printf '%s\n' "$unterminated_output" | grep -Fqx "Added PATH entry to $unterminated_home/.zshrc."
sh -n "$unterminated_home/.zshrc"
test "$(grep -Fxc 'export TECHJOBSNL_TEST=1' "$unterminated_home/.zshrc")" -eq 1
test "$(grep -Fxc 'export PATH="$HOME/.local/bin:$PATH"' "$unterminated_home/.zshrc")" -eq 1
unterminated_second_output=$(
    HOME="$unterminated_home" XDG_CONFIG_HOME= SHELL=/bin/zsh PATH=/usr/bin:/bin \
    TECHJOBSNL_DOWNLOAD_BASE="file://$temporary_dir/release" \
        sh scripts/install.sh
)
test "$(grep -Fxc 'export PATH="$HOME/.local/bin:$PATH"' "$unterminated_home/.zshrc")" -eq 1
printf '%s\n' "$unterminated_second_output" | grep -q 'already available'

custom_home="$temporary_dir/custom-home"
mkdir -p "$custom_home"
TECHJOBSNL_DOWNLOAD_BASE="file://$temporary_dir/release" \
TECHJOBSNL_INSTALL_DIR="$temporary_dir/bin" \
HOME="$custom_home" SHELL=/bin/bash PATH=/usr/bin:/bin \
    sh scripts/install.sh >/dev/null
test ! -e "$custom_home/.bashrc"

test "$("$temporary_dir/bin/techjobsnl")" = installed
printf stale > "$temporary_dir/bin/techjobsnl"
update_output=$(
    TECHJOBSNL_DOWNLOAD_BASE="file://$temporary_dir/release" \
    TECHJOBSNL_INSTALL_DIR="$temporary_dir/bin" \
        sh scripts/install.sh
)
test "$("$temporary_dir/bin/techjobsnl")" = installed
printf '%s\n' "$update_output" | grep -q '^Updated techjobsnl at '

uninstall_output=$(TECHJOBSNL_INSTALL_DIR="$temporary_dir/bin" sh scripts/uninstall.sh)
test ! -e "$temporary_dir/bin/techjobsnl"
printf '%s\n' "$uninstall_output" | grep -q 'Configuration and job history were not removed.'
printf '%s\n' "$uninstall_output" | grep -Fqx 'Feedback welcome: https://github.com/imhalawa/techjobsnl/issues/new?labels=feedback'
grep -Fq 'https://github.com/imhalawa/techjobsnl/issues/new?labels=feedback' scripts/uninstall.ps1
TECHJOBSNL_INSTALL_DIR="$temporary_dir/bin" sh scripts/uninstall.sh >/dev/null

release_workflow=.github/workflows/release.yml
ci_workflow=.github/workflows/ci.yml

test "$(grep -c 'target:' "$release_workflow")" -eq 6
for target in \
    x86_64-unknown-linux-gnu \
    aarch64-unknown-linux-gnu \
    x86_64-apple-darwin \
    aarch64-apple-darwin \
    x86_64-pc-windows-msvc \
    aarch64-pc-windows-msvc
do
    grep -q "$target" "$release_workflow"
done

for runner in ubuntu-22.04 macos-15-intel windows-2022
do
    grep -q "$runner" "$ci_workflow"
done
grep -q 'pull_request:' "$ci_workflow"
grep -q 'cargo test --locked --all-targets' "$ci_workflow"
grep -q 'cargo build --locked --release' "$ci_workflow"
grep -q 'Download checksum verification failed' scripts/install.ps1
grep -q 'PROCESSOR_ARCHITEW6432' scripts/install.ps1
grep -q 'PROCESSOR_ARCHITECTURE' scripts/install.ps1
! grep -q 'RuntimeInformation' scripts/install.ps1
grep -q 'Configuration and job history were not removed.' scripts/uninstall.ps1
