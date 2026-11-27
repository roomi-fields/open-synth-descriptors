//! The fields 0.7 removes (`transform_reverse`, `device.revision`, `transport.rate_limit_hz`,
//! `sysex.footer`, `midi_out.note_offset`, `identity.sends`, `sources[].imported_at`): a 0.7 text
//! that carries one is refused as an unknown field at its pointer (a source too, though it is
//! an open object), the converter 0.6 to 0.7 drops each but the date of a source, which it keeps
//! as the note `_imported_at`, a 0.6 text reads as 0.6 and the identity rule of `midi_of` follows
//! the version of the pair.

use crate::common;

use osd::{Capability, FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn device(schema: &str, extra: Value) -> Value {
    let mut doc = json!({"$schema": schema, "device": {"name": "Synth", "osc_prefix": "/synth"}});
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn refused(doc: &Value) -> (FaultKind, String) {
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err("refused");
    (f.kind, f.pointer)
}

fn converted(doc: &Value) -> Value {
    let out = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"));
    serde_json::from_str(&out).unwrap()
}

fn routes() -> Value {
    json!({"entries": [{"osc": "/a", "cc": 1, "transform": "x * 2", "transform_reverse": "x // 2"}]})
}

fn params() -> Value {
    json!({"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7",
        "entries": [{"osc": "/b", "id": 3, "transform": "x * 2", "transform_reverse": "x // 2"}]})
}

/// The same control of 0.7, its message a way.
fn routes_07() -> Value {
    json!({"entries": [{"osc": "/a", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}], "transform": "x * 2", "transform_reverse": "x // 2"}]})
}

fn params_07() -> Value {
    json!({"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7",
        "entries": [{"osc": "/b", "ways": [{"ctrlType": "sysex", "id": 3}], "transform": "x * 2", "transform_reverse": "x // 2"}]})
}

fn way_cc(entry: Value) -> Value {
    let mut entry = entry;
    entry["ways"] = json!([{"ctrlType": "cc", "ctrlIndex": [1]}]);
    entry
}

/// Each removed field: its name, a 0.7 text that carries it (and nothing else wrong), its pointer.
fn removed_in_closed_objects() -> Vec<(&'static str, Value, &'static str)> {
    let mut dev_revision = device(D07, json!({}));
    dev_revision["device"]["revision"] = json!("2");
    let mut transport = device(D07, json!({}));
    transport["device"]["transport"] = json!({"kind": "midi", "rate_limit_hz": 100});
    vec![
        ("cc_params transform_reverse", device(D07, json!({"capabilities": ["lua"], "cc_params": routes_07()})), "/cc_params/entries/0/transform_reverse"),
        ("params transform_reverse", device(D07, json!({"capabilities": ["lua"], "params": params_07()})), "/params/entries/0/transform_reverse"),
        ("device.revision", dev_revision, "/device/revision"),
        ("transport.rate_limit_hz", transport, "/device/transport/rate_limit_hz"),
        ("sysex.footer", device(D07, json!({"sysex": {"header": [240, 67], "footer": [247]}})), "/sysex/footer"),
        ("midi_out.note_offset", device(D07, json!({"midi_out": {"note_offset": 0}})), "/midi_out/note_offset"),
        ("identity.sends", device(D07, json!({"identity": {"reply": [240, 126, "any", 6, 2], "sends": {"clock": true}}})), "/identity/sends"),
    ]
}

#[test]
fn each_removed_field_is_refused_in_a_0_7_device_as_an_unknown_field_at_its_pointer() {
    for (name, doc, pointer) in removed_in_closed_objects() {
        assert_eq!(refused(&doc), (FaultKind::UnknownField, pointer.to_string()), "{name}");
    }
}

