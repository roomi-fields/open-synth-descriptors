//! A control of 0.4 or 0.5 whose flat fields mix kinds of message (a CC, an NRPN, an RPN) reaches
//! 0.7 as one control with one way per message (`spec/0.7.md` 21.3, proposal 0009), a message not
//! fully known going to the note `_incomplete`. A block with no NRPN, the refusal toward 0.6 and
//! the controls of a fingerprint stay refused.

// @lit spec/0.7.md

use crate::common;

use osd::{Descriptor, Fault, FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const FLAT_VERSIONS: [Version; 2] = [Version::V0_4, Version::V0_5];

fn address(version: Version, kind: &str) -> String {
    format!("https://w3id.org/osd/{}/{kind}.schema.json", version.name())
}

/// `body` as a descriptor of `version`; a 0.4 text states no `$schema`.
fn of_version(version: Version, kind: &str, mut body: Value) -> Value {
    if version != Version::V0_4 {
        body["$schema"] = json!(address(version, kind));
    }
    body
}

fn device(version: Version, entries: Vec<Value>) -> Value {
    of_version(version, "device", json!({"device": {"name": "T", "osc_prefix": "/t"}, "cc_params": {"channel": 0, "entries": entries}}))
}

fn to_0_7(doc: &Value) -> Result<String, Fault> {
    osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7)
}

/// The 0.7 document of `doc`, checked to read as 0.7, to validate against the 0.7 schema and to
/// convert to itself.
fn converted(doc: &Value) -> Value {
    let out = to_0_7(doc).unwrap_or_else(|f| panic!("{f}\n{doc}"));
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["$schema"], address(Version::V0_7, "device"), "{doc}");
    osd::read_as(&out, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{out}"));
    assert_eq!(common::Schemas::load("0.7").check(Kind::Device, &value), None, "{out}");
    assert_eq!(osd::convert_to(&out, Kind::Device, Version::V0_7).as_deref(), Ok(out.as_str()), "converting the result changes it");
    value
}

/// A way of a CC: one number, or two with the 14 bits.
fn cc(index: &[i64]) -> Value {
    match index.len() {
        1 => json!({"ctrlType": "cc", "ctrlIndex": index}),
        _ => json!({"ctrlType": "cc", "ctrlIndex": index, "value_bits": 14}),
    }
}

/// A way of an NRPN or an RPN of 0.4 or 0.5, whose value has 14 bits.
fn parameter(kind: &str, msb: i64, lsb: i64) -> Value {
    json!({"ctrlType": kind, "ctrlIndex": [msb, lsb], "value_bits": 14})
}

fn merged(mut base: Value, more: &Value) -> Value {
    for (k, v) in more.as_object().unwrap() {
        base[k.as_str()] = v.clone();
    }
    base
}

/// Every set of two kinds or three, each message with all its numbers: the flat fields of the entry
/// and the ways they become.
fn complete_mixes() -> Vec<(Value, Vec<Value>)> {
    vec![
        (json!({"cc": 93, "nrpn_msb": 1, "nrpn_lsb": 102}), vec![cc(&[93]), parameter("nrpn", 1, 102)]),
        (json!({"nrpn_lsb": 0, "nrpn_msb": 0, "cc": 0}), vec![cc(&[0]), parameter("nrpn", 0, 0)]),
        (json!({"cc": 93, "rpn_msb": 0, "rpn_lsb": 2}), vec![cc(&[93]), parameter("rpn", 0, 2)]),
        (json!({"rpn_lsb": 6, "rpn_msb": 5, "nrpn_lsb": 4, "nrpn_msb": 3}), vec![parameter("nrpn", 3, 4), parameter("rpn", 5, 6)]),
        (json!({"cc": 1, "cc_lsb": 33, "nrpn_msb": 1, "nrpn_lsb": 8}), vec![cc(&[1, 33]), parameter("nrpn", 1, 8)]),
        (json!({"cc": 0, "cc_lsb": 32, "rpn_msb": 0, "rpn_lsb": 0}), vec![cc(&[0, 32]), parameter("rpn", 0, 0)]),
        (
            json!({"rpn_msb": 0, "rpn_lsb": 2, "nrpn_msb": 1, "nrpn_lsb": 102, "cc": 93}),
            vec![cc(&[93]), parameter("nrpn", 1, 102), parameter("rpn", 0, 2)],
        ),
        (
            json!({"rpn_msb": 127, "rpn_lsb": 127, "nrpn_msb": 127, "nrpn_lsb": 127, "cc_lsb": 39, "cc": 7}),
            vec![cc(&[7, 39]), parameter("nrpn", 127, 127), parameter("rpn", 127, 127)],
        ),
    ]
}

