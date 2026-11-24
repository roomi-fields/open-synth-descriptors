//! The three crates share one version and one set of internal dependencies,
//! and each carries what crates.io requires.

use crate::common;
use toml::Table;

const CRATES: [&str; 3] = ["osd", "osd-formats", "osd-cli"];

fn read(path: &str) -> Table {
    let text = std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
    text.parse::<Table>().unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn workspace() -> Table {
    read("Cargo.toml")
}

fn is_inherited(value: &toml::Value) -> bool {
    value.get("workspace").and_then(|w| w.as_bool()) == Some(true)
}

#[test]
fn the_workspace_declares_the_one_version_of_the_crates() {
    let ws = workspace();
    let version = ws["workspace"]["package"]["version"].as_str().expect("[workspace.package] version");
    assert!(!version.is_empty());
}

#[test]
fn the_workspace_declares_each_internal_dependency_once_with_name_path_and_version() {
    let ws = workspace();
    let deps = ws["workspace"]["dependencies"].as_table().expect("[workspace.dependencies]");
    let version = ws["workspace"]["package"]["version"].as_str().unwrap();
    for (name, path) in [("osd", "osd"), ("osd-formats", "osd-formats")] {
        let dep = deps.get(name).unwrap_or_else(|| panic!("{name} missing from [workspace.dependencies]"));
        assert_eq!(dep["path"].as_str(), Some(path), "{name} path");
        assert_eq!(dep["version"].as_str(), Some(version), "{name} version");
    }
}

#[test]
fn a_crate_inherits_its_version_and_writes_none() {
    for name in CRATES {
        let manifest = read(&format!("{name}/Cargo.toml"));
        let version = &manifest["package"]["version"];
        assert!(is_inherited(version), "{name} writes its own version: {version}");
    }
}

#[test]
fn a_crate_inherits_its_internal_dependencies_and_writes_no_version_or_path() {
    for name in CRATES {
        let manifest = read(&format!("{name}/Cargo.toml"));
        for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(deps) = manifest.get(table).and_then(|t| t.as_table()) else { continue };
            for internal in ["osd", "osd-formats"] {
                let Some(dep) = deps.get(internal) else { continue };
                assert!(is_inherited(dep), "{name} [{table}] writes its own {internal}: {dep}");
                assert!(dep.get("version").is_none() && dep.get("path").is_none(), "{name} [{table}] {internal}");
            }
        }
    }
}

#[test]
fn a_crate_carries_every_field_crates_io_requires() {
    let ws = workspace();
    let shared = ws["workspace"]["package"].as_table().unwrap().clone();
    for name in CRATES {
        let manifest = read(&format!("{name}/Cargo.toml"));
        let package = manifest["package"].as_table().unwrap();
        for field in ["description", "license", "repository", "readme", "keywords", "categories"] {
            let value = package.get(field).unwrap_or_else(|| panic!("{name}: {field} missing"));
            let resolved = if is_inherited(value) {
                shared.get(field).unwrap_or_else(|| panic!("{name}: {field} inherited but not in [workspace.package]"))
            } else {
                value
            };
            let empty = match resolved {
                toml::Value::String(s) => s.is_empty(),
                toml::Value::Array(a) => a.is_empty(),
                _ => false,
            };
            assert!(!empty, "{name}: {field} empty");
        }
    }
}

#[test]
fn a_crate_ships_its_licence_files_and_a_readme_that_the_package_includes() {
    for name in CRATES {
        let dir = common::repo().join(name);
        for file in ["LICENSE-MIT", "LICENSE-APACHE"] {
            assert!(dir.join(file).is_file(), "{name}/{file} missing");
        }
        let manifest = read(&format!("{name}/Cargo.toml"));
        let readme = manifest["package"]["readme"].as_str().unwrap_or("README.md");
        let text = std::fs::read_to_string(dir.join(readme)).unwrap_or_else(|e| panic!("{name}/{readme}: {e}"));
        assert!(!text.trim().is_empty(), "{name}/{readme} empty");
        if let Some(include) = manifest["package"].get("include").and_then(|i| i.as_array()) {
            let names: Vec<&str> = include.iter().filter_map(|v| v.as_str()).collect();
            for file in [readme, "LICENSE-MIT", "LICENSE-APACHE"] {
                assert!(names.contains(&file), "{name}: include leaves out {file}");
            }
        }
        if let Some(exclude) = manifest["package"].get("exclude").and_then(|i| i.as_array()) {
            for v in exclude.iter().filter_map(|v| v.as_str()) {
                assert!(v != readme && !v.starts_with("LICENSE"), "{name}: exclude drops {v}");
            }
        }
    }
}

#[test]
fn the_npm_package_carries_the_workspace_version() {
    let ws = workspace();
    let version = ws["workspace"]["package"]["version"].as_str().unwrap();
    let text = std::fs::read_to_string(common::repo().join("npm/package.json")).expect("npm/package.json");
    let package: serde_json::Value = serde_json::from_str(&text).expect("npm/package.json is JSON");
    assert_eq!(package["version"].as_str(), Some(version), "npm/package.json version");
}

#[test]
fn the_first_two_numbers_of_the_workspace_version_are_the_published_version_of_the_format() {
    let ws = workspace();
    let version = ws["workspace"]["package"]["version"].as_str().expect("[workspace.package] version");
    let mut numbers = version.split('.');
    let first_two = format!("{}.{}", numbers.next().unwrap(), numbers.next().expect("a minor number"));
    assert_eq!(first_two, osd::PUBLISHED.name(), "workspace version {version}");
    assert!(numbers.next().is_some_and(|patch| patch.chars().all(|c| c.is_ascii_digit())), "a third number: {version}");
    assert!(numbers.next().is_none(), "three numbers only: {version}");
}

#[test]
fn the_npm_package_lives_outside_the_cargo_workspace() {
    let ws = workspace();
    let members = ws["workspace"]["members"].as_array().unwrap();
    assert!(members.iter().all(|m| !m.as_str().unwrap().starts_with("npm")));
    assert!(!common::repo().join("npm/Cargo.toml").exists());
}
