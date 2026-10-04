//! `cargo xtask gen-types [--check]`: makes the TypeScript types in
//! `ui/src/generated/` from the Rust types that derive `ts_rs::TS` (ADR-014).
//! With `--check` it changes nothing and fails if the committed files are out of date.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the generated files live, from the repo root.
const GENERATED_FOLDER: &str = "ui/src/generated";

/// Generates the types, or with `check_only` just compares them.
pub fn run(root: &Path, check_only: bool) -> Result<(), String> {
    let fresh_folder = root.join("target").join("ts-gen");
    if fresh_folder.exists() {
        fs::remove_dir_all(&fresh_folder).map_err(|error| error.to_string())?;
    }
    export_types(root, &fresh_folder)?;

    let fresh = read_ts_files(&fresh_folder)?;
    let generated_folder = root.join(GENERATED_FOLDER);
    let committed = read_ts_files(&generated_folder)?;
    let differences = compare(&committed, &fresh);

    if check_only {
        if differences.is_empty() {
            println!("gen-types: ok, {} files are up to date.", fresh.len());
            return Ok(());
        }
        for difference in &differences {
            eprintln!("gen-types: {difference}");
        }
        return Err("generated TypeScript types are out of date. Run `cargo xtask gen-types` and commit the result.".to_string());
    }

    write_files(&generated_folder, &committed, &fresh)?;
    println!(
        "gen-types: wrote {} files to {GENERATED_FOLDER} ({} changed).",
        fresh.len(),
        differences.len()
    );
    Ok(())
}

/// Runs the ts-rs export tests, which write one `.ts` file per type into `folder`.
fn export_types(root: &Path, folder: &Path) -> Result<(), String> {
    let status = Command::new(env!("CARGO"))
        .current_dir(root)
        .env("TS_RS_EXPORT_DIR", folder)
        .args([
            "test",
            "--workspace",
            "--lib",
            "--quiet",
            "--",
            "export_bindings",
        ])
        .status()
        .map_err(|error| format!("could not run cargo test: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("exporting the TypeScript types failed".to_string())
    }
}

/// Reads every `.ts` file in a folder: file name → contents. A missing folder is empty.
fn read_ts_files(folder: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut files = BTreeMap::new();
    if !folder.exists() {
        return Ok(files);
    }
    let entries = fs::read_dir(folder).map_err(|error| error.to_string())?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_some_and(|extension| extension == "ts") {
            let name = file_name(&path);
            let contents = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            files.insert(name, contents);
        }
    }
    Ok(files)
}

/// Lists, in plain words, how the committed files differ from fresh ones.
pub fn compare(
    committed: &BTreeMap<String, String>,
    fresh: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut differences = Vec::new();
    for (name, contents) in fresh {
        match committed.get(name) {
            None => differences.push(format!("{name} is missing")),
            Some(old) if old != contents => differences.push(format!("{name} has changed")),
            Some(_) => {}
        }
    }
    for name in committed.keys() {
        if !fresh.contains_key(name) {
            differences.push(format!("{name} is no longer made by any Rust type"));
        }
    }
    differences
}

/// Writes the fresh files and removes ones no Rust type makes any more.
fn write_files(
    folder: &Path,
    committed: &BTreeMap<String, String>,
    fresh: &BTreeMap<String, String>,
) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    for name in committed.keys() {
        if !fresh.contains_key(name) {
            fs::remove_file(folder.join(name)).map_err(|error| error.to_string())?;
        }
    }
    for (name, contents) in fresh {
        let path: PathBuf = folder.join(name);
        fs::write(&path, contents).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(name, contents)| (name.to_string(), contents.to_string()))
            .collect()
    }

    #[test]
    fn identical_files_have_no_differences() {
        let same = files(&[("Money.ts", "export type Money = number;")]);
        assert!(compare(&same, &same).is_empty());
    }

    #[test]
    fn missing_changed_and_stale_files_are_all_reported() {
        let committed = files(&[("Money.ts", "old"), ("Gone.ts", "x")]);
        let fresh = files(&[("Money.ts", "new"), ("Qty.ts", "y")]);
        assert_eq!(
            compare(&committed, &fresh),
            vec![
                "Money.ts has changed",
                "Qty.ts is missing",
                "Gone.ts is no longer made by any Rust type",
            ]
        );
    }
}
