//! No conversion to 0.7 drops a field of a control without saying it (`spec/0.7.md` 5.3.3, what
//! the converter writes). The `nrpn_lsb_range` of a control that has ways goes to its NRPN way: the
//! one of the flat fields, else the single NRPN way of `also`; when no single way can receive it,
//! it stays on the entry as the note `_nrpn_lsb_range`. A `range` that no way takes, beside a
//! non-empty `also` and no flat message, stays on the entry as the note `_range`; it is the
//! reference scale of the control, written as `scale` when a `default`, `values` or `step_count`
//! need it. A message not fully known has no way and keeps its fields in the note `_incomplete`.

// @lit spec/0.7.md spec/proposals/0009-a-control-reaches-its-messages-by-ways.md osd/docs/INTERFACE.md

use crate::common;

use osd::{Descriptor, Fault, FaultKind, Kind, Version};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

const ALL: [Version; 3] = [Version::V0_4, Version::V0_5, Version::V0_6];

/// The `nrpn_lsb_range` of every case here.
const BLOCK: [i64; 2] = [0, 7];

fn address(version: Version) -> String {
    format!("https://w3id.org/osd/{}/device.schema.json", version.name())
}

/// `body` as a device of `version`; a 0.4 text states no `$schema`.
fn of_version(version: Version, mut body: Value) -> Value {
    if body.get("device").is_none() {
        body["device"] = json!({"name": "T", "osc_prefix": "/t"});
    }
    if version != Version::V0_4 {
        body["$schema"] = json!(address(version));
    }
    body
}

fn controls(version: Version, entries: Vec<Value>) -> Value {
    of_version(version, json!({"cc_params": {"channel": 0, "entries": entries}}))
}

fn with(mut base: Value, more: &Value) -> Value {
    for (k, v) in more.as_object().unwrap() {
        base[k.as_str()] = v.clone();
    }
    base
}

/// An NRPN of 0.6, its width said.
fn nrpn(msb: u8, lsb: u8) -> Value {
    json!({"nrpn_msb": msb, "nrpn_lsb": lsb, "value_bits": 14})
}

fn nrpn_way(msb: u8, lsb: u8) -> Value {
    json!({"ctrlType": "nrpn", "ctrlIndex": [msb, lsb], "value_bits": 14})
}

fn blocked(way: Value) -> Value {
    with(way, &json!({"nrpn_lsb_range": BLOCK}))
}

/// The 0.7 document of `doc`, checked to read as 0.7, to be written back as converted and to
/// validate against the 0.7 schema.
#[track_caller]
fn converted(doc: &Value) -> Value {
    let out = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{doc}"));
    let value: Value = serde_json::from_str(&out).unwrap();
    let read = osd::read_as(&out, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{out}"));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&read)).unwrap(), value);
    assert_eq!(common::Schemas::load("0.7").check(Kind::Device, &value), None, "{out}");
    value
}

/// The 0.7 entry that the single control `entry` of `version` becomes.
#[track_caller]
fn converted_entry(version: Version, entry: &Value) -> Value {
    converted(&controls(version, vec![entry.clone()]))["cc_params"]["entries"][0].clone()
}

// ---- the NRPN way that receives the block

#[test]
fn the_block_goes_to_the_single_nrpn_way_of_also_beside_a_flat_message_of_another_kind() {
    let flats = [
        (json!({"cc": 1}), json!({"ctrlType": "cc", "ctrlIndex": [1]})),
        (json!({"cc": 1, "cc_lsb": 33}), json!({"ctrlType": "cc", "ctrlIndex": [1, 33], "value_bits": 14})),
        (json!({"rpn_msb": 0, "rpn_lsb": 2, "value_bits": 14}), json!({"ctrlType": "rpn", "ctrlIndex": [0, 2], "value_bits": 14})),
    ];
    for (flat, first) in flats {
        let entry = with(json!({"osc": "/a", "nrpn_lsb_range": BLOCK, "also": [{"cc": 9}, nrpn(3, 0)]}), &flat);
        let want = json!({"osc": "/a", "ways": [first, {"ctrlType": "cc", "ctrlIndex": [9]}, blocked(nrpn_way(3, 0))]});
        assert_eq!(converted_entry(Version::V0_6, &entry), want, "{entry}");
    }
}

#[test]
fn the_block_goes_to_the_single_nrpn_way_of_a_control_whose_messages_are_all_in_also() {
    let entry = json!({"osc": "/a", "nrpn_lsb_range": BLOCK, "also": [nrpn(3, 0)]});
    assert_eq!(converted_entry(Version::V0_6, &entry), json!({"osc": "/a", "ways": [blocked(nrpn_way(3, 0))]}));
    // A `cc_lsb` alone, which has no way: the block goes to the NRPN way of `also`.
    let entry = json!({"osc": "/a", "cc_lsb": 33, "nrpn_lsb_range": BLOCK, "also": [{"cc": 9}, nrpn(3, 0)]});
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({"osc": "/a", "ways": [{"ctrlType": "cc", "ctrlIndex": [9]}, blocked(nrpn_way(3, 0))], "_incomplete": [{"cc_lsb": 33}]})
    );
    // An NRPN of one number, between two other messages, with the direction of the entry: no
    // single NRPN way is left to receive the block.
    let entry = json!({
        "osc": "/a", "nrpn_lsb_range": BLOCK, "transmit": "absolute",
        "also": [{"rpn_msb": 0, "rpn_lsb": 1, "value_bits": 7}, {"nrpn_msb": 3, "value_bits": 7, "range": [0, 9]}, {"cc": 9}]
    });
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({"osc": "/a", "_nrpn_lsb_range": BLOCK, "_incomplete": [{"nrpn_msb": 3, "value_bits": 7, "range": [0, 9]}], "ways": [
            {"ctrlType": "rpn", "ctrlIndex": [0, 1], "value_bits": 7, "transmit": "absolute"},
            {"ctrlType": "cc", "ctrlIndex": [9], "transmit": "absolute"}
        ]})
    );
}