// ---- the ways written

#[test]
fn each_mix_of_complete_messages_of_0_4_or_0_5_becomes_one_control_with_its_ways_in_the_order_cc_nrpn_rpn() {
    for version in FLAT_VERSIONS {
        for (fields, ways) in complete_mixes() {
            let doc = device(version, vec![merged(json!({"osc": "/x"}), &fields)]);
            assert_eq!(converted(&doc)["cc_params"], json!({"channel": 0, "entries": [{"osc": "/x", "ways": ways}]}), "{doc}");
        }
    }
}

#[test]
fn the_range_of_a_mixed_control_is_on_every_way() {
    for version in FLAT_VERSIONS {
        for range in [[0, 127], [0, 16383], [10, 20]] {
            for (fields, ways) in complete_mixes() {
                let entry = merged(json!({"osc": "/x", "range": range}), &fields);
                let ways: Vec<Value> = ways.into_iter().map(|w| merged(w, &json!({"range": range}))).collect();
                let doc = device(version, vec![entry]);
                assert_eq!(converted(&doc)["cc_params"]["entries"], json!([{"osc": "/x", "ways": ways}]), "{doc}");
            }
        }
    }
}

#[test]
fn the_nrpn_lsb_range_of_a_mixed_control_is_on_its_nrpn_way_alone() {
    for version in FLAT_VERSIONS {
        let mut checked = 0;
        for (fields, ways) in complete_mixes() {
            if fields.get("nrpn_msb").is_none() {
                continue;
            }
            checked += 1;
            let entry = merged(json!({"osc": "/x", "nrpn_lsb_range": [0, 15], "range": [0, 127]}), &fields);
            let ways: Vec<Value> = ways
                .into_iter()
                .map(|w| {
                    let w = merged(w, &json!({"range": [0, 127]}));
                    if w["ctrlType"] == "nrpn" { merged(w, &json!({"nrpn_lsb_range": [0, 15]})) } else { w }
                })
                .collect();
            let doc = device(version, vec![entry]);
            assert_eq!(converted(&doc)["cc_params"]["entries"], json!([{"osc": "/x", "ways": ways}]), "{doc}");
        }
        assert!(checked >= 5);
    }
}

#[test]
fn the_other_fields_of_a_mixed_control_convert_as_those_of_a_control_with_one_message() {
    let own = json!({
        "osc": "/x", "channel": 3, "section": "Filter", "orientation": "centered",
        "_name": "Cutoff", "_description": "The cutoff.", "_usage": "0 closed", "_notes": "n", "_custom": {"a": [1]}
    });
    for version in FLAT_VERSIONS {
        let alone = converted(&device(version, vec![merged(own.clone(), &json!({"cc": 93}))]));
        let mut want = alone["cc_params"]["entries"][0].clone();
        assert!(want.get("polarity").is_some() && want.get("title").is_some(), "the control of one message converts its fields: {want}");
        want["ways"] = json!([cc(&[93]), parameter("nrpn", 1, 8)]);
        let doc = device(version, vec![merged(own.clone(), &json!({"cc": 93, "nrpn_msb": 1, "nrpn_lsb": 8}))]);
        assert_eq!(converted(&doc)["cc_params"]["entries"], json!([want]), "{doc}");
    }
}

