#!/usr/bin/env bash
set -euo pipefail

info() { printf '\033[1;34m[setup]\033[0m %s\n' "$1"; }
ok()   { printf '\033[1;32m[setup]\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33m[setup]\033[0m %s\n' "$1"; }

install_cargo_tool() {
    local cmd="$1"
    local crate="${2:-$1}"

    if command -v "$cmd" &>/dev/null; then
        ok "$cmd already installed"
    else
        info "Installing $crate..."
        cargo install --locked "$crate"
        ok "$cmd installed"
    fi
}

# --- Rust components ---
for component in rustfmt clippy llvm-tools-preview; do
    if rustup component list --installed 2>/dev/null | grep -q "$component"; then
        ok "$component found"
    else
        warn "$component not found. Installing..."
        rustup component add "$component"
        ok "$component installed"
    fi
done

# --- CLI tools ---
install_cargo_tool just
install_cargo_tool cog cocogitto
install_cargo_tool "cargo-deny" cargo-deny
install_cargo_tool "cargo-nextest" cargo-nextest
install_cargo_tool "cargo-llvm-cov" cargo-llvm-cov
install_cargo_tool "cargo-audit" cargo-audit
install_cargo_tool "cargo-vet" cargo-vet
install_cargo_tool hyperfine
install_cargo_tool "cargo-flamegraph" cargo-flamegraph

# --- Git hooks ---
info "Setting up git hooks..."
cog install-hook --all --overwrite
ok "Git hooks installed (commit-msg + pre-commit)"

# --- Dependencies ---
info "Fetching dependencies..."
cargo fetch --quiet
ok "Dependencies fetched"

# --- Verify ---
echo ""
info "Running checks..."
just check && ok "All checks passed. You're good to go."
