//! Resolving a `midi_of` file and its target of two versions: the older text
//! is converted to the newer version before the two are joined, so the
//! resolved document states one version.

use osd::FaultKind;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const DEVICE: &str = "https://w3id.org/osd/0.5/device.schema.json";

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn device_part() -> Value {
    json!({"name": "Synth", "osc_prefix": "/synth"})
}

/// A 0.4 file: no `$schema`.
fn v04(extra: Value) -> Value {
    let mut doc = json!({"device": device_part()});
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

/// A 0.5 file: it carries its `$schema`.
fn v05(extra: Value) -> Value {
    let mut doc = v04(extra);
    doc["$schema"] = json!(DEVICE);
    doc
}

fn routes(list: &[(&str, u8)]) -> Value {
    json!({"entries": list.iter().map(|(osc, cc)| json!({"osc": osc, "cc": cc})).collect::<Vec<_>>()})
}

/// The resolved document of `id`, as the reader writes it.
fn resolved(id: &str, files: &BTreeMap<String, String>) -> Value {
    let r = osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"));
    serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap()
}

#[test]
fn a_0_5_file_sharing_a_0_4_target_resolves_into_a_0_5_document_with_both_routes() {
    let files = catalogue(vec![
        ("acme/fw-1", v04(json!({"cc_params": routes(&[("/x", 1), ("/y", 2)])}))),
        ("acme/fw-2", v05(json!({"midi_of": "acme/fw-1", "cc_params": routes(&[("/y", 3)])}))),
    ]);
    let doc = resolved("acme/fw-2", &files);
    assert_eq!(doc["$schema"], DEVICE);
    assert_eq!(doc["cc_params"], routes(&[("/x", 1), ("/y", 3)]));
    assert_eq!(doc["midi_of"], "acme/fw-1");
}

#[test]
fn a_0_4_file_sharing_a_0_5_target_resolves_into_a_0_5_document_with_both_routes() {
    let files = catalogue(vec![
        ("acme/fw-1", v05(json!({"cc_params": routes(&[("/x", 1), ("/y", 2)])}))),
        ("acme/fw-2", v04(json!({"midi_of": "acme/fw-1", "cc_params": routes(&[("/y", 3)])}))),
    ]);
    let doc = resolved("acme/fw-2", &files);
    assert_eq!(doc["$schema"], DEVICE);
    assert_eq!(doc["cc_params"], routes(&[("/x", 1), ("/y", 3)]));
    assert_eq!(doc["midi_of"], "acme/fw-1");
}

#[test]
fn a_chain_compares_each_link_with_its_own_target_and_resolves_to_the_newest_version() {
    let files = catalogue(vec![
        ("acme/a", v04(json!({"cc_params": routes(&[("/x", 1)])}))),
        ("acme/b", v04(json!({"midi_of": "acme/a", "cc_params": routes(&[("/y", 2)])}))),
        ("acme/c", v05(json!({"midi_of": "acme/b", "cc_params": routes(&[("/z", 3)])}))),
    ]);
    let doc = resolved("acme/c", &files);
    assert_eq!(doc["$schema"], DEVICE);
    assert_eq!(doc["cc_params"], routes(&[("/x", 1), ("/y", 2), ("/z", 3)]));
    assert_eq!(doc["midi_of"], "acme/b");
}

#[test]
fn a_chain_whose_middle_link_is_the_oldest_resolves_into_a_0_5_document() {
    let files = catalogue(vec![
        ("acme/a", v05(json!({"cc_params": routes(&[("/x", 1)])}))),
        ("acme/b", v04(json!({"midi_of": "acme/a", "cc_params": routes(&[("/y", 2)])}))),
        ("acme/c", v05(json!({"midi_of": "acme/b", "cc_params": routes(&[("/z", 3)])}))),
    ]);
    let doc = resolved("acme/c", &files);
    assert_eq!(doc["$schema"], DEVICE);
    assert_eq!(doc["cc_params"], routes(&[("/x", 1), ("/y", 2), ("/z", 3)]));
}

#[test]
fn two_0_4_files_resolve_as_before_into_a_document_without_schema() {
    let files = catalogue(vec![
        ("acme/fw-1", v04(json!({"cc_params": routes(&[("/x", 1), ("/y", 2)])}))),
        ("acme/fw-2", v04(json!({"midi_of": "acme/fw-1", "cc_params": routes(&[("/y", 3)])}))),
    ]);
    let doc = resolved("acme/fw-2", &files);
    assert!(doc.get("$schema").is_none(), "{doc}");
    assert_eq!(doc["cc_params"], routes(&[("/x", 1), ("/y", 3)]));
}

#[test]
fn two_0_5_files_resolve_as_before_into_a_0_5_document() {
    let files = catalogue(vec![
        ("acme/fw-1", v05(json!({"cc_params": routes(&[("/x", 1), ("/y", 2)])}))),
        ("acme/fw-2", v05(json!({"midi_of": "acme/fw-1", "cc_params": routes(&[("/y", 3)])}))),
    ]);
    let doc = resolved("acme/fw-2", &files);
    assert_eq!(doc["$schema"], DEVICE);
    assert_eq!(doc["cc_params"], routes(&[("/x", 1), ("/y", 3)]));
}

#[test]
fn the_sources_of_a_converted_target_are_credited_as_without_conversion() {
    let src = |n: &str| json!([{"type": "vendor-doc", "firmware": n}]);
    let files = catalogue(vec![
        ("acme/a", v04(json!({"_sources": src("A"), "cc_params": routes(&[("/x", 1)])}))),
        ("acme/b", v04(json!({"midi_of": "acme/a", "_sources": src("B")}))),
        ("acme/c", v05(json!({"midi_of": "acme/b", "_sources": src("C")}))),
    ]);
    let doc = resolved("acme/c", &files);
    assert_eq!(doc["$schema"], DEVICE);
    assert_eq!(doc["_sources"], src("C"));
    assert_eq!(doc["_midi_of_sources"], json!([{"type": "vendor-doc", "firmware": "B"}, {"type": "vendor-doc", "firmware": "A"}]));
}

#[test]
fn a_target_whose_conversion_fails_refuses_the_resolution_with_its_file_and_place() {
    let target = v04(json!({"params": {"get_frame": [240, "{function}", 247], "set_frame": [240, 247], "value_type": "u7", "entries": []}}));
    let files = catalogue(vec![
        ("acme/fw-1", target),
        ("acme/fw-2", v05(json!({"midi_of": "acme/fw-1", "cc_params": routes(&[("/y", 3)])}))),
    ]);
    let f = osd::resolve("acme/fw-2", &files).expect_err("refused");
    assert_eq!(f.kind, FaultKind::NotConvertible);
    assert_eq!(f.file.as_deref(), Some("acme/fw-1"));
    assert_eq!(f.pointer, "/params/get_frame/1");
}

#[test]
fn a_target_of_an_unread_version_is_still_refused_at_its_schema() {
    let mut target = v05(json!({"cc_params": routes(&[("/x", 1)])}));
    target["$schema"] = json!("https://w3id.org/osd/0.9/device.schema.json");
    let files = catalogue(vec![
        ("acme/fw-1", target),
        ("acme/fw-2", v05(json!({"midi_of": "acme/fw-1"}))),
    ]);
    let f = osd::resolve("acme/fw-2", &files).expect_err("refused");
    assert_eq!(f.kind, FaultKind::UnreadVersion);
    assert_eq!(f.pointer, "/$schema");
}