#[test]
fn several_mixed_controls_of_one_file_each_become_their_own_ways_in_their_place() {
    for version in FLAT_VERSIONS {
        let doc = device(version, vec![
            json!({"osc": "/a", "cc": 1, "nrpn_msb": 1, "nrpn_lsb": 1}),
            json!({"osc": "/b", "cc": 2}),
            json!({"osc": "/c", "cc": 3, "rpn_msb": 0, "rpn_lsb": 3}),
        ]);
        assert_eq!(
            converted(&doc)["cc_params"]["entries"],
            json!([
                {"osc": "/a", "ways": [cc(&[1]), parameter("nrpn", 1, 1)]},
                {"osc": "/b", "ways": [cc(&[2])]},
                {"osc": "/c", "ways": [cc(&[3]), parameter("rpn", 0, 3)]}
            ])
        );
    }
}

// ---- the rest of the file

/// A file that changes in many places on the way to 0.7, with these entries after its own.
fn rich(version: Version, entries: Vec<Value>) -> Value {
    let mut all = vec![
        json!({"osc": "/centered", "cc": 1, "orientation": "centered", "_name": "N", "_usage": "u"}),
        json!({"osc": "/nrpn", "nrpn_msb": 2, "nrpn_lsb": 3}),
        json!({"osc": "/msb-alone", "nrpn_msb": 4, "range": [0, 9]}),
        json!({"osc": "/lsb-alone", "cc_lsb": 40}),
        json!({"osc": "/block", "cc": 7, "nrpn_lsb_range": [0, 15]}),
        json!({"osc": "/rpn", "rpn_msb": 0, "rpn_lsb": 1}),
    ];
    all.extend(entries);
    let mut doc = device(version, all);
    doc["identity"] = json!({"ports": ["Synth", ""]});
    doc["_sources"] = json!([{"type": "electra-preset", "url": "https://example.org/p/abc", "preset_id": "abc", "preset_name": "P"}]);
    doc
}

#[test]
fn the_rest_of_a_file_with_a_mixed_control_converts_as_in_the_file_without_it() {
    let mixed = json!({"osc": "/m", "cc": 93, "nrpn_msb": 1, "nrpn_lsb": 102});
    let ways = json!({"osc": "/m", "ways": [cc(&[93]), parameter("nrpn", 1, 102)]});
    for version in FLAT_VERSIONS {
        let without = converted(&rich(version, vec![]));
        for first in [false, true] {
            let mut doc = rich(version, vec![]);
            let mut want = without.clone();
            let list = doc["cc_params"]["entries"].as_array_mut().unwrap();
            let at = if first { 0 } else { list.len() };
            list.insert(at, mixed.clone());
            want["cc_params"]["entries"].as_array_mut().unwrap().insert(at, ways.clone());
            assert_eq!(converted(&doc), want, "{doc}");
        }
    }
}

#[test]
fn a_file_with_a_mixed_control_is_refused_for_the_rest_with_the_fault_it_has_without_it() {
    let mixed = json!({"osc": "/m", "cc": 93, "nrpn_msb": 1, "nrpn_lsb": 102});
    // The control that stands at the place of the mixed one in the file without it: same addresses.
    let plain = json!({"osc": "/m", "cc": 93});
    let others: [Vec<Value>; 4] = [
        vec![json!({"osc": "/o", "cc": 10, "orientation": "sideways"})],
        vec![json!({"osc": "/a", "cc": 20}), json!({"osc": "/b", "cc": 20})],
        vec![json!({"osc": "/a", "nrpn_msb": 9, "nrpn_lsb": 9}), json!({"osc": "/b", "nrpn_msb": 9, "nrpn_lsb": 9})],
        vec![json!({"osc": "/a", "cc": 20, "default": 1})],
    ];
    for version in FLAT_VERSIONS {
        for other in &others {
            for first in [false, true] {
                let place = |control: &Value| {
                    let mut list = other.clone();
                    let at = if first { 0 } else { list.len() };
                    list.insert(at, control.clone());
                    device(version, list)
                };
                let want = to_0_7(&place(&plain)).expect_err("refused without the mixed control");
                let doc = place(&mixed);
                let got = to_0_7(&doc).expect_err("refused with the mixed control");
                assert_eq!((got.kind, got.pointer.as_str()), (want.kind, want.pointer.as_str()), "{got}\n{doc}");
            }
        }
    }
}

// ---- the mixes with a message not fully known

