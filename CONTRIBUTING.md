# Contributing to TARDIS

## Quick setup

```bash
git clone https://github.com/your-user/tardis.git
cd tardis
bash scripts/dev-setup.sh        # or just install-tools
just lint-all
```

## Workflow

1. Fork and create a feature branch.
2. Commit — the pre-commit hook runs `just lint-all`.
3. Open a pull request; CI must pass.
4. Maintainers tag `vX.Y.Z`; CI publishes automatically.

### Handy commands

| Command         | Purpose                    |
| --------------- | -------------------------- |
| `just fmt`      | formatting check           |
| `just clippy`   | static analysis            |
| `just test`     | unit + integration tests   |
| `just audit`    | vulnerable dependency scan |
| `just lint-all` | run *every* check          |

All required CLI tools are installed via `scripts/dev-setup.sh`.
