//! Criterion benchmarks for every report over one year of reference data (RPT-11).
//! Run with `cargo bench -p reports`. The pass/fail budgets are checked by
//! `tests/benchmark.rs`; this gives detailed timings to compare between changes.

#![allow(clippy::unwrap_used)]

#[path = "../tests/benchmark.rs"]
#[allow(dead_code)]
mod budgets;

use criterion::{Criterion, criterion_group, criterion_main};
use pos_core::BusinessDate;
use reports::{ShopContext, daily_summary};

fn every_report(criterion: &mut Criterion) {
    let (database, _folder) = budgets::reference_db();
    database
        .read(|conn| {
            let mut group = criterion.benchmark_group("reports");
            group.sample_size(10);
            group.bench_function("daily summary", |bencher| {
                let day = BusinessDate::parse("2025-07-01").unwrap();
                bencher.iter(|| daily_summary(conn, day, None, &ShopContext::default()).unwrap())
            });
            for (name, report) in budgets::every_report(conn) {
                group.bench_function(&name, |bencher| bencher.iter(&report));
            }
            group.finish();
            Ok(())
        })
        .unwrap();
}

criterion_group!(benches, every_report);
criterion_main!(benches);