/// Mixes with a message no way says: the flat fields, the ways of the complete messages and the
/// incomplete ones, each in the order CC, NRPN, RPN.
fn incomplete_mixes() -> Vec<(Value, Vec<Value>, Vec<Value>)> {
    vec![
        (json!({"cc": 93, "nrpn_msb": 1}), vec![cc(&[93])], vec![json!({"nrpn_msb": 1})]),
        (json!({"cc": 93, "nrpn_lsb": 0}), vec![cc(&[93])], vec![json!({"nrpn_lsb": 0})]),
        (json!({"cc": 93, "rpn_msb": 0}), vec![cc(&[93])], vec![json!({"rpn_msb": 0})]),
        (json!({"cc": 93, "rpn_lsb": 2}), vec![cc(&[93])], vec![json!({"rpn_lsb": 2})]),
        (json!({"cc_lsb": 33, "nrpn_msb": 1, "nrpn_lsb": 2}), vec![parameter("nrpn", 1, 2)], vec![json!({"cc_lsb": 33})]),
        (json!({"cc_lsb": 0, "rpn_msb": 0, "rpn_lsb": 1}), vec![parameter("rpn", 0, 1)], vec![json!({"cc_lsb": 0})]),
        (json!({"nrpn_msb": 1, "nrpn_lsb": 2, "rpn_msb": 0}), vec![parameter("nrpn", 1, 2)], vec![json!({"rpn_msb": 0})]),
        (json!({"nrpn_lsb": 2, "rpn_msb": 0, "rpn_lsb": 1}), vec![parameter("rpn", 0, 1)], vec![json!({"nrpn_lsb": 2})]),
        (json!({"rpn_lsb": 1, "nrpn_msb": 1}), vec![], vec![json!({"nrpn_msb": 1}), json!({"rpn_lsb": 1})]),
        (json!({"nrpn_lsb": 2, "cc_lsb": 33}), vec![], vec![json!({"cc_lsb": 33}), json!({"nrpn_lsb": 2})]),
        (
            json!({"cc": 93, "nrpn_msb": 1, "nrpn_lsb": 2, "rpn_lsb": 2}),
            vec![cc(&[93]), parameter("nrpn", 1, 2)],
            vec![json!({"rpn_lsb": 2})],
        ),
    ]
}

#[test]
fn a_mixed_control_with_an_incomplete_message_converts_to_its_complete_ways_and_the_note() {
    for version in FLAT_VERSIONS {
        for (fields, ways, note) in incomplete_mixes() {
            let doc = device(version, vec![json!({"osc": "/first", "cc": 1}), merged(json!({"osc": "/x"}), &fields)]);
            let want = json!({"osc": "/x", "ways": ways, "_incomplete": note});
            assert_eq!(converted(&doc)["cc_params"]["entries"][1], want, "{doc}");
        }
    }
}

#[test]
fn the_range_of_a_mixed_control_with_an_incomplete_message_is_on_its_ways_and_in_the_note_else_on_the_entry() {
    for version in FLAT_VERSIONS {
        for (fields, ways, note) in incomplete_mixes() {
            let range = json!({"range": [10, 20]});
            let want = if ways.is_empty() {
                json!({"osc": "/x", "ways": [], "range": [10, 20], "_incomplete": note})
            } else {
                let ways: Vec<Value> = ways.into_iter().map(|w| merged(w, &range)).collect();
                let note: Vec<Value> = note.into_iter().map(|n| merged(n, &range)).collect();
                json!({"osc": "/x", "ways": ways, "_incomplete": note})
            };
            let doc = device(version, vec![merged(json!({"osc": "/x", "range": [10, 20]}), &fields)]);
            assert_eq!(converted(&doc)["cc_params"]["entries"][0], want, "{doc}");
        }
    }
}

#[test]
fn the_block_of_a_mixed_control_with_an_incomplete_nrpn_goes_to_no_way() {
    for version in FLAT_VERSIONS {
        let doc = device(version, vec![
            json!({"osc": "/a", "cc": 93, "nrpn_msb": 1, "nrpn_lsb_range": [0, 15]}),
            json!({"osc": "/b", "cc_lsb": 33, "nrpn_lsb": 2, "nrpn_lsb_range": [0, 15]}),
        ]);
        assert_eq!(
            converted(&doc)["cc_params"]["entries"],
            json!([
                {"osc": "/a", "ways": [cc(&[93])], "_nrpn_lsb_range": [0, 15], "_incomplete": [{"nrpn_msb": 1}]},
                {"osc": "/b", "ways": [], "nrpn_lsb_range": [0, 15], "_incomplete": [{"cc_lsb": 33}, {"nrpn_lsb": 2}]}
            ]),
            "{doc}"
        );
    }
}

