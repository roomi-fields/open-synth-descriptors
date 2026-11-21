//! The builder of a 0.6 firmware descriptor: the tool (Elekloader, Modwerk) that makes the
//! assembly from the stock OS and modules, named by `name` and `url`; `install.method` may be
//! `builder`, which then requires it.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

const SCHEMA_06: &str = "https://w3id.org/osd/0.6/firmware.schema.json";

fn builder() -> Value {
    json!({"name": "Elekloader", "url": "https://example.org/elekloader"})
}

/// A 0.6 firmware descriptor of the given kind.
fn fw(kind: &str) -> Value {
    json!({
        "$schema": SCHEMA_06, "name": "Elekloader mod", "version": "2.0", "kind": kind,
        "targets": [{"device": {"vendor": "Elektron", "model": "Digitone"}}]
    })
}

fn with(mut doc: Value, key: &str, value: Value) -> Value {
    doc[key] = value;
    doc
}

/// The refusal of `doc` by the reader and by the schema of `version`: the kind and pointer of the
/// reader's, and the pointer of the schema's errors (at the parent for a missing or unknown field).
fn refused_in(version: &str, doc: &Value, kind: FaultKind, pointer: &str) {
    let fault = osd::read(&doc.to_string(), Kind::Firmware).expect_err(&format!("{doc} is refused"));
    assert_eq!((fault.kind, fault.pointer.as_str()), (kind, pointer), "the reader refuses {doc}: {fault}");
    let errors = common::Schemas::load(version).errors(Kind::Firmware, doc);
    let want = if matches!(kind, FaultKind::Missing | FaultKind::UnknownField) {
        pointer.rsplit_once('/').map_or("", |(parent, _)| parent)
    } else {
        pointer
    };
    let got: Vec<&str> = errors.iter().map(|(at, _)| at.as_str()).collect();
    assert!(!got.is_empty() && got.iter().all(|at| *at == want), "the schema refuses {doc} at {want}, not {errors:?}");
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    refused_in("0.6", doc, kind, pointer);
}

fn valid_and_round_trips(doc: &Value) {
    assert_eq!(common::Schemas::load("0.6").check(Kind::Firmware, doc), None, "{doc}");
    let d = osd::read(&doc.to_string(), Kind::Firmware).unwrap_or_else(|f| panic!("{doc}: {f}"));
    let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
    assert_eq!(&back, doc);
}

#[test]
fn a_module_names_its_builder_with_a_name_and_an_address_and_writes_it_back_as_written() {
    valid_and_round_trips(&with(fw("mod"), "builder", builder()));
}

#[test]
fn a_builder_is_allowed_for_every_kind_of_firmware() {
    for kind in ["firmware", "mod", "script", "extension"] {
        valid_and_round_trips(&with(fw(kind), "builder", builder()));
    }
}

#[test]
fn a_builder_without_a_name_or_without_an_address_is_refused_at_the_missing_field() {
    refused(&with(fw("mod"), "builder", json!({"url": "https://example.org/elekloader"})), FaultKind::Missing, "/builder/name");
    refused(&with(fw("mod"), "builder", json!({"name": "Elekloader"})), FaultKind::Missing, "/builder/url");
    refused(&with(fw("mod"), "builder", json!({})), FaultKind::Missing, "/builder/name");
}

#[test]
fn a_builder_that_is_not_an_object_or_whose_address_is_not_text_is_a_bad_value() {
    refused(&with(fw("mod"), "builder", json!("Elekloader")), FaultKind::BadValue, "/builder");
    refused(&with(fw("mod"), "builder", json!(["Elekloader"])), FaultKind::BadValue, "/builder");
    refused(&with(fw("mod"), "builder", json!({"name": "Elekloader", "url": 5})), FaultKind::BadValue, "/builder/url");
    refused(&with(fw("mod"), "builder", json!({"name": 3, "url": "https://example.org"})), FaultKind::BadValue, "/builder/name");
}

#[test]
fn a_builder_holds_nothing_but_a_name_and_an_address() {
    let doc = with(fw("mod"), "builder", json!({"name": "Elekloader", "url": "https://example.org", "colour": "red"}));
    refused(&doc, FaultKind::UnknownField, "/builder/colour");
}

#[test]
fn builder_is_an_unknown_field_of_a_0_4_and_a_0_5_firmware() {
    for (version, host) in [("0.4", "synthhub.roomi-fields.com/osd"), ("0.5", "w3id.org/osd")] {
        let doc = json!({
            "$schema": format!("https://{host}/{version}/firmware.schema.json"),
            "name": "TRIBE", "version": "1", "kind": "mod",
            "device": {"vendor": "M-VAVE", "model": "FM-1"}, "builder": builder()
        });
        refused_in(version, &doc, FaultKind::UnknownField, "/builder");
    }
}

#[test]
fn an_install_method_builder_without_a_builder_is_refused_at_the_missing_builder() {
    let doc = with(fw("mod"), "install", json!({"method": "builder"}));
    refused(&doc, FaultKind::Missing, "/builder");
    let doc = with(fw("firmware"), "install", json!({"method": "builder", "steps": "Open the builder.", "revert": "Flash the stock OS."}));
    refused(&doc, FaultKind::Missing, "/builder");
}

#[test]
fn an_install_method_builder_with_a_builder_keeps_its_steps_and_its_revert_as_written() {
    let install = json!({"method": "builder", "steps": "Pick the modules, then flash the result.", "revert": "Flash the stock Elektron OS."});
    valid_and_round_trips(&with(with(fw("mod"), "builder", builder()), "install", install));
    let revert_only = json!({"method": "builder", "revert": "Flash the stock Elektron OS."});
    valid_and_round_trips(&with(with(fw("script"), "builder", builder()), "install", revert_only));
}

#[test]
fn a_builder_with_another_install_method_reads_without_fault() {
    let install = json!({"method": "usb-drive", "steps": "Copy the file.", "revert": "Copy the stock file."});
    valid_and_round_trips(&with(with(fw("mod"), "builder", builder()), "install", install));
    valid_and_round_trips(&with(with(fw("mod"), "builder", builder()), "install", json!({})));
}

#[test]
fn an_install_method_other_than_builder_needs_no_builder() {
    valid_and_round_trips(&with(fw("mod"), "install", json!({"method": "usb-drive"})));
}
