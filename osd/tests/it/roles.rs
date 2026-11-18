//! The role of a control (0.6): an optional object `role {parts: [{part, index?}…],
//! setting}` on a CC, NRPN or RPN entry and on a param entry: a path of one part or more
//! (each a term of the vocabulary state the descriptor names in `role_vocabulary`, with
//! its number when the instrument has several) ending in one setting. A term may be an
//! extension `x-<registrant>-<term>`. `osd::roles` resolves each role of a descriptor to
//! `RoleMeaning::Known` (every term known) or `RoleMeaning::NoKnownRole` (an extension);
//! a part says under which parts it may stand (`stands_under`, `at_top`), a setting on
//! which parts it applies (`applies_to`); nothing is written per role.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, RoleMeaning};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";
const ROLE: &str = "/cc_params/entries/0/role";

fn cutoff() -> Value {
    json!({"parts": [{"part": "filter"}], "setting": "cutoff"})
}

/// A role written as a path: the parts (each with an optional index) and the setting.
fn path(parts: &[(&str, Option<i64>)], setting: &str) -> Value {
    let parts: Vec<Value> = parts
        .iter()
        .map(|(p, i)| match i {
            Some(i) => json!({"part": p, "index": i}),
            None => json!({"part": p}),
        })
        .collect();
    json!({"parts": parts, "setting": setting})
}

/// A 0.6 device whose one CC entry carries `role`, naming the state `vocab`.
fn with_cc_role(role: Value, vocab: Option<&str>) -> Value {
    let mut doc = json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/filter/cutoff", "cc": 74, "role": role}]}
    });
    if let Some(v) = vocab {
        doc["role_vocabulary"] = json!(v);
    }
    doc
}

