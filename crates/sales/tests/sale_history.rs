//! Completed sales afterwards: reading them back, the exchange link, and that they never change.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Id, Money, Qty};
use sales::{NewPayment, PaymentKind, SalesError};
use support::{Till, cart_with, now, sale_of_cart, sell, test_db};

#[test]
fn ret_05_exchange_sale_keeps_the_link_to_its_return() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let return_id = support_return(&mut conn, &till);
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    sales::set_exchange_return(&tx, &cart, Some(&return_id), now()).unwrap();
    assert_eq!(
        sales::get_cart(&tx, &cart).unwrap().exchange_return_id,
        Some(return_id.clone())
    );
    let mut sale = sale_of_cart(&tx, &till, &cart);
    sale.exchange_return_id = Some(return_id.clone());
    let sale_ref = sales::complete(&tx, sale).unwrap();
    assert_eq!(
        sales::get_sale(&tx, &sale_ref.id)
            .unwrap()
            .exchange_return_id,
        Some(return_id)
    );

    let error = sales::set_exchange_return(&tx, &cart, Some(&Id::new()), now()).unwrap_err();
    assert!(matches!(error, SalesError::NotFound { .. }));
}

/// Sells one item and returns it, giving the return's id.
fn support_return(conn: &mut rusqlite::Connection, till: &Till) -> Id {
    let sale = sell(conn, till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let saved = sales::get_sale(&tx, &sale.id).unwrap();
    let new_return = sales::NewReturn {
        sale_id: Some(sale.id),
        till_id: till.till.clone(),
        till_code: till.code.clone(),
        shift_id: till.shift.clone(),
        user_id: till.user.clone(),
        approved_by: None,
        business_date: support::today(),
        at: now(),
        customer_id: None,
        reason: None,
        lines: vec![sales::NewReturnLine::FromSale(sales::ReturnedSaleLine {
            sale_line_id: saved.lines[0].id.clone(),
            qty: Qty(1_000),
            restock: true,
        })],
        rounding: Money(0),
        payments: vec![NewPayment {
            method_id: sales::system_payment_method(&tx, PaymentKind::ExchangeCredit)
                .unwrap()
                .id,
            amount: saved.totals.gross,
            tendered: None,
            reference: None,
        }],
    };
    let return_ref = sales::record_return(&tx, new_return).unwrap();
    tx.commit().unwrap();
    return_ref.id
}

#[test]
fn reading_sales_back_by_invoice_and_last_on_the_till() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let other = Till::new("T2");
    assert!(!sales::has_any_sale(&conn).unwrap());
    assert!(sales::last_sale(&conn, &till.till).unwrap().is_none());

    let first = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let second = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    sell(&mut conn, &other, &[(&Id::new(), 1_000)]);

    assert!(sales::has_any_sale(&conn).unwrap());
    assert_eq!(
        sales::find_by_invoice(&conn, " T1-000001 ")
            .unwrap()
            .unwrap()
            .id,
        first.id
    );
    assert!(
        sales::find_by_invoice(&conn, "T1-999999")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        sales::last_sale(&conn, &till.till).unwrap().unwrap().id,
        second.id
    );
    assert!(matches!(
        sales::get_sale(&conn, &Id::new()),
        Err(SalesError::NotFound { .. })
    ));
}

#[test]
fn a_completed_sale_can_never_be_edited_or_deleted() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let sale = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let id = sale.id.as_str();
    for sql in [
        "UPDATE sales_sales SET total_minor = 1 WHERE id = ?1",
        "DELETE FROM sales_sales WHERE id = ?1",
        "UPDATE sales_lines SET gross_minor = 1 WHERE sale_id = ?1",
        "DELETE FROM sales_lines WHERE sale_id = ?1",
        "UPDATE sales_payments SET amount_minor = 1 WHERE doc_id = ?1",
        "DELETE FROM sales_payments WHERE doc_id = ?1",
    ] {
        assert!(conn.execute(sql, [id]).is_err(), "{sql} should be refused");
    }
}
