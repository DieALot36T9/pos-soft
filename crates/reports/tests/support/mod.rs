//! The fixture shop: one scripted business day (2026-03-05) plus a little of the
//! days around it. Every report test checks its numbers against this script.
//!
//! The rows are written straight into the other modules' tables. This is test code
//! only: it keeps every amount visible here, so each expected number can be worked
//! out by hand. The `reports` code itself reads only the `*_v_*` views.
//!
//! The shop is in Dhaka (UTC+6) and prices include tax.
//!
//! | Doc | Till / cashier | Local time | Lines | Paid |
//! |---|---|---|---|---|
//! | T1-000001 | T1 Rina | 03-05 10:15 | 2 Milk 200.00, 1 kg Rice 50.00 | cash 250.00 |
//! | T1-000002 | T1 Rina | 03-05 11:30 | Soap 40.00 − promo 5.00, Milk 100.00 − manual 10.00 | card 125.00 |
//! | T1-000003 | T1 Rina | 03-05 12:00 | 3 kg Rice, price overridden to 44.99 = 134.97, rounding +0.03 | cash 135.00 |
//! | T1-000004 | T1 Rina | 03-05 13:00 | Soap 40.00, **voided** ("Wrong item") | cash 40.00 |
//! | T1-R000001 | T1 Rina | 03-05 15:00 | return 1 Milk from T1-000001: 100.00 | cash refund 100.00 |
//! | T2-000001 | T2 Karim | 03-05 16:00 | Soap 40.00 | cash 40.00 |
//! | T1-R000002 | T1 Rina | 03-06 10:00 | return the Soap from T1-000002: 35.00 | card refund 35.00 |

// Each test file uses a different part of this module.
#![allow(dead_code)]
// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod owed_data;
mod sales_data;
mod stock_data;
mod till_data;

use std::collections::BTreeMap;

use pos_core::{BusinessDate, Cell, Id, ModuleMigrations, Money, Qty, ReportTable};
use reports::ShopContext;
use rusqlite::Connection;

/// A fixed id made from a short tag, so test data is easy to read. Example: "T1".
pub fn id(tag: &str) -> Id {
    Id::parse(&format!("{tag:0>26}")).unwrap()
}

pub fn date(text: &str) -> BusinessDate {
    BusinessDate::parse(text).unwrap()
}

/// 2026-03-05 HH:MM in Dhaka (UTC+6), as unix milliseconds. `day` moves it by whole days.
pub fn local_ms(day: i64, hour: i64, minute: i64) -> i64 {
    let midnight_utc_0305 = 1_772_668_800_000;
    midnight_utc_0305 + day * 86_400_000 + ((hour * 60 + minute) - 360) * 60_000
}

/// Money from minor units. `m(12345)` is 123.45.
pub fn m(minor: i64) -> Cell {
    Cell::Money(Money(minor))
}

pub fn q(milli: i64) -> Cell {
    Cell::Qty(Qty(milli))
}

pub fn t(text: &str) -> Cell {
    Cell::Text(text.to_string())
}

/// The row whose first cell is this text. Fails the test if there is none.
pub fn row<'a>(table: &'a ReportTable, first: &str) -> &'a Vec<Cell> {
    table
        .rows
        .iter()
        .find(|row| row.first() == Some(&t(first)))
        .unwrap_or_else(|| panic!("no row {first:?} in {}", table.title))
}

/// The section with this title. Fails the test if there is none.
pub fn section<'a>(table: &'a ReportTable, title: &str) -> &'a ReportTable {
    table
        .sections
        .iter()
        .find(|section| section.title == title)
        .unwrap_or_else(|| panic!("no section {title:?} in {}", table.title))
}

/// The names `app` would pass in, and the Dhaka time zone.
pub fn shop_context() -> ShopContext {
    let mut user_names = BTreeMap::new();
    user_names.insert(id("R"), "Rina".to_string());
    user_names.insert(id("K"), "Karim".to_string());
    user_names.insert(id("A"), "Admin".to_string());
    let mut till_names = BTreeMap::new();
    till_names.insert(id("T1"), "Till 1".to_string());
    till_names.insert(id("T2"), "Till 2".to_string());
    let mut payment_method_names = BTreeMap::new();
    payment_method_names.insert(id("1"), "Cash".to_string());
    payment_method_names.insert(id("CARD"), "Card".to_string());
    ShopContext {
        user_names,
        till_names,
        payment_method_names,
        utc_offset_minutes: 360,
    }
}

/// An empty database with the views of every module the reports read.
pub fn empty_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    let modules: [ModuleMigrations; 7] = [
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
    conn
}

/// The fixture shop with the whole script above.
pub fn shop() -> Connection {
    let conn = empty_db();
    stock_data::add_catalog(&conn);
    sales_data::add_sales(&conn);
    sales_data::add_returns(&conn);
    till_data::add_shifts(&conn);
    till_data::add_audit(&conn);
    stock_data::add_stock(&conn);
    owed_data::add_payables(&conn);
    owed_data::add_receivables(&conn);
    conn
}