#[test]
fn the_incomplete_message_of_a_mixed_control_claims_nothing() {
    for version in FLAT_VERSIONS {
        converted(&device(version, vec![
            json!({"osc": "/a", "cc": 93, "nrpn_msb": 1}),
            json!({"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 0}),
        ]));
    }
}

#[test]
fn a_mixed_control_with_a_block_and_no_nrpn_is_not_convertible_to_0_7_at_the_control() {
    for version in FLAT_VERSIONS {
        for mix in [
            json!({"cc": 93, "rpn_msb": 0, "rpn_lsb": 2, "nrpn_lsb_range": [0, 15]}),
            json!({"cc": 93, "rpn_msb": 0, "nrpn_lsb_range": [0, 15]}),
        ] {
            let doc = device(version, vec![json!({"osc": "/first", "cc": 1}), merged(json!({"osc": "/x"}), &mix)]);
            match to_0_7(&doc) {
                Err(f) => assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, "/cc_params/entries/1"), "{f}\n{doc}"),
                Ok(out) => panic!("converted: {doc}\ninto {out}"),
            }
        }
    }
}

// ---- a message claimed twice

/// The entries, the index of the later claimant and of the earlier one.
fn double_claims() -> Vec<(Vec<Value>, usize, usize)> {
    let mixed = json!({"osc": "/m", "cc": 74, "nrpn_msb": 1, "nrpn_lsb": 8});
    vec![
        (vec![mixed.clone(), json!({"osc": "/b", "cc": 74})], 1, 0),
        (vec![mixed.clone(), json!({"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 8})], 1, 0),
        (vec![json!({"osc": "/a", "cc": 74}), mixed.clone()], 1, 0),
        (vec![json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 8}), mixed.clone()], 1, 0),
        (vec![mixed.clone(), json!({"osc": "/n", "cc": 75, "nrpn_msb": 1, "nrpn_lsb": 8})], 1, 0),
        (vec![json!({"osc": "/m", "cc": 1, "cc_lsb": 33, "rpn_msb": 0, "rpn_lsb": 1}), json!({"osc": "/b", "cc": 33})], 1, 0),
        (vec![json!({"osc": "/m", "nrpn_msb": 0, "nrpn_lsb": 0, "rpn_msb": 0, "rpn_lsb": 0}), json!({"osc": "/b", "rpn_msb": 0, "rpn_lsb": 0})], 1, 0),
        (vec![json!({"osc": "/f", "cc": 5}), mixed.clone(), json!({"osc": "/g", "cc": 6}), json!({"osc": "/b", "cc": 74})], 3, 1),
        // The channel of the entry is the table's, written out.
        (vec![mixed, json!({"osc": "/b", "channel": 0, "cc": 74})], 1, 0),
    ]
}

#[test]
fn a_message_claimed_by_a_mixed_control_and_another_refuses_the_later_one_and_names_the_earlier() {
    for version in FLAT_VERSIONS {
        for (entries, later, earlier) in double_claims() {
            let doc = device(version, entries);
            let f = match to_0_7(&doc) {
                Err(f) => f,
                Ok(out) => panic!("converted: {doc}\ninto {out}"),
            };
            assert_eq!((f.kind, f.pointer.clone()), (FaultKind::NotConvertible, format!("/cc_params/entries/{later}")), "{f}\n{doc}");
            assert!(f.rule.contains(&format!("/cc_params/entries/{earlier}")), "the rule names the earlier control: {f}\n{doc}");
        }
    }
}

#[test]
fn the_messages_of_a_mixed_control_on_another_channel_or_of_another_kind_are_no_second_claim() {
    let mixed = json!({"osc": "/m", "cc": 74, "nrpn_msb": 1, "nrpn_lsb": 8});
    for version in FLAT_VERSIONS {
        for other in [
            json!({"osc": "/b", "channel": 1, "cc": 74}),
            json!({"osc": "/b", "channel": 1, "cc": 74, "nrpn_msb": 1, "nrpn_lsb": 8}),
            json!({"osc": "/b", "rpn_msb": 1, "rpn_lsb": 8}),
            json!({"osc": "/b", "cc": 8, "nrpn_msb": 8, "nrpn_lsb": 1}),
        ] {
            let doc = device(version, vec![mixed.clone(), other]);
            let out = converted(&doc);
            assert_eq!(out["cc_params"]["entries"][0]["ways"], json!([cc(&[74]), parameter("nrpn", 1, 8)]), "{doc}");
        }
    }
}

// ---- the controls of a fingerprint

#[test]
fn a_fingerprint_control_that_mixes_kinds_is_not_convertible_to_0_7_as_to_0_6() {
    // The controls of a 0.5 fingerprint know `cc`, `nrpn_msb` and `nrpn_lsb`: their mixes are a CC and an NRPN.
    for fields in [json!({"cc": 93, "nrpn_msb": 1, "nrpn_lsb": 102}), json!({"cc": 0, "nrpn_msb": 0})] {
        for (key, items, pointer) in [
            ("controls", json!([{"cc": 100}, fields.clone()]), "/controls/1"),
            ("names", json!([{"control": fields.clone(), "name": "cutoff"}]), "/names/0/control"),
        ] {
            let mut doc = of_version(Version::V0_5, "fingerprint", json!({
                "device": {"vendor": "Example", "model": "Mono One", "firmware": "1.0"},
                "observed": "2026-10-27", "via": "web-midi", "ports": {"inputs": ["t"], "outputs": ["t"]}
            }));
            doc[key] = items;
            for target in [Version::V0_6, Version::V0_7] {
                let f = osd::convert_to(&doc.to_string(), Kind::Fingerprint, target).expect_err("a fingerprint control names one kind");
                assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, pointer), "{f}\n{doc}");
            }
        }
    }
}

