//! Lexer (tokenizer) for the TARDIS natural-language date parser.
//!
//! Scans input character-by-character, producing a `Vec<SpannedToken>` with
//! byte-accurate span tracking. Keywords are matched via an inline
//! `match_keyword()` function. UTF-8 multi-byte characters are handled
//! correctly in word scanning.
//!
//! Only `Token::Word(String)` carries owned data
//! (for unrecognized words used in error messages and typo suggestions).

use crate::parser::token::{
    BoundaryKind, ByteSpan, EpochPrecision, SpannedToken, TemporalUnit, Token,
};

/// Complete keyword table for the suggestion engine and iteration.
///
/// Every entry corresponds to a match arm in [`match_keyword()`]. The two
/// sources (match + array) are kept in sync manually -- adding a keyword
/// in one and not the other is a logic error caught by the
/// `keyword_list_count` test.
pub(crate) const KEYWORD_LIST: &[(&str, Token)] = &[
    // Relative keywords
    ("now", Token::Now),
    ("today", Token::Today),
    ("tomorrow", Token::Tomorrow),
    ("yesterday", Token::Yesterday),
    ("overmorrow", Token::Overmorrow),
    // Direction modifiers
    ("next", Token::Next),
    ("last", Token::Last),
    ("this", Token::This),
    ("in", Token::In),
    ("ago", Token::Ago),
    ("from", Token::From),
    // Verbal arithmetic keywords
    ("after", Token::After),
    ("before", Token::Before),
    // Articles
    ("a", Token::A),
    ("an", Token::An),
    // Connectors
    ("at", Token::At),
    ("and", Token::And),
    // AM/PM meridiem indicators
    ("am", Token::Am),
    ("pm", Token::Pm),
    // Weekdays (full)
    ("monday", Token::Weekday(jiff::civil::Weekday::Monday)),
    ("tuesday", Token::Weekday(jiff::civil::Weekday::Tuesday)),
    ("wednesday", Token::Weekday(jiff::civil::Weekday::Wednesday)),
    ("thursday", Token::Weekday(jiff::civil::Weekday::Thursday)),
    ("friday", Token::Weekday(jiff::civil::Weekday::Friday)),
    ("saturday", Token::Weekday(jiff::civil::Weekday::Saturday)),
    ("sunday", Token::Weekday(jiff::civil::Weekday::Sunday)),
    // Weekdays (abbreviated)
    ("mon", Token::Weekday(jiff::civil::Weekday::Monday)),
    ("tue", Token::Weekday(jiff::civil::Weekday::Tuesday)),
    ("wed", Token::Weekday(jiff::civil::Weekday::Wednesday)),
    ("thu", Token::Weekday(jiff::civil::Weekday::Thursday)),
    ("fri", Token::Weekday(jiff::civil::Weekday::Friday)),
    ("sat", Token::Weekday(jiff::civil::Weekday::Saturday)),
    ("sun", Token::Weekday(jiff::civil::Weekday::Sunday)),
    // Months (full)
    ("january", Token::Month(1)),
    ("february", Token::Month(2)),
    ("march", Token::Month(3)),
    ("april", Token::Month(4)),
    ("may", Token::Month(5)),
    ("june", Token::Month(6)),
    ("july", Token::Month(7)),
    ("august", Token::Month(8)),
    ("september", Token::Month(9)),
    ("october", Token::Month(10)),
    ("november", Token::Month(11)),
    ("december", Token::Month(12)),
    // Months (abbreviated)
    ("jan", Token::Month(1)),
    ("feb", Token::Month(2)),
    ("mar", Token::Month(3)),
    ("apr", Token::Month(4)),
    ("jun", Token::Month(6)),
    ("jul", Token::Month(7)),
    ("aug", Token::Month(8)),
    ("sep", Token::Month(9)),
    ("oct", Token::Month(10)),
    ("nov", Token::Month(11)),
    ("dec", Token::Month(12)),
    // Temporal units (singular + plural + abbreviations)
    ("year", Token::Unit(TemporalUnit::Year)),
    ("years", Token::Unit(TemporalUnit::Year)),
    ("month", Token::Unit(TemporalUnit::Month)),
    ("months", Token::Unit(TemporalUnit::Month)),
    ("week", Token::Unit(TemporalUnit::Week)),
    ("weeks", Token::Unit(TemporalUnit::Week)),
    ("day", Token::Unit(TemporalUnit::Day)),
    ("days", Token::Unit(TemporalUnit::Day)),
    ("hour", Token::Unit(TemporalUnit::Hour)),
    ("hours", Token::Unit(TemporalUnit::Hour)),
    ("minute", Token::Unit(TemporalUnit::Minute)),
    ("minutes", Token::Unit(TemporalUnit::Minute)),
    ("min", Token::Unit(TemporalUnit::Minute)),
    ("mins", Token::Unit(TemporalUnit::Minute)),
    ("second", Token::Unit(TemporalUnit::Second)),
    ("seconds", Token::Unit(TemporalUnit::Second)),
    ("sec", Token::Unit(TemporalUnit::Second)),
    ("secs", Token::Unit(TemporalUnit::Second)),
    // Abbreviated duration units
    ("h", Token::Unit(TemporalUnit::Hour)),
    ("hr", Token::Unit(TemporalUnit::Hour)),
    ("hrs", Token::Unit(TemporalUnit::Hour)),
    ("d", Token::Unit(TemporalUnit::Day)),
    ("w", Token::Unit(TemporalUnit::Week)),
    ("wk", Token::Unit(TemporalUnit::Week)),
    ("wks", Token::Unit(TemporalUnit::Week)),
    ("y", Token::Unit(TemporalUnit::Year)),
    ("yr", Token::Unit(TemporalUnit::Year)),
    ("yrs", Token::Unit(TemporalUnit::Year)),
    ("mo", Token::Unit(TemporalUnit::Month)),
    ("mos", Token::Unit(TemporalUnit::Month)),
    // TaskWarrior boundary keywords -- current period (12)
    ("sod", Token::Boundary(BoundaryKind::Sod)),
    ("eod", Token::Boundary(BoundaryKind::Eod)),
    ("sow", Token::Boundary(BoundaryKind::Sow)),
    ("eow", Token::Boundary(BoundaryKind::Eow)),
    ("soww", Token::Boundary(BoundaryKind::Soww)),
    ("eoww", Token::Boundary(BoundaryKind::Eoww)),
    ("som", Token::Boundary(BoundaryKind::Som)),
    ("eom", Token::Boundary(BoundaryKind::Eom)),
    ("soq", Token::Boundary(BoundaryKind::Soq)),
    ("eoq", Token::Boundary(BoundaryKind::Eoq)),
    ("soy", Token::Boundary(BoundaryKind::Soy)),
    ("eoy", Token::Boundary(BoundaryKind::Eoy)),
    // TaskWarrior boundary keywords -- previous period (10)
    ("sopd", Token::Boundary(BoundaryKind::Sopd)),
    ("eopd", Token::Boundary(BoundaryKind::Eopd)),
    ("sopw", Token::Boundary(BoundaryKind::Sopw)),
    ("eopw", Token::Boundary(BoundaryKind::Eopw)),
    ("sopm", Token::Boundary(BoundaryKind::Sopm)),
    ("eopm", Token::Boundary(BoundaryKind::Eopm)),
    ("sopq", Token::Boundary(BoundaryKind::Sopq)),
    ("eopq", Token::Boundary(BoundaryKind::Eopq)),
    ("sopy", Token::Boundary(BoundaryKind::Sopy)),
    ("eopy", Token::Boundary(BoundaryKind::Eopy)),
    // TaskWarrior boundary keywords -- next period (10)
    ("sond", Token::Boundary(BoundaryKind::Sond)),
    ("eond", Token::Boundary(BoundaryKind::Eond)),
    ("sonw", Token::Boundary(BoundaryKind::Sonw)),
    ("eonw", Token::Boundary(BoundaryKind::Eonw)),
    ("sonm", Token::Boundary(BoundaryKind::Sonm)),
    ("eonm", Token::Boundary(BoundaryKind::Eonm)),
    ("sonq", Token::Boundary(BoundaryKind::Sonq)),
    ("eonq", Token::Boundary(BoundaryKind::Eonq)),
    ("sony", Token::Boundary(BoundaryKind::Sony)),
    ("eony", Token::Boundary(BoundaryKind::Eony)),
];

