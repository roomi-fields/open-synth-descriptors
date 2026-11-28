//! The credit of 0.7: a descriptor names its sources under `sources` (an ordinary field), the key
//! `_sources` is refused, any other key starting with `_` is a note that is kept; the converter
//! from 0.6 renames `_sources` and `_midi_of_sources` and carries the keys of a source; a
//! resolution writes `midi_of_sources`.

use osd::{Descriptor, FaultKind, Kind, Version};
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

/// The sources of a 0.6 text, converted.
fn sources_of(list: Value) -> Value {
    converted(&device(D06, json!({"_sources": list})))["sources"].clone()
}

// ---- reading ----

#[test]
fn a_0_7_device_reads_its_sources_and_writes_them_back_under_sources() {
    let doc = device(D07, json!({"sources": [{"type": "vendor-doc", "title": "Manual", "version": "2", "sha256": "ab", "license": "CC0-1.0"}]}));
    let d = osd::read(&doc.to_string(), Kind::Device).unwrap();
    let Descriptor::Device(dev) = &d else { panic!("a device") };
    assert_eq!(dev.sources.as_ref().map(Vec::len), Some(1));
    let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
    assert_eq!(back, doc);
    assert!(back.get("_sources").is_none());
}

#[test]
fn a_source_of_0_7_without_a_type_is_refused_at_its_place_under_sources() {
    let doc = device(D07, json!({"sources": [{"title": "Manual"}]}));
    assert_eq!(refused(&doc), (FaultKind::Missing, "/sources/0/type".to_string()));
}

#[test]
fn a_source_licence_outside_the_list_is_refused_at_its_place_under_sources() {
    let doc = device(D07, json!({"sources": [{"type": "vendor-doc", "license": "two words"}]}));
    assert_eq!(refused(&doc), (FaultKind::BadValue, "/sources/0/license".to_string()));
}

#[test]
fn a_source_licence_id_in_lower_case_is_read_under_sources() {
    let doc = device(D07, json!({"sources": [{"type": "vendor-doc", "license": "free"}]}));
    let d = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&d)).unwrap(), doc);
}

#[test]
fn underscore_sources_alone_is_refused_as_a_bad_value_at_its_key() {
    let doc = device(D07, json!({"_sources": [{"type": "vendor-doc"}]}));
    assert_eq!(refused(&doc), (FaultKind::BadValue, "/_sources".to_string()));
}

#[test]
fn underscore_sources_beside_sources_is_refused_the_same_way() {
    let doc = device(D07, json!({"sources": [{"type": "vendor-doc"}], "_sources": [{"type": "vendor-doc"}]}));
    assert_eq!(refused(&doc), (FaultKind::BadValue, "/_sources".to_string()));
}

#[test]
fn the_value_of_underscore_sources_is_not_read_so_any_value_is_refused_at_the_key() {
    for value in [json!("a text"), json!(null), json!([]), json!({"x": 1}), json!([{"no_type": true}])] {
        let doc = device(D07, json!({"_sources": value}));
        assert_eq!(refused(&doc), (FaultKind::BadValue, "/_sources".to_string()), "{doc}");
    }
}