fn read(doc: &Value) -> Result<Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).err().unwrap_or_else(|| panic!("accepted: {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

fn refused_role(role: Value, kind: FaultKind, pointer: &str) {
    refused(&with_cc_role(role, Some("1")), kind, pointer);
}

fn meanings(doc: &Value) -> Vec<(String, RoleMeaning)> {
    osd::roles(&read(doc).unwrap_or_else(|f| panic!("{f}"))).into_iter().map(|r| (r.pointer, r.meaning)).collect()
}

fn known(role: Value) -> bool {
    meanings(&with_cc_role(role, Some("1"))) == vec![(ROLE.to_string(), RoleMeaning::Known)]
}

#[test]
fn a_descriptor_without_any_role_reads_as_before_and_has_no_role_to_resolve() {
    let text = std::fs::read_to_string(common::repo().join("examples/0.5/synth.device.json"))
        .unwrap()
        .replace("https://w3id.org/osd/0.5/", "https://w3id.org/osd/0.6/");
    let d = osd::read(&text, Kind::Device).unwrap();
    assert!(osd::roles(&d).is_empty());
}

#[test]
fn a_descriptor_may_name_a_vocabulary_state_without_carrying_any_role() {
    let doc = json!({"$schema": SCHEMA, "role_vocabulary": "1", "device": {"name": "T", "osc_prefix": "/t"}});
    assert!(read(&doc).is_ok());
}

#[test]
fn a_role_that_is_a_path_of_known_terms_resolves_to_a_known_role_and_writes_back_as_read() {
    for role in [
        cutoff(),
        path(&[("filter", None), ("envelope", None)], "attack-time"),
        path(&[("amplifier", None), ("envelope", None)], "attack-time"),
        path(&[("oscillator", Some(2))], "pitch"),
    ] {
        let doc = with_cc_role(role.clone(), Some("1"));
        assert_eq!(meanings(&doc), vec![(ROLE.to_string(), RoleMeaning::Known)], "{role}");
        let back: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
        assert_eq!(back, doc, "{role}");
    }
}

#[test]
fn the_attack_of_the_filters_envelope_and_the_attack_of_the_amplifiers_are_two_distinct_roles() {
    let filter = path(&[("filter", None), ("envelope", None)], "attack-time");
    let amplifier = path(&[("amplifier", None), ("envelope", None)], "attack-time");
    assert_ne!(filter, amplifier);
    let doc = json!({
        "$schema": SCHEMA,
        "role_vocabulary": "1",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [
            {"osc": "/fenv/attack", "cc": 20, "role": filter},
            {"osc": "/aenv/attack", "cc": 21, "role": amplifier}
        ]}
    });
    let paths: Vec<String> = osd::roles(&read(&doc).unwrap()).into_iter().map(|r| r.param_path).collect();
    assert_eq!(paths, ["/Filter/Envelope/AttackTime", "/Amplifier/Envelope/AttackTime"]);
}

#[test]
fn each_part_of_a_path_carries_its_own_index() {
    assert!(known(path(&[("oscillator", Some(2)), ("envelope", Some(1))], "attack-time")));
    assert!(known(path(&[("oscillator", Some(2)), ("envelope", None)], "attack-time")));
    for index in [1, 2, 16] {
        assert!(known(path(&[("filter", Some(index))], "cutoff")), "index {index}");
        assert!(known(path(&[("effect", Some(index))], "depth")), "index {index}");
    }
    let back: Value =
        serde_json::from_str(&osd::write(&read(&with_cc_role(path(&[("oscillator", Some(2)), ("envelope", Some(1))], "attack-time"), Some("1"))).unwrap())).unwrap();
    assert_eq!(back["cc_params"]["entries"][0]["role"]["parts"][1]["index"], json!(1));
}

#[test]
fn other_known_paths_compose_as_well() {
    for (parts, setting) in [
        (vec![("voice", None)], "portamento-time"),
        (vec![("output", None)], "pan"),
        (vec![("voice", None), ("envelope", None)], "release-time"),
        (vec![("oscillator", Some(2))], "pitch"),
    ] {
        assert!(known(path(&parts, setting)), "{parts:?} {setting}");
    }
}

#[test]
fn an_entry_of_each_control_form_accepts_a_role() {
    let role = cutoff();
    let doc = json!({
        "$schema": SCHEMA,
        "role_vocabulary": "1",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [
            {"osc": "/a", "cc": 74, "role": role},
            {"osc": "/b", "cc": 74, "cc_lsb": 106, "role": role},
            {"osc": "/c", "nrpn_msb": 1, "nrpn_lsb": 2, "role": role},
            {"osc": "/d", "rpn_msb": 0, "rpn_lsb": 1, "role": role}
        ]},
        "params": {
            "get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7",
            "entries": [{"osc": "/e", "id": 3, "role": role}]
        }
    });
    let got = meanings(&doc);
    let pointers: Vec<&str> = got.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        pointers,
        [
            "/cc_params/entries/0/role",
            "/cc_params/entries/1/role",
            "/cc_params/entries/2/role",
            "/cc_params/entries/3/role",
            "/params/entries/0/role"
        ]
    );
    assert!(got.iter().all(|(_, m)| *m == RoleMeaning::Known));
}

#[test]
fn a_role_with_an_extension_is_valid_and_reads_as_no_known_role() {
    for role in [
        path(&[("filter", None)], "x-acme-wobble"),
        path(&[("x-acme-ladder", None)], "x-acme-drive-2"),
        // The extension stands in for a term, so the part/setting match is not checked.
        path(&[("x-acme-ladder", None)], "cutoff"),
        // An extension in the middle of a path.
        path(&[("filter", None), ("x-acme-ladder", None), ("envelope", None)], "attack-time"),
        path(&[("x-acme-ladder", Some(2)), ("envelope", None)], "attack-time"),
        path(&[("filter", None), ("x-acme-ladder", None)], "cutoff"),
    ] {
        let doc = with_cc_role(role.clone(), Some("1"));
        assert_eq!(meanings(&doc), vec![(ROLE.to_string(), RoleMeaning::NoKnownRole)], "{role}");
    }
}

