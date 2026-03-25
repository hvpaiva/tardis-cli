//! Integration tests for the TARDIS locale system.
//!
//! These tests exercise the full parse pipeline (input -> tokens -> AST -> Zoned)
//! for both English (EN) and Portuguese (PT) locales, validating TEST-04.
//!
//! Reference anchor: `2025-06-24T12:00:00Z` (a Tuesday, UTC noon).

use assert_cmd::Command;
use assert_fs::TempDir;
use jiff::{tz::TimeZone, Timestamp, Zoned};
use predicates::prelude::*;
use tardis_cli::{locale, parser};

// ============================================================
// Helpers
// ============================================================

fn utc() -> TimeZone {
    TimeZone::get("UTC").unwrap()
}

fn fixed_now() -> Zoned {
    "2025-06-24T12:00:00Z"
        .parse::<Timestamp>()
        .unwrap()
        .to_zoned(utc())
}

fn parse_en(input: &str) -> Zoned {
    let now = fixed_now();
    let loc = locale::get_locale("en");
    let kw = locale::LocaleKeywords::from_locale(loc);
    parser::parse(input, &now, &kw).unwrap()
}

fn parse_pt(input: &str) -> Zoned {
    let now = fixed_now();
    let loc = locale::get_locale("pt");
    let kw = locale::LocaleKeywords::from_locale(loc);
    parser::parse(input, &now, &kw).unwrap()
}

fn parse_pt_err(input: &str) -> String {
    let now = fixed_now();
    let loc = locale::get_locale("pt");
    let kw = locale::LocaleKeywords::from_locale(loc);
    parser::parse(input, &now, &kw)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "no error".to_string())
}

fn parse_en_err(input: &str) -> String {
    let now = fixed_now();
    let loc = locale::get_locale("en");
    let kw = locale::LocaleKeywords::from_locale(loc);
    parser::parse(input, &now, &kw)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "no error".to_string())
}

/// Extract just the date portion "YYYY-MM-DD" from a Zoned datetime.
fn date_str(z: &Zoned) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        z.date().year(),
        z.date().month() as u8,
        z.date().day()
    )
}

/// Extract "YYYY-MM-DD HH:MM" from a Zoned datetime.
fn datetime_str(z: &Zoned) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        z.date().year(),
        z.date().month() as u8,
        z.date().day(),
        z.time().hour(),
        z.time().minute()
    )
}

// ============================================================
// EN locale tests (TEST-04 regression coverage)
// ============================================================

#[test]
fn en_now() {
    let result = parse_en("now");
    assert_eq!(datetime_str(&result), "2025-06-24 12:00");
}

#[test]
fn en_today() {
    let result = parse_en("today");
    assert_eq!(date_str(&result), "2025-06-24");
}

#[test]
fn en_tomorrow() {
    let result = parse_en("tomorrow");
    assert_eq!(date_str(&result), "2025-06-25");
}

#[test]
fn en_yesterday() {
    let result = parse_en("yesterday");
    assert_eq!(date_str(&result), "2025-06-23");
}

#[test]
fn en_overmorrow() {
    let result = parse_en("overmorrow");
    assert_eq!(date_str(&result), "2025-06-26");
}

#[test]
fn en_next_friday() {
    // 2025-06-24 is Tuesday, next Friday = 2025-06-27
    let result = parse_en("next friday");
    assert_eq!(date_str(&result), "2025-06-27");
}

#[test]
fn en_3_hours_ago() {
    let result = parse_en("3 hours ago");
    assert_eq!(datetime_str(&result), "2025-06-24 09:00");
}

#[test]
fn en_in_2_days() {
    let result = parse_en("in 2 days");
    assert_eq!(date_str(&result), "2025-06-26");
}

#[test]
fn en_epoch() {
    let result = parse_en("@1735689600");
    assert_eq!(date_str(&result), "2025-01-01");
}

// ============================================================
// PT locale tests (TEST-04, D-03)
// ============================================================

// --- Relative dates ---

#[test]
fn pt_hoje() {
    let result = parse_pt("hoje");
    assert_eq!(date_str(&result), "2025-06-24");
}

#[test]
fn pt_amanha_no_accent() {
    let result = parse_pt("amanha");
    assert_eq!(date_str(&result), "2025-06-25");
}

#[test]
fn pt_amanha_with_accent() {
    // "amanha" with tilde on the second 'a' -> accent-insensitive (D-04)
    let result = parse_pt("amanh\u{00e3}");
    assert_eq!(date_str(&result), "2025-06-25");
}

