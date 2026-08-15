#!/bin/sh
set -eu

install_dir=${TECHJOBSNL_INSTALL_DIR:-${JOB_WATCH_INSTALL_DIR:-"$HOME/.local/bin"}}
binary="$install_dir/techjobsnl"

if [ -e "$binary" ] || [ -L "$binary" ]; then
    rm -f -- "$binary"
    printf 'Removed %s\n' "$binary"
else
    printf 'techjobsnl is not installed at %s\n' "$binary"
fi

printf '%s\n' 'Configuration and job history were not removed.'
printf '%s\n' 'Feedback welcome: https://github.com/imhalawa/techjobsnl/issues/new?labels=feedback'
