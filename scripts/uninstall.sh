#!/bin/sh
set -eu

install_dir=${TECHJOBSNL_INSTALL_DIR:-"$HOME/.local/bin"}
binary="$install_dir/techjobsnl"

case "$(uname -s)" in
    Linux) data_dir="${XDG_CONFIG_HOME:-"$HOME/.config"}/techjobsnl" ;;
    Darwin) data_dir="$HOME/Library/Application Support/techjobsnl" ;;
    *)
        printf '%s\n' 'unsupported operating system; use the Windows uninstaller on Windows' >&2
        exit 1
        ;;
esac

if [ -e "$binary" ] || [ -L "$binary" ]; then
    rm -f -- "$binary"
    printf 'Removed %s\n' "$binary"
else
    printf 'techjobsnl is not installed at %s\n' "$binary"
fi

if [ -e "$data_dir" ] || [ -L "$data_dir" ]; then
    remove_data=${TECHJOBSNL_REMOVE_DATA:-}
    if [ -z "$remove_data" ] && ( : < /dev/tty ) 2>/dev/null; then
        printf 'Remove configuration and job history at %s? [y/N] ' "$data_dir" > /dev/tty
        IFS= read -r remove_data < /dev/tty || remove_data=
    fi
    case "$remove_data" in
        y|Y|yes|YES|Yes|1|true|TRUE|True)
            rm -rf -- "$data_dir"
            printf 'Removed configuration and job history at %s\n' "$data_dir"
            ;;
        *) printf 'Kept configuration and job history at %s\n' "$data_dir" ;;
    esac
else
    printf 'No configuration or job history found at %s\n' "$data_dir"
fi
printf '%s\n' 'Feedback welcome: https://github.com/imhalawa/techjobsnl/issues/new?labels=feedback'
