#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

output=$(CARGO=/bin/false make --no-print-directory run </dev/null 2>&1 || true)
case "$output" in
    *"interactive terminal"*) ;;
    *)
        printf '%s\n' "make run did not explain that it needs an interactive terminal" >&2
        exit 1
        ;;
esac

target_dir=$(env -u CARGO_TARGET_DIR make --no-print-directory -s \
    -f Makefile -f - print-target-dir <<'MAKEFILE'
print-target-dir:
	@printf '%s\n' '$(CARGO_TARGET_DIR)'
MAKEFILE
)
case "$target_dir" in
    /mnt/*)
        printf '%s\n' "Cargo target directory must not use the mounted Windows filesystem" >&2
        exit 1
        ;;
esac