#[test]
fn an_extension_that_does_not_follow_the_registered_form_is_refused_at_its_term() {
    for bad in ["x-acme", "X-acme-wobble", "x--wobble", "x-acme-Wobble", "x-a_b-c"] {
        refused_role(path(&[("filter", None)], bad), FaultKind::BadValue, &format!("{ROLE}/setting"));
        refused_role(path(&[("filter", None), (bad, None)], "cutoff"), FaultKind::BadValue, &format!("{ROLE}/parts/1/part"));
    }
}

#[test]
fn a_term_neither_known_nor_an_extension_is_refused_at_its_term() {
    refused_role(path(&[("filter", None)], "wobble"), FaultKind::BadValue, &format!("{ROLE}/setting"));
    refused_role(path(&[("gizmo", None)], "cutoff"), FaultKind::BadValue, &format!("{ROLE}/parts/0/part"));
    refused_role(path(&[("filter", None), ("gizmo", None)], "cutoff"), FaultKind::BadValue, &format!("{ROLE}/parts/1/part"));
}

#[test]
fn a_term_used_in_the_wrong_slot_is_refused_at_the_faulty_field() {
    // `cutoff` is a setting, `filter` a part.
    refused_role(path(&[("cutoff", None)], "cutoff"), FaultKind::BadValue, &format!("{ROLE}/parts/0/part"));
    refused_role(path(&[("filter", None), ("cutoff", None)], "cutoff"), FaultKind::BadValue, &format!("{ROLE}/parts/1/part"));
    refused_role(path(&[("filter", None)], "filter"), FaultKind::BadValue, &format!("{ROLE}/setting"));
}

#[test]
fn a_path_whose_first_part_cannot_open_a_path_is_refused_at_that_part() {
    // `envelope` stands under other parts: it does not open a path.
    refused_role(path(&[("envelope", None)], "attack-time"), FaultKind::BadValue, &format!("{ROLE}/parts/0"));
    // An index does not change where a part may stand.
    refused_role(path(&[("envelope", Some(1))], "attack-time"), FaultKind::BadValue, &format!("{ROLE}/parts/0"));
    // `lfo` opens a path and stands under others: both are written, both hold.
    assert!(known(path(&[("lfo", None)], "rate")));
}

#[test]
fn a_part_that_does_not_stand_under_the_one_before_it_is_refused_at_that_part() {
    // `filter` stands under no part, `envelope` stands under filter, amplifier, oscillator, voice.
    refused_role(path(&[("oscillator", None), ("filter", None)], "cutoff"), FaultKind::BadValue, &format!("{ROLE}/parts/1"));
    refused_role(path(&[("noise", None), ("envelope", None)], "attack-time"), FaultKind::BadValue, &format!("{ROLE}/parts/1"));
    refused_role(path(&[("output", None), ("envelope", None)], "attack-time"), FaultKind::BadValue, &format!("{ROLE}/parts/1"));
    // The fault names the first part that breaks the rule.
    refused_role(path(&[("filter", None), ("envelope", None), ("filter", None)], "cutoff"), FaultKind::BadValue, &format!("{ROLE}/parts/2"));
    // A part may stand under a part that itself stands under another.
    assert!(known(path(&[("filter", None), ("lfo", None)], "rate")));
    assert!(known(path(&[("oscillator", Some(2)), ("lfo", Some(1))], "rate")));
    // `lfo` stands under oscillator, filter and amplifier only.
    refused_role(path(&[("output", None), ("lfo", None)], "rate"), FaultKind::BadValue, &format!("{ROLE}/parts/1"));
}

