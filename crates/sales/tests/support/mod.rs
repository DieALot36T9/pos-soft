//! Helpers shared by the sales tests: a fresh database and ready-made sales.
//! Prices and tax here stand in for the pricing and tax modules, which sales never calls.

// Each test file uses a different part of this module.
#![allow(dead_code)]
// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use pos_core::{Bps, BusinessDate, Id, Money, Qty, Timestamp, mul_qty, round_half_away};
use rusqlite::Connection;
use sales::{
    CompletedSale, LineSnapshot, LineSource, NewCartLine, NewPayment, PaymentKind, SaleRef,
    SaleTotals, TaxMode,
};

/// A new in-memory database with the sales migrations, like `app` builds it.
pub fn test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
    run_migrations(&conn);
    conn
}

/// Runs every sales migration on a connection.
pub fn run_migrations(conn: &Connection) {
    for migration in sales::migrations().migrations {
        conn.execute_batch(migration.sql).unwrap();
    }
}

/// The ids a cashier works with: till, shift and user.
#[derive(Clone)]
pub struct Till {
    pub till: Id,
    pub code: String,
    pub shift: Id,
    pub user: Id,
}

impl Till {
    pub fn new(code: &str) -> Till {
        Till {
            till: Id::new(),
            code: code.to_string(),
            shift: Id::new(),
            user: Id::new(),
        }
    }
}

/// 2026-03-05 10:00 UTC, the time used in the test data.
pub fn now() -> Timestamp {
    Timestamp(1_772_704_800_000)
}

pub fn today() -> BusinessDate {
    BusinessDate::parse("2026-03-05").unwrap()
}

/// A scanned line of `qty` thousandths.
pub fn scanned(product: &Id, qty: i64) -> NewCartLine {
    NewCartLine {
        product_id: product.clone(),
        qty: Qty(qty),
        source: LineSource::Scan,
        embedded_price: None,
    }
}

/// The system cash method's id, made by the migration.
pub fn cash_method(conn: &Connection) -> Id {
    sales::system_payment_method(conn, PaymentKind::Cash)
        .unwrap()
        .id
}

/// Cash covering exactly `amount`.
pub fn exact_cash(conn: &Connection, amount: Money) -> NewPayment {
    NewPayment {
        method_id: cash_method(conn),
        amount,
        tendered: Some(amount),
        reference: None,
    }
}

/// A line priced like the pricing and tax modules would: `unit_price` × qty,
/// prices include 15% VAT, unit cost 60% of the price.
pub fn priced_line(product: &Id, qty: Qty, unit_price: Money) -> LineSnapshot {
    let base = mul_qty(unit_price, qty).unwrap();
    let tax = Money(round_half_away(i128::from(base.0) * 1500, 11_500).unwrap());
    LineSnapshot {
        product_id: product.clone(),
        name: "Milk 1L".to_string(),
        sku: "MILK1".to_string(),
        unit_symbol: "pc".to_string(),
        category_id: Id::new(),
        brand_id: None,
        qty,
        unit_price,
        retail_price: unit_price,
        base,
        promotion: None,
        promo_discount: Money(0),
        tier_discount: Money(0),
        manual_discount: Money(0),
        invoice_discount: Money(0),
        net: base.checked_sub(tax).unwrap(),
        tax,
        gross: base,
        tax_rate_id: Id::new(),
        tax_name: "VAT 15%".to_string(),
        tax_bps: Bps(1500),
        tax_components: Vec::new(),
        unit_cost: Money(unit_price.0 * 6 / 10),
        price_overridden: false,
    }
}

/// Header totals made by adding up the lines, with no rounding and no change.
pub fn totals_of(lines: &[LineSnapshot]) -> SaleTotals {
    let mut totals = SaleTotals::default();
    for line in lines {
        totals.base.0 += line.base.0;
        totals.promo_discount.0 += line.promo_discount.0;
        totals.tier_discount.0 += line.tier_discount.0;
        totals.manual_discount.0 += line.manual_discount.0;
        totals.invoice_discount.0 += line.invoice_discount.0;
        totals.net.0 += line.net.0;
        totals.tax.0 += line.tax.0;
        totals.gross.0 += line.gross.0;
        totals.cost.0 += mul_qty(line.unit_cost, line.qty).unwrap().0;
    }
    totals.total = totals.gross;
    totals
}

/// A complete sale of the cart's lines at 90.00 each, paid with exact cash.
pub fn sale_of_cart(conn: &Connection, till: &Till, cart: &Id) -> CompletedSale {
    let cart = sales::get_cart(conn, cart).unwrap();
    let lines: Vec<LineSnapshot> = cart
        .lines
        .iter()
        .map(|line| priced_line(&line.product_id, line.qty, Money(9_000)))
        .collect();
    let totals = totals_of(&lines);
    CompletedSale {
        cart_id: cart.id.clone(),
        till_id: till.till.clone(),
        till_code: till.code.clone(),
        shift_id: till.shift.clone(),
        user_id: till.user.clone(),
        business_date: today(),
        completed_at: now(),
        customer: None,
        price_list_id: None,
        tax_mode: TaxMode::Inclusive,
        exchange_return_id: None,
        totals,
        lines,
        payments: vec![exact_cash(conn, totals.total)],
    }
}

/// Opens a cart, adds the products with their quantities, and returns the cart id.
pub fn cart_with(conn: &mut Connection, till: &Till, products: &[(&Id, i64)]) -> Id {
    let tx = conn.transaction().unwrap();
    let cart = sales::open_cart(&tx, &till.till, &till.shift, &till.user, now()).unwrap();
    for (product, qty) in products {
        sales::add_line(&tx, &cart, scanned(product, *qty), false, now()).unwrap();
    }
    tx.commit().unwrap();
    cart
}

/// Rings up and completes a sale of the products, each in its own transaction.
pub fn sell(conn: &mut Connection, till: &Till, products: &[(&Id, i64)]) -> SaleRef {
    let cart = cart_with(conn, till, products);
    let tx = conn.transaction().unwrap();
    let sale = sale_of_cart(&tx, till, &cart);
    let sale_ref = sales::complete(&tx, sale).unwrap();
    tx.commit().unwrap();
    sale_ref
}

/// Rings up the lines' products and completes the sale with exactly these priced lines.
pub fn sell_priced(conn: &mut Connection, till: &Till, lines: Vec<LineSnapshot>) -> SaleRef {
    let products: Vec<(&Id, i64)> = lines
        .iter()
        .map(|line| (&line.product_id, line.qty.0))
        .collect();
    let cart = cart_with(conn, till, &products);
    let tx = conn.transaction().unwrap();
    let mut sale = sale_of_cart(&tx, till, &cart);
    sale.totals = totals_of(&lines);
    sale.payments = vec![exact_cash(&tx, sale.totals.total)];
    sale.lines = lines;
    let sale_ref = sales::complete(&tx, sale).unwrap();
    tx.commit().unwrap();
    sale_ref
}
