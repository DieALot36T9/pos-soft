//! A reference shop for the report benchmark (NFR-P6, NFR-P7, RPT-11).
//!
//! Sizes follow "Reference data" in `docs/02 Requirements/Non-Functional Requirements.md`:
//! 50,000 products, 5,000 customers and 1,500 sales a day, here for one year
//! (2025-01-01 to 2025-12-31, about 1.6 million sale lines). The rows are written
//! straight into each module's tables with plain SQL, because building a year through
//! the modules' functions would take far longer than the benchmark itself.
//! Ids the reports take as `Id` (shifts, suppliers, customers) are written as ULID text:
//! [`shift_id`], [`supplier_id`] and [`customer_id`] give them.

// Benchmarks may stop at the first failure, like tests.
#![allow(clippy::unwrap_used)]

use rusqlite::Connection;

/// Sales a day, and days of data.
pub const SALES_PER_DAY: i64 = 1_500;
pub const DAYS: i64 = 365;
pub const PRODUCTS: i64 = 50_000;
pub const CUSTOMERS: i64 = 5_000;
pub const SUPPLIERS: i64 = 200;

/// A database with every module's views and one year of reference data.
pub fn build(path: &std::path::Path) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;")
        .unwrap();
    let modules = [
        audit::migrations(),
        catalog::migrations(),
        customers::migrations(),
        inventory::migrations(),
        purchasing::migrations(),
        sales::migrations(),
        shifts::migrations(),
    ];
    for module in modules {
        for migration in module.migrations {
            conn.execute_batch(migration.sql).unwrap();
        }
    }
    // The rows are written in bulk, so parents may come after their children.
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    for step in [CATALOG, SALES, RETURNS, SHIFTS, AUDIT, STOCK, OWED] {
        conn.execute_batch(&format!("BEGIN; {} COMMIT;", fill(step)))
            .unwrap();
    }
    conn.execute_batch("ANALYZE; PRAGMA foreign_keys = ON;")
        .unwrap();
    conn
}

/// The id of the shift on `till` (0–2) on day `day` (0 = 2025-01-01).
pub fn shift_id(day: i64, till: i64) -> String {
    format!("{:024}SH", day * 3 + till)
}

/// The id of supplier `number` (1–200).
pub fn supplier_id(number: i64) -> String {
    format!("{number:024}SP")
}

/// The id of customer `number` (1–5,000).
pub fn customer_id(number: i64) -> String {
    format!("{number:024}CS")
}

/// Puts the sizes into the SQL.
fn fill(sql: &str) -> String {
    sql.replace("{SALES}", &(SALES_PER_DAY * DAYS).to_string())
        .replace("{PER_DAY}", &SALES_PER_DAY.to_string())
        .replace("{DAYS}", &DAYS.to_string())
        .replace("{PRODUCTS}", &PRODUCTS.to_string())
        .replace("{CUSTOMERS}", &CUSTOMERS.to_string())
        .replace("{SUPPLIERS}", &SUPPLIERS.to_string())
}

/// 50 categories, 200 brands, 50,000 products. A third are tax exempt.
const CATALOG: &str = "
INSERT INTO catalog_categories (id, name)
  WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 50)
  SELECT 'C' || i, 'Category ' || i FROM n;
INSERT INTO catalog_brands (id, name)
  WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 200)
  SELECT 'B' || i, 'Brand ' || i FROM n;
INSERT INTO catalog_products (id, kind, name, sku, category_id, brand_id, unit_id,
    sale_price_minor, cost_price_minor, tax_rate_id, created_at, updated_at)
  WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < {PRODUCTS})
  SELECT 'P' || i, 'standard', 'Product ' || i, 'SKU' || i, 'C' || (i % 50 + 1),
         CASE WHEN i % 4 = 0 THEN NULL ELSE 'B' || (i % 200 + 1) END,
         '00000000000000000000000001', 1000 + (i % 50) * 500, 800 + (i % 50) * 400,
         CASE WHEN i % 3 = 0 THEN 'TR0' ELSE 'TR15' END, 0, 0
  FROM n;
";

