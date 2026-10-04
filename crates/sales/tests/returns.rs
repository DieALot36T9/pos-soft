//! Returns and voids: returnable quantities, exact refunds, return numbers, voiding.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Bps, Id, Money, Qty};
use rusqlite::Connection;
use sales::{
    NewPayment, NewReturn, NewReturnLine, PaymentKind, ReturnQty, ReturnedProduct,
    ReturnedSaleLine, SaleRef, SalesError,
};
use support::{Till, now, priced_line, sell, sell_priced, test_db, today};

/// A return of `qty` thousandths of the sale's first line, refunded in cash.
fn return_of(conn: &Connection, till: &Till, sale: &SaleRef, qty: i64) -> NewReturn {
    let saved = sales::get_sale(conn, &sale.id).unwrap();
    let line = &saved.lines[0];
    let refund = sales::refund_amounts(
        conn,
        &sale.id,
        &[ReturnQty {
            sale_line_id: line.id.clone(),
            qty: Qty(qty),
        }],
    )
    .unwrap();
    NewReturn {
        sale_id: Some(sale.id.clone()),
        till_id: till.till.clone(),
        till_code: till.code.clone(),
        shift_id: till.shift.clone(),
        user_id: till.user.clone(),
        approved_by: None,
        business_date: today(),
        at: now(),
        customer_id: None,
        reason: Some("damaged box".into()),
        lines: vec![NewReturnLine::FromSale(ReturnedSaleLine {
            sale_line_id: line.id.clone(),
            qty: Qty(qty),
            restock: true,
        })],
        rounding: Money(0),
        payments: vec![cash_refund(conn, refund[0].amounts.gross)],
    }
}

fn cash_refund(conn: &Connection, amount: Money) -> NewPayment {
    NewPayment {
        tendered: None,
        ..support::exact_cash(conn, amount)
    }
}

fn record(conn: &mut Connection, new_return: NewReturn) -> sales::ReturnRef {
    let tx = conn.transaction().unwrap();
    let return_ref = sales::record_return(&tx, new_return).unwrap();
    tx.commit().unwrap();
    return_ref
}

#[test]
fn ret_01_returnable_quantity_after_partial_returns() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let sale = sell(
        &mut conn,
        &till,
        &[(&Id::new(), 3_000), (&Id::new(), 2_000)],
    );
    let lines = sales::get_sale(&conn, &sale.id).unwrap().lines;

    let new_return = return_of(&conn, &till, &sale, 1_000);
    record(&mut conn, new_return);

    let left = sales::returnable_qty(&conn, &sale.id).unwrap();
    assert_eq!(left[&lines[0].id], Qty(2_000));
    assert_eq!(left[&lines[1].id], Qty(2_000));
}

#[test]
fn ret_01_returning_more_than_is_left_is_refused() {
    // Sold × 3 with 1 already returned: trying to return 3 is refused (max 2).
    let mut conn = test_db();
    let till = Till::new("T1");
    let sale = sell(&mut conn, &till, &[(&Id::new(), 3_000)]);
    let new_return = return_of(&conn, &till, &sale, 1_000);
    record(&mut conn, new_return);

    let line = sales::get_sale(&conn, &sale.id).unwrap().lines[0]
        .id
        .clone();
    let error = sales::refund_amounts(
        &conn,
        &sale.id,
        &[ReturnQty {
            sale_line_id: line,
            qty: Qty(3_000),
        }],
    )
    .unwrap_err();
    let SalesError::Invalid { code, params } = error else {
        panic!("expected a validation error");
    };
    assert_eq!(code, "return_qty_too_large");
    assert_eq!(params.get("max_milli").map(String::as_str), Some("2000"));
}

#[test]
fn ret_01_a_line_from_another_sale_or_repeated_is_refused() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let sale = sell(&mut conn, &till, &[(&Id::new(), 3_000)]);
    let other = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let other_line = sales::get_sale(&conn, &other.id).unwrap().lines[0]
        .id
        .clone();
    let wanted = [ReturnQty {
        sale_line_id: other_line,
        qty: Qty(1_000),
    }];
    assert!(matches!(
        sales::refund_amounts(&conn, &sale.id, &wanted),
        Err(SalesError::NotFound { .. })
    ));

    let line = sales::get_sale(&conn, &sale.id).unwrap().lines[0]
        .id
        .clone();
    let twice = [
        ReturnQty {
            sale_line_id: line.clone(),
            qty: Qty(1_000),
        },
        ReturnQty {
            sale_line_id: line,
            qty: Qty(1_000),
        },
    ];
    let error = sales::refund_amounts(&conn, &sale.id, &twice).unwrap_err();
    assert_eq!(error.code(), Some("return_line_repeated"));
}

