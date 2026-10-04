//! PUR-05 Payables (ledger, payments, aging) and PUR-06 Supplier statement.
// Helpers outside #[test] functions may unwrap too: a failing test should stop right there.
#![allow(clippy::unwrap_used)]

mod common;

use common::{add_supplier, ctx, date, direct_grn, grn_line, money, qty, test_db};
use pos_core::{FixedClock, Id, Money, Timestamp, business_date};
use purchasing::{LedgerKind, NewReturnLine, NewSupplierReturn, PurchasingError};
use rusqlite::Transaction;

/// Records a GRN for one line whose value is `amount_minor`, on `day`.
fn invoice(tx: &Transaction, supplier: &Id, amount_minor: i64, day: &str) {
    let grn = direct_grn(
        supplier,
        vec![grn_line("Goods", 1, amount_minor)],
        amount_minor,
    );
    purchasing::record_grn(tx, &grn, &ctx(day)).unwrap();
}

fn pay(tx: &Transaction, supplier: &Id, amount_minor: i64, day: &str) -> Id {
    purchasing::record_payment(tx, supplier, money(amount_minor), "cash", None, &ctx(day)).unwrap()
}

#[test]
fn pur_05_balance_after_invoice_payment_and_return() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);

    invoice(&tx, &supplier, 100_000, "2026-01-10");
    let payment = purchasing::record_payment(
        &tx,
        &supplier,
        money(30_000),
        "bank",
        Some("CHQ 1234"),
        &ctx("2026-01-11"),
    )
    .unwrap();
    let sent_back = NewSupplierReturn {
        supplier_id: supplier.clone(),
        grn_id: None,
        reason: Some("Expired".to_string()),
        lines: vec![NewReturnLine {
            product_id: Id::new(),
            product_name: "Goods".to_string(),
            qty: qty(1),
            unit_cost: money(5_000),
            inventory_batch_id: None,
        }],
    };
    purchasing::record_return(&tx, &sent_back, &ctx("2026-01-12")).unwrap();

    // 1,000.00 − 300.00 − 50.00 = 650.00
    assert_eq!(purchasing::balance(&tx, &supplier).unwrap(), Money(65_000));
    let from_view: i64 = tx
        .query_row(
            "SELECT balance_minor FROM purchasing_v_payables WHERE supplier_id = ?1",
            [supplier.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(from_view, 65_000);
    let (kind, amount, method, reference): (String, i64, String, String) = tx
        .query_row(
            "SELECT kind, amount_minor, method_kind, reference FROM purchasing_v_ledger WHERE id = ?1",
            [payment.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        (kind.as_str(), amount, method.as_str(), reference.as_str()),
        ("payment", -30_000, "bank", "CHQ 1234")
    );
    assert!(purchasing::unbalanced_suppliers(&tx).unwrap().is_empty());
}

#[test]
fn pur_05_ledger_is_append_only() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let payment = pay(&tx, &supplier, 1_000, "2026-01-10");

    let update = tx.execute(
        "UPDATE purchasing_ledger SET amount_minor = 0 WHERE id = ?1",
        [payment.as_str()],
    );
    let delete = tx.execute(
        "DELETE FROM purchasing_ledger WHERE id = ?1",
        [payment.as_str()],
    );

    assert!(update.is_err());
    assert!(delete.is_err());
    assert_eq!(purchasing::balance(&tx, &supplier).unwrap(), Money(-1_000));
}

#[test]
fn pur_05_payment_must_be_positive_and_have_a_method() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let today = ctx("2026-01-10");

    let zero = purchasing::record_payment(&tx, &supplier, Money(0), "cash", None, &today);
    let no_method = purchasing::record_payment(&tx, &supplier, Money(5), " ", None, &today);
    let unknown = purchasing::record_payment(&tx, &Id::new(), Money(5), "cash", None, &today);

    assert!(matches!(
        zero,
        Err(PurchasingError::Invalid {
            code: "amount_not_positive",
            ..
        })
    ));
    assert!(matches!(
        no_method,
        Err(PurchasingError::Invalid {
            code: "method_missing",
            ..
        })
    ));
    assert!(matches!(
        unknown,
        Err(PurchasingError::NotFound {
            entity: "supplier",
            ..
        })
    ));
}

