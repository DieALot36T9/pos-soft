//! The list of every module's migrations, in a fixed order. `db::migrate` runs them.

use pos_core::ModuleMigrations;

/// Every module that owns tables, in the order their migrations run.
pub fn all() -> Vec<ModuleMigrations> {
    vec![
        settings::migrations(),
        auth::migrations(),
        audit::migrations(),
        tax::migrations(),
        catalog::migrations(),
        pricing::migrations(),
        inventory::migrations(),
        sales::migrations(),
        shifts::migrations(),
        customers::migrations(),
        purchasing::migrations(),
    ]
}
