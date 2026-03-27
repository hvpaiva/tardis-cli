% TD-DIFF(1) TARDIS Manual
% TARDIS Contributors
% 2026

# NAME

td-diff - compute the difference between two dates

# SYNOPSIS

**td diff** *DATE1* *DATE2* [*OPTIONS*]

# DESCRIPTION

**td diff** computes the duration between two date expressions and prints
the result.  By default, the output is a human-readable duration string
(e.g. "2 months, 14 days").  Alternative output formats include total
seconds and ISO 8601 duration.

Both *DATE1* and *DATE2* accept the same natural-language expressions as
the main **td** command, including epoch timestamps with the **@** prefix.

# OPTIONS

**-o**, **-\-output** *FORMAT*
:   Select the diff output format.  Accepted values:

    - **human** -- Human-readable duration (default).  Example: "2 months, 14 days".
    - **seconds** -- Total seconds between the two dates.
    - **iso** -- ISO 8601 duration format.  Example: "P2M14D".

**-j**, **-\-json**
:   Output as a JSON object.

**-n**, **-\-no-newline**
:   Suppress the trailing newline.

**-\-now** *DATETIME*
:   Override the current time (RFC 3339).

**-t**, **-\-timezone** *TZ*
:   Timezone for resolution (IANA/Olson ID).

**-v**, **-\-verbose**
:   Print verbose diagnostics to stderr.

**-h**, **-\-help**
:   Print help information.

# EXAMPLES

Basic difference between two dates:

    td diff "2025-01-01" "2025-03-15" -t UTC

Output as total seconds:

    td diff "2025-01-01" "2025-06-01" --output seconds -t UTC

Output as ISO 8601 duration:

    td diff "2025-01-01" "2025-03-15" --output iso -t UTC

JSON output:

    td diff yesterday tomorrow --json -t UTC

Timezone-aware diff:

    td diff "2025-01-01" "2025-07-01" -t America/New_York

Deterministic output with --now (for scripting):

    td diff "now" "next friday" --now 2025-06-24T09:00:00Z -t UTC

# SEE ALSO

**td**(1), **td-convert**(1), **td-tz**(1), **td-info**(1),
**td-range**(1), **td-config**(1), **td-completions**(1)
