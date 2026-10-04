//! The shape of a module's database migrations. Modules make these lists and
//! `db` runs them. They live here so modules don't need to depend on `db`.

/// One SQL file from a module's `migrations/` folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    /// 1, 2, 3, … in the order they run. Taken from the file name `001_init.sql`.
    pub version: u32,
    /// Example: "init".
    pub name: &'static str,
    /// The SQL, embedded with `include_str!`.
    pub sql: &'static str,
}

/// All migrations of one module, oldest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleMigrations {
    /// The module name, which is also the prefix of its tables. Example: "catalog".
    pub module: &'static str,
    pub migrations: &'static [Migration],
}
