//! A virtual instrument says what it stands for and how true it is (0.6): a software device may
//! give `device.stands_for`, the catalogue address `maker/device` of the device it stands for, and
//! `device.fidelity`, one of `faithful`, `modelled`, `principle`. `fidelity` needs `stands_for`,
//! and both need `kind` to be `software`. The reader checks the form, the resolver the target.

use crate::common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn doc(device: Value) -> Value {
    let mut d = json!({"name": "Emulation", "osc_prefix": "/emu"});
    d.as_object_mut().unwrap().extend(device.as_object().unwrap().clone());
    json!({"$schema": SCHEMA, "device": d})
}

fn read(doc: &Value) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn written(d: &osd::Descriptor) -> Value {
    serde_json::from_str(&osd::write(d)).unwrap()
}

fn refused_at(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

fn schema_errors(doc: &Value) -> Vec<(String, String)> {
    common::Schemas::load("0.6").errors(Kind::Device, doc)
}

#[test]
fn a_software_device_reads_with_what_it_stands_for_and_each_of_the_three_fidelities() {
    for fidelity in ["faithful", "modelled", "principle"] {
        let d = doc(json!({"kind": "software", "stands_for": "korg/monologue", "fidelity": fidelity}));
        let got = written(&read(&d).unwrap_or_else(|f| panic!("{fidelity}: {f}")));
        assert_eq!(got["device"]["stands_for"], "korg/monologue", "{fidelity}");
        assert_eq!(got["device"]["fidelity"], fidelity);
        assert_eq!(got, d);
        assert!(schema_errors(&d).is_empty(), "{fidelity}: {:?}", schema_errors(&d));
    }
}

#[test]
fn a_device_stands_for_another_of_another_maker_by_its_catalogue_address() {
    let d = doc(json!({"kind": "software", "stands_for": "roland/tb-303", "fidelity": "modelled"}));
    assert_eq!(written(&read(&d).unwrap())["device"]["stands_for"], "roland/tb-303");
}

#[test]
fn stands_for_is_read_without_a_fidelity() {
    let d = doc(json!({"kind": "software", "stands_for": "korg/monologue"}));
    let got = written(&read(&d).unwrap());
    assert_eq!(got["device"]["stands_for"], "korg/monologue");
    assert!(got["device"].get("fidelity").is_none());
    assert!(schema_errors(&d).is_empty());
}

#[test]
fn a_software_device_that_stands_for_nothing_reads_without_either_field() {
    let d = doc(json!({"kind": "software"}));
    let got = written(&read(&d).unwrap());
    assert_eq!(got, d);
    assert!(got["device"].get("stands_for").is_none() && got["device"].get("fidelity").is_none());
    assert!(schema_errors(&d).is_empty());
}

#[test]
fn a_fidelity_without_what_it_stands_for_is_refused_at_the_fidelity_in_the_reader_and_the_schema() {
    let d = doc(json!({"kind": "software", "fidelity": "faithful"}));
    refused_at(&d, FaultKind::BadValue, "/device/fidelity");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn stands_for_on_a_hardware_device_is_refused_at_the_field_in_the_reader_and_the_schema() {
    let d = doc(json!({"kind": "hardware", "stands_for": "korg/monologue"}));
    refused_at(&d, FaultKind::BadValue, "/device/stands_for");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_fidelity_on_a_hardware_device_is_refused_at_the_field() {
    let d = doc(json!({"kind": "hardware", "stands_for": "korg/monologue", "fidelity": "faithful"}));
    let f = read(&d).expect_err("refused");
    assert_eq!(f.kind, FaultKind::BadValue, "{f}");
    assert!(f.pointer == "/device/stands_for" || f.pointer == "/device/fidelity", "{f}");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn stands_for_on_a_device_that_gives_no_kind_is_refused_at_the_field_in_the_reader_and_the_schema() {
    let d = doc(json!({"stands_for": "korg/monologue"}));
    refused_at(&d, FaultKind::BadValue, "/device/stands_for");
    assert!(!schema_errors(&d).is_empty());
    let d = doc(json!({"stands_for": "korg/monologue", "fidelity": "modelled"}));
    assert!(read(&d).is_err());
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_stands_for_that_is_not_maker_slash_device_is_refused_at_the_field_in_the_reader_and_the_schema() {
    for bad in ["monologue", "korg/", "/monologue", "korg/mono/logue","korg monologue", "", "korg/{x}"] {
        let d = doc(json!({"kind": "software", "stands_for": bad}));
        let f = read(&d).expect_err(bad);
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/device/stands_for"), "{bad}: {f}");
        assert!(!schema_errors(&d).is_empty(), "{bad}");
    }
    let d = doc(json!({"kind": "software", "stands_for": 7}));
    refused_at(&d, FaultKind::BadValue, "/device/stands_for");
}

#[test]
fn a_fidelity_outside_the_three_words_is_refused_at_the_field_in_the_reader_and_the_schema() {
    for bad in ["exact", "Faithful", "good", ""] {
        let d = doc(json!({"kind": "software", "stands_for": "korg/monologue", "fidelity": bad}));
        let f = read(&d).expect_err(bad);
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/device/fidelity"), "{bad}: {f}");
        assert!(!schema_errors(&d).is_empty(), "{bad}");
    }
}

#[test]
fn a_device_of_0_4_or_0_5_refuses_both_fields_as_unknown() {
    // A 0.4 text names no `$schema` (and no `kind`, which 0.5 adds); a 0.5 text names its own.
    for version in ["0.4", "0.5"] {
        for field in ["stands_for", "fidelity"] {
            let value = if field == "stands_for" { "korg/monologue" } else { "faithful" };
            let d = if version == "0.4" {
                json!({"device": {"name": "E", "osc_prefix": "/e", field: value}})
            } else {
                json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "E", "osc_prefix": "/e", "kind": "software", field: value}})
            };
            let f = read(&d).expect_err("refused");
            assert_eq!((f.kind, f.pointer.clone()), (FaultKind::UnknownField, format!("/device/{field}")), "{version}: {f}");
        }
    }
}

#[test]
fn converting_a_0_5_software_device_adds_neither_field() {
    let text = json!({
        "$schema": "https://w3id.org/osd/0.5/device.schema.json",
        "device": {"name": "Plugin", "osc_prefix": "/plug", "kind": "software"}
    })
    .to_string();
    for out in [osd::convert(&text, Kind::Device).unwrap(), osd::convert_to(&text, Kind::Device, Version::V0_6).unwrap()] {
        let got: Value = serde_json::from_str(&out).unwrap();
        assert!(got["device"].get("stands_for").is_none() && got["device"].get("fidelity").is_none(), "{got}");
    }
}

fn catalogue(files: &[(&str, Value)]) -> BTreeMap<String, String> {
    files.iter().map(|(id, d)| (id.to_string(), d.to_string())).collect()
}

fn hardware() -> Value {
    doc(json!({"name": "Monologue", "kind": "hardware"}))
}

#[test]
fn a_resolved_device_gives_what_it_stands_for_and_its_fidelity_as_written() {
    let files = catalogue(&[
        ("korg/monologue", hardware()),
        ("acme/emu", doc(json!({"kind": "software", "stands_for": "korg/monologue", "fidelity": "modelled"}))),
    ]);
    let r = osd::resolve("acme/emu", &files).unwrap_or_else(|f| panic!("{f}"));
    let got = written(&osd::Descriptor::Device(Box::new(r.device)));
    assert_eq!(got["device"]["stands_for"], "korg/monologue");
    assert_eq!(got["device"]["fidelity"], "modelled");
}

#[test]
fn resolving_a_device_that_stands_for_an_address_absent_from_the_catalogue_is_refused_at_the_field() {
    let files = catalogue(&[("acme/emu", doc(json!({"kind": "software", "stands_for": "korg/monologue", "fidelity": "faithful"})))]);
    let f = osd::resolve("acme/emu", &files).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/device/stands_for"), "{f}");
}

#[test]
fn a_software_device_may_stand_for_another_software_device() {
    let files = catalogue(&[
        ("acme/base", doc(json!({"kind": "software"}))),
        ("acme/emu", doc(json!({"kind": "software", "stands_for": "acme/base", "fidelity": "principle"}))),
    ]);
    let r = osd::resolve("acme/emu", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(written(&osd::Descriptor::Device(Box::new(r.device)))["device"]["stands_for"], "acme/base");
}

#[test]
fn a_software_device_that_stands_for_nothing_resolves_without_looking_for_a_target() {
    let files = catalogue(&[("acme/daw", doc(json!({"kind": "software"})))]);
    assert!(osd::resolve("acme/daw", &files).is_ok());
}
