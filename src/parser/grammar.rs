//! Recursive descent parser: `Vec<SpannedToken>` -> `DateExpr`.
//!
//! Each grammar production is a method that tries to match its pattern.
//! On failure, it restores the cursor position (backtrack via save/restore).
//! On success, it advances past consumed tokens and returns an AST node.
//!
//! Productions are tried in priority order (most specific first):
//! P0: epoch, P1: duration offset, P2: relative with time, P3: day ref,
//! P4: absolute datetime, P5: time only, P6: bare weekday.

use crate::parser::{
    ast::*,
    error::ParseError,
    suggest,
    token::*,
};

/// Recursive descent parser over a token slice.
pub(crate) struct Parser<'a> {
    tokens: &'a [SpannedToken],
    pos: usize,
    input: &'a str,
}

impl<'a> Parser<'a> {
    /// Create a new parser over the given token slice.
    pub(crate) fn new(tokens: &'a [SpannedToken], input: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            input,
        }
    }

    /// Parse the token stream into a `DateExpr`.
    ///
    /// Empty token list -> `DateExpr::Now` (Pitfall 6: empty input = now).
    pub(crate) fn parse_expression(&mut self) -> Result<DateExpr, ParseError> {
        if self.tokens.is_empty() {
            return Ok(DateExpr::Now);
        }

        // Single `Now` token
        if self.tokens.len() == 1 && self.tokens[0].kind == Token::Now {
            self.pos = 1;
            return Ok(DateExpr::Now);
        }

        // Try productions in priority order (most specific first)
        if let Some(expr) = self.try_epoch()? {
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_duration_offset()? {
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_relative_with_time()? {
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_day_ref_with_time()? {
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_absolute_datetime()? {
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_time_only()? {
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_bare_weekday()? {
            return self.with_optional_trailing(expr);
        }

        // Nothing matched -- produce error with suggestion
        Err(self.unexpected_input_error())
    }

    // ── Helper methods ─────────────────────────────────────────

    /// Look at current token without consuming.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|st| &st.kind)
    }

    /// Consume and return current token.
    fn advance(&mut self) -> Option<&SpannedToken> {
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    /// Consume if current token matches (discriminant comparison for data variants).
    fn match_token(&mut self, expected: &Token) -> bool {
        if let Some(tok) = self.peek() {
            if std::mem::discriminant(tok) == std::mem::discriminant(expected) {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    /// Save cursor position for backtracking.
    fn save(&self) -> usize {
        self.pos
    }

    /// Restore cursor position (backtrack).
    fn restore(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// True if all tokens have been consumed.
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Span of current token (or end-of-input span).
    fn current_span(&self) -> ByteSpan {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].span
        } else {
            let end = self.input.len();
            ByteSpan { start: end, end }
        }
    }

    /// Verify all tokens were consumed; error if significant tokens remain.
    fn with_optional_trailing(&self, expr: DateExpr) -> Result<DateExpr, ParseError> {
        if self.at_end() {
            return Ok(expr);
        }
        // Remaining tokens are an error
        let span = self.current_span();
        let remaining: Vec<String> = self.tokens[self.pos..]
            .iter()
            .map(|t| format!("{:?}", t.kind))
            .collect();
        Err(ParseError::unexpected(
            self.input,
            span,
            "end of input",
            &remaining.join(", "),
        ))
    }

    /// Extract the `i64` value from the previously consumed `Number` token.
    fn last_number(&self) -> i64 {
        match &self.tokens[self.pos - 1].kind {
            Token::Number(n) => *n,
            _ => 0,
        }
    }

    /// Extract the `Weekday` value from the previously consumed `Weekday` token.
    fn last_weekday(&self) -> jiff::civil::Weekday {
        match &self.tokens[self.pos - 1].kind {
            Token::Weekday(w) => *w,
            _ => unreachable!("called last_weekday after non-Weekday token"),
        }
    }

    /// Extract the month number from the previously consumed `Month` token.
    fn last_month(&self) -> i8 {
        match &self.tokens[self.pos - 1].kind {
            Token::Month(m) => *m,
            _ => unreachable!("called last_month after non-Month token"),
        }
    }

    // ── P0: Epoch ──────────────────────────────────────────────

    /// `AtSign Number [EpochSuffix]`
    fn try_epoch(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        if !self.match_token(&Token::AtSign) {
            return Ok(None);
        }

        if !self.match_token(&Token::Number(0)) {
            self.restore(saved);
            return Ok(None);
        }

        let raw = self.last_number();

        // Check for explicit epoch suffix
        let precision = if self.match_token(&Token::EpochSuffix(EpochPrecision::Seconds)) {
            match &self.tokens[self.pos - 1].kind {
                Token::EpochSuffix(p) => *p,
                _ => unreachable!(),
            }
        } else {
            // Auto-detect precision from magnitude
            detect_epoch_precision(raw)
        };

        Ok(Some(DateExpr::Epoch(EpochValue { raw, precision })))
    }

    // ── P1: Duration offset ────────────────────────────────────

    /// `In [A|An|Number] Unit ...` or `[A|An|Number] Unit ... Ago [From expr]`
    fn try_duration_offset(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        // Pattern A: "in N unit(s) [and N unit(s) ...]"
        if self.match_token(&Token::In) {
            if let Some(comps) = self.try_duration_components() {
                return Ok(Some(DateExpr::Offset(Direction::Future, comps)));
            }
            self.restore(saved);
        }

        // Pattern B: "N unit(s) [and N unit(s) ...] ago [from expr]"
        self.restore(saved);
        if let Some(comps) = self.try_duration_components() {
            if self.match_token(&Token::Ago) {
                // Check for "from" clause
                if self.match_token(&Token::From) {
                    let base = self.parse_expression()?;
                    return Ok(Some(DateExpr::OffsetFrom(
                        Direction::Past,
                        comps,
                        Box::new(base),
                    )));
                }
                return Ok(Some(DateExpr::Offset(Direction::Past, comps)));
            }
            self.restore(saved);
        }

        Ok(None)
    }

    /// Parse one or more duration components: `[A|An|Number] Unit [(And|,) [A|An|Number] Unit ...]`
    fn try_duration_components(&mut self) -> Option<Vec<DurationComponent>> {
        let mut comps = Vec::new();

        if let Some(comp) = self.try_single_duration() {
            comps.push(comp);
        } else {
            return None;
        }

        // Compound: loop consuming [And] Number/A/An Unit sequences
        loop {
            let saved = self.save();
            // Optional "and" connector (commas are already stripped by lexer)
            let _ = self.match_token(&Token::And);

            if let Some(comp) = self.try_single_duration() {
                comps.push(comp);
            } else {
                self.restore(saved);
                break;
            }
        }

        Some(comps)
    }

    /// Parse a single `[A|An|Number] Unit`.
    fn try_single_duration(&mut self) -> Option<DurationComponent> {
        let saved = self.save();

        let count = if self.match_token(&Token::A) || self.match_token(&Token::An) {
            1
        } else if self.match_token(&Token::Number(0)) {
            self.last_number()
        } else {
            return None;
        };

        if self.match_token(&Token::Unit(TemporalUnit::Year)) {
            let unit = match &self.tokens[self.pos - 1].kind {
                Token::Unit(u) => *u,
                _ => unreachable!(),
            };
            Some(DurationComponent { count, unit })
        } else {
            self.restore(saved);
            None
        }
    }

    // ── P2: Relative with time ─────────────────────────────────

    /// `(Today|Tomorrow|Yesterday|Overmorrow) [time_suffix]`
    /// Also handles reversed: `[time_suffix] (Today|Tomorrow|Yesterday|Overmorrow)`
    fn try_relative_with_time(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        // Forward order: relative keyword then optional time
        if let Some(rel) = self.try_relative_keyword() {
            let time = self.try_time_suffix();
            return Ok(Some(DateExpr::Relative(rel, time)));
        }

        // Reversed order: time then relative keyword (e.g., "15:00 tomorrow")
        self.restore(saved);
        if let Some(time) = self.try_time_suffix() {
            if let Some(rel) = self.try_relative_keyword() {
                return Ok(Some(DateExpr::Relative(rel, Some(time))));
            }
            self.restore(saved);
        }

        Ok(None)
    }

    /// Try to consume a relative keyword: Today, Tomorrow, Yesterday, Overmorrow.
    fn try_relative_keyword(&mut self) -> Option<RelativeDate> {
        match self.peek() {
            Some(Token::Today) => {
                self.advance();
                Some(RelativeDate::Today)
            }
            Some(Token::Tomorrow) => {
                self.advance();
                Some(RelativeDate::Tomorrow)
            }
            Some(Token::Yesterday) => {
                self.advance();
                Some(RelativeDate::Yesterday)
            }
            Some(Token::Overmorrow) => {
                self.advance();
                Some(RelativeDate::Overmorrow)
            }
            _ => None,
        }
    }

    // ── P3: Day reference with time ────────────────────────────

    /// `(Next|Last|This) Weekday(w) [time_suffix]`
    fn try_day_ref_with_time(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        let dir = match self.peek() {
            Some(Token::Next) => {
                self.advance();
                Direction::Next
            }
            Some(Token::Last) => {
                self.advance();
                Direction::Last
            }
            Some(Token::This) => {
                self.advance();
                Direction::This
            }
            _ => return Ok(None),
        };

        if self.match_token(&Token::Weekday(jiff::civil::Weekday::Monday)) {
            let weekday = self.last_weekday();
            let time = self.try_time_suffix();
            return Ok(Some(DateExpr::DayRef(dir, weekday, time)));
        }

        self.restore(saved);
        Ok(None)
    }

    // ── P4: Absolute datetime ──────────────────────────────────

    /// ISO date: `Number Dash Number Dash Number [time]`
    /// Day-month: `Number Month [Number] [time]`
    fn try_absolute_datetime(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        // Pattern A: ISO date "YYYY-MM-DD [HH:MM[:SS]]"
        if self.match_token(&Token::Number(0)) {
            let first = self.last_number();

            if self.match_token(&Token::Dash)
                && self.match_token(&Token::Number(0))
            {
                let second = self.last_number();

                if self.match_token(&Token::Dash)
                    && self.match_token(&Token::Number(0))
                {
                    let third = self.last_number();

                    let abs = AbsoluteDate {
                        year: first as i16,
                        month: second as i8,
                        day: third as i8,
                    };

                    let time = self.try_time_suffix();
                    return Ok(Some(DateExpr::Absolute(abs, time)));
                }
            }

            // Pattern B: Day-month "DD Month [YYYY] [time]"
            // first is the day number, next should be Month
            self.restore(saved);
            self.advance(); // consume the number again
            let day = first;

            if self.match_token(&Token::Month(1)) {
                let month = self.last_month();

                // Optional year
                let year = if self.match_token(&Token::Number(0)) {
                    self.last_number() as i16
                } else {
                    0 // sentinel: resolver will fill in current year
                };

                let abs = AbsoluteDate {
                    year,
                    month,
                    day: day as i8,
                };

                let time = self.try_time_suffix();
                return Ok(Some(DateExpr::Absolute(abs, time)));
            }
        }

        self.restore(saved);
        Ok(None)
    }

    // ── P5: Time only ──────────────────────────────────────────

    /// `Number Colon Number [Colon Number]` as the entire expression.
    fn try_time_only(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        if let Some(time) = self.try_time_pattern() {
            // Only valid if this is the entire expression
            if self.at_end() {
                return Ok(Some(DateExpr::TimeOnly(time)));
            }
        }

        self.restore(saved);
        Ok(None)
    }

    // ── P6: Bare weekday ───────────────────────────────────────

    /// Single `Weekday(w)` token -> `DayRef(Next, w, None)` (future-biased D-05).
    fn try_bare_weekday(&mut self) -> Result<Option<DateExpr>, ParseError> {
        if self.match_token(&Token::Weekday(jiff::civil::Weekday::Monday)) {
            let weekday = self.last_weekday();
            return Ok(Some(DateExpr::DayRef(Direction::Next, weekday, None)));
        }
        Ok(None)
    }

    // ── Time suffix helper ─────────────────────────────────────

    /// `[At] Number Colon Number [Colon Number]`
    fn try_time_suffix(&mut self) -> Option<TimeExpr> {
        let saved = self.save();
        // Optional leading "at"
        let _ = self.match_token(&Token::At);

        if let Some(time) = self.try_time_pattern() {
            return Some(time);
        }

        self.restore(saved);
        None
    }

    /// `Number Colon Number [Colon Number]`
    fn try_time_pattern(&mut self) -> Option<TimeExpr> {
        let saved = self.save();

        if !self.match_token(&Token::Number(0)) {
            return None;
        }
        let hour = self.last_number() as i8;

        if !self.match_token(&Token::Colon) {
            self.restore(saved);
            return None;
        }

        if !self.match_token(&Token::Number(0)) {
            self.restore(saved);
            return None;
        }
        let minute = self.last_number() as i8;

        // Optional seconds
        let saved_after_hm = self.save();
        if self.match_token(&Token::Colon) {
            if self.match_token(&Token::Number(0)) {
                let second = self.last_number() as i8;
                return Some(TimeExpr::HourMinuteSecond(hour, minute, second));
            }
            self.restore(saved_after_hm);
        }

        Some(TimeExpr::HourMinute(hour, minute))
    }

    // ── Error production ───────────────────────────────────────

    /// Produce an error with typo suggestion for unrecognized words (D-08).
    fn unexpected_input_error(&self) -> ParseError {
        if let Some(Token::Word(w)) = self.peek() {
            if let Some(suggestion) = suggest::suggest_keyword(w, 2) {
                return ParseError::unrecognized(self.input)
                    .with_suggestion(suggestion.to_string());
            }
        }
        ParseError::unrecognized(self.input)
    }
}

/// Determine epoch precision from magnitude when no explicit suffix is given.
///
/// Thresholds (from research):
/// - `|value| < 1e12` -> seconds
/// - `1e12 <= |value| < 1e15` -> milliseconds
/// - `1e15 <= |value| < 1e18` -> microseconds
/// - `|value| >= 1e18` -> nanoseconds
pub(crate) fn detect_epoch_precision(value: i64) -> EpochPrecision {
    let abs = value.unsigned_abs();
    if abs < 1_000_000_000_000 {
        EpochPrecision::Seconds
    } else if abs < 1_000_000_000_000_000 {
        EpochPrecision::Milliseconds
    } else if abs < 1_000_000_000_000_000_000 {
        EpochPrecision::Microseconds
    } else {
        EpochPrecision::Nanoseconds
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use jiff::civil::Weekday;

    /// Helper: create a SpannedToken with a dummy span.
    fn st(kind: Token) -> SpannedToken {
        SpannedToken {
            kind,
            span: ByteSpan { start: 0, end: 0 },
        }
    }

    /// Helper: parse a token list and return the AST.
    fn parse_tokens(tokens: &[SpannedToken]) -> Result<DateExpr, ParseError> {
        let mut parser = Parser::new(tokens, "");
        parser.parse_expression()
    }

    #[test]
    fn empty_tokens_yields_now() {
        let result = parse_tokens(&[]).unwrap();
        assert_eq!(result, DateExpr::Now);
    }

    #[test]
    fn single_now_token() {
        let result = parse_tokens(&[st(Token::Now)]).unwrap();
        assert_eq!(result, DateExpr::Now);
    }

    #[test]
    fn today_yields_relative() {
        let result = parse_tokens(&[st(Token::Today)]).unwrap();
        assert_eq!(result, DateExpr::Relative(RelativeDate::Today, None));
    }

    #[test]
    fn tomorrow_with_time() {
        let tokens = vec![
            st(Token::Tomorrow),
            st(Token::Number(15)),
            st(Token::Colon),
            st(Token::Number(0)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourMinute(15, 0)))
        );
    }

    #[test]
    fn next_friday() {
        let tokens = vec![st(Token::Next), st(Token::Weekday(Weekday::Friday))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::DayRef(Direction::Next, Weekday::Friday, None)
        );
    }

    #[test]
    fn last_monday() {
        let tokens = vec![st(Token::Last), st(Token::Weekday(Weekday::Monday))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::DayRef(Direction::Last, Weekday::Monday, None)
        );
    }

    #[test]
    fn in_3_days() {
        let tokens = vec![
            st(Token::In),
            st(Token::Number(3)),
            st(Token::Unit(TemporalUnit::Day)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Future,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Day
                }]
            )
        );
    }

    #[test]
    fn three_hours_ago() {
        let tokens = vec![
            st(Token::Number(3)),
            st(Token::Unit(TemporalUnit::Hour)),
            st(Token::Ago),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Past,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Hour
                }]
            )
        );
    }

    #[test]
    fn a_week_ago_article() {
        let tokens = vec![
            st(Token::A),
            st(Token::Unit(TemporalUnit::Week)),
            st(Token::Ago),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Past,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Week
                }]
            )
        );
    }

    #[test]
    fn epoch_at_sign_number() {
        let tokens = vec![st(Token::AtSign), st(Token::Number(1_735_689_600))];
        let result = parse_tokens(&tokens).unwrap();
        assert!(matches!(result, DateExpr::Epoch(EpochValue { raw: 1_735_689_600, precision: EpochPrecision::Seconds })));
    }

    #[test]
    fn iso_date_absolute() {
        let tokens = vec![
            st(Token::Number(2025)),
            st(Token::Dash),
            st(Token::Number(1)),
            st(Token::Dash),
            st(Token::Number(1)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Absolute(
                AbsoluteDate {
                    year: 2025,
                    month: 1,
                    day: 1
                },
                None
            )
        );
    }

    #[test]
    fn bare_weekday_future_biased() {
        let tokens = vec![st(Token::Weekday(Weekday::Friday))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::DayRef(Direction::Next, Weekday::Friday, None)
        );
    }

    #[test]
    fn unknown_word_error_with_suggestion() {
        let tokens = vec![SpannedToken {
            kind: Token::Word("thursdya".to_string()),
            span: ByteSpan { start: 0, end: 8 },
        }];
        let err = parse_tokens(&tokens).unwrap_err();
        let msg = err.format_message();
        assert!(msg.contains("Did you mean 'thursday'?"), "got: {msg}");
    }

    #[test]
    fn an_hour_ago_article() {
        let tokens = vec![
            st(Token::An),
            st(Token::Unit(TemporalUnit::Hour)),
            st(Token::Ago),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Past,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Hour
                }]
            )
        );
    }

    #[test]
    fn compound_duration_with_and() {
        let tokens = vec![
            st(Token::Number(2)),
            st(Token::Unit(TemporalUnit::Hour)),
            st(Token::And),
            st(Token::Number(5)),
            st(Token::Unit(TemporalUnit::Minute)),
            st(Token::Ago),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Past,
                vec![
                    DurationComponent {
                        count: 2,
                        unit: TemporalUnit::Hour
                    },
                    DurationComponent {
                        count: 5,
                        unit: TemporalUnit::Minute
                    },
                ]
            )
        );
    }

    #[test]
    fn this_weekday() {
        let tokens = vec![st(Token::This), st(Token::Weekday(Weekday::Sunday))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::DayRef(Direction::This, Weekday::Sunday, None)
        );
    }

    #[test]
    fn next_friday_with_time() {
        let tokens = vec![
            st(Token::Next),
            st(Token::Weekday(Weekday::Friday)),
            st(Token::Number(17)),
            st(Token::Colon),
            st(Token::Number(0)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::DayRef(
                Direction::Next,
                Weekday::Friday,
                Some(TimeExpr::HourMinute(17, 0))
            )
        );
    }

    #[test]
    fn yesterday_relative() {
        let result = parse_tokens(&[st(Token::Yesterday)]).unwrap();
        assert_eq!(result, DateExpr::Relative(RelativeDate::Yesterday, None));
    }

    #[test]
    fn overmorrow_relative() {
        let result = parse_tokens(&[st(Token::Overmorrow)]).unwrap();
        assert_eq!(result, DateExpr::Relative(RelativeDate::Overmorrow, None));
    }

    #[test]
    fn iso_date_with_time() {
        let tokens = vec![
            st(Token::Number(2022)),
            st(Token::Dash),
            st(Token::Number(11)),
            st(Token::Dash),
            st(Token::Number(7)),
            st(Token::Number(13)),
            st(Token::Colon),
            st(Token::Number(25)),
            st(Token::Colon),
            st(Token::Number(30)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Absolute(
                AbsoluteDate {
                    year: 2022,
                    month: 11,
                    day: 7
                },
                Some(TimeExpr::HourMinuteSecond(13, 25, 30))
            )
        );
    }

    #[test]
    fn time_only() {
        let tokens = vec![
            st(Token::Number(15)),
            st(Token::Colon),
            st(Token::Number(30)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::TimeOnly(TimeExpr::HourMinute(15, 30)));
    }

    #[test]
    fn epoch_auto_detect_milliseconds() {
        let tokens = vec![st(Token::AtSign), st(Token::Number(1_735_689_600_000))];
        let result = parse_tokens(&tokens).unwrap();
        assert!(matches!(
            result,
            DateExpr::Epoch(EpochValue {
                raw: 1_735_689_600_000,
                precision: EpochPrecision::Milliseconds
            })
        ));
    }

    #[test]
    fn epoch_explicit_suffix() {
        let tokens = vec![
            st(Token::AtSign),
            st(Token::Number(1_735_689_600)),
            st(Token::EpochSuffix(EpochPrecision::Milliseconds)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert!(matches!(
            result,
            DateExpr::Epoch(EpochValue {
                raw: 1_735_689_600,
                precision: EpochPrecision::Milliseconds
            })
        ));
    }

    #[test]
    fn detect_epoch_precision_seconds() {
        assert_eq!(detect_epoch_precision(1_735_689_600), EpochPrecision::Seconds);
    }

    #[test]
    fn detect_epoch_precision_milliseconds() {
        assert_eq!(
            detect_epoch_precision(1_735_689_600_000),
            EpochPrecision::Milliseconds
        );
    }

    #[test]
    fn detect_epoch_precision_microseconds() {
        assert_eq!(
            detect_epoch_precision(1_735_689_600_000_000),
            EpochPrecision::Microseconds
        );
    }

    #[test]
    fn detect_epoch_precision_nanoseconds() {
        assert_eq!(
            detect_epoch_precision(1_735_689_600_000_000_000),
            EpochPrecision::Nanoseconds
        );
    }

    #[test]
    fn day_month_year_absolute() {
        let tokens = vec![
            st(Token::Number(24)),
            st(Token::Month(3)), // March
            st(Token::Number(2025)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Absolute(
                AbsoluteDate {
                    year: 2025,
                    month: 3,
                    day: 24,
                },
                None
            )
        );
    }

    #[test]
    fn day_month_no_year() {
        let tokens = vec![
            st(Token::Number(24)),
            st(Token::Month(3)), // March
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Absolute(
                AbsoluteDate {
                    year: 0, // sentinel
                    month: 3,
                    day: 24,
                },
                None
            )
        );
    }

    #[test]
    fn offset_from_base() {
        let tokens = vec![
            st(Token::Number(3)),
            st(Token::Unit(TemporalUnit::Hour)),
            st(Token::Ago),
            st(Token::From),
            st(Token::Tomorrow),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::OffsetFrom(
                Direction::Past,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Hour
                }],
                Box::new(DateExpr::Relative(RelativeDate::Tomorrow, None)),
            )
        );
    }

    #[test]
    fn today_at_time() {
        let tokens = vec![
            st(Token::Today),
            st(Token::At),
            st(Token::Number(18)),
            st(Token::Colon),
            st(Token::Number(30)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Relative(RelativeDate::Today, Some(TimeExpr::HourMinute(18, 30)))
        );
    }

    #[test]
    fn negative_epoch() {
        let tokens = vec![st(Token::AtSign), st(Token::Number(-86400))];
        let result = parse_tokens(&tokens).unwrap();
        assert!(matches!(
            result,
            DateExpr::Epoch(EpochValue {
                raw: -86400,
                precision: EpochPrecision::Seconds
            })
        ));
    }

    #[test]
    fn in_a_week() {
        let tokens = vec![
            st(Token::In),
            st(Token::A),
            st(Token::Unit(TemporalUnit::Week)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Future,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Week
                }]
            )
        );
    }
}