#[test]
fn a_setting_that_does_not_apply_to_the_last_part_is_refused_at_the_role() {
    refused_role(path(&[("filter", None)], "pan"), FaultKind::BadValue, ROLE);
    refused_role(path(&[("output", None)], "cutoff"), FaultKind::BadValue, ROLE);
    // Only the last part is held against `applies_to`: cutoff applies to the filter, not to its envelope.
    refused_role(path(&[("filter", None), ("envelope", None)], "cutoff"), FaultKind::BadValue, ROLE);
    // Attack time applies to the envelope, not to the filter that carries it.
    refused_role(path(&[("filter", None)], "attack-time"), FaultKind::BadValue, ROLE);
}

#[test]
fn a_malformed_role_is_refused_at_the_faulty_value() {
    let v = Some("1");
    let r = |role: Value, kind: FaultKind, pointer: &str| refused(&with_cc_role(role, v), kind, pointer);
    r(json!("filter-cutoff"), FaultKind::BadValue, ROLE);
    r(json!("/Filter/Cutoff"), FaultKind::BadValue, ROLE);
    r(json!(["filter", "cutoff"]), FaultKind::BadValue, ROLE);
    r(json!({"parts": [{"part": "filter"}], "setting": "cutoff", "colour": "red"}), FaultKind::UnknownField, &format!("{ROLE}/colour"));
    r(json!({"parts": [{"part": "filter", "colour": "red"}], "setting": "cutoff"}), FaultKind::UnknownField, &format!("{ROLE}/parts/0/colour"));
    for bad in [json!(0), json!(-1), json!(1.5), json!("2")] {
        r(json!({"parts": [{"part": "filter", "index": bad}], "setting": "cutoff"}), FaultKind::BadValue, &format!("{ROLE}/parts/0/index"));
        r(
            json!({"parts": [{"part": "filter"}, {"part": "envelope", "index": bad}], "setting": "attack-time"}),
            FaultKind::BadValue,
            &format!("{ROLE}/parts/1/index"),
        );
    }
    r(json!({"parts": [{"part": "filter"}]}), FaultKind::Missing, &format!("{ROLE}/setting"));
    r(json!({"setting": "cutoff"}), FaultKind::Missing, &format!("{ROLE}/parts"));
    r(json!({"parts": [{"index": 1}], "setting": "cutoff"}), FaultKind::Missing, &format!("{ROLE}/parts/0/part"));
    r(json!({"parts": [{"part": 3}], "setting": "cutoff"}), FaultKind::BadValue, &format!("{ROLE}/parts/0/part"));
    r(json!({"parts": "filter", "setting": "cutoff"}), FaultKind::BadValue, &format!("{ROLE}/parts"));
    r(json!({"parts": ["filter"], "setting": "cutoff"}), FaultKind::BadValue, &format!("{ROLE}/parts/0"));
    r(json!({"parts": [{"part": "filter"}], "setting": 4}), FaultKind::BadValue, &format!("{ROLE}/setting"));
}

#[test]
fn a_role_with_no_part_is_refused_at_its_list_of_parts() {
    refused_role(json!({"parts": [], "setting": "cutoff"}), FaultKind::BadValue, &format!("{ROLE}/parts"));
}

#[test]
fn the_flat_form_of_a_part_an_index_and_a_setting_is_no_longer_a_role() {
    refused_role(json!({"part": "filter", "setting": "cutoff"}), FaultKind::UnknownField, &format!("{ROLE}/part"));
    refused_role(json!({"part": "filter", "index": 2, "setting": "cutoff"}), FaultKind::UnknownField, &format!("{ROLE}/part"));
}

#[test]
fn a_role_without_the_vocabulary_state_it_was_written_with_is_refused() {
    refused(&with_cc_role(cutoff(), None), FaultKind::Missing, "/role_vocabulary");
}

