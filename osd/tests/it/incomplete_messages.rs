//! A message not fully known (an NRPN or an RPN given by one number, a `cc_lsb` without its
//! `cc`) has no way (`spec/0.7.md` 5.3.3): the conversion to 0.7 writes no number the text does
//! not give and refuses nothing for it. It keeps its fields in the note `_incomplete` of the
//! entry, and a reading of an older version gives it no way either.

// @lit spec/0.7.md

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version, WarningKind};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const ALL: [Version; 3] = [Version::V0_4, Version::V0_5, Version::V0_6];

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

/// `fields` as a source of `version` gives them: 0.6 says the width of an NRPN or an RPN.
fn sized(version: Version, mut fields: Value) -> Value {
    let parameter = ["nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb"].iter().any(|k| fields.get(*k).is_some());
    if version == Version::V0_6 && parameter && fields.get("value_bits").is_none() {
        fields["value_bits"] = json!(14);
    }
    fields
}

/// The 0.7 document of `doc`, checked to read as 0.7, to validate against the 0.7 schema and to
/// convert to itself.
#[track_caller]
fn converted(doc: &Value) -> Value {
    let out = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{doc}"));
    let value: Value = serde_json::from_str(&out).unwrap();
    osd::read_as(&out, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{out}"));
    assert_eq!(common::Schemas::load("0.7").check(Kind::Device, &value), None, "{out}");
    assert_eq!(osd::convert_to(&out, Kind::Device, Version::V0_7).as_deref(), Ok(out.as_str()), "converting the result changes it");
    value
}

#[track_caller]
fn converted_entry(version: Version, entry: &Value) -> Value {
    converted(&controls(version, vec![entry.clone()]))["cc_params"]["entries"][0].clone()
}

fn cc(n: u8) -> Value {
    json!({"ctrlType": "cc", "ctrlIndex": [n]})
}

fn nrpn_way(msb: u8, lsb: u8, bits: u8) -> Value {
    json!({"ctrlType": "nrpn", "ctrlIndex": [msb, lsb], "value_bits": bits})
}

/// Each message not fully known, with the numbers it gives.
fn incomplete() -> Vec<Value> {
    let mut out = vec![json!({"cc_lsb": 38}), json!({"cc_lsb": 0})];
    for key in ["nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb"] {
        for number in [0, 1, 127] {
            out.push(json!({ key: number }));
        }
    }
    out
}

fn read(doc: &Value) -> Descriptor {
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}\n{doc}"))
}

fn entries_of(d: &Descriptor) -> &[osd::CcEntry] {
    match d {
        Descriptor::Device(d) => &d.cc_params.as_ref().unwrap().entries,
        _ => panic!("a device"),
    }
}

fn warned(d: &Descriptor) -> Vec<String> {
    osd::warnings(d).into_iter().filter(|w| w.kind == WarningKind::PartialControl).map(|w| w.pointer).collect()
}

// ---- the control that gives only a message not fully known

#[test]
fn a_control_with_only_an_incomplete_message_converts_to_no_way_and_a_note_from_each_version() {
    for version in ALL {
        for message in incomplete() {
            let message = sized(version, message);
            let entry = with(json!({"osc": "/a"}), &message);
            let want = json!({"osc": "/a", "ways": [], "_incomplete": [message]});
            assert_eq!(converted_entry(version, &entry), want, "{} {entry}", version.name());
        }
    }
}

#[test]
fn an_nrpn_of_one_number_of_7_bits_keeps_its_width_in_the_note_and_nowhere_else() {
    for key in ["nrpn_msb", "rpn_lsb"] {
        let entry = json!({"osc": "/a", key: 9, "value_bits": 7});
        assert_eq!(
            converted_entry(Version::V0_6, &entry),
            json!({"osc": "/a", "ways": [], "_incomplete": [{key: 9, "value_bits": 7}]})
        );
    }
}