#[test]
fn e14_ret_02_three_single_returns_refund_the_line_exactly() {
    let mut conn = test_db();
    let till = Till::new("T1");
    // E14: a line of 100.00 for 3 units (3 × 40.00 with a 20.00 promotion).
    let product = Id::new();
    let mut line = priced_line(&product, Qty(3_000), Money(4_000));
    line.promo_discount = Money(2_000);
    line.gross = Money(10_000);
    line.tax = Money(1_304);
    line.net = Money(8_696);
    let sale = sell_priced(&mut conn, &till, vec![line]);
    let line = sales::get_sale(&conn, &sale.id).unwrap().lines[0].clone();

    let mut grosses = Vec::new();
    let mut nets = 0;
    let mut taxes = 0;
    let mut costs = 0;
    for _ in 0..3 {
        let new_return = return_of(&conn, &till, &sale, 1_000);
        let return_ref = record(&mut conn, new_return);
        let saved = sales::get_return(&conn, &return_ref.id).unwrap();
        grosses.push(saved.totals.gross.0);
        nets += saved.totals.net.0;
        taxes += saved.totals.tax.0;
        costs += saved.totals.cost.0;
        assert_eq!(
            saved.totals.net.0 + saved.totals.tax.0,
            saved.totals.gross.0
        );
    }
    assert_eq!(grosses, vec![3_333, 3_333, 3_334]);
    assert_eq!(nets, line.snapshot.net.0);
    assert_eq!(taxes, line.snapshot.tax.0);
    assert_eq!(costs, line.cost.0);
    assert_eq!(
        sales::returnable_qty(&conn, &sale.id).unwrap()[&line.id],
        Qty(0)
    );
}

#[test]
fn ret_02_the_last_return_refunds_exactly_what_is_left() {
    let mut conn = test_db();
    let till = Till::new("T1");
    // 1.234 kg at 90.00/kg = 111.06 gross.
    let sale = sell(&mut conn, &till, &[(&Id::new(), 1_234)]);
    let line = sales::get_sale(&conn, &sale.id).unwrap().lines[0].clone();
    let mut refunded = 0;
    for qty in [500, 500, 234] {
        let new_return = return_of(&conn, &till, &sale, qty);
        let return_ref = record(&mut conn, new_return);
        refunded += sales::get_return(&conn, &return_ref.id)
            .unwrap()
            .totals
            .gross
            .0;
    }
    assert_eq!(refunded, line.snapshot.gross.0);
}

#[test]
fn ret_06_return_numbers_count_per_till_with_an_r() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let sale = sell(&mut conn, &till, &[(&Id::new(), 3_000)]);
    let first = return_of(&conn, &till, &sale, 1_000);
    assert_eq!(record(&mut conn, first).return_no, "T1-R000001");
    let second = return_of(&conn, &till, &sale, 1_000);
    let return_ref = record(&mut conn, second);
    assert_eq!(return_ref.return_no, "T1-R000002");

    let saved = sales::get_return(&conn, &return_ref.id).unwrap();
    assert_eq!(saved.sale_id, Some(sale.id.clone()));
    assert_eq!(saved.reason.as_deref(), Some("damaged box"));
    assert_eq!(saved.lines.len(), 1);
    assert!(saved.lines[0].restock);
    assert_eq!(saved.payments[0].kind, PaymentKind::Cash);
    // Returns don't use up invoice numbers.
    assert_eq!(
        sell(&mut conn, &till, &[(&Id::new(), 1_000)]).invoice_no,
        "T1-000002"
    );
}

