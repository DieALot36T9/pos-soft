//! `cargo xtask new-module <name>`: makes the skeleton of a new module.
//! Steps after this one are in `docs/09 Guides/Adding a Module.md`.

use std::fs;
use std::path::Path;

/// Names that can't be used: the existing shared crates and Rust's own crates.
const RESERVED_NAMES: &[&str] = &["core", "pos_core", "app", "std", "alloc", "test", "xtask"];

/// Makes the crate, the screen folder and (if missing) the module page.
pub fn run(root: &Path, name: &str) -> Result<(), String> {
    check_name(root, name)?;
    let type_name = pascal_case(name);
    let claude_template = claude_md_template(root)?;

    let crate_folder = root.join("crates").join(name);
    write(
        &crate_folder.join("Cargo.toml"),
        &fill(CARGO_TOML, name, &type_name),
    )?;
    write(
        &crate_folder.join("src/lib.rs"),
        &fill(LIB_RS, name, &type_name),
    )?;
    write(
        &crate_folder.join("src/error.rs"),
        &fill(ERROR_RS, name, &type_name),
    )?;
    write(
        &crate_folder.join("migrations/001_init.sql"),
        &fill(INIT_SQL, name, &type_name),
    )?;
    write(
        &crate_folder.join("tests/contract.rs"),
        &fill(CONTRACT_RS, name, &type_name),
    )?;
    write(
        &crate_folder.join("CLAUDE.md"),
        &fill(&claude_template, name, &type_name),
    )?;

    let screen_folder = root.join("ui/src/modules").join(name);
    write(
        &screen_folder.join("api.ts"),
        &fill(API_TS, name, &type_name),
    )?;
    write(
        &screen_folder.join("text/en.ts"),
        &fill(EN_TS, name, &type_name),
    )?;
    write(
        &screen_folder.join("text/index.ts"),
        &fill(TEXT_INDEX_TS, name, &type_name),
    )?;
    write(
        &screen_folder.join("CLAUDE.md"),
        &fill(&claude_template, name, &type_name),
    )?;

    let page = root.join(format!("docs/05 Modules/Module - {name}.md"));
    if !page.exists() {
        let template = read(&root.join("docs/Templates/Template - Module Page.md"))?;
        write(&page, &template.replace("{{name}}", name))?;
    }

    println!(
        "new-module: made `{name}`. Next steps: docs/09 Guides/Adding a Module.md (from step 4)."
    );
    Ok(())
}

/// A module name is lower-case letters, digits and `_`, and not already used.
fn check_name(root: &Path, name: &str) -> Result<(), String> {
    let mut characters = name.chars();
    let starts_with_letter = characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase());
    let rest_is_plain =
        characters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !starts_with_letter || !rest_is_plain {
        return Err(format!(
            "`{name}` is not a valid module name. Use lower-case letters, digits and _, like `loyalty`."
        ));
    }
    if RESERVED_NAMES.contains(&name) {
        return Err(format!("`{name}` is reserved. Choose another name."));
    }
    if root.join("crates").join(name).exists() {
        return Err(format!("crates/{name} already exists."));
    }
    Ok(())
}

/// The CLAUDE.md text: the markdown block inside `Template - Module CLAUDE.md.md`.
fn claude_md_template(root: &Path) -> Result<String, String> {
    let page = read(&root.join("docs/Templates/Template - Module CLAUDE.md.md"))?;
    let start = page
        .find("```markdown\n")
        .ok_or("the CLAUDE.md template has no ```markdown block")?;
    let body = &page[start + "```markdown\n".len()..];
    let end = body
        .find("\n```")
        .ok_or("the CLAUDE.md template block is not closed")?;
    Ok(format!("{}\n", &body[..end]))
}

/// Puts the module's names into a template.
fn fill(template: &str, name: &str, type_name: &str) -> String {
    template
        .replace("<name>", name)
        .replace("<folder>", name)
        .replace("{{name}}", name)
        .replace("{{Name}}", type_name)
}

/// `stock_take` → `StockTake`.
fn pascal_case(name: &str) -> String {
    let mut result = String::new();
    for word in name.split('_') {
        let mut letters = word.chars();
        if let Some(first) = letters.next() {
            result.push(first.to_ascii_uppercase());
            result.extend(letters);
        }
    }
    result
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("could not read {}: {error}", path.display()))
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    }
    fs::write(path, contents)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

const CARGO_TOML: &str = r#"[package]
name = "{{name}}"
description = "One sentence about what the {{name}} module does."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
pos-core.workspace = true
rusqlite.workspace = true
thiserror.workspace = true

