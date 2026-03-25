//! Portuguese (Brazilian) locale for the TARDIS date parser.
//!
//! All keywords are lowercased and accent-stripped for lookup.
//! Gender variants (proxima/proximo, ultima/ultimo) are all mapped
//! to the same Token variant. Accent-insensitive matching is handled
//! by the lexer's `strip_diacritics` normalization pass.
//!
//! Multi-word patterns handle PT constructs like "daqui a" -> In,
//! "depois de amanha" -> Overmorrow, "antes de ontem" -> Ereyesterday.

use crate::parser::token::{TemporalUnit, Token};

use super::Locale;

/// Portuguese (Brazilian) locale implementation.
pub struct PortugueseLocale;

impl Locale for PortugueseLocale {
    fn name(&self) -> &'static str {
        "Portugues"
    }

    fn code(&self) -> &'static str {
        "pt"
    }

    fn keywords(&self) -> &'static [(&'static str, Token)] {
        &PT_KEYWORDS
    }

    fn multi_word_patterns(&self) -> &'static [(&'static [&'static str], Token)] {
        &PT_MULTI_WORD_PATTERNS
    }

    fn articles(&self) -> &'static [&'static str] {
        &["um", "uma"]
    }
}

/// Static Portuguese locale instance.
pub static PT_LOCALE: PortugueseLocale = PortugueseLocale;

/// Complete Portuguese keyword table. All keys are lowercased and accent-stripped.
///
/// IMPORTANT: "ago" maps to Token::Month(8) in PT (agosto abbreviation),
/// not Token::Ago. PT uses "ha"/"atras" for the ago direction.
static PT_KEYWORDS: [(&str, Token); 89] = [
    // Relative keywords
    ("agora", Token::Now),
    ("hoje", Token::Today),
    ("amanha", Token::Tomorrow),
    ("ontem", Token::Yesterday),
    ("anteontem", Token::Ereyesterday),
    // Direction modifiers (gender variants)
    ("proxima", Token::Next),
    ("proximo", Token::Next),
    ("proximas", Token::Next),
    ("proximos", Token::Next),
    ("ultima", Token::Last),
    ("ultimo", Token::Last),
    ("ultimas", Token::Last),
    ("ultimos", Token::Last),
    ("passada", Token::Last),
    ("passado", Token::Last),
    ("esta", Token::This),
    ("este", Token::This),
    ("estas", Token::This),
    ("estes", Token::This),
    // Duration direction
    ("ha", Token::Ago),    // prefix-ago: "ha 2 horas" = "2 hours ago"
    ("atras", Token::Ago), // postfix: "3 dias atras" = "3 days ago"
    // Duration preposition
    ("em", Token::In), // "em 5 minutos" = "in 5 minutes"
    // Verbal arithmetic
    ("depois", Token::After),
    ("antes", Token::Before),
    ("mais", Token::Plus),
    ("menos", Token::Dash),
    // Connectors
    ("e", Token::And),
    ("as", Token::At), // "as 15:00" = "at 15:00"
    // Articles (gendered, meaning "1")
    ("um", Token::A),
    ("uma", Token::A),
    // Weekdays (full)
    ("segunda", Token::Weekday(jiff::civil::Weekday::Monday)),
    ("terca", Token::Weekday(jiff::civil::Weekday::Tuesday)),
    ("quarta", Token::Weekday(jiff::civil::Weekday::Wednesday)),
    ("quinta", Token::Weekday(jiff::civil::Weekday::Thursday)),
    ("sexta", Token::Weekday(jiff::civil::Weekday::Friday)),
    ("sabado", Token::Weekday(jiff::civil::Weekday::Saturday)),
    ("domingo", Token::Weekday(jiff::civil::Weekday::Sunday)),
    // Weekdays (abbreviated)
    ("seg", Token::Weekday(jiff::civil::Weekday::Monday)),
    ("ter", Token::Weekday(jiff::civil::Weekday::Tuesday)),
    ("qua", Token::Weekday(jiff::civil::Weekday::Wednesday)),
    ("qui", Token::Weekday(jiff::civil::Weekday::Thursday)),
    ("sex", Token::Weekday(jiff::civil::Weekday::Friday)),
    ("sab", Token::Weekday(jiff::civil::Weekday::Saturday)),
    ("dom", Token::Weekday(jiff::civil::Weekday::Sunday)),
    // Months (full)
    ("janeiro", Token::Month(1)),
    ("fevereiro", Token::Month(2)),
    ("marco", Token::Month(3)),
    ("abril", Token::Month(4)),
    ("maio", Token::Month(5)),
    ("junho", Token::Month(6)),
    ("julho", Token::Month(7)),
    ("agosto", Token::Month(8)),
    ("setembro", Token::Month(9)),
    ("outubro", Token::Month(10)),
    ("novembro", Token::Month(11)),
    ("dezembro", Token::Month(12)),
    // Months (abbreviated) -- "ago" maps to Month(8), not Ago!
    ("jan", Token::Month(1)),
    ("fev", Token::Month(2)),
    ("mar", Token::Month(3)),
    ("abr", Token::Month(4)),
    ("mai", Token::Month(5)),
    ("jun", Token::Month(6)),
    ("jul", Token::Month(7)),
    ("ago", Token::Month(8)),
    ("set", Token::Month(9)),
    ("out", Token::Month(10)),
    ("nov", Token::Month(11)),
    ("dez", Token::Month(12)),
    // Temporal units (singular + plural)
    ("ano", Token::Unit(TemporalUnit::Year)),
    ("anos", Token::Unit(TemporalUnit::Year)),
    ("mes", Token::Unit(TemporalUnit::Month)),
    ("meses", Token::Unit(TemporalUnit::Month)),
    ("semana", Token::Unit(TemporalUnit::Week)),
    ("semanas", Token::Unit(TemporalUnit::Week)),
    ("dia", Token::Unit(TemporalUnit::Day)),
    ("dias", Token::Unit(TemporalUnit::Day)),
    ("hora", Token::Unit(TemporalUnit::Hour)),
    ("horas", Token::Unit(TemporalUnit::Hour)),
    ("minuto", Token::Unit(TemporalUnit::Minute)),
    ("minutos", Token::Unit(TemporalUnit::Minute)),
    ("segundo", Token::Unit(TemporalUnit::Second)),
    ("segundos", Token::Unit(TemporalUnit::Second)),
    // Abbreviated duration units (PT)
    ("h", Token::Unit(TemporalUnit::Hour)),
    ("hr", Token::Unit(TemporalUnit::Hour)),
    ("hrs", Token::Unit(TemporalUnit::Hour)),
    ("d", Token::Unit(TemporalUnit::Day)),
    ("sem", Token::Unit(TemporalUnit::Week)),   // abbreviation for "semana"
    ("sems", Token::Unit(TemporalUnit::Week)),  // plural abbreviation
    ("a", Token::Unit(TemporalUnit::Year)),     // abbreviation for "ano"
];