#[test]
fn a_0_7_device_without_the_removed_fields_still_reads_what_stays() {
    let mut doc = device(D07, json!({
        "capabilities": ["lua"],
        "_coverage": {"controls": 3},
        "sysex": {"header": [240, 67]},
        "midi_out": {"default_channel": 1},
        "identity": {"reply": [240, 126, "any", 6, 2]},
        "cc_params": {"entries": [way_cc(json!({"osc": "/a", "transform": "x * 2"}))]},
    }));
    doc["device"]["rate_limit_hz"] = json!(50);
    doc["device"]["transport"] = json!({"kind": "midi"});
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn a_0_7_source_refuses_imported_at_though_a_source_is_an_open_object() {
    let doc = device(D07, json!({"sources": [{"type": "vendor-doc", "imported_at": "2026-01-01"}]}));
    assert_eq!(refused(&doc), (FaultKind::UnknownField, "/sources/0/imported_at".to_string()));
}

#[test]
fn a_0_7_device_refuses_imported_at_in_midi_of_sources_at_any_rank_and_whatever_its_value() {
    let values = [json!("2026-01-01"), json!(""), json!(null), json!(0), json!(false), json!([]), json!({"on": "2026-01-01"})];
    for value in &values {
        for rank in 0..3 {
            for linked in [true, false] {
                let mut list = json!([{"type": "vendor-doc"}, {"type": "community-re", "_imported_at": "a"}, {"type": "test-fixture"}]);
                list[rank]["imported_at"] = value.clone();
                let mut doc = device(D07, json!({"midi_of_sources": list}));
                if linked {
                    doc["midi_of"] = json!("acme/b");
                }
                let want = (FaultKind::UnknownField, format!("/midi_of_sources/{rank}/imported_at"));
                assert_eq!(refused(&doc), want, "{doc}");
                let f = osd::read_as(&doc.to_string(), Kind::Device, Version::V0_7).expect_err("refused");
                assert_eq!((f.kind, f.pointer), want, "{doc}");
            }
        }
    }
}

#[test]
fn a_0_7_source_of_midi_of_sources_keeps_its_other_unknown_keys_and_is_written_back_as_it_is() {
    let list = json!([
        {"type": "vendor-doc", "_imported_at": "2026-01-01", "_imported_at_2": null, "x-other": [1]},
        {"type": "community-re", "imported_on": "2026-02-02", "_note": {"imported_at": "kept"}},
    ]);
    for linked in [true, false] {
        let mut doc = device(D07, json!({"midi_of_sources": list}));
        if linked {
            doc["midi_of"] = json!("acme/b");
        }
        let d = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
        let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
        assert_eq!(back["midi_of_sources"], list, "{back}");
    }
}

#[test]
fn a_text_before_0_7_with_a_date_in_midi_of_sources_still_reads_and_converts_to_0_7() {
    let list = json!([{"type": "vendor-doc", "imported_at": "2026-01-01"}]);
    for schema in [None, Some("https://w3id.org/osd/0.5/device.schema.json"), Some(D06)] {
        let mut doc = device(D06, json!({"midi_of": "acme/b", "_midi_of_sources": list}));
        match schema {
            Some(s) => doc["$schema"] = json!(s),
            None => {
                doc.as_object_mut().unwrap().remove("$schema");
            }
        }
        let d = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f} in {doc}"));
        let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
        assert_eq!(back["_midi_of_sources"], list, "{back}");
        let out = converted(&doc);
        assert_eq!(out["midi_of_sources"], json!([{"type": "vendor-doc", "_imported_at": "2026-01-01"}]), "{out}");
        osd::read(&out.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f} in {out}"));
    }
}

#[test]
fn a_0_7_source_still_keeps_its_other_unknown_keys() {
    let doc = device(D07, json!({"sources": [{"type": "vendor-doc", "x-other": 1}]}));
    let d = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
    assert_eq!(back["sources"][0]["x-other"], 1);
}

