#!/usr/bin/env bash
# Install / update development CLI tooling.
set -euo pipefail

if ! command -v rustup &>/dev/null; then
  echo "🦀 Rustup não encontrado — instalando..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
    sh -s -- -y --profile minimal --default-toolchain stable
  export PATH="$HOME/.cargo/bin:$PATH"
else
  echo "✅ rustup já presente"
fi

tools=(
  just
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
