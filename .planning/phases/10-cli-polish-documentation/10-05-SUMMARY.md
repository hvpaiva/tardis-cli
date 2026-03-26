---
phase: 10-cli-polish-documentation
plan: 05
subsystem: docs
tags: [man-pages, pandoc, markdown, cli-reference, subcommands]

requires:
  - phase: 10-03
    provides: "verbose flag and diff output routing in all subcommands"
provides:
  - "8 man page markdown files in docs/ (pandoc format)"
  - "SUBCOMMANDS.md complete subcommand reference"
  - "Full OPTIONS/EXAMPLES/SEE ALSO coverage for all CLI commands"
affects: [10-06, release, ci-man-page-validation]

tech-stack:
  added: []
  patterns: ["pandoc title block format for man pages", "definition list format for flag docs"]

key-files:
  created:
    - docs/td.1.md
    - docs/td-diff.1.md
    - docs/td-convert.1.md
    - docs/td-tz.1.md
    - docs/td-info.1.md
    - docs/td-range.1.md
    - docs/td-config.1.md
    - docs/td-completions.1.md
    - docs/SUBCOMMANDS.md
  modified: []

key-decisions:
  - "Pandoc title block format (% TD(1)) for man page generation compatibility"
  - "Definition list format with -\\- escaping for long flags in pandoc"
  - "All examples use --now and -t UTC for deterministic reproducibility"
  - "SUBCOMMANDS.md uses console code blocks for trycmd testability"

patterns-established:
  - "Man page structure: NAME, SYNOPSIS, DESCRIPTION, OPTIONS, EXAMPLES, SEE ALSO"
  - "Granularity expansion documentation pattern for range subcommand"

requirements-completed: [D-06, D-07, D-08, D-14, D-24]

duration: 3min
completed: 2026-03-26
---

# Phase 10 Plan 05: Man Pages and Subcommand Reference Summary

**8 hand-crafted pandoc man pages for td and all subcommands plus SUBCOMMANDS.md consolidated reference**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-26T14:32:18Z
- **Completed:** 2026-03-26T14:36:13Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- Created 8 man page markdown files in pandoc format, one for each td command and subcommand
- Each man page has complete OPTIONS, 5-10 EXAMPLES, and cross-referenced SEE ALSO sections
- Created SUBCOMMANDS.md (333 lines) as a consolidated reference with options tables and console examples
- All content written from scratch based on current codebase state (no legacy generate_man_page issues)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create main td(1) man page and subcommand man pages (diff, convert, tz, info)** - `da1e3c5` (docs)
2. **Task 2: Create remaining man pages (range, config, completions) and SUBCOMMANDS.md** - `4157914` (docs)

## Files Created/Modified
- `docs/td.1.md` - Main td man page with full OPTIONS, ENVIRONMENT, EXIT STATUS, and SEE ALSO
- `docs/td-diff.1.md` - Man page for td diff with --output (human/seconds/iso) documentation
- `docs/td-convert.1.md` - Man page for td convert with --from/--to and built-in format names
- `docs/td-tz.1.md` - Man page for td tz with --from/--to timezone conversion
- `docs/td-info.1.md` - Man page for td info with default "now" behavior documented
- `docs/td-range.1.md` - Man page for td range with granularity expansion and --delimiter documentation
- `docs/td-config.1.md` - Man page for td config with path/show/edit/presets subcommands
- `docs/td-completions.1.md` - Man page for td completions with all 5 shells and install examples
- `docs/SUBCOMMANDS.md` - Consolidated subcommand reference with options tables and console examples

## Decisions Made
- Used pandoc title block format (`% TD(1) TARDIS Manual`) for man page generation compatibility
- Used definition list format with `-\-` escaping for long flags (required by pandoc)
- All examples include `--now` and `-t UTC` flags for deterministic, timezone-independent output
- SUBCOMMANDS.md uses `console` code blocks for trycmd testability
- Man pages reference `docs/FORMAT-SPECIFIERS.md` instead of external jiff/chrono URLs
- SEE ALSO cross-references between all 8 man pages for discoverability

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all documentation is complete and self-contained.

## Next Phase Readiness
- 8 man pages ready for CI validation pipeline
- SUBCOMMANDS.md ready for inclusion in README or website
- Format consistent with pandoc-to-roff conversion via `pandoc -s -t man`

## Self-Check: PASSED

All 9 created files verified on disk. Both task commits (da1e3c5, 4157914) verified in git log.

---
*Phase: 10-cli-polish-documentation*
*Completed: 2026-03-26*
