use chrono::TimeZone;
use chrono_tz::UTC;
use criterion::{Criterion, criterion_group, criterion_main};
use tardis_cli::{config::Config, core, core::App, core::Preset};

/// Fixed "now" for deterministic benchmarks: 2025-06-15T12:00:00 UTC
fn fixed_now() -> chrono::DateTime<chrono_tz::Tz> {
    UTC.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap()
}

/// Create an App with a standard format and fixed "now".
fn make_app(expr: &str) -> App {
    App::new(
        expr.into(),
        "%Y-%m-%dT%H:%M:%S".into(),
        UTC,
        Some(fixed_now()),
    )
}

// ---------------------------------------------------------------------------
// Group 1: Relative date expressions
// ---------------------------------------------------------------------------

fn bench_relative(c: &mut Criterion) {
    let app_today = make_app("today");
    c.bench_function("relative_today", |b| {
        b.iter(|| core::process(&app_today, &[]).unwrap());
    });

    let app_tomorrow = make_app("tomorrow");
    c.bench_function("relative_tomorrow", |b| {
        b.iter(|| core::process(&app_tomorrow, &[]).unwrap());
    });

    let app_yesterday = make_app("yesterday");
    c.bench_function("relative_yesterday", |b| {
        b.iter(|| core::process(&app_yesterday, &[]).unwrap());
    });

    let app_now = make_app("now");
    c.bench_function("relative_now", |b| {
        b.iter(|| core::process(&app_now, &[]).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 2: Day reference expressions
// ---------------------------------------------------------------------------

fn bench_dayref(c: &mut Criterion) {
    let app_next_friday = make_app("next friday");
    c.bench_function("dayref_next_friday", |b| {
        b.iter(|| core::process(&app_next_friday, &[]).unwrap());
    });

    let app_last_monday = make_app("last monday");
    c.bench_function("dayref_last_monday", |b| {
        b.iter(|| core::process(&app_last_monday, &[]).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 3: Time suffix expressions
// ---------------------------------------------------------------------------

fn bench_time_suffix(c: &mut Criterion) {
    let app_tomorrow_10am = make_app("tomorrow 10 am");
    c.bench_function("time_tomorrow_10am", |b| {
        b.iter(|| core::process(&app_tomorrow_10am, &[]).unwrap());
    });

    let app_today_15 = make_app("today 15:00");
    c.bench_function("time_today_1500", |b| {
        b.iter(|| core::process(&app_today_15, &[]).unwrap());
    });

    let app_yesterday_1800 = make_app("yesterday 18:00");
    c.bench_function("time_yesterday_1800", |b| {
        b.iter(|| core::process(&app_yesterday_1800, &[]).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 4: Duration expressions
// ---------------------------------------------------------------------------

fn bench_duration(c: &mut Criterion) {
    let app_3_days = make_app("in 3 days");
    c.bench_function("duration_in_3_days", |b| {
        b.iter(|| core::process(&app_3_days, &[]).unwrap());
    });

    let app_2_hours = make_app("in 2 hours");
    c.bench_function("duration_in_2_hours", |b| {
        b.iter(|| core::process(&app_2_hours, &[]).unwrap());
    });

    let app_1_week = make_app("in 1 week");
    c.bench_function("duration_in_1_week", |b| {
        b.iter(|| core::process(&app_1_week, &[]).unwrap());
    });

    let app_2_weeks_ago = make_app("2 weeks ago");
    c.bench_function("duration_2_weeks_ago", |b| {
        b.iter(|| core::process(&app_2_weeks_ago, &[]).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 5: Absolute date/time expressions
// ---------------------------------------------------------------------------

fn bench_absolute(c: &mut Criterion) {
    let app_iso_date = make_app("2025-01-01");
    c.bench_function("absolute_iso_date", |b| {
        b.iter(|| core::process(&app_iso_date, &[]).unwrap());
    });

    let app_iso_datetime = make_app("2025-01-01 12:30:45");
    c.bench_function("absolute_iso_datetime", |b| {
        b.iter(|| core::process(&app_iso_datetime, &[]).unwrap());
    });

    let app_iso_full = make_app("2025-06-24 10:00");
    c.bench_function("absolute_iso_full", |b| {
        b.iter(|| core::process(&app_iso_full, &[]).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 6: Epoch input expressions
// ---------------------------------------------------------------------------

fn bench_epoch(c: &mut Criterion) {
    let app_epoch_simple = make_app("@1735689600");
    c.bench_function("epoch_seconds", |b| {
        b.iter(|| core::process(&app_epoch_simple, &[]).unwrap());
    });

    let app_epoch_zero = make_app("@0");
    c.bench_function("epoch_zero", |b| {
        b.iter(|| core::process(&app_epoch_zero, &[]).unwrap());
    });

    let app_epoch_negative = make_app("@-86400");
    c.bench_function("epoch_negative", |b| {
        b.iter(|| core::process(&app_epoch_negative, &[]).unwrap());
    });

    let app_epoch_large = make_app("@1893456000");
    c.bench_function("epoch_large_timestamp", |b| {
        b.iter(|| core::process(&app_epoch_large, &[]).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 7: Format rendering variants
// ---------------------------------------------------------------------------

fn bench_format(c: &mut Criterion) {
    let app_complex_fmt = App::new(
        "2025-01-01 12:30:45".into(),
        "%A, %B %e, %Y %H:%M:%S %Z".into(),
        UTC,
        Some(fixed_now()),
    );
    c.bench_function("format_complex_strftime", |b| {
        b.iter(|| core::process(&app_complex_fmt, &[]).unwrap());
    });

    let app_epoch_fmt = App::new(
        "2025-01-01".into(),
        "epoch".into(),
        UTC,
        Some(fixed_now()),
    );
    c.bench_function("format_epoch_output", |b| {
        b.iter(|| core::process(&app_epoch_fmt, &[]).unwrap());
    });

    let app_unix_fmt = App::new(
        "2025-01-01".into(),
        "unix".into(),
        UTC,
        Some(fixed_now()),
    );
    c.bench_function("format_unix_output", |b| {
        b.iter(|| core::process(&app_unix_fmt, &[]).unwrap());
    });

    let app_compact = App::new(
        "today".into(),
        "%Y%m%d".into(),
        UTC,
        Some(fixed_now()),
    );
    c.bench_function("format_compact_date", |b| {
        b.iter(|| core::process(&app_compact, &[]).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 8: Preset resolution
// ---------------------------------------------------------------------------

fn bench_preset(c: &mut Criterion) {
    let presets = vec![
        Preset::new("iso".into(), "%Y-%m-%dT%H:%M:%S".into()),
        Preset::new("br".into(), "%d/%m/%Y".into()),
        Preset::new("time".into(), "%H:%M".into()),
        Preset::new("short".into(), "%Y-%m-%d".into()),
    ];

    let app_preset = App::new(
        "2025-01-01 10:00".into(),
        "iso".into(),
        UTC,
        Some(fixed_now()),
    );
    c.bench_function("preset_iso_resolve", |b| {
        b.iter(|| core::process(&app_preset, &presets).unwrap());
    });

    let app_preset_br = App::new(
        "2025-01-01".into(),
        "br".into(),
        UTC,
        Some(fixed_now()),
    );
    c.bench_function("preset_br_resolve", |b| {
        b.iter(|| core::process(&app_preset_br, &presets).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 9: Timezone conversion
// ---------------------------------------------------------------------------

fn bench_timezone(c: &mut Criterion) {
    let app_sao_paulo = App::new(
        "2025-01-01 12:00".into(),
        "%Y-%m-%dT%H:%M:%S %Z".into(),
        chrono_tz::America::Sao_Paulo,
        Some(chrono_tz::America::Sao_Paulo
            .with_ymd_and_hms(2025, 6, 15, 12, 0, 0)
            .unwrap()),
    );
    c.bench_function("timezone_sao_paulo", |b| {
        b.iter(|| core::process(&app_sao_paulo, &[]).unwrap());
    });

    let app_tokyo = App::new(
        "2025-01-01 12:00".into(),
        "%Y-%m-%dT%H:%M:%S %Z".into(),
        chrono_tz::Asia::Tokyo,
        Some(chrono_tz::Asia::Tokyo
            .with_ymd_and_hms(2025, 6, 15, 12, 0, 0)
            .unwrap()),
    );
    c.bench_function("timezone_tokyo", |b| {
        b.iter(|| core::process(&app_tokyo, &[]).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 10: Config loading
// ---------------------------------------------------------------------------

fn bench_config(c: &mut Criterion) {
    c.bench_function("config_load", |b| {
        b.iter(|| Config::load().unwrap());
    });
}

// ---------------------------------------------------------------------------
// Group 11: Error paths (measure cost of invalid input)
// ---------------------------------------------------------------------------

fn bench_error(c: &mut Criterion) {
    let app_bad = make_app("???");
    c.bench_function("error_invalid_expression", |b| {
        b.iter(|| core::process(&app_bad, &[]).unwrap_err());
    });

    let app_bad_epoch = make_app("@notanumber");
    c.bench_function("error_invalid_epoch", |b| {
        b.iter(|| core::process(&app_bad_epoch, &[]).unwrap_err());
    });
}

// ---------------------------------------------------------------------------
// Criterion registration
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_relative,
    bench_dayref,
    bench_time_suffix,
    bench_duration,
    bench_absolute,
    bench_epoch,
    bench_format,
    bench_preset,
    bench_timezone,
    bench_config,
    bench_error
);
criterion_main!(benches);