#[test]
fn ret_01_refunds_must_add_up_to_the_return_total() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let sale = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let mut new_return = return_of(&conn, &till, &sale, 1_000);
    new_return.payments = vec![cash_refund(&conn, Money(100))];
    let tx = conn.transaction().unwrap();
    assert!(matches!(
        sales::record_return(&tx, new_return.clone()),
        Err(SalesError::TotalsDontAddUp(_))
    ));

    // A refund of 90.00 with −0.50 cash rounding pays out 89.50.
    new_return.rounding = Money(-50);
    new_return.payments = vec![cash_refund(&tx, Money(8_950))];
    let return_ref = sales::record_return(&tx, new_return).unwrap();
    let saved = sales::get_return(&tx, &return_ref.id).unwrap();
    assert_eq!(saved.totals.total, Money(8_950));
    assert_eq!(saved.totals.rounding, Money(-50));
}

#[test]
fn ret_03_return_without_a_receipt() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let tx = conn.transaction().unwrap();
    let product = ReturnedProduct {
        product_id: Id::new(),
        name: "Rice 5kg".into(),
        qty: Qty(2_000),
        net: Money(80_000),
        tax: Money(12_000),
        gross: Money(92_000),
        tax_rate_id: Id::new(),
        tax_bps: Bps(1_500),
        unit_cost: Money(35_000),
        restock: false,
    };
    let new_return = NewReturn {
        sale_id: None,
        till_id: till.till.clone(),
        till_code: "T1".into(),
        shift_id: till.shift.clone(),
        user_id: till.user.clone(),
        approved_by: Some(Id::new()),
        business_date: today(),
        at: now(),
        customer_id: None,
        reason: None,
        lines: vec![NewReturnLine::WithoutReceipt(product)],
        rounding: Money(0),
        payments: vec![cash_refund(&tx, Money(92_000))],
    };
    let return_ref = sales::record_return(&tx, new_return.clone()).unwrap();
    let saved = sales::get_return(&tx, &return_ref.id).unwrap();
    assert_eq!(saved.sale_id, None);
    assert_eq!(saved.lines[0].sale_line_id, None);
    assert_eq!(saved.totals.cost, Money(70_000));
    assert!(!saved.lines[0].restock);
    assert!(sales::has_any_sale(&tx).unwrap());

    // A return without a receipt can't include lines of a sale.
    let mut mixed = new_return;
    mixed.lines.push(NewReturnLine::FromSale(ReturnedSaleLine {
        sale_line_id: Id::new(),
        qty: Qty(1_000),
        restock: true,
    }));
    assert_eq!(
        sales::record_return(&tx, mixed).unwrap_err().code(),
        Some("return_lines_mixed")
    );
}

#[test]
fn ret_04_a_sale_with_returns_cannot_be_voided_and_a_void_sale_cannot_be_returned() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let returned = sell(&mut conn, &till, &[(&Id::new(), 2_000)]);
    let new_return = return_of(&conn, &till, &returned, 1_000);
    record(&mut conn, new_return);
    let voided = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);

    let tx = conn.transaction().unwrap();
    let error = sales::void(&tx, &returned.id, &till.user, None, "too late", now()).unwrap_err();
    assert_eq!(error.code(), Some("sale_has_returns"));

    sales::void(&tx, &voided.id, &till.user, None, "mistake", now()).unwrap();
    let line = sales::get_sale(&tx, &voided.id).unwrap().lines[0]
        .id
        .clone();
    let error = sales::refund_amounts(
        &tx,
        &voided.id,
        &[ReturnQty {
            sale_line_id: line,
            qty: Qty(1_000),
        }],
    )
    .unwrap_err();
    assert_eq!(error.code(), Some("sale_voided"));
}

#[test]
fn returns_can_never_be_edited_or_deleted() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let sale = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let new_return = return_of(&conn, &till, &sale, 1_000);
    let return_ref = record(&mut conn, new_return);
    let id = return_ref.id.as_str();
    for sql in [
        "UPDATE sales_returns SET total_minor = 1 WHERE id = ?1",
        "DELETE FROM sales_returns WHERE id = ?1",
        "UPDATE sales_return_lines SET gross_minor = 1 WHERE return_id = ?1",
        "DELETE FROM sales_return_lines WHERE return_id = ?1",
    ] {
        assert!(conn.execute(sql, [id]).is_err(), "{sql} should be refused");
    }
}
