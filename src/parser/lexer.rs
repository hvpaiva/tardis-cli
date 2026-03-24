//! Lexer (tokenizer) for the TARDIS natural-language date parser.
//!
//! Scans input character-by-character, producing a `Vec<SpannedToken>` with
//! byte-accurate span tracking. Keywords are matched case-insensitively to
//! zero-heap-alloc enum variants. Only `Token::Word(String)` carries owned data
//! (for unrecognized words used in error messages and typo suggestions).


use crate::parser::token::{ByteSpan, EpochPrecision, SpannedToken, TemporalUnit, Token};

/// Tokenize the input string into a sequence of spanned tokens.
///
/// - Whitespace is consumed but not emitted.
/// - Commas are consumed but not emitted (optional separators in compound durations).
/// - Keywords are matched case-insensitively.
/// - Unrecognized alpha words are captured as `Token::Word` with original casing.
/// - Epoch suffixes (`ms`, `us`, `ns`, `s`) are detected immediately after numbers
///   when not separated by whitespace and not followed by more alpha chars.
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
            b'-' => {
                // Look ahead: if next char is a digit AND the previous token is NOT
                // a Number (to distinguish negative numbers from ISO date separators),
                // parse as negative number.
                let prev_is_number = tokens
                    .last()
                    .is_some_and(|t| matches!(t.kind, Token::Number(_)));
                if pos + 1 < len && bytes[pos + 1].is_ascii_digit() && !prev_is_number {
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
                        span: ByteSpan {
                            start,
                            end: pos,
                        },
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
                span: ByteSpan {
                    start,
                    end: pos,
                },
            });
            // Check for epoch suffix immediately after number
            try_epoch_suffix(input, &mut pos, &mut tokens);
            continue;
        }

        // Alpha characters: consume word, match keyword
        if b.is_ascii_alphabetic() {
            let start = pos;
            while pos < len && bytes[pos].is_ascii_alphabetic() {
                pos += 1;
            }
            let original = &input[start..pos];
            let lower = original.to_ascii_lowercase();
            let kind = match_keyword(&lower, original);
            tokens.push(SpannedToken {
                kind,
                span: ByteSpan {
                    start,
                    end: pos,
                },
            });
            continue;
        }

        // Any other character: emit as Word
        let start = pos;
        pos += 1;
        tokens.push(SpannedToken {
            kind: Token::Word(input[start..pos].to_string()),
            span: ByteSpan {
                start,
                end: pos,
            },
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
                    span: ByteSpan {
                        start,
                        end: *pos,
                    },
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
                span: ByteSpan {
                    start,
                    end: *pos,
                },
            });
        }
    }
}

/// Match a lowercase word against the keyword table.
///
/// Returns the corresponding `Token` variant, or `Token::Word` with original
/// casing for unrecognized words.
fn match_keyword(lower: &str, original: &str) -> Token {
    match lower {
        // Relative keywords
        "now" => Token::Now,
        "today" => Token::Today,
        "tomorrow" => Token::Tomorrow,
        "yesterday" => Token::Yesterday,
        "overmorrow" => Token::Overmorrow,

        // Direction modifiers
        "next" => Token::Next,
        "last" => Token::Last,
        "this" => Token::This,
        "in" => Token::In,
        "ago" => Token::Ago,
        "from" => Token::From,

        // Articles
        "a" => Token::A,
        "an" => Token::An,

        // Connectors
        "at" => Token::At,
        "and" => Token::And,

        // Weekdays (full and abbreviated)
        "monday" | "mon" => Token::Weekday(jiff::civil::Weekday::Monday),
        "tuesday" | "tue" => Token::Weekday(jiff::civil::Weekday::Tuesday),
        "wednesday" | "wed" => Token::Weekday(jiff::civil::Weekday::Wednesday),
        "thursday" | "thu" => Token::Weekday(jiff::civil::Weekday::Thursday),
        "friday" | "fri" => Token::Weekday(jiff::civil::Weekday::Friday),
        "saturday" | "sat" => Token::Weekday(jiff::civil::Weekday::Saturday),
        "sunday" | "sun" => Token::Weekday(jiff::civil::Weekday::Sunday),

        // Months (full and abbreviated)
        "january" | "jan" => Token::Month(1),
        "february" | "feb" => Token::Month(2),
        "march" | "mar" => Token::Month(3),
        "april" | "apr" => Token::Month(4),
        "may" => Token::Month(5),
        "june" | "jun" => Token::Month(6),
        "july" | "jul" => Token::Month(7),
        "august" | "aug" => Token::Month(8),
        "september" | "sep" => Token::Month(9),
        "october" | "oct" => Token::Month(10),
        "november" | "nov" => Token::Month(11),
        "december" | "dec" => Token::Month(12),

        // Temporal units (singular and plural)
        "year" | "years" => Token::Unit(TemporalUnit::Year),
        "month" | "months" => Token::Unit(TemporalUnit::Month),
        "week" | "weeks" => Token::Unit(TemporalUnit::Week),
        "day" | "days" => Token::Unit(TemporalUnit::Day),
        "hour" | "hours" => Token::Unit(TemporalUnit::Hour),
        "minute" | "minutes" | "min" | "mins" => Token::Unit(TemporalUnit::Minute),
        "second" | "seconds" | "sec" | "secs" => Token::Unit(TemporalUnit::Second),

        // Unrecognized: keep original casing for error messages
        _ => Token::Word(original.to_string()),
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
            vec![Token::Number(3), Token::Unit(TemporalUnit::Hour), Token::Ago]
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
        assert_eq!(
            kinds("@-86400"),
            vec![Token::AtSign, Token::Number(-86400)]
        );
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
        assert_eq!(
            kinds("YESTERDAY"),
            vec![Token::Yesterday]
        );
    }

    #[test]
    fn mixed_case_camel() {
        assert_eq!(
            kinds("Tomorrow"),
            vec![Token::Tomorrow]
        );
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
    fn negative_number_span() {
        let tokens = tokenize("-42");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, Token::Number(-42));
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 3 });
    }

    #[test]
    fn epoch_suffix_span() {
        let tokens = tokenize("100ms");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, Token::Number(100));
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 3 });
        assert_eq!(tokens[1].kind, Token::EpochSuffix(EpochPrecision::Milliseconds));
        assert_eq!(tokens[1].span, ByteSpan { start: 3, end: 5 });
    }
}