[lints]
workspace = true
"#;

const LIB_RS: &str = r#"//! One sentence about what the {{name}} module does.
//!
//! Page: `docs/05 Modules/Module - {{name}}.md`.

mod error;

pub use error::{{Name}}Error;

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "{{name}}",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
"#;

const ERROR_RS: &str = r#"//! What can go wrong in the {{name}} module.

use pos_core::AppError;

/// Everything that can go wrong in the {{name}} module.
#[derive(Debug, thiserror::Error)]
pub enum {{Name}}Error {
    /// SQLite refused a query. The details go to the log.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
}

impl From<{{Name}}Error> for AppError {
    fn from(error: {{Name}}Error) -> AppError {
        AppError::internal(error.to_string())
    }
}
"#;

const INIT_SQL: &str =
    "-- The first tables of the {{name}} module. Every table and view starts with {{name}}_.\n";

const CONTRACT_RS: &str = r#"//! Contract tests: the public API and views behave as the module page says.

/// An in-memory database built from this module's own migrations.
fn test_db() -> rusqlite::Connection {
    let connection = rusqlite::Connection::open_in_memory().expect("memory database opens");
    for migration in {{name}}::migrations().migrations {
        connection.execute_batch(migration.sql).expect("migration runs");
    }
    connection
}

#[test]
fn migrations_run_on_an_empty_database() {
    test_db();
}
"#;

const API_TS: &str = r#"// Calls from the {{name}} screens into Rust. Components use these, never the bridge itself.
// Add one small function per command, using types from ../../generated. Example:
// export const listThings = (): Promise<Thing[]> => call("thing_list");
export {};
"#;

const EN_TS: &str = r#"// All screen text for the {{name}} module: labels, tooltips and messages, in plain English.
// A future language copies this shape exactly, so keep every word here (ADR-022).
export const en = {};

export type {{Name}}Text = typeof en;
"#;

const TEXT_INDEX_TS: &str = r#"import type { TextBundle } from "../../../shared/text/useText";
import { en, type {{Name}}Text } from "./en";

// Every language for this module. Only English exists today.
export const {{name}}Text: TextBundle<{{Name}}Text> = { en };
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// A throw-away copy of the repo's templates to run the generator in.
    fn scratch_root() -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| time.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!("xtask-new-module-{unique}"));
        let real_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        fs::create_dir_all(root.join("docs/Templates")).unwrap();
        fs::create_dir_all(root.join("docs/05 Modules")).unwrap();
        for template in [
            "Template - Module Page.md",
            "Template - Module CLAUDE.md.md",
        ] {
            fs::copy(
                real_root.join("docs/Templates").join(template),
                root.join("docs/Templates").join(template),
            )
            .unwrap();
        }
        root
    }

    #[test]
    fn makes_every_file_the_guide_lists() {
        let root = scratch_root();
        run(&root, "stock_take").unwrap();

        for file in [
            "crates/stock_take/Cargo.toml",
            "crates/stock_take/src/lib.rs",
            "crates/stock_take/src/error.rs",
            "crates/stock_take/migrations/001_init.sql",
            "crates/stock_take/tests/contract.rs",
            "crates/stock_take/CLAUDE.md",
            "ui/src/modules/stock_take/api.ts",
            "ui/src/modules/stock_take/text/en.ts",
            "ui/src/modules/stock_take/text/index.ts",
            "ui/src/modules/stock_take/CLAUDE.md",
            "docs/05 Modules/Module - stock_take.md",
        ] {
            assert!(root.join(file).exists(), "{file} is missing");
        }
        let error_rs = read(&root.join("crates/stock_take/src/error.rs")).unwrap();
        assert!(error_rs.contains("pub enum StockTakeError"));
        let claude_md = read(&root.join("crates/stock_take/CLAUDE.md")).unwrap();
        assert!(claude_md.starts_with("# Module: stock_take"));
        assert!(!claude_md.contains("<name>"));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn refuses_bad_and_used_names() {
        let root = scratch_root();
        assert!(run(&root, "Loyalty").is_err());
        assert!(run(&root, "core").is_err());
        assert!(run(&root, "").is_err());
        run(&root, "loyalty").unwrap();
        assert!(run(&root, "loyalty").is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn pascal_case_joins_words() {
        assert_eq!(pascal_case("stock_take"), "StockTake");
        assert_eq!(pascal_case("loyalty"), "Loyalty");
    }
}