#[test]
fn a_control_left_with_no_way_keeps_its_range_its_block_and_its_other_fields_on_the_entry() {
    for version in ALL {
        for message in [json!({"nrpn_msb": 1}), json!({"cc_lsb": 38}), json!({"rpn_lsb": 2})] {
            let message = sized(version, message);
            let mut own = json!({"range": [0, 99], "nrpn_lsb_range": [0, 7], "section": "Filter", "channel": 3});
            let mut want = own.clone();
            if version == Version::V0_6 {
                let more = json!({"transmit": "absolute", "recognize": "none", "default": 50, "step_count": 3});
                own = with(own, &more);
                want = with(want, &more);
                own["values"] = json!({"Half": 50});
                want["values"] = json!([{"value": 50, "title": "Half"}]);
            }
            let entry = with(with(json!({"osc": "/a"}), &own), &message);
            let want = with(json!({"osc": "/a", "ways": [], "_incomplete": [message]}), &want);
            assert_eq!(converted_entry(version, &entry), want, "{} {entry}", version.name());
        }
    }
}

#[test]
fn a_control_left_with_no_way_is_warned_as_partial_in_the_converted_text() {
    for version in ALL {
        let doc = controls(version, vec![
            json!({"osc": "/a", "cc": 1}),
            sized(version, json!({"osc": "/b", "nrpn_msb": 1})),
            json!({"osc": "/c", "cc_lsb": 38}),
        ]);
        let d = read(&converted(&doc));
        assert_eq!(warned(&d), ["/cc_params/entries/1", "/cc_params/entries/2"], "{}", version.name());
    }
}

#[test]
fn a_range_beside_an_also_of_incomplete_messages_alone_stays_on_the_entry_with_no_way() {
    for also in [
        json!([{"nrpn_msb": 1, "value_bits": 14}]),
        json!([{"cc_lsb": 38}]),
        json!([{"cc_lsb": 38}, {"rpn_lsb": 1, "value_bits": 7}]),
    ] {
        let entry = json!({"osc": "/x", "range": [0, 127], "default": 64, "also": also});
        assert_eq!(
            converted_entry(Version::V0_6, &entry),
            json!({"osc": "/x", "ways": [], "range": [0, 127], "default": 64, "_incomplete": also}),
            "{entry}"
        );
    }
}

#[test]
fn a_range_beside_an_also_that_keeps_a_way_is_the_note_range_and_the_scale_before_that_of_a_first_incomplete_message() {
    // The first message implies another scale than the `range` of the entry, which is the one of
    // the `default`.
    let entry = json!({
        "osc": "/x", "range": [0, 500], "default": 300,
        "also": [{"cc_lsb": 40, "range": [0, 50]}, {"cc": 5}]
    });
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({
            "osc": "/x", "default": 300, "scale": [0, 500], "_range": [0, 500], "ways": [cc(5)],
            "_incomplete": [{"cc_lsb": 40, "range": [0, 50]}]
        })
    );
    // The most precise way has the scale of the `range`: no `scale`.
    let entry = json!({
        "osc": "/x", "range": [0, 127], "default": 64,
        "also": [{"nrpn_msb": 1, "value_bits": 14}, {"cc": 5}]
    });
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({
            "osc": "/x", "default": 64, "_range": [0, 127], "ways": [cc(5)],
            "_incomplete": [{"nrpn_msb": 1, "value_bits": 14}]
        })
    );
}

// ---- the complete messages beside it

#[test]
fn the_complete_messages_stay_ways_in_their_order_with_the_direction_of_the_entry() {
    let entry = json!({
        "osc": "/a", "cc": 5, "transmit": "absolute", "recognize": "both",
        "also": [
            {"nrpn_msb": 1, "value_bits": 14},
            {"nrpn_msb": 2, "nrpn_lsb": 3, "value_bits": 7},
            {"cc_lsb": 40},
            {"cc": 6}
        ]
    });
    let direction = json!({"transmit": "absolute", "recognize": "both"});
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({
            "osc": "/a",
            "ways": [with(cc(5), &direction), with(nrpn_way(2, 3, 7), &direction), with(cc(6), &direction)],
            "_incomplete": [{"nrpn_msb": 1, "value_bits": 14}, {"cc_lsb": 40}]
        })
    );
}

