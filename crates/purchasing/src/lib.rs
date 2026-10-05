//! Suppliers, purchase orders, goods received (GRN), supplier returns and the payables ledger.
//!
//! Page: `docs/05 Modules/Module - purchasing.md`. Requirements PUR-01 to PUR-06.
//! Recording a GRN or a return does not change stock: `app` calls inventory for that.

mod aging;
mod context;
mod error;
mod ledger;
mod orders;
mod receiving;
mod returns;
mod sql;
mod statement;
mod suggest;
mod suppliers;

pub use aging::{AgingRow, aging};
pub use context::PurchasingCtx;
pub use error::PurchasingError;
pub use ledger::{LedgerKind, balance, record_adjustment, record_payment, unbalanced_suppliers};
pub use orders::{
    NewOrder, NewOrderLine, Order, OrderLine, OrderLineRemaining, OrderRef, OrderStatus,
    cancel_order, create_order, get_order, mark_sent, order_for_receiving, update_order,
};
pub use receiving::{GrnRef, NewGrn, NewGrnLine, record_grn};
pub use returns::{NewReturnLine, NewSupplierReturn, ReturnRef, record_return};
pub use statement::{Statement, StatementLine, statement};
pub use suggest::{LowStockItem, suggest_order_lines};
pub use suppliers::{
    Supplier, SupplierInput, archive_supplier, create_supplier, get_supplier, list_suppliers,
    update_supplier,
};

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "purchasing",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
