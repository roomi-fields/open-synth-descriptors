//! What a 0.6 sound says about the firmware it needs and was made on, beside its `firmware` in
//! words: `modules`, a list of `{module, min_version?}` (the catalogue id of a module, and the
//! least version of it, both non-empty text), and `made_on`, the exact assembly of the author
//! (an assembly without its own `$schema`, read as written). Both are optional and 0.6's own: a
//! 0.4 or 0.5 sound refuses them as unknown fields. The reader neither resolves nor compares
//! them with each other or with `firmware`.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

fn base(schema: &str) -> Value {
    json!({
        "$schema": if schema == "0.4" {
            "https://synthhub.roomi-fields.com/osd/0.4/sound.schema.json".to_string()
        } else {
            format!("https://w3id.org/osd/{schema}/sound.schema.json")
        },
        "name": "Warm pad",
        "kind": "sound",
        "device": {"descriptor": "korg/minilogue", "vendor": "KORG", "model": "minilogue"},
        "content": {"settings": {"/amp/attack": [90]}}
    })
}

fn with(schema: &str, key: &str, value: Value) -> Value {
    let mut doc = base(schema);
    doc[key] = value;
    doc
}

fn read(doc: &Value) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Sound)
}

fn rewritten(doc: &Value) -> Value {
    let d = read(doc).unwrap_or_else(|f| panic!("{doc} should read: {f}"));
    serde_json::from_str(&osd::write(&d)).unwrap()
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).expect_err(&format!("{doc} should be refused"));
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{doc}: {f}");
}

fn assembly() -> Value {
    json!({"stock": "acme/stock", "modules": ["acme/mod-a", "acme/mod-b"]})
}

#[test]
fn a_sound_reads_and_rewrites_the_modules_it_needs_with_or_without_a_minimum_version() {
    let modules = json!([{"module": "acme/mod-a", "min_version": "1.2"}, {"module": "acme/mod-b"}]);
    let back = rewritten(&with("0.6", "modules", modules.clone()));
    assert_eq!(back["modules"], modules);
}

#[test]
fn a_sound_reads_and_rewrites_the_assembly_it_was_made_on_as_written() {
    let back = rewritten(&with("0.6", "made_on", assembly()));
    assert_eq!(back["made_on"], assembly());
    assert!(back["made_on"].get("$schema").is_none(), "{back}");
}

#[test]
fn a_sound_may_carry_firmware_in_words_modules_and_made_on_together_without_one_overriding_another() {
    let mut doc = with("0.6", "firmware", json!("Stock 9.9"));
    doc["modules"] = json!([{"module": "acme/mod-z", "min_version": "3"}]);
    // The assembly neither lists the needed module nor agrees with the words: no refusal.
    doc["made_on"] = assembly();
    let back = rewritten(&doc);
    assert_eq!(back["firmware"], "Stock 9.9");
    assert_eq!(back["modules"], doc["modules"]);
    assert_eq!(back["made_on"], doc["made_on"]);
}

#[test]
fn a_sound_without_modules_and_made_on_reads_and_writes_as_before() {
    let back = rewritten(&base("0.6"));
    assert!(back.get("modules").is_none() && back.get("made_on").is_none(), "{back}");
    let doc = with("0.6", "modules", json!([]));
    assert_eq!(rewritten(&doc)["modules"], json!([]));
}

#[test]
fn a_module_id_that_is_empty_is_refused_at_the_id() {
    refused(&with("0.6", "modules", json!([{"module": ""}])), FaultKind::BadValue, "/modules/0/module");
    refused(&with("0.6", "modules", json!([{"module": "acme/a"}, {"module": ""}])), FaultKind::BadValue, "/modules/1/module");
}

#[test]
fn a_minimum_version_that_is_empty_is_refused_at_the_version() {
    refused(&with("0.6", "modules", json!([{"module": "acme/a", "min_version": ""}])), FaultKind::BadValue, "/modules/0/min_version");
}

#[test]
fn an_entry_of_modules_without_a_module_or_with_a_wrong_form_is_refused() {
    refused(&with("0.6", "modules", json!([{"min_version": "1"}])), FaultKind::Missing, "/modules/0/module");
    refused(&with("0.6", "modules", json!(["acme/a"])), FaultKind::BadValue, "/modules/0");
    refused(&with("0.6", "modules", json!({"module": "acme/a"})), FaultKind::BadValue, "/modules");
    refused(&with("0.6", "modules", json!([{"module": "acme/a", "min_version": 2}])), FaultKind::BadValue, "/modules/0/min_version");
    refused(&with("0.6", "modules", json!([{"module": "acme/a", "colour": "red"}])), FaultKind::UnknownField, "/modules/0/colour");
}