// ---- resolution

fn catalogue(files: &[(&str, &Value)]) -> BTreeMap<String, String> {
    files.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

/// The resolved device of `acme/a`, as written.
fn resolved(files: &BTreeMap<String, String>) -> Value {
    let r = osd::resolve("acme/a", files).unwrap_or_else(|f| panic!("{f}"));
    serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap()
}

/// A variant of the target `device` writes: a `midi_of` names a driver of the same device (23.18).
fn variant(version: Version) -> Value {
    of_version(version, "device", json!({"device": {"name": "T", "osc_prefix": "/v"}, "midi_of": "acme/b"}))
}

fn profile_0_7() -> Value {
    of_version(Version::V0_7, "profile", json!({"identity": {"ports": [{"operator": "equals", "value": "Synth"}]}}))
}

fn mixed_target(version: Version) -> Value {
    device(version, vec![json!({"osc": "/x", "cc": 93, "nrpn_msb": 1, "nrpn_lsb": 102, "range": [0, 127]})])
}

fn mixed_target_ways() -> Value {
    json!([{"osc": "/x", "ways": [
        {"ctrlType": "cc", "ctrlIndex": [93], "range": [0, 127]},
        {"ctrlType": "nrpn", "ctrlIndex": [1, 102], "value_bits": 14, "range": [0, 127]}
    ]}])
}

fn assert_resolved_into_ways(files: &BTreeMap<String, String>) {
    let out = resolved(files);
    assert_eq!(out["$schema"], address(Version::V0_7, "device"));
    assert_eq!(out["cc_params"]["entries"], mixed_target_ways());
}

#[test]
fn a_0_7_file_joined_to_a_0_4_or_0_5_target_with_a_mixed_control_resolves_into_its_ways() {
    for version in FLAT_VERSIONS {
        assert_resolved_into_ways(&catalogue(&[("acme/a", &variant(Version::V0_7)), ("acme/b", &mixed_target(version))]));
    }
}

#[test]
fn a_0_4_or_0_5_device_with_a_mixed_control_joined_to_a_0_7_profile_resolves_into_its_ways() {
    for version in FLAT_VERSIONS {
        let mut d = mixed_target(version);
        d["identity"] = json!({"profile": "acme"});
        assert_resolved_into_ways(&catalogue(&[("acme/a", &d), ("acme/_profile", &profile_0_7())]));
    }
}

#[test]
fn a_0_5_file_on_a_0_4_target_with_a_mixed_control_joined_to_a_0_7_profile_resolves_into_its_ways() {
    let mut v = variant(Version::V0_5);
    v["identity"] = json!({"profile": "acme"});
    assert_resolved_into_ways(&catalogue(&[("acme/a", &v), ("acme/b", &mixed_target(Version::V0_4)), ("acme/_profile", &profile_0_7())]));
}

fn assert_refused_in_the_target(version: Version, target: &Value) {
    let files = catalogue(&[("acme/a", &variant(version)), ("acme/b", target)]);
    let f = match osd::resolve("acme/a", &files) {
        Err(f) => f,
        Ok(r) => panic!("{version:?}: resolved into {}", osd::write(&Descriptor::Device(Box::new(r.device)))),
    };
    assert_eq!(
        (f.kind, f.file.as_deref(), f.pointer.as_str()),
        (FaultKind::NotConvertible, Some("acme/b"), "/cc_params/entries/0"),
        "{f}"
    );
}

#[test]
fn a_0_6_file_on_a_target_with_a_mixed_control_is_refused_in_the_target_at_the_control() {
    for version in FLAT_VERSIONS {
        assert_refused_in_the_target(Version::V0_6, &mixed_target(version));
    }
}

#[test]
fn a_0_7_file_on_a_target_with_an_incomplete_mix_resolves_into_its_ways_and_the_note() {
    for version in FLAT_VERSIONS {
        let target = device(version, vec![json!({"osc": "/x", "cc": 93, "nrpn_msb": 1})]);
        let files = catalogue(&[("acme/a", &variant(Version::V0_7)), ("acme/b", &target)]);
        assert_eq!(
            resolved(&files)["cc_params"]["entries"],
            json!([{"osc": "/x", "ways": [cc(&[93])], "_incomplete": [{"nrpn_msb": 1}]}])
        );
    }
}

// ---- the text of the version

/// The lines of the section of `spec/0.7.md` that starts with `heading`, up to the next heading.
fn section(heading: &str) -> String {
    let spec = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    let lines: Vec<&str> = spec.lines().skip_while(|l| !l.starts_with(heading)).skip(1).take_while(|l| !l.starts_with('#')).collect();
    assert!(!lines.is_empty(), "no section {heading}");
    lines.join("\n")
}

#[test]
fn the_section_on_one_message_kind_per_control_cites_the_cases_of_a_mixed_control() {
    let text = section("### 21.3 ");
    let missing: Vec<String> = [
        "mixed-cc-and-nrpn-control-of-0-4-becomes-two-ways",
        "mixed-cc-and-nrpn-control-of-0-5-becomes-two-ways",
        "mixed-cc-and-rpn-control-becomes-two-ways",
        "mixed-nrpn-and-rpn-control-becomes-two-ways",
        "mixed-three-kinds-control-becomes-three-ways",
        "mixed-control-with-nrpn-of-one-number-kept-as-note",
        "mixed-control-with-cc-lsb-without-cc-kept-as-note",
        "mixed-control-with-block-without-nrpn-refused",
        "mixed-control-message-claimed-twice-refused",
        "fingerprint-control-mixing-cc-and-nrpn-refused",
        "fingerprint-name-control-mixing-cc-and-nrpn-refused",
    ]
    .iter()
    .map(|name| format!("0.7/convert/{name}"))
    .filter(|case| !text.contains(&format!("`{case}`")))
    .collect();
    assert!(missing.is_empty(), "21.3 of spec/0.7.md does not cite {missing:?}");
}

#[test]
fn what_the_converter_writes_refers_to_the_rule_of_a_mixed_control() {
    let ways = section("### 5.3.3 ");
    let paragraph = ways.lines().find(|l| l.starts_with("**What the converter writes.**")).expect("the paragraph of what the converter writes");
    assert!(paragraph.contains("21.3"), "{paragraph}");
}
