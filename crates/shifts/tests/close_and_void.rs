//! Contract tests for closing a shift, the summary and voids.
//! Each test names the requirement it covers (docs/02 Requirements/Req - Shifts and Cash.md).

mod common;

use common::{NOW, for_ref, movement, open_shift, record, taka, test_db, with_reason, write};
use pos_core::{Id, Timestamp};
use shifts::{CloseResult, MethodTotal, MovementKind, MovementTotal, ShiftStatus, ShiftsError};

// ---- SHF-06 Close shift ----

/// The numbers from SHF-06: expected 12,450.00.
fn shf_06_shift(conn: &mut rusqlite::Connection, user: &Id) -> Id {
    let shift = open_shift(conn, &Id::new(), user, taka(2_000));
    record(
        conn,
        &shift,
        movement(MovementKind::SalePayment, "cash", taka(11_000), user),
    );
    record(
        conn,
        &shift,
        movement(MovementKind::SalePayment, "card", taka(3_000), user),
    );
    record(
        conn,
        &shift,
        movement(MovementKind::Refund, "cash", taka(-300), user),
    );
    let extra_change = movement(MovementKind::CashIn, "cash", taka(500), user);
    record(conn, &shift, with_reason(extra_change, "Extra change"));
    let delivery = movement(MovementKind::CashOut, "cash", taka(-1_000), user);
    record(conn, &shift, with_reason(delivery, "Delivery"));
    record(
        conn,
        &shift,
        movement(MovementKind::CustomerPayment, "cash", taka(250), user),
    );
    shift
}

#[test]
fn shf_06_expected_12450_counted_12400_records_a_shortage_of_50() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = shf_06_shift(&mut conn, &user);
    assert_eq!(shifts::expected_cash(&conn, &shift).unwrap(), taka(12_450));

    let counts = [(taka(1_000), 12), (taka(100), 4)];
    let result = write(&mut conn, |tx| {
        shifts::close(
            tx,
            &shift,
            &counts,
            &user,
            false,
            None,
            Timestamp(NOW.0 + 1),
        )
        .unwrap()
    });
    assert_eq!(
        result,
        CloseResult {
            expected_cash: taka(12_450),
            counted_cash: Some(taka(12_400)),
            difference: Some(taka(-50)),
        }
    );

    let summary = shifts::summary(&conn, &shift).unwrap();
    assert_eq!(summary.shift.status, ShiftStatus::Closed);
    assert_eq!(summary.shift.closed_at, Some(Timestamp(NOW.0 + 1)));
    assert_eq!(summary.shift.closed_by, Some(user));
    assert!(!summary.shift.forced);
    assert_eq!(summary.shift.expected_cash, Some(taka(12_450)));
    assert_eq!(summary.shift.counted_cash, Some(taka(12_400)));
    assert_eq!(summary.shift.difference, Some(taka(-50)));
    assert_eq!(summary.close_counts.len(), 2);
}

#[test]
fn shf_06_expected_cash_adds_every_cash_kind_with_its_sign_and_skips_other_methods() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));
    let cash_rows = [
        movement(MovementKind::SalePayment, "cash", taka(500), &user),
        movement(MovementKind::Refund, "cash", taka(-50), &user),
        with_reason(
            movement(MovementKind::CashIn, "cash", taka(200), &user),
            "Change",
        ),
        with_reason(
            movement(MovementKind::CashOut, "cash", taka(-30), &user),
            "Tea",
        ),
        with_reason(
            movement(MovementKind::CashDrop, "cash", taka(-400), &user),
            "Safe",
        ),
        movement(MovementKind::CustomerPayment, "cash", taka(70), &user),
        movement(MovementKind::SupplierPayment, "cash", taka(-90), &user),
    ];
    for row in cash_rows {
        record(&mut conn, &shift, row);
    }
    for method in ["card", "wallet"] {
        record(
            &mut conn,
            &shift,
            movement(MovementKind::SalePayment, method, taka(999), &user),
        );
        record(
            &mut conn,
            &shift,
            movement(MovementKind::Refund, method, taka(-9), &user),
        );
    }

    // 1,000 + 500 − 50 + 200 − 30 − 400 + 70 − 90 = 1,200.
    assert_eq!(shifts::expected_cash(&conn, &shift).unwrap(), taka(1_200));
}

#[test]
fn shf_06_closing_without_a_count_is_refused() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));
    let error = write(&mut conn, |tx| {
        shifts::close(tx, &shift, &[], &user, false, None, NOW).unwrap_err()
    });
    assert_eq!(error, ShiftsError::Invalid("count_missing"));
}

#[test]
fn shf_06_a_closed_shift_takes_no_more_movements_and_cannot_close_again() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));
    write(&mut conn, |tx| {
        shifts::close(tx, &shift, &[(taka(1_000), 1)], &user, false, None, NOW).unwrap()
    });

    let error = write(&mut conn, |tx| {
        let sale = movement(MovementKind::SalePayment, "cash", taka(10), &user);
        shifts::record(tx, &shift, sale).unwrap_err()
    });
    assert_eq!(error, ShiftsError::Closed(shift.clone()));

    let error = write(&mut conn, |tx| {
        shifts::close(tx, &shift, &[(taka(1_000), 1)], &user, false, None, NOW).unwrap_err()
    });
    assert_eq!(error, ShiftsError::Closed(shift));
}

// ---- SHF-07 Force close ----

