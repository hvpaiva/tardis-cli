//! Lexer (tokenizer) for the TARDIS natural-language date parser.
//!
//! Scans input character-by-character, producing a `Vec<SpannedToken>` with
//! byte-accurate span tracking. Keywords are matched via locale-driven
//! `LocaleKeywords` lookup. UTF-8 multi-byte characters are handled for
//! accented words (e.g., "amanha" in Portuguese).
//!
//! Only `Token::Word(String)` carries owned data
//! (for unrecognized words used in error messages and typo suggestions).

use crate::locale::{self, LocaleKeywords};
use crate::parser::token::{ByteSpan, EpochPrecision, SpannedToken, Token};

/// Tokenize the input string into a sequence of spanned tokens.
///
/// - Whitespace is consumed but not emitted.
/// - Commas are consumed but not emitted (optional separators in compound durations).
/// - Keywords are matched via the locale-driven `locale_keywords` table.
/// - Unrecognized alpha words are captured as `Token::Word` with original casing.
/// - Epoch suffixes (`ms`, `us`, `ns`, `s`) are detected immediately after numbers
///   when not separated by whitespace and not followed by more alpha chars.
/// - UTF-8 multi-byte alphabetic characters are handled in word scanning.
/// - After initial tokenization, a multi-word merge pass combines locale-specific
///   multi-word patterns into single tokens.
pub(crate) fn tokenize(input: &str, locale_keywords: &LocaleKeywords) -> Vec<SpannedToken> {
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
                // "+5" is a positive number ONLY at the start of input or after @
                // (epoch like @+1735689600). In all other positions, emit Plus operator
                // so "tomorrow+3h" tokenizes as [Tomorrow, Plus, Number(3), Unit(Hour)].
                let is_sign_position = tokens.is_empty()
                    || tokens
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
                // "-5" is a negative number ONLY at the start of input or after @
                // (epoch like @-1735689600). In all other positions, emit Dash so
                // "tomorrow-3h" tokenizes as [Tomorrow, Dash, Number(3), Unit(Hour)]
                // and ISO dates "2025-03-24" remain [Number, Dash, Number, Dash, Number].
                let is_sign_position = tokens.is_empty()
                    || tokens
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
            // Normalize for keyword lookup: lowercase + strip diacritics
            let normalized: String = original
                .chars()
                .map(|c| {
                    let lower = c.to_lowercase().next().unwrap_or(c);
                    locale::strip_diacritics(lower)
                })
                .collect();

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

            let kind = locale_keywords.lookup(&normalized, original);
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

    // Multi-word token merge pass (for locale-specific patterns)
    merge_multi_word_patterns(&mut tokens, locale_keywords, input);

    tokens
}

