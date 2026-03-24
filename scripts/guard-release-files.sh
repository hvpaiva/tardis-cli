#!/bin/sh
# Guard against manual edits to release-managed files (CHANGELOG.md, Cargo.toml version).
# Used by: pre-commit hook (staged files) and CI (PR diff).
#
# Only flags MODIFIED files, not newly added ones — so creating initial
# CHANGELOG.md and Cargo.toml is always allowed.
#
# Bypass:
#   Local:  SKIP_RELEASE_GUARD=1 git commit ...
#   CI:     add the "release-override" label to the PR
#
# Usage:
#   ./scripts/guard-release-files.sh --staged        # pre-commit (local)
#   ./scripts/guard-release-files.sh --pr <base_ref>  # CI (pull request)
set -e

if [ "${SKIP_RELEASE_GUARD:-}" = "1" ]; then
  echo "release guard: skipped (SKIP_RELEASE_GUARD=1)"
  exit 0
fi

mode="${1:---staged}"

case "$mode" in
  --staged)
    modified=$(git diff --cached --diff-filter=M --name-only)
    diff_cmd="git diff --cached"
    ;;
  --pr)
    base_ref="${2:?Usage: $0 --pr <base_ref>}"
    modified=$(git diff --diff-filter=M --name-only "origin/$base_ref"...HEAD)
    diff_cmd="git diff origin/$base_ref...HEAD"
    ;;
  *)
    echo "Usage: $0 --staged | --pr <base_ref>"
    exit 1
    ;;
esac

blocked=""
newline="
"

for f in $modified; do
  case "$f" in
    */CHANGELOG.md|CHANGELOG.md)
      blocked="${blocked}${f}${newline}"
      ;;
    */Cargo.toml|Cargo.toml)
      if $diff_cmd -- "$f" | grep -qE '^\+version\s*='; then
        blocked="${blocked}${f} (version field)${newline}"
      fi
      ;;
  esac
done

if [ -n "$blocked" ]; then
  echo "ERROR: Release files should not be edited manually."
  echo "Changelogs and versions are managed automatically by the CD pipeline."
  echo ""
  echo "Blocked files:"
  printf '%s' "$blocked" | while IFS= read -r line; do
    echo "  - $line"
  done
  echo ""
  echo "If you really need to bypass this, use: SKIP_RELEASE_GUARD=1 git commit ..."
  exit 1
fi
