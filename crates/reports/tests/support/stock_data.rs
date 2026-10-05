//! Catalog and stock of the fixture shop.

use rusqlite::{Connection, named_params};

use super::id;

const PIECE: &str = "00000000000000000000000001";
const KG: &str = "00000000000000000000000002";

pub fn add_catalog(conn: &Connection) {
    conn.execute_batch(&format!(
        "INSERT INTO catalog_categories (id, name) VALUES
           ('{dairy}', 'Dairy'), ('{grocery}', 'Grocery'), ('{household}', 'Household');
         INSERT INTO catalog_brands (id, name) VALUES ('{aarong}', 'Aarong'), ('{lux}', 'Lux');",
        dairy = id("CD"),
        grocery = id("CG"),
        household = id("CH"),
        aarong = id("BA"),
        lux = id("BX"),
    ))
    .unwrap();
    add_product(
        conn,
        "P1",
        "Milk 1L",
        "CD",
        Some("BA"),
        PIECE,
        10_000,
        8_000,
        "TR15",
    );
    add_product(conn, "P2", "Rice", "CG", None, KG, 5_000, 4_000, "TR0");
    add_product(
        conn,
        "P3",
        "Soap",
        "CH",
        Some("BX"),
        PIECE,
        4_000,
        2_500,
        "TR15",
    );
    add_product(
        conn, "P4", "Oil 1L", "CG", None, PIECE, 20_000, 18_000, "TR0",
    );
}

#[allow(clippy::too_many_arguments)]
fn add_product(
    conn: &Connection,
    tag: &str,
    name: &str,
    category: &str,
    brand: Option<&str>,
    unit: &str,
    price: i64,
    cost: i64,
    rate: &str,
) {
    conn.execute(
        "INSERT INTO catalog_products (id, kind, name, sku, category_id, brand_id, unit_id,
           sale_price_minor, cost_price_minor, tax_rate_id, created_at, updated_at)
         VALUES (:id, 'standard', :name, :sku, :category, :brand, :unit, :price, :cost, :rate, 0, 0)",
        named_params! {
            ":id": id(tag).as_str(), ":name": name, ":sku": tag, ":category": id(category).as_str(),
            ":brand": brand.map(|brand| id(brand).to_string()), ":unit": unit,
            ":price": price, ":cost": cost, ":rate": id(rate).as_str(),
        },
    )
    .unwrap();
}

fn add_ledger_row(conn: &Connection, product: &str, kind: &str, qty: i64, date: &str) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM inventory_ledger", [], |row| {
            row.get(0)
        })
        .unwrap();
    conn.execute(
        "INSERT INTO inventory_ledger (id, product_id, qty_delta_milli, kind, at, business_date, till_id)
         VALUES (:id, :product, :qty, :kind, 0, :date, :till)",
        named_params! {
            ":id": id(&format!("NV{count}")).as_str(), ":product": id(product).as_str(),
            ":qty": qty, ":kind": kind, ":date": date, ":till": id("T1").as_str(),
        },
    )
    .unwrap();
}

/// Stock now: Milk 20, Rice 5.5 kg, Soap 50, Oil only has a reorder rule.
/// Milk's ledger: opening 15 (03-01), received 10 (03-04), then on 03-05 sold 3,
/// 1 returned and 3 written off as damaged.
pub fn add_stock(conn: &Connection) {
    conn.execute_batch(&format!(
        "INSERT INTO inventory_levels (product_id, on_hand_milli, updated_at) VALUES
           ('{milk}', 20000, 0), ('{rice}', 5500, 0), ('{soap}', 50000, 0);
         INSERT INTO inventory_reorder_rules (product_id, reorder_level_milli, reorder_qty_milli) VALUES
           ('{milk}', 25000, 30000), ('{rice}', 10000, 0), ('{soap}', 10000, 24000), ('{oil}', 5000, 12000);
         INSERT INTO inventory_batches (id, product_id, batch_no, expiry_date, unit_cost_minor, received_at, on_hand_milli) VALUES
           ('{b1}', '{milk}', 'M-07', '2026-03-07', 8000, 0, 12000),
           ('{b2}', '{milk}', 'M-30', '2026-04-30', 8000, 0, 8000),
           ('{b3}', '{soap}', 'S-01', '2026-03-01', 2500, 0, 3000),
           ('{b4}', '{soap}', NULL, NULL, 2500, 0, 47000);",
        milk = id("P1"),
        rice = id("P2"),
        soap = id("P3"),
        oil = id("P4"),
        b1 = id("B1"),
        b2 = id("B2"),
        b3 = id("B3"),
        b4 = id("B4"),
    ))
    .unwrap();
    add_ledger_row(conn, "P1", "opening", 15_000, "2026-03-01");
    add_ledger_row(conn, "P1", "receive", 10_000, "2026-03-04");
    add_ledger_row(conn, "P1", "sale", -2_000, "2026-03-05");
    add_ledger_row(conn, "P1", "sale", -1_000, "2026-03-05");
    add_ledger_row(conn, "P1", "return", 1_000, "2026-03-05");
    add_ledger_row(conn, "P1", "adjust", -3_000, "2026-03-05");
    add_ledger_row(conn, "P2", "opening", 9_500, "2026-03-01");
    add_ledger_row(conn, "P2", "sale", -4_000, "2026-03-05");
}