#[test]
fn shf_07_force_close_needs_no_count_and_is_marked_forced() {
    let mut conn = test_db();
    let (cashier, admin) = (Id::new(), Id::new());
    let shift = open_shift(&mut conn, &Id::new(), &cashier, taka(1_000));
    record(
        &mut conn,
        &shift,
        movement(MovementKind::SalePayment, "cash", taka(250), &cashier),
    );

    let note = Some("Cashier went home".to_string());
    let result = write(&mut conn, |tx| {
        shifts::close(tx, &shift, &[], &admin, true, note.clone(), NOW).unwrap()
    });
    assert_eq!(
        result,
        CloseResult {
            expected_cash: taka(1_250),
            counted_cash: None,
            difference: None
        }
    );

    let closed = shifts::summary(&conn, &shift).unwrap().shift;
    assert!(closed.forced);
    assert_eq!(closed.closed_by, Some(admin));
    assert_eq!(closed.close_note, note);
    assert_eq!(closed.counted_cash, None);
}

// ---- SHF-08 Z report content: totals by kind and method ----

#[test]
fn shf_08_summary_gives_totals_by_kind_and_payments_by_method() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = shf_06_shift(&mut conn, &user);
    let summary = shifts::summary(&conn, &shift).unwrap();

    let total = |kind, method: &str, count, amount| MovementTotal {
        kind,
        method_kind: method.to_string(),
        count,
        total: amount,
    };
    assert_eq!(
        summary.totals,
        vec![
            total(MovementKind::Float, "cash", 1, taka(2_000)),
            total(MovementKind::SalePayment, "card", 1, taka(3_000)),
            total(MovementKind::SalePayment, "cash", 1, taka(11_000)),
            total(MovementKind::Refund, "cash", 1, taka(-300)),
            total(MovementKind::CashIn, "cash", 1, taka(500)),
            total(MovementKind::CashOut, "cash", 1, taka(-1_000)),
            total(MovementKind::CustomerPayment, "cash", 1, taka(250)),
        ]
    );
    // Payments by method are sales − refunds (SHF-08).
    assert_eq!(
        summary.payments_by_method,
        vec![
            MethodTotal {
                method_kind: "card".to_string(),
                method_id: None,
                total: taka(3_000)
            },
            MethodTotal {
                method_kind: "cash".to_string(),
                method_id: None,
                total: taka(10_700)
            },
        ]
    );
    assert_eq!(summary.expected_cash, taka(12_450));
}

// ---- RET-04 Void a completed sale: reverse_ref ----

#[test]
fn ret_04_reverse_ref_returns_the_shift_totals_to_before_the_voided_sale() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));
    record(
        &mut conn,
        &shift,
        movement(MovementKind::SalePayment, "cash", taka(300), &user),
    );
    record(
        &mut conn,
        &shift,
        movement(MovementKind::SalePayment, "card", taka(200), &user),
    );
    let before = shifts::summary(&conn, &shift).unwrap();

    let sale = Id::new();
    let cash_part = movement(MovementKind::SalePayment, "cash", taka(450), &user);
    let card_part = movement(MovementKind::SalePayment, "card", taka(120), &user);
    record(&mut conn, &shift, for_ref(cash_part, "sale", &sale));
    record(&mut conn, &shift, for_ref(card_part, "sale", &sale));
    assert_eq!(shifts::expected_cash(&conn, &shift).unwrap(), taka(1_750));

    write(&mut conn, |tx| {
        shifts::reverse_ref(tx, &shift, "sale", &sale, &user, NOW).unwrap()
    });

    let after = shifts::summary(&conn, &shift).unwrap();
    assert_eq!(after.expected_cash, before.expected_cash);
    assert_eq!(after.payments_by_method, before.payments_by_method);
    let reversed: Vec<_> = after
        .totals
        .iter()
        .filter(|row| row.kind == MovementKind::VoidReversal)
        .collect();
    assert_eq!(reversed.len(), 2);
}

#[test]
fn ret_04_a_sale_can_be_reversed_only_once() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));
    let sale = Id::new();
    let payment = movement(MovementKind::SalePayment, "cash", taka(450), &user);
    record(&mut conn, &shift, for_ref(payment, "sale", &sale));
    write(&mut conn, |tx| {
        shifts::reverse_ref(tx, &shift, "sale", &sale, &user, NOW).unwrap()
    });

    let error = write(&mut conn, |tx| {
        shifts::reverse_ref(tx, &shift, "sale", &sale, &user, NOW).unwrap_err()
    });
    assert!(matches!(error, ShiftsError::AlreadyReversed { .. }));
    assert_eq!(shifts::expected_cash(&conn, &shift).unwrap(), taka(1_000));
}

#[test]
fn ret_04_reversing_a_sale_with_no_movements_is_refused() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));
    let error = write(&mut conn, |tx| {
        shifts::reverse_ref(tx, &shift, "sale", &Id::new(), &user, NOW).unwrap_err()
    });
    assert!(matches!(error, ShiftsError::NothingToReverse { .. }));
}

#[test]
fn ret_04_a_sale_from_a_closed_shift_cannot_be_reversed() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));
    let sale = Id::new();
    let payment = movement(MovementKind::SalePayment, "cash", taka(450), &user);
    record(&mut conn, &shift, for_ref(payment, "sale", &sale));
    write(&mut conn, |tx| {
        shifts::close(tx, &shift, &[(taka(1_450), 1)], &user, false, None, NOW).unwrap()
    });

    let error = write(&mut conn, |tx| {
        shifts::reverse_ref(tx, &shift, "sale", &sale, &user, NOW).unwrap_err()
    });
    assert_eq!(error, ShiftsError::Closed(shift));
}