#[test]
fn a_vocabulary_state_osd_does_not_hold_is_refused_at_the_field() {
    refused(&with_cc_role(cutoff(), Some("999")), FaultKind::BadValue, "/role_vocabulary");
    refused(&with_cc_role(cutoff(), Some("")), FaultKind::BadValue, "/role_vocabulary");
    let mut numeric = with_cc_role(cutoff(), None);
    numeric["role_vocabulary"] = json!(1);
    refused(&numeric, FaultKind::BadValue, "/role_vocabulary");
    // Named without a role, an unknown state is refused as well.
    let doc = json!({"$schema": SCHEMA, "role_vocabulary": "999", "device": {"name": "T", "osc_prefix": "/t"}});
    refused(&doc, FaultKind::BadValue, "/role_vocabulary");
}

#[test]
fn a_role_written_elsewhere_than_on_a_control_is_refused_where_it_stands() {
    let base = |at: &str| -> Value {
        let mut doc = json!({
            "$schema": SCHEMA,
            "role_vocabulary": "1",
            "device": {"name": "T", "osc_prefix": "/t"},
            "midi_out": {},
            "triggers": {"entries": [{"osc": "/kick", "sound": "kick", "note_range": [36, 36]}]}
        });
        match at {
            "/device/role" => doc["device"]["role"] = cutoff(),
            "/midi_out/role" => doc["midi_out"]["role"] = cutoff(),
            "/triggers/entries/0/role" => doc["triggers"]["entries"][0]["role"] = cutoff(),
            "/role" => doc["role"] = cutoff(),
            _ => unreachable!(),
        }
        doc
    };
    for at in ["/device/role", "/midi_out/role", "/triggers/entries/0/role", "/role"] {
        refused(&base(at), FaultKind::UnknownField, at);
    }
}

#[test]
fn the_0_6_device_schema_states_the_role_as_a_path_and_requires_its_vocabulary_state() {
    let schemas = common::Schemas::load("0.6");
    let errors = |doc: &Value| schemas.errors(Kind::Device, doc);
    let at = |doc: &Value| -> Vec<String> {
        let mut paths: Vec<String> = errors(doc).into_iter().map(|(p, _)| p).collect();
        paths.dedup();
        paths
    };
    assert!(errors(&with_cc_role(cutoff(), Some("1"))).is_empty());
    assert!(errors(&with_cc_role(path(&[("oscillator", Some(2)), ("envelope", None)], "x-acme-wobble"), Some("1"))).is_empty());
    // A role the reader refuses for the vocabulary alone is a descriptor the schema accepts.
    assert!(errors(&with_cc_role(path(&[("filter", None)], "pan"), Some("1"))).is_empty());
    assert!(errors(&with_cc_role(path(&[("envelope", None)], "attack-time"), Some("1"))).is_empty());
    assert!(errors(&with_cc_role(path(&[("oscillator", None), ("filter", None)], "cutoff"), Some("1"))).is_empty());
    assert_eq!(at(&with_cc_role(json!("filter-cutoff"), Some("1"))), [ROLE]);
    assert_eq!(at(&with_cc_role(path(&[("filter", Some(0))], "cutoff"), Some("1"))), [format!("{ROLE}/parts/0/index")]);
    assert_eq!(at(&with_cc_role(cutoff(), None)), [""], "the missing field is reported at the object that lacks it");
    // The flat form and an empty list of parts are refused.
    assert_eq!(at(&with_cc_role(json!({"part": "filter", "setting": "cutoff"}), Some("1"))), [ROLE]);
    assert_eq!(at(&with_cc_role(json!({"part": "filter", "index": 2, "setting": "cutoff"}), Some("1"))), [ROLE]);
    assert_eq!(at(&with_cc_role(json!({"parts": [], "setting": "cutoff"}), Some("1"))), [format!("{ROLE}/parts")]);
    let mut doc = with_cc_role(cutoff(), Some("1"));
    doc["cc_params"]["entries"][0]["role"]["colour"] = json!("red");
    assert_eq!(at(&doc), [ROLE]);
    // A role elsewhere than on a control.
    let mut elsewhere = with_cc_role(cutoff(), Some("1"));
    elsewhere["device"]["role"] = cutoff();
    assert_eq!(at(&elsewhere), ["/device"]);
}

