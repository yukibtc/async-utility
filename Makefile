precommit:
	cargo fmt --all -- --config format_code_in_doc_comments=true
	cargo test --no-default-features
	cargo test --features blocking
	cargo clippy --no-default-features
	cargo clippy --features blocking
	cargo clippy --target wasm32-unknown-unknown

clean:
	cargo clean

loc:
	@echo "--- Counting lines of .rs files (LOC):" && find src/ -type f -name "*.rs" -exec cat {} \; | wc -l