/// 1,500 sales a day on 3 tills by 5 cashiers, 3 lines each. Every 20th line has a
/// manual discount, every 250th sale is voided. Prices include tax.
const SALES: &str = "
CREATE TEMP TABLE bench_lines AS
  WITH RECURSIVE n(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM n WHERE i < {SALES} - 1),
  seqs(seq) AS (VALUES (1), (2), (3)),
  picked AS (
    SELECT i AS sale, seq, (i * 7919 + seq * 104729) % {PRODUCTS} + 1 AS product FROM n, seqs
  ),
  priced AS (
    SELECT sale, seq, product, 1000 + (product % 50) * 500 AS base,
           CASE WHEN (sale * 3 + seq) % 20 = 0 THEN (1000 + (product % 50) * 500) / 10 ELSE 0 END AS manual,
           CASE WHEN product % 3 = 0 THEN 0 ELSE 1500 END AS bps
    FROM picked
  )
  SELECT sale, seq, product, base, manual, bps, base - manual AS gross,
         ((base - manual) * bps + (10000 + bps) / 2) / (10000 + bps) AS tax
  FROM priced;
INSERT INTO sales_lines (id, sale_id, seq, product_id, name, sku, unit_symbol, category_id,
    brand_id, qty_milli, unit_price_minor, retail_price_minor, base_minor, promo_discount_minor,
    tier_discount_minor, manual_discount_minor, invoice_discount_minor, net_minor, tax_minor,
    gross_minor, tax_rate_id, tax_name, tax_bps, tax_components_json, unit_cost_minor,
    cost_minor, price_overridden)
  SELECT 'L' || sale || '-' || seq, 'S' || sale, seq, 'P' || product, 'Product ' || product,
         'SKU' || product, 'pc', 'C' || (product % 50 + 1),
         CASE WHEN product % 4 = 0 THEN NULL ELSE 'B' || (product % 200 + 1) END,
         1000, base, base, base, 0, 0, manual, 0, gross - tax, tax, gross,
         CASE WHEN bps = 0 THEN 'TR0' ELSE 'TR15' END,
         CASE WHEN bps = 0 THEN 'Exempt 0%' ELSE 'Standard 15%' END, bps,
         CASE WHEN bps = 0 THEN '[]' ELSE json_array(
           json_object('name', 'CGST', 'bps', 750, 'tax_minor', tax / 2),
           json_object('name', 'SGST', 'bps', 750, 'tax_minor', tax - tax / 2)) END,
         800 + (product % 50) * 400, 800 + (product % 50) * 400, (sale + seq) % 500 = 0
  FROM bench_lines;
INSERT INTO sales_sales (id, invoice_no, till_id, shift_id, user_id, business_date, completed_at,
    status, base_minor, promo_discount_minor, tier_discount_minor, manual_discount_minor,
    invoice_discount_minor, net_minor, tax_minor, gross_minor, rounding_minor, total_minor,
    change_minor, cost_minor, tax_mode, void_reason)
  SELECT 'S' || sale, 'T' || (sale % 3) || '-' || sale, 'T' || (sale % 3),
         printf('%024dSH', (sale / {PER_DAY}) * 3 + sale % 3), 'U' || (sale % 5),
         date('2025-01-01', '+' || (sale / {PER_DAY}) || ' days'),
         1735668000000 + (sale / {PER_DAY}) * 86400000 + (sale % {PER_DAY}) * 40000,
         CASE WHEN sale % 250 = 0 THEN 'voided' ELSE 'completed' END,
         SUM(base), 0, 0, SUM(manual), 0, SUM(gross - tax), SUM(tax), SUM(gross), 0, SUM(gross),
         0, SUM(800 + (product % 50) * 400), 'inclusive',
         CASE WHEN sale % 250 = 0 THEN 'Wrong item' END
  FROM bench_lines GROUP BY sale;
INSERT INTO sales_payments (id, doc_type, doc_id, method_id, kind, amount_minor, at)
  SELECT 'PY' || id, 'sale', id, '00000000000000000000000001', 'cash', total_minor, completed_at
  FROM sales_sales;
";

