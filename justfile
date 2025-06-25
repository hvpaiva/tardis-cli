default: lint_all

fmt:
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all-features

audit:
    cargo audit

deny:
    cargo deny check

vet:
    cargo vet

lint_all: fmt clippy test audit deny vet
    @echo "✅ All checks passed"

install_tools:
    bash scripts/dev-setup.sh

bench:
    cargo bench

bench_quick:
    hyperfine -N --warmup 3 --runs 100 'td "in 3 days" -f "%s"'

bench_compare: bench_quick


flamegraph:
    CARGO_PROFILE_RELEASE_DEBUG=true CARGO_PROFILE_RELEASE_STRIP=none \
      cargo flamegraph --bench parse
    @echo "🔥  flamegraph.svg generated"