#[test]
fn the_note_lists_the_incomplete_messages_flat_first_then_also_with_their_range_and_notes() {
    let entry = json!({
        "osc": "/a", "rpn_lsb": 9, "value_bits": 7, "range": [0, 100], "_comment": "stays",
        "also": [
            {"cc": 2, "range": [0, 5]},
            {"cc_lsb": 40, "range": [1, 2], "_why": {"a": [1]}, "_incomplete": "of the way"},
            {"nrpn_lsb": 0, "value_bits": 14}
        ]
    });
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({
            "osc": "/a", "_comment": "stays",
            "ways": [with(cc(2), &json!({"range": [0, 5]}))],
            "_incomplete": [
                {"rpn_lsb": 9, "value_bits": 7, "range": [0, 100]},
                {"cc_lsb": 40, "range": [1, 2], "_why": {"a": [1]}, "_incomplete": "of the way"},
                {"nrpn_lsb": 0, "value_bits": 14}
            ]
        })
    );
}

#[test]
fn the_flat_range_of_an_incomplete_message_goes_into_its_note_only_when_a_way_remains() {
    let alone = json!({"osc": "/a", "cc_lsb": 38, "range": [0, 9], "also": [{"nrpn_msb": 1, "value_bits": 14}]});
    assert_eq!(
        converted_entry(Version::V0_6, &json!({"osc": "/a", "cc_lsb": 38, "range": [0, 9], "also": []})),
        json!({"osc": "/a", "ways": [], "range": [0, 9], "_incomplete": [{"cc_lsb": 38}]})
    );
    assert_eq!(
        converted_entry(Version::V0_6, &alone),
        json!({"osc": "/a", "ways": [], "range": [0, 9], "_incomplete": [{"cc_lsb": 38}, {"nrpn_msb": 1, "value_bits": 14}]})
    );
    // The range of a flat message is not the note `_range`, that of a range beside no flat message.
    let kept = json!({"osc": "/a", "cc_lsb": 38, "range": [0, 9], "also": [{"cc": 3}]});
    assert_eq!(
        converted_entry(Version::V0_6, &kept),
        json!({"osc": "/a", "ways": [cc(3)], "_incomplete": [{"cc_lsb": 38, "range": [0, 9]}]})
    );
}

#[test]
fn the_note_of_an_incomplete_message_takes_the_first_free_key_and_leaves_the_taken_ones_as_they_are() {
    for version in ALL {
        let entry = json!({"osc": "/a", "cc_lsb": 38, "_incomplete": "mine"});
        assert_eq!(
            converted_entry(version, &entry),
            json!({"osc": "/a", "ways": [], "_incomplete": "mine", "_incomplete_2": [{"cc_lsb": 38}]}),
            "{}", version.name()
        );
        let entry = json!({"osc": "/a", "cc_lsb": 38, "_incomplete": [1], "_incomplete_2": null, "_incomplete_4": 4});
        assert_eq!(
            converted_entry(version, &entry),
            json!({
                "osc": "/a", "ways": [], "_incomplete": [1], "_incomplete_2": null,
                "_incomplete_3": [{"cc_lsb": 38}], "_incomplete_4": 4
            }),
            "{}", version.name()
        );
    }
}

#[test]
fn each_control_of_a_file_keeps_its_own_incomplete_messages() {
    for version in ALL {
        let doc = controls(version, vec![
            json!({"osc": "/a", "cc_lsb": 33}),
            json!({"osc": "/b", "cc": 2}),
            json!({"osc": "/c", "cc_lsb": 34}),
        ]);
        assert_eq!(
            converted(&doc)["cc_params"]["entries"],
            json!([
                {"osc": "/a", "ways": [], "_incomplete": [{"cc_lsb": 33}]},
                {"osc": "/b", "ways": [cc(2)]},
                {"osc": "/c", "ways": [], "_incomplete": [{"cc_lsb": 34}]}
            ])
        );
    }
}

// ---- the scale the values are written in

/// The entry of 0.6 `flat` + `also` with a `default`, converted.
#[track_caller]
fn scaled(entry: Value) -> Value {
    converted_entry(Version::V0_6, &entry)
}