/// One return in every 100 sales: the first line, the same day, one hour later.
const RETURNS: &str = "
INSERT INTO sales_returns (id, return_no, sale_id, till_id, shift_id, user_id, business_date,
    at, net_minor, tax_minor, gross_minor, rounding_minor, total_minor, cost_minor)
  SELECT 'R' || s.id, s.invoice_no || 'R', s.id, s.till_id, s.shift_id, s.user_id,
         s.business_date, s.completed_at + 3600000, l.net_minor, l.tax_minor, l.gross_minor, 0,
         l.gross_minor, l.cost_minor
  FROM sales_sales s JOIN sales_lines l ON l.sale_id = s.id AND l.seq = 1
  WHERE CAST(substr(s.id, 2) AS INTEGER) % 100 = 50;
INSERT INTO sales_return_lines (id, return_id, sale_line_id, product_id, name, qty_milli,
    net_minor, tax_minor, gross_minor, tax_rate_id, tax_bps, unit_cost_minor, cost_minor, restock)
  SELECT 'RL' || r.id, r.id, l.id, l.product_id, l.name, 1000, l.net_minor, l.tax_minor,
         l.gross_minor, l.tax_rate_id, l.tax_bps, l.unit_cost_minor, l.cost_minor, 1
  FROM sales_returns r JOIN sales_lines l ON l.sale_id = r.sale_id AND l.seq = 1;
INSERT INTO sales_payments (id, doc_type, doc_id, method_id, kind, amount_minor, at)
  SELECT 'PY' || id, 'return', id, '00000000000000000000000001', 'cash', total_minor, at
  FROM sales_returns;
";

/// A closed shift a day on each till, with a float, one movement per sale and counts.
const SHIFTS: &str = "
INSERT INTO shifts_shifts (id, till_id, user_id, business_date, status, opened_at,
    opening_float_minor, closed_at, closed_by, expected_cash_minor, counted_cash_minor,
    difference_minor)
  WITH RECURSIVE d(day) AS (SELECT 0 UNION ALL SELECT day + 1 FROM d WHERE day < {DAYS} - 1),
  t(till) AS (VALUES (0), (1), (2))
  SELECT printf('%024dSH', day * 3 + till), 'T' || till, 'U' || till,
         date('2025-01-01', '+' || day || ' days'), 'closed',
         1735668000000 + day * 86400000 - 3600000, 100000,
         1735668000000 + day * 86400000 + 72000000, 'U' || till, 0, 0, 0
  FROM d, t;
INSERT INTO shifts_movements (id, shift_id, kind, method_id, method_kind, amount_minor,
    ref_type, ref_id, at, user_id)
  SELECT 'MV' || id, shift_id, 'sale_payment', '00000000000000000000000001', 'cash',
         total_minor, 'sale', id, completed_at, user_id
  FROM sales_sales;
INSERT INTO shifts_movements (id, shift_id, kind, method_kind, amount_minor, at, user_id)
  SELECT 'MF' || id, id, 'float', 'cash', opening_float_minor, opened_at, user_id FROM shifts_shifts;
INSERT INTO shifts_counts (shift_id, stage, denomination_minor, count)
  SELECT id, 'close', 100000, 10 FROM shifts_shifts;
";

/// Audit events: every void, a cancelled cart every 150 sales, a no-sale every 300.
const AUDIT: &str = "
INSERT INTO audit_events (id, at, business_date, till_id, user_id, approved_by, action)
  SELECT 'EV' || id, completed_at, business_date, till_id, user_id,
         CASE WHEN status = 'voided' THEN 'ADMIN' END,
         CASE WHEN status = 'voided' THEN 'sale.void'
              WHEN CAST(substr(id, 2) AS INTEGER) % 150 = 1 THEN 'cart.cancel'
              ELSE 'drawer.no_sale' END
  FROM sales_sales
  WHERE status = 'voided' OR CAST(substr(id, 2) AS INTEGER) % 150 = 1
     OR CAST(substr(id, 2) AS INTEGER) % 300 = 2;
";

/// Stock: an opening row and a level for every product, one ledger row per sale line,
/// reorder rules for every tenth product and 10,000 batches with expiry dates.
const STOCK: &str = "
INSERT INTO inventory_ledger (id, product_id, qty_delta_milli, kind, at, business_date, till_id)
  SELECT 'O' || id, id, 100000, 'opening', 0, '2024-12-31', 'T0' FROM catalog_products;
INSERT INTO inventory_ledger (id, product_id, qty_delta_milli, kind, ref_type, ref_id, at,
    business_date, till_id)
  SELECT 'G' || l.id, l.product_id, -l.qty_milli, 'sale', 'sale', l.sale_id, l.completed_at,
         l.business_date, l.till_id
  FROM sales_v_lines l;