#[test]
fn pt_ontem() {
    let result = parse_pt("ontem");
    assert_eq!(date_str(&result), "2025-06-23");
}

#[test]
fn pt_anteontem() {
    // Day before yesterday = 2025-06-22
    let result = parse_pt("anteontem");
    assert_eq!(date_str(&result), "2025-06-22");
}

#[test]
fn pt_depois_de_amanha() {
    // Overmorrow = 2025-06-26
    let result = parse_pt("depois de amanha");
    assert_eq!(date_str(&result), "2025-06-26");
}

#[test]
fn pt_antes_de_ontem() {
    // Alternative form for ereyesterday = 2025-06-22
    let result = parse_pt("antes de ontem");
    assert_eq!(date_str(&result), "2025-06-22");
}

// --- Duration expressions ---

#[test]
fn pt_daqui_a_3_dias() {
    // "daqui a 3 dias" = "in 3 days" = 2025-06-27
    let result = parse_pt("daqui a 3 dias");
    assert_eq!(date_str(&result), "2025-06-27");
}

#[test]
fn pt_ha_2_horas() {
    // "ha 2 horas" = "2 hours ago" = 2025-06-24 10:00
    let result = parse_pt("ha 2 horas");
    assert_eq!(datetime_str(&result), "2025-06-24 10:00");
}

#[test]
fn pt_em_5_minutos() {
    // "em 5 minutos" = "in 5 minutes" = 2025-06-24 12:05
    let result = parse_pt("em 5 minutos");
    assert_eq!(datetime_str(&result), "2025-06-24 12:05");
}

#[test]
fn pt_3_dias_atras() {
    // "3 dias atras" = "3 days ago" = 2025-06-21
    let result = parse_pt("3 dias atras");
    assert_eq!(date_str(&result), "2025-06-21");
}

// --- Day references ---

#[test]
fn pt_proxima_sexta() {
    // "proxima sexta" = "next friday"
    // 2025-06-24 is Tuesday, next Friday = 2025-06-27
    let result = parse_pt("proxima sexta");
    assert_eq!(date_str(&result), "2025-06-27");
}

#[test]
fn pt_ultima_segunda() {
    // "ultima segunda" = "last monday"
    // 2025-06-24 is Tuesday, last Monday = 2025-06-23
    let result = parse_pt("ultima segunda");
    assert_eq!(date_str(&result), "2025-06-23");
}

#[test]
fn pt_esta_quarta() {
    // "esta quarta" = "this wednesday"
    // 2025-06-24 is Tuesday, this Wednesday = 2025-06-25
    let result = parse_pt("esta quarta");
    assert_eq!(date_str(&result), "2025-06-25");
}

#[test]
fn pt_proximo_sabado() {
    // "proximo sabado" = "next saturday" (masculine gender variant)
    // 2025-06-24 is Tuesday, next Saturday = 2025-06-28
    let result = parse_pt("proximo sabado");
    assert_eq!(date_str(&result), "2025-06-28");
}

// --- Verbal arithmetic ---

#[test]
fn pt_amanha_mais_3_horas() {
    // "amanha mais 3 horas" = "tomorrow + 3 hours" = 2025-06-25 03:00
    let result = parse_pt("amanha mais 3 horas");
    assert_eq!(datetime_str(&result), "2025-06-25 03:00");
}

// --- Accent-insensitive tests (D-04) ---

#[test]
fn pt_accent_proxima() {
    // "proxima" with accent on 'o' -> same as "proxima"
    let result = parse_pt("pr\u{00f3}xima sexta");
    assert_eq!(date_str(&result), "2025-06-27");
}

#[test]
fn pt_accent_sabado() {
    // "sabado" with accent on first 'a'
    let result = parse_pt("s\u{00e1}bado");
    // Bare weekday resolves as "this saturday" -> next occurrence
    // 2025-06-24 is Tuesday, this Saturday = 2025-06-28
    assert_eq!(date_str(&result), "2025-06-28");
}

#[test]
fn pt_accent_marco() {
    // "marco" with cedilla on 'c' -- need an absolute date context to test month
    // We test that the lexer correctly handles the accent, which was already
    // verified in unit tests. Here test via a date expression.
    let result = parse_pt("24 mar\u{00e7}o 2025");
    assert_eq!(date_str(&result), "2025-03-24");
}

// ============================================================
// CLI integration tests (using assert_cmd)
// ============================================================

fn td_locale_cmd(locale: &str) -> Command {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("td").unwrap();
    cmd.env("XDG_CONFIG_HOME", tmp.path());
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env_remove("LC_TIME");
    cmd.args(["--locale", locale]);
    cmd
}

