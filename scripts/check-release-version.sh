#!/bin/sh
set -eu

tag=${1:?usage: scripts/check-release-version.sh vX.Y.Z}
version=$(awk '
    /^\[package\]$/ { in_package = 1; next }
    /^\[/ { in_package = 0 }
    in_package && /^version[[:space:]]*=/ {
        value = $0
        sub(/^[^=]*=[[:space:]]*"/, "", value)
        sub(/"[[:space:]]*$/, "", value)
        print value
        exit
    }
' Cargo.toml)

if [ -z "$version" ]; then
    printf '%s\n' 'could not read [package].version from Cargo.toml' >&2
    exit 1
fi

expected="v$version"
if [ "$tag" != "$expected" ]; then
    printf 'release tag %s does not match Cargo.toml version %s\n' "$tag" "$expected" >&2
    exit 1
fi

printf 'release version verified: %s\n' "$tag"
