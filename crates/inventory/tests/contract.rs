//! Contract tests: the migrations, published views and enum texts are what the
//! module page says. Other modules (reports) read these views, so their columns
//! must not change without a new migration (Module Rules R5).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use inventory::{CountScope, CountStatus, MoveKind, ReasonDirection};
use pos_core::Id;
use rusqlite::Connection;
use support::{batch, pieces, receive, receive_into, test_db};

/// The column names of a table or view, in order.
fn columns(connection: &Connection, name: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("SELECT name FROM pragma_table_info('{name}')"))
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn migrations_belong_to_this_module_and_are_numbered_from_one() {
    let migrations = inventory::migrations();
    assert_eq!(migrations.module, "inventory");
    let versions: Vec<u32> = migrations.migrations.iter().map(|m| m.version).collect();
    let expected: Vec<u32> = (1..=u32::try_from(versions.len()).unwrap()).collect();
    assert_eq!(versions, expected);
}

#[test]
fn every_table_and_view_has_the_module_prefix() {
    let connection = test_db();
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table', 'view', 'index', 'trigger') AND name NOT LIKE 'sqlite_%'")
        .unwrap();
    let names: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!names.is_empty());
    for name in names {
        assert!(
            name.starts_with("inventory_"),
            "{name} has no inventory_ prefix"
        );
    }
}

#[test]
fn view_inventory_v_levels_has_its_published_columns() {
    let connection = test_db();
    assert_eq!(
        columns(&connection, "inventory_v_levels"),
        vec![
            "product_id",
            "on_hand_milli",
            "reorder_level_milli",
            "reorder_qty_milli"
        ]
    );
}

#[test]
fn view_inventory_v_levels_includes_products_with_only_a_reorder_rule() {
    let mut connection = test_db();
    let stocked = Id::new();
    let rule_only = Id::new();
    receive(&mut connection, &stocked, 2);
    let transaction = connection.transaction().unwrap();
    inventory::set_reorder_rule(&transaction, &rule_only, pieces(5), pieces(10)).unwrap();
    transaction.commit().unwrap();

    let mut statement = connection
        .prepare("SELECT product_id, on_hand_milli, reorder_level_milli FROM inventory_v_levels ORDER BY on_hand_milli")
        .unwrap();
    let rows: Vec<(String, i64, Option<i64>)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        rows,
        vec![
            (rule_only.to_string(), 0, Some(5_000)),
            (stocked.to_string(), 2_000, None),
        ]
    );
}

#[test]
fn view_inventory_v_ledger_has_every_ledger_column() {
    let connection = test_db();
    assert_eq!(
        columns(&connection, "inventory_v_ledger"),
        vec![
            "id",
            "product_id",
            "batch_id",
            "qty_delta_milli",
            "kind",
            "reason_code",
            "note",
            "ref_type",
            "ref_id",
            "unit_cost_minor",
            "at",
            "business_date",
            "till_id",
            "user_id"
        ]
    );
}

#[test]
fn view_inventory_v_batches_shows_only_batches_with_stock() {
    let mut connection = test_db();
    let product = Id::new();
    let full = batch(&mut connection, &product, Some("2026-05-01"));
    let _empty = batch(&mut connection, &product, Some("2026-06-01"));
    receive_into(&mut connection, &product, &full, 1);

    assert_eq!(
        columns(&connection, "inventory_v_batches"),
        vec![
            "id",
            "product_id",
            "batch_no",
            "expiry_date",
            "unit_cost_minor",
            "received_at",
            "on_hand_milli"
        ]
    );
    let ids: Vec<String> = connection
        .prepare("SELECT id FROM inventory_v_batches")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(ids, vec![full.to_string()]);
}

#[test]
fn kinds_are_stored_as_the_page_names_them() {
    let all = [
        (MoveKind::Sale, "sale"),
        (MoveKind::SaleVoid, "sale_void"),
        (MoveKind::Return, "return"),
        (MoveKind::Receive, "receive"),
        (MoveKind::SupplierReturn, "supplier_return"),
        (MoveKind::Adjust, "adjust"),
        (MoveKind::Count, "count"),
        (MoveKind::Opening, "opening"),
    ];
    for (kind, text) in all {
        assert_eq!(kind.as_str(), text);
        assert_eq!(MoveKind::parse(text).unwrap(), kind);
    }
    assert!(MoveKind::parse("gift").is_err());
}

#[test]
fn other_enums_are_stored_as_lower_snake_case_text() {
    for direction in [
        ReasonDirection::In,
        ReasonDirection::Out,
        ReasonDirection::Both,
    ] {
        assert_eq!(
            ReasonDirection::parse(direction.as_str()).unwrap(),
            direction
        );
    }
    for scope in [CountScope::Full, CountScope::Partial] {
        assert_eq!(CountScope::parse(scope.as_str()).unwrap(), scope);
    }
    for status in [
        CountStatus::Open,
        CountStatus::Posted,
        CountStatus::Cancelled,
    ] {
        assert_eq!(CountStatus::parse(status.as_str()).unwrap(), status);
    }
    assert_eq!(ReasonDirection::Both.as_str(), "both");
    assert_eq!(CountScope::Partial.as_str(), "partial");
    assert_eq!(CountStatus::Cancelled.as_str(), "cancelled");
    assert!(CountStatus::parse("closed").is_err());
}

#[test]
fn kinds_are_sent_to_the_ui_as_snake_case() {
    assert_eq!(
        serde_json::to_string(&MoveKind::SupplierReturn).unwrap(),
        r#""supplier_return""#
    );
}
