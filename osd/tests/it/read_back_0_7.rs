//! The converter writes in 0.7 nothing its own reader refuses (`spec/0.7.md` 2.4): from 0.4, 0.5
//! or 0.6, such a text is refused as `NotConvertible`, at the place and with the rule in the words
//! of the text given, and a resolution that carries it to 0.7 is refused with that fault (23.22).

use osd::{Descriptor, Fault, FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const ALL: [Version; 3] = [Version::V0_4, Version::V0_5, Version::V0_6];

fn address(version: Version, kind: &str) -> String {
    format!("https://w3id.org/osd/{}/{kind}.schema.json", version.name())
}

/// `body` as a device of `version`; a 0.4 text states no `$schema`.
fn of_version(version: Version, mut body: Value) -> Value {
    if body.get("device").is_none() {
        body["device"] = json!({"name": "T", "osc_prefix": "/t"});
    }
    if version != Version::V0_4 {
        body["$schema"] = json!(address(version, "device"));
    }
    body
}

fn controls(version: Version, entries: Value) -> Value {
    of_version(version, json!({"cc_params": {"channel": 0, "entries": entries}}))
}

/// An NRPN way, its width said (0.6).
fn nrpn(msb: u8, lsb: Option<u8>) -> Value {
    let mut way = json!({"nrpn_msb": msb, "value_bits": 14});
    if let Some(lsb) = lsb {
        way["nrpn_lsb"] = json!(lsb);
    }
    way
}

fn with(mut base: Value, more: Value) -> Value {
    for (k, v) in more.as_object().unwrap() {
        base[k.as_str()] = v.clone();
    }
    base
}

/// The refusal of the conversion of `doc` to 0.7: no text, `NotConvertible`, at `pointer`.
#[track_caller]
fn refused_in_0_7(doc: &Value, pointer: &str) -> Fault {
    let text = doc.to_string();
    match osd::convert_to(&text, Kind::Device, Version::V0_7) {
        Ok(out) => panic!("converted: {doc}\ninto {out}"),
        Err(f) => {
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, pointer), "{f}\n{doc}");
            f
        }
    }
}

