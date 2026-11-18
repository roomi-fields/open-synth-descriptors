//! A role as the `paramPath` of MIDI-CI Property Exchange Controller Resources
//! (M2-117-UM): a text path of CamelCase segments, the parts (each followed by its
//! number when it has one) and the setting last. `osd::roles` gives each role its
//! `paramPath`; `osd::role_from_param_path` reads such a path back to the same role,
//! against a named state of the vocabulary.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn device(roles: &[Value]) -> Value {
    let entries: Vec<Value> = roles.iter().enumerate().map(|(n, r)| json!({"osc": format!("/c{n}"), "cc": n + 1, "role": r})).collect();
    json!({
        "$schema": SCHEMA,
        "role_vocabulary": "1",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": entries}
    })
}

fn param_paths(roles: &[Value]) -> Vec<String> {
    let d = osd::read(&device(roles).to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    osd::roles(&d).into_iter().map(|r| r.param_path).collect()
}

fn role(parts: &[(&str, Option<i64>)], setting: &str) -> Value {
    let parts: Vec<Value> = parts
        .iter()
        .map(|(p, i)| match i {
            Some(i) => json!({"part": p, "index": i}),
            None => json!({"part": p}),
        })
        .collect();
    json!({"parts": parts, "setting": setting})
}

/// The CamelCase segment of a term name, written here from the rule: each word between
/// hyphens starts with a capital letter, the hyphens go.
fn camel(name: &str) -> String {
    name.split('-')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
        })
        .collect()
}

#[test]
fn a_role_is_given_as_the_param_path_of_its_parts_and_its_setting() {
    let got = param_paths(&[
        role(&[("filter", None)], "cutoff"),
        role(&[("filter", None), ("envelope", None)], "attack-time"),
        role(&[("amplifier", None), ("envelope", None)], "release-time"),
    ]);
    assert_eq!(got, ["/Filter/Cutoff", "/Filter/Envelope/AttackTime", "/Amplifier/Envelope/ReleaseTime"]);
}

#[test]
fn an_index_is_a_numeric_segment_after_its_part() {
    let got = param_paths(&[
        role(&[("oscillator", Some(2))], "pitch"),
        role(&[("oscillator", Some(2)), ("envelope", Some(1))], "attack-time"),
        role(&[("oscillator", Some(2)), ("envelope", None)], "attack-time"),
        role(&[("effect", Some(12))], "depth"),
    ]);
    assert_eq!(got, ["/Oscillator/2/Pitch", "/Oscillator/2/Envelope/1/AttackTime", "/Oscillator/2/Envelope/AttackTime", "/Effect/12/Depth"]);
}

#[test]
fn an_extension_segment_is_given_as_written_without_changing_its_case() {
    let got = param_paths(&[
        role(&[("filter", None)], "x-acme-wobble"),
        role(&[("filter", None), ("x-acme-ladder", None)], "cutoff"),
        role(&[("x-acme-ladder", Some(3)), ("envelope", None)], "attack-time"),
        role(&[("x-acme-ladder", None)], "x-acme-drive-2"),
    ]);
    assert_eq!(got, ["/Filter/x-acme-wobble", "/Filter/x-acme-ladder/Cutoff", "/x-acme-ladder/3/Envelope/AttackTime", "/x-acme-ladder/x-acme-drive-2"]);
}

#[test]
fn every_term_of_the_first_state_is_given_as_its_camel_case_segment_and_read_back() {
    let state: Value = serde_json::from_str(&std::fs::read_to_string(common::repo().join("schemas/vocabulary/1/terms.json")).unwrap()).unwrap();
    let terms = state["terms"].as_array().unwrap();
    let opens = |t: &Value| t["at_top"] == json!(true) || (t.get("stands_under").is_none() && t["at_top"] != json!(false));
    let mut failures = Vec::new();
    let mut checked = 0;
    for s in terms.iter().filter(|t| t["type"] == "setting") {
        for part in s["applies_to"].as_array().unwrap() {
            let p = terms.iter().find(|t| t["name"] == *part).unwrap();
            if !opens(p) {
                continue;
            }
            let want = format!("/{}/{}", camel(part.as_str().unwrap()), camel(s["name"].as_str().unwrap()));
            let r = role(&[(part.as_str().unwrap(), None)], s["name"].as_str().unwrap());
            let got = param_paths(std::slice::from_ref(&r));
            if got != [want.clone()] {
                failures.push(format!("{part} {}: {got:?}, expected {want}", s["name"]));
            }
            match osd::role_from_param_path(&want, "1") {
                Ok(back) => {
                    let d = osd::read(&device(&[r]).to_string(), Kind::Device).unwrap();
                    if osd::roles(&d)[0].role != back {
                        failures.push(format!("{want}: read back as another role"));
                    }
                }
                Err(f) => failures.push(format!("{want}: {f}")),
            }
            checked += 1;
        }
    }
    assert!(checked >= 20, "{checked} paths");
    common::report("terms fail", failures);
}