/// Match a lowercased word against the known keyword table.
///
/// Returns `Some(Token)` for a recognized keyword, `None` otherwise.
/// The match arms are grouped by semantic category and use `|` for synonyms.
fn match_keyword(word: &str) -> Option<Token> {
    match word {
        // Relative keywords
        "now" => Some(Token::Now),
        "today" => Some(Token::Today),
        "tomorrow" => Some(Token::Tomorrow),
        "yesterday" => Some(Token::Yesterday),
        "overmorrow" => Some(Token::Overmorrow),
        // Direction modifiers
        "next" => Some(Token::Next),
        "last" => Some(Token::Last),
        "this" => Some(Token::This),
        "in" => Some(Token::In),
        "ago" => Some(Token::Ago),
        "from" => Some(Token::From),
        // Verbal arithmetic
        "after" => Some(Token::After),
        "before" => Some(Token::Before),
        // Articles
        "a" => Some(Token::A),
        "an" => Some(Token::An),
        // Connectors
        "at" => Some(Token::At),
        "and" => Some(Token::And),
        // AM/PM meridiem indicators
        "am" => Some(Token::Am),
        "pm" => Some(Token::Pm),
        // Weekdays (full + abbreviated)
        "monday" | "mon" => Some(Token::Weekday(jiff::civil::Weekday::Monday)),
        "tuesday" | "tue" => Some(Token::Weekday(jiff::civil::Weekday::Tuesday)),
        "wednesday" | "wed" => Some(Token::Weekday(jiff::civil::Weekday::Wednesday)),
        "thursday" | "thu" => Some(Token::Weekday(jiff::civil::Weekday::Thursday)),
        "friday" | "fri" => Some(Token::Weekday(jiff::civil::Weekday::Friday)),
        "saturday" | "sat" => Some(Token::Weekday(jiff::civil::Weekday::Saturday)),
        "sunday" | "sun" => Some(Token::Weekday(jiff::civil::Weekday::Sunday)),
        // Months (full + abbreviated)
        "january" | "jan" => Some(Token::Month(1)),
        "february" | "feb" => Some(Token::Month(2)),
        "march" | "mar" => Some(Token::Month(3)),
        "april" | "apr" => Some(Token::Month(4)),
        "may" => Some(Token::Month(5)),
        "june" | "jun" => Some(Token::Month(6)),
        "july" | "jul" => Some(Token::Month(7)),
        "august" | "aug" => Some(Token::Month(8)),
        "september" | "sep" => Some(Token::Month(9)),
        "october" | "oct" => Some(Token::Month(10)),
        "november" | "nov" => Some(Token::Month(11)),
        "december" | "dec" => Some(Token::Month(12)),
        // Temporal units (singular + plural + abbreviations)
        "year" | "years" | "y" | "yr" | "yrs" => Some(Token::Unit(TemporalUnit::Year)),
        "month" | "months" | "mo" | "mos" => Some(Token::Unit(TemporalUnit::Month)),
        "week" | "weeks" | "w" | "wk" | "wks" => Some(Token::Unit(TemporalUnit::Week)),
        "day" | "days" | "d" => Some(Token::Unit(TemporalUnit::Day)),
        "hour" | "hours" | "h" | "hr" | "hrs" => Some(Token::Unit(TemporalUnit::Hour)),
        "minute" | "minutes" | "min" | "mins" => Some(Token::Unit(TemporalUnit::Minute)),
        "second" | "seconds" | "sec" | "secs" => Some(Token::Unit(TemporalUnit::Second)),
        // TaskWarrior boundary keywords -- current period
        "sod" => Some(Token::Boundary(BoundaryKind::Sod)),
        "eod" => Some(Token::Boundary(BoundaryKind::Eod)),
        "sow" => Some(Token::Boundary(BoundaryKind::Sow)),
        "eow" => Some(Token::Boundary(BoundaryKind::Eow)),
        "soww" => Some(Token::Boundary(BoundaryKind::Soww)),
        "eoww" => Some(Token::Boundary(BoundaryKind::Eoww)),
        "som" => Some(Token::Boundary(BoundaryKind::Som)),
        "eom" => Some(Token::Boundary(BoundaryKind::Eom)),
        "soq" => Some(Token::Boundary(BoundaryKind::Soq)),
        "eoq" => Some(Token::Boundary(BoundaryKind::Eoq)),
        "soy" => Some(Token::Boundary(BoundaryKind::Soy)),
        "eoy" => Some(Token::Boundary(BoundaryKind::Eoy)),
        // TaskWarrior boundary keywords -- previous period
        "sopd" => Some(Token::Boundary(BoundaryKind::Sopd)),
        "eopd" => Some(Token::Boundary(BoundaryKind::Eopd)),
        "sopw" => Some(Token::Boundary(BoundaryKind::Sopw)),
        "eopw" => Some(Token::Boundary(BoundaryKind::Eopw)),
        "sopm" => Some(Token::Boundary(BoundaryKind::Sopm)),
        "eopm" => Some(Token::Boundary(BoundaryKind::Eopm)),
        "sopq" => Some(Token::Boundary(BoundaryKind::Sopq)),
        "eopq" => Some(Token::Boundary(BoundaryKind::Eopq)),
        "sopy" => Some(Token::Boundary(BoundaryKind::Sopy)),
        "eopy" => Some(Token::Boundary(BoundaryKind::Eopy)),
        // TaskWarrior boundary keywords -- next period
        "sond" => Some(Token::Boundary(BoundaryKind::Sond)),
        "eond" => Some(Token::Boundary(BoundaryKind::Eond)),
        "sonw" => Some(Token::Boundary(BoundaryKind::Sonw)),
        "eonw" => Some(Token::Boundary(BoundaryKind::Eonw)),
        "sonm" => Some(Token::Boundary(BoundaryKind::Sonm)),
        "eonm" => Some(Token::Boundary(BoundaryKind::Eonm)),
        "sonq" => Some(Token::Boundary(BoundaryKind::Sonq)),
        "eonq" => Some(Token::Boundary(BoundaryKind::Eonq)),
        "sony" => Some(Token::Boundary(BoundaryKind::Sony)),
        "eony" => Some(Token::Boundary(BoundaryKind::Eony)),
        _ => None,
    }
}

