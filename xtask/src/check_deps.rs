//! `cargo xtask check-deps`: fails if a crate other than `app` depends on a module crate.
//! Rule R1 in `docs/03 Architecture/Module Rules.md`.

use std::process::Command;

use serde_json::Value;

/// The only crate allowed to depend on modules.
const APP: &str = "app";
/// The shared crate every module may use.
const CORE: &str = "pos-core";
/// May list modules as dev-dependencies, so its tests can build their views.
const REPORTS: &str = "reports";

/// One workspace package, with just what the check needs.
#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    /// True for packages in the `crates/` folder.
    pub in_crates_folder: bool,
    pub dependencies: Vec<Dependency>,
}

/// One dependency of a package.
#[derive(Debug, Clone)]
pub struct Dependency {
    /// The package it points at (not a renamed alias).
    pub name: String,
    /// True for `[dev-dependencies]`.
    pub dev_only: bool,
}

/// Runs the check on the real workspace and prints what's wrong.
pub fn run() -> Result<(), String> {
    let packages = read_workspace()?;
    let problems = find_problems(&packages);
    if problems.is_empty() {
        println!("check-deps: ok, no module depends on another module.");
        return Ok(());
    }
    for problem in &problems {
        eprintln!("check-deps: {problem}");
    }
    Err(format!(
        "{} module dependency problem(s). See docs/03 Architecture/Module Rules.md (R1).",
        problems.len()
    ))
}

/// Module crates: everything in `crates/` except `core` and `app`.
pub fn module_names(packages: &[Package]) -> Vec<String> {
    let mut names = Vec::new();
    for package in packages {
        if package.in_crates_folder && package.name != CORE && package.name != APP {
            names.push(package.name.clone());
        }
    }
    names
}

/// Lists every broken rule, one sentence each. An empty list means all is well.
pub fn find_problems(packages: &[Package]) -> Vec<String> {
    let modules = module_names(packages);
    let mut problems = Vec::new();
    for package in packages {
        if package.name == APP {
            continue;
        }
        for dependency in &package.dependencies {
            if !modules.contains(&dependency.name) {
                continue;
            }
            // Rule R1 exception: reports' tests may build other modules' views.
            let is_allowed_test_helper = package.name == REPORTS && dependency.dev_only;
            if !is_allowed_test_helper {
                problems.push(format!(
                    "`{}` depends on the module `{}`. Only `app` may connect modules.",
                    package.name, dependency.name
                ));
            }
        }
    }
    problems
}

/// Reads the workspace's packages from `cargo metadata`.
fn read_workspace() -> Result<Vec<Package>, String> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|error| format!("could not run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cargo metadata gave unreadable output: {error}"))?;
    parse_packages(&metadata)
}

/// Turns `cargo metadata` JSON into packages.
fn parse_packages(metadata: &Value) -> Result<Vec<Package>, String> {
    let root = metadata["workspace_root"]
        .as_str()
        .ok_or("cargo metadata has no workspace_root")?;
    let crates_folder = std::path::Path::new(root).join("crates");
    let listed = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata has no packages")?;

    let mut packages = Vec::new();
    for entry in listed {
        let name = entry["name"].as_str().unwrap_or_default().to_string();
        let manifest = entry["manifest_path"].as_str().unwrap_or_default();
        let mut dependencies = Vec::new();
        for dependency in entry["dependencies"].as_array().into_iter().flatten() {
            dependencies.push(Dependency {
                name: dependency["name"].as_str().unwrap_or_default().to_string(),
                dev_only: dependency["kind"].as_str() == Some("dev"),
            });
        }
        packages.push(Package {
            name,
            in_crates_folder: std::path::Path::new(manifest).starts_with(&crates_folder),
            dependencies,
        });
    }
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, dependencies: &[(&str, bool)]) -> Package {
        Package {
            name: name.to_string(),
            in_crates_folder: name != "xtask",
            dependencies: dependencies
                .iter()
                .map(|(dependency, dev_only)| Dependency {
                    name: dependency.to_string(),
                    dev_only: *dev_only,
                })
                .collect(),
        }
    }

    /// A small workspace that follows the rules.
    fn good_workspace() -> Vec<Package> {
        vec![
            package("pos-core", &[("serde", false)]),
            package("catalog", &[("pos-core", false), ("rusqlite", false)]),
            package("sales", &[("pos-core", false)]),
            package("db", &[("pos-core", false)]),
            package("reports", &[("pos-core", false), ("sales", true)]),
            package(
                "app",
                &[
                    ("pos-core", false),
                    ("catalog", false),
                    ("sales", false),
                    ("db", false),
                ],
            ),
            package("xtask", &[("serde_json", false)]),
        ]
    }

    #[test]
    fn a_workspace_that_follows_the_rules_passes() {
        assert!(find_problems(&good_workspace()).is_empty());
    }

    #[test]
    fn a_module_depending_on_another_module_fails() {
        let mut workspace = good_workspace();
        workspace[1].dependencies.push(Dependency {
            name: "sales".to_string(),
            dev_only: false,
        });

        let problems = find_problems(&workspace);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("`catalog` depends on the module `sales`"));
    }

    #[test]
    fn a_module_depending_on_db_fails() {
        let mut workspace = good_workspace();
        workspace[2].dependencies.push(Dependency {
            name: "db".to_string(),
            dev_only: false,
        });
        assert_eq!(find_problems(&workspace).len(), 1);
    }

    #[test]
    fn even_a_dev_dependency_on_a_module_fails_outside_reports() {
        let mut workspace = good_workspace();
        workspace[1].dependencies.push(Dependency {
            name: "sales".to_string(),
            dev_only: true,
        });
        assert_eq!(find_problems(&workspace).len(), 1);
    }

    #[test]
    fn reports_may_not_use_a_module_outside_its_tests() {
        let mut workspace = good_workspace();
        workspace[4].dependencies.push(Dependency {
            name: "catalog".to_string(),
            dev_only: false,
        });
        assert_eq!(find_problems(&workspace).len(), 1);
    }

    #[test]
    fn core_may_not_depend_on_a_module() {
        let mut workspace = good_workspace();
        workspace[0].dependencies.push(Dependency {
            name: "catalog".to_string(),
            dev_only: false,
        });
        assert_eq!(find_problems(&workspace).len(), 1);
    }

    #[test]
    fn modules_are_the_crates_except_core_and_app() {
        assert_eq!(
            module_names(&good_workspace()),
            vec!["catalog", "sales", "db", "reports"]
        );
    }

    #[test]
    fn the_real_workspace_passes() {
        let packages = read_workspace().unwrap();
        assert_eq!(find_problems(&packages), Vec::<String>::new());
        // 19 crates: core, app and 17 module crates.
        assert_eq!(module_names(&packages).len(), 17);
    }
}