#[test]
fn another_key_starting_with_an_underscore_is_kept_and_written_back_as_it_is() {
    let doc = device(
        D07,
        json!({
            "_note": "checked",
            "_usage": {"any": ["shape", 1]},
            "sources": [{"type": "vendor-doc", "_why": "kept too"}],
            "cc_params": {"entries": [{"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}], "_limitations": "half the range"}]},
        }),
    );
    let d = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
    assert_eq!(back, doc);
}

// ---- the converter ----

#[test]
fn the_converter_renames_underscore_sources_and_midi_of_sources_and_writes_no_old_key() {
    let list = json!([{"type": "vendor-doc", "notes": "A"}]);
    let out = converted(&device(D06, json!({"_sources": list, "_midi_of_sources": [{"type": "vendor-doc", "notes": "B"}]})));
    assert_eq!(out["sources"], list);
    assert_eq!(out["midi_of_sources"], json!([{"type": "vendor-doc", "notes": "B"}]));
    assert!(out.get("_sources").is_none() && out.get("_midi_of_sources").is_none(), "{out}");
    osd::read_as(&out.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn the_converter_carries_a_0_5_text_with_underscore_sources_to_sources() {
    let out = converted(&device("https://w3id.org/osd/0.5/device.schema.json", json!({"_sources": [{"type": "vendor-doc"}]})));
    assert_eq!(out["sources"], json!([{"type": "vendor-doc"}]));
    assert!(out.get("_sources").is_none());
}

#[test]
fn preset_name_becomes_title_and_name_stays_when_both_are_present() {
    let out = sources_of(json!([{"type": "vendor-doc", "preset_name": "P", "name": "N"}]));
    assert_eq!(out, json!([{"type": "vendor-doc", "title": "P", "name": "N"}]));
}

#[test]
fn name_alone_becomes_title() {
    assert_eq!(sources_of(json!([{"type": "vendor-doc", "name": "N"}])), json!([{"type": "vendor-doc", "title": "N"}]));
}

#[test]
fn preset_name_alone_becomes_title() {
    assert_eq!(sources_of(json!([{"type": "community-re", "preset_name": "P"}])), json!([{"type": "community-re", "title": "P"}]));
}

#[test]
fn revision_becomes_version_and_content_sha256_becomes_sha256() {
    let out = sources_of(json!([{"type": "vendor-doc", "revision": "r7", "content_sha256": "ff00"}]));
    assert_eq!(out, json!([{"type": "vendor-doc", "version": "r7", "sha256": "ff00"}]));
}

#[test]
fn a_target_that_already_exists_wins_and_the_old_key_stays() {
    let src = json!({"type": "vendor-doc", "title": "T", "preset_name": "P", "name": "N", "version": "1", "revision": "2", "sha256": "aa", "content_sha256": "bb"});
    assert_eq!(sources_of(json!([src])), json!([src]));
    let only_name = json!({"type": "vendor-doc", "title": "T", "name": "N"});
    assert_eq!(sources_of(json!([only_name])), json!([only_name]));
}

#[test]
fn each_source_of_the_list_is_carried_on_its_own_in_order() {
    let out = sources_of(json!([
        {"type": "vendor-doc", "name": "A"},
        {"type": "vendor-doc", "revision": "9"},
        {"type": "vendor-doc", "notes": "untouched"},
    ]));
    assert_eq!(
        out,
        json!([{"type": "vendor-doc", "title": "A"}, {"type": "vendor-doc", "version": "9"}, {"type": "vendor-doc", "notes": "untouched"}])
    );
}

#[test]
fn preset_id_and_adaptation_drop_when_the_last_segment_of_the_url_path_is_exactly_their_text() {
    let out = sources_of(json!([
        {"type": "community-re", "url": "https://x.org/p/abc123", "preset_id": "abc123"},
        {"type": "vendor-doc", "url": "https://x.org/p/Acme", "adaptation": "Acme"},
        {"type": "vendor-doc", "url": "https://x.org/p/abc123", "preset_id": "abc123", "adaptation": "abc123"},
    ]));
    assert_eq!(
        out,
        json!([
            {"type": "community-re", "url": "https://x.org/p/abc123"},
            {"type": "vendor-doc", "url": "https://x.org/p/Acme"},
            {"type": "vendor-doc", "url": "https://x.org/p/abc123"},
        ])
    );
}

#[test]
fn the_query_the_fragment_and_a_final_slash_do_not_count_in_the_end_of_the_url() {
    for url in ["https://x.org/p/abc123?x=1", "https://x.org/p/abc123#top", "https://x.org/p/abc123/", "https://x.org/p/abc123/?a=b#c"] {
        let out = sources_of(json!([{"type": "community-re", "url": url, "preset_id": "abc123"}]));
        assert_eq!(out, json!([{"type": "community-re", "url": url}]), "{url}");
    }
}

#[test]
fn preset_id_and_adaptation_stay_when_the_url_ends_otherwise() {
    for url in [
        "https://x.org/p/zzz",
        "https://x.org/p/ABC123",
        "https://x.org/p/abc",
        "https://x.org/p/xabc123",
        "https://x.org/p/abc123/more",
        "https://x.org/abc123/p",
        "https://x.org/p?id=abc123",
        "https://x.org/p#abc123",
    ] {
        let src = json!({"type": "community-re", "url": url, "preset_id": "abc123", "adaptation": "abc123"});
        assert_eq!(sources_of(json!([src])), json!([src]), "{url}");
    }
}

#[test]
fn preset_id_and_adaptation_stay_when_the_source_has_no_url() {
    let src = json!({"type": "community-re", "preset_id": "abc123", "adaptation": "Acme"});
    assert_eq!(sources_of(json!([src])), json!([src]));
}

#[test]
fn a_type_of_evidence_and_every_other_key_of_its_source_are_left_as_they_are() {
    for word in ["hardware-verified", "community-re", "contributed", "test-fixture", "vendor-doc"] {
        let src = json!({"type": word, "url": "https://x.org/p", "author": "Dee", "license": "CC-BY-4.0", "notes": "n", "firmware": "2.1", "x-other": [1, {"a": null}]});
        assert_eq!(sources_of(json!([src])), json!([src]), "{word}");
    }
}

#[test]
fn the_converter_leaves_the_text_of_every_script_as_it_is_even_one_that_reads_by_a_number() {
    let scripts = [
        "return reply[5] == 2",
        "return string.format('%d.%02d', reply[1] >> 4, reply[1] & 15)",
        "return {{0xC0, ctx.program}, {0xB0, 7, ctx.values[3]}}",
        "return {240, 1, ctx.program, 247}",
    ];
    let doc = device(
        D06,
        json!({
            "capabilities": ["lua"],
            "_sources": [{"type": "vendor-doc", "name": "N"}],
            "identity": {"reply": [240, 126, "any", 6, 2], "script": scripts[0], "firmware": {"decode": "lua", "at": [5], "script": scripts[1]}},
            "programs": {"recall_script": scripts[2], "dump": {"kind": "request", "script": scripts[3]}},
        }),
    );
    let out = converted(&doc);
    assert_eq!(out["identity"]["script"], scripts[0]);
    assert_eq!(out["identity"]["firmware"]["script"], scripts[1]);
    assert_eq!(out["programs"]["recall_script"], scripts[2]);
    assert_eq!(out["programs"]["dump"]["script"], scripts[3]);
    assert_eq!(out["capabilities"], json!(["lua"]));
    assert!(out.get("_sources").is_none());
    assert_eq!(out["sources"], json!([{"type": "vendor-doc", "title": "N"}]));
}

#[test]
fn a_converted_text_converts_to_itself() {
    let out = converted(&device(D06, json!({"_sources": [{"type": "knobkraft", "name": "N", "revision": "1", "url": "https://x.org/a", "adaptation": "a", "notes": "n"}]})));
    assert_eq!(out["sources"][0]["title"], "N");
    assert_eq!(osd::convert_to(&out.to_string(), Kind::Device, Version::V0_7).map(|t| serde_json::from_str::<Value>(&t).unwrap()), Ok(out));
}

// ---- the resolver ----

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn resolved(id: &str, files: &BTreeMap<String, String>) -> Value {
    let r = osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"));
    serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap()
}

fn src(title: &str) -> Value {
    json!({"type": "vendor-doc", "title": title})
}

fn routes_06() -> Value {
    json!({"entries": [{"osc": "/x", "cc": 1}]})
}

fn routes() -> Value {
    json!({"entries": [{"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]})
}

#[test]
fn the_resolver_writes_midi_of_sources_never_the_underscore_form_for_a_0_7_target() {
    let files = catalogue(vec![
        ("acme/a", device(D07, json!({"midi_of": "acme/b", "sources": [src("A")]}))),
        ("acme/b", device(D07, json!({"cc_params": routes(), "sources": [src("B")]}))),
    ]);
    let doc = resolved("acme/a", &files);
    assert_eq!(doc["sources"], json!([src("A")]));
    assert_eq!(doc["midi_of_sources"], json!([src("B")]));
    assert!(doc.get("_midi_of_sources").is_none() && doc.get("_sources").is_none(), "{doc}");
}

#[test]
fn the_midi_of_sources_hold_the_target_sources_then_its_own_midi_of_sources_without_removing_a_repeat() {
    let files = catalogue(vec![
        ("acme/a", device(D07, json!({"midi_of": "acme/b", "sources": [src("A")]}))),
        ("acme/b", device(D07, json!({"midi_of": "acme/c", "sources": [src("B")]}))),
        ("acme/c", device(D07, json!({"cc_params": routes(), "sources": [src("C"), src("B")]}))),
    ]);
    let doc = resolved("acme/a", &files);
    assert_eq!(doc["midi_of_sources"], json!([src("B"), src("C"), src("B")]));
}

#[test]
fn the_sources_of_a_midi_of_file_replace_the_targets_in_the_result_and_stay_out_of_midi_of_sources() {
    let files = catalogue(vec![
        ("acme/a", device(D07, json!({"midi_of": "acme/b", "sources": [src("own 1"), src("own 2")]}))),
        ("acme/b", device(D07, json!({"cc_params": routes(), "sources": [src("target")]}))),
    ]);
    let doc = resolved("acme/a", &files);
    assert_eq!(doc["sources"], json!([src("own 1"), src("own 2")]));
    assert_eq!(doc["midi_of_sources"], json!([src("target")]));
}

#[test]
fn a_midi_of_file_without_sources_has_the_targets_sources_only_in_midi_of_sources() {
    let files = catalogue(vec![
        ("acme/a", device(D07, json!({"midi_of": "acme/b"}))),
        ("acme/b", device(D07, json!({"cc_params": routes(), "sources": [src("target")]}))),
    ]);
    let doc = resolved("acme/a", &files);
    assert!(doc.get("sources").is_none(), "{doc}");
    assert_eq!(doc["midi_of_sources"], json!([src("target")]));
}

#[test]
fn a_target_without_credit_gives_no_midi_of_sources() {
    let files = catalogue(vec![
        ("acme/a", device(D07, json!({"midi_of": "acme/b", "sources": [src("A")]}))),
        ("acme/b", device(D07, json!({"cc_params": routes()}))),
    ]);
    assert!(resolved("acme/a", &files).get("midi_of_sources").is_none());
}

#[test]
fn a_0_6_target_converted_on_resolving_gives_its_credit_in_midi_of_sources_with_the_converted_keys() {
    let files = catalogue(vec![
        ("acme/a", device(D07, json!({"midi_of": "acme/b", "sources": [src("A")]}))),
        ("acme/b", device(D06, json!({"cc_params": routes_06(), "_sources": [{"type": "vendor-doc", "name": "B", "revision": "3"}]}))),
    ]);
    let doc = resolved("acme/a", &files);
    assert_eq!(doc["$schema"], D07);
    assert_eq!(doc["midi_of_sources"], json!([{"type": "vendor-doc", "title": "B", "version": "3"}]));
    assert!(doc.get("_midi_of_sources").is_none() && doc.get("_sources").is_none(), "{doc}");
}

#[test]
fn a_midi_of_file_of_0_7_that_carries_underscore_sources_is_refused_in_its_own_file() {
    let files = catalogue(vec![
        ("acme/a", device(D07, json!({"midi_of": "acme/b", "_sources": [src("A")]}))),
        ("acme/b", device(D07, json!({"cc_params": routes(), "sources": [src("B")]}))),
    ]);
    let f = osd::resolve("acme/a", &files).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str(), f.file.as_deref()), (FaultKind::BadValue, "/_sources", Some("acme/a")));
}
