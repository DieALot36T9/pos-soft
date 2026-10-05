//! Crash recovery: carts and held carts survive closing and reopening the database (CHK-09, CHK-19).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::Id;
use rusqlite::Connection;
use support::{Till, cart_with, now, run_migrations};

#[test]
fn chk_19_cart_is_back_after_reopening_the_database() {
    let folder = std::env::temp_dir().join(format!("pos-sales-test-{}", Id::new()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("pos.sqlite");
    let till = Till::new("T1");
    let products: Vec<Id> = (0..5).map(|_| Id::new()).collect();
    let (before, held_before) = {
        let mut conn = Connection::open(&path).unwrap();
        run_migrations(&conn);
        let held = cart_with(&mut conn, &till, &[(&products[0], 1_000)]);
        let tx = conn.transaction().unwrap();
        sales::hold(&tx, &held, Some("back soon".into()), now()).unwrap();
        tx.commit().unwrap();
        let lines: Vec<(&Id, i64)> = products.iter().map(|product| (product, 1_250)).collect();
        let cart = cart_with(&mut conn, &till, &lines);
        (
            sales::get_cart(&conn, &cart).unwrap(),
            sales::held_carts(&conn, &till.till).unwrap(),
        )
        // The connection closes here, like the app being switched off.
    };

    let conn = Connection::open(&path).unwrap();
    let after = sales::current_cart(&conn, &till.till, &till.user)
        .unwrap()
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(after.lines.len(), 5);
    assert_eq!(sales::held_carts(&conn, &till.till).unwrap(), held_before);

    drop(conn);
    std::fs::remove_dir_all(&folder).unwrap();
}