#[test]
fn the_scale_of_a_first_incomplete_message_is_written_when_the_ways_left_give_another() {
    // The width of the first message, above that of the way left.
    let out = scaled(json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14, "default": 8000, "also": [{"cc": 5}]}));
    assert_eq!((&out["scale"], &out["default"]), (&json!([0, 16383]), &json!(8000)), "{out}");
    // Below that of the way left.
    let out = scaled(json!({
        "osc": "/a", "nrpn_msb": 1, "value_bits": 7, "default": 100,
        "also": [{"nrpn_msb": 2, "nrpn_lsb": 3, "value_bits": 14}]
    }));
    assert_eq!((&out["scale"], &out["default"]), (&json!([0, 127]), &json!(100)), "{out}");
    // Its range, before its width.
    let out = scaled(json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14, "range": [0, 1000], "default": 900, "also": [{"cc": 5}]}));
    assert_eq!((&out["scale"], &out["default"]), (&json!([0, 1000]), &json!(900)), "{out}");
    // The first message of a control that gives them all in `also`.
    let out = scaled(json!({"osc": "/a", "default": 40, "also": [{"cc_lsb": 40, "range": [0, 50]}, {"cc": 5}]}));
    assert_eq!((&out["scale"], &out["default"]), (&json!([0, 50]), &json!(40)), "{out}");
    // A `cc_lsb` alone implies 7 bits.
    let out = scaled(json!({"osc": "/a", "cc_lsb": 40, "default": 100, "also": [{"cc": 1, "cc_lsb": 33}]}));
    assert_eq!((&out["scale"], &out["default"]), (&json!([0, 127]), &json!(100)), "{out}");
}

#[test]
fn named_values_and_a_step_count_take_the_scale_of_a_first_incomplete_message_unchanged() {
    let out = scaled(json!({"osc": "/a", "rpn_msb": 0, "value_bits": 14, "values": {"Top": 16000}, "also": [{"cc": 5}]}));
    assert_eq!(out["scale"], json!([0, 16383]), "{out}");
    assert_eq!(out["values"], json!([{"value": 16000, "title": "Top"}]), "{out}");
    let out = scaled(json!({"osc": "/a", "rpn_msb": 0, "value_bits": 14, "step_count": 4, "also": [{"cc": 5}]}));
    assert_eq!((&out["scale"], &out["step_count"]), (&json!([0, 16383]), &json!(4)), "{out}");
}

#[test]
fn no_scale_is_written_for_a_first_incomplete_message_that_the_ways_left_agree_with_or_that_measures_nothing() {
    for entry in [
        json!({"osc": "/a", "cc_lsb": 40, "default": 100, "also": [{"cc": 5}]}),
        json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 7, "range": [0, 9], "default": 5, "also": [{"cc": 5, "range": [0, 9]}]}),
        json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14, "default": 5, "also": [{"cc": 1, "cc_lsb": 33}]}),
        // Nothing is written in a scale.
        json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14, "also": [{"cc": 5}]}),
        // No way is left.
        json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14, "default": 8000}),
    ] {
        let out = scaled(entry.clone());
        assert!(out.get("scale").is_none() && out["_incomplete"].is_array(), "{entry} became {out}");
    }
}

// ---- the block of an NRPN

#[test]
fn the_block_beside_a_flat_incomplete_nrpn_goes_to_no_way_and_becomes_a_note() {
    for also in [json!([{"cc": 9}]), json!([{"nrpn_msb": 3, "nrpn_lsb": 0, "value_bits": 14}])] {
        let entry = json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14, "nrpn_lsb_range": [0, 7], "also": also});
        let out = converted_entry(Version::V0_6, &entry);
        assert!(out["ways"].as_array().unwrap().iter().all(|w| w.get("nrpn_lsb_range").is_none()), "{out}");
        assert_eq!(out["_nrpn_lsb_range"], json!([0, 7]), "{out}");
        assert!(out.get("nrpn_lsb_range").is_none(), "{out}");
        assert_eq!(out["_incomplete"], json!([{"nrpn_msb": 1, "value_bits": 14}]), "{out}");
    }
}

