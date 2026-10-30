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
        bad.extend(each(&ids, &dir, |id, text| {
            let original: Value = serde_json::from_str(text).unwrap();
            let out = match osd::convert(text, kind) {
                Ok(t) => t,
                // only the three expected faults (criterion 11) are skipped here
                Err(_) if holds_expected_fault(id, &original) => return None,
                Err(f) => return Some(f.to_string()),
            };
            if let Err(f) = osd::read_as(&out, kind, osd::Version::V0_5) {
                return Some(f.to_string());
            }
            if osd::convert(&out, kind).as_deref() != Ok(out.as_str()) {
                return Some("converting the 0.5 text changes it".into());
            }
            schemas.check(kind, &serde_json::from_str(&out).unwrap())
        }));
    }
    report("files not converted into 0.5", bad);
}

/// The descriptor of the catalogue whose `{position:lsb}` and `{position:msb}`
/// the engine does not know (`unknown placeholder kind`); SyntHub fixes its
/// driver to `u14_lsb`/`u14_msb` in its own ticket, and this entry goes then.
const LSB_MSB_FILE: &str = "novation/circuit.vendor.fw-extended-perf-v2";
const LSB_MSB_POINTER: &str = "/commands/8";

/// A string frame element of 0.4 that names no algorithm or no function
/// (`{checksum}`, `{function}`), or, in the one file named above, an
/// encoding `lsb` or `msb`: the converter cannot give it a structure.
/// The closed list of expected faults (criterion 11): a fourth is a failure.
/// Each is removed by a SyntHub ticket:
/// - `{checksum}` without an algorithm: SH-cgi.7;
/// - `{function}`: SH-cgi.7;
/// - `{position:lsb}`/`{position:msb}` in the Circuit descriptor: SH-cgi.10.
fn is_expected_fault(id: &str, s: &str) -> bool {
    s == "{checksum}"
        || s.starts_with("{function")
        || (id == LSB_MSB_FILE && (s.ends_with(":lsb}") || s.ends_with(":msb}")))
}

fn holds_expected_fault(id: &str, v: &Value) -> bool {
    match v {
        Value::String(s) => is_expected_fault(id, s),
        Value::Array(a) => a.iter().any(|x| holds_expected_fault(id, x)),
        Value::Object(o) => o.values().any(|x| holds_expected_fault(id, x)),
        _ => false,
    }
}

#[test]
fn criterion_3_8_10_and_11_only_the_three_expected_faults_refuse_all_others_convert() {
    let Some(dir) = catalogue() else { return };
    let (mut faults, mut converted, mut lsb_msb) = (0, 0, 0);
    let mut bad = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        bad.extend(each(&ids, &dir, |id, text| {
            let original: Value = serde_json::from_str(text).unwrap();
            let result = osd::convert(text, kind);
            // any other failure is red: every other form of the catalogue converts.
            if holds_expected_fault(id, &original) {
                faults += 1;
                let f = match result {
                    Err(f) => f,
                    Ok(_) => return Some("converted, but holds an expected fault".into()),
                };
                if f.kind != osd::FaultKind::NotConvertible {
                    return Some(format!("fault of kind {:?}: {f}", f.kind));
                }
                return match original.pointer(&f.pointer) {
                    Some(Value::String(s)) if is_expected_fault(id, s) => {
                        if id == LSB_MSB_FILE {
                            lsb_msb += 1;
                            if !f.pointer.starts_with(LSB_MSB_POINTER) {
                                return Some(format!("fault at {}, not under {LSB_MSB_POINTER}", f.pointer));
                            }
                        }
                        None
                    }
                    other => Some(format!("fault at {}, which holds {other:?}", f.pointer)),
                };
            }
            converted += 1;
            result.err().map(|f| format!("unexpected fault (a fourth goes up to the supervisor): {f}"))
        }));
    }
    report("files not converted as expected", bad);
    assert_eq!(lsb_msb, 1, "{LSB_MSB_FILE} must be refused at {LSB_MSB_POINTER}");
    assert!(faults > 0 && converted > 1000, "{faults} faults, {converted} converted");
}

#[test]
fn criterion_7_a_converted_catalogue_file_holds_no_string_frame_element_and_reads_back_unchanged() {
    let Some(dir) = catalogue() else { return };
    let mut bad = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        bad.extend(each(&ids, &dir, |id, text| {
            let original: Value = serde_json::from_str(text).unwrap();
            let out = match osd::convert(text, kind) {
                Ok(t) => t,
                Err(_) if holds_expected_fault(id, &original) => return None,
                Err(f) => return Some(f.to_string()),
            };
            let read = match osd::read_as(&out, kind, osd::Version::V0_5) { Ok(d) => d, Err(f) => return Some(f.to_string()) };
            (osd::write(&read) != out).then(|| "written back differently".to_string())
        }));
    }
    report("converted files not read back", bad);
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