/// Post-processing pass: scan for known multi-word sequences from the locale
/// and replace them with single tokens. Handles PT patterns like
/// "daqui a" -> Token::In, "depois de amanha" -> Token::Overmorrow, etc.
///
/// Matching uses the original input text (via byte spans) rather than
/// token kinds, because the same word can be recognized as different
/// keyword tokens across locales (e.g., "amanha" -> Token::Tomorrow in PT,
/// but the multi-word pattern "depois de amanha" needs to match on the
/// raw text "amanha", not the token kind "Tomorrow").
fn merge_multi_word_patterns(
    tokens: &mut Vec<SpannedToken>,
    locale_keywords: &LocaleKeywords,
    input: &str,
) {
    let patterns = locale_keywords.multi_word_patterns();
    if patterns.is_empty() {
        return;
    }

    let mut i = 0;
    while i < tokens.len() {
        let mut matched = false;
        for &(words, ref target_token) in patterns {
            if i + words.len() > tokens.len() {
                continue;
            }
            // Check if the next N tokens match the multi-word pattern
            // by comparing normalized text from the original input
            let all_match = words.iter().enumerate().all(|(j, &expected)| {
                let span = &tokens[i + j].span;
                let original = &input[span.start..span.end];
                let normalized: String = original
                    .chars()
                    .map(|c| {
                        let lower = c.to_lowercase().next().unwrap_or(c);
                        locale::strip_diacritics(lower)
                    })
                    .collect();
                normalized == expected
            });

            if all_match {
                // Merge: replace first token, remove the rest
                let start_span = tokens[i].span.start;
                let end_span = tokens[i + words.len() - 1].span.end;
                tokens[i] = SpannedToken {
                    kind: target_token.clone(),
                    span: ByteSpan {
                        start: start_span,
                        end: end_span,
                    },
                };
                for _ in 1..words.len() {
                    tokens.remove(i + 1);
                }
                matched = true;
                break;
            }
        }
        if !matched {
            i += 1;
        }
    }
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
    use crate::locale::en::EN_LOCALE;
    use jiff::civil::Weekday;

    /// Helper to build EN locale keywords for tests.
    fn en_kw() -> LocaleKeywords {
        LocaleKeywords::from_locale(&EN_LOCALE)
    }

    /// Helper to extract just the token kinds from a tokenize result.
    fn kinds(input: &str) -> Vec<Token> {
        let kw = en_kw();
        tokenize(input, &kw).into_iter().map(|st| st.kind).collect()
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
        let kw = en_kw();
        let tokens = tokenize("next friday", &kw);
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
        let kw = en_kw();
        let tokens = tokenize("3 hours ago", &kw);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 1 });
        assert_eq!(tokens[1].span, ByteSpan { start: 2, end: 7 });
        assert_eq!(tokens[2].span, ByteSpan { start: 8, end: 11 });
    }

    #[test]
    fn span_tracking_iso_date() {
        let kw = en_kw();
        let tokens = tokenize("2025-01-15", &kw);
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 4 }); // 2025
        assert_eq!(tokens[1].span, ByteSpan { start: 4, end: 5 }); // -
        assert_eq!(tokens[2].span, ByteSpan { start: 5, end: 7 }); // 01
        assert_eq!(tokens[3].span, ByteSpan { start: 7, end: 8 }); // -
        assert_eq!(tokens[4].span, ByteSpan { start: 8, end: 10 }); // 15
    }

    #[test]
    fn negative_number_span() {
        let kw = en_kw();
        let tokens = tokenize("-42", &kw);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, Token::Number(-42));
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 3 });
    }

    #[test]
    fn epoch_suffix_span() {
        let kw = en_kw();
        let tokens = tokenize("100ms", &kw);
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
    fn plus_5_still_tokenizes_as_number_pitfall_7() {
        // Pitfall 7: "+5" at the start should tokenize as Number(5), not Plus+Number
        assert_eq!(kinds("+5"), vec![Token::Number(5)]);
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
        let kw = en_kw();
        let tokens = tokenize("amanh\u{00e3}", &kw);
        assert_eq!(tokens.len(), 1);
        // EN locale doesn't know "amanha", so it becomes a Word
        match &tokens[0].kind {
            Token::Word(w) => assert_eq!(w, "amanh\u{00e3}"),
            _ => panic!("expected Word token for unrecognized accented word"),
        }
    }

    #[test]
    fn utf8_span_tracking_correct() {
        // "amanha" = 6 bytes: a(1) m(1) a(1) n(1) h(1) a-tilde(2 bytes: 0xC3 0xA3)
        let kw = en_kw();
        let tokens = tokenize("amanh\u{00e3}", &kw);
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

    use crate::parser::token::TemporalUnit;

    // ── PT locale lexer tests ──────────────────────────────────

    use crate::locale::pt::PT_LOCALE;

    fn pt_kw() -> LocaleKeywords {
        LocaleKeywords::from_locale(&PT_LOCALE)
    }

    fn pt_kinds(input: &str) -> Vec<Token> {
        let kw = pt_kw();
        tokenize(input, &kw).into_iter().map(|st| st.kind).collect()
    }

    #[test]
    fn pt_tokenize_amanha_no_accent() {
        assert_eq!(pt_kinds("amanha"), vec![Token::Tomorrow]);
    }

    #[test]
    fn pt_tokenize_amanha_with_accent() {
        // "amanha" (with tilde) should also map to Tomorrow via accent stripping
        assert_eq!(pt_kinds("amanh\u{00e3}"), vec![Token::Tomorrow]);
    }

    #[test]
    fn pt_tokenize_daqui_a_3_dias_multi_word_merge() {
        assert_eq!(
            pt_kinds("daqui a 3 dias"),
            vec![Token::In, Token::Number(3), Token::Unit(TemporalUnit::Day)]
        );
    }

    #[test]
    fn pt_tokenize_ha_2_horas() {
        assert_eq!(
            pt_kinds("ha 2 horas"),
            vec![
                Token::Ago,
                Token::Number(2),
                Token::Unit(TemporalUnit::Hour)
            ]
        );
    }

    #[test]
    fn pt_tokenize_depois_de_amanha_multi_word_merge() {
        assert_eq!(pt_kinds("depois de amanha"), vec![Token::Overmorrow]);
    }

    #[test]
    fn pt_tokenize_proxima_sexta() {
        assert_eq!(
            pt_kinds("proxima sexta"),
            vec![Token::Next, Token::Weekday(jiff::civil::Weekday::Friday)]
        );
    }

    #[test]
    fn pt_tokenize_anteontem() {
        assert_eq!(pt_kinds("anteontem"), vec![Token::Ereyesterday]);
    }

    #[test]
    fn pt_tokenize_antes_de_ontem_multi_word_merge() {
        assert_eq!(pt_kinds("antes de ontem"), vec![Token::Ereyesterday]);
    }

    #[test]
    fn pt_tokenize_accented_proxima() {
        // "proxima" with accent on o should normalize to "proxima"
        assert_eq!(
            pt_kinds("pr\u{00f3}xima sexta"),
            vec![Token::Next, Token::Weekday(jiff::civil::Weekday::Friday)]
        );
    }

    #[test]
    fn pt_tokenize_accented_sabado() {
        assert_eq!(
            pt_kinds("s\u{00e1}bado"),
            vec![Token::Weekday(jiff::civil::Weekday::Saturday)]
        );
    }

    #[test]
    fn pt_tokenize_accented_marco() {
        // "marco" (with cedilla/accent) -> March
        assert_eq!(pt_kinds("mar\u{00e7}o"), vec![Token::Month(3)]);
    }

    #[test]
    fn pt_span_tracking_amanha_with_accent() {
        let kw = pt_kw();
        // "amanha" = a(1) m(1) a(1) n(1) h(1) a-tilde(2 bytes) = 7 bytes
        let tokens = tokenize("amanh\u{00e3}", &kw);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, Token::Tomorrow);
        assert_eq!(tokens[0].span, ByteSpan { start: 0, end: 7 });
    }

    #[test]
    fn pt_tokenize_em_5_minutos() {
        assert_eq!(
            pt_kinds("em 5 minutos"),
            vec![
                Token::In,
                Token::Number(5),
                Token::Unit(TemporalUnit::Minute)
            ]
        );
    }

    #[test]
    fn pt_tokenize_3_dias_atras() {
        assert_eq!(
            pt_kinds("3 dias atras"),
            vec![Token::Number(3), Token::Unit(TemporalUnit::Day), Token::Ago]
        );
    }

    #[test]
    fn pt_depois_de_amanha_before_depois_de() {
        // Verify longest match: "depois de amanha" should merge to Overmorrow,
        // not "depois de" -> After + leftover "amanha"
        assert_eq!(pt_kinds("depois de amanha"), vec![Token::Overmorrow]);
    }
}
