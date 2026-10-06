#!/usr/bin/env just --justfile

fmt:
    cargo +nightly fmt --all -- --config format_code_in_doc_comments=true

check:
	cargo check --locked --all-targets
	cargo check --locked --all-targets --target wasm32-unknown-unknown

clippy:
	cargo clippy --locked --all-targets -- -D warnings
	cargo clippy --locked --all-targets --target wasm32-unknown-unknown -- -D warnings

test:
	cargo test --locked
	wasm-pack test --firefox --headless --locked

precommit: fmt check clippy test
