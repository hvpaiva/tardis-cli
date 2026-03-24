//! Typo correction via Levenshtein distance for near-miss keyword suggestions.
//!
//! Hand-rolled implementation (~20 lines DP algorithm) instead of adding a
//! `strsim` dependency, per the project's "fewer is better" dependency philosophy.

// Allow dead code: consumed by lexer/grammar error paths in Plan 02 and Plan 03.
#![allow(dead_code)]

/// Compute Levenshtein edit distance between two strings.
pub(crate) fn edit_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
        *cell = j;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    matrix[a_len][b_len]
}

/// All known keywords the parser recognizes. Used for typo suggestions.
const KEYWORDS: &[&str] = &[
    "now",
    "today",
    "tomorrow",
    "yesterday",
    "overmorrow",
    "next",
    "last",
    "this",
    "in",
    "ago",
    "at",
    "from",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "mon",
    "tue",
    "wed",
    "thu",
    "fri",
    "sat",
    "sun",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "jan",
    "feb",
    "mar",
    "apr",
    "jun",
    "jul",
    "aug",
    "sep",
    "oct",
    "nov",
    "dec",
    "year",
    "years",
    "month",
    "months",
    "week",
    "weeks",
    "day",
    "days",
    "hour",
    "hours",
    "minute",
    "minutes",
    "second",
    "seconds",
    "min",
    "mins",
    "sec",
    "secs",
    "a",
    "an",
    "and",
];

/// Find the closest keyword match for an unrecognized word.
///
/// Returns `None` if no keyword is within `max_distance` edits.
/// Default max_distance of 2 catches most single-character typos.
pub(crate) fn suggest_keyword(word: &str, max_distance: usize) -> Option<&'static str> {
    let word_lower = word.to_ascii_lowercase();
    let mut best: Option<(&str, usize)> = None;

    for &kw in KEYWORDS {
        let dist = edit_distance(&word_lower, kw);
        if dist <= max_distance
            && dist > 0
            && (best.is_none() || dist < best.map_or(usize::MAX, |b| b.1))
        {
            best = Some((kw, dist));
        }
    }

    best.map(|(kw, _)| kw)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn edit_distance_identical() {
        assert_eq!(edit_distance("hello", "hello"), 0);
    }

    #[test]
    fn edit_distance_one_char_diff() {
        assert_eq!(edit_distance("hello", "hallo"), 1);
    }

    #[test]
    fn edit_distance_empty() {
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
    }

    #[test]
    fn edit_distance_completely_different() {
        assert_eq!(edit_distance("abc", "xyz"), 3);
    }

    #[test]
    fn suggest_thursdya_finds_thursday() {
        let suggestion = suggest_keyword("thursdya", 2);
        assert_eq!(suggestion, Some("thursday"));
    }

    #[test]
    fn suggest_tomorow_finds_tomorrow() {
        let suggestion = suggest_keyword("tomorow", 2);
        assert_eq!(suggestion, Some("tomorrow"));
    }

    #[test]
    fn suggest_no_match_for_gibberish() {
        let suggestion = suggest_keyword("xyzzy", 2);
        assert!(suggestion.is_none());
    }

    #[test]
    fn suggest_case_insensitive() {
        let suggestion = suggest_keyword("THURSDYA", 2);
        assert_eq!(suggestion, Some("thursday"));
    }

    #[test]
    fn suggest_exact_match_excluded() {
        // Exact matches (distance 0) should not be returned as "suggestions"
        let suggestion = suggest_keyword("wednesday", 2);
        assert!(suggestion.is_none());
    }
}