#[test]
fn the_block_goes_to_the_complete_nrpn_way_of_also_beside_an_incomplete_one() {
    let entry = json!({
        "osc": "/a", "nrpn_lsb_range": [0, 7],
        "also": [{"nrpn_msb": 1, "value_bits": 14}, {"nrpn_msb": 3, "nrpn_lsb": 0, "value_bits": 14}]
    });
    assert_eq!(
        converted_entry(Version::V0_6, &entry),
        json!({
            "osc": "/a",
            "ways": [with(nrpn_way(3, 0, 14), &json!({"nrpn_lsb_range": [0, 7]}))],
            "_incomplete": [{"nrpn_msb": 1, "value_bits": 14}]
        })
    );
}

// ---- the claims

#[test]
fn an_incomplete_message_claims_nothing_beside_the_same_with_a_number_0_from_each_version() {
    for version in ALL {
        for (one, both) in [
            (json!({"nrpn_msb": 1}), json!({"nrpn_msb": 1, "nrpn_lsb": 0})),
            (json!({"nrpn_lsb": 3}), json!({"nrpn_msb": 0, "nrpn_lsb": 3})),
            (json!({"rpn_msb": 2}), json!({"rpn_msb": 2, "rpn_lsb": 0})),
            (json!({"rpn_lsb": 4}), json!({"rpn_msb": 0, "rpn_lsb": 4})),
        ] {
            for (first, second) in [(&one, &both), (&both, &one)] {
                let entry = |osc: &str, fields: &Value| sized(version, with(json!({"osc": osc}), fields));
                converted(&controls(version, vec![entry("/a", first), entry("/b", second)]));
            }
        }
    }
}

#[test]
fn a_text_that_claims_one_incomplete_message_twice_stays_refused_as_its_version_says() {
    let doc = controls(Version::V0_6, vec![
        json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14}),
        json!({"osc": "/b", "nrpn_msb": 1, "value_bits": 14}),
    ]);
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err("claimed twice");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/cc_params/entries/1"), "{f}");
    assert!(osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).is_err());
    let doc = controls(Version::V0_6, vec![json!({"osc": "/a", "nrpn_msb": 1})]);
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err("no width");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::Missing, "/cc_params/entries/0/value_bits"), "{f}");
}

// ---- a resolution carried to 0.7

fn resolved_entries(files: &[(&str, &Value)]) -> Value {
    let files: BTreeMap<String, String> = files.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect();
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap();
    out["cc_params"]["entries"].clone()
}

#[test]
fn a_resolution_carried_to_0_7_gives_the_entries_the_conversion_gives() {
    for version in ALL {
        let device = controls(version, vec![
            sized(version, json!({"osc": "/a", "nrpn_msb": 1, "range": [0, 9]})),
            json!({"osc": "/b", "cc_lsb": 38}),
            json!({"osc": "/c", "cc": 4}),
        ]);
        let want = converted(&device)["cc_params"]["entries"].clone();
        assert_eq!(want[0]["ways"], json!([]), "{want}");
        let variant = of_version(Version::V0_7, json!({"device": {"name": "T", "osc_prefix": "/v"}, "midi_of": "acme/b"}));
        assert_eq!(resolved_entries(&[("acme/a", &variant), ("acme/b", &device)]), want, "{} target", version.name());
        let mut named = device.clone();
        named["identity"] = json!({"profile": "acme"});
        let profile = json!({"$schema": "https://w3id.org/osd/0.7/profile.schema.json", "identity": {"ports": [{"operator": "equals", "value": "Synth"}]}});
        assert_eq!(resolved_entries(&[("acme/a", &named), ("acme/_profile", &profile)]), want, "{} with a profile", version.name());
    }
}

// ---- the reading of an older version

