//! Contract tests: the migrations and the published `sales_v_*` views look and behave
//! as the module page says. Reports read these views, so a change here must be deliberate.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Id, Money, Qty};
use rusqlite::Connection;
use sales::{NewReturn, NewReturnLine, ReturnedSaleLine};
use support::{Till, now, sell, test_db, today};

fn columns(conn: &Connection, view: &str) -> Vec<String> {
    let statement = conn.prepare(&format!("SELECT * FROM {view}")).unwrap();
    statement
        .column_names()
        .iter()
        .map(|name| name.to_string())
        .collect()
}

fn names(list: &str) -> Vec<String> {
    list.split(',')
        .map(|name| name.trim().to_string())
        .collect()
}

const SALE_LINE_COLUMNS: &str =
    "id, sale_id, seq, product_id, name, sku, unit_symbol, category_id, brand_id,
    qty_milli, unit_price_minor, retail_price_minor, base_minor, promotion_id, promotion_name,
    promo_discount_minor, tier_discount_minor, manual_discount_minor, invoice_discount_minor,
    net_minor, tax_minor, gross_minor, tax_rate_id, tax_name, tax_bps, tax_components_json,
    unit_cost_minor, cost_minor, price_overridden";

const SALE_COLUMNS: &str = "id, invoice_no, till_id, shift_id, user_id, business_date, completed_at,
    customer_id, customer_name, customer_phone, price_list_id, status,
    base_minor, promo_discount_minor, tier_discount_minor, manual_discount_minor, invoice_discount_minor,
    net_minor, tax_minor, gross_minor, rounding_minor, total_minor, change_minor, cost_minor,
    tax_mode, exchange_return_id, voided_at, voided_by, void_approved_by, void_reason";

const RETURN_LINE_COLUMNS: &str = "id, return_id, sale_line_id, product_id, name, qty_milli,
    net_minor, tax_minor, gross_minor, tax_rate_id, tax_bps, unit_cost_minor, cost_minor, restock";

const PAYMENT_COLUMNS: &str =
    "id, doc_type, doc_id, method_id, kind, amount_minor, tendered_minor, reference, at";

#[test]
fn migrations_belong_to_this_module() {
    let migrations = sales::migrations();
    assert_eq!(migrations.module, "sales");
    assert_eq!(migrations.migrations[0].version, 1);
    assert_eq!(migrations.migrations[0].name, "init");
}

#[test]
fn every_table_view_index_and_trigger_is_prefixed_with_sales() {
    let conn = test_db();
    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'")
        .unwrap();
    let all: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(all.len() > 20);
    for name in all {
        assert!(name.starts_with("sales_"), "{name} is not prefixed");
    }
}

#[test]
fn sales_v_lines_has_the_line_and_header_columns() {
    let conn = test_db();
    let mut expected = names(SALE_LINE_COLUMNS);
    expected.extend(names(
        "invoice_no, business_date, completed_at, till_id, shift_id, user_id, customer_id, status",
    ));
    assert_eq!(columns(&conn, "sales_v_lines"), expected);
}

#[test]
fn sales_v_sales_has_the_sale_columns() {
    let conn = test_db();
    assert_eq!(columns(&conn, "sales_v_sales"), names(SALE_COLUMNS));
}

#[test]
fn sales_v_return_lines_has_the_line_and_header_columns() {
    let conn = test_db();
    let mut expected = names(RETURN_LINE_COLUMNS);
    expected.extend(names(
        "return_no, business_date, at, till_id, shift_id, user_id, customer_id, sale_id",
    ));
    assert_eq!(columns(&conn, "sales_v_return_lines"), expected);
}

#[test]
fn sales_v_payments_has_the_payment_columns_and_the_document_date() {
    let conn = test_db();
    let mut expected = names(PAYMENT_COLUMNS);
    expected.extend(names("business_date, shift_id, doc_status"));
    assert_eq!(columns(&conn, "sales_v_payments"), expected);
}

