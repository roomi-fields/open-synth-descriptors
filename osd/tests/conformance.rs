//! The conformance cases of the repository (`conformance/<version>/`), each
//! run as the specification states it.

mod common;

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// The files of a case, by catalogue id.
fn files(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for maker in std::fs::read_dir(dir).unwrap() {
        let maker = maker.unwrap().path();
        for f in std::fs::read_dir(&maker).unwrap() {
            let f = f.unwrap().path();
            let id = format!("{}/{}", maker.file_name().unwrap().to_str().unwrap(), f.file_stem().unwrap().to_str().unwrap());
            out.insert(id, std::fs::read_to_string(&f).unwrap());
        }
    }
    out
}

/// What a resolution case asked for that the reader did not return, if anything.
fn run_resolve(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let id = case["resolve"].as_str().unwrap();
    let got = osd::resolve(id, &files(&dir.join("files")));
    match (got, case.get("result"), case.get("fault")) {
        (Ok(r), Some(want), None) => {
            let doc: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap();
            (doc != *want).then(|| format!("resolved into {doc}"))
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "file": f.file, "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        (Ok(_), None, Some(want)) => Some(format!("resolved, but should be refused with {want}")),
        (Err(f), Some(_), None) => Some(format!("refused: {f}")),
        _ => Some("case.json has neither result nor fault, or both".into()),
    }
}

/// The kind a case reads, by the name of its schema.
fn kind(word: &str) -> osd::Kind {
    match word {
        "device" => osd::Kind::Device,
        "profile" => osd::Kind::Profile,
        "sound" => osd::Kind::Sound,
        "fingerprint" => osd::Kind::Fingerprint,
        "firmware" => osd::Kind::Firmware,
        _ => panic!("{word} is not a kind"),
    }
}

/// What a reading case asked for that the reader did not return, if anything.
fn run_read(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let got = osd::read(&text, kind(case["read"].as_str().unwrap()));
    match (got, case.get("result"), case.get("fault")) {
        (Ok(d), Some(want), None) => {
            let doc: Value = serde_json::from_str(&osd::write(&d)).unwrap();
            (doc != *want).then(|| format!("read into {doc}"))
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        (Ok(_), None, Some(want)) => Some(format!("read, but should be refused with {want}")),
        (Err(f), Some(_), None) => Some(format!("refused: {f}")),
        _ => Some("case.json has neither result nor fault, or both".into()),
    }
}

/// Every case of a folder run by `run`, the failures reported together.
fn every_case(root: &Path, least: usize, run: fn(&Path) -> Option<String>) {
    let mut cases: Vec<_> = std::fs::read_dir(root).unwrap().map(|e| e.unwrap().path()).collect();
    cases.sort();
    assert!(cases.len() >= least, "{} cases", cases.len());
    let failures = cases.iter()
        .filter_map(|c| run(c).map(|why| format!("{}: {why}", c.file_name().unwrap().to_string_lossy())))
        .collect();
    common::report("cases failed", failures);
}

#[test]
fn every_reading_case_of_0_4_passes() {
    every_case(&common::repo().join("conformance/0.4/read"), 2, run_read);
}

/// The schema errors a reading case's descriptor has, against what its case
/// expects: none for a result, exactly one at the fault's pointer for a
/// fault.
fn schema_errors(dir: &Path, schemas: &common::Schemas) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("descriptor.json")).unwrap()).unwrap();
    let errors = schemas.errors(kind(case["read"].as_str().unwrap()), &doc);
    let want: Vec<&str> = case.get("fault").map(|f| f["pointer"].as_str().unwrap()).into_iter().collect();
    let got: Vec<&str> = errors.iter().map(|(at, _)| at.as_str()).collect();
    (got != want).then(|| format!("schema errors {errors:?}, expected at {want:?}"))
}

#[test]
fn every_reading_case_validates_as_its_case_expects() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for version in std::fs::read_dir(common::repo().join("conformance")).unwrap() {
        let version = version.unwrap().path();
        let read = version.join("read");
        if !read.is_dir() {
            continue;
        }
        let schemas = common::Schemas::load(version.file_name().unwrap().to_str().unwrap());
        for case in std::fs::read_dir(&read).unwrap() {
            let case = case.unwrap().path();
            if let Some(why) = schema_errors(&case, &schemas) {
                failures.push(format!("{}: {why}", case.display()));
            }
            checked += 1;
        }
    }
    assert!(checked >= 2, "{checked} cases");
    common::report("cases do not validate as they expect", failures);
}

#[test]
fn every_file_of_a_resolution_case_that_expects_a_result_validates() {
    let schemas = common::Schemas::load("0.4");
    let mut failures = Vec::new();
    for case in std::fs::read_dir(common::repo().join("conformance/0.4/resolve")).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        if spec.get("result").is_none() {
            continue;
        }
        for (id, text) in files(&case.join("files")) {
            let kind = if id.contains("/_profile") { osd::Kind::Profile } else { osd::Kind::Device };
            if let Some(e) = schemas.check(kind, &serde_json::from_str(&text).unwrap()) {
                failures.push(format!("{}: {id}: {e}", case.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    common::report("files do not validate", failures);
}

#[test]
fn every_resolution_case_of_0_4_passes() {
    every_case(&common::repo().join("conformance/0.4/resolve"), 13, run_resolve);
}
