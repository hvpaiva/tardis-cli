//! Locale infrastructure for the TARDIS natural-language date parser.
//!
//! Each locale implements [`Locale`], providing keyword tables that map
//! lowercased (and accent-stripped) strings to `Token` variants.
//! [`LocaleKeywords`] wraps these tables into a `HashMap` for O(1) lookup.
//!
//! Locale resolution follows D-06 precedence:
//! `--locale` flag > config `locale` > `LANG`/`LC_TIME` env > English fallback.

pub mod en;

use std::collections::HashMap;

use crate::parser::token::Token;

/// Trait for locale definitions. Each locale provides keyword tables that
/// the lexer uses for keyword recognition. Grammar and resolver remain
/// locale-agnostic -- they operate on Token/AST types, not strings.
///
/// All data is compile-time const (D-01): no runtime file I/O.
pub trait Locale: Send + Sync {
    /// Human-readable name (e.g., "English", "Portugues").
    fn name(&self) -> &'static str;

    /// BCP 47 language code (e.g., "en", "pt").
    fn code(&self) -> &'static str;

    /// Keyword table: lowercased (accent-stripped) string -> Token mapping.
    /// Each entry maps a normalized keyword to its Token variant.
    fn keywords(&self) -> &'static [(&'static str, Token)];

    /// Multi-word patterns that the lexer should merge into single tokens.
    /// Each entry maps a sequence of normalized words to a Token.
    /// Default: empty (EN has no multi-word patterns).
    fn multi_word_patterns(&self) -> &'static [(&'static [&'static str], Token)] {
        &[]
    }

    /// Articles that mean "1" (like "a"/"an" in EN, "um"/"uma" in PT).
    /// Default: empty.
    fn articles(&self) -> &'static [&'static str] {
        &[]
    }
}

/// Wrapper around a `HashMap<String, Token>` built from a [`Locale`]'s
/// keyword table. Provides O(1) keyword lookup for the lexer.
pub struct LocaleKeywords {
    map: HashMap<String, Token>,
    locale: &'static dyn Locale,
}

impl LocaleKeywords {
    /// Build a `LocaleKeywords` from a locale's keyword table.
    pub fn from_locale(locale: &'static dyn Locale) -> Self {
        let keywords = locale.keywords();
        let mut map = HashMap::with_capacity(keywords.len());
        for &(word, ref token) in keywords {
            map.insert(word.to_string(), token.clone());
        }
        Self { map, locale }
    }

    /// Look up a normalized (lowercased + accent-stripped) word.
    /// Returns the matched token or `Token::Word(original)` for unrecognized words.
    pub fn lookup(&self, normalized: &str, original: &str) -> Token {
        self.map
            .get(normalized)
            .cloned()
            .unwrap_or_else(|| Token::Word(original.to_string()))
    }

    /// Return all keyword strings for the suggestion engine.
    pub fn all_keywords(&self) -> Vec<String> {
        self.map.keys().cloned().collect()
    }

    /// Delegate to the locale's multi-word patterns.
    pub fn multi_word_patterns(&self) -> &'static [(&'static [&'static str], Token)] {
        self.locale.multi_word_patterns()
    }
}

/// Strip diacritics for accent-insensitive matching (D-04).
/// Covers Latin-1 supplement characters used in Portuguese.
/// All listed characters have clean Unicode canonical decomposition.
pub fn strip_diacritics(c: char) -> char {
    match c {
        '\u{00e1}' | '\u{00e0}' | '\u{00e2}' | '\u{00e3}' => 'a', // a with acute/grave/circumflex/tilde
        '\u{00e9}' | '\u{00ea}' => 'e', // e with acute/circumflex
        '\u{00ed}' => 'i',              // i with acute
        '\u{00f3}' | '\u{00f4}' | '\u{00f5}' => 'o', // o with acute/circumflex/tilde
        '\u{00fa}' | '\u{00fc}' => 'u', // u with acute/diaeresis
        '\u{00e7}' => 'c',              // c with cedilla
        // Uppercase variants (mapped to lowercase base)
        '\u{00c1}' | '\u{00c0}' | '\u{00c2}' | '\u{00c3}' => 'a',
        '\u{00c9}' | '\u{00ca}' => 'e',
        '\u{00cd}' => 'i',
        '\u{00d3}' | '\u{00d4}' | '\u{00d5}' => 'o',
        '\u{00da}' | '\u{00dc}' => 'u',
        '\u{00c7}' => 'c',
        other => other,
    }
}

