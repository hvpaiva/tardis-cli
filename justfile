# Run all checks
check: fmt-check lint test audit

# Format code
fmt:
    cargo fmt --all

# Verify formatting
fmt-check:
    cargo fmt --all --check

# Lint with clippy
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run tests
test:
    cargo nextest run

# Audit dependencies
audit:
    cargo deny check

# Generate coverage report
coverage:
    cargo llvm-cov --html
    @echo "Report at target/llvm-cov/html/index.html"

# Generate coverage summary
coverage-summary:
    cargo llvm-cov --fail-under-lines 80

# Run the CLI
run *args:
    cargo run -- {{args}}

# Generate and open docs
doc:
    cargo doc --no-deps --open

# Install/update git hooks
hooks:
    cog install-hook --all --overwrite

# Setup development environment
setup:
    ./scripts/dev-setup.sh

# Run criterion benchmarks
bench:
    cargo bench

# Quick hyperfine benchmark
bench-quick:
    hyperfine -N --warmup 3 --runs 100 'td "in 3 days" -f "%s"'

# Generate flamegraph
flamegraph:
    CARGO_PROFILE_RELEASE_DEBUG=true CARGO_PROFILE_RELEASE_STRIP=none \
      cargo flamegraph --bench parse
    @echo "flamegraph.svg generated"

# Run cargo vet check
vet:
    cargo vet

# Generate SBOM
sbom:
    cargo sbom > sbom.json

# Check semver compatibility
semver-check:
    cargo semver-checks check-release