#[test]
fn views_show_sales_returns_and_voids_for_reports() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let kept = sell(&mut conn, &till, &[(&Id::new(), 2_000)]);
    let voided = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);

    let tx = conn.transaction().unwrap();
    sales::void(&tx, &voided.id, &till.user, None, "mistake", now()).unwrap();
    let line = sales::get_sale(&tx, &kept.id).unwrap().lines[0].clone();
    let refund = Money(line.snapshot.gross.0 / 2);
    sales::record_return(
        &tx,
        NewReturn {
            sale_id: Some(kept.id.clone()),
            till_id: till.till.clone(),
            till_code: till.code.clone(),
            shift_id: till.shift.clone(),
            user_id: till.user.clone(),
            approved_by: None,
            business_date: today(),
            at: now(),
            customer_id: None,
            reason: None,
            lines: vec![NewReturnLine::FromSale(ReturnedSaleLine {
                sale_line_id: line.id.clone(),
                qty: Qty(1_000),
                restock: true,
            })],
            rounding: Money(0),
            payments: vec![sales::NewPayment {
                tendered: None,
                ..support::exact_cash(&tx, refund)
            }],
        },
    )
    .unwrap();
    tx.commit().unwrap();

    let (invoice_no, date, status): (String, String, String) = conn
        .query_row(
            "SELECT invoice_no, business_date, status FROM sales_v_lines WHERE sale_id = ?1",
            [voided.id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        (invoice_no.as_str(), date.as_str(), status.as_str()),
        ("T1-000002", "2026-03-05", "voided")
    );

    let payment_of = |doc_id: &str| -> (String, i64, String, String) {
        conn.query_row(
            "SELECT doc_type, amount_minor, doc_status, business_date FROM sales_v_payments WHERE doc_id = ?1",
            [doc_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap()
    };
    assert_eq!(payment_of(voided.id.as_str()).2, "voided");
    assert_eq!(payment_of(kept.id.as_str()).2, "completed");
    let return_id: String = conn
        .query_row("SELECT id FROM sales_returns", [], |row| row.get(0))
        .unwrap();
    let refund_row = payment_of(&return_id);
    assert_eq!(
        refund_row,
        (
            "return".to_string(),
            refund.0,
            "completed".to_string(),
            "2026-03-05".to_string()
        )
    );

    let (return_no, sale_id, gross): (String, String, i64) = conn
        .query_row(
            "SELECT return_no, sale_id, gross_minor FROM sales_v_return_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(return_no, "T1-R000001");
    assert_eq!(sale_id, kept.id.as_str());
    assert_eq!(gross, refund.0);
}

#[test]
fn errors_turn_into_app_errors_with_stable_codes() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = support::cart_with(&mut conn, &till, &[]);
    let tx = conn.transaction().unwrap();
    let error: pos_core::AppError = sales::hold(&tx, &cart, None, now()).unwrap_err().into();
    assert!(matches!(error, pos_core::AppError::Validation { code, .. } if code == "cart_empty"));
}

#[test]
fn migrations_are_numbered_in_order() {
    for (index, migration) in sales::migrations().migrations.iter().enumerate() {
        assert_eq!(migration.version as usize, index + 1);
    }
}

/// RPT-11: reports read a year of sale lines (about 1.6 million) through `sales_v_lines`.
/// The columns they add up come from one index, so the wide line rows aren't read.
#[test]
fn rpt_11_reports_read_sale_lines_from_one_covering_index() {
    let conn = test_db();
    let plan: Vec<String> = conn
        .prepare(
            "EXPLAIN QUERY PLAN
             SELECT l.sale_id, l.product_id, l.category_id, l.brand_id, l.user_id, l.qty_milli,
                    l.net_minor, l.tax_minor, l.promo_discount_minor, l.tier_discount_minor,
                    l.manual_discount_minor, l.invoice_discount_minor, l.cost_minor,
                    l.tax_rate_id, l.tax_bps, l.price_overridden
             FROM sales_v_lines l
             WHERE l.status = 'completed' AND l.business_date BETWEEN '2026-01-01' AND '2026-12-31'",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(3))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(
        plan.iter()
            .any(|step| step.contains("COVERING INDEX sales_lines_report")),
        "{plan:?}"
    );
}

#[test]
fn cat_08_a_product_in_a_cart_or_sale_is_used() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let sold = Id::new();
    let in_cart = Id::new();
    sell(&mut conn, &till, &[(&sold, 1000)]);
    support::cart_with(&mut conn, &till, &[(&in_cart, 1000)]);

    assert!(sales::is_product_used(&conn, &sold).unwrap());
    assert!(sales::is_product_used(&conn, &in_cart).unwrap());
    assert!(!sales::is_product_used(&conn, &Id::new()).unwrap());
}
