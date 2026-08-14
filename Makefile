.DEFAULT_GOAL := help

CARGO_TARGET_DIR ?= $(HOME)/.cache/job-watch-target
export CARGO_TARGET_DIR

.PHONY: help run require-tty build test test-live check fmt clippy clean

help:
	@printf '%s\n' \
		'make run        Run the release build' \
		'make build      Build the release binary' \
		'make test       Run all offline tests' \
		'make test-live  Run ignored live-source tests' \
		'make check      Check formatting, lint, and tests' \
		'make fmt        Format Rust sources' \
		'make clippy     Run Clippy' \
		'make clean      Remove Cargo build artifacts (keeps .data)'

run: require-tty
	cargo run --release

require-tty:
	@if ! test -t 0 || ! test -t 1; then \
		printf '%s\n' 'make run requires an interactive terminal' >&2; \
		exit 2; \
	fi

build:
	cargo build --release

test:
	sh tests/makefile_test.sh
	sh tests/release_test.sh
	cargo test --all-targets

test-live:
	cargo test --all-targets -- --ignored --nocapture

check:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings
	sh tests/makefile_test.sh
	sh tests/release_test.sh
	cargo test --all-targets

fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

clean:
	cargo clean
