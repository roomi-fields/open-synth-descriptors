//! The SyntHub catalogue read by `osd`: every driver and maker profile
//! strictly typed, written back unchanged, resolved, and valid against the
//! published schemas. The catalogue is the folder `SYNTHHUB_DEVICES` names;
//! without it these tests do nothing.

mod common;

use common::{catalogue, report, Schemas};
use osd::Kind;
use serde_json::Value;

fn each(ids: &[String], dir: &common::Folder, mut check: impl FnMut(&str, &str) -> Option<String>) -> Vec<String> {
    ids.iter().filter_map(|id| check(id, &dir.text(id)).map(|why| format!("{id}: {why}"))).collect()
}

#[test]
fn every_driver_and_profile_reads_strictly() {
    let Some(dir) = catalogue() else { return };
    let (drivers, profiles) = (dir.drivers(), dir.profiles());
    assert!(drivers.len() > 1000 && !profiles.is_empty(), "{} drivers, {} profiles", drivers.len(), profiles.len());
    let mut bad = each(&drivers, &dir, |_, text| osd::read(text, Kind::Device).err().map(|f| f.to_string()));
    bad.extend(each(&profiles, &dir, |_, text| osd::read(text, Kind::Profile).err().map(|f| f.to_string())));
    report("files not read", bad);
}

#[test]
fn documentary_fields_are_in_the_types() {
    let Some(dir) = catalogue() else { return };
    let (mut sources, mut coverage, mut sections, mut labels, mut notes) = (0, 0, 0, 0, 0);
    for id in dir.drivers() {
        let Ok(osd::Descriptor::Device(d)) = osd::read(&dir.text(&id), Kind::Device) else { continue };
        sources += d.sources.as_ref().map_or(0, Vec::len);
        coverage += usize::from(d.coverage.is_some());
        notes += d.remarks.len();
        let cc = d.cc_params.iter().flat_map(|t| &t.entries);
        sections += cc.filter(|e| e.section.is_some()).count();
        let params = d.params.iter().flat_map(|t| &t.entries);
        labels += params.filter_map(|e| e.values.as_ref()).map(|v| v.len()).sum::<usize>();
    }
    assert!(sources > 1000 && coverage > 1000 && sections > 1000 && notes > 1000 && labels > 0,
        "sources {sources}, coverage {coverage}, sections {sections}, notes {notes}, labels {labels}");
}

#[test]
fn every_file_written_back_equals_the_original() {
    let Some(dir) = catalogue() else { return };
    let mut bad = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        bad.extend(each(&ids, &dir, |_, text| {
            let original: Value = serde_json::from_str(text).unwrap();
            let read = match osd::read(text, kind) { Ok(d) => d, Err(f) => return Some(f.to_string()) };
            let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
            (back != original).then(|| first_difference(&original, &back, ""))
        }));
    }
    report("files changed by a read and a write", bad);
}

#[test]
fn every_driver_resolves_into_a_device_valid_against_the_schema() {
    let Some(dir) = catalogue() else { return };
    let schemas = Schemas::load();
    let bad = each(&dir.drivers(), &dir, |id, _| {
        let r = match osd::resolve(id, &dir) { Ok(r) => r, Err(f) => return Some(f.to_string()) };
        if r.files.first().map(String::as_str) != Some(id) {
            return Some(format!("built from {:?}", r.files));
        }
        let doc: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap();
        schemas.check(Kind::Device, &doc)
    });
    report("drivers not resolved", bad);
}

#[test]
fn every_file_model_validates_against_the_published_schemas() {
    let Some(dir) = catalogue() else { return };
    let schemas = Schemas::load();
    let mut bad = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        bad.extend(each(&ids, &dir, |_, text| {
            let read = match osd::read(text, kind) { Ok(d) => d, Err(f) => return Some(f.to_string()) };
            schemas.check(kind, &serde_json::from_str(&osd::write(&read)).unwrap())
        }));
    }
    report("models not valid", bad);
}

/// Where two documents first differ, as a JSON pointer.
fn first_difference(a: &Value, b: &Value, at: &str) -> String {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                if x.get(k) != y.get(k) {
                    return match (x.get(k), y.get(k)) {
                        (Some(p), Some(q)) => first_difference(p, q, &format!("{at}/{k}")),
                        (p, q) => format!("{at}/{k}: {p:?} became {q:?}"),
                    };
                }
            }
            format!("{at}: differs")
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            let i = x.iter().zip(y).position(|(p, q)| p != q).unwrap_or(0);
            first_difference(&x[i], &y[i], &format!("{at}/{i}"))
        }
        _ => format!("{at}: {a} became {b}"),
    }
}
