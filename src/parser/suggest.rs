//! Typo correction via Levenshtein distance for near-miss keyword suggestions.
//!
//! Uses the compile-time [`KEYWORD_LIST`](super::lexer::KEYWORD_LIST) for
//! near-miss suggestions. Hand-rolled implementation (~20 lines DP algorithm)
//! instead of adding a `strsim` dependency, per the project's "fewer is better"
//! dependency philosophy.

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

/// Find the closest keyword match for an unrecognized word.
///
/// Returns `None` if no keyword is within `max_distance` edits.
/// Default max_distance of 2 catches most single-character typos.
///
/// Iterates over [`KEYWORD_LIST`](super::lexer::KEYWORD_LIST) directly.
pub(crate) fn suggest_keyword(word: &str, max_distance: usize) -> Option<String> {
    let word_lower = word.to_ascii_lowercase();
    let mut best: Option<(String, usize)> = None;

    for &(kw, _) in super::lexer::KEYWORD_LIST {
        let dist = edit_distance(&word_lower, kw);
        if dist <= max_distance && dist > 0 {
            let is_better = match &best {
                None => true,
                Some((prev_kw, prev_dist)) => {
                    dist < *prev_dist || (dist == *prev_dist && kw < prev_kw.as_str())
                }
            };
            if is_better {
                best = Some((kw.to_string(), dist));
            }
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
        assert_eq!(suggestion, Some("thursday".to_string()));
    }

    #[test]
    fn suggest_tomorow_finds_tomorrow() {
        let suggestion = suggest_keyword("tomorow", 2);
        assert_eq!(suggestion, Some("tomorrow".to_string()));
    }

    #[test]
    fn suggest_no_match_for_gibberish() {
        let suggestion = suggest_keyword("xyzzy", 2);
        assert!(suggestion.is_none());
    }

    #[test]
    fn suggest_case_insensitive() {
        let suggestion = suggest_keyword("THURSDYA", 2);
        assert_eq!(suggestion, Some("thursday".to_string()));
    }

    #[test]
    fn suggest_exact_match_excluded() {
        let suggestion = suggest_keyword("wednesday", 2);
        assert!(suggestion.is_none());
    }
}
