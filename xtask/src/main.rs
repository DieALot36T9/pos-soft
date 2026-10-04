//! Project scripts, written in plain Rust so they work the same on every OS.
//! Run them with `cargo xtask <command>`.

mod gen_types;
mod new_module;

use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "\
cargo xtask <command>

Commands:
  gen-types           Write TypeScript types for the UI into ui/src/generated
  gen-types --check   Fail if ui/src/generated is out of date (used by CI)
  new-module <name>   Make a new module: crate, screen folder and module page
";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let root = repo_root();

    let result = match words.as_slice() {
        ["gen-types"] => gen_types::run(&root, false),
        ["gen-types", "--check"] => gen_types::run(&root, true),
        ["new-module", name] => new_module::run(&root, name),
        _ => {
            print!("{HELP}");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// The repo root: the folder above `xtask/`.
fn repo_root() -> PathBuf {
    let xtask_folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xtask_folder
        .parent()
        .map(PathBuf::from)
        .unwrap_or(xtask_folder)
}