#[test]
fn a_param_path_is_read_back_to_the_same_role() {
    let roles = [
        role(&[("filter", None)], "cutoff"),
        role(&[("filter", None), ("envelope", None)], "attack-time"),
        role(&[("oscillator", Some(2))], "pitch"),
        role(&[("oscillator", Some(2)), ("envelope", Some(1))], "attack-time"),
        role(&[("filter", None)], "x-acme-wobble"),
        role(&[("filter", None), ("x-acme-ladder", None)], "cutoff"),
        role(&[("x-acme-ladder", Some(3)), ("envelope", None)], "attack-time"),
    ];
    let d = osd::read(&device(&roles).to_string(), Kind::Device).unwrap();
    for e in osd::roles(&d) {
        let back = osd::role_from_param_path(&e.param_path, "1").unwrap_or_else(|f| panic!("{}: {f}", e.param_path));
        assert_eq!(back, e.role, "{}", e.param_path);
    }
}

#[test]
fn every_role_of_a_conformance_case_goes_to_its_param_path_and_back() {
    let root = common::repo().join("conformance/0.6/read");
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in std::fs::read_dir(&root).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        if spec.get("result").is_none() || spec.get("roles").is_none() {
            continue;
        }
        let text = std::fs::read_to_string(case.join("descriptor.json")).unwrap();
        let d = osd::read(&text, Kind::Device).unwrap();
        for e in osd::roles(&d) {
            checked += 1;
            match osd::role_from_param_path(&e.param_path, "1") {
                Ok(back) if back == e.role => {}
                other => failures.push(format!("{}: {} read back as {other:?}", case.display(), e.param_path)),
            }
        }
    }
    assert!(checked >= 4, "{checked} roles");
    common::report("roles fail", failures);
}

#[test]
fn a_segment_that_is_no_known_term_no_extension_and_no_number_is_refused_at_its_place() {
    // The pointer is the position of the segment, from 0, among the segments.
    for (path, pointer) in [
        ("/Gizmo/Cutoff", "/0"),
        ("/Filter/Wobble", "/1"),
        ("/filter/cutoff", "/0"),
        ("/Filter/cutoff", "/1"),
        ("/Filter/ATTACKTIME", "/1"),
        ("/Filter/Envelope/Attack-Time", "/2"),
        ("/Filter/x-acme", "/1"),
        ("/Filter/X-acme-wobble", "/1"),
        ("/Filter//Cutoff", "/1"),
    ] {
        let f = osd::role_from_param_path(path, "1").err().unwrap_or_else(|| panic!("{path} accepted"));
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{path}: {f}");
    }
}

#[test]
fn a_number_is_read_only_as_the_index_of_the_part_before_it() {
    for path in ["/2/Filter/Cutoff", "/Filter/2/3/Cutoff", "/Filter/0/Cutoff", "/Filter/Cutoff/2", "/Filter/-1/Cutoff", "/Filter/1.5/Cutoff"] {
        assert!(osd::role_from_param_path(path, "1").is_err(), "{path} accepted");
    }
    let f = osd::role_from_param_path("/2/Filter/Cutoff", "1").unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/0"), "{f}");
    let f = osd::role_from_param_path("/Filter/2/3/Cutoff", "1").unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/2"), "{f}");
    let f = osd::role_from_param_path("/Filter/0/Cutoff", "1").unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/1"), "{f}");
}

#[test]
fn a_text_that_is_not_a_path_of_at_least_one_part_and_a_setting_is_refused() {
    for path in ["", "/", "Filter/Cutoff", "/Cutoff", "/Filter", "/Filter/Cutoff/", "//Filter/Cutoff"] {
        assert!(osd::role_from_param_path(path, "1").is_err(), "{path:?} accepted");
    }
    assert!(osd::role_from_param_path("/Filter/Cutoff", "999").is_err(), "a state osd does not hold");
}
