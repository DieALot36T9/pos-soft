//! Voiding completed sales (RET-04).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::Id;
use sales::SaleStatus;
use support::{Till, now, sell, test_db};

#[test]
fn ret_04_void_keeps_the_number_and_contents() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let manager = Id::new();
    let sale = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let before = sales::get_sale(&conn, &sale.id).unwrap();
    // The workflow checks this shift is still open before voiding.
    assert_eq!(before.shift_id, till.shift);

    let tx = conn.transaction().unwrap();
    sales::void(
        &tx,
        &sale.id,
        &till.user,
        Some(&manager),
        "customer changed mind",
        now(),
    )
    .unwrap();
    tx.commit().unwrap();

    let after = sales::get_sale(&conn, &sale.id).unwrap();
    assert_eq!(after.status, SaleStatus::Voided);
    assert_eq!(after.invoice_no, before.invoice_no);
    assert_eq!(after.lines, before.lines);
    assert_eq!(after.totals, before.totals);
    let void = after.void.unwrap();
    assert_eq!(void.by, till.user);
    assert_eq!(void.approved_by, Some(manager));
    assert_eq!(void.reason, "customer changed mind");
    assert_eq!(void.at, now());
}

#[test]
fn ret_04_void_needs_a_reason_and_happens_once() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let sale = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let error = sales::void(&tx, &sale.id, &till.user, None, "  ", now()).unwrap_err();
    assert_eq!(error.code(), Some("void_reason_missing"));
    sales::void(&tx, &sale.id, &till.user, None, "wrong items", now()).unwrap();
    let error = sales::void(&tx, &sale.id, &till.user, None, "again", now()).unwrap_err();
    assert_eq!(error.code(), Some("sale_already_voided"));
    // The database refuses it too, even without the module.
    assert!(
        tx.execute(
            "UPDATE sales_sales SET void_reason = 'x' WHERE id = ?1",
            [sale.id.as_str()]
        )
        .is_err()
    );
}
