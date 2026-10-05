//! How long one PIN check takes with the argon2id settings (Security Model).
//! The target is 300 ms or less on the reference PC. Run with `cargo bench -p auth`.

// A benchmark may stop at the first failure, like a test.
#![allow(clippy::unwrap_used)]

use std::hint::black_box;

use auth::{CASHIER_ROLE_ID, LoginOutcome, NewUser};
use criterion::{Criterion, criterion_group, criterion_main};
use pos_core::{Id, Secret, Timestamp};
use rusqlite::Connection;

fn login_check(criterion: &mut Criterion) {
    let mut connection = Connection::open_in_memory().unwrap();
    for migration in auth::migrations().migrations {
        connection.execute_batch(migration.sql).unwrap();
    }
    let transaction = connection.transaction().unwrap();
    let new_user = NewUser {
        username: "rina".to_string(),
        display_name: "Rina".to_string(),
        role_id: Id::parse(CASHIER_ROLE_ID).unwrap(),
        password: None,
        pin: Some(Secret::new("1234".to_string())),
    };
    let user = auth::create_user(&transaction, new_user, Timestamp(0)).unwrap();

    criterion.bench_function("verify_login with a correct PIN", |bench| {
        bench.iter(|| {
            let pin = Secret::new("1234".to_string());
            let outcome = auth::verify_login(&transaction, &user, pin, Timestamp(0)).unwrap();
            assert!(matches!(black_box(outcome), LoginOutcome::LoggedIn(_)));
        });
    });
}

criterion_group! {
    name = benches;
    // Each check takes a while, so a small sample is enough.
    config = Criterion::default().sample_size(10);
    targets = login_check
}
criterion_main!(benches);
