verify: check build test

check: check_fmt lint doc

fmt:
	cargo +nightly fmt --all

check_fmt:
	cargo +nightly fmt --all -- --check

lint:
	cargo clippy --tests --all-features --no-deps -- -D clippy::all

build:
	cargo build-all-features

test:
	cargo test --all-features

doc:
	cargo doc --all-features --workspace
