#!/bin/sh
set -e
./scripts/guard-release-files.sh --staged
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