#[test]
fn pur_05_adjustment_moves_the_balance_either_way() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);

    purchasing::record_adjustment(
        &tx,
        &supplier,
        Money(20_000),
        "Opening balance",
        &ctx("2026-01-01"),
    )
    .unwrap();
    purchasing::record_adjustment(&tx, &supplier, Money(-500), "Rounding", &ctx("2026-01-02"))
        .unwrap();
    let zero =
        purchasing::record_adjustment(&tx, &supplier, Money(0), "Nothing", &ctx("2026-01-02"));
    let no_reason = purchasing::record_adjustment(&tx, &supplier, Money(1), "", &ctx("2026-01-02"));

    assert_eq!(purchasing::balance(&tx, &supplier).unwrap(), Money(19_500));
    assert!(matches!(
        zero,
        Err(PurchasingError::Invalid {
            code: "amount_zero",
            ..
        })
    ));
    assert!(matches!(
        no_reason,
        Err(PurchasingError::Invalid {
            code: "reason_missing",
            ..
        })
    ));
}

#[test]
fn pur_05_aging_buckets_with_a_fixed_clock() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 10);
    let paid_up = add_supplier(&tx, "Bakery", 0);
    // Due dates are 10 days after each GRN.
    invoice(&tx, &supplier, 50_000, "2025-10-01"); // due 10-11: 112 days late → 90+
    invoice(&tx, &supplier, 40_000, "2025-11-15"); // due 11-25: 67 days late → 61–90
    invoice(&tx, &supplier, 30_000, "2025-12-10"); // due 12-20: 42 days late → 31–60
    invoice(&tx, &supplier, 20_000, "2026-01-05"); // due 01-15: 16 days late → 1–30
    invoice(&tx, &supplier, 10_000, "2026-01-25"); // due 02-04: not due yet → current
    // A payment pays off the oldest invoices first: all of the 90+ one and 100.00 of the next.
    pay(&tx, &supplier, 60_000, "2026-01-20");
    invoice(&tx, &paid_up, 5_000, "2026-01-10");
    pay(&tx, &paid_up, 5_000, "2026-01-11");

    // Dhaka, 31 January 2026 at 15:00 local time.
    let clock = FixedClock::new(Timestamp(1_769_850_000_000), 360);
    let today = business_date(pos_core::Clock::now(&clock), 360, 4)
        .unwrap()
        .0;
    assert_eq!(today, date("2026-01-31"));
    let rows = purchasing::aging(&tx, today).unwrap();

    assert_eq!(rows.len(), 1, "suppliers who owe nothing are left out");
    let row = &rows[0];
    assert_eq!(row.supplier_id, supplier);
    assert_eq!(row.name, "Fresh Farms");
    assert_eq!(row.current, Money(10_000));
    assert_eq!(row.days_1_30, Money(20_000));
    assert_eq!(row.days_31_60, Money(30_000));
    assert_eq!(row.days_61_90, Money(30_000));
    assert_eq!(row.days_over_90, Money(0));
    assert_eq!(row.balance, Money(90_000));
}

#[test]
fn pur_06_statement_has_opening_and_closing_balances() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    invoice(&tx, &supplier, 10_000, "2026-01-05");
    invoice(&tx, &supplier, 20_000, "2026-01-15");
    pay(&tx, &supplier, 5_000, "2026-01-20");
    invoice(&tx, &supplier, 7_000, "2026-02-02");

    let statement =
        purchasing::statement(&tx, &supplier, date("2026-01-10"), date("2026-01-31")).unwrap();

    assert_eq!(statement.supplier_name, "Fresh Farms");
    assert_eq!(statement.opening, Money(10_000));
    assert_eq!(statement.lines.len(), 2);
    assert_eq!(statement.lines[0].kind, LedgerKind::Invoice);
    assert_eq!(statement.lines[0].balance_after, Money(30_000));
    assert_eq!(statement.lines[1].kind, LedgerKind::Payment);
    assert_eq!(statement.lines[1].amount, Money(-5_000));
    assert_eq!(statement.lines[1].balance_after, Money(25_000));
    assert_eq!(statement.closing, Money(25_000));
}

#[test]
fn pur_06_statement_refuses_a_backwards_date_range() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);

    let result = purchasing::statement(&tx, &supplier, date("2026-02-01"), date("2026-01-01"));

    assert!(matches!(
        result,
        Err(PurchasingError::Invalid {
            code: "date_range_backwards",
            ..
        })
    ));
}
