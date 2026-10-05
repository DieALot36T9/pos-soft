//! Supplier and customer ledgers of the fixture shop.

use rusqlite::{Connection, named_params};

use super::id;

fn add_supplier(conn: &Connection, tag: &str, name: &str) {
    conn.execute(
        "INSERT INTO purchasing_suppliers (id, name, created_at, updated_at) VALUES (:id, :name, 0, 0)",
        named_params! { ":id": id(tag).as_str(), ":name": name },
    )
    .unwrap();
}

fn add_supplier_row(
    conn: &Connection,
    supplier: &str,
    kind: &str,
    amount: i64,
    due: Option<&str>,
    date: &str,
) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM purchasing_ledger", [], |row| {
            row.get(0)
        })
        .unwrap();
    conn.execute(
        "INSERT INTO purchasing_ledger (id, supplier_id, amount_minor, kind, due_date, at, business_date, user_id)
         VALUES (:id, :supplier, :amount, :kind, :due, 0, :date, :user)",
        named_params! {
            ":id": id(&format!("PR{count}")).as_str(), ":supplier": id(supplier).as_str(),
            ":amount": amount, ":kind": kind, ":due": due, ":date": date, ":user": id("A").as_str(),
        },
    )
    .unwrap();
}

/// Fresh Dairy: 5,000 (due 01-20) + 3,000 (due 02-25) + 2,000 (due 03-10), paid 6,000.
/// Soap Co: 1,000 (due 2025-11-01), paid 1,500, so the shop is owed 500.
/// Rice Mill: 700 due 2025-12-20. Fresh Dairy last delivered Milk.
pub fn add_payables(conn: &Connection) {
    add_supplier(conn, "SP1", "Fresh Dairy");
    add_supplier(conn, "SP2", "Soap Co");
    add_supplier(conn, "SP3", "Rice Mill");
    add_supplier_row(
        conn,
        "SP1",
        "invoice",
        500_000,
        Some("2026-01-20"),
        "2026-01-10",
    );
    add_supplier_row(
        conn,
        "SP1",
        "invoice",
        300_000,
        Some("2026-02-25"),
        "2026-02-15",
    );
    add_supplier_row(
        conn,
        "SP1",
        "invoice",
        200_000,
        Some("2026-03-10"),
        "2026-03-01",
    );
    add_supplier_row(conn, "SP1", "payment", -600_000, None, "2026-03-02");
    add_supplier_row(
        conn,
        "SP2",
        "invoice",
        100_000,
        Some("2025-11-01"),
        "2025-10-01",
    );
    add_supplier_row(conn, "SP2", "payment", -150_000, None, "2025-11-05");
    add_supplier_row(
        conn,
        "SP3",
        "invoice",
        70_000,
        Some("2025-12-20"),
        "2025-12-01",
    );
    conn.execute_batch(&format!(
        "INSERT INTO purchasing_grns (id, number, supplier_id, invoice_total_minor, lines_total_minor,
           received_at, business_date, user_id)
         VALUES ('{grn}', 'GRN-000001', '{supplier}', 80000, 80000, 0, '2026-03-04', '{admin}');
         INSERT INTO purchasing_grn_lines (id, grn_id, product_id, product_name, qty_milli, unit_cost_minor)
         VALUES ('{line}', '{grn}', '{milk}', 'Milk 1L', 10000, 8000);",
        grn = id("GRN1"),
        supplier = id("SP1"),
        admin = id("A"),
        line = id("GRN1A"),
        milk = id("P1"),
    ))
    .unwrap();
}

fn add_customer(conn: &Connection, tag: &str, name: &str, phone: &str) {
    conn.execute(
        "INSERT INTO customers_customers (id, phone, name, created_at, updated_at)
         VALUES (:id, :phone, :name, 0, 0)",
        named_params! { ":id": id(tag).as_str(), ":phone": phone, ":name": name },
    )
    .unwrap();
}

fn add_credit_row(conn: &Connection, customer: &str, kind: &str, amount: i64, date: &str) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM customers_credit_ledger", [], |row| {
            row.get(0)
        })
        .unwrap();
    conn.execute(
        "INSERT INTO customers_credit_ledger (id, customer_id, amount_minor, kind, at, business_date)
         VALUES (:id, :customer, :amount, :kind, 0, :date)",
        named_params! {
            ":id": id(&format!("CC{count}")).as_str(), ":customer": id(customer).as_str(),
            ":amount": amount, ":kind": kind, ":date": date,
        },
    )
    .unwrap();
}

/// Nadia: owes 500 from 01-10 and 300 from 03-01, paid 400. Omar: owes 250 from today.
/// Zara: bought on credit and paid it all back.
pub fn add_receivables(conn: &Connection) {
    add_customer(conn, "CN", "Nadia", "+8801711000001");
    add_customer(conn, "CM", "Omar", "+8801711000002");
    add_customer(conn, "CZ", "Zara", "+8801711000003");
    add_credit_row(conn, "CN", "credit_sale", 50_000, "2026-01-10");
    add_credit_row(conn, "CN", "credit_sale", 30_000, "2026-03-01");
    add_credit_row(conn, "CN", "payment", -40_000, "2026-03-02");
    add_credit_row(conn, "CM", "credit_sale", 25_000, "2026-03-05");
    add_credit_row(conn, "CZ", "credit_sale", 10_000, "2026-02-01");
    add_credit_row(conn, "CZ", "payment", -10_000, "2026-02-03");
}