/// Multi-word patterns for Portuguese. Longer patterns first so they are
/// matched before shorter prefixes ("depois de amanha" before "depois de").
static PT_MULTI_WORD_PATTERNS: [(&[&str], Token); 5] = [
    // 3-word patterns first (longest match)
    (&["depois", "de", "amanha"], Token::Overmorrow), // "depois de amanha" = overmorrow
    (&["antes", "de", "ontem"], Token::Ereyesterday), // "antes de ontem" = ereyesterday
    // 2-word patterns
    (&["daqui", "a"], Token::In),      // "daqui a 3 dias" = "in 3 days"
    (&["depois", "de"], Token::After), // "depois de" = "after"
    (&["antes", "de"], Token::Before), // "antes de" = "before"
];

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::locale::LocaleKeywords;

    fn pt_kw() -> LocaleKeywords {
        LocaleKeywords::from_locale(&PT_LOCALE)
    }

    // --- Basic keyword lookup ---

    #[test]
    fn pt_hoje_maps_to_today() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("hoje", "hoje"), Token::Today);
    }

    #[test]
    fn pt_amanha_maps_to_tomorrow() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("amanha", "amanha"), Token::Tomorrow);
    }

    #[test]
    fn pt_ontem_maps_to_yesterday() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("ontem", "ontem"), Token::Yesterday);
    }

    #[test]
    fn pt_anteontem_maps_to_ereyesterday() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("anteontem", "anteontem"), Token::Ereyesterday);
    }

    // --- Gender variants ---

    #[test]
    fn pt_gender_variants_next() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("proxima", "proxima"), Token::Next);
        assert_eq!(kw.lookup("proximo", "proximo"), Token::Next);
        assert_eq!(kw.lookup("proximas", "proximas"), Token::Next);
        assert_eq!(kw.lookup("proximos", "proximos"), Token::Next);
    }

    #[test]
    fn pt_gender_variants_last() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("ultima", "ultima"), Token::Last);
        assert_eq!(kw.lookup("ultimo", "ultimo"), Token::Last);
        assert_eq!(kw.lookup("ultimas", "ultimas"), Token::Last);
        assert_eq!(kw.lookup("ultimos", "ultimos"), Token::Last);
        assert_eq!(kw.lookup("passada", "passada"), Token::Last);
        assert_eq!(kw.lookup("passado", "passado"), Token::Last);
    }

    #[test]
    fn pt_gender_variants_this() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("esta", "esta"), Token::This);
        assert_eq!(kw.lookup("este", "este"), Token::This);
        assert_eq!(kw.lookup("estas", "estas"), Token::This);
        assert_eq!(kw.lookup("estes", "estes"), Token::This);
    }

    // --- Weekday mappings ---

    #[test]
    fn pt_weekday_full_names() {
        let kw = pt_kw();
        assert_eq!(
            kw.lookup("segunda", "segunda"),
            Token::Weekday(jiff::civil::Weekday::Monday)
        );
        assert_eq!(
            kw.lookup("terca", "terca"),
            Token::Weekday(jiff::civil::Weekday::Tuesday)
        );
        assert_eq!(
            kw.lookup("quarta", "quarta"),
            Token::Weekday(jiff::civil::Weekday::Wednesday)
        );
        assert_eq!(
            kw.lookup("quinta", "quinta"),
            Token::Weekday(jiff::civil::Weekday::Thursday)
        );
        assert_eq!(
            kw.lookup("sexta", "sexta"),
            Token::Weekday(jiff::civil::Weekday::Friday)
        );
        assert_eq!(
            kw.lookup("sabado", "sabado"),
            Token::Weekday(jiff::civil::Weekday::Saturday)
        );
        assert_eq!(
            kw.lookup("domingo", "domingo"),
            Token::Weekday(jiff::civil::Weekday::Sunday)
        );
    }

    #[test]
    fn pt_weekday_abbreviations() {
        let kw = pt_kw();
        assert_eq!(
            kw.lookup("seg", "seg"),
            Token::Weekday(jiff::civil::Weekday::Monday)
        );
        assert_eq!(
            kw.lookup("ter", "ter"),
            Token::Weekday(jiff::civil::Weekday::Tuesday)
        );
        assert_eq!(
            kw.lookup("qua", "qua"),
            Token::Weekday(jiff::civil::Weekday::Wednesday)
        );
        assert_eq!(
            kw.lookup("qui", "qui"),
            Token::Weekday(jiff::civil::Weekday::Thursday)
        );
        assert_eq!(
            kw.lookup("sex", "sex"),
            Token::Weekday(jiff::civil::Weekday::Friday)
        );
        assert_eq!(
            kw.lookup("sab", "sab"),
            Token::Weekday(jiff::civil::Weekday::Saturday)
        );
        assert_eq!(
            kw.lookup("dom", "dom"),
            Token::Weekday(jiff::civil::Weekday::Sunday)
        );
    }

    // --- Month mappings ---

    #[test]
    fn pt_month_full_names() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("janeiro", "janeiro"), Token::Month(1));
        assert_eq!(kw.lookup("fevereiro", "fevereiro"), Token::Month(2));
        assert_eq!(kw.lookup("marco", "marco"), Token::Month(3));
        assert_eq!(kw.lookup("abril", "abril"), Token::Month(4));
        assert_eq!(kw.lookup("maio", "maio"), Token::Month(5));
        assert_eq!(kw.lookup("junho", "junho"), Token::Month(6));
        assert_eq!(kw.lookup("julho", "julho"), Token::Month(7));
        assert_eq!(kw.lookup("agosto", "agosto"), Token::Month(8));
        assert_eq!(kw.lookup("setembro", "setembro"), Token::Month(9));
        assert_eq!(kw.lookup("outubro", "outubro"), Token::Month(10));
        assert_eq!(kw.lookup("novembro", "novembro"), Token::Month(11));
        assert_eq!(kw.lookup("dezembro", "dezembro"), Token::Month(12));
    }

    #[test]
    fn pt_ago_maps_to_month_8_not_ago() {
        // In PT, "ago" is the abbreviation for August, not the "ago" direction keyword.
        // PT uses "ha"/"atras" for the ago direction.
        let kw = pt_kw();
        assert_eq!(kw.lookup("ago", "ago"), Token::Month(8));
    }

    #[test]
    fn pt_month_abbreviations() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("jan", "jan"), Token::Month(1));
        assert_eq!(kw.lookup("fev", "fev"), Token::Month(2));
        assert_eq!(kw.lookup("mar", "mar"), Token::Month(3));
        assert_eq!(kw.lookup("abr", "abr"), Token::Month(4));
        assert_eq!(kw.lookup("mai", "mai"), Token::Month(5));
        assert_eq!(kw.lookup("jun", "jun"), Token::Month(6));
        assert_eq!(kw.lookup("jul", "jul"), Token::Month(7));
        assert_eq!(kw.lookup("set", "set"), Token::Month(9));
        assert_eq!(kw.lookup("out", "out"), Token::Month(10));
        assert_eq!(kw.lookup("nov", "nov"), Token::Month(11));
        assert_eq!(kw.lookup("dez", "dez"), Token::Month(12));
    }

    // --- Temporal units ---

    #[test]
    fn pt_temporal_units() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("ano", "ano"), Token::Unit(TemporalUnit::Year));
        assert_eq!(kw.lookup("anos", "anos"), Token::Unit(TemporalUnit::Year));
        assert_eq!(kw.lookup("mes", "mes"), Token::Unit(TemporalUnit::Month));
        assert_eq!(
            kw.lookup("meses", "meses"),
            Token::Unit(TemporalUnit::Month)
        );
        assert_eq!(
            kw.lookup("semana", "semana"),
            Token::Unit(TemporalUnit::Week)
        );
        assert_eq!(
            kw.lookup("semanas", "semanas"),
            Token::Unit(TemporalUnit::Week)
        );
        assert_eq!(kw.lookup("dia", "dia"), Token::Unit(TemporalUnit::Day));
        assert_eq!(kw.lookup("dias", "dias"), Token::Unit(TemporalUnit::Day));
        assert_eq!(kw.lookup("hora", "hora"), Token::Unit(TemporalUnit::Hour));
        assert_eq!(kw.lookup("horas", "horas"), Token::Unit(TemporalUnit::Hour));
        assert_eq!(
            kw.lookup("minuto", "minuto"),
            Token::Unit(TemporalUnit::Minute)
        );
        assert_eq!(
            kw.lookup("minutos", "minutos"),
            Token::Unit(TemporalUnit::Minute)
        );
        assert_eq!(
            kw.lookup("segundo", "segundo"),
            Token::Unit(TemporalUnit::Second)
        );
        assert_eq!(
            kw.lookup("segundos", "segundos"),
            Token::Unit(TemporalUnit::Second)
        );
    }

    // --- Articles ---

    #[test]
    fn pt_articles_um_uma() {
        let kw = pt_kw();
        assert_eq!(kw.lookup("um", "um"), Token::A);
        assert_eq!(kw.lookup("uma", "uma"), Token::A);
    }

    #[test]
    fn pt_articles_list() {
        assert_eq!(PT_LOCALE.articles(), &["um", "uma"]);
    }

    // --- Locale metadata ---

    #[test]
    fn pt_locale_name_and_code() {
        assert_eq!(PT_LOCALE.name(), "Portugues");
        assert_eq!(PT_LOCALE.code(), "pt");
    }

    #[test]
    fn pt_keyword_count() {
        // 82 original keywords + 7 abbreviated duration units
        assert_eq!(PT_KEYWORDS.len(), 89);
    }

    #[test]
    fn pt_multi_word_pattern_count() {
        assert_eq!(PT_MULTI_WORD_PATTERNS.len(), 5);
    }

    #[test]
    fn pt_abbreviated_units() {
        let kw = pt_kw();
        // Hour abbreviations (universal)
        assert_eq!(kw.lookup("h", "h"), Token::Unit(TemporalUnit::Hour));
        assert_eq!(kw.lookup("hr", "hr"), Token::Unit(TemporalUnit::Hour));
        assert_eq!(kw.lookup("hrs", "hrs"), Token::Unit(TemporalUnit::Hour));
        // Day abbreviation (universal)
        assert_eq!(kw.lookup("d", "d"), Token::Unit(TemporalUnit::Day));
        // Week abbreviations (PT-specific: "sem" for semana)
        assert_eq!(kw.lookup("sem", "sem"), Token::Unit(TemporalUnit::Week));
        assert_eq!(kw.lookup("sems", "sems"), Token::Unit(TemporalUnit::Week));
        // Year abbreviation (PT-specific: "a" for ano)
        assert_eq!(kw.lookup("a", "a"), Token::Unit(TemporalUnit::Year));
    }
}