/// `doc` converts to a text read as 0.7, and to a text of 0.6.
#[track_caller]
fn converts(doc: &Value) {
    let text = doc.to_string();
    let out = osd::convert_to(&text, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{doc}"));
    osd::read_as(&out, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{out}"));
    let older = osd::convert_to(&text, Kind::Device, Version::V0_6).unwrap_or_else(|f| panic!("{f}\n{doc}"));
    osd::read_as(&older, Kind::Device, Version::V0_6).unwrap_or_else(|f| panic!("{f}\n{older}"));
}

/// The rule the reader of 0.7 gives for `body`, a faulty device of 0.7.
fn reader_rule(body: Value) -> String {
    let doc = of_version(Version::V0_7, body);
    osd::read(&doc.to_string(), Kind::Device).expect_err("a faulty text of 0.7").rule
}

/// [`refused_in_0_7`], of a text that is still converted to 0.6.
#[track_caller]
fn refused(doc: &Value, pointer: &str) -> Fault {
    if let Err(f) = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_6) {
        panic!("0.6 accepts the text: {f}\n{doc}");
    }
    refused_in_0_7(doc, pointer)
}

// ---- a default and a named value outside the scale of the ways

#[test]
fn a_default_outside_the_scale_of_its_ways_is_not_convertible_at_the_default_with_the_readers_rule() {
    let want = reader_rule(json!({"cc_params": {"entries": [
        {"osc": "/a", "default": 200, "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}
    ]}}));
    let f = refused(&controls(Version::V0_6, json!([{"osc": "/a", "cc": 1, "default": 200}])), "/cc_params/entries/0/default");
    assert_eq!(f.rule, want);
    // A negative default, on a later control.
    let doc = controls(Version::V0_6, json!([{"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 2}, {"osc": "/c", "cc": 3, "default": -1}]));
    refused(&doc, "/cc_params/entries/2/default");
    // An NRPN of 7 bits.
    let doc = controls(Version::V0_6, json!([{"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 7, "default": 128}]));
    refused(&doc, "/cc_params/entries/0/default");
    // The scale written by the converter is that of the first way, which the default leaves.
    let doc = controls(Version::V0_6, json!([{"osc": "/a", "cc": 1, "default": 200, "also": [nrpn(1, Some(2))]}]));
    refused(&doc, "/cc_params/entries/0/default");
    // A control given by `also` alone.
    let doc = controls(Version::V0_6, json!([{"osc": "/a", "default": 200, "also": [{"cc": 1}]}]));
    refused(&doc, "/cc_params/entries/0/default");
}

#[test]
fn a_named_value_outside_the_scale_is_not_convertible_at_its_label_in_the_text_given() {
    // The faulty value is the second written and not the second of the labels in order.
    let doc = controls(Version::V0_6, json!([
        {"osc": "/a", "cc": 1},
        {"osc": "/b", "cc": 2, "values": {"z": 1, "high": 200, "a": 2}}
    ]));
    let f = refused(&doc, "/cc_params/entries/1/values/high");
    let want = reader_rule(json!({"cc_params": {"entries": [
        {"osc": "/b", "values": [{"value": 200, "title": "high"}], "ways": [{"ctrlType": "cc", "ctrlIndex": [2]}]}
    ]}}));
    assert_eq!(f.rule, want);
}

#[test]
fn the_label_of_a_refused_named_value_is_escaped_as_a_json_pointer_says() {
    for (label, escaped) in [("On/Off", "On~1Off"), ("a~b", "a~0b"), ("~/1", "~0~11"), ("two words", "two words"), ("0", "0")] {
        let mut values = serde_json::Map::new();
        values.insert("first".into(), json!(0));
        values.insert(label.into(), json!(-3));
        let doc = controls(Version::V0_6, json!([{"osc": "/a", "cc": 1, "values": values}]));
        refused(&doc, &format!("/cc_params/entries/0/values/{escaped}"));
    }
}

#[test]
fn defaults_and_named_values_inside_the_scale_of_the_ways_convert_into_a_text_0_7_reads() {
    for entry in [
        json!({"osc": "/a", "cc": 1, "default": 127, "values": {"min": 0, "max": 127}}),
        json!({"osc": "/a", "cc": 1, "cc_lsb": 33, "default": 200, "values": {"max": 16383}}),
        json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "default": 16383}),
        json!({"osc": "/a", "cc": 1, "range": [0, 300], "default": 200, "values": {"On/Off": 300}}),
        json!({"osc": "/a", "cc": 1, "default": 100, "also": [nrpn(1, Some(2))]}),
        // No message and no range: no known scale.
        json!({"osc": "/a", "default": 200, "values": {"x": -5}}),
    ] {
        converts(&controls(Version::V0_6, json!([entry])));
    }
    // A `params` entry whose ways give no scale.
    converts(&of_version(Version::V0_6, json!({"params": {
        "get_frame": [240, {"arg": "p"}, 247],
        "set_frame": [240, {"arg": "p"}, {"arg": "value"}, 247],
        "value_type": "u7",
        "entries": [{"osc": "/a", "p": 1, "default": 900, "values": {"x": 999}}]
    }})));
}

// ---- two controls that claim one message

#[track_caller]
fn assert_claim(f: &Fault, doc: &Value, earlier: &str) {
    assert!(f.rule.contains("already claimed"), "the reader's rule: {f}");
    assert!(f.rule.contains(&format!("at {earlier}:")), "the rule names {earlier}: {f}\n{doc}");
    assert!(!f.rule.contains("/ways/"), "a place of the text given: {f}");
}

#[test]
fn a_claim_of_a_control_that_mixes_kinds_in_0_4_or_0_5_is_named_at_the_control() {
    let mixed = json!({"osc": "/m", "cc": 5, "nrpn_msb": 1, "nrpn_lsb": 0});
    let alone = json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 0});
    for version in [Version::V0_4, Version::V0_5] {
        for entries in [json!([alone, mixed]), json!([mixed, alone])] {
            // 0.6 refuses a control that mixes kinds: 0.7 alone converts it.
            let doc = controls(version, entries);
            assert_claim(&refused_in_0_7(&doc, "/cc_params/entries/1"), &doc, "/cc_params/entries/0");
        }
    }
}

#[test]
fn nrpn_controls_whose_ways_claim_distinct_messages_convert_from_each_version() {
    for version in ALL {
        let mut entries = vec![
            json!({"osc": "/a", "nrpn_msb": 1}),
            json!({"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 1}),
            json!({"osc": "/c", "rpn_msb": 1, "rpn_lsb": 0}),
            json!({"osc": "/d", "channel": 1, "nrpn_msb": 1, "nrpn_lsb": 0}),
        ];
        if version == Version::V0_6 {
            entries.iter_mut().for_each(|e| e["value_bits"] = json!(14));
        }
        converts(&controls(version, Value::Array(entries)));
    }
}

// ---- the sources

fn source(id: &str) -> Value {
    json!({"type": "vendor-doc", "id": id})
}

#[test]
fn two_sources_of_one_id_are_not_convertible_at_the_later_source_of_the_text_given_from_each_version() {
    let want = reader_rule(json!({"sources": [source("manual"), source("manual")]}));
    for version in ALL {
        let doc = of_version(version, json!({"_sources": [source("manual"), source("manual")]}));
        assert_eq!(refused(&doc, "/_sources/1").rule, want);
        let doc = of_version(version, json!({"_sources": [source("a"), source("b"), source("a")]}));
        refused(&doc, "/_sources/2");
        let doc = of_version(version, json!({"_midi_of_sources": [source("a"), source("a")]}));
        refused(&doc, "/_midi_of_sources/1");
        let doc = of_version(version, json!({"_sources": [source("a")], "_midi_of_sources": [source("b"), source("a")]}));
        refused(&doc, "/_midi_of_sources/1");
    }
}

#[test]
fn a_source_id_0_7_refuses_is_not_convertible_at_the_id_of_the_source_of_the_text_given() {
    for version in ALL {
        for id in [json!(""), json!(3)] {
            let bad = json!({"type": "vendor-doc", "id": id});
            refused(&of_version(version, json!({"_sources": [source("a"), bad.clone()]})), "/_sources/1/id");
            refused(&of_version(version, json!({"_midi_of_sources": [bad]})), "/_midi_of_sources/0/id");
        }
    }
}

#[test]
fn sources_of_distinct_ids_or_without_an_id_convert_from_each_version() {
    for version in ALL {
        converts(&of_version(version, json!({
            "_sources": [source("a"), {"type": "vendor-doc"}, {"type": "vendor-doc"}],
            "_midi_of_sources": [source("b")]
        })));
    }
}

// ---- resolution

fn catalogue(files: &[(&str, &Value)]) -> BTreeMap<String, String> {
    files.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

/// A file of `version` that follows `target`.
fn variant(version: Version, target: &str, body: Value) -> Value {
    of_version(version, with(json!({"midi_of": target}), body))
}

fn profile_0_7() -> Value {
    json!({"$schema": address(Version::V0_7, "profile"), "identity": {"ports": [{"operator": "equals", "value": "Synth"}]}})
}

fn table(entries: Value) -> Value {
    json!({"cc_params": {"entries": entries}})
}

/// The entries of a 0.7 file.
fn ways_table(osc: &str, cc: u8) -> Value {
    table(json!([{"osc": osc, "ways": [{"ctrlType": "cc", "ctrlIndex": [cc]}]}]))
}

fn faulty_default() -> Value {
    table(json!([{"osc": "/ok", "cc": 1}, {"osc": "/x", "cc": 2, "default": 200}]))
}

/// The resolution of `acme/a` is refused as `kind` in `file` at `pointer`.
#[track_caller]
fn unresolved(files: &[(&str, &Value)], kind: FaultKind, file: &str, pointer: &str) -> Fault {
    match osd::resolve("acme/a", &catalogue(files)) {
        Ok(r) => panic!("resolved: {}", osd::write(&Descriptor::Device(Box::new(r.device)))),
        Err(f) => {
            assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (kind, Some(file), pointer), "{f}");
            f
        }
    }
}

#[test]
fn resolving_refuses_a_target_0_7_cannot_convert_in_the_target_at_its_pointer() {
    let b = of_version(Version::V0_6, faulty_default());
    for own in [json!({}), ways_table("/own", 9)] {
        let a = variant(Version::V0_7, "acme/b", own);
        unresolved(&[("acme/a", &a), ("acme/b", &b)], FaultKind::NotConvertible, "acme/b", "/cc_params/entries/1/default");
    }
}

#[test]
fn resolving_refuses_a_target_0_7_cannot_convert_even_when_the_file_replaces_the_faulty_entry() {
    let b = of_version(Version::V0_6, faulty_default());
    let a = variant(Version::V0_7, "acme/b", ways_table("/x", 2));
    unresolved(&[("acme/a", &a), ("acme/b", &b)], FaultKind::NotConvertible, "acme/b", "/cc_params/entries/1/default");
}

#[test]
fn resolving_names_the_claim_of_a_refused_target_as_its_text_writes_it() {
    // A control that mixes kinds, which 0.7 alone converts, claims the NRPN of another.
    let b = controls(Version::V0_5, json!([
        {"osc": "/a", "cc": 2, "nrpn_msb": 1, "nrpn_lsb": 0},
        {"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 0}
    ]));
    let a = variant(Version::V0_7, "acme/b", json!({}));
    let f = unresolved(&[("acme/a", &a), ("acme/b", &b)], FaultKind::NotConvertible, "acme/b", "/cc_params/entries/1");
    assert!(f.rule.contains("at /cc_params/entries/0:"), "{f}");
}

#[test]
fn resolving_refuses_a_file_0_7_cannot_convert_over_a_target_of_0_7_in_that_file() {
    let b = of_version(Version::V0_7, ways_table("/t", 7));
    let a = variant(Version::V0_6, "acme/b", table(json!([{"osc": "/x", "cc": 2, "default": 200}])));
    unresolved(&[("acme/a", &a), ("acme/b", &b)], FaultKind::NotConvertible, "acme/a", "/cc_params/entries/0/default");
}

#[test]
fn resolving_refuses_a_deeper_target_of_a_chain_0_7_cannot_convert_in_its_file() {
    let a = variant(Version::V0_7, "acme/b", json!({}));
    let b = variant(Version::V0_6, "acme/c", json!({}));
    let c = of_version(Version::V0_6, json!({"_sources": [source("m"), source("m")]}));
    unresolved(&[("acme/a", &a), ("acme/b", &b), ("acme/c", &c)], FaultKind::NotConvertible, "acme/c", "/_sources/1");
}

#[test]
fn resolving_refuses_a_driver_carried_to_0_7_by_a_profile_when_0_7_cannot_convert_it() {
    let a = of_version(Version::V0_6, with(faulty_default(), json!({"identity": {"profile": "acme"}})));
    unresolved(&[("acme/a", &a), ("acme/_profile", &profile_0_7())], FaultKind::NotConvertible, "acme/a", "/cc_params/entries/1/default");
}

#[test]
fn resolving_refuses_a_joined_text_carried_to_0_7_in_the_file_asked_for_at_the_pointer_of_the_joined_text() {
    let profile = profile_0_7();
    let identity = json!({"identity": {"profile": "acme"}});
    // One `id` in the file and its target: the target's source is the joined `_midi_of_sources`.
    let b = of_version(Version::V0_6, json!({"_sources": [source("manual")]}));
    let a = variant(Version::V0_6, "acme/b", with(identity.clone(), json!({"_sources": [source("manual")]})));
    unresolved(&[("acme/a", &a), ("acme/b", &b), ("acme/_profile", &profile)], FaultKind::NotConvertible, "acme/a", "/_midi_of_sources/0");
    // The file's own control stands after the target's in the joined table.
    let b = controls(Version::V0_6, json!([{"osc": "/t", "cc": 7}, {"osc": "/u", "cc": 8}]));
    let a = variant(Version::V0_6, "acme/b", with(identity.clone(), table(json!([{"osc": "/x", "cc": 2, "default": 200}]))));
    unresolved(&[("acme/a", &a), ("acme/b", &b), ("acme/_profile", &profile)], FaultKind::NotConvertible, "acme/a", "/cc_params/entries/2/default");
}

#[test]
fn an_incomplete_message_of_a_target_and_the_same_with_a_number_0_resolve_together_into_0_7() {
    let b = of_version(Version::V0_6, table(json!([with(json!({"osc": "/t"}), nrpn(1, None))])));
    let identity = json!({"identity": {"profile": "acme"}});
    let a = variant(Version::V0_6, "acme/b", with(identity, table(json!([with(json!({"osc": "/x"}), nrpn(1, Some(0)))]))));
    let files = catalogue(&[("acme/a", &a), ("acme/b", &b), ("acme/_profile", &profile_0_7())]);
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap();
    assert_eq!(
        out["cc_params"]["entries"],
        json!([
            {"osc": "/t", "ways": [], "_incomplete": [{"nrpn_msb": 1, "value_bits": 14}]},
            {"osc": "/x", "ways": [{"ctrlType": "nrpn", "ctrlIndex": [1, 0], "value_bits": 14}]}
        ])
    );
}

#[test]
fn a_fault_of_joining_texts_of_0_7_or_converted_one_by_one_stays_a_bad_value() {
    // The same source `id` in a file of 0.7 and a target of 0.6 that converts.
    let b = of_version(Version::V0_6, json!({"_sources": [source("manual")]}));
    let a = variant(Version::V0_7, "acme/b", json!({"sources": [source("manual")]}));
    unresolved(&[("acme/a", &a), ("acme/b", &b)], FaultKind::BadValue, "acme/a", "/sources/0");
    // A message claimed by the file and by the converted target.
    let b = controls(Version::V0_6, json!([{"osc": "/t", "cc": 7}]));
    let a = variant(Version::V0_7, "acme/b", ways_table("/x", 7));
    unresolved(&[("acme/a", &a), ("acme/b", &b)], FaultKind::BadValue, "acme/a", "/cc_params/entries/1/ways/0");
}

#[test]
fn a_fault_the_version_before_already_refuses_in_the_joined_text_stays_a_bad_value() {
    let b = controls(Version::V0_6, json!([{"osc": "/t", "cc": 7}]));
    let a = variant(Version::V0_6, "acme/b", with(table(json!([{"osc": "/x", "cc": 7}])), json!({"identity": {"profile": "acme"}})));
    unresolved(&[("acme/a", &a), ("acme/b", &b), ("acme/_profile", &profile_0_7())], FaultKind::BadValue, "acme/a", "/cc_params/entries/1");
}