/// Detect locale from environment variables (D-05).
/// Checks `LC_TIME` first (more specific), then `LANG`.
/// Extracts the language prefix (e.g., "pt_BR.UTF-8" -> "pt").
/// Returns "en" as the default fallback.
pub fn detect_locale_from_env() -> &'static str {
    for var in &["LC_TIME", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if val.is_empty() {
                continue;
            }
            // Extract language prefix: "pt_BR.UTF-8" -> "pt"
            let lower = val.to_lowercase();
            let lang = lower
                .split(|c: char| c == '_' || c == '-' || c == '.')
                .next()
                .unwrap_or("");
            if lang == "pt" {
                return "pt";
            }
            // Could add more locale prefixes here as locales are added
        }
    }
    "en"
}

/// Resolve the effective locale following D-06 precedence:
/// `cli_locale` > `config_locale` > `detect_locale_from_env()` > English.
pub fn resolve_locale(
    cli_locale: Option<&str>,
    config_locale: Option<&str>,
) -> &'static dyn Locale {
    let code = cli_locale
        .filter(|s| !s.is_empty())
        .or_else(|| config_locale.filter(|s| !s.is_empty()))
        .unwrap_or_else(|| detect_locale_from_env());

    get_locale(code)
}

/// Look up a locale by its code. Returns English for unrecognized codes.
pub fn get_locale(code: &str) -> &'static dyn Locale {
    match code.to_lowercase().as_str() {
        // PT locale not yet implemented; will be added in Plan 04-02
        "pt" | "pt-br" | "pt_br" => &en::EN_LOCALE, // TODO: return &pt::PT_LOCALE
        _ => &en::EN_LOCALE,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::parser::token::{TemporalUnit, Token};
    use serial_test::serial;
    use std::ffi::OsString;

    struct EnvGuard {
        key: &'static str,
        prior: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl Into<OsString>) -> Self {
            let prior = std::env::var_os(key);
            unsafe { std::env::set_var(key, value.into()) };
            Self { key, prior }
        }

        fn remove(key: &'static str) -> Self {
            let prior = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, prior }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prior {
                Some(val) => unsafe { std::env::set_var(self.key, val) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    // --- strip_diacritics tests ---

    #[test]
    fn strip_diacritics_pt_accented_chars() {
        // a with various accents
        assert_eq!(strip_diacritics('\u{00e1}'), 'a'); // a acute
        assert_eq!(strip_diacritics('\u{00e0}'), 'a'); // a grave
        assert_eq!(strip_diacritics('\u{00e2}'), 'a'); // a circumflex
        assert_eq!(strip_diacritics('\u{00e3}'), 'a'); // a tilde
        // e with accents
        assert_eq!(strip_diacritics('\u{00e9}'), 'e'); // e acute
        assert_eq!(strip_diacritics('\u{00ea}'), 'e'); // e circumflex
        // i with accent
        assert_eq!(strip_diacritics('\u{00ed}'), 'i'); // i acute
        // o with various accents
        assert_eq!(strip_diacritics('\u{00f3}'), 'o'); // o acute
        assert_eq!(strip_diacritics('\u{00f4}'), 'o'); // o circumflex
        assert_eq!(strip_diacritics('\u{00f5}'), 'o'); // o tilde
        // u with accents
        assert_eq!(strip_diacritics('\u{00fa}'), 'u'); // u acute
        assert_eq!(strip_diacritics('\u{00fc}'), 'u'); // u diaeresis
        // c with cedilla
        assert_eq!(strip_diacritics('\u{00e7}'), 'c'); // c cedilla
    }

    #[test]
    fn strip_diacritics_uppercase_variants() {
        assert_eq!(strip_diacritics('\u{00c1}'), 'a'); // A acute
        assert_eq!(strip_diacritics('\u{00c0}'), 'a'); // A grave
        assert_eq!(strip_diacritics('\u{00c2}'), 'a'); // A circumflex
        assert_eq!(strip_diacritics('\u{00c3}'), 'a'); // A tilde
        assert_eq!(strip_diacritics('\u{00c9}'), 'e'); // E acute
        assert_eq!(strip_diacritics('\u{00ca}'), 'e'); // E circumflex
        assert_eq!(strip_diacritics('\u{00cd}'), 'i'); // I acute
        assert_eq!(strip_diacritics('\u{00d3}'), 'o'); // O acute
        assert_eq!(strip_diacritics('\u{00d4}'), 'o'); // O circumflex
        assert_eq!(strip_diacritics('\u{00d5}'), 'o'); // O tilde
        assert_eq!(strip_diacritics('\u{00da}'), 'u'); // U acute
        assert_eq!(strip_diacritics('\u{00dc}'), 'u'); // U diaeresis
        assert_eq!(strip_diacritics('\u{00c7}'), 'c'); // C cedilla
    }

    #[test]
    fn strip_diacritics_passthrough() {
        assert_eq!(strip_diacritics('a'), 'a');
        assert_eq!(strip_diacritics('z'), 'z');
        assert_eq!(strip_diacritics('0'), '0');
        assert_eq!(strip_diacritics(' '), ' ');
    }

    // --- detect_locale_from_env tests ---

    #[test]
    #[serial]
    fn detect_locale_from_lc_time_pt() {
        let _lc = EnvGuard::set("LC_TIME", "pt_BR.UTF-8");
        let _lang = EnvGuard::remove("LANG");
        assert_eq!(detect_locale_from_env(), "pt");
    }

    #[test]
    #[serial]
    fn detect_locale_from_lang_pt() {
        let _lc = EnvGuard::remove("LC_TIME");
        let _lang = EnvGuard::set("LANG", "pt_BR.UTF-8");
        assert_eq!(detect_locale_from_env(), "pt");
    }

    #[test]
    #[serial]
    fn detect_locale_lc_time_takes_precedence_over_lang() {
        let _lc = EnvGuard::set("LC_TIME", "pt_BR.UTF-8");
        let _lang = EnvGuard::set("LANG", "en_US.UTF-8");
        assert_eq!(detect_locale_from_env(), "pt");
    }

    #[test]
    #[serial]
    fn detect_locale_defaults_to_en() {
        let _lc = EnvGuard::remove("LC_TIME");
        let _lang = EnvGuard::remove("LANG");
        assert_eq!(detect_locale_from_env(), "en");
    }

    #[test]
    #[serial]
    fn detect_locale_en_us_returns_en() {
        let _lc = EnvGuard::remove("LC_TIME");
        let _lang = EnvGuard::set("LANG", "en_US.UTF-8");
        assert_eq!(detect_locale_from_env(), "en");
    }

    #[test]
    #[serial]
    fn detect_locale_empty_env_returns_en() {
        let _lc = EnvGuard::set("LC_TIME", "");
        let _lang = EnvGuard::set("LANG", "");
        assert_eq!(detect_locale_from_env(), "en");
    }

    // --- resolve_locale tests ---

    #[test]
    #[serial]
    fn resolve_locale_cli_overrides_all() {
        let _lc = EnvGuard::set("LC_TIME", "pt_BR.UTF-8");
        let locale = resolve_locale(Some("en"), Some("pt"));
        assert_eq!(locale.code(), "en");
    }

    #[test]
    #[serial]
    fn resolve_locale_config_overrides_env() {
        let _lc = EnvGuard::remove("LC_TIME");
        let _lang = EnvGuard::set("LANG", "pt_BR.UTF-8");
        let locale = resolve_locale(None, Some("en"));
        assert_eq!(locale.code(), "en");
    }

    #[test]
    #[serial]
    fn resolve_locale_env_used_when_no_cli_or_config() {
        let _lc = EnvGuard::set("LC_TIME", "pt_BR.UTF-8");
        let _lang = EnvGuard::remove("LANG");
        let locale = resolve_locale(None, None);
        // PT not yet implemented, falls back to EN
        assert_eq!(locale.code(), "en");
    }

    #[test]
    #[serial]
    fn resolve_locale_defaults_to_english() {
        let _lc = EnvGuard::remove("LC_TIME");
        let _lang = EnvGuard::remove("LANG");
        let locale = resolve_locale(None, None);
        assert_eq!(locale.code(), "en");
    }

    #[test]
    #[serial]
    fn resolve_locale_empty_cli_falls_through() {
        let _lc = EnvGuard::remove("LC_TIME");
        let _lang = EnvGuard::remove("LANG");
        let locale = resolve_locale(Some(""), Some("en"));
        assert_eq!(locale.code(), "en");
    }

    // --- LocaleKeywords tests ---

    #[test]
    fn locale_keywords_from_locale_builds_map() {
        let kw = LocaleKeywords::from_locale(&en::EN_LOCALE);
        assert_eq!(kw.lookup("now", "now"), Token::Now);
        assert_eq!(kw.lookup("today", "today"), Token::Today);
        assert_eq!(kw.lookup("tomorrow", "tomorrow"), Token::Tomorrow);
    }

    #[test]
    fn locale_keywords_lookup_returns_word_for_unknown() {
        let kw = LocaleKeywords::from_locale(&en::EN_LOCALE);
        assert_eq!(
            kw.lookup("xyzzy", "Xyzzy"),
            Token::Word("Xyzzy".to_string())
        );
    }

    #[test]
    fn locale_keywords_all_keywords_includes_entries() {
        let kw = LocaleKeywords::from_locale(&en::EN_LOCALE);
        let all = kw.all_keywords();
        assert!(all.contains(&"now".to_string()));
        assert!(all.contains(&"today".to_string()));
        assert!(all.contains(&"monday".to_string()));
        assert!(all.len() >= 72); // EN has 72 keywords
    }

    #[test]
    fn locale_keywords_weekdays_mapped() {
        let kw = LocaleKeywords::from_locale(&en::EN_LOCALE);
        assert_eq!(
            kw.lookup("monday", "monday"),
            Token::Weekday(jiff::civil::Weekday::Monday)
        );
        assert_eq!(
            kw.lookup("fri", "fri"),
            Token::Weekday(jiff::civil::Weekday::Friday)
        );
    }

    #[test]
    fn locale_keywords_units_mapped() {
        let kw = LocaleKeywords::from_locale(&en::EN_LOCALE);
        assert_eq!(
            kw.lookup("hour", "hour"),
            Token::Unit(TemporalUnit::Hour)
        );
        assert_eq!(
            kw.lookup("minutes", "minutes"),
            Token::Unit(TemporalUnit::Minute)
        );
    }

    // --- Extensibility test ---

    #[test]
    fn mock_locale_works_with_locale_keywords() {
        struct MockLocale;
        impl Locale for MockLocale {
            fn name(&self) -> &'static str {
                "Mock"
            }
            fn code(&self) -> &'static str {
                "xx"
            }
            fn keywords(&self) -> &'static [(&'static str, Token)] {
                &[("foo", Token::Now), ("bar", Token::Today)]
            }
        }

        static MOCK: MockLocale = MockLocale;
        let kw = LocaleKeywords::from_locale(&MOCK);
        assert_eq!(kw.lookup("foo", "foo"), Token::Now);
        assert_eq!(kw.lookup("bar", "bar"), Token::Today);
        assert_eq!(
            kw.lookup("baz", "Baz"),
            Token::Word("Baz".to_string())
        );
    }

    #[test]
    fn get_locale_returns_en_for_unknown() {
        let locale = get_locale("zz");
        assert_eq!(locale.code(), "en");
    }

    #[test]
    fn get_locale_returns_en_for_en() {
        let locale = get_locale("en");
        assert_eq!(locale.code(), "en");
    }
}
