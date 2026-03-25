//! English locale for the TARDIS date parser.
//!
//! All keywords are extracted from the former `match_keyword()` function
//! in `src/parser/lexer.rs`. The keyword table here must produce identical
//! tokens to the old hardcoded match for every keyword (LOCL-02).

use crate::parser::token::{TemporalUnit, Token};

use super::Locale;

/// English locale implementation.
pub struct EnglishLocale;

impl Locale for EnglishLocale {
    fn name(&self) -> &'static str {
        "English"
    }

    fn code(&self) -> &'static str {
        "en"
    }

    fn keywords(&self) -> &'static [(&'static str, Token)] {
        &EN_KEYWORDS
    }

    fn multi_word_patterns(&self) -> &'static [(&'static [&'static str], Token)] {
        &[] // English has no multi-word patterns
    }

    fn articles(&self) -> &'static [&'static str] {
        &["a", "an"]
    }
}

/// Static English locale instance.
pub static EN_LOCALE: EnglishLocale = EnglishLocale;

/// Complete English keyword table. Every entry here corresponds exactly to
/// a match arm in the old `match_keyword()` function (lexer.rs:300-364).
///
/// IMPORTANT: This table must be kept in sync with Token variants.
/// Adding a keyword here and not in the Token enum (or vice versa) will
/// cause a compile error.
static EN_KEYWORDS: [(&str, Token); 72] = [
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
];

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::locale::LocaleKeywords;

    /// Verify that the EN locale keyword table produces the same Token for
    /// every keyword that the old `match_keyword()` function returned.
    /// This is the regression safety test for LOCL-02.
    #[test]
    fn en_locale_matches_old_match_keyword_for_all_keywords() {
        let kw = LocaleKeywords::from_locale(&EN_LOCALE);

        // Relative keywords
        assert_eq!(kw.lookup("now", "now"), Token::Now);
        assert_eq!(kw.lookup("today", "today"), Token::Today);
        assert_eq!(kw.lookup("tomorrow", "tomorrow"), Token::Tomorrow);
        assert_eq!(kw.lookup("yesterday", "yesterday"), Token::Yesterday);
        assert_eq!(kw.lookup("overmorrow", "overmorrow"), Token::Overmorrow);

        // Direction modifiers
        assert_eq!(kw.lookup("next", "next"), Token::Next);
        assert_eq!(kw.lookup("last", "last"), Token::Last);
        assert_eq!(kw.lookup("this", "this"), Token::This);
        assert_eq!(kw.lookup("in", "in"), Token::In);
        assert_eq!(kw.lookup("ago", "ago"), Token::Ago);
        assert_eq!(kw.lookup("from", "from"), Token::From);

        // Verbal arithmetic
        assert_eq!(kw.lookup("after", "after"), Token::After);
        assert_eq!(kw.lookup("before", "before"), Token::Before);

        // Articles
        assert_eq!(kw.lookup("a", "a"), Token::A);
        assert_eq!(kw.lookup("an", "an"), Token::An);

        // Connectors
        assert_eq!(kw.lookup("at", "at"), Token::At);
        assert_eq!(kw.lookup("and", "and"), Token::And);

        // Weekdays (full)
        assert_eq!(kw.lookup("monday", "monday"), Token::Weekday(jiff::civil::Weekday::Monday));
        assert_eq!(kw.lookup("tuesday", "tuesday"), Token::Weekday(jiff::civil::Weekday::Tuesday));
        assert_eq!(kw.lookup("wednesday", "wednesday"), Token::Weekday(jiff::civil::Weekday::Wednesday));
        assert_eq!(kw.lookup("thursday", "thursday"), Token::Weekday(jiff::civil::Weekday::Thursday));
        assert_eq!(kw.lookup("friday", "friday"), Token::Weekday(jiff::civil::Weekday::Friday));
        assert_eq!(kw.lookup("saturday", "saturday"), Token::Weekday(jiff::civil::Weekday::Saturday));
        assert_eq!(kw.lookup("sunday", "sunday"), Token::Weekday(jiff::civil::Weekday::Sunday));

        // Weekdays (abbreviated)
        assert_eq!(kw.lookup("mon", "mon"), Token::Weekday(jiff::civil::Weekday::Monday));
        assert_eq!(kw.lookup("tue", "tue"), Token::Weekday(jiff::civil::Weekday::Tuesday));
        assert_eq!(kw.lookup("wed", "wed"), Token::Weekday(jiff::civil::Weekday::Wednesday));
        assert_eq!(kw.lookup("thu", "thu"), Token::Weekday(jiff::civil::Weekday::Thursday));
        assert_eq!(kw.lookup("fri", "fri"), Token::Weekday(jiff::civil::Weekday::Friday));
        assert_eq!(kw.lookup("sat", "sat"), Token::Weekday(jiff::civil::Weekday::Saturday));
        assert_eq!(kw.lookup("sun", "sun"), Token::Weekday(jiff::civil::Weekday::Sunday));

        // Months (full)
        assert_eq!(kw.lookup("january", "january"), Token::Month(1));
        assert_eq!(kw.lookup("february", "february"), Token::Month(2));
        assert_eq!(kw.lookup("march", "march"), Token::Month(3));
        assert_eq!(kw.lookup("april", "april"), Token::Month(4));
        assert_eq!(kw.lookup("may", "may"), Token::Month(5));
        assert_eq!(kw.lookup("june", "june"), Token::Month(6));
        assert_eq!(kw.lookup("july", "july"), Token::Month(7));
        assert_eq!(kw.lookup("august", "august"), Token::Month(8));
        assert_eq!(kw.lookup("september", "september"), Token::Month(9));
        assert_eq!(kw.lookup("october", "october"), Token::Month(10));
        assert_eq!(kw.lookup("november", "november"), Token::Month(11));
        assert_eq!(kw.lookup("december", "december"), Token::Month(12));

        // Months (abbreviated)
        assert_eq!(kw.lookup("jan", "jan"), Token::Month(1));
        assert_eq!(kw.lookup("feb", "feb"), Token::Month(2));
        assert_eq!(kw.lookup("mar", "mar"), Token::Month(3));
        assert_eq!(kw.lookup("apr", "apr"), Token::Month(4));
        assert_eq!(kw.lookup("jun", "jun"), Token::Month(6));
        assert_eq!(kw.lookup("jul", "jul"), Token::Month(7));
        assert_eq!(kw.lookup("aug", "aug"), Token::Month(8));
        assert_eq!(kw.lookup("sep", "sep"), Token::Month(9));
        assert_eq!(kw.lookup("oct", "oct"), Token::Month(10));
        assert_eq!(kw.lookup("nov", "nov"), Token::Month(11));
        assert_eq!(kw.lookup("dec", "dec"), Token::Month(12));

        // Temporal units
        assert_eq!(kw.lookup("year", "year"), Token::Unit(TemporalUnit::Year));
        assert_eq!(kw.lookup("years", "years"), Token::Unit(TemporalUnit::Year));
        assert_eq!(kw.lookup("month", "month"), Token::Unit(TemporalUnit::Month));
        assert_eq!(kw.lookup("months", "months"), Token::Unit(TemporalUnit::Month));
        assert_eq!(kw.lookup("week", "week"), Token::Unit(TemporalUnit::Week));
        assert_eq!(kw.lookup("weeks", "weeks"), Token::Unit(TemporalUnit::Week));
        assert_eq!(kw.lookup("day", "day"), Token::Unit(TemporalUnit::Day));
        assert_eq!(kw.lookup("days", "days"), Token::Unit(TemporalUnit::Day));
        assert_eq!(kw.lookup("hour", "hour"), Token::Unit(TemporalUnit::Hour));
        assert_eq!(kw.lookup("hours", "hours"), Token::Unit(TemporalUnit::Hour));
        assert_eq!(kw.lookup("minute", "minute"), Token::Unit(TemporalUnit::Minute));
        assert_eq!(kw.lookup("minutes", "minutes"), Token::Unit(TemporalUnit::Minute));
        assert_eq!(kw.lookup("min", "min"), Token::Unit(TemporalUnit::Minute));
        assert_eq!(kw.lookup("mins", "mins"), Token::Unit(TemporalUnit::Minute));
        assert_eq!(kw.lookup("second", "second"), Token::Unit(TemporalUnit::Second));
        assert_eq!(kw.lookup("seconds", "seconds"), Token::Unit(TemporalUnit::Second));
        assert_eq!(kw.lookup("sec", "sec"), Token::Unit(TemporalUnit::Second));
        assert_eq!(kw.lookup("secs", "secs"), Token::Unit(TemporalUnit::Second));
    }

    #[test]
    fn en_locale_unknown_word_returns_word_token() {
        let kw = LocaleKeywords::from_locale(&EN_LOCALE);
        assert_eq!(
            kw.lookup("thursdya", "Thursdya"),
            Token::Word("Thursdya".to_string())
        );
    }

    #[test]
    fn en_locale_articles_returns_a_and_an() {
        assert_eq!(EN_LOCALE.articles(), &["a", "an"]);
    }

    #[test]
    fn en_locale_has_no_multi_word_patterns() {
        assert!(EN_LOCALE.multi_word_patterns().is_empty());
    }

    #[test]
    fn en_locale_keyword_count() {
        // Verify we have all 72 keywords from the old match_keyword
        assert_eq!(EN_KEYWORDS.len(), 72);
    }

    #[test]
    fn en_locale_name_and_code() {
        assert_eq!(EN_LOCALE.name(), "English");
        assert_eq!(EN_LOCALE.code(), "en");
    }
}
