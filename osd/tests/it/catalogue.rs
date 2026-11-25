//! The SyntHub catalogue read by `osd`: every driver and maker profile
//! strictly typed, written back unchanged, resolved, and valid against the
//! published schemas. The catalogue is the folder `SYNTHHUB_DEVICES` names;
//! without it these tests do nothing.

use crate::common;
use crate::common::trial::{first_difference, trial};
use crate::common::{catalogue, report, Schemas};
use osd::Kind;
use serde_json::Value;
use std::io::Write;

fn each(ids: &[String], dir: &common::Folder, mut check: impl FnMut(&str, &str) -> Option<String>) -> Vec<String> {
    ids.iter().filter_map(|id| check(id, &dir.text(id)).map(|why| format!("{id}: {why}"))).collect()
}

#[test]
fn the_catalogue_converted_to_0_7_is_refused_nowhere_0_6_accepts_it_and_loses_nothing() {
    let Some(dir) = catalogue() else { return };
    let found = trial(&dir);
    // Written to stderr itself, so the counts show on a green run too.
    writeln!(std::io::stderr(), "{found}").unwrap();
    assert!(found.files > 0, "no descriptor in the catalogue");
    assert!(found.failures.is_empty(), "the trial of 0.7:\n{found}");
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
            (back != original).then(|| {
                let (at, how) = first_difference(&original, &back, "");
                format!("{at}: {how}")
            })
        }));
    }
    report("files changed by a read and a write", bad);
}

#[test]
fn every_driver_resolves_into_a_device_valid_against_the_schema() {
    let Some(dir) = catalogue() else { return };
    let schemas = Schemas::load("0.4");
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
    let schemas = Schemas::load("0.4");
    let mut bad = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        bad.extend(each(&ids, &dir, |_, text| {
            let read = match osd::read(text, kind) { Ok(d) => d, Err(f) => return Some(f.to_string()) };
            schemas.check(kind, &serde_json::from_str(&osd::write(&read)).unwrap())
        }));
    }
    report("models not valid", bad);
}

#[test]
fn criterion_5_every_converted_file_reads_in_0_5_validates_and_converts_to_itself() {
    let Some(dir) = catalogue() else { return };
    let schemas = Schemas::load("0.5");
    let mut bad = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        bad.extend(each(&ids, &dir, |_, text| {
            let original: Value = serde_json::from_str(text).unwrap();
            let out = match osd::convert_to(text, kind, osd::Version::V0_5) {
                Ok(t) => t,
                // only the two expected faults (criterion 11) are skipped here
                Err(_) if holds_expected_fault(&original) => return None,
                Err(f) => return Some(f.to_string()),
            };
            if let Err(f) = osd::read_as(&out, kind, osd::Version::V0_5) {
                return Some(f.to_string());
            }
            if osd::convert_to(&out, kind, osd::Version::V0_5).as_deref() != Ok(out.as_str()) {
                return Some("converting the 0.5 text changes it".into());
            }
            schemas.check(kind, &serde_json::from_str(&out).unwrap())
        }));
    }
    report("files not converted into 0.5", bad);
}

/// A string frame element of 0.4 that names no algorithm or no function
/// (`{checksum}`, `{function}`): the converter cannot give it a structure.
/// The closed list of expected faults (criterion 11): any other fault is a failure.
/// Both are removed by a SyntHub ticket:
/// - `{checksum}` without an algorithm: SH-cgi.7;
/// - `{function}`: SH-cgi.7.
fn is_expected_fault(s: &str) -> bool {
    s == "{checksum}" || s.starts_with("{function")
}

fn holds_expected_fault(v: &Value) -> bool {
    match v {
        Value::String(s) => is_expected_fault(s),
        Value::Array(a) => a.iter().any(holds_expected_fault),
        Value::Object(o) => o.values().any(holds_expected_fault),
        _ => false,
    }
}

/// A control whose flat fields mix a CC, an NRPN or an RPN: 0.4 and 0.5 say it, and no converter
/// to 0.6 chooses between them (`spec/0.6.md` 21.3).
fn mixes_messages(entry: &Value) -> bool {
    [["cc", "cc_lsb"], ["nrpn_msb", "nrpn_lsb"], ["rpn_msb", "rpn_lsb"]]
        .iter()
        .filter(|keys| keys.iter().any(|k| entry.get(k).is_some()))
        .count()
        > 1
}

fn holds_mixed_control(v: &Value) -> bool {
    v.pointer("/cc_params/entries").and_then(Value::as_array).is_some_and(|list| list.iter().any(mixes_messages))
}

/// Whether `pointer` is an entry of `cc_params` of `original` that mixes kinds of message.
fn at_mixed_control(original: &Value, pointer: &str) -> bool {
    let entry = pointer.strip_prefix("/cc_params/entries/").is_some_and(|n| !n.contains('/'));
    entry && original.pointer(pointer).is_some_and(mixes_messages)
}

