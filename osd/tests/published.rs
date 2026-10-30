//! A published version never changes: every schema file of a version recorded
//! in `schemas/SHA256SUMS` keeps its checksum, and no file joins the version.

mod common;

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// The checksums recorded in `sums` (`sha256sum` lines: the hex digest, two
/// spaces, the path under `schemas/`), by path.
fn recorded(sums: &str) -> BTreeMap<String, String> {
    sums.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (hash, path) = l.split_once("  ").unwrap_or_else(|| panic!("bad line: {l}"));
            (path.to_string(), hash.to_string())
        })
        .collect()
}

/// What breaks the record in the `schemas` folder: each recorded file changed
/// or missing, and each file of a recorded version that is not recorded.
fn faults(schemas: &Path, sums: &str) -> Vec<String> {
    let recorded = recorded(sums);
    let mut out = Vec::new();
    for (path, hash) in &recorded {
        match std::fs::read(schemas.join(path)) {
            Err(_) => out.push(format!("{path}: missing")),
            Ok(bytes) => {
                let now = format!("{:x}", Sha256::digest(&bytes));
                if &now != hash {
                    out.push(format!("{path}: changed"));
                }
            }
        }
    }
    let versions: std::collections::BTreeSet<&str> =
        recorded.keys().filter_map(|p| p.split_once('/')).map(|(v, _)| v).collect();
    for version in versions {
        for f in std::fs::read_dir(schemas.join(version)).unwrap() {
            let name = f.unwrap().file_name().into_string().unwrap();
            let path = format!("{version}/{name}");
            if !recorded.contains_key(&path) {
                out.push(format!("{path}: added"));
            }
        }
    }
    out
}

#[test]
fn the_published_versions_are_unchanged() {
    let schemas = common::repo().join("schemas");
    let sums = std::fs::read_to_string(schemas.join("SHA256SUMS")).unwrap();
    let recorded = recorded(&sums);
    for version in ["0.1", "0.2", "0.3", "0.4", "0.5"] {
        assert!(recorded.keys().any(|p| p.starts_with(&format!("{version}/"))), "{version} not recorded");
    }
    assert_eq!(faults(&schemas, &sums), Vec::<String>::new());
}

#[test]
fn criterion_12_the_checksums_record_the_five_files_of_0_5() {
    let sums = std::fs::read_to_string(common::repo().join("schemas/SHA256SUMS")).unwrap();
    let mut files: Vec<String> = recorded(&sums).into_keys().filter(|p| p.starts_with("0.5/")).collect();
    files.sort();
    let want: Vec<String> = ["device", "fingerprint", "firmware", "profile", "sound"]
        .iter()
        .map(|k| format!("0.5/{k}.schema.json"))
        .collect();
    assert_eq!(files, want);
}

#[test]
fn a_changed_missing_or_added_file_is_named() {
    let dir = std::env::temp_dir().join(format!("osd-published-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("9.9")).unwrap();
    for name in ["a.schema.json", "b.schema.json", "c.schema.json"] {
        std::fs::write(dir.join("9.9").join(name), name).unwrap();
    }
    let sums: String = ["a.schema.json", "b.schema.json", "c.schema.json"]
        .iter()
        .map(|name| format!("{:x}  9.9/{name}\n", Sha256::digest(name.as_bytes())))
        .collect();
    assert_eq!(faults(&dir, &sums), Vec::<String>::new());

    std::fs::write(dir.join("9.9/a.schema.json"), "a, edited").unwrap();
    std::fs::remove_file(dir.join("9.9/b.schema.json")).unwrap();
    std::fs::write(dir.join("9.9/d.schema.json"), "d").unwrap();
    let found = faults(&dir, &sums);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(found, ["9.9/a.schema.json: changed", "9.9/b.schema.json: missing", "9.9/d.schema.json: added"]);
}
