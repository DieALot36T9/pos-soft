//! Helpers shared by the purchasing tests.
#![allow(dead_code, clippy::unwrap_used)]

use chrono::NaiveDate;
use pos_core::{BusinessDate, Id, Money, Qty, Timestamp};
use purchasing::{
    GrnRef, NewGrn, NewGrnLine, NewOrder, NewOrderLine, PurchasingCtx, SupplierInput,
};
use rusqlite::{Connection, Transaction};

/// An in-memory database built by this module's own migrations (Module Rules R6).
pub fn test_db() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    for migration in purchasing::migrations().migrations {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
}

pub fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
}

/// Who and when, on the given business date.
pub fn ctx(day: &str) -> PurchasingCtx {
    let business_date = date(day);
    let at = business_date.and_hms_opt(10, 0, 0).unwrap().and_utc();
    PurchasingCtx {
        at: Timestamp(at.timestamp_millis()),
        business_date: BusinessDate(business_date),
        user_id: Id::new(),
    }
}

pub fn money(minor: i64) -> Money {
    Money(minor)
}

/// Whole units: `qty(5)` is 5 pieces.
pub fn qty(units: i64) -> Qty {
    Qty(units * 1000)
}

pub fn supplier_named(name: &str, terms_days: u32) -> SupplierInput {
    SupplierInput {
        name: name.to_string(),
        payment_terms_days: terms_days,
        ..SupplierInput::default()
    }
}

pub fn add_supplier(tx: &Transaction, name: &str, terms_days: u32) -> Id {
    purchasing::create_supplier(tx, &supplier_named(name, terms_days), Timestamp(1)).unwrap()
}

pub fn order_line(name: &str, units: i64, unit_cost_minor: i64) -> NewOrderLine {
    NewOrderLine {
        product_id: Id::new(),
        product_name: name.to_string(),
        sku: format!("SKU-{name}"),
        qty: qty(units),
        unit_cost: money(unit_cost_minor),
    }
}

pub fn order_for(supplier: &Id, lines: Vec<NewOrderLine>) -> NewOrder {
    NewOrder {
        supplier_id: supplier.clone(),
        expected_date: None,
        notes: None,
        lines,
    }
}

pub fn grn_line(name: &str, units: i64, unit_cost_minor: i64) -> NewGrnLine {
    NewGrnLine {
        order_line_id: None,
        product_id: Id::new(),
        product_name: name.to_string(),
        qty: qty(units),
        unit_cost: money(unit_cost_minor),
        batch_no: None,
        expiry_date: None,
        inventory_batch_id: None,
    }
}

/// A GRN without a purchase order, with the invoice total equal to the line total.
pub fn direct_grn(supplier: &Id, lines: Vec<NewGrnLine>, invoice_total_minor: i64) -> NewGrn {
    NewGrn {
        supplier_id: supplier.clone(),
        order_id: None,
        supplier_invoice_no: Some("INV-1".to_string()),
        invoice_total: money(invoice_total_minor),
        notes: None,
        lines,
    }
}

/// Receives `units` of each listed order line in one GRN against the order.
pub fn receive(tx: &Transaction, order: &Id, units_per_line: &[(Id, i64)]) -> GrnRef {
    let saved = purchasing::get_order(tx, order).unwrap();
    let mut lines = Vec::new();
    for (order_line_id, units) in units_per_line {
        let mut line = grn_line("item", *units, 1000);
        line.order_line_id = Some(order_line_id.clone());
        lines.push(line);
    }
    let grn = NewGrn {
        supplier_id: saved.supplier_id,
        order_id: Some(order.clone()),
        supplier_invoice_no: None,
        invoice_total: Money(0),
        notes: None,
        lines,
    };
    purchasing::record_grn(tx, &grn, &ctx("2026-01-12")).unwrap()
}