/// What is wrong with the conversion of a catalogue file to the published version, if anything.
/// A file that holds an expected fault (a string frame element with no structure, or a mixed
/// control) is refused as `NotConvertible` there; every other file converts.
fn published_conversion_fault(text: &str, kind: Kind) -> Option<String> {
    let original: Value = serde_json::from_str(text).unwrap();
    let result = osd::convert(text, kind);
    if !holds_expected_fault(&original) && !holds_mixed_control(&original) {
        return result.err().map(|f| format!("unexpected fault (any other fault goes up to the supervisor): {f}"));
    }
    let f = match result {
        Err(f) => f,
        Ok(_) => return Some("converted, but holds an expected fault".into()),
    };
    if f.kind != osd::FaultKind::NotConvertible {
        return Some(format!("fault of kind {:?}: {f}", f.kind));
    }
    match original.pointer(&f.pointer) {
        Some(Value::String(s)) if is_expected_fault(s) => None,
        _ if at_mixed_control(&original, &f.pointer) => None,
        other => Some(format!("fault at {}, which holds {other:?}", f.pointer)),
    }
}

#[test]
fn criterion_3_8_10_and_11_only_the_two_expected_faults_refuse_all_others_convert() {
    let Some(dir) = catalogue() else { return };
    let (mut faults, mut converted) = (0, 0);
    let mut bad = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        bad.extend(each(&ids, &dir, |_, text| {
            let original: Value = serde_json::from_str(text).unwrap();
            if holds_expected_fault(&original) || holds_mixed_control(&original) {
                faults += 1;
            } else {
                converted += 1;
            }
            published_conversion_fault(text, kind)
        }));
    }
    report("files not converted as expected", bad);
    assert!(faults > 0 && converted > 1000, "{faults} faults, {converted} converted");
}

#[test]
fn criterion_7_a_converted_catalogue_file_holds_no_string_frame_element_and_reads_back_unchanged() {
    let Some(dir) = catalogue() else { return };
    let mut bad = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        bad.extend(each(&ids, &dir, |_, text| {
            let original: Value = serde_json::from_str(text).unwrap();
            let out = match osd::convert_to(text, kind, osd::Version::V0_5) {
                Ok(t) => t,
                Err(_) if holds_expected_fault(&original) => return None,
                Err(f) => return Some(f.to_string()),
            };
            let read = match osd::read_as(&out, kind, osd::Version::V0_5) { Ok(d) => d, Err(f) => return Some(f.to_string()) };
            (osd::write(&read) != out).then(|| "written back differently".to_string())
        }));
    }
    report("converted files not read back", bad);
}

/// A 0.4 or 0.5 driver with these controls, as a text.
fn driver(schema: Option<&str>, entries: Value) -> String {
    let mut doc = serde_json::json!({"device": {"name": "S", "osc_prefix": "/s"}, "cc_params": {"entries": entries}});
    if let Some(s) = schema {
        doc["$schema"] = s.into();
    }
    doc.to_string()
}

const S5: &str = "https://w3id.org/osd/0.5/device.schema.json";

#[test]
fn the_catalogue_conversion_admits_the_refusal_of_a_mixed_control_at_that_control() {
    let entries = serde_json::json!([
        {"osc": "/a", "cc": 1},
        {"osc": "/b", "cc": 93, "nrpn_msb": 1, "nrpn_lsb": 102}
    ]);
    for schema in [None, Some(S5)] {
        assert_eq!(published_conversion_fault(&driver(schema, entries.clone()), Kind::Device), None, "{schema:?}");
    }
    let plain = serde_json::json!([{"osc": "/a", "cc": 1}]);
    assert_eq!(published_conversion_fault(&driver(None, plain), Kind::Device), None);
}

#[test]
fn the_catalogue_conversion_admits_that_refusal_at_no_other_control() {
    // Two controls claim one message: 0.6 refuses the later one, which mixes nothing.
    let twice = serde_json::json!([{"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 1}]);
    assert!(published_conversion_fault(&driver(Some(S5), twice), Kind::Device).is_some());
    // The same beside a mixed control that comes after the refused one.
    let beside = serde_json::json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 1}, {"osc": "/c", "cc": 2, "rpn_msb": 0, "rpn_lsb": 1}
    ]}});
    assert!(!at_mixed_control(&beside, "/cc_params/entries/1"));
    assert!(at_mixed_control(&beside, "/cc_params/entries/2"));
    assert!(!at_mixed_control(&beside, "/cc_params/entries/2/cc"));
}

#[test]
fn criterion_13_a_community_file_is_neither_a_driver_nor_a_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let maker = root.join("acme");
    std::fs::create_dir_all(&maker).unwrap();
    std::fs::write(maker.join("synth.json"), r#"{"device": {"name": "S", "osc_prefix": "/s"}}"#).unwrap();
    std::fs::write(maker.join("_profile.json"), r#"{"identity": {"reply_text": "acme"}}"#).unwrap();
    std::fs::write(maker.join("_community.json"), r#"{"links": []}"#).unwrap();
    std::fs::write(maker.join("_community.discord.json"), r#"{"links": []}"#).unwrap();
    let dir = common::Folder(root);
    let (drivers, profiles) = (dir.drivers(), dir.profiles());
    assert_eq!(drivers, ["acme/synth"]);
    assert_eq!(profiles, ["acme/_profile"]);
}