#[test]
fn the_block_beside_a_flat_nrpn_stays_on_the_flat_way_whatever_also_holds() {
    for also in [json!([{"cc": 9}]), json!([nrpn(4, 0)]), json!([nrpn(4, 0), nrpn(5, 0)])] {
        let entry = with(json!({"osc": "/a", "nrpn_lsb_range": BLOCK, "also": also}), &nrpn(3, 0));
        let ways = converted_entry(Version::V0_6, &entry)["ways"].clone();
        assert_eq!(ways[0], blocked(nrpn_way(3, 0)), "{entry}");
        assert!(ways.as_array().unwrap()[1..].iter().all(|w| w.get("nrpn_lsb_range").is_none()), "{ways}");
    }
    for version in ALL {
        let mut entry = json!({"osc": "/a", "nrpn_msb": 3, "nrpn_lsb": 0, "nrpn_lsb_range": BLOCK});
        if version == Version::V0_6 {
            entry["value_bits"] = json!(14);
        }
        assert_eq!(converted_entry(version, &entry), json!({"osc": "/a", "ways": [blocked(nrpn_way(3, 0))]}));
    }
}

// ---- the block no single NRPN way can receive

/// The entry, converted, keeps the block as the note `_nrpn_lsb_range` and converts the rest as the
/// same entry without a block.
#[track_caller]
fn kept_as_a_note(version: Version, entry: Value) {
    let mut bare = entry.clone();
    bare.as_object_mut().unwrap().remove("nrpn_lsb_range");
    let want = with(converted_entry(version, &bare), &json!({"_nrpn_lsb_range": BLOCK}));
    let given = with(entry, &json!({"nrpn_lsb_range": BLOCK}));
    assert_eq!(converted_entry(version, &given), want, "{version:?} {given}");
}

#[test]
fn the_block_of_a_control_with_no_nrpn_is_kept_on_the_entry_as_a_note_from_each_version() {
    for version in ALL {
        for entry in [
            json!({"osc": "/a", "cc": 1}),
            json!({"osc": "/a", "cc": 1, "cc_lsb": 33, "range": [0, 16383]}),
        ] {
            kept_as_a_note(version, entry);
        }
        let rpn = json!({"osc": "/a", "rpn_msb": 0, "rpn_lsb": 2});
        let rpn = if version == Version::V0_6 { with(rpn, &json!({"value_bits": 14})) } else { rpn };
        kept_as_a_note(version, rpn);
    }
}

#[test]
fn the_block_beside_an_also_without_an_nrpn_is_kept_on_the_entry_as_a_note() {
    for entry in [
        json!({"osc": "/a", "also": [{"cc": 1}]}),
        json!({"osc": "/a", "also": [{"cc": 1}, {"rpn_msb": 0, "rpn_lsb": 1, "value_bits": 14}]}),
        json!({"osc": "/a", "cc": 2, "also": [{"cc": 3}]}),
        json!({"osc": "/a", "rpn_msb": 0, "rpn_lsb": 2, "value_bits": 14, "also": [{"cc": 3, "range": [0, 9]}]}),
    ] {
        kept_as_a_note(Version::V0_6, entry);
    }
}

#[test]
fn the_block_beside_several_nrpn_ways_of_also_and_no_flat_nrpn_is_kept_on_the_entry_as_a_note() {
    for entry in [
        json!({"osc": "/a", "also": [nrpn(3, 0), nrpn(4, 0)]}),
        json!({"osc": "/a", "cc": 1, "also": [nrpn(3, 0), {"cc": 9}, nrpn(4, 0)]}),
        json!({"osc": "/a", "also": [nrpn(3, 0), nrpn(4, 0), nrpn(5, 0)]}),
    ] {
        kept_as_a_note(Version::V0_6, entry);
    }
}

