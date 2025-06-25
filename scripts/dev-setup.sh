#!/usr/bin/env bash
# Install / update development CLI tooling.
set -euo pipefail

tools=(
  cargo-audit
  cargo-deny
  cargo-release
  cargo-vet
  hyperfine
  cargo-flamegraph
)

for t in "${tools[@]}"; do
  if ! command -v "${t}" &>/dev/null; then
    echo "🔧 Installing ${t} ..."
    cargo install "${t}" --locked
  else
    echo "✅ ${t} already present"
  fi
done

echo "🟢 Dev tools ready"