#[test]
fn a_reading_of_0_4_to_0_6_gives_an_incomplete_message_no_way_and_warns_a_control_with_no_other() {
    for version in ALL {
        for message in incomplete() {
            let entry = with(json!({"osc": "/a", "range": [0, 9], "nrpn_lsb_range": [0, 7]}), &sized(version, message));
            let d = read(&controls(version, vec![entry.clone()]));
            let e = &entries_of(&d)[0];
            assert!(e.ways().is_empty(), "{} {entry}: {:?}", version.name(), e.ways());
            assert_eq!((e.range, e.nrpn_lsb_range), (Some([0, 9]), Some([0, 7])), "{} {entry}", version.name());
            assert_eq!(warned(&d), ["/cc_params/entries/0"], "{} {entry}", version.name());
        }
    }
}

#[test]
fn a_reading_gives_the_ways_of_the_complete_messages_alone() {
    let d = read(&controls(Version::V0_6, vec![json!({
        "osc": "/a", "nrpn_msb": 1, "value_bits": 14,
        "also": [{"cc": 5}, {"cc_lsb": 40}, {"rpn_msb": 0, "rpn_lsb": 1, "value_bits": 7}]
    })]));
    let got: Vec<(String, Option<Vec<i64>>)> = entries_of(&d)[0].ways().iter().map(|w| (w.ctrl_type.clone(), w.ctrl_index.clone())).collect();
    assert_eq!(got, [("cc".to_string(), Some(vec![5])), ("rpn".to_string(), Some(vec![0, 1]))]);
    assert!(warned(&d).is_empty());
    for version in [Version::V0_4, Version::V0_5] {
        let d = read(&controls(version, vec![json!({"osc": "/a", "cc": 93, "nrpn_msb": 1})]));
        let got: Vec<Option<Vec<i64>>> = entries_of(&d)[0].ways().iter().map(|w| w.ctrl_index.clone()).collect();
        assert_eq!(got, [Some(vec![93])], "{}", version.name());
        assert!(warned(&d).is_empty());
    }
}

#[test]
fn a_text_with_incomplete_messages_is_written_back_in_its_version_with_its_own_note() {
    let mut texts = Vec::new();
    for version in ALL {
        texts.push(controls(version, vec![
            sized(version, json!({"osc": "/a", "nrpn_msb": 1, "range": [0, 9], "_incomplete": "mine"})),
            json!({"osc": "/b", "cc_lsb": 38, "nrpn_lsb_range": [0, 7], "_incomplete_2": [1]}),
        ]));
    }
    for version in [Version::V0_4, Version::V0_5] {
        texts.push(controls(version, vec![json!({"osc": "/a", "cc": 93, "nrpn_msb": 1, "range": [0, 127], "nrpn_lsb_range": [0, 7]})]));
    }
    texts.push(controls(Version::V0_6, vec![
        json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14, "range": [0, 9], "nrpn_lsb_range": [0, 7],
            "also": [{"cc": 5}, {"cc_lsb": 40, "range": [1, 2], "_incomplete": "of the way"}]}),
        json!({"osc": "/b", "also": [{"rpn_lsb": 2, "value_bits": 7}, {"cc": 6}, {"nrpn_lsb": 0, "value_bits": 14}]}),
    ]));
    for text in texts {
        let back: Value = serde_json::from_str(&osd::write(&read(&text))).unwrap();
        assert_eq!(back, text);
    }
}

// ---- the text of the version and its cases

#[test]
fn what_the_converter_writes_says_the_note_of_an_incomplete_message_and_cites_its_cases() {
    let spec = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    let section: Vec<&str> = spec.lines().skip_while(|l| !l.starts_with("### 5.3.3 ")).skip(1).take_while(|l| !l.starts_with('#')).collect();
    let section = section.join("\n");
    let mut wanted = vec!["`_incomplete`".to_string()];
    wanted.extend(
        ["nrpn-of-one-number-kept-as-note", "rpn-of-one-number-kept-as-note", "cc-lsb-without-cc-kept-as-note"]
            .map(|name| format!("`0.7/convert/{name}`")),
    );
    let missing: Vec<&String> = wanted.iter().filter(|w| !section.contains(w.as_str())).collect();
    assert!(missing.is_empty(), "5.3.3 of spec/0.7.md does not say {missing:?}");
}