#[test]
fn the_note_of_a_block_takes_the_next_free_key_and_leaves_the_taken_ones_as_they_are() {
    for version in ALL {
        let entry = json!({"osc": "/a", "cc": 1, "nrpn_lsb_range": BLOCK, "_nrpn_lsb_range": "mine"});
        assert_eq!(
            converted_entry(version, &entry),
            json!({"osc": "/a", "_nrpn_lsb_range": "mine", "_nrpn_lsb_range_2": BLOCK, "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}),
            "{version:?}"
        );
        let entry = json!({
            "osc": "/a", "cc": 1, "nrpn_lsb_range": BLOCK,
            "_nrpn_lsb_range": [1, 2], "_nrpn_lsb_range_2": null, "_nrpn_lsb_range_4": 4
        });
        assert_eq!(
            converted_entry(version, &entry),
            json!({
                "osc": "/a", "_nrpn_lsb_range": [1, 2], "_nrpn_lsb_range_2": null, "_nrpn_lsb_range_3": BLOCK, "_nrpn_lsb_range_4": 4,
                "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]
            }),
            "{version:?}"
        );
    }
}

#[test]
fn each_control_of_a_file_keeps_its_own_block() {
    for version in ALL {
        let doc = controls(version, vec![
            json!({"osc": "/a", "cc": 1, "nrpn_lsb_range": [0, 1]}),
            json!({"osc": "/b", "cc": 2}),
            json!({"osc": "/c", "cc": 3, "nrpn_lsb_range": [2, 3]}),
        ]);
        assert_eq!(
            converted(&doc)["cc_params"]["entries"],
            json!([
                {"osc": "/a", "_nrpn_lsb_range": [0, 1], "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]},
                {"osc": "/b", "ways": [{"ctrlType": "cc", "ctrlIndex": [2]}]},
                {"osc": "/c", "_nrpn_lsb_range": [2, 3], "ways": [{"ctrlType": "cc", "ctrlIndex": [3]}]}
            ])
        );
    }
}

// ---- the range no way takes

fn cc_way(n: u8) -> Value {
    json!({"ctrlType": "cc", "ctrlIndex": [n]})
}

/// 0.6 entries whose `range` stands beside no flat message and a `also` that is not empty, with
/// none of `default`, `values` and `step_count`, each with the 0.7 entry it becomes.
fn range_beside_also_alone() -> Vec<(Value, Value)> {
    vec![
        (
            json!({"osc": "/x", "range": [0, 127], "also": [{"cc": 74}]}),
            json!({"osc": "/x", "ways": [cc_way(74)], "_range": [0, 127]}),
        ),
        (
            json!({"osc": "/x", "range": [0, 500], "also": [{"cc": 74}]}),
            json!({"osc": "/x", "ways": [cc_way(74)], "_range": [0, 500]}),
        ),
        // A way has a `range` only when its element of `also` gave one, the same as the entry's too.
        (
            json!({"osc": "/x", "range": [0, 9], "also": [{"cc": 2, "range": [0, 9]}, {"cc": 3}]}),
            json!({"osc": "/x", "ways": [with(cc_way(2), &json!({"range": [0, 9]})), cc_way(3)], "_range": [0, 9]}),
        ),
        // Several kinds, each way with what its element gave.
        (
            json!({"osc": "/x", "range": [0, 127], "also": [
                {"cc": 2, "range": [5, 60]}, {"cc": 3, "cc_lsb": 35}, nrpn(3, 0),
                {"rpn_msb": 0, "rpn_lsb": 1, "value_bits": 7, "range": [0, 12]}
            ]}),
            json!({"osc": "/x", "ways": [
                with(cc_way(2), &json!({"range": [5, 60]})),
                {"ctrlType": "cc", "ctrlIndex": [3, 35], "value_bits": 14},
                nrpn_way(3, 0),
                {"ctrlType": "rpn", "ctrlIndex": [0, 1], "value_bits": 7, "range": [0, 12]}
            ], "_range": [0, 127]}),
        ),
        // The block goes to the single NRPN way, the direction of the entry to every way.
        (
            json!({"osc": "/x", "range": [0, 127], "transmit": "absolute", "nrpn_lsb_range": [0, 7], "also": [nrpn(3, 0)]}),
            json!({"osc": "/x", "ways": [with(blocked(nrpn_way(3, 0)), &json!({"transmit": "absolute"}))], "_range": [0, 127]}),
        ),
        (
            json!({"osc": "/x", "range": [0, 127], "transmit": "absolute", "recognize": "relative", "also": [{"cc": 2}, nrpn(3, 0)]}),
            json!({"osc": "/x", "ways": [
                with(cc_way(2), &json!({"transmit": "absolute", "recognize": "relative"})),
                with(nrpn_way(3, 0), &json!({"transmit": "absolute", "recognize": "relative"}))
            ], "_range": [0, 127]}),
        ),
        // No single NRPN way: both notes.
        (
            json!({"osc": "/x", "range": [0, 127], "nrpn_lsb_range": BLOCK, "also": [{"cc": 2}]}),
            json!({"osc": "/x", "ways": [cc_way(2)], "_range": [0, 127], "_nrpn_lsb_range": BLOCK}),
        ),
        (
            json!({"osc": "/x", "range": [0, 127], "nrpn_lsb_range": BLOCK, "also": [nrpn(3, 0), nrpn(4, 0)]}),
            json!({"osc": "/x", "ways": [nrpn_way(3, 0), nrpn_way(4, 0)], "_range": [0, 127], "_nrpn_lsb_range": BLOCK}),
        ),
    ]
}

#[test]
fn a_range_beside_an_also_and_no_flat_message_is_kept_on_the_entry_as_the_note_range() {
    for (entry, want) in range_beside_also_alone() {
        assert_eq!(converted_entry(Version::V0_6, &entry), want, "{entry}");
    }
}

#[test]
fn each_control_of_a_file_keeps_its_own_range_note_beside_the_other_controls() {
    let (entries, mut want): (Vec<Value>, Vec<Value>) = range_beside_also_alone()
        .into_iter()
        .take(2)
        .enumerate()
        .map(|(n, (mut entry, mut want))| {
            // Messages of their own: one file claims each message once.
            for (value, key) in [(&mut entry, "also"), (&mut want, "ways")] {
                value["osc"] = json!(format!("/x{n}"));
                match key {
                    "also" => value[key][0]["cc"] = json!(20 + n),
                    _ => value[key][0]["ctrlIndex"] = json!([20 + n]),
                }
            }
            (entry, want)
        })
        .unzip();
    let mut all = vec![json!({"osc": "/first", "cc": 1})];
    all.extend(entries);
    want.insert(0, json!({"osc": "/first", "ways": [cc_way(1)]}));
    assert_eq!(converted(&controls(Version::V0_6, all))["cc_params"]["entries"], Value::from(want));
}

#[test]
fn the_note_of_a_range_takes_the_next_free_key_and_leaves_the_taken_ones_as_they_are() {
    for (taken, free) in [
        (json!({"_range": "mine"}), "_range_2"),
        (json!({"_range": null}), "_range_2"),
        (json!({"_range": [0, 127]}), "_range_2"),
        (json!({"_range": [1, 2], "_range_2": null, "_range_4": 4}), "_range_3"),
        (json!({"_range": 1, "_range_2": 2, "_range_3": 3}), "_range_4"),
        // A later suffix alone leaves `_range` free.
        (json!({"_range_2": "mine"}), "_range"),
    ] {
        let entry = with(json!({"osc": "/a", "range": [0, 99], "also": [{"cc": 74}]}), &taken);
        let mut want = with(json!({"osc": "/a", "ways": [cc_way(74)]}), &taken);
        want[free] = json!([0, 99]);
        assert_eq!(converted_entry(Version::V0_6, &entry), want, "{entry}");
    }
}

#[test]
fn the_notes_of_a_range_and_of_a_block_each_take_their_own_free_key() {
    let entry = json!({
        "osc": "/a", "range": [0, 99], "nrpn_lsb_range": BLOCK, "_range": "r", "_nrpn_lsb_range": "b",
        "_nrpn_lsb_range_2": "c", "also": [{"cc": 74}]
    });
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({
            "osc": "/a", "ways": [cc_way(74)], "_range": "r", "_range_2": [0, 99], "_nrpn_lsb_range": "b",
            "_nrpn_lsb_range_2": "c", "_nrpn_lsb_range_3": BLOCK
        })
    );
}

/// The reference scale a reader of 0.7 gives the single control that `entry` of 0.6 becomes.
#[track_caller]
fn reference_scale_of(entry: &Value) -> [i64; 2] {
    let doc = converted(&controls(Version::V0_6, vec![entry.clone()]));
    match osd::read_as(&doc.to_string(), Kind::Device, Version::V0_7) {
        Ok(Descriptor::Device(d)) => d.cc_params.unwrap().entries[0].reference_scale(),
        other => panic!("not a device: {other:?}"),
    }
}

/// `default`, `values` and `step_count` beside a `range` that no way takes: the fields of the entry,
/// its `range`, its `also`, and the `scale` 0.7 writes, when it writes one.
fn measured_beside_a_range() -> Vec<(Value, [i64; 2], Value, bool)> {
    let cc = json!([{"cc": 2}]);
    let cc_then_nrpn = json!([{"cc": 2}, nrpn(3, 0)]);
    vec![
        // A `default` or a named value the ways do not reach.
        (json!({"default": 300}), [0, 500], cc.clone(), true),
        (json!({"values": {"Max": 500, "Off": 0}}), [0, 500], cc.clone(), true),
        (json!({"step_count": 5}), [0, 500], cc.clone(), true),
        (json!({"default": 300, "values": {"Max": 500}, "step_count": 5}), [0, 500], json!([{"cc": 2, "range": [0, 9]}, {"cc": 3}]), true),
        (json!({"default": 17000}), [0, 20000], json!([nrpn(3, 0)]), true),
        // The `range`, not the scale of the first way.
        (json!({"default": 300}), [0, 500], cc_then_nrpn.clone(), true),
        (json!({"default": 64}), [0, 127], cc_then_nrpn.clone(), true),
        // The most precise way has that scale, the first one has not: nothing to write.
        (json!({"default": 8000}), [0, 16383], cc_then_nrpn.clone(), false),
        (json!({"values": {"Max": 16383}}), [0, 16383], cc_then_nrpn, false),
        (json!({"default": 64}), [0, 127], cc.clone(), false),
        (json!({"step_count": 3}), [0, 127], cc.clone(), false),
        (json!({"default": 5}), [0, 99], json!([{"cc": 2, "range": [0, 99]}]), false),
        // The same highest value and another lowest is another scale.
        (json!({"default": 3}), [0, 127], json!([{"cc": 2, "range": [1, 127]}]), true),
        // Two ways as precise: the first of them is the most precise.
        (json!({"default": 3}), [0, 127], json!([{"cc": 2, "range": [1, 127]}, {"cc": 3}]), true),
        (json!({"default": 3}), [0, 127], json!([{"cc": 2}, {"cc": 3, "range": [1, 127]}]), false),
        // A range below zero.
        (json!({"default": -10}), [-64, 63], cc, true),
    ]
}

#[test]
fn the_range_that_no_way_takes_is_the_reference_scale_of_a_default_values_or_a_step_count() {
    for (fields, range, also, scaled) in measured_beside_a_range() {
        let entry = with(json!({"osc": "/x", "range": range, "also": also}), &fields);
        let out = converted_entry(Version::V0_6, &entry);
        assert_eq!(out.get("scale"), scaled.then(|| json!(range)).as_ref(), "{entry} into {out}");
        assert_eq!(out["_range"], json!(range), "{entry} into {out}");
        assert_eq!(out.get("range"), None, "{out}");
        assert_eq!(out.get("default"), fields.get("default"), "{out}");
        assert_eq!(out.get("step_count"), fields.get("step_count"), "{out}");
        let named = fields.get("values").map(|v| {
            v.as_object().unwrap().iter().map(|(t, v)| json!({"value": v, "title": t})).collect::<Value>()
        });
        assert_eq!(out.get("values"), named.as_ref(), "{out}");
        for (way, element) in out["ways"].as_array().unwrap().iter().zip(also.as_array().unwrap()) {
            assert_eq!(way.get("range"), element.get("range"), "{out}");
        }
        assert_eq!(reference_scale_of(&entry), range, "{entry}");
    }
}

#[test]
fn a_range_that_no_way_takes_with_nothing_written_in_it_leaves_the_reference_scale_to_the_ways() {
    for (also, scale) in [
        (json!([{"cc": 74}]), [0, 127]),
        (json!([{"cc": 74}, nrpn(3, 0)]), [0, 16383]),
        (json!([{"cc": 74, "range": [0, 99]}]), [0, 99]),
    ] {
        let entry = json!({"osc": "/a", "range": [0, 500], "also": also});
        assert_eq!(converted_entry(Version::V0_6, &entry).get("scale"), None, "{entry}");
        assert_eq!(reference_scale_of(&entry), scale, "{entry}");
    }
}

#[test]
fn a_range_beside_a_flat_message_goes_to_the_first_way() {
    let entry = json!({"osc": "/a", "cc": 1, "range": [0, 99], "also": [{"cc": 2}]});
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({"osc": "/a", "ways": [{"ctrlType": "cc", "ctrlIndex": [1], "range": [0, 99]}, {"ctrlType": "cc", "ctrlIndex": [2]}]})
    );
}

#[test]
fn a_control_with_no_message_keeps_its_range_and_its_block_on_the_entry() {
    for version in ALL {
        let mut entries = vec![json!({"osc": "/a", "range": [0, 9], "nrpn_lsb_range": BLOCK})];
        if version == Version::V0_6 {
            entries.push(json!({"osc": "/b", "range": [0, 9], "nrpn_lsb_range": BLOCK, "also": []}));
        }
        for entry in entries {
            let osc = entry["osc"].clone();
            assert_eq!(converted_entry(version, &entry), json!({"osc": osc, "range": [0, 9], "nrpn_lsb_range": BLOCK, "ways": []}));
        }
    }
}

// ---- before 0.7

/// Controls that 0.7 refuses or writes elsewhere: the versions before keep them as given.
fn kept_before_0_7() -> Vec<Value> {
    let mut entries = vec![
        json!({"osc": "/block", "cc": 93, "nrpn_lsb_range": BLOCK}),
        json!({"osc": "/also", "nrpn_lsb_range": BLOCK, "also": [nrpn(3, 0), nrpn(4, 0)]}),
    ];
    entries.push(json!({"osc": "/range", "range": [0, 127], "also": [{"cc": 2}]}));
    entries
}

#[test]
fn a_0_6_text_reads_writes_and_converts_to_0_6_with_the_block_and_the_range_as_given() {
    let doc = controls(Version::V0_6, kept_before_0_7());
    let text = doc.to_string();
    let read = osd::read(&text, Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&read)).unwrap(), doc);
    let out = osd::convert_to(&text, Kind::Device, Version::V0_6).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), doc);
}

