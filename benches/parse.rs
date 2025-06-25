use chrono_tz::UTC;
use criterion::{Criterion, criterion_group, criterion_main};
use tardis::{core, core::App};

fn bench_parse(c: &mut Criterion) {
    let app = App::new("in 3 days".into(), "%Y-%m-%d".into(), UTC, None);
    c.bench_function("in_3_days", |b| {
        b.iter(|| core::process(&app, &[]).unwrap());
    });
}
criterion_group!(benches, bench_parse);
criterion_main!(benches);