#[test]
fn the_converter_drops_each_removed_field_but_the_date_kept_as_a_note_and_keeps_the_neighbouring_ones() {
    let mut doc = device(D06, json!({
        "capabilities": ["lua"],
        "_coverage": {"controls": 3},
        "_sources": [{"type": "vendor-doc", "notes": "n", "imported_at": "2026-01-01"}],
        "sysex": {"header": [240, 67], "footer": [247]},
        "midi_out": {"default_channel": 1, "note_offset": 0},
        "identity": {"reply": [240, 126, "any", 6, 2], "sends": {"clock": true}},
        "cc_params": routes(),
        "params": params(),
    }));
    doc["device"]["revision"] = json!("2");
    doc["device"]["rate_limit_hz"] = json!(50);
    doc["device"]["transport"] = json!({"kind": "midi", "rate_limit_hz": 100});
    let out = converted(&doc);
    assert_eq!(out["device"], json!({"name": "Synth", "osc_prefix": "/synth", "rate_limit_hz": 50, "transport": {"kind": "midi"}}));
    assert_eq!(out["sources"], json!([{"type": "vendor-doc", "notes": "n", "_imported_at": "2026-01-01"}]));
    assert_eq!(out["sysex"], json!({"header": [240, 67]}));
    assert_eq!(out["midi_out"], json!({"default_channel": 1}));
    assert_eq!(out["identity"], json!({"reply": [240, 126, "any", 6, 2]}));
    assert_eq!(out["cc_params"]["entries"][0], way_cc(json!({"osc": "/a", "transform": "x * 2"})));
    assert_eq!(out["params"]["entries"][0], json!({"osc": "/b", "ways": [{"ctrlType": "sysex", "id": 3}], "transform": "x * 2"}));
    assert_eq!(out["capabilities"], json!(["lua"]));
    assert_eq!(out["_coverage"], json!({"controls": 3}));
    osd::read(&out.to_string(), Kind::Device).unwrap_or_else(|f| panic!("the converted text does not read: {f}"));
}

#[test]
fn the_converter_renames_imported_at_in_every_source_that_has_one_and_leaves_other_keys() {
    let out = converted(&device(D06, json!({"_sources": [
        {"type": "vendor-doc", "imported_at": "2026-01-01"},
        {"type": "community-re", "url": "https://x.org/p", "imported_at": "2026-02-02", "x-other": [1]},
        {"type": "test-fixture"},
    ]})));
    assert_eq!(out["sources"], json!([
        {"type": "vendor-doc", "_imported_at": "2026-01-01"},
        {"type": "community-re", "url": "https://x.org/p", "_imported_at": "2026-02-02", "x-other": [1]},
        {"type": "test-fixture"},
    ]));
}

#[test]
fn the_converter_keeps_the_date_of_the_midi_of_sources_as_a_note_too() {
    let out = converted(&device(D06, json!({"midi_of": "acme/b", "_midi_of_sources": [{"type": "vendor-doc", "imported_at": "2026-01-01"}]})));
    assert_eq!(out["midi_of_sources"], json!([{"type": "vendor-doc", "_imported_at": "2026-01-01"}]));
}

#[test]
fn a_converted_text_converts_to_itself_after_the_removals() {
    let doc = device(D06, json!({"cc_params": routes(), "capabilities": ["lua"], "sysex": {"header": [240], "footer": [247]}}));
    let once = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap();
    assert_eq!(osd::convert_to(&once, Kind::Device, Version::V0_7).unwrap(), once);
}

#[test]
fn a_0_5_text_with_the_removed_fields_reaches_0_7_without_them() {
    let doc = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t", "revision": "2"},
        "capabilities": ["lua"],
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "transform": "x * 2", "transform_reverse": "x // 2"}]}});
    let out = converted(&doc);
    assert!(out["device"].get("revision").is_none(), "{out}");
    assert!(out["cc_params"]["entries"][0].get("transform_reverse").is_none(), "{out}");
}