#[test]
fn a_0_4_or_0_5_block_beside_a_cc_converts_to_0_5_and_0_6_as_given() {
    let entry = json!({"osc": "/block", "cc": 93, "nrpn_lsb_range": BLOCK});
    for (from, to) in [(Version::V0_4, Version::V0_5), (Version::V0_4, Version::V0_6), (Version::V0_5, Version::V0_6)] {
        let text = controls(from, vec![entry.clone()]).to_string();
        let out = osd::convert_to(&text, Kind::Device, to).unwrap_or_else(|f| panic!("{f}\n{text}"));
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap()["cc_params"]["entries"], json!([entry]), "{from:?} to {to:?}");
    }
}

fn catalogue(files: &[(&str, &Value)]) -> BTreeMap<String, String> {
    files.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

/// A variant of the target `device` writes: a `midi_of` names a driver of the same device (23.18).
fn variant(version: Version) -> Value {
    of_version(version, json!({"device": {"name": "T", "osc_prefix": "/v"}, "midi_of": "acme/b"}))
}

fn profile_0_7() -> Value {
    json!({"$schema": "https://w3id.org/osd/0.7/profile.schema.json", "identity": {"ports": [{"operator": "equals", "value": "Synth"}]}})
}

/// The entries of the resolved device of `acme/a`, as written.
fn resolved_entries(files: &BTreeMap<String, String>) -> Value {
    let r = osd::resolve("acme/a", files).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap();
    out["cc_params"]["entries"].clone()
}

#[test]
fn a_resolution_that_stays_in_0_6_keeps_the_block_and_the_range_as_given() {
    let files = catalogue(&[("acme/a", &variant(Version::V0_6)), ("acme/b", &controls(Version::V0_6, kept_before_0_7()))]);
    assert_eq!(resolved_entries(&files), Value::from(kept_before_0_7()));
}

#[test]
fn a_resolution_carried_to_0_7_keeps_the_block_of_its_target_as_a_note() {
    for version in ALL {
        let target = controls(version, vec![json!({"osc": "/a", "cc": 1, "nrpn_lsb_range": BLOCK})]);
        let files = catalogue(&[("acme/a", &variant(Version::V0_7)), ("acme/b", &target)]);
        assert_eq!(
            resolved_entries(&files),
            json!([{"osc": "/a", "_nrpn_lsb_range": BLOCK, "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]),
            "{version:?}"
        );
    }
}

#[test]
fn a_resolution_carried_to_0_7_keeps_the_range_no_way_takes_as_the_converter_writes_it() {
    let target = controls(Version::V0_6, vec![
        json!({"osc": "/x", "range": [0, 127], "also": [{"cc": 2}]}),
        json!({"osc": "/y", "range": [0, 500], "default": 300, "also": [{"cc": 3}]}),
        json!({"osc": "/z", "range": [0, 500], "values": {"Max": 500}, "_range": "mine", "also": [{"cc": 4}]}),
    ]);
    let want = json!([
        {"osc": "/x", "ways": [cc_way(2)], "_range": [0, 127]},
        {"osc": "/y", "default": 300, "scale": [0, 500], "ways": [cc_way(3)], "_range": [0, 500]},
        {"osc": "/z", "values": [{"value": 500, "title": "Max"}], "scale": [0, 500], "ways": [cc_way(4)], "_range": "mine", "_range_2": [0, 500]},
    ]);
    // The target of a file of 0.7.
    let files = catalogue(&[("acme/a", &variant(Version::V0_7)), ("acme/b", &target)]);
    assert_eq!(resolved_entries(&files), want);
    // The device of a profile of 0.7.
    let mut device = target;
    device["identity"] = json!({"profile": "acme"});
    let files = catalogue(&[("acme/a", &device), ("acme/_profile", &profile_0_7())]);
    assert_eq!(resolved_entries(&files), want);
}

// ---- every field of a control, alone and beside each kind of message

const FLAT_KEYS: [&str; 6] = ["cc", "cc_lsb", "nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb"];

/// The message fields of an entry of `cc_params` of `version`: none, each kind on the entry (an
/// NRPN and an RPN by two numbers and by one), and in 0.6 each `also`, alone and beside each flat
/// message; before 0.6, the mixes of kinds.
fn messages(version: Version) -> Vec<Value> {
    let mut flat = vec![
        json!({}),
        json!({"cc": 1}),
        json!({"cc": 1, "cc_lsb": 33}),
        json!({"cc_lsb": 33}),
        json!({"nrpn_msb": 2, "nrpn_lsb": 3}),
        json!({"nrpn_msb": 2}),
        json!({"nrpn_lsb": 3}),
        json!({"rpn_msb": 4, "rpn_lsb": 5}),
        json!({"rpn_msb": 4}),
        json!({"rpn_lsb": 5}),
    ];
    if version < Version::V0_6 {
        flat.extend([
            json!({"cc": 1, "nrpn_msb": 2, "nrpn_lsb": 3}),
            json!({"cc": 1, "cc_lsb": 33, "rpn_msb": 4, "rpn_lsb": 5}),
            json!({"nrpn_msb": 2, "nrpn_lsb": 3, "rpn_msb": 4, "rpn_lsb": 5}),
            json!({"cc": 1, "nrpn_msb": 2}),
            json!({"cc_lsb": 33, "nrpn_msb": 2, "nrpn_lsb": 3}),
        ]);
        return flat;
    }
    for entry in &mut flat {
        if FLAT_KEYS[2..].iter().any(|k| entry.get(*k).is_some()) {
            entry["value_bits"] = json!(7);
        }
    }
    let alsos = [
        json!([]),
        json!([{"cc": 10, "range": [0, 50], "_way": "w"}]),
        json!([{"cc": 11, "cc_lsb": 43}]),
        json!([{"cc_lsb": 44}]),
        json!([{"nrpn_msb": 20, "nrpn_lsb": 21, "value_bits": 14, "range": [0, 1000]}]),
        json!([{"nrpn_msb": 24, "value_bits": 7}]),
        json!([{"rpn_msb": 30, "rpn_lsb": 31, "value_bits": 7}, {"cc": 12}]),
        json!([{"cc": 13}, nrpn(22, 23)]),
        json!([nrpn(20, 21), nrpn(22, 23)]),
    ];
    let mut out = flat.clone();
    for entry in &flat {
        out.extend(alsos.iter().map(|also| with(entry.clone(), &json!({"also": also}))));
    }
    out
}

/// The fields of an entry that `cc_params` and `params` of `version` both admit, beside the message.
fn common_fields(version: Version) -> Vec<(&'static str, Value)> {
    let mut fields = vec![
        ("range", json!([0, 90])),
        ("section", json!("Filter")),
        ("expects", json!("discrete")),
        ("transform", json!("x * 2")),
        ("transform_reverse", json!("x // 2")),
        ("_comment", json!({"a": [1]})),
    ];
    if version >= Version::V0_6 {
        fields.extend([
            ("default", json!(5)),
            ("priority", json!(2)),
            ("step_count", json!(3)),
            ("transmit", json!("absolute")),
            ("recognize", json!("relative")),
            ("role", json!({"parts": [{"part": "filter"}], "setting": "cutoff"})),
        ]);
    }
    fields
}

/// Each message alone, with each field alone, and with all the fields together.
fn entries(messages: Vec<Value>, fields: &[(&str, Value)]) -> Vec<Value> {
    let mut out = Vec::new();
    for message in messages {
        let base = with(json!({"osc": "/a"}), &message);
        let mut all = base.clone();
        out.push(base.clone());
        for (key, value) in fields {
            let mut one = base.clone();
            one[*key] = value.clone();
            all[*key] = value.clone();
            out.push(one);
        }
        out.push(all);
    }
    out
}

fn also_of(entry: &Value) -> Vec<Value> {
    entry.get("also").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn has(way: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|k| way.get(*k).is_some())
}

/// The kinds of message the flat fields of an entry name, in the order of the ways.
fn flat_kinds(entry: &Value) -> Vec<&'static str> {
    [("cc", &FLAT_KEYS[0..2]), ("nrpn", &FLAT_KEYS[2..4]), ("rpn", &FLAT_KEYS[4..6])]
        .into_iter()
        .filter(|(_, keys)| has(entry, keys))
        .map(|(kind, _)| kind)
        .collect()
}

/// Where the conversion of this control of `cc_params` is refused, when it is.
fn refusal(entry: &Value) -> Option<&'static str> {
    let kinds = flat_kinds(entry);
    // 21.3: a block with no NRPN has no way to hold it.
    let bad = kinds.len() > 1 && entry.get("nrpn_lsb_range").is_some() && !kinds.contains(&"nrpn");
    bad.then_some("/cc_params/entries/0")
}

/// Whether the `range` of the entry stands beside no flat message and a non-empty `also`.
fn no_way_takes_the_range(version: Version, entry: &Value) -> bool {
    version >= Version::V0_6 && flat_kinds(entry).is_empty() && !also_of(entry).is_empty() && entry.get("range").is_some()
}

/// The scale of a 0.7 way: its `range`, else the one its width implies.
fn scale_of(way: &Value) -> Value {
    let wide = way["value_bits"] == 14;
    way.get("range").cloned().unwrap_or_else(|| json!([0, if wide { 16383 } else { 127 }]))
}

/// What the 0.7 entry `out`, which has ways, misses of a `range` that no way takes: the note, no
/// `range` the element of `also` of a way did not give, and the `scale` equal to it when something
/// is written in it and the most precise way, the first of the highest, has another.
fn lost_range(entry: &Value, out: &Value, lost: &mut Vec<String>) {
    let range = &entry["range"];
    if out.get("_range") != Some(range) || out.get("range").is_some() {
        lost.push("range kept as _range".into());
    }
    let ways = out["ways"].as_array().cloned().unwrap_or_default();
    let of_ways = also_of(entry).into_iter().filter(|e| complete(flat_kinds(e)[0], e));
    if ways.iter().zip(of_ways).any(|(way, element)| way.get("range") != element.get("range")) {
        lost.push("a way with a range its element of also did not give".into());
    }
    let highest = ways.iter().filter_map(|w| scale_of(w)[1].as_i64()).max();
    let precise = ways.iter().find(|w| scale_of(w)[1].as_i64() == highest).map(scale_of);
    let measured = has(entry, &["default", "values", "step_count"]) && precise.as_ref() != Some(range);
    if out.get("scale") != measured.then_some(range) {
        lost.push(format!("scale (found {:?})", out.get("scale")));
    }
}

/// Whether `from` gives every number a way of its kind says.
fn complete(kind: &str, from: &Value) -> bool {
    match kind {
        "cc" => from.get("cc").is_some(),
        _ => from.get(format!("{kind}_msb")).is_some() && from.get(format!("{kind}_lsb")).is_some(),
    }
}

/// The message numbers, the width, the range and the notes of the source `from` of one way, that
/// the way does not hold.
fn lost_in_way(kind: &str, from: &Value, keys: &[&str], way: &Value, lost: &mut Vec<String>) {
    if way["ctrlType"] != kind {
        lost.push(format!("the {kind} way (found {way})"));
        return;
    }
    let given: Vec<&str> = match kind {
        "cc" => vec!["cc", "cc_lsb"],
        "nrpn" => vec!["nrpn_msb", "nrpn_lsb"],
        _ => vec!["rpn_msb", "rpn_lsb"],
    };
    // A CC given by one number says it alone; an NRPN and an RPN say each number at its place.
    let places: Vec<&str> = given.iter().copied().filter(|k| from.get(*k).is_some()).collect();
    if way["ctrlIndex"].as_array().map(Vec::len) != Some(places.len()) {
        lost.push(format!("the numbers of the {kind} way (found {way})"));
        return;
    }
    for (place, key) in places.iter().enumerate() {
        if from.get(*key).is_some_and(|n| way["ctrlIndex"][place] != *n) {
            lost.push((*key).to_string());
        }
    }
    for key in keys.iter().copied().chain(from.as_object().unwrap().keys().map(String::as_str).filter(|k| k.starts_with('_'))) {
        if from.get(key).is_some_and(|v| way.get(key) != Some(v)) {
            lost.push(format!("{key} of the {kind} way"));
        }
    }
}

/// What the 0.7 entry `out` holds of the fields of the entry itself, message fields apart.
fn lost_on_entry(entry: &Value, out: &Value, elsewhere: &[&str], lost: &mut Vec<String>) {
    for (key, value) in entry.as_object().unwrap() {
        let kept = match key.as_str() {
            k if elsewhere.contains(&k) => true,
            // 0.7 removes it.
            "transform_reverse" => true,
            "orientation" => out["polarity"] == if value == "centered" { "bipolar" } else { "unipolar" },
            "values" => {
                let list: Vec<Value> = value.as_object().unwrap().iter().map(|(t, v)| json!({"value": v, "title": t})).collect();
                out["values"] == Value::from(list)
            }
            k => out.get(k) == Some(value),
        };
        if !kept {
            lost.push(key.clone());
        }
    }
}

/// The values of the control `entry` of `cc_params` that its 0.7 form `out` does not hold in their
/// place.
fn lost_control(version: Version, entry: &Value, out: &Value) -> Vec<String> {
    let mut lost = Vec::new();
    let kinds = flat_kinds(entry);
    let also = also_of(entry);
    let ways = out["ways"].as_array().cloned().unwrap_or_default();
    // The sources of the messages, in order: each kind of the flat fields, then each element of `also`.
    let flat = kinds.iter().filter(|k| complete(k, entry)).count();
    let all_also = also.iter().filter(|e| complete(flat_kinds(e)[0], e)).count();
    if ways.len() != flat + all_also {
        return vec![format!("{} ways for {} complete messages", ways.len(), flat + all_also)];
    }
    let mut elsewhere: Vec<&str> = FLAT_KEYS.to_vec();
    elsewhere.extend(["value_bits", "also", "nrpn_lsb_range"]);
    let partial = ways.is_empty();
    if !partial {
        elsewhere.extend(["range", "transmit", "recognize"]);
    }
    lost_on_entry(entry, out, &elsewhere, &mut lost);
    // A control with no way keeps its `range`; with ways, one that no way takes is a note.
    if !partial && no_way_takes_the_range(version, entry) {
        lost_range(entry, out, &mut lost);
    } else if out.get("_range").is_some() {
        lost.push("a _range the entry does not call for".into());
    }
    let mut written = ways.iter();
    let mut incomplete = Vec::new();
    for kind in &kinds {
        let mut from = entry.clone();
        from.as_object_mut().unwrap().retain(|k, _| !k.starts_with('_'));
        if complete(kind, entry) {
            lost_in_way(kind, &from, &["value_bits", "range"], written.next().unwrap(), &mut lost);
        } else {
            from.as_object_mut().unwrap().retain(|k, _| {
                let number = FLAT_KEYS.contains(&k.as_str()) && k.starts_with(kind);
                number || k == "value_bits" || (!partial && k == "range")
            });
            incomplete.push(from);
        }
    }
    for element in &also {
        let kind = flat_kinds(element)[0];
        if complete(kind, element) {
            lost_in_way(kind, element, &["value_bits", "range"], written.next().unwrap(), &mut lost);
        } else {
            incomplete.push(element.clone());
        }
    }
    let note = (!incomplete.is_empty()).then(|| Value::from(incomplete));
    if out.get("_incomplete") != note.as_ref() {
        lost.push(format!("the note _incomplete, to be {note:?}"));
    }
    for key in ["transmit", "recognize"] {
        if let Some(v) = entry.get(key) {
            if ways.iter().any(|w| w.get(key) != Some(v)) {
                lost.push(format!("{key} on every way"));
            }
        }
    }
    if let Some(block) = entry.get("nrpn_lsb_range") {
        // The way of the flat NRPN, none when it is incomplete; else the single NRPN way of `also`.
        let of_also: Vec<usize> = (flat..ways.len()).filter(|n| ways[*n]["ctrlType"] == "nrpn").collect();
        let receiver = if kinds.contains(&"nrpn") {
            complete("nrpn", entry).then(|| kinds.iter().take_while(|k| **k != "nrpn").filter(|k| complete(k, entry)).count())
        } else {
            (of_also.len() == 1).then(|| of_also[0])
        };
        let on_ways: Vec<usize> = (0..ways.len()).filter(|n| ways[*n].get("nrpn_lsb_range").is_some()).collect();
        let placed = match receiver {
            Some(n) => on_ways == [n] && ways[n]["nrpn_lsb_range"] == *block && !has(out, &["nrpn_lsb_range", "_nrpn_lsb_range"]),
            None if partial => out.get("nrpn_lsb_range") == Some(block),
            None => on_ways.is_empty() && out.get("_nrpn_lsb_range") == Some(block) && out.get("nrpn_lsb_range").is_none(),
        };
        if !placed {
            lost.push("nrpn_lsb_range".into());
        }
    }
    lost
}

/// `entry` as the device of `version` of the sweep, in the table named.
fn swept(version: Version, table: &str, entry: &Value) -> Value {
    let mut body = Map::new();
    let content = match table {
        "cc_params" => json!({"channel": 0, "entries": [entry]}),
        _ => json!({"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [entry]}),
    };
    body.insert(table.into(), content);
    if version != Version::V0_4 && has(entry, &["transform", "transform_reverse"]) {
        body.insert("capabilities".into(), json!(["lua"]));
    }
    if has(entry, &["role"]) {
        body.insert("role_vocabulary".into(), json!("1"));
    }
    of_version(version, Value::Object(body))
}

/// The 0.7 entry of the sweep's device `doc`, checked to read as 0.7, or the fault.
fn swept_entry(doc: &Value, table: &str) -> Result<Value, Fault> {
    let out = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7)?;
    osd::read_as(&out, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{out}"));
    Ok(serde_json::from_str::<Value>(&out).unwrap()[table]["entries"][0].clone())
}

#[test]
fn every_field_of_a_control_reaches_0_7_or_the_conversion_is_refused_at_it() {
    let mut faults = Vec::new();
    let mut checked = 0;
    for version in ALL {
        let mut fields = common_fields(version);
        fields.extend([
            ("nrpn_lsb_range", json!(BLOCK)),
            ("orientation", json!("centered")),
            ("channel", json!(3)),
        ]);
        if version >= Version::V0_6 {
            fields.push(("values", json!({"B": 2, "A": 1})));
        }
        for entry in entries(messages(version), &fields) {
            let doc = swept(version, "cc_params", &entry);
            if let Err(f) = osd::read(&doc.to_string(), Kind::Device) {
                panic!("the sweep wrote a text its version refuses: {f}\n{doc}");
            }
            checked += 1;
            match (swept_entry(&doc, "cc_params"), refusal(&entry)) {
                (Ok(out), None) => {
                    let lost = lost_control(version, &entry, &out);
                    if !lost.is_empty() {
                        faults.push(format!("{version:?} {entry} lost {lost:?}: {out}"));
                    }
                }
                (Err(f), Some(pointer)) if f.kind == FaultKind::NotConvertible && f.pointer == pointer => {}
                (Ok(out), Some(pointer)) => faults.push(format!("{version:?} {entry} not refused at {pointer}: {out}")),
                (Err(f), _) => faults.push(format!("{version:?} {entry}: {f}")),
            }
        }
    }
    assert!(checked > 1000, "{checked} controls swept");
    common::report("controls with a field that vanished or a wrong refusal", faults);
}

#[test]
fn every_field_of_a_params_entry_reaches_0_7_in_its_place() {
    let numbers = [
        json!({}),
        json!({"pr": 1, "p": 2, "c": 3, "r": 4}),
        json!({"id": 7}),
        json!({"pr": 1, "p": 2, "c": 3, "r": 4, "id": 7}),
    ];
    let mut faults = Vec::new();
    for version in ALL {
        let mut fields = common_fields(version);
        fields.push(("values", json!({"B": 2, "A": 1})));
        for entry in entries(numbers.to_vec(), &fields) {
            let doc = swept(version, "params", &entry);
            if let Err(f) = osd::read(&doc.to_string(), Kind::Device) {
                panic!("the sweep wrote a text its version refuses: {f}\n{doc}");
            }
            let out = match swept_entry(&doc, "params") {
                Ok(out) => out,
                Err(f) => {
                    faults.push(format!("{version:?} {entry}: {f}"));
                    continue;
                }
            };
            let mut lost = Vec::new();
            let way = out["ways"].get(0);
            let mut elsewhere = vec!["pr", "p", "c", "r", "id"];
            if let Some(way) = way {
                elsewhere.extend(["range", "transmit", "recognize"]);
                for key in ["range", "transmit", "recognize"] {
                    if entry.get(key).is_some_and(|v| way.get(key) != Some(v)) {
                        lost.push(format!("{key} of the way"));
                    }
                }
                let both = entry.get("pr").is_some() && entry.get("id").is_some();
                for (place, key) in ["pr", "p", "c", "r", "id"].into_iter().enumerate() {
                    let Some(n) = entry.get(key) else { continue };
                    let found = match key {
                        "id" if both => way["_flat"]["id"] == *n,
                        "id" => way["ctrlIndex"] == json!([n]),
                        _ => way["ctrlIndex"][place] == *n,
                    };
                    if !found {
                        lost.push(key.to_string());
                    }
                }
            } else if has(&entry, &["pr", "p", "c", "r", "id"]) {
                lost.push("the way".to_string());
            }
            lost_on_entry(&entry, &out, &elsewhere, &mut lost);
            if !lost.is_empty() {
                faults.push(format!("{version:?} {entry} lost {lost:?}: {out}"));
            }
        }
    }
    common::report("params entries with a field that vanished", faults);
}

// ---- the text of the version

/// The lines of `path` under the heading that starts with `heading`, up to the next heading of
/// its level or above.
fn section(path: &str, heading: &str) -> String {
    let text = std::fs::read_to_string(common::repo().join(path)).unwrap();
    let level = heading.chars().take_while(|c| *c == '#').count();
    let above = |l: &&str| {
        let n = l.chars().take_while(|c| *c == '#').count();
        n > 0 && n <= level
    };
    let lines: Vec<&str> = text.lines().skip_while(|l| !l.starts_with(heading)).skip(1).take_while(|l| !above(l)).collect();
    assert!(!lines.is_empty(), "{path} has no section {heading:?}");
    lines.join("\n")
}

#[track_caller]
fn says(text: &str, wanted: &[&str]) {
    let missing: Vec<&&str> = wanted.iter().filter(|w| !text.contains(**w)).collect();
    assert!(missing.is_empty(), "the text does not say {missing:?}:\n{text}");
}

#[track_caller]
fn no_longer_says(text: &str, gone: &[&str]) {
    let left: Vec<&&str> = gone.iter().filter(|w| text.contains(**w)).collect();
    assert!(left.is_empty(), "the text still says {left:?}:\n{text}");
}

/// The refusal of a `range` no way takes, and its case.
const OLD_REFUSAL: [&str; 2] = ["/cc_params/entries/N/range", "range-beside-also-alone-refused"];

#[test]
fn what_the_converter_writes_says_where_the_block_goes_the_notes_of_the_block_and_of_the_range_and_their_cases() {
    let section = section("spec/0.7.md", "### 5.3.3 ");
    says(&section, &["`_nrpn_lsb_range`", "`_range`", "`_range_2`", "`scale`", "21.3"]);
    for name in [
        "nrpn-block-beside-a-cc-of-0-4-kept-as-note",
        "nrpn-block-beside-a-cc-of-0-5-kept-as-note",
        "nrpn-block-beside-a-cc-of-0-6-kept-as-note",
        "nrpn-block-goes-to-the-only-nrpn-way-of-also",
        "nrpn-block-beside-also-without-nrpn-kept-as-note",
        "nrpn-block-beside-two-nrpn-ways-of-also-kept-as-note",
        "nrpn-block-note-takes-the-next-free-key",
        "range-beside-also-alone-stays-a-note",
        "range-beside-also-alone-gives-the-scale-of-a-default",
    ] {
        says(&section, &[&format!("`0.7/convert/{name}`")]);
    }
    no_longer_says(&section, &OLD_REFUSAL);
}

#[test]
fn the_proposal_of_ways_says_the_note_of_a_range_and_still_needs_no_count() {
    let proposal = "spec/proposals/0009-a-control-reaches-its-messages-by-ways.md";
    let writes = section(proposal, "### What the converter writes");
    says(&writes, &["`_range`", "`scale`"]);
    no_longer_says(&writes, &OLD_REFUSAL);
    let count = section(proposal, "### The round trip with the partners' data");
    says(&count, &["`_range`", "no count is needed"]);
}

#[test]
fn the_interface_says_that_convert_to_keeps_a_range_no_way_takes_as_a_note() {
    let text = std::fs::read_to_string(common::repo().join("osd/docs/INTERFACE.md")).unwrap();
    let row = text.lines().find(|l| l.starts_with("| `convert_to` | `fn convert_to(")).expect("the row of convert_to");
    says(row, &["`_range`"]);
    no_longer_says(row, &["/cc_params/entries/N/range"]);
}
