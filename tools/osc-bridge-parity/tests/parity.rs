//! `osd` and osc-bridge resolve the same driver into the same document: the
//! drivers of the catalogue `SYNTHHUB_DEVICES` names, and the resolution
//! conformance cases of the repository.

use serde_json::Value;
use std::path::{Path, PathBuf};

/// Files kept in a folder by catalogue id: `korg/minilogue` is
/// `<folder>/korg/minilogue.json`.
struct Folder(PathBuf);

impl osd::Files for Folder {
    fn get(&self, id: &str) -> Option<String> {
        std::fs::read_to_string(self.0.join(format!("{id}.json"))).ok()
    }
}

/// What osc-bridge resolves the driver `id` into, or its refusal.
fn osc_bridge(dir: &Path, id: &str) -> Result<Value, String> {
    let path = dir.join(format!("{id}.json"));
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    osc_bridge::device::resolve(&path, doc).map_err(|e| e.to_string())
}

fn osd(dir: &Path, id: &str) -> Result<Value, String> {
    let r = osd::resolve(id, &Folder(dir.to_path_buf())).map_err(|f| f.to_string())?;
    Ok(serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap())
}

/// The drivers of a catalogue folder: every `.json` whose name does not
/// start with `_`.
fn drivers(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for maker in std::fs::read_dir(dir).unwrap() {
        let maker = maker.unwrap().path();
        if !maker.is_dir() {
            continue;
        }
        for f in std::fs::read_dir(&maker).unwrap() {
            let f = f.unwrap().path();
            let name = f.file_stem().unwrap().to_str().unwrap();
            if f.extension().is_some_and(|e| e == "json") && !name.starts_with('_') {
                out.push(format!("{}/{name}", maker.file_name().unwrap().to_str().unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// The drivers whose resolution differs, with how; and how many agree.
fn compare(dir: &Path, ids: &[String]) -> (usize, Vec<String>) {
    let mut same = 0;
    let mut differ = Vec::new();
    for id in ids {
        match (osd(dir, id), osc_bridge(dir, id)) {
            (Ok(a), Ok(b)) if a == b => same += 1,
            (Ok(_), Ok(_)) => differ.push(format!("{id}: the documents differ")),
            (Err(a), Err(_)) => differ.push(format!("{id}: both refuse it ({a})")),
            (Ok(_), Err(b)) => differ.push(format!("{id}: only osc-bridge refuses it: {b}")),
            (Err(a), Ok(_)) => differ.push(format!("{id}: only osd refuses it: {a}")),
        }
    }
    (same, differ)
}

#[test]
fn every_driver_of_the_catalogue_resolves_as_osc_bridge_resolves_it() {
    let Some(dir) = std::env::var_os("SYNTHHUB_DEVICES").map(PathBuf::from) else { return };
    let ids = drivers(&dir);
    let (same, differ) = compare(&dir, &ids);
    assert!(differ.is_empty(), "{} of {} drivers differ:\n{}", differ.len(), ids.len(), differ.join("\n"));
    assert!(same > 1000, "{same} drivers compared");
    // osc-bridge's own loading accepts every one of them too.
    let refused: Vec<String> = ids.iter()
        .filter_map(|id| osc_bridge::device::Device::load(dir.join(format!("{id}.json"))).err().map(|e| format!("{id}: {e}")))
        .collect();
    assert!(refused.is_empty(), "Device::load refuses:\n{}", refused.join("\n"));
}

#[test]
fn every_resolution_case_resolves_as_osc_bridge_resolves_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/0.4/resolve");
    let mut checked = 0;
    for case in std::fs::read_dir(&root).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        let id = spec["resolve"].as_str().unwrap();
        let files = case.join("files");
        let (a, b) = (osd(&files, id), osc_bridge(&files, id));
        match (&a, &b) {
            (Ok(x), Ok(y)) => assert_eq!(x, y, "{}", case.display()),
            (Err(_), Err(_)) => {}
            _ => panic!("{}: osd {a:?}, osc-bridge {b:?}", case.display()),
        }
        checked += 1;
    }
    assert!(checked >= 13, "{checked} cases");
}