INSERT INTO inventory_levels (product_id, on_hand_milli, updated_at)
  SELECT product_id, SUM(qty_delta_milli), 0 FROM inventory_ledger GROUP BY product_id;
INSERT INTO inventory_reorder_rules (product_id, reorder_level_milli, reorder_qty_milli)
  SELECT id, 80000, CASE WHEN rowid % 20 = 0 THEN 0 ELSE 24000 END
  FROM catalog_products WHERE rowid % 10 = 0;
INSERT INTO inventory_batches (id, product_id, batch_no, expiry_date, unit_cost_minor,
    received_at, on_hand_milli)
  SELECT 'BT' || rowid, id, 'B' || rowid, date('2025-11-01', '+' || (rowid % 120) || ' days'),
         cost_price_minor, 0, 5000
  FROM catalog_products WHERE rowid % 5 = 0;
";

/// 200 suppliers with an invoice and a payment each week; 5,000 customers, a fifth of
/// them buying on credit each month and paying back half.
const OWED: &str = "
INSERT INTO purchasing_suppliers (id, name, created_at, updated_at)
  WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < {SUPPLIERS})
  SELECT printf('%024dSP', i), 'Supplier ' || i, 0, 0 FROM n;
INSERT INTO purchasing_ledger (id, supplier_id, amount_minor, kind, due_date, at, business_date, user_id)
  WITH RECURSIVE w(week) AS (SELECT 0 UNION ALL SELECT week + 1 FROM w WHERE week < 51)
  SELECT 'PI' || s.id || '-' || week, s.id, 500000, 'invoice',
         date('2025-01-01', '+' || (week * 7 + 30) || ' days'), 0,
         date('2025-01-01', '+' || (week * 7) || ' days'), 'U0'
  FROM purchasing_suppliers s, w;
INSERT INTO purchasing_ledger (id, supplier_id, amount_minor, kind, at, business_date, user_id)
  WITH RECURSIVE w(week) AS (SELECT 0 UNION ALL SELECT week + 1 FROM w WHERE week < 51)
  SELECT 'PP' || s.id || '-' || week, s.id, -450000, 'payment', 0,
         date('2025-01-01', '+' || (week * 7 + 3) || ' days'), 'U0'
  FROM purchasing_suppliers s, w;
INSERT INTO purchasing_grns (id, number, supplier_id, invoice_total_minor, lines_total_minor,
    received_at, business_date, user_id)
  SELECT 'GRN' || id, 'GRN-' || id, id, 500000, 500000, 0, '2025-06-01', 'U0'
  FROM purchasing_suppliers;
INSERT INTO purchasing_grn_lines (id, grn_id, product_id, product_name, qty_milli, unit_cost_minor)
  SELECT 'GL' || p.rowid, 'GRN' || printf('%024dSP', p.rowid % {SUPPLIERS} + 1), p.id, p.name, 10000,
         p.cost_price_minor
  FROM catalog_products p WHERE p.rowid % 10 = 0;
INSERT INTO customers_customers (id, phone, name, created_at, updated_at)
  WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < {CUSTOMERS})
  SELECT printf('%024dCS', i), '+88017' || printf('%08d', i), 'Customer ' || i, 0, 0 FROM n;
INSERT INTO customers_credit_ledger (id, customer_id, amount_minor, kind, at, business_date)
  WITH RECURSIVE m(month) AS (SELECT 0 UNION ALL SELECT month + 1 FROM m WHERE month < 11)
  SELECT 'CS' || c.id || '-' || month, c.id, 20000, 'credit_sale', 0,
         date('2025-01-01', '+' || month || ' months')
  FROM customers_customers c, m WHERE c.rowid % 5 = 0;
INSERT INTO customers_credit_ledger (id, customer_id, amount_minor, kind, at, business_date)
  WITH RECURSIVE m(month) AS (SELECT 0 UNION ALL SELECT month + 1 FROM m WHERE month < 11)
  SELECT 'CP' || c.id || '-' || month, c.id, -10000, 'payment', 0,
         date('2025-01-01', '+' || month || ' months', '+10 days')
  FROM customers_customers c, m WHERE c.rowid % 5 = 0;
";
