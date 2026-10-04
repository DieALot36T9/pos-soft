//! Criterion benchmarks for search and scanning over 50,000 products (NFR-P1, NFR-P2).
//! Run with `cargo bench -p catalog`. The pass/fail budgets are checked by
//! `tests/benchmark.rs`; this gives detailed timings to compare between changes.

#![allow(clippy::unwrap_used)]

#[path = "../tests/common/mod.rs"]
mod common;

use criterion::{Criterion, criterion_group, criterion_main};

fn search_and_scan(criterion: &mut Criterion) {
    let mut db = common::test_db();
    common::seed_reference(&mut db, 50_000);

    for query in ["mil", "rice sug", "p0012"] {
        criterion.bench_function(&format!("search {query}"), |bencher| {
            bencher.iter(|| catalog::search(&db, query, 50).unwrap())
        });
    }
    criterion.bench_function("find_by_code barcode", |bencher| {
        bencher.iter(|| catalog::find_by_code(&db, "8900000049999").unwrap())
    });
    criterion.bench_function("find_by_code sku", |bencher| {
        bencher.iter(|| catalog::find_by_code(&db, "P025000").unwrap())
    });
}

criterion_group!(benches, search_and_scan);
criterion_main!(benches);
