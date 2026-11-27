//! What the credit of 0.7 leaves as it was for a maker profile: before 0.7 `_sources` is a note of
//! any shape and `sources` no field; in 0.7 `midi_of_sources` is no field of a profile and the
//! other `_` keys are notes, the date of a source is no field.

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};

const P07: &str = "https://w3id.org/osd/0.7/profile.schema.json";
const P06: &str = "https://w3id.org/osd/0.6/profile.schema.json";
const P05: &str = "https://w3id.org/osd/0.5/profile.schema.json";

fn profile(schema: Option<&str>, extra: Value) -> Value {
    let mut doc = json!({});
    if let Some(s) = schema {
        doc["$schema"] = json!(s);
    }
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn read_ok(doc: &Value) -> Descriptor {
    osd::read(&doc.to_string(), Kind::Profile).unwrap_or_else(|f| panic!("{f} in {doc}"))
}

fn written(d: &Descriptor) -> Value {
    serde_json::from_str(&osd::write(d)).unwrap()
}

fn refused(doc: &Value) -> (FaultKind, String) {
    let f = osd::read(&doc.to_string(), Kind::Profile).expect_err("refused");
    (f.kind, f.pointer)
}

fn notes() -> [Value; 5] {
    [json!("a text"), json!(null), json!({"x": 1}), json!([{"title": "no type"}]), json!([{"type": "vendor", "license": "none", "imported_at": 3}])]
}

#[test]
fn the_underscore_sources_of_a_profile_before_0_7_is_a_note_of_any_shape_written_back() {
    for schema in [None, Some(P05), Some(P06)] {
        for value in notes() {
            let doc = profile(schema, json!({"_sources": value}));
            assert_eq!(written(&read_ok(&doc)), doc);
            assert_eq!(osd::warnings(&read_ok(&doc)), []);
        }
    }
}

#[test]
fn sources_is_no_field_of_a_profile_before_0_7() {
    for schema in [None, Some(P05), Some(P06)] {
        let doc = profile(schema, json!({"sources": [{"type": "vendor-doc"}]}));
        assert_eq!(refused(&doc), (FaultKind::UnknownField, "/sources".to_string()), "{doc}");
    }
}

#[test]
fn the_date_of_a_source_of_a_0_7_profile_is_an_unknown_field() {
    for value in [json!("2026-01-01"), json!(null), json!(3)] {
        let doc = profile(Some(P07), json!({"sources": [{"type": "vendor-doc", "imported_at": value}]}));
        assert_eq!(refused(&doc), (FaultKind::UnknownField, "/sources/0/imported_at".to_string()), "{doc}");
    }
}

#[test]
fn midi_of_sources_is_no_field_of_a_0_7_profile() {
    let doc = profile(Some(P07), json!({"midi_of_sources": [{"type": "vendor-doc"}]}));
    assert_eq!(refused(&doc), (FaultKind::UnknownField, "/midi_of_sources".to_string()));
}

#[test]
fn another_underscore_key_of_a_0_7_profile_is_a_note_written_back() {
    let doc = profile(Some(P07), json!({"_midi_of_sources": [{"type": "anything"}], "_sources_old": "x", "_notes": "n"}));
    assert_eq!(written(&read_ok(&doc)), doc);
}

#[test]
fn a_profile_without_underscore_sources_gains_no_sources_when_converted() {
    for schema in [None, Some(P05), Some(P06)] {
        let doc = profile(schema, json!({"identity": {"reply_text": "acme"}, "_notes": "n"}));
        let out = osd::convert_to(&doc.to_string(), Kind::Profile, Version::V0_7).unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out, profile(Some(P07), json!({"identity": {"reply_text": "acme"}, "_notes": "n"})));
    }
}

#[test]
fn a_free_underscore_sources_of_a_0_4_or_0_5_profile_stays_a_note_in_0_6() {
    for schema in [None, Some(P05)] {
        let doc = profile(schema, json!({"_sources": "scraped from the manual"}));
        let out = osd::convert_to(&doc.to_string(), Kind::Profile, Version::V0_6).unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["_sources"], "scraped from the manual");
    }
}
