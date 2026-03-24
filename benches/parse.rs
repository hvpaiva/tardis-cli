use criterion::{Criterion, criterion_group, criterion_main};
use jiff::{civil, tz::TimeZone};
use tardis_cli::{core, core::App};

fn fixed_now() -> jiff::Zoned {
    let tz = TimeZone::get("UTC").unwrap();
    let dt = civil::date(2025, 6, 15).at(12, 0, 0, 0);
    tz.to_ambiguous_zoned(dt).compatible().unwrap()
}

fn make_app(expr: &str) -> App {
    App::new(
        expr.into(),
        "%Y-%m-%dT%H:%M:%S".into(),
        TimeZone::get("UTC").unwrap(),
        Some(fixed_now()),
    )
}

fn bench_relative(c: &mut Criterion) {
    let app_now = make_app("now");
    c.bench_function("relative_now", |b| {
        b.iter(|| core::process(&app_now, &[]).unwrap());
    });

    let app_today = make_app("today");
    c.bench_function("relative_today", |b| {
        b.iter(|| core::process(&app_today, &[]).unwrap());
    });

    let app_tomorrow = make_app("tomorrow");
    c.bench_function("relative_tomorrow", |b| {
        b.iter(|| core::process(&app_tomorrow, &[]).unwrap());
    });
}

fn bench_dayref(c: &mut Criterion) {
    let app_next_friday = make_app("next friday");
    c.bench_function("dayref_next_friday", |b| {
        b.iter(|| core::process(&app_next_friday, &[]).unwrap());
    });

    let app_last_monday = make_app("last monday");
    c.bench_function("dayref_last_monday", |b| {
        b.iter(|| core::process(&app_last_monday, &[]).unwrap());
    });

    let app_this_wednesday = make_app("this wednesday");
    c.bench_function("dayref_this_wednesday", |b| {
        b.iter(|| core::process(&app_this_wednesday, &[]).unwrap());
    });
}

fn bench_time_suffix(c: &mut Criterion) {
    let app_today_1830 = make_app("today 18:30");
    c.bench_function("time_today_1830", |b| {
        b.iter(|| core::process(&app_today_1830, &[]).unwrap());
    });
}

fn bench_duration(c: &mut Criterion) {
    let app_past = make_app("3 hours ago");
    c.bench_function("duration_past", |b| {
        b.iter(|| core::process(&app_past, &[]).unwrap());
    });

    let app_future = make_app("in 3 days");
    c.bench_function("duration_future", |b| {
        b.iter(|| core::process(&app_future, &[]).unwrap());
    });

    let app_article = make_app("a week ago");
    c.bench_function("duration_article", |b| {
        b.iter(|| core::process(&app_article, &[]).unwrap());
    });
}

fn bench_absolute(c: &mut Criterion) {
    let app_iso = make_app("2025-01-01");
    c.bench_function("absolute_iso", |b| {
        b.iter(|| core::process(&app_iso, &[]).unwrap());
    });

    let app_datetime = make_app("2022-11-07 13:25:30");
    c.bench_function("absolute_datetime", |b| {
        b.iter(|| core::process(&app_datetime, &[]).unwrap());
    });
}

fn bench_epoch(c: &mut Criterion) {
    let app_standard = make_app("@1735689600");
    c.bench_function("epoch_standard", |b| {
        b.iter(|| core::process(&app_standard, &[]).unwrap());
    });

    let app_negative = make_app("@-86400");
    c.bench_function("epoch_negative", |b| {
        b.iter(|| core::process(&app_negative, &[]).unwrap());
    });
}

fn bench_error(c: &mut Criterion) {
    let app_gibberish = make_app("???");
    c.bench_function("error_gibberish", |b| {
        b.iter(|| core::process(&app_gibberish, &[]).is_err());
    });
}

criterion_group!(
    benches,
    bench_relative,
    bench_dayref,
    bench_time_suffix,
    bench_duration,
    bench_absolute,
    bench_epoch,
    bench_error
);
criterion_main!(benches);