#[test]
fn a_0_6_text_that_carries_the_removed_fields_still_reads_as_0_6() {
    let mut doc = device(D06, json!({
        "capabilities": ["lua"],
        "_sources": [{"type": "vendor-doc", "imported_at": "2026-01-01"}],
        "sysex": {"header": [240, 67], "footer": [247]},
        "midi_out": {"default_channel": 1, "note_offset": 0},
        "identity": {"reply": [240, 126, "any", 6, 2], "sends": {"clock": true}},
        "cc_params": routes(),
        "params": params(),
    }));
    doc["device"]["revision"] = json!("2");
    doc["device"]["transport"] = json!({"kind": "midi", "rate_limit_hz": 100});
    let d = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(d.schema(), Some(Version::V0_6));
    osd::read_as(&doc.to_string(), Kind::Device, Version::V0_6).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn a_0_6_device_with_only_transform_reverse_still_requires_lua_but_a_0_7_one_with_transform_does() {
    let only_reverse = device(D06, json!({"capabilities": ["lua"], "cc_params": {"entries": [{"osc": "/a", "cc": 1, "transform_reverse": "x // 2"}]}}));
    osd::read(&only_reverse.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    let forward = device(D07, json!({"capabilities": ["lua"], "cc_params": {"entries": [way_cc(json!({"osc": "/a", "transform": "x * 2"}))]}}));
    osd::read(&forward.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn a_0_7_device_with_no_lua_field_and_lua_declared_is_refused_at_capabilities() {
    let none = device(D07, json!({"capabilities": ["lua"], "cc_params": {"entries": [way_cc(json!({"osc": "/a"}))]}}));
    assert_eq!(refused(&none), (FaultKind::BadValue, "/capabilities".to_string()));
}

#[test]
fn a_0_7_device_derives_lua_from_transform_alone() {
    let doc = device(D07, json!({"capabilities": ["lua"], "cc_params": {"entries": [way_cc(json!({"osc": "/a", "transform": "x * 2"}))]}}));
    let files: BTreeMap<String, String> = [("acme/a".to_string(), doc.to_string())].into();
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.capabilities, vec![Capability::Lua]);
}

// ---- the identity rule of `midi_of` follows the version of the pair ----

fn resolved(files: Vec<(&str, Value)>) -> Value {
    let files: BTreeMap<String, String> = files.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap()
}

fn with_revision(schema: &str, revision: Option<&str>, extra: Value) -> Value {
    let mut d = device(schema, extra);
    if let Some(r) = revision {
        d["device"]["revision"] = json!(r);
    }
    d
}

#[test]
fn a_0_6_file_on_a_0_6_target_keeps_revision_as_an_identity_key() {
    let doc = resolved(vec![
        ("acme/a", with_revision(D06, Some("2"), json!({"midi_of": "acme/b"}))),
        ("acme/b", with_revision(D06, Some("1"), json!({"cc_params": {"entries": [{"osc": "/x", "cc": 1}]}}))),
    ]);
    assert_eq!(doc["device"]["revision"], "2");
    assert_eq!(doc["$schema"], D06);
}

#[test]
fn a_0_6_file_without_revision_on_a_0_6_target_with_one_takes_none_from_the_target() {
    let doc = resolved(vec![
        ("acme/a", with_revision(D06, None, json!({"midi_of": "acme/b"}))),
        ("acme/b", with_revision(D06, Some("1"), json!({"cc_params": {"entries": [{"osc": "/x", "cc": 1}]}}))),
    ]);
    assert!(doc["device"].get("revision").is_none(), "{doc}");
}

#[test]
fn a_0_6_file_with_revision_on_a_0_7_target_does_not_count_revision() {
    let doc = resolved(vec![
        ("acme/a", with_revision(D06, Some("2"), json!({"midi_of": "acme/b"}))),
        ("acme/b", device(D07, json!({"cc_params": {"entries": [way_cc(json!({"osc": "/x"}))]}}))),
    ]);
    assert_eq!(doc["$schema"], D07);
    assert!(doc["device"].get("revision").is_none(), "{doc}");
}

#[test]
fn a_0_7_file_on_a_0_7_target_keeps_its_four_identity_keys_and_takes_the_rest_from_the_target() {
    let identity = json!({"name": "Own", "vendor": "V1", "osc_prefix": "/own", "kind": "hardware"});
    let mut file = device(D07, json!({"midi_of": "acme/b"}));
    file["device"] = identity.clone();
    let mut target = device(D07, json!({"cc_params": {"entries": [way_cc(json!({"osc": "/x"}))]}}));
    target["device"] = identity.clone();
    target["device"]["rate_limit_hz"] = json!(10);
    let doc = resolved(vec![("acme/a", file), ("acme/b", target)]);
    let mut want = identity;
    want["rate_limit_hz"] = json!(10);
    assert_eq!(doc["device"], want);
}

#[test]
fn the_cases_of_the_removed_fields_exist_under_their_names() {
    let root = crate::common::repo().join("conformance/0.7");
    for case in [
        "convert/removed-fields-dropped",
        "read/transform-reverse-refused",
        "read/params-transform-reverse-refused",
        "read/device-revision-refused",
        "read/transport-rate-limit-refused",
        "read/sysex-footer-refused",
        "read/midi-out-note-offset-refused",
        "read/identity-sends-refused",
        "read/source-imported-at-refused",
        "resolve/midi-of-0-6-file-revision-ignored",
    ] {
        assert!(root.join(case).join("case.json").is_file(), "{case}");
    }
}

// ---- the resolver and midi_of_sources ----

/// The refusal of the resolution of `acme/a`: its kind, its file and its pointer.
fn resolution_refused(files: &[(&str, Value)]) -> (FaultKind, Option<String>, String) {
    let files: BTreeMap<String, String> = files.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let f = osd::resolve("acme/a", &files).expect_err("refused");
    (f.kind, f.file, f.pointer)
}

/// A 0.7 device whose second source of `midi_of_sources` is dated by `value`.
fn dated_07(extra: Value, value: Value) -> Value {
    let mut doc = device(D07, extra);
    doc["midi_of_sources"] = json!([{"type": "vendor-doc"}, {"type": "community-re", "imported_at": value}]);
    doc
}

fn refused_in(file: &str) -> (FaultKind, Option<String>, String) {
    (FaultKind::UnknownField, Some(file.to_string()), "/midi_of_sources/1/imported_at".to_string())
}

#[test]
fn resolve_refuses_imported_at_in_midi_of_sources_of_the_0_7_file_asked_for() {
    let target = device(D07, json!({}));
    for value in [json!("2026-01-01"), json!(null)] {
        let alone = [("acme/a", dated_07(json!({}), value.clone()))];
        assert_eq!(resolution_refused(&alone), refused_in("acme/a"), "{value}");
        let linked = [("acme/a", dated_07(json!({"midi_of": "acme/b"}), value.clone())), ("acme/b", target.clone())];
        assert_eq!(resolution_refused(&linked), refused_in("acme/a"), "{value}");
    }
}

#[test]
fn resolve_refuses_imported_at_in_midi_of_sources_of_a_0_7_target_in_the_target() {
    for schema in [D07, D06] {
        for value in [json!("2026-01-01"), json!(null)] {
            let files = [("acme/a", device(schema, json!({"midi_of": "acme/b"}))), ("acme/b", dated_07(json!({}), value.clone()))];
            assert_eq!(resolution_refused(&files), refused_in("acme/b"), "{schema} {value}");
        }
    }
    let chain = [
        ("acme/a", device(D07, json!({"midi_of": "acme/b"}))),
        ("acme/b", device(D07, json!({"midi_of": "acme/c"}))),
        ("acme/c", dated_07(json!({}), json!("2026-01-01"))),
    ];
    assert_eq!(resolution_refused(&chain), refused_in("acme/c"));
}

// ---- the text ----

#[test]
fn the_text_of_0_7_says_the_reader_refuses_imported_at_in_both_lists_of_sources_and_names_its_cases() {
    let spec = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    let section: Vec<&str> = spec.lines().skip_while(|l| !l.starts_with("### 5.3.2 ")).skip(1).take_while(|l| !l.starts_with('#')).collect();
    assert!(
        section.iter().any(|l| ["reader", "refuse", "`imported_at`", "`sources`", "`midi_of_sources`"].iter().all(|w| l.contains(w))),
        "no paragraph of 5.3.2 says the reader refuses `imported_at` in `sources` and in `midi_of_sources`"
    );
    let cases = section.iter().find(|l| l.starts_with("Cases:")).expect("the Cases line of 5.3.2");
    for case in ["0.7/read/midi-of-source-imported-at-refused", "0.7/convert/midi-of-source-imported-at-becomes-a-note"] {
        assert!(cases.contains(&format!("`{case}`")), "the Cases line of 5.3.2 does not name {case}");
        assert!(common::repo().join("conformance").join(case).join("case.json").is_file(), "{case}");
    }
}
