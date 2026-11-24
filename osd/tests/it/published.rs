//! A published version never changes: every schema file of a version recorded
//! in `schemas/SHA256SUMS` keeps its checksum, and no file joins the version.
//! The vocabulary folder `vocabulary/1` is held the same way, as one unit.

use crate::common;

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
/// or missing, and each file of a recorded unit (a version, or `vocabulary/1`)
/// that is not recorded.
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
    let units: std::collections::BTreeSet<&str> =
        recorded.keys().filter_map(|p| p.rsplit_once('/')).map(|(unit, _)| unit).collect();
    for unit in units {
        for f in std::fs::read_dir(schemas.join(unit)).unwrap() {
            let name = f.unwrap().file_name().into_string().unwrap();
            let path = format!("{unit}/{name}");
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
    for version in ["0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "vocabulary/1"] {
        assert!(recorded.keys().any(|p| p.starts_with(&format!("{version}/"))), "{version} not recorded");
    }
    assert_eq!(faults(&schemas, &sums), Vec::<String>::new());
}

#[test]
fn the_checksums_record_the_five_files_of_0_5() {
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
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
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
    assert_eq!(found, ["9.9/a.schema.json: changed", "9.9/b.schema.json: missing", "9.9/d.schema.json: added"]);
}

#[test]
fn a_patch_version_folder_is_a_unit_beside_the_folder_it_corrects() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    for unit in ["9.9", "9.9.1"] {
        std::fs::create_dir_all(dir.join(unit)).unwrap();
        std::fs::write(dir.join(unit).join("a.schema.json"), "a").unwrap();
    }
    let sums: String = ["9.9", "9.9.1"]
        .iter()
        .map(|unit| format!("{:x}  {unit}/a.schema.json\n", Sha256::digest(b"a")))
        .collect();
    assert_eq!(faults(&dir, &sums), Vec::<String>::new());

    std::fs::write(dir.join("9.9.1/a.schema.json"), "edited").unwrap();
    std::fs::write(dir.join("9.9.1/b.schema.json"), "b").unwrap();
    assert_eq!(faults(&dir, &sums), ["9.9.1/a.schema.json: changed", "9.9.1/b.schema.json: added"]);
}

fn recorded_files(unit: &str) -> Vec<String> {
    let sums = std::fs::read_to_string(common::repo().join("schemas/SHA256SUMS")).unwrap();
    let mut files: Vec<String> = recorded(&sums).into_keys().filter(|p| p.starts_with(&format!("{unit}/"))).collect();
    files.sort();
    files
}

#[test]
fn the_checksums_record_the_six_files_of_0_6() {
    let want: Vec<String> = ["assembly", "device", "fingerprint", "firmware", "profile", "sound"]
        .iter()
        .map(|k| format!("0.6/{k}.schema.json"))
        .collect();
    assert_eq!(recorded_files("0.6"), want);
}

#[test]
fn the_checksums_record_the_two_files_of_the_vocabulary_unit() {
    assert_eq!(recorded_files("vocabulary/1"), ["vocabulary/1/terms.json", "vocabulary/1/vocabulary.schema.json"]);
}

#[test]
fn a_changed_missing_or_added_file_of_the_vocabulary_unit_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(dir.join("vocabulary/1")).unwrap();
    for name in ["terms.json", "vocabulary.schema.json"] {
        std::fs::write(dir.join("vocabulary/1").join(name), name).unwrap();
    }
    let sums: String = ["terms.json", "vocabulary.schema.json"]
        .iter()
        .map(|name| format!("{:x}  vocabulary/1/{name}\n", Sha256::digest(name.as_bytes())))
        .collect();
    assert_eq!(faults(&dir, &sums), Vec::<String>::new());

    std::fs::write(dir.join("vocabulary/1/terms.json"), "edited").unwrap();
    std::fs::remove_file(dir.join("vocabulary/1/vocabulary.schema.json")).unwrap();
    std::fs::write(dir.join("vocabulary/1/extra.json"), "x").unwrap();
    assert_eq!(
        faults(&dir, &sums),
        [
            "vocabulary/1/terms.json: changed",
            "vocabulary/1/vocabulary.schema.json: missing",
            "vocabulary/1/extra.json: added"
        ]
    );
}

#[test]
fn every_id_of_the_six_schemas_of_0_6_is_its_w3id_address() {
    for kind in ["assembly", "device", "fingerprint", "firmware", "profile", "sound"] {
        let path = common::repo().join(format!("schemas/0.6/{kind}.schema.json"));
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(doc["$id"], format!("https://w3id.org/osd/0.6/{kind}.schema.json"), "{kind}");
    }
}