/// Tokenize the input string into a sequence of spanned tokens.
///
/// - Whitespace is consumed but not emitted.
/// - Commas are consumed but not emitted (optional separators in compound durations).
/// - Keywords are matched via [`match_keyword()`].
/// - Unrecognized alpha words are captured as `Token::Word` with original casing.
/// - Epoch suffixes (`ms`, `us`, `ns`, `s`) are detected immediately after numbers
///   when not separated by whitespace and not followed by more alpha chars.
/// - UTF-8 multi-byte alphabetic characters are handled in word scanning.
pub(crate) fn tokenize(input: &str) -> Vec<SpannedToken> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut pos: usize = 0;
    let mut tokens = Vec::new();

    while pos < len {
        let b = bytes[pos];

        // Skip whitespace
        if b.is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        // Skip commas (optional separators in compound durations)
        if b == b',' {
            pos += 1;
            continue;
        }

        // Single-character separators
        match b {
            b':' => {
                tokens.push(SpannedToken {
                    kind: Token::Colon,
                    span: ByteSpan {
                        start: pos,
                        end: pos + 1,
                    },
                });
                pos += 1;
                continue;
            }
            b'/' => {
                tokens.push(SpannedToken {
                    kind: Token::Slash,
                    span: ByteSpan {
                        start: pos,
                        end: pos + 1,
                    },
                });
                pos += 1;
                continue;
            }
            b'@' => {
                tokens.push(SpannedToken {
                    kind: Token::AtSign,
                    span: ByteSpan {
                        start: pos,
                        end: pos + 1,
                    },
                });
                pos += 1;
                continue;
            }
            b'+' => {
                // "+" is a positive number sign ONLY after @ (epoch like @+1735689600).
                // At start of input, emit Plus operator so "+3h" tokenizes as
                // [Plus, Number(3), Unit(Hour)].
                let is_sign_position = tokens
                    .last()
                    .is_some_and(|t| matches!(t.kind, Token::AtSign));
                if pos + 1 < len && bytes[pos + 1].is_ascii_digit() && is_sign_position {
                    // Parse as positive number (skip the '+')
                    let start = pos;
                    pos += 1; // skip '+'
                    while pos < len && bytes[pos].is_ascii_digit() {
                        pos += 1;
                    }
                    let num_str = &input[start + 1..pos];
                    let value: i64 = num_str.parse().unwrap_or(0);
                    tokens.push(SpannedToken {
                        kind: Token::Number(value),
                        span: ByteSpan { start, end: pos },
                    });
                    try_epoch_suffix(input, &mut pos, &mut tokens);
                    continue;
                }
                // Otherwise emit Plus operator
                tokens.push(SpannedToken {
                    kind: Token::Plus,
                    span: ByteSpan {
                        start: pos,
                        end: pos + 1,
                    },
                });
                pos += 1;
                continue;
            }
            b'-' => {
                // "-" is a negative number sign ONLY after @ (epoch like @-1735689600).
                // At start of input, emit Dash so "-3h" tokenizes as
                // [Dash, Number(3), Unit(Hour)].
                let is_sign_position = tokens
                    .last()
                    .is_some_and(|t| matches!(t.kind, Token::AtSign));
                if pos + 1 < len && bytes[pos + 1].is_ascii_digit() && is_sign_position {
                    let start = pos;
                    pos += 1; // skip the '-'
                    let num_start = pos;
                    while pos < len && bytes[pos].is_ascii_digit() {
                        pos += 1;
                    }
                    let num_str = &input[num_start..pos];
                    // Safe: we only consumed ASCII digits
                    let value: i64 = num_str.parse().unwrap_or(0);
                    tokens.push(SpannedToken {
                        kind: Token::Number(-value),
                        span: ByteSpan { start, end: pos },
                    });
                    // Check for epoch suffix immediately after number
                    try_epoch_suffix(input, &mut pos, &mut tokens);
                    continue;
                }
                // Otherwise emit Dash (separator in ISO dates, standalone minus)
                tokens.push(SpannedToken {
                    kind: Token::Dash,
                    span: ByteSpan {
                        start: pos,
                        end: pos + 1,
                    },
                });
                pos += 1;
                continue;
            }
            _ => {}
        }

        // Digits: parse number
        if b.is_ascii_digit() {
            let start = pos;
            while pos < len && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            let num_str = &input[start..pos];
            let value: i64 = num_str.parse().unwrap_or(0);
            tokens.push(SpannedToken {
                kind: Token::Number(value),
                span: ByteSpan { start, end: pos },
            });
            // Check for epoch suffix immediately after number
            try_epoch_suffix(input, &mut pos, &mut tokens);
            continue;
        }

        // Alpha characters (ASCII or UTF-8 multi-byte): consume word, match keyword
        if b.is_ascii_alphabetic() || (b & 0x80 != 0) {
            let start = pos;

            // Scan word: ASCII alpha or multi-byte UTF-8 alphabetic chars
            while pos < len {
                let b = bytes[pos];
                if b.is_ascii_alphabetic() {
                    pos += 1;
                } else if b & 0x80 != 0 {
                    // Multi-byte UTF-8: decode char, check if alphabetic
                    if let Some(ch) = input[pos..].chars().next() {
                        if ch.is_alphabetic() {
                            pos += ch.len_utf8();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Special handling for Q+digit patterns (Q1, Q2, Q3, Q4)
            let original = &input[start..pos];
            let normalized = original.to_ascii_lowercase();

            if normalized == "q" && pos < len && bytes[pos].is_ascii_digit() {
                let digit_start = pos;
                while pos < len && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
                let combined_lower = format!("q{}", &input[digit_start..pos]);
                if let Some(kind) = match_quarter(&combined_lower) {
                    tokens.push(SpannedToken {
                        kind,
                        span: ByteSpan { start, end: pos },
                    });
                    continue;
                }
                // Not a valid quarter -- restore pos and treat "q" as word
                pos = digit_start;
            }

            let kind = match match_keyword(&normalized) {
                Some(token) => token,
                None => Token::Word(original.to_string()),
            };
            tokens.push(SpannedToken {
                kind,
                span: ByteSpan { start, end: pos },
            });
            continue;
        }

        // Any other character: emit as Word
        let start = pos;
        pos += 1;
        tokens.push(SpannedToken {
            kind: Token::Word(input[start..pos].to_string()),
            span: ByteSpan { start, end: pos },
        });
    }

    tokens
}

/// Try to consume an epoch suffix immediately after a number token.
///
/// Recognized suffixes: `ms`, `us`, `ns`, `s` (only when not followed by more
/// alpha characters, to distinguish `"s"` suffix from a word like `"seconds"`).
fn try_epoch_suffix(input: &str, pos: &mut usize, tokens: &mut Vec<SpannedToken>) {
    let bytes = input.as_bytes();
    let len = bytes.len();

    if *pos >= len || !bytes[*pos].is_ascii_alphabetic() {
        return;
    }

    // Try two-char suffixes first: ms, us, ns
    if *pos + 1 < len {
        let two = &input[*pos..*pos + 2];
        let two_lower = two.to_ascii_lowercase();
        let precision = match two_lower.as_str() {
            "ms" => Some(EpochPrecision::Milliseconds),
            "us" => Some(EpochPrecision::Microseconds),
            "ns" => Some(EpochPrecision::Nanoseconds),
            _ => None,
        };
        if let Some(p) = precision {
            // Ensure not followed by more alpha chars
            if *pos + 2 >= len || !bytes[*pos + 2].is_ascii_alphabetic() {
                let start = *pos;
                *pos += 2;
                tokens.push(SpannedToken {
                    kind: Token::EpochSuffix(p),
                    span: ByteSpan { start, end: *pos },
                });
                return;
            }
        }
    }

    // Try single-char suffix: s
    if bytes[*pos].eq_ignore_ascii_case(&b's') {
        // Ensure not followed by more alpha chars
        if *pos + 1 >= len || !bytes[*pos + 1].is_ascii_alphabetic() {
            let start = *pos;
            *pos += 1;
            tokens.push(SpannedToken {
                kind: Token::EpochSuffix(EpochPrecision::Seconds),
                span: ByteSpan { start, end: *pos },
            });
        }
    }
}

/// Try to match a quarter pattern like "q1", "q2", "q3", "q4".
fn match_quarter(lower: &str) -> Option<Token> {
    match lower {
        "q1" => Some(Token::Quarter(1)),
        "q2" => Some(Token::Quarter(2)),
        "q3" => Some(Token::Quarter(3)),
        "q4" => Some(Token::Quarter(4)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use jiff::civil::Weekday;

    /// Helper to extract just the token kinds from a tokenize result.
    fn kinds(input: &str) -> Vec<Token> {
        tokenize(input).into_iter().map(|st| st.kind).collect()
    }

    // ── match_keyword tests ───────────────────────────────────────

    #[test]
    fn match_keyword_known() {
        assert_eq!(match_keyword("tomorrow"), Some(Token::Tomorrow));
        assert_eq!(
            match_keyword("eod"),
            Some(Token::Boundary(BoundaryKind::Eod))
        );
        assert_eq!(
            match_keyword("mon"),
            Some(Token::Weekday(jiff::civil::Weekday::Monday))
        );
    }

    #[test]
    fn match_keyword_unknown() {
        assert_eq!(match_keyword("xyzzy"), None);
        assert_eq!(match_keyword("thursdya"), None);
    }

    #[test]
    fn keyword_list_count() {
        assert_eq!(KEYWORD_LIST.len(), 118);
    }

    // ── Basic tokenize tests ──────────────────────────────────────

    #[test]
    fn simple_keyword_now() {
        assert_eq!(kinds("now"), vec![Token::Now]);
    }

    #[test]
    fn relative_with_time() {
        assert_eq!(
            kinds("today 18:30"),
            vec![
                Token::Today,
                Token::Number(18),
                Token::Colon,
                Token::Number(30),
            ]
        );
    }

    #[test]
    fn day_reference_next_friday() {
        assert_eq!(
            kinds("next friday"),
            vec![Token::Next, Token::Weekday(Weekday::Friday)]
        );
    }

    #[test]
    fn duration_past() {
        assert_eq!(
            kinds("3 hours ago"),
            vec![
                Token::Number(3),
                Token::Unit(TemporalUnit::Hour),
                Token::Ago
            ]
        );
    }

    #[test]
    fn duration_future() {
        assert_eq!(
            kinds("in 3 days"),
            vec![Token::In, Token::Number(3), Token::Unit(TemporalUnit::Day)]
        );
    }

    #[test]
    fn article_a_week_ago() {
        assert_eq!(
            kinds("a week ago"),
            vec![Token::A, Token::Unit(TemporalUnit::Week), Token::Ago]
        );
    }

    #[test]
    fn compound_duration() {
        assert_eq!(
            kinds("2 hours and 5 minutes ago"),
            vec![
                Token::Number(2),
                Token::Unit(TemporalUnit::Hour),
                Token::And,
                Token::Number(5),
                Token::Unit(TemporalUnit::Minute),
                Token::Ago,
            ]
        );
    }

    #[test]
    fn iso_date() {
        assert_eq!(
            kinds("2025-01-01"),
            vec![
                Token::Number(2025),
                Token::Dash,
                Token::Number(1),
                Token::Dash,
                Token::Number(1),
            ]
        );
    }

    #[test]
    fn epoch_plain() {
        assert_eq!(
            kinds("@1735689600"),
            vec![Token::AtSign, Token::Number(1_735_689_600)]
        );
    }

    #[test]
    fn epoch_with_ms_suffix() {
        assert_eq!(
            kinds("@1735689600ms"),
            vec![
                Token::AtSign,
                Token::Number(1_735_689_600),
                Token::EpochSuffix(EpochPrecision::Milliseconds),
            ]
        );
    }

    #[test]
    fn negative_epoch() {
        assert_eq!(kinds("@-86400"), vec![Token::AtSign, Token::Number(-86400)]);
    }

    #[test]
    fn case_insensitive_keywords() {
        assert_eq!(
            kinds("NEXT Friday"),
            vec![Token::Next, Token::Weekday(Weekday::Friday)]
        );
    }

    #[test]
    fn unknown_word_preserves_case() {
        assert_eq!(kinds("thursdya"), vec![Token::Word("thursdya".to_string())]);
    }

    #[test]
    fn empty_input() {
        assert_eq!(kinds(""), Vec::<Token>::new());
    }

    #[test]
    fn whitespace_only() {
        assert_eq!(kinds("   "), Vec::<Token>::new());
    }

    #[test]
    fn spans_are_correct() {
        let tokens = tokenize("next friday");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 4 });
        assert_eq!(tokens[1].span, ByteSpan { start: 5, end: 11 });
    }

    #[test]
    fn month_names() {
        assert_eq!(kinds("january"), vec![Token::Month(1)]);
    }

    #[test]
    fn abbreviated_weekday() {
        assert_eq!(kinds("mon"), vec![Token::Weekday(Weekday::Monday)]);
    }

    #[test]
    fn from_keyword() {
        assert_eq!(kinds("from"), vec![Token::From]);
    }

    #[test]
    fn comma_separator_ignored() {
        assert_eq!(
            kinds("2 hours, 5 minutes"),
            vec![
                Token::Number(2),
                Token::Unit(TemporalUnit::Hour),
                Token::Number(5),
                Token::Unit(TemporalUnit::Minute),
            ]
        );
    }

    // Additional tests beyond the minimum 20

    #[test]
    fn epoch_suffix_us() {
        assert_eq!(
            kinds("@1735689600us"),
            vec![
                Token::AtSign,
                Token::Number(1_735_689_600),
                Token::EpochSuffix(EpochPrecision::Microseconds),
            ]
        );
    }

    #[test]
    fn epoch_suffix_ns() {
        assert_eq!(
            kinds("@1735689600ns"),
            vec![
                Token::AtSign,
                Token::Number(1_735_689_600),
                Token::EpochSuffix(EpochPrecision::Nanoseconds),
            ]
        );
    }

    #[test]
    fn epoch_suffix_s() {
        assert_eq!(
            kinds("@1735689600s"),
            vec![
                Token::AtSign,
                Token::Number(1_735_689_600),
                Token::EpochSuffix(EpochPrecision::Seconds),
            ]
        );
    }

    #[test]
    fn number_followed_by_word_not_suffix() {
        // "5 seconds" -- "seconds" is separated by whitespace so NOT a suffix
        assert_eq!(
            kinds("5 seconds"),
            vec![Token::Number(5), Token::Unit(TemporalUnit::Second)]
        );
    }

    #[test]
    fn number_immediately_followed_by_non_suffix_word() {
        // "5abc" -- "abc" is not a recognized epoch suffix. The suffix check sees "ab"
        // (not ms/us/ns) then "a" (not "s"), so nothing is consumed as suffix. The main
        // loop then consumes "abc" as an alpha word -> no keyword match -> Word("abc").
        assert_eq!(
            kinds("5abc"),
            vec![Token::Number(5), Token::Word("abc".to_string())]
        );
    }

    #[test]
    fn number_directly_followed_by_non_suffix_alpha() {
        // "10seconds" -- "se" is not ms/us/ns, and "s" IS "s" but followed by "econds"
        // (more alpha), so the suffix is NOT consumed. The word "seconds" is then parsed
        // as Unit(Second).
        assert_eq!(
            kinds("10seconds"),
            vec![Token::Number(10), Token::Unit(TemporalUnit::Second)]
        );
    }

    #[test]
    fn all_weekday_abbreviations() {
        assert_eq!(kinds("tue"), vec![Token::Weekday(Weekday::Tuesday)]);
        assert_eq!(kinds("wed"), vec![Token::Weekday(Weekday::Wednesday)]);
        assert_eq!(kinds("thu"), vec![Token::Weekday(Weekday::Thursday)]);
        assert_eq!(kinds("fri"), vec![Token::Weekday(Weekday::Friday)]);
        assert_eq!(kinds("sat"), vec![Token::Weekday(Weekday::Saturday)]);
        assert_eq!(kinds("sun"), vec![Token::Weekday(Weekday::Sunday)]);
    }

    #[test]
    fn all_month_names() {
        assert_eq!(kinds("february"), vec![Token::Month(2)]);
        assert_eq!(kinds("mar"), vec![Token::Month(3)]);
        assert_eq!(kinds("apr"), vec![Token::Month(4)]);
        assert_eq!(kinds("may"), vec![Token::Month(5)]);
        assert_eq!(kinds("jun"), vec![Token::Month(6)]);
        assert_eq!(kinds("jul"), vec![Token::Month(7)]);
        assert_eq!(kinds("aug"), vec![Token::Month(8)]);
        assert_eq!(kinds("sep"), vec![Token::Month(9)]);
        assert_eq!(kinds("oct"), vec![Token::Month(10)]);
        assert_eq!(kinds("nov"), vec![Token::Month(11)]);
        assert_eq!(kinds("dec"), vec![Token::Month(12)]);
    }

    #[test]
    fn all_temporal_units() {
        assert_eq!(kinds("year"), vec![Token::Unit(TemporalUnit::Year)]);
        assert_eq!(kinds("years"), vec![Token::Unit(TemporalUnit::Year)]);
        assert_eq!(kinds("month"), vec![Token::Unit(TemporalUnit::Month)]);
        assert_eq!(kinds("months"), vec![Token::Unit(TemporalUnit::Month)]);
        assert_eq!(kinds("week"), vec![Token::Unit(TemporalUnit::Week)]);
        assert_eq!(kinds("weeks"), vec![Token::Unit(TemporalUnit::Week)]);
        assert_eq!(kinds("day"), vec![Token::Unit(TemporalUnit::Day)]);
        assert_eq!(kinds("days"), vec![Token::Unit(TemporalUnit::Day)]);
        assert_eq!(kinds("hour"), vec![Token::Unit(TemporalUnit::Hour)]);
        assert_eq!(kinds("hours"), vec![Token::Unit(TemporalUnit::Hour)]);
        assert_eq!(kinds("minute"), vec![Token::Unit(TemporalUnit::Minute)]);
        assert_eq!(kinds("minutes"), vec![Token::Unit(TemporalUnit::Minute)]);
        assert_eq!(kinds("min"), vec![Token::Unit(TemporalUnit::Minute)]);
        assert_eq!(kinds("mins"), vec![Token::Unit(TemporalUnit::Minute)]);
        assert_eq!(kinds("second"), vec![Token::Unit(TemporalUnit::Second)]);
        assert_eq!(kinds("seconds"), vec![Token::Unit(TemporalUnit::Second)]);
        assert_eq!(kinds("sec"), vec![Token::Unit(TemporalUnit::Second)]);
        assert_eq!(kinds("secs"), vec![Token::Unit(TemporalUnit::Second)]);
    }

    #[test]
    fn all_relative_keywords() {
        assert_eq!(kinds("now"), vec![Token::Now]);
        assert_eq!(kinds("today"), vec![Token::Today]);
        assert_eq!(kinds("tomorrow"), vec![Token::Tomorrow]);
        assert_eq!(kinds("yesterday"), vec![Token::Yesterday]);
        assert_eq!(kinds("overmorrow"), vec![Token::Overmorrow]);
    }

    #[test]
    fn all_direction_modifiers() {
        assert_eq!(kinds("next"), vec![Token::Next]);
        assert_eq!(kinds("last"), vec![Token::Last]);
        assert_eq!(kinds("this"), vec![Token::This]);
        assert_eq!(kinds("in"), vec![Token::In]);
        assert_eq!(kinds("ago"), vec![Token::Ago]);
    }

    #[test]
    fn articles_and_connectors() {
        assert_eq!(kinds("a"), vec![Token::A]);
        assert_eq!(kinds("an"), vec![Token::An]);
        assert_eq!(kinds("at"), vec![Token::At]);
        assert_eq!(kinds("and"), vec![Token::And]);
    }

    #[test]
    fn separators() {
        assert_eq!(kinds(":"), vec![Token::Colon]);
        assert_eq!(kinds("/"), vec![Token::Slash]);
        assert_eq!(kinds("@"), vec![Token::AtSign]);
    }

    #[test]
    fn dash_without_following_digit() {
        assert_eq!(kinds("-"), vec![Token::Dash]);
        assert_eq!(kinds("- "), vec![Token::Dash]);
    }

    #[test]
    fn unknown_single_char() {
        // Characters not matching any rule become Word tokens
        assert_eq!(kinds("!"), vec![Token::Word("!".to_string())]);
        assert_eq!(kinds("#"), vec![Token::Word("#".to_string())]);
    }

    #[test]
    fn complex_expression_an_hour_ago() {
        assert_eq!(
            kinds("an hour ago"),
            vec![Token::An, Token::Unit(TemporalUnit::Hour), Token::Ago]
        );
    }

    #[test]
    fn mixed_case_all_caps() {
        assert_eq!(kinds("YESTERDAY"), vec![Token::Yesterday]);
    }

    #[test]
    fn mixed_case_camel() {
        assert_eq!(kinds("Tomorrow"), vec![Token::Tomorrow]);
    }

    #[test]
    fn multiple_spaces_between_tokens() {
        assert_eq!(
            kinds("next    friday"),
            vec![Token::Next, Token::Weekday(Weekday::Friday)]
        );
    }

    #[test]
    fn epoch_suffix_case_insensitive() {
        assert_eq!(
            kinds("@100MS"),
            vec![
                Token::AtSign,
                Token::Number(100),
                Token::EpochSuffix(EpochPrecision::Milliseconds),
            ]
        );
    }

    #[test]
    fn span_tracking_multiword() {
        let tokens = tokenize("3 hours ago");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 1 });
        assert_eq!(tokens[1].span, ByteSpan { start: 2, end: 7 });
        assert_eq!(tokens[2].span, ByteSpan { start: 8, end: 11 });
    }

    #[test]
    fn span_tracking_iso_date() {
        let tokens = tokenize("2025-01-15");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 4 }); // 2025
        assert_eq!(tokens[1].span, ByteSpan { start: 4, end: 5 }); // -
        assert_eq!(tokens[2].span, ByteSpan { start: 5, end: 7 }); // 01
        assert_eq!(tokens[3].span, ByteSpan { start: 7, end: 8 }); // -
        assert_eq!(tokens[4].span, ByteSpan { start: 8, end: 10 }); // 15
    }

    #[test]
    fn negative_number_at_start_emits_dash_and_number() {
        // After D-07 fix: "-42" at start emits Dash+Number, not Number(-42)
        let tokens = tokenize("-42");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, Token::Dash);
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 1 });
        assert_eq!(tokens[1].kind, Token::Number(42));
        assert_eq!(tokens[1].span, ByteSpan { start: 1, end: 3 });
    }

    #[test]
    fn epoch_suffix_span() {
        let tokens = tokenize("100ms");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, Token::Number(100));
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 3 });
        assert_eq!(
            tokens[1].kind,
            Token::EpochSuffix(EpochPrecision::Milliseconds)
        );
        assert_eq!(tokens[1].span, ByteSpan { start: 3, end: 5 });
    }

    // ── Phase 3: Arithmetic and range token tests ────────────────

    #[test]
    fn tomorrow_plus_3_hours_tokenizes() {
        assert_eq!(
            kinds("tomorrow + 3 hours"),
            vec![
                Token::Tomorrow,
                Token::Plus,
                Token::Number(3),
                Token::Unit(TemporalUnit::Hour),
            ]
        );
    }

    #[test]
    fn now_minus_30_minutes_tokenizes() {
        // Dash is reused for minus since context resolves
        assert_eq!(
            kinds("now - 30 minutes"),
            vec![
                Token::Now,
                Token::Dash,
                Token::Number(30),
                Token::Unit(TemporalUnit::Minute),
            ]
        );
    }

    #[test]
    fn three_hours_after_tomorrow_tokenizes() {
        assert_eq!(
            kinds("3 hours after tomorrow"),
            vec![
                Token::Number(3),
                Token::Unit(TemporalUnit::Hour),
                Token::After,
                Token::Tomorrow,
            ]
        );
    }

    #[test]
    fn two_days_before_next_friday_tokenizes() {
        assert_eq!(
            kinds("2 days before next friday"),
            vec![
                Token::Number(2),
                Token::Unit(TemporalUnit::Day),
                Token::Before,
                Token::Next,
                Token::Weekday(Weekday::Friday),
            ]
        );
    }

    #[test]
    fn plus_5_at_start_emits_operator_and_number() {
        // After D-07 fix: "+5" at start emits Plus+Number, not Number(5)
        assert_eq!(kinds("+5"), vec![Token::Plus, Token::Number(5)]);
    }

    #[test]
    fn plus_after_number_is_operator() {
        // "3 + 5" -- the plus after number IS an operator
        assert_eq!(
            kinds("3 + 5"),
            vec![Token::Number(3), Token::Plus, Token::Number(5)]
        );
    }

    #[test]
    fn quarter_q1_tokenizes() {
        assert_eq!(kinds("Q1"), vec![Token::Quarter(1)]);
    }

    #[test]
    fn quarter_q3_2025_tokenizes() {
        assert_eq!(
            kinds("Q3 2025"),
            vec![Token::Quarter(3), Token::Number(2025)]
        );
    }

    #[test]
    fn quarter_case_insensitive() {
        assert_eq!(kinds("q2"), vec![Token::Quarter(2)]);
        assert_eq!(kinds("q4"), vec![Token::Quarter(4)]);
    }

    #[test]
    fn after_keyword() {
        assert_eq!(kinds("after"), vec![Token::After]);
    }

    #[test]
    fn before_keyword() {
        assert_eq!(kinds("before"), vec![Token::Before]);
    }

    // ── UTF-8 multi-byte character tests ────────────────────────

    #[test]
    fn utf8_accented_word_scans_as_single_token() {
        // "amanha" with tilde should scan as one word token
        let tokens = tokenize("amanh\u{00e3}");
        assert_eq!(tokens.len(), 1);
        // EN keywords don't include accented words, so it becomes a Word
        match &tokens[0].kind {
            Token::Word(w) => assert_eq!(w, "amanh\u{00e3}"),
            _ => panic!("expected Word token for unrecognized accented word"),
        }
    }

    #[test]
    fn utf8_span_tracking_correct() {
        // "amanha" = 6 bytes: a(1) m(1) a(1) n(1) h(1) a-tilde(2 bytes: 0xC3 0xA3)
        let tokens = tokenize("amanh\u{00e3}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 7 }); // 7 bytes total
    }

    #[test]
    fn utf8_mixed_with_ascii_tokens() {
        // "amanh\u{00e3} 3 days" -- accented word followed by ASCII tokens
        assert_eq!(
            kinds("amanh\u{00e3} 3 days"),
            vec![
                Token::Word("amanh\u{00e3}".to_string()),
                Token::Number(3),
                Token::Unit(TemporalUnit::Day),
            ]
        );
    }

    // ── Phase 8: Sign-position fix and boundary keyword tests ──

    #[test]
    fn test_plus_at_start_emits_operator() {
        assert_eq!(
            kinds("+3h"),
            vec![
                Token::Plus,
                Token::Number(3),
                Token::Unit(TemporalUnit::Hour)
            ]
        );
    }

    #[test]
    fn test_dash_at_start_emits_operator() {
        assert_eq!(
            kinds("-1d"),
            vec![
                Token::Dash,
                Token::Number(1),
                Token::Unit(TemporalUnit::Day)
            ]
        );
    }

    #[test]
    fn test_epoch_plus_still_sign() {
        assert_eq!(
            kinds("@+1735689600"),
            vec![Token::AtSign, Token::Number(1_735_689_600)]
        );
    }

    #[test]
    fn test_epoch_minus_still_sign() {
        assert_eq!(kinds("@-86400"), vec![Token::AtSign, Token::Number(-86400)]);
    }

    #[test]
    fn test_boundary_keyword_tokenizes() {
        assert_eq!(kinds("eod"), vec![Token::Boundary(BoundaryKind::Eod)]);
    }
}
