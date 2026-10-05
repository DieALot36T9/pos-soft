//! Contract tests: the migrations, views and errors behave as the module page says.
//! The public functions are covered by `suppliers.rs`, `orders.rs`, `receiving.rs`
//! and `payables.rs`.
// Helpers outside #[test] functions may unwrap too: a failing test should stop right there.
#![allow(clippy::unwrap_used)]

mod common;

use common::{add_supplier, ctx, direct_grn, grn_line, test_db};
use pos_core::{AppError, Id};
use purchasing::PurchasingError;
use rusqlite::Connection;

/// The column names of a table or view, in order.
fn columns(connection: &Connection, name: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("SELECT name FROM pragma_table_info('{name}')"))
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|name| name.unwrap())
        .collect()
}

#[test]
fn r4_migrations_belong_to_this_module_and_count_up_from_one() {
    let migrations = purchasing::migrations();
    assert_eq!(migrations.module, "purchasing");
    for (index, migration) in migrations.migrations.iter().enumerate() {
        assert_eq!(migration.version as usize, index + 1);
    }
}

#[test]
fn r2_every_table_and_view_has_the_module_prefix() {
    let connection = test_db();
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','view','index','trigger') AND name NOT LIKE 'sqlite_%'")
        .unwrap();
    let names: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|name| name.unwrap())
        .collect();
    assert!(!names.is_empty());
    for name in names {
        assert!(name.starts_with("purchasing_"), "{name} has no prefix");
    }
}

#[test]
fn r2_payables_view_has_the_published_columns() {
    let connection = test_db();
    assert_eq!(
        columns(&connection, "purchasing_v_payables"),
        ["supplier_id", "name", "payment_terms_days", "balance_minor"]
    );
}

#[test]
fn pur_05_payables_view_shows_zero_for_a_supplier_with_no_ledger_rows() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 7);

    let (terms, balance): (i64, i64) = tx
        .query_row(
            "SELECT payment_terms_days, balance_minor FROM purchasing_v_payables WHERE supplier_id = ?1",
            [supplier.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((terms, balance), (7, 0));
}

#[test]
fn r2_ledger_view_has_the_published_columns() {
    let connection = test_db();
    assert_eq!(
        columns(&connection, "purchasing_v_ledger"),
        [
            "id",
            "supplier_id",
            "amount_minor",
            "kind",
            "method_kind",
            "reference",
            "ref_type",
            "ref_id",
            "due_date",
            "at",
            "business_date",
            "user_id"
        ]
    );
}

#[test]
fn r2_grn_lines_view_has_the_published_columns() {
    let connection = test_db();
    assert_eq!(
        columns(&connection, "purchasing_v_grn_lines"),
        [
            "id",
            "grn_id",
            "order_line_id",
            "product_id",
            "product_name",
            "qty_milli",
            "unit_cost_minor",
            "batch_no",
            "expiry_date",
            "inventory_batch_id",
            "number",
            "supplier_id",
            "received_at",
            "business_date"
        ]
    );
}

#[test]
fn pur_03_grn_lines_view_lists_each_received_line_with_its_grn() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let grn = direct_grn(
        &supplier,
        vec![grn_line("Milk", 10, 5000), grn_line("Eggs", 30, 1200)],
        86_000,
    );
    purchasing::record_grn(&tx, &grn, &ctx("2026-01-10")).unwrap();

    let (count, total_qty, business_date): (i64, i64, String) = tx
        .query_row(
            "SELECT COUNT(*), SUM(qty_milli), MAX(business_date) FROM purchasing_v_grn_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        (count, total_qty, business_date.as_str()),
        (2, 40_000, "2026-01-10")
    );
}

#[test]
fn r3_errors_turn_into_app_errors_with_codes() {
    let invalid: AppError = PurchasingError::Invalid {
        field: "name",
        code: "supplier_name_missing",
    }
    .into();
    assert!(matches!(
        invalid,
        AppError::Validation { field: Some(ref field), ref code, .. }
            if field == "name" && code == "supplier_name_missing"
    ));

    let missing: AppError = PurchasingError::NotFound {
        entity: "supplier",
        key: "X".to_string(),
    }
    .into();
    assert!(matches!(missing, AppError::NotFound { ref entity, .. } if entity == "supplier"));

    let conflict: AppError = PurchasingError::Conflict {
        code: "order_closed",
    }
    .into();
    assert!(matches!(conflict, AppError::Conflict { ref code, .. } if code == "order_closed"));

    let database: AppError = PurchasingError::Database(rusqlite::Error::InvalidQuery).into();
    assert!(matches!(database, AppError::Internal { .. }));
}

#[test]
fn pur_05_balance_of_an_unknown_supplier_is_not_found() {
    let connection = test_db();
    assert!(matches!(
        purchasing::balance(&connection, &Id::new()),
        Err(PurchasingError::NotFound { .. })
    ));
}