#[test]
fn a_made_on_assembly_follows_the_assembly_form() {
    refused(&with("0.6", "made_on", json!({"modules": []})), FaultKind::Missing, "/made_on/stock");
    refused(&with("0.6", "made_on", json!({"stock": "acme/stock"})), FaultKind::Missing, "/made_on/modules");
    refused(&with("0.6", "made_on", json!({"stock": "acme/stock", "modules": [1]})), FaultKind::BadValue, "/made_on/modules/0");
    refused(&with("0.6", "made_on", json!({"stock": "acme/stock", "modules": [], "colour": 1})), FaultKind::UnknownField, "/made_on/colour");
    refused(&with("0.6", "made_on", json!("acme/asm")), FaultKind::BadValue, "/made_on");
}

#[test]
fn a_sound_before_0_6_refuses_modules_and_made_on_as_unknown_fields() {
    for schema in ["0.4", "0.5"] {
        refused(&with(schema, "modules", json!([{"module": "acme/a"}])), FaultKind::UnknownField, "/modules");
        refused(&with(schema, "made_on", assembly()), FaultKind::UnknownField, "/made_on");
    }
}

#[test]
fn the_schema_and_the_reader_give_the_same_verdict_on_a_sound_with_modules_or_made_on() {
    let schemas = common::Schemas::load("0.6");
    let accepted = [
        with("0.6", "modules", json!([{"module": "acme/a", "min_version": "1.2"}, {"module": "acme/b"}])),
        with("0.6", "modules", json!([])),
        with("0.6", "made_on", assembly()),
        with("0.6", "made_on", json!({"stock": "acme/stock", "modules": []})),
    ];
    for doc in &accepted {
        assert_eq!(schemas.check(Kind::Sound, doc), None, "schema on {doc}");
        assert!(read(doc).is_ok(), "reader on {doc}");
    }
    let refused = [
        with("0.6", "modules", json!([{"module": ""}])),
        with("0.6", "modules", json!([{"module": "acme/a", "min_version": ""}])),
        with("0.6", "modules", json!([{"min_version": "1"}])),
        with("0.6", "modules", json!(["acme/a"])),
        with("0.6", "made_on", json!({"modules": []})),
        with("0.6", "made_on", json!({"stock": "acme/stock"})),
        with("0.6", "made_on", json!({"stock": "acme/stock", "modules": [], "colour": 1})),
    ];
    for doc in &refused {
        assert!(schemas.check(Kind::Sound, doc).is_some(), "schema on {doc}");
        assert!(read(doc).is_err(), "reader on {doc}");
    }
}

#[test]
fn the_conformance_cases_of_modules_and_made_on_exist_and_both_agree_with_them() {
    let schemas = common::Schemas::load("0.6");
    let results = ["sound-modules-needed", "sound-made-on-assembly", "sound-modules-and-made-on"];
    let faults = [("sound-modules-module-empty-refused", "/modules/0/module"), ("sound-modules-min-version-empty-refused", "/modules/0/min_version")];
    let dir = |name: &str| common::repo().join("conformance/0.6/read").join(name);
    let json = |name: &str, file: &str| -> Value {
        let text = std::fs::read_to_string(dir(name).join(file)).unwrap_or_else(|e| panic!("{name}/{file}: {e}"));
        serde_json::from_str(&text).unwrap()
    };
    for name in results {
        let (case, doc) = (json(name, "case.json"), json(name, "descriptor.json"));
        assert_eq!(case["read"], "sound", "{name}");
        assert_eq!(schemas.check(Kind::Sound, &doc), None, "{name}: schema");
        assert_eq!(rewritten(&doc), case["result"], "{name}: reader");
    }
    for (name, pointer) in faults {
        let (case, doc) = (json(name, "case.json"), json(name, "descriptor.json"));
        assert_eq!(case["fault"]["kind"], "BadValue", "{name}");
        assert_eq!(case["fault"]["pointer"], pointer, "{name}");
        refused(&doc, FaultKind::BadValue, pointer);
        let errors = schemas.errors(Kind::Sound, &doc);
        assert_eq!(errors.len(), 1, "{name}: {errors:?}");
        assert_eq!(errors[0].0, pointer, "{name}");
    }
}
