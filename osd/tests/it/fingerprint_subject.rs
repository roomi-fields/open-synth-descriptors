//! A 0.6 fingerprint always names what it was taken on: a device (a catalogue descriptor, or the
//! maker and model the person declares, each with its firmware) or a program (a firmware or an
//! application, in the words of its card), exactly one of the two. One that names neither is
//! refused by the schema and the reader; a 0.5 fingerprint without a named device is not converted.

use crate::common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};

const FINGERPRINT: &str = "https://w3id.org/osd/0.6/fingerprint.schema.json";

fn fingerprint(subject: Value) -> Value {
    let mut doc = json!({"$schema": FINGERPRINT, "observed": "2026-11-22", "ports": {"inputs": ["usb midi interface"]}});
    for (k, v) in subject.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn accepted(doc: &Value) {
    let got = osd::read(&doc.to_string(), Kind::Fingerprint).unwrap_or_else(|f| panic!("{doc}: {f}"));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&got)).unwrap(), *doc);
    let errors = common::Schemas::load("0.6").errors(Kind::Fingerprint, doc);
    assert!(errors.is_empty(), "{doc}: {errors:?}");
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = osd::read(&doc.to_string(), Kind::Fingerprint).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{doc}: {f}");
    assert!(!common::Schemas::load("0.6").errors(Kind::Fingerprint, doc).is_empty(), "the schema accepts {doc}");
}

#[test]
fn a_fingerprint_names_a_catalogue_device_by_its_descriptor_and_its_firmware() {
    accepted(&fingerprint(json!({"device": {"descriptor": "korg/minilogue", "firmware": "2.01"}})));
    accepted(&fingerprint(json!({"device": {"descriptor": "elektron/digitone", "firmware": "1.43"}})));
}

#[test]
fn a_fingerprint_names_a_device_the_person_declares_by_maker_model_and_firmware() {
    accepted(&fingerprint(json!({"device": {"vendor": "Example", "model": "Mono One", "firmware": "1.0"}})));
    accepted(&fingerprint(json!({"device": {"vendor": "Other", "model": "Poly Two", "firmware": "v3 beta"}})));
}

#[test]
fn a_fingerprint_names_a_program_by_its_name_its_version_and_its_kind() {
    accepted(&fingerprint(json!({"program": {"name": "Elekloader mod", "version": "2.0", "kind": "firmware"}})));
    accepted(&fingerprint(json!({"program": {"name": "Grainer", "version": "1.2", "kind": "application"}})));
    accepted(&fingerprint(json!({"program": {"name": "Grainer", "version": "1.2"}})));
}

#[test]
fn a_fingerprint_naming_no_subject_is_refused_as_missing_its_device_by_both() {
    refused(&fingerprint(json!({})), FaultKind::Missing, "/device");
    let observed_only = json!({"$schema": FINGERPRINT, "observed": "2026-11-22"});
    refused(&observed_only, FaultKind::Missing, "/device");
}

#[test]
fn a_device_without_its_firmware_is_refused_at_the_firmware() {
    refused(&fingerprint(json!({"device": {"descriptor": "korg/minilogue"}})), FaultKind::Missing, "/device/firmware");
    refused(&fingerprint(json!({"device": {"vendor": "Example", "model": "Mono One"}})), FaultKind::Missing, "/device/firmware");
}

#[test]
fn a_declared_device_lacking_its_maker_or_its_model_is_refused() {
    refused(&fingerprint(json!({"device": {"vendor": "Example", "firmware": "1.0"}})), FaultKind::Missing, "/device/descriptor");
    refused(&fingerprint(json!({"device": {"model": "Mono One", "firmware": "1.0"}})), FaultKind::Missing, "/device/descriptor");
}

#[test]
fn a_program_without_its_name_or_its_version_is_refused_at_the_missing_word() {
    refused(&fingerprint(json!({"program": {"version": "1.2"}})), FaultKind::Missing, "/program/name");
    refused(&fingerprint(json!({"program": {"name": "Grainer"}})), FaultKind::Missing, "/program/version");
}

#[test]
fn a_program_of_an_unknown_kind_is_refused_at_its_kind() {
    refused(&fingerprint(json!({"program": {"name": "Grainer", "version": "1.2", "kind": "gadget"}})), FaultKind::BadValue, "/program/kind");
}

#[test]
fn a_fingerprint_naming_a_device_and_a_program_is_refused() {
    let both = json!({"device": {"descriptor": "korg/minilogue", "firmware": "2.01"}, "program": {"name": "Grainer", "version": "1.2"}});
    refused(&fingerprint(both), FaultKind::BadValue, "/program");
}

fn fingerprint_05(device: Option<Value>) -> String {
    let mut doc = json!({"$schema": "https://w3id.org/osd/0.5/fingerprint.schema.json", "observed": "2026-11-22", "controls": [{"cc": 74}]});
    if let Some(d) = device {
        doc["device"] = d;
    }
    doc.to_string()
}

#[test]
fn a_0_5_fingerprint_without_a_device_is_refused_by_the_conversion_and_stays_readable_as_0_5() {
    let text = fingerprint_05(None);
    let f = osd::convert_to(&text, Kind::Fingerprint, Version::V0_6).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::Missing, "/device"), "{f}");
    assert!(osd::read(&text, Kind::Fingerprint).is_ok());
}

#[test]
fn a_0_5_fingerprint_whose_device_holds_no_firmware_is_refused_by_the_conversion() {
    for device in [json!({"descriptor": "korg/minilogue"}), json!({"vendor": "Example", "model": "Mono One"})] {
        let f = osd::convert_to(&fingerprint_05(Some(device)), Kind::Fingerprint, Version::V0_6).expect_err("refused");
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::Missing, "/device/firmware"), "{f}");
    }
}

#[test]
fn a_0_5_fingerprint_whose_device_holds_its_firmware_converts_with_what_it_holds() {
    for device in [json!({"descriptor": "korg/minilogue", "firmware": "2.01"}), json!({"vendor": "Example", "model": "Mono One", "firmware": "1.0"})] {
        let out = osd::convert_to(&fingerprint_05(Some(device.clone())), Kind::Fingerprint, Version::V0_6).unwrap_or_else(|f| panic!("{f}"));
        let doc: Value = serde_json::from_str(&out).unwrap();
        assert_eq!((&doc["$schema"], &doc["device"]), (&json!(FINGERPRINT), &device));
        assert!(osd::read(&out, Kind::Fingerprint).is_ok());
    }
}

#[test]
fn a_0_5_fingerprint_without_a_subject_still_reads_as_0_5() {
    let doc = json!({"$schema": "https://w3id.org/osd/0.5/fingerprint.schema.json", "observed": "2026-11-22"});
    assert!(osd::read(&doc.to_string(), Kind::Fingerprint).is_ok());
}
