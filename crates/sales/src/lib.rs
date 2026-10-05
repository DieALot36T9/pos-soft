//! Carts, held sales, completed sales, payments, invoice numbers, returns and voids.
//!
//! Page: `docs/05 Modules/Module - sales.md`. Sales never calculates prices or tax:
//! the workflow sends fully priced lines, and sales checks that they add up.

// Money maths must never use floating-point numbers.
#![deny(clippy::float_arithmetic)]

mod cart;
mod cart_edit;
mod cart_hold;
mod check;
mod complete;
mod error;
mod numbers;
mod payment_method;
mod refund;
mod return_read;
mod returns;
mod sale;
mod sale_read;
mod settle;
mod sql;
mod void;

pub use cart::{
    Cart, CartLine, CartStatus, Discount, HeldCartRow, LineSource, NewCartLine, PriceOverride,
    current_cart, get_cart, held_carts, open_carts_on_shift,
};
pub use cart_edit::{
    add_line, open_cart, remove_line, set_customer, set_exchange_return, set_invoice_discount,
    set_line_discount, set_price_list, set_price_override, set_qty,
};
pub use cart_hold::{cancel, hold, resume};
pub use complete::complete;
pub use error::SalesError;
pub use payment_method::{
    PaymentKind, PaymentMethod, PaymentMethodInput, payment_method, payment_methods,
    save_payment_method, system_payment_method,
};
pub use refund::LineAmounts;
pub use return_read::{Return, ReturnLine, ReturnTotals, get_return};
pub use returns::{
    LineRefund, NewReturn, NewReturnLine, ReturnQty, ReturnRef, ReturnedProduct, ReturnedSaleLine,
    record_return, refund_amounts,
};
pub use sale::{
    CompletedSale, CustomerSnapshot, LineSnapshot, NewPayment, Payment, PromotionSnapshot, Sale,
    SaleLine, SaleRef, SaleStatus, SaleTotals, TaxMode, TaxPart, VoidInfo,
};
pub use sale_read::{
    SalesSummary, find_by_invoice, get_sale, has_any_sale, last_sale, returnable_qty, sales_summary,
};
pub use settle::{EnteredPayment, Settlement, settle_payments};
pub use void::void;

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "sales",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
