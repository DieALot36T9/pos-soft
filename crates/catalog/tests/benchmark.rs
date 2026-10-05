//! Speed budgets over the reference database of 50,000 products (NFR-P1, NFR-P2).
//!
//! Building 50,000 products takes a while, so this test runs only when asked:
//! `cargo nextest run -p catalog --release --run-ignored only`.
//! `cargo bench -p catalog` measures the same searches in more detail (criterion).

#![allow(clippy::unwrap_used)]

mod common;

use std::time::{Duration, Instant};

use common::{seed_reference, test_db};

/// The fastest of three runs, so one slow moment on a busy computer doesn't fail the test.
fn best_of_three(mut work: impl FnMut()) -> Duration {
    let mut best = Duration::MAX;
    for _ in 0..3 {
        let started = Instant::now();
        work();
        best = best.min(started.elapsed());
    }
    best
}

#[test]
#[ignore = "builds 50,000 products; run with --run-ignored only"]
fn nfr_p2_search_takes_at_most_150_ms_and_find_by_code_2_ms_over_50k_products() {
    let mut db = test_db();
    seed_reference(&mut db, 50_000);

    // NFR-P2: a search keystroke shows results within 150 ms (also CHK-03).
    for query in [
        "mi",
        "mil",
        "milk",
        "rice sug",
        "sha 200",
        "p0012",
        "8900000123",
        "brand 12",
    ] {
        let took = best_of_three(|| {
            catalog::search(&db, query, 50).unwrap();
        });
        assert!(
            took <= Duration::from_millis(150),
            "search {query:?} took {took:?}"
        );
    }

    // NFR-P1: a scan is looked up within 2 ms (part of the 100 ms scan budget).
    for code in ["8900000049999", "2000000000002", "P025000", "unknown"] {
        let took = best_of_three(|| {
            catalog::find_by_code(&db, code).unwrap();
        });
        assert!(
            took <= Duration::from_millis(2),
            "find_by_code {code:?} took {took:?}"
        );
    }
}
