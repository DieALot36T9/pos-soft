//! One file per use case. Each workflow that saves data is one transaction;
//! printing and the drawer happen after the commit.

pub mod audit_view;
pub mod backup;
pub mod catalog_lists;
pub mod folders;
pub mod products;
pub mod settings_change;
pub mod setup;
pub mod tax_rates;
pub mod test_print;
pub mod users;