#[test]
fn cli_pt_amanha() {
    td_locale_cmd("pt")
        .args(["--now", "2025-06-24T12:00:00Z", "-f", "%Y-%m-%d", "--timezone", "UTC"])
        .arg("amanha")
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-25"));
}

#[test]
fn cli_pt_daqui_a_3_dias() {
    td_locale_cmd("pt")
        .args(["--now", "2025-06-24T12:00:00Z", "-f", "%Y-%m-%d", "--timezone", "UTC"])
        .arg("daqui a 3 dias")
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-27"));
}

#[test]
fn cli_en_tomorrow() {
    td_locale_cmd("en")
        .args(["--now", "2025-06-24T12:00:00Z", "-f", "%Y-%m-%d", "--timezone", "UTC"])
        .arg("tomorrow")
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-25"));
}

#[test]
fn cli_pt_ha_2_horas() {
    td_locale_cmd("pt")
        .args(["--now", "2025-06-24T12:00:00Z", "-f", "%Y-%m-%d %H:%M", "--timezone", "UTC"])
        .arg("ha 2 horas")
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-24 10:00"));
}

#[test]
fn cli_invalid_locale_falls_back_to_en() {
    // Unknown locale falls back to English
    td_locale_cmd("invalid")
        .args(["--now", "2025-06-24T12:00:00Z", "-f", "%Y-%m-%d", "--timezone", "UTC"])
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-24"));
}

// ============================================================
// Cross-locale isolation tests
// ============================================================

#[test]
fn en_does_not_recognize_amanha() {
    // "amanha" is not an English keyword -> parse error
    let err = parse_en_err("amanha");
    assert!(
        err.contains("could not parse"),
        "Expected parse error for 'amanha' in EN, got: {err}"
    );
}

#[test]
fn pt_does_not_recognize_tomorrow() {
    // "tomorrow" is not a Portuguese keyword -> parse error
    let err = parse_pt_err("tomorrow");
    assert!(
        err.contains("could not parse"),
        "Expected parse error for 'tomorrow' in PT, got: {err}"
    );
}

// ============================================================
// Extensibility test (LOCL-04)
// ============================================================

#[test]
fn extensibility_new_locale_only_needs_data_module() {
    // Verify LOCL-04: adding a locale requires only implementing the Locale trait.
    // We can test this by verifying that get_locale returns a proper locale
    // for "pt" and that LocaleKeywords::from_locale works with it.
    let pt = locale::get_locale("pt");
    assert_eq!(pt.code(), "pt");
    assert_eq!(pt.name(), "Portugues");

    let kw = locale::LocaleKeywords::from_locale(pt);
    let now = fixed_now();

    // PT locale keywords work through the parse pipeline
    let result = parser::parse("hoje", &now, &kw);
    assert!(result.is_ok(), "PT locale should parse 'hoje'");
    assert_eq!(date_str(&result.unwrap()), "2025-06-24");

    // EN keywords don't work with PT locale
    let result = parser::parse("today", &now, &kw);
    assert!(result.is_err(), "PT locale should not parse 'today'");

    // Epoch is locale-independent
    let result = parser::parse("@1735689600", &now, &kw);
    assert!(result.is_ok(), "Epoch should work with any locale");
}

// ============================================================
// PT: additional coverage
// ============================================================

#[test]
fn pt_ha_with_accent() {
    // "ha" with accent on 'a' -> same as "ha"
    let result = parse_pt("h\u{00e1} 2 horas");
    assert_eq!(datetime_str(&result), "2025-06-24 10:00");
}

#[test]
fn pt_depois_de_amanha_with_accent() {
    // "depois de amanha" with tilde on second a of amanha
    let result = parse_pt("depois de amanh\u{00e3}");
    assert_eq!(date_str(&result), "2025-06-26");
}

#[test]
fn pt_uma_semana_atras() {
    // "uma semana atras" = "a week ago" = 2025-06-17
    let result = parse_pt("uma semana atras");
    assert_eq!(date_str(&result), "2025-06-17");
}

#[test]
fn pt_agora() {
    // "agora" = "now"
    let result = parse_pt("agora");
    assert_eq!(datetime_str(&result), "2025-06-24 12:00");
}

#[test]
fn pt_epoch_works_with_pt_locale() {
    // Epoch parsing is locale-independent
    let result = parse_pt("@1735689600");
    assert_eq!(date_str(&result), "2025-01-01");
}