#[test]
fn the_0_5_schema_and_reader_do_not_know_the_role() {
    let mut doc = with_cc_role(cutoff(), Some("1"));
    doc["$schema"] = json!("https://w3id.org/osd/0.5/device.schema.json");
    let f = read(&doc).unwrap_err();
    assert_eq!(f.kind, FaultKind::UnknownField, "{f}");
    assert!(!common::Schemas::load("0.5").errors(Kind::Device, &doc).is_empty());
}

fn files(driver: &Value) -> std::collections::BTreeMap<String, String> {
    [("acme/synth".to_string(), driver.to_string())].into_iter().collect()
}

#[test]
fn resolving_keeps_each_role_as_written_and_the_state_the_descriptor_names() {
    let role = path(&[("oscillator", Some(2)), ("envelope", None)], "attack-time");
    let doc = with_cc_role(role.clone(), Some("1"));
    let r = osd::resolve("acme/synth", &files(&doc)).unwrap();
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device.clone())))).unwrap();
    assert_eq!(out["role_vocabulary"], json!("1"));
    assert_eq!(out["cc_params"]["entries"][0]["role"], role);
    let got: Vec<(String, RoleMeaning)> = osd::roles(&Descriptor::Device(Box::new(r.device))).into_iter().map(|r| (r.pointer, r.meaning)).collect();
    assert_eq!(got, vec![(ROLE.to_string(), RoleMeaning::Known)]);
}

#[test]
fn resolving_a_driver_whose_role_is_refused_is_refused_with_its_file() {
    let doc = with_cc_role(cutoff(), Some("999"));
    let f = osd::resolve("acme/synth", &files(&doc)).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str(), f.file.as_deref()), (FaultKind::BadValue, "/role_vocabulary", Some("acme/synth")), "{f}");
    let doc = with_cc_role(path(&[("envelope", None)], "attack-time"), Some("1"));
    let f = osd::resolve("acme/synth", &files(&doc)).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str(), f.file.as_deref()), (FaultKind::BadValue, "/cc_params/entries/0/role/parts/0", Some("acme/synth")), "{f}");
}

#[test]
fn a_profile_and_the_driver_that_links_it_are_each_read_against_their_own_file() {
    let profile = json!({
        "$schema": "https://w3id.org/osd/0.6/profile.schema.json",
        "identity": {"request": [240, 126, 127, 6, 1, 247], "reply": [240, 126, 0, 6, 2, 67]}
    });
    let mut driver = with_cc_role(cutoff(), Some("1"));
    driver["identity"] = json!({"profile": "acme"});
    let mut files = files(&driver);
    files.insert("acme/_profile".into(), profile.to_string());
    let r = osd::resolve("acme/synth", &files).unwrap();
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap();
    assert_eq!(out["role_vocabulary"], json!("1"));
    assert_eq!(out["cc_params"]["entries"][0]["role"], cutoff());
}

#[test]
fn converting_a_0_5_descriptor_adds_no_role_and_the_result_converts_to_itself() {
    let root = common::repo();
    let cases = [
        (Kind::Device, "examples/0.5/synth.device.json"),
        (Kind::Device, "examples/devices/test_synth.json"),
        (Kind::Profile, "examples/0.5/acme.profile.json"),
    ];
    for (kind, path) in cases {
        let text = std::fs::read_to_string(root.join(path)).unwrap();
        let out = osd::convert_to(&text, kind, osd::Version::V0_6).unwrap();
        assert!(!out.contains("\"role\"") && !out.contains("role_vocabulary"), "{path}");
        assert_eq!(osd::convert_to(&out, kind, osd::Version::V0_6).unwrap(), out, "{path}");
        let again = osd::read(&out, kind).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&osd::write(&again)).unwrap(), serde_json::from_str::<Value>(&out).unwrap(), "{path}");
    }
}
