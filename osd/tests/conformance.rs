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

/// What the case asked for that the reader did not return, if anything.
fn run(dir: &Path) -> Option<String> {
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

#[test]
fn every_resolution_case_of_0_4_passes() {
    let root = common::repo().join("conformance/0.4/resolve");
    let mut cases: Vec<_> = std::fs::read_dir(&root).unwrap().map(|e| e.unwrap().path()).collect();
    cases.sort();
    assert!(cases.len() >= 13, "{} cases", cases.len());
    let failures = cases.iter()
        .filter_map(|c| run(c).map(|why| format!("{}: {why}", c.file_name().unwrap().to_string_lossy())))
        .collect();
    common::report("cases failed", failures);
}
