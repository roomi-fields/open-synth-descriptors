//! The five kinds of descriptor whose credit 0.7 adds (a device had it already), each as a
//! small descriptor of a version, with what the tests of the credit do with one.

use super::Schemas;
use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The kinds that carry `sources` from 0.7 beside the device.
pub const KINDS: [Kind; 5] = [Kind::Profile, Kind::Sound, Kind::Fingerprint, Kind::Firmware, Kind::Assembly];

pub fn word(kind: Kind) -> &'static str {
    match kind {
        Kind::Device => "device",
        Kind::Profile => "profile",
        Kind::Sound => "sound",
        Kind::Fingerprint => "fingerprint",
        Kind::Firmware => "firmware",
        Kind::Assembly => "assembly",
    }
}

/// The `$schema` of `kind` in `version` (0.5 or later).
pub fn address(version: &str, kind: Kind) -> String {
    format!("https://w3id.org/osd/{version}/{}.schema.json", word(kind))
}

/// `doc` with the keys of `extra`.
pub fn with(mut doc: Value, extra: Value) -> Value {
    for (k, v) in extra.as_object().expect("an object") {
        doc[k] = v.clone();
    }
    doc
}

/// A descriptor of `kind` in `version` (0.5 or later) holding the keys of `extra` beside the few
/// it requires. An assembly exists from 0.6.
pub fn descriptor(version: &str, kind: Kind, extra: Value) -> Value {
    let target = json!({"descriptor": "korg/minilogue", "vendor": "KORG", "model": "minilogue"});
    let body = match kind {
        Kind::Device => json!({"device": {"name": "Synth", "osc_prefix": "/synth"}}),
        Kind::Profile => json!({"identity": {"reply_text": "acme"}}),
        Kind::Sound => json!({"name": "Warm pad", "kind": "sound", "device": target, "content": {"settings": {"/amp/attack": [90]}}}),
        Kind::Fingerprint => json!({"observed": "2026-11-22", "device": {"descriptor": "korg/minilogue", "firmware": "2.01"}}),
        Kind::Firmware => json!({"name": "Elekloader mod", "version": "2.0", "kind": "mod", "targets": [{"device": target}]}),
        Kind::Assembly => json!({"stock": {"firmware": "acme/stock", "version": "1.0"}, "modules": []}),
    };
    let mut doc = json!({"$schema": address(version, kind)});
    doc = with(doc, body);
    with(doc, extra)
}

/// An assembly as a sound's `made_on` writes it (no `$schema`), with the keys of `extra`.
pub fn made_on(extra: Value) -> Value {
    with(json!({"stock": {"firmware": "acme/stock", "version": "1.0"}, "modules": []}), extra)
}

pub fn read(doc: &Value, kind: Kind) -> Descriptor {
    osd::read(&doc.to_string(), kind).unwrap_or_else(|f| panic!("{f} in {doc}"))
}

/// The kind and the pointer of the refusal of `doc`.
pub fn refused(doc: &Value, kind: Kind) -> (FaultKind, String) {
    let f = osd::read(&doc.to_string(), kind).expect_err(&format!("refused: {doc}"));
    (f.kind, f.pointer)
}

pub fn written_back(doc: &Value, kind: Kind) -> Value {
    serde_json::from_str(&osd::write(&read(doc, kind))).unwrap()
}

/// `doc` carried to 0.7: it reads as 0.7, the schema of 0.7 accepts it and it converts to itself.
pub fn converted(doc: &Value, kind: Kind, schemas: &Schemas) -> Value {
    let out = osd::convert_to(&doc.to_string(), kind, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {doc}"));
    osd::read_as(&out, kind, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {out}"));
    assert_eq!(osd::convert_to(&out, kind, Version::V0_7).as_deref(), Ok(out.as_str()), "the converted text converts to itself");
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(schemas.check(kind, &value), None, "{value}");
    value
}

/// The warnings of `doc`, each as the name of its kind and its pointer, in the order given.
pub fn warned(doc: &Value, kind: Kind) -> Vec<(String, String)> {
    osd::warnings(&read(doc, kind))
        .into_iter()
        .map(|w| {
            let name = format!("{:?}", w.kind);
            (name.split('(').next().unwrap().to_string(), w.pointer)
        })
        .collect()
}

pub fn catalogue(files: &[(&str, Value)]) -> BTreeMap<String, String> {
    files.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}
