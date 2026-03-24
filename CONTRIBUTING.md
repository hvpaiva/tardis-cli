# Contributing to TARDIS

Thanks for your interest! Here's how to get started.

## Quick Setup

```bash
git clone https://github.com/hvpaiva/tardis-cli.git
cd tardis-cli
./scripts/dev-setup.sh   # installs all tools + git hooks
just check               # runs fmt, lint, test, audit
```

## Workflow

1. **Fork** the repo and create a feature branch
2. Make your changes — the pre-commit hook runs `just check` automatically
3. Commit using [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat(cli): add --json flag`
   - `fix(core): handle ambiguous DST times`
   - `docs: update README examples`
4. Open a PR against `main` — CI will run all checks

## Handy Commands

| Command | What it does |
|---------|-------------|
| `just check` | fmt + lint + test + audit (full pipeline) |
| `just fmt` | auto-format code |
| `just test` | run tests with nextest |
| `just coverage` | generate HTML coverage report |
| `just bench` | run criterion benchmarks |
| `just run "tomorrow"` | run the CLI locally |

## Guidelines

- Keep PRs focused and small
- Add tests for new functionality
- Run `just check` before pushing
- Don't edit `CHANGELOG.md` or version in `Cargo.toml` manually — the CD pipeline handles this

## Project Structure

```
src/
  main.rs    — binary entry-point
  lib.rs     — module re-exports
  cli.rs     — CLI parsing (clap)
  core.rs    — date parsing + formatting logic
  config.rs  — TOML config loading
  errors.rs  — error types + exit handling
```
