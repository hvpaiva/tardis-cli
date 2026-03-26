//! Recursive descent parser: `Vec<SpannedToken>` -> `DateExpr`.
//!
//! Each grammar production is a method that tries to match its pattern.
//! On failure, it restores the cursor position (backtrack via save/restore).
//! On success, it advances past consumed tokens and returns an AST node.
//!
//! Productions are tried in priority order (most specific first):
//! P0: epoch, P1: duration offset, P2: relative with time, P3: day ref,
//! P4: absolute datetime, P5: time only, P6: bare weekday.

use crate::parser::{ast::*, error::ParseError, suggest, token::*};

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

        // Single `Now` token (only if truly the only token)
        if self.tokens.len() == 1 && self.tokens[0].kind == Token::Now {
            self.pos = 1;
            return Ok(DateExpr::Now);
        }

        // Multi-token starting with `Now` -- consume and try arithmetic tail
        if self.peek() == Some(&Token::Now) && self.tokens.len() > 1 {
            self.advance();
            let expr = DateExpr::Now;
            let expr = self.try_arithmetic_tail(expr)?;
            return self.with_optional_trailing(expr);
        }

        // Try productions in priority order (most specific first)
        if let Some(expr) = self.try_epoch()? {
            let expr = self.try_arithmetic_tail(expr)?;
            return self.with_optional_trailing(expr);
        }
        // P0.5: Operator-prefixed offset (+3h, -1d) — must come before duration_offset
        if let Some(expr) = self.try_operator_prefixed_offset()? {
            let expr = self.try_arithmetic_tail(expr)?;
            return self.with_optional_trailing(expr);
        }
        // TaskWarrior boundary keywords (eod, sow, etc.) — must come before duration_offset
        if let Some(expr) = self.try_boundary_keyword()? {
            let expr = self.try_arithmetic_tail(expr)?;
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_duration_offset()? {
            let expr = self.try_arithmetic_tail(expr)?;
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_relative_with_time()? {
            let expr = self.try_arithmetic_tail(expr)?;
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_day_ref_with_time()? {
            let expr = self.try_arithmetic_tail(expr)?;
            return self.with_optional_trailing(expr);
        }
        // Range expressions: "last week", "this month", "next year", "Q3 2025"
        // Must come after day_ref (so "last monday" still parses as DayRef)
        // but before absolute_datetime
        if let Some(expr) = self.try_range()? {
            return self.with_optional_trailing(expr);
        }
        if let Some(expr) = self.try_absolute_datetime()? {
            let expr = self.try_arithmetic_tail(expr)?;
            return self.with_optional_trailing(expr);
        }
        // P5 (time-only) removed: standalone time expressions like "15:00", "3pm",
        // "15h" are rejected. Time requires day context (e.g., "tomorrow 15:00").
        // This is consistent with other units: "1d", "3 hours" are also not valid alone.
        if let Some(expr) = self.try_bare_weekday()? {
            let expr = self.try_arithmetic_tail(expr)?;
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

    /// Check if the next token is any Unit variant.
    fn peek_is_unit(&self) -> bool {
        matches!(self.peek(), Some(Token::Unit(_)))
    }

    /// Match a `Token::Word(w)` where the word equals the given target (case-insensitive).
    fn match_word(&mut self, target: &str) -> bool {
        if let Some(Token::Word(w)) = self.peek() {
            if w.eq_ignore_ascii_case(target) {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    /// Apply 12-hour AM/PM conversion to a time expression.
    ///
    /// Rules:
    /// - 12am -> hour 0 (midnight)
    /// - 1am-11am -> hour 1-11 (no change)
    /// - 12pm -> hour 12 (noon)
    /// - 1pm-11pm -> hour 13-23
    fn apply_meridiem(&self, time: TimeExpr, is_pm: bool) -> TimeExpr {
        let convert = |h: i8| -> i8 {
            if is_pm {
                if h == 12 { 12 } else { h + 12 }
            } else if h == 12 {
                0
            } else {
                h
            }
        };
        match time {
            TimeExpr::HourMinute(h, m) => TimeExpr::HourMinute(convert(h), m),
            TimeExpr::HourMinuteSecond(h, m, s) => TimeExpr::HourMinuteSecond(convert(h), m, s),
            TimeExpr::HourOnly(h) => TimeExpr::HourOnly(convert(h)),
            TimeExpr::SameTime => TimeExpr::SameTime,
        }
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

    // ── P0.5: Operator-prefixed offset ──────────────────────────

    /// P0.5: Operator-prefixed duration offset (D-07).
    /// `Plus/Dash DurationComponents` -> Offset(Future/Past, comps) with implicit "now".
    /// Examples: "+3h", "-1d", "+1h30min", "+1d3h"
    fn try_operator_prefixed_offset(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        if self.match_token(&Token::Plus) {
            if let Some(comps) = self.try_duration_components() {
                return Ok(Some(DateExpr::Offset(Direction::Future, comps)));
            }
            self.restore(saved);
            return Ok(None);
        }

        if self.match_token(&Token::Dash) {
            if let Some(comps) = self.try_duration_components() {
                return Ok(Some(DateExpr::Offset(Direction::Past, comps)));
            }
            self.restore(saved);
            return Ok(None);
        }

        Ok(None)
    }

    // ── P0.6: Boundary keyword ────────────────────────────────

    /// TaskWarrior boundary keyword production (D-11, D-12, D-13).
    /// `Boundary(kind)` -> DateExpr::Boundary(kind)
    /// Composes with arithmetic tail: "eod + 1h" works.
    fn try_boundary_keyword(&mut self) -> Result<Option<DateExpr>, ParseError> {
        if let Some(Token::Boundary(kind)) = self.peek().cloned() {
            self.advance();
            return Ok(Some(DateExpr::Boundary(kind)));
        }
        Ok(None)
    }

    // ── P1: Duration offset ────────────────────────────────────

    /// `In [A|An|Number] Unit ...` or `[A|An|Number] Unit ... Ago [From expr]`
    /// Also handles verbal arithmetic: `[A|An|Number] Unit ... After/Before expr`
    fn try_duration_offset(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        // Pattern A: "in N unit(s) [and N unit(s) ...]"
        if self.match_token(&Token::In) {
            if let Some(comps) = self.try_duration_components() {
                return Ok(Some(DateExpr::Offset(Direction::Future, comps)));
            }
            self.restore(saved);
        }

        // Pattern C: "Ago N unit(s) ..." (PT prefix-ago: "ha 2 horas" = "2 hours ago")
        // Token::Ago starts this pattern only when followed by duration components
        self.restore(saved);
        if self.match_token(&Token::Ago) {
            if let Some(comps) = self.try_duration_components() {
                return Ok(Some(DateExpr::Offset(Direction::Past, comps)));
            }
            self.restore(saved);
        }

        // Pattern B: "N unit(s) [and N unit(s) ...] after/before/ago [from expr]"
        self.restore(saved);
        if let Some(comps) = self.try_duration_components() {
            // Check for verbal arithmetic: "after" / "before" (D-06)
            // Must check BEFORE "ago" since "3 hours after tomorrow" != "3 hours ago"
            if self.match_token(&Token::After) {
                let base = self.parse_expression()?;
                return Ok(Some(DateExpr::OffsetFrom(
                    Direction::Future,
                    comps,
                    Box::new(base),
                )));
            }
            if self.match_token(&Token::Before) {
                let base = self.parse_expression()?;
                return Ok(Some(DateExpr::OffsetFrom(
                    Direction::Past,
                    comps,
                    Box::new(base),
                )));
            }

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
    ///
    /// Also handles NhMM inference: "13h30" -> 13 hours + 30 minutes (D-07, gap #1/#2).
    fn try_duration_components(&mut self) -> Option<Vec<DurationComponent>> {
        let mut comps = Vec::new();

        if let Some(comp) = self.try_single_duration() {
            // NhMM inference: "13h30" -> 13 hours + 30 minutes
            if comp.unit == TemporalUnit::Hour {
                let saved_after_hour = self.save();
                if self.match_token(&Token::Number(0)) {
                    let minutes = self.last_number();
                    if !self.peek_is_unit() {
                        comps.push(comp);
                        comps.push(DurationComponent {
                            count: minutes,
                            unit: TemporalUnit::Minute,
                        });
                        // Continue to compound loop for further components
                    } else {
                        self.restore(saved_after_hour);
                        comps.push(comp);
                    }
                } else {
                    comps.push(comp);
                }
            } else {
                comps.push(comp);
            }
        } else {
            return None;
        }

        // Compound: loop consuming [And] Number/A/An Unit sequences
        loop {
            let saved = self.save();
            // Optional "and" connector (commas are already stripped by lexer)
            let _ = self.match_token(&Token::And);

            if let Some(comp) = self.try_single_duration() {
                // NhMM inference in compound loop
                if comp.unit == TemporalUnit::Hour {
                    let saved_after_hour = self.save();
                    if self.match_token(&Token::Number(0)) {
                        let minutes = self.last_number();
                        if !self.peek_is_unit() {
                            comps.push(comp);
                            comps.push(DurationComponent {
                                count: minutes,
                                unit: TemporalUnit::Minute,
                            });
                            continue;
                        } else {
                            self.restore(saved_after_hour);
                            comps.push(comp);
                            continue;
                        }
                    }
                }
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
            Some(Token::Ereyesterday) => {
                self.advance();
                Some(RelativeDate::Ereyesterday)
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

            if self.match_token(&Token::Dash) && self.match_token(&Token::Number(0)) {
                let second = self.last_number();

                if self.match_token(&Token::Dash) && self.match_token(&Token::Number(0)) {
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

    // try_time_only removed: standalone time expressions ("15:00", "3pm", "15h")
    // are no longer valid. Time requires day context (e.g., "tomorrow 15:00").
    // This is consistent: "1d", "3 hours" standalone are also errors.

    // ── P6: Bare weekday ───────────────────────────────────────

    /// Single `Weekday(w)` token -> `DayRef(Next, w, None)` (future-biased D-05).
    fn try_bare_weekday(&mut self) -> Result<Option<DateExpr>, ParseError> {
        if self.match_token(&Token::Weekday(jiff::civil::Weekday::Monday)) {
            let weekday = self.last_weekday();
            return Ok(Some(DateExpr::DayRef(Direction::Next, weekday, None)));
        }
        Ok(None)
    }

    // ── Arithmetic tail (Phase 3) ────────────────────────────────

    /// After parsing a primary expression, consume trailing `+ duration` or `- duration`
    /// chains, wrapping left-to-right: `Arithmetic(Arithmetic(base, Add, [1d]), Sub, [30m])`
    fn try_arithmetic_tail(&mut self, base: DateExpr) -> Result<DateExpr, ParseError> {
        let mut result = base;

        loop {
            let saved = self.save();

            if self.match_token(&Token::Plus) {
                // Try N:MM compound duration first (gap #3)
                // "now + 13:30" = now + 13 hours 30 minutes
                {
                    let saved_colon = self.save();
                    if self.match_token(&Token::Number(0)) {
                        let hours = self.last_number();
                        if self.match_token(&Token::Colon) && self.match_token(&Token::Number(0)) {
                            let minutes = self.last_number();
                            let comps = vec![
                                DurationComponent {
                                    count: hours,
                                    unit: TemporalUnit::Hour,
                                },
                                DurationComponent {
                                    count: minutes,
                                    unit: TemporalUnit::Minute,
                                },
                            ];
                            result = DateExpr::Arithmetic(Box::new(result), ArithOp::Add, comps);
                            continue;
                        }
                    }
                    self.restore(saved_colon);
                }

                // Then try standard duration components
                if let Some(comps) = self.try_duration_components() {
                    result = DateExpr::Arithmetic(Box::new(result), ArithOp::Add, comps);
                    continue;
                }
                // No duration components after operator -- backtrack
                self.restore(saved);
                break;
            } else if self.match_token(&Token::Dash) {
                // Try N:MM compound duration first (gap #3)
                {
                    let saved_colon = self.save();
                    if self.match_token(&Token::Number(0)) {
                        let hours = self.last_number();
                        if self.match_token(&Token::Colon) && self.match_token(&Token::Number(0)) {
                            let minutes = self.last_number();
                            let comps = vec![
                                DurationComponent {
                                    count: hours,
                                    unit: TemporalUnit::Hour,
                                },
                                DurationComponent {
                                    count: minutes,
                                    unit: TemporalUnit::Minute,
                                },
                            ];
                            result = DateExpr::Arithmetic(Box::new(result), ArithOp::Sub, comps);
                            continue;
                        }
                    }
                    self.restore(saved_colon);
                }

                // Then try standard duration components
                if let Some(comps) = self.try_duration_components() {
                    result = DateExpr::Arithmetic(Box::new(result), ArithOp::Sub, comps);
                    continue;
                }
                // Not a duration after dash -- backtrack
                self.restore(saved);
                break;
            }

            // No operator found -- done
            break;
        }

        Ok(result)
    }

    // ── Range expressions (Phase 3) ────────────────────────────

    /// Try to parse range expressions:
    /// - `Last/This/Next Unit(Week/Month/Year)` -> `Range(LastWeek/ThisWeek/etc.)`
    /// - `Quarter(n) [Number(year)]` -> `Range(Quarter(year_or_0, n))`
    fn try_range(&mut self) -> Result<Option<DateExpr>, ParseError> {
        let saved = self.save();

        // Pattern A: "last/this/next week/month/year"
        let dir = match self.peek() {
            Some(Token::Last) => {
                self.advance();
                Some(Direction::Last)
            }
            Some(Token::This) => {
                self.advance();
                Some(Direction::This)
            }
            Some(Token::Next) => {
                self.advance();
                Some(Direction::Next)
            }
            _ => None,
        };

        if let Some(dir) = dir {
            // Only match if next token is Unit(Week/Month/Year) -- NOT Weekday
            if let Some(Token::Unit(unit)) = self.peek() {
                let unit = *unit;
                match unit {
                    TemporalUnit::Week | TemporalUnit::Month | TemporalUnit::Year => {
                        self.advance();
                        match dir {
                            Direction::Last => {
                                let range = match unit {
                                    TemporalUnit::Week => RangeExpr::LastWeek,
                                    TemporalUnit::Month => RangeExpr::LastMonth,
                                    TemporalUnit::Year => RangeExpr::LastYear,
                                    _ => unreachable!(),
                                };
                                return Ok(Some(DateExpr::Range(range)));
                            }
                            Direction::This => {
                                let range = match unit {
                                    TemporalUnit::Week => RangeExpr::ThisWeek,
                                    TemporalUnit::Month => RangeExpr::ThisMonth,
                                    TemporalUnit::Year => RangeExpr::ThisYear,
                                    _ => unreachable!(),
                                };
                                return Ok(Some(DateExpr::Range(range)));
                            }
                            Direction::Next => {
                                let range = match unit {
                                    TemporalUnit::Week => RangeExpr::NextWeek,
                                    TemporalUnit::Month => RangeExpr::NextMonth,
                                    TemporalUnit::Year => RangeExpr::NextYear,
                                    _ => unreachable!(),
                                };
                                return Ok(Some(DateExpr::Range(range)));
                            }
                            _ => unreachable!(),
                        }
                    }
                    _ => {}
                }
            }
            self.restore(saved);
        }

        // Pattern B: "Q3 2025" or "Q1" (standalone quarter)
        if let Some(Token::Quarter(q)) = self.peek() {
            let q = *q;
            self.advance();
            // Optional year
            let year = if self.match_token(&Token::Number(0)) {
                self.last_number() as i16
            } else {
                0 // sentinel: resolver will fill in current year
            };
            return Ok(Some(DateExpr::Range(RangeExpr::Quarter(year, q))));
        }

        Ok(None)
    }

    // ── Time suffix helper ─────────────────────────────────────

    /// `[At] Number Colon Number [Colon Number]` or `[At] Number Unit(Hour)` (D-10).
    /// Also handles AM/PM: `[At] Number [Colon Number [Colon Number]] Am/Pm`
    /// Also handles "at same time" -> SameTime.
    fn try_time_suffix(&mut self) -> Option<TimeExpr> {
        let saved = self.save();
        // Optional leading "at"
        let _ = self.match_token(&Token::At);

        if let Some(time) = self.try_time_pattern() {
            return Some(time);
        }

        // Number Unit(Hour) [Number] -> HourOnly or HourMinute
        // "today 18h" = "today 18:00", "today 15h30" = "today 15:30"
        self.restore(saved);
        let _ = self.match_token(&Token::At); // re-consume optional "at"
        if self.match_token(&Token::Number(0)) {
            let hour = self.last_number() as i8;
            if self.match_token(&Token::Unit(TemporalUnit::Hour)) {
                // Check for trailing minutes: "15h30" -> HourMinute(15, 30)
                let saved_after_h = self.save();
                if self.match_token(&Token::Number(0)) {
                    let minute = self.last_number() as i8;
                    // Only consume if not followed by a unit (otherwise it's a duration like "1h 30m")
                    if !self.peek_is_unit() {
                        return Some(TimeExpr::HourMinute(hour, minute));
                    }
                    self.restore(saved_after_h);
                }
                return Some(TimeExpr::HourOnly(hour));
            }
        }

        // Bare Number Am/Pm -> HourOnly (e.g., "3pm" -> HourOnly(15))
        // No explicit minutes → hour granularity, same as "15h"
        self.restore(saved);
        let _ = self.match_token(&Token::At); // re-consume optional "at"
        if self.match_token(&Token::Number(0)) {
            let hour = self.last_number() as i8;
            if self.match_token(&Token::Am) {
                return Some(self.apply_meridiem(TimeExpr::HourOnly(hour), false));
            }
            if self.match_token(&Token::Pm) {
                return Some(self.apply_meridiem(TimeExpr::HourOnly(hour), true));
            }
        }

        // "at same time" -> SameTime
        self.restore(saved);
        if self.match_token(&Token::At) && self.match_word("same") && self.match_word("time") {
            return Some(TimeExpr::SameTime);
        }

        self.restore(saved);
        None
    }

    /// `Number Colon Number [Colon Number] [Am|Pm]`
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
                let time = TimeExpr::HourMinuteSecond(hour, minute, second);
                // Check for trailing AM/PM on HH:MM:SS
                if self.match_token(&Token::Am) {
                    return Some(self.apply_meridiem(time, false));
                }
                if self.match_token(&Token::Pm) {
                    return Some(self.apply_meridiem(time, true));
                }
                return Some(time);
            }
            self.restore(saved_after_hm);
        }

        let time = TimeExpr::HourMinute(hour, minute);
        // Check for trailing AM/PM on HH:MM
        if self.match_token(&Token::Am) {
            return Some(self.apply_meridiem(time, false));
        }
        if self.match_token(&Token::Pm) {
            return Some(self.apply_meridiem(time, true));
        }

        Some(time)
    }

    // ── Error production ───────────────────────────────────────

    /// Produce an error with typo suggestion for unrecognized words (D-08).
    fn unexpected_input_error(&self) -> ParseError {
        if let Some(Token::Word(w)) = self.peek() {
            if let Some(suggestion) = suggest::suggest_keyword(w, 2) {
                return ParseError::unrecognized(self.input).with_suggestion(suggestion);
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
        assert!(matches!(
            result,
            DateExpr::Epoch(EpochValue {
                raw: 1_735_689_600,
                precision: EpochPrecision::Seconds
            })
        ));
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
        assert!(msg.contains("Did you mean"), "got: {msg}");
        assert!(msg.contains("thursday"), "got: {msg}");
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
    fn standalone_time_is_error() {
        // Standalone time expressions are rejected (no implicit "today")
        let tokens = vec![
            st(Token::Number(15)),
            st(Token::Colon),
            st(Token::Number(30)),
        ];
        assert!(parse_tokens(&tokens).is_err());
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
        assert_eq!(
            detect_epoch_precision(1_735_689_600),
            EpochPrecision::Seconds
        );
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

    // ── Phase 3: Arithmetic expression grammar tests ──────────────

    #[test]
    fn tomorrow_plus_3_hours_arithmetic() {
        let tokens = vec![
            st(Token::Tomorrow),
            st(Token::Plus),
            st(Token::Number(3)),
            st(Token::Unit(TemporalUnit::Hour)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Arithmetic(
                Box::new(DateExpr::Relative(RelativeDate::Tomorrow, None)),
                ArithOp::Add,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Hour,
                }],
            )
        );
    }

    #[test]
    fn now_plus_1_day_plus_3_hours_minus_30_minutes_chained() {
        let tokens = vec![
            st(Token::Now),
            st(Token::Plus),
            st(Token::Number(1)),
            st(Token::Unit(TemporalUnit::Day)),
            st(Token::Plus),
            st(Token::Number(3)),
            st(Token::Unit(TemporalUnit::Hour)),
            st(Token::Dash),
            st(Token::Number(30)),
            st(Token::Unit(TemporalUnit::Minute)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        // Left-to-right: Arithmetic(Arithmetic(Arithmetic(Now, Add, [1d]), Add, [3h]), Sub, [30m])
        assert_eq!(
            result,
            DateExpr::Arithmetic(
                Box::new(DateExpr::Arithmetic(
                    Box::new(DateExpr::Arithmetic(
                        Box::new(DateExpr::Now),
                        ArithOp::Add,
                        vec![DurationComponent {
                            count: 1,
                            unit: TemporalUnit::Day,
                        }],
                    )),
                    ArithOp::Add,
                    vec![DurationComponent {
                        count: 3,
                        unit: TemporalUnit::Hour,
                    }],
                )),
                ArithOp::Sub,
                vec![DurationComponent {
                    count: 30,
                    unit: TemporalUnit::Minute,
                }],
            )
        );
    }

    #[test]
    fn three_hours_after_tomorrow_verbal() {
        let tokens = vec![
            st(Token::Number(3)),
            st(Token::Unit(TemporalUnit::Hour)),
            st(Token::After),
            st(Token::Tomorrow),
        ];
        let result = parse_tokens(&tokens).unwrap();
        // Per D-06: reuses OffsetFrom(Future, ...)
        assert_eq!(
            result,
            DateExpr::OffsetFrom(
                Direction::Future,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Hour,
                }],
                Box::new(DateExpr::Relative(RelativeDate::Tomorrow, None)),
            )
        );
    }

    #[test]
    fn two_days_before_next_friday_verbal() {
        let tokens = vec![
            st(Token::Number(2)),
            st(Token::Unit(TemporalUnit::Day)),
            st(Token::Before),
            st(Token::Next),
            st(Token::Weekday(Weekday::Friday)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::OffsetFrom(
                Direction::Past,
                vec![DurationComponent {
                    count: 2,
                    unit: TemporalUnit::Day,
                }],
                Box::new(DateExpr::DayRef(Direction::Next, Weekday::Friday, None)),
            )
        );
    }

    // ── Phase 3: Range expression grammar tests ──────────────

    #[test]
    fn last_week_range() {
        // "last week" -> Range(LastWeek) -- period start, consistent with this/next
        let tokens = vec![st(Token::Last), st(Token::Unit(TemporalUnit::Week))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::LastWeek));
    }

    #[test]
    fn last_month_range() {
        // "last month" -> Range(LastMonth) -- period start, consistent with this/next
        let tokens = vec![st(Token::Last), st(Token::Unit(TemporalUnit::Month))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::LastMonth));
    }

    #[test]
    fn last_year_range() {
        // "last year" -> Range(LastYear) -- period start, consistent with this/next
        let tokens = vec![st(Token::Last), st(Token::Unit(TemporalUnit::Year))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::LastYear));
    }

    #[test]
    fn this_week_still_range() {
        // "this week" should remain a range expression
        let tokens = vec![st(Token::This), st(Token::Unit(TemporalUnit::Week))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::ThisWeek));
    }

    #[test]
    fn this_month_range() {
        let tokens = vec![st(Token::This), st(Token::Unit(TemporalUnit::Month))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::ThisMonth));
    }

    #[test]
    fn next_week_still_range() {
        // "next week" should remain a range expression
        let tokens = vec![st(Token::Next), st(Token::Unit(TemporalUnit::Week))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::NextWeek));
    }

    #[test]
    fn next_year_range() {
        let tokens = vec![st(Token::Next), st(Token::Unit(TemporalUnit::Year))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::NextYear));
    }

    #[test]
    fn q3_2025_quarter_range() {
        let tokens = vec![st(Token::Quarter(3)), st(Token::Number(2025))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::Quarter(2025, 3)));
    }

    #[test]
    fn q1_no_year_quarter_range() {
        let tokens = vec![st(Token::Quarter(1))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::Quarter(0, 1)));
    }

    #[test]
    fn iso_date_minus_3_days_arithmetic() {
        let tokens = vec![
            st(Token::Number(2025)),
            st(Token::Dash),
            st(Token::Number(1)),
            st(Token::Dash),
            st(Token::Number(1)),
            st(Token::Dash),
            st(Token::Number(3)),
            st(Token::Unit(TemporalUnit::Day)),
        ];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::Arithmetic(
                Box::new(DateExpr::Absolute(
                    AbsoluteDate {
                        year: 2025,
                        month: 1,
                        day: 1,
                    },
                    None,
                )),
                ArithOp::Sub,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Day,
                }],
            )
        );
    }

    #[test]
    fn last_monday_still_parses_as_day_ref() {
        // "last monday" should NOT become a range -- must stay as DayRef
        let tokens = vec![st(Token::Last), st(Token::Weekday(Weekday::Monday))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(
            result,
            DateExpr::DayRef(Direction::Last, Weekday::Monday, None)
        );
    }

    #[test]
    fn this_week_range() {
        let tokens = vec![st(Token::This), st(Token::Unit(TemporalUnit::Week))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::ThisWeek));
    }

    #[test]
    fn next_week_range() {
        let tokens = vec![st(Token::Next), st(Token::Unit(TemporalUnit::Week))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::NextWeek));
    }

    #[test]
    fn next_month_range() {
        let tokens = vec![st(Token::Next), st(Token::Unit(TemporalUnit::Month))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::NextMonth));
    }

    #[test]
    fn this_year_range() {
        let tokens = vec![st(Token::This), st(Token::Unit(TemporalUnit::Year))];
        let result = parse_tokens(&tokens).unwrap();
        assert_eq!(result, DateExpr::Range(RangeExpr::ThisYear));
    }

    // ── Phase 8: Operator-prefixed offset, boundary, compound, Nh tests ──

    /// Helper: tokenize and parse to DateExpr.
    fn parse_expr(input: &str) -> Result<DateExpr, ParseError> {
        let tokens = crate::parser::lexer::tokenize(input);
        let mut parser = Parser::new(&tokens, input);
        parser.parse_expression()
    }

    #[test]
    fn test_operator_prefixed_plus_hours() {
        let result = parse_expr("+3h").unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Future,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Hour,
                }]
            )
        );
    }

    #[test]
    fn test_operator_prefixed_minus_days() {
        let result = parse_expr("-1d").unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Past,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Day,
                }]
            )
        );
    }

    #[test]
    fn test_operator_prefixed_compound() {
        let result = parse_expr("+1h30min").unwrap();
        assert_eq!(
            result,
            DateExpr::Offset(
                Direction::Future,
                vec![
                    DurationComponent {
                        count: 1,
                        unit: TemporalUnit::Hour,
                    },
                    DurationComponent {
                        count: 30,
                        unit: TemporalUnit::Minute,
                    },
                ]
            )
        );
    }

    #[test]
    fn test_nhmm_inferred_minutes() {
        // "now+13h30" -> Arithmetic(Now, Add, [13 Hour, 30 Minute])
        let result = parse_expr("now+13h30").unwrap();
        assert_eq!(
            result,
            DateExpr::Arithmetic(
                Box::new(DateExpr::Now),
                ArithOp::Add,
                vec![
                    DurationComponent {
                        count: 13,
                        unit: TemporalUnit::Hour,
                    },
                    DurationComponent {
                        count: 30,
                        unit: TemporalUnit::Minute,
                    },
                ]
            )
        );
    }

    #[test]
    fn test_colon_duration_in_arithmetic() {
        // "now+13:30" -> Arithmetic(Now, Add, [13 Hour, 30 Minute])
        let result = parse_expr("now+13:30").unwrap();
        assert_eq!(
            result,
            DateExpr::Arithmetic(
                Box::new(DateExpr::Now),
                ArithOp::Add,
                vec![
                    DurationComponent {
                        count: 13,
                        unit: TemporalUnit::Hour,
                    },
                    DurationComponent {
                        count: 30,
                        unit: TemporalUnit::Minute,
                    },
                ]
            )
        );
    }

    #[test]
    fn test_boundary_keyword_eod() {
        let result = parse_expr("eod").unwrap();
        assert_eq!(
            result,
            DateExpr::Boundary(crate::parser::token::BoundaryKind::Eod)
        );
    }

    #[test]
    fn test_boundary_with_arithmetic() {
        // "eod+1h" -> Arithmetic(Boundary(Eod), Add, [1 Hour])
        let result = parse_expr("eod+1h").unwrap();
        assert_eq!(
            result,
            DateExpr::Arithmetic(
                Box::new(DateExpr::Boundary(crate::parser::token::BoundaryKind::Eod)),
                ArithOp::Add,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Hour,
                }]
            )
        );
    }

    #[test]
    fn test_time_suffix_nh() {
        // "today 18h" -> Relative(Today, Some(HourOnly(18)))
        let result = parse_expr("today 18h").unwrap();
        assert!(matches!(
            result,
            DateExpr::Relative(RelativeDate::Today, Some(TimeExpr::HourOnly(18)))
        ));
    }

    #[test]
    fn test_time_suffix_at_nh() {
        // "today at 18h" -> Relative(Today, Some(HourOnly(18)))
        let result = parse_expr("today at 18h").unwrap();
        assert!(matches!(
            result,
            DateExpr::Relative(RelativeDate::Today, Some(TimeExpr::HourOnly(18)))
        ));
    }

    #[test]
    fn test_bare_duration_still_errors() {
        // "3h" alone (no operator, no day context) should error
        let result = parse_expr("3h");
        assert!(result.is_err(), "bare '3h' without operator should error");
    }

    #[test]
    fn test_operator_without_unit_errors() {
        // "+1" (number without unit after operator) should error
        let result = parse_expr("+1");
        assert!(
            result.is_err(),
            "'+1' without unit should error, got: {result:?}"
        );
    }

    // ── AM/PM tests ─────────────────────────────────────────────

    #[test]
    fn standalone_3pm_is_error() {
        assert!(parse_expr("3pm").is_err());
    }

    #[test]
    fn standalone_3am_is_error() {
        assert!(parse_expr("3am").is_err());
    }

    #[test]
    fn standalone_3_30pm_is_error() {
        assert!(parse_expr("3:30pm").is_err());
    }

    #[test]
    fn standalone_12am_is_error() {
        assert!(parse_expr("12am").is_err());
    }

    #[test]
    fn standalone_12pm_is_error() {
        assert!(parse_expr("12pm").is_err());
    }

    #[test]
    fn standalone_15_30_is_error() {
        assert!(parse_expr("15:30").is_err());
    }

    #[test]
    fn standalone_15h_is_error() {
        assert!(parse_expr("15h").is_err());
    }

    #[test]
    fn standalone_15h30_is_error() {
        assert!(parse_expr("15h30").is_err());
    }

    // ── Notation equivalence: all time styles produce the same result ──

    #[test]
    fn notation_equivalence_hour_only() {
        // "tomorrow 15h" = "tomorrow 3pm" = "tomorrow 3 pm" → HourOnly(15)
        // "tomorrow 15:00" → HourMinute(15, 0) — different granularity, intentional
        let a = parse_expr("tomorrow 15h").unwrap();
        let b = parse_expr("tomorrow 3pm").unwrap();
        let c = parse_expr("tomorrow 3 pm").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
        // Verify it's HourOnly, not HourMinute
        assert_eq!(
            a,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourOnly(15)))
        );
    }

    #[test]
    fn notation_equivalence_hour_minute() {
        // "tomorrow 15h30" = "tomorrow 15:30" = "tomorrow 3:30pm" → HourMinute(15, 30)
        let a = parse_expr("tomorrow 15h30").unwrap();
        let b = parse_expr("tomorrow 15:30").unwrap();
        let c = parse_expr("tomorrow 3:30pm").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(
            a,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourMinute(15, 30)))
        );
    }

    #[test]
    fn notation_equivalence_explicit_zero_minute() {
        // "tomorrow 15:00" = "tomorrow 3:00pm" → HourMinute(15, 0)
        // These explicitly specify minute=0, so HourMinute not HourOnly
        let a = parse_expr("tomorrow 15:00").unwrap();
        let b = parse_expr("tomorrow 3:00pm").unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourMinute(15, 0)))
        );
    }

    #[test]
    fn notation_equivalence_midnight() {
        // "tomorrow 0h" = "tomorrow 12am" → HourOnly(0)
        let a = parse_expr("tomorrow 0h").unwrap();
        let b = parse_expr("tomorrow 12am").unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourOnly(0)))
        );
    }

    #[test]
    fn notation_equivalence_noon() {
        // "tomorrow 12h" = "tomorrow 12pm" → HourOnly(12)
        let a = parse_expr("tomorrow 12h").unwrap();
        let b = parse_expr("tomorrow 12pm").unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourOnly(12)))
        );
    }

    #[test]
    fn notation_equivalence_with_at() {
        // "tomorrow at 15h" = "tomorrow at 3pm" → HourOnly(15)
        let a = parse_expr("tomorrow at 15h").unwrap();
        let b = parse_expr("tomorrow at 3pm").unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourOnly(15)))
        );
    }

    #[test]
    fn notation_equivalence_day_ref() {
        // "next friday 15h30" = "next friday 15:30" = "next friday 3:30pm" → HourMinute(15, 30)
        let a = parse_expr("next friday 15h30").unwrap();
        let b = parse_expr("next friday 15:30").unwrap();
        let c = parse_expr("next friday 3:30pm").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn test_tomorrow_at_3pm() {
        // "tomorrow at 3pm" -> Relative(Tomorrow, Some(HourOnly(15)))
        // Bare pm = no explicit minutes → HourOnly
        let result = parse_expr("tomorrow at 3pm").unwrap();
        assert_eq!(
            result,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourOnly(15)))
        );
    }

    #[test]
    fn test_next_friday_at_3_30pm() {
        // "next friday at 3:30pm" -> DayRef(Next, Friday, Some(HourMinute(15, 30)))
        // Explicit minutes → HourMinute
        let result = parse_expr("next friday at 3:30pm").unwrap();
        assert_eq!(
            result,
            DateExpr::DayRef(
                Direction::Next,
                Weekday::Friday,
                Some(TimeExpr::HourMinute(15, 30)),
            )
        );
    }

    #[test]
    fn test_today_3pm() {
        // "today 3pm" -> Relative(Today, Some(HourOnly(15)))
        let result = parse_expr("today 3pm").unwrap();
        assert_eq!(
            result,
            DateExpr::Relative(RelativeDate::Today, Some(TimeExpr::HourOnly(15)))
        );
    }

    // ── SameTime tests ──────────────────────────────────────────

    #[test]
    fn test_tomorrow_at_same_time() {
        // "tomorrow at same time" -> Relative(Tomorrow, Some(SameTime))
        let result = parse_expr("tomorrow at same time").unwrap();
        assert_eq!(
            result,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::SameTime))
        );
    }

    #[test]
    fn test_next_friday_at_same_time() {
        // "next friday at same time" -> DayRef(Next, Friday, Some(SameTime))
        let result = parse_expr("next friday at same time").unwrap();
        assert_eq!(
            result,
            DateExpr::DayRef(Direction::Next, Weekday::Friday, Some(TimeExpr::SameTime),)
        );
    }

    #[test]
    fn test_yesterday_at_same_time() {
        // "yesterday at same time" -> Relative(Yesterday, Some(SameTime))
        let result = parse_expr("yesterday at same time").unwrap();
        assert_eq!(
            result,
            DateExpr::Relative(RelativeDate::Yesterday, Some(TimeExpr::SameTime))
        );
    }
}
