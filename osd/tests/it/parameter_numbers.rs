//! The numbers that address a parameter in the maker's frame. An entry of `params` of 0.4 to 0.6
//! gives `pr`, `p`, `c`, `r`, each a byte, and `id`, one integer not limited to a byte
//! (`spec/0.6.md`, table of words). In 0.7 they are the `ctrlIndex` of a `sysex` way
//! (`spec/0.7.md` 5.3.3): a list of one number is not bounded to a byte and is 0 or more, a list of
//! two to four numbers holds bytes. Nothing a published version admits is refused or lost.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const OLDER: [(&str, Version); 3] = [("0.4", Version::V0_4), ("0.5", Version::V0_5), ("0.6", Version::V0_6)];

/// A device of `version` whose `params` table holds `entries`.
fn doc(version: &str, entries: Vec<Value>) -> Value {
    let mut d = json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": entries}
    });
    // A text of 0.4 declares no schema.
    if version != "0.4" {
        d["$schema"] = format!("https://w3id.org/osd/{version}/device.schema.json").into();
    }
    d
}

fn read(d: &Value) -> Descriptor {
    osd::read(&d.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}\n{d}"))
}

/// The numbers of the first way of each entry of `params`, none for an entry without a way.
fn numbers(d: &Value) -> Vec<Option<Vec<i64>>> {
    let Descriptor::Device(device) = read(d) else { panic!("a device") };
    device.params.unwrap().entries.iter().map(|e| e.ways().first().and_then(|w| w.ctrl_index.clone())).collect()
}

fn written(d: &Value) -> Value {
    serde_json::from_str(&osd::write(&read(d))).unwrap()
}

/// The text of `d` converted to `to`, which reads as `to` and converts to itself.
fn converted(d: &Value, to: Version) -> Value {
    let out = osd::convert_to(&d.to_string(), Kind::Device, to).unwrap_or_else(|f| panic!("{f}\n{d}"));
    osd::read_as(&out, Kind::Device, to).unwrap_or_else(|f| panic!("{f} in {out}"));
    assert_eq!(osd::convert_to(&out, Kind::Device, to).as_deref(), Ok(out.as_str()), "converting the result changes it");
    serde_json::from_str(&out).unwrap()
}

/// The entries of `params` of `d` converted to 0.7.
fn to_0_7(d: &Value) -> Value {
    let out = converted(d, Version::V0_7);
    assert_eq!(common::Schemas::load("0.7").check(Kind::Device, &out), None, "{out}");
    out["params"]["entries"].clone()
}

fn refused(d: &Value, pointer: &str) {
    let f = osd::read(&d.to_string(), Kind::Device).expect_err(&format!("refused: {d}"));
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{f}");
}

fn sysex(index: Value) -> Value {
    json!({"ctrlType": "sysex", "ctrlIndex": index})
}

// ---- the sysex way of 0.7

#[test]
fn a_sysex_way_of_one_number_is_not_bounded_to_a_byte() {
    for n in [0_i64, 255, 256, 300, 70000, 4_294_967_296] {
        let d = doc("0.7", vec![json!({"osc": "/a", "ways": [sysex(json!([n]))]})]);
        assert_eq!(numbers(&d), vec![Some(vec![n])]);
        assert_eq!(written(&d), d);
    }
}

#[test]
fn a_sysex_way_of_one_negative_number_is_refused_at_its_ctrlindex() {
    for n in [-1_i64, -300] {
        let d = doc(
            "0.7",
            vec![
                json!({"osc": "/a", "ways": [sysex(json!([1]))]}),
                json!({"osc": "/b", "ways": [sysex(json!([2])), sysex(json!([n]))]}),
            ],
        );
        refused(&d, "/params/entries/1/ways/1/ctrlIndex");
    }
}

#[test]
fn a_sysex_way_of_two_to_four_numbers_holds_bytes() {
    for index in [json!([256, 0]), json!([0, 256]), json!([0, -1]), json!([-1, 0]), json!([300, 0, 0]), json!([0, 0, 0, 256])] {
        refused(&doc("0.7", vec![json!({"osc": "/a", "ways": [sysex(index)]})]), "/params/entries/0/ways/0/ctrlIndex");
    }
    for index in [json!([255, 0]), json!([0, 0, 0]), json!([255, 255, 255, 255])] {
        let d = doc("0.7", vec![json!({"osc": "/a", "ways": [sysex(index)]})]);
        assert_eq!(written(&d), d);
    }
}

#[test]
fn a_sysex_way_of_no_number_or_of_five_is_refused() {
    for index in [json!([]), json!([1, 2, 3, 4, 5]), json!([300, 1, 2, 3, 4])] {
        refused(&doc("0.7", vec![json!({"osc": "/a", "ways": [sysex(index)]})]), "/params/entries/0/ways/0/ctrlIndex");
    }
}

#[test]
fn the_0_7_schema_bounds_the_numbers_of_a_sysex_way_by_the_length_of_its_list() {
    let schemas = common::Schemas::load("0.7");
    let check = |index: &Value| schemas.check(Kind::Device, &doc("0.7", vec![json!({"osc": "/a", "ways": [sysex(index.clone())]})]));
    for index in [json!([0]), json!([256]), json!([300]), json!([70000]), json!([255, 0]), json!([0, 0, 0]), json!([255, 255, 255, 255])] {
        assert_eq!(check(&index), None, "the schema refuses {index}");
    }
    for index in [json!([-1]), json!([256, 0]), json!([0, -1]), json!([0, 256, 0]), json!([0, 0, 0, 256]), json!([]), json!([1, 2, 3, 4, 5])] {
        assert!(check(&index).is_some(), "the schema accepts {index}");
    }
}

// ---- the flat numbers of 0.4 to 0.6

#[test]
fn the_bytes_pr_p_c_r_of_an_older_text_are_bounded_where_they_are_read() {
    for (version, _) in OLDER {
        for key in ["pr", "p", "c", "r"] {
            for n in [256, 300, -1] {
                let mut entry = json!({"osc": "/a"});
                entry[key] = n.into();
                refused(&doc(version, vec![entry.clone()]), &format!("/params/entries/0/{key}"));
                entry["id"] = 300.into();
                refused(&doc(version, vec![entry]), &format!("/params/entries/0/{key}"));
            }
        }
    }
}

#[test]
fn an_id_beyond_a_byte_of_an_older_text_is_read_and_written_back_unchanged() {
    for (version, _) in OLDER {
        for n in [256_i64, 300, 70000] {
            let d = doc(version, vec![json!({"osc": "/a", "id": n}), json!({"osc": "/b", "id": 5})]);
            assert_eq!(numbers(&d), vec![Some(vec![n]), Some(vec![5])], "{version}");
            assert_eq!(written(&d), d, "{version}");
        }
    }
}

#[test]
fn an_id_beyond_a_byte_converts_to_the_one_number_of_a_sysex_way_without_a_note() {
    for (version, _) in OLDER {
        let d = doc(
            version,
            vec![
                json!({"osc": "/a", "id": 300}),
                json!({"osc": "/b", "id": 70000, "range": [0, 100]}),
            ],
        );
        assert_eq!(
            to_0_7(&d),
            json!([
                {"osc": "/a", "ways": [sysex(json!([300]))]},
                {"osc": "/b", "ways": [{"ctrlType": "sysex", "ctrlIndex": [70000], "range": [0, 100]}]}
            ]),
            "{version}"
        );
    }
}

#[test]
fn a_negative_id_alone_of_an_older_text_is_read_and_written_back_unchanged() {
    for (version, _) in OLDER {
        for n in [-1, -70000] {
            let d = doc(version, vec![json!({"osc": "/a", "id": n}), json!({"osc": "/b", "id": n - 1, "range": [0, 100]})]);
            assert_eq!(numbers(&d), vec![None, None], "{version}: no message of 0.7 holds a negative number");
            assert_eq!(written(&d), d, "{version}");
        }
    }
}

#[test]
fn a_negative_id_alone_is_kept_in_the_flat_note_of_its_entry_by_the_conversion_to_0_7() {
    for (version, _) in OLDER {
        let d = doc(version, vec![json!({"osc": "/a", "id": -1}), json!({"osc": "/b", "id": -70000, "range": [0, 100]})]);
        assert_eq!(
            to_0_7(&d),
            json!([
                {"osc": "/a", "ways": [], "_flat": {"id": -1}},
                {"osc": "/b", "ways": [], "range": [0, 100], "_flat": {"id": -70000}}
            ]),
            "{version}"
        );
    }
}

#[test]
fn a_note_of_the_entry_and_a_negative_id_alone_are_both_kept() {
    let d = doc("0.6", vec![json!({"osc": "/a", "id": -1, "_flat": "mine"})]);
    assert_eq!(written(&d), d);
    let entry = to_0_7(&d)[0].clone();
    assert_eq!(entry["_flat"], "mine", "{entry}");
    let kept = entry.as_object().unwrap().iter().any(|(k, v)| k.starts_with('_') && *v == json!({"id": -1}));
    assert!(kept, "the id is lost: {entry}");
}

#[test]
fn a_note_of_an_entry_without_a_number_stays_a_note() {
    let d = doc("0.6", vec![json!({"osc": "/a", "_flat": {"id": 3}})]);
    assert_eq!(written(&d), d);
    assert_eq!(to_0_7(&d), json!([{"osc": "/a", "ways": [], "_flat": {"id": 3}}]));
}

#[test]
fn an_older_text_converts_to_a_newer_older_version_with_every_id_kept() {
    let entries = vec![json!({"osc": "/a", "id": 300}), json!({"osc": "/b", "id": -1}), json!({"osc": "/c", "pr": 16})];
    for (from, to) in [("0.4", Version::V0_5), ("0.4", Version::V0_6), ("0.5", Version::V0_6)] {
        assert_eq!(converted(&doc(from, entries.clone()), to)["params"]["entries"], Value::Array(entries.clone()), "{from}");
    }
}

// ---- pr alone, and an id beside the other numbers

#[test]
fn an_entry_that_gives_pr_alone_is_written_back_as_pr() {
    for (version, _) in OLDER {
        for n in [0, 16, 255] {
            let d = doc(version, vec![json!({"osc": "/a", "pr": n, "range": [0, 100]})]);
            assert_eq!(numbers(&d), vec![Some(vec![n])], "{version}");
            assert_eq!(written(&d), d, "{version}");
        }
    }
}

#[test]
fn an_entry_that_gives_pr_alone_converts_to_a_way_that_names_it_in_its_flat_note() {
    for (version, _) in OLDER {
        let d = doc(version, vec![json!({"osc": "/a", "pr": 16, "range": [0, 100]})]);
        assert_eq!(
            to_0_7(&d),
            json!([{"osc": "/a", "ways": [{"ctrlType": "sysex", "ctrlIndex": [16], "range": [0, 100], "_flat": {"pr": 16}}]}]),
            "{version}"
        );
    }
}

#[test]
fn an_id_alone_and_the_first_bytes_in_order_convert_without_a_note() {
    let d = doc("0.6", vec![json!({"osc": "/a", "id": 16}), json!({"osc": "/b", "pr": 16, "p": 2})]);
    assert_eq!(written(&d), d);
    assert_eq!(to_0_7(&d), json!([{"osc": "/a", "ways": [sysex(json!([16]))]}, {"osc": "/b", "ways": [sysex(json!([16, 2]))]}]));
}

#[test]
fn an_id_beyond_a_byte_or_negative_beside_other_numbers_is_kept_in_the_flat_note_of_the_way() {
    for (version, _) in OLDER {
        for id in [300, -1] {
            for given in [json!({"pr": 16}), json!({"p": 2}), json!({"c": 3}), json!({"r": 4}), json!({"pr": 1, "p": 2, "c": 3, "r": 4})] {
                let mut entry = given.clone();
                entry["osc"] = "/a".into();
                entry["id"] = id.into();
                let d = doc(version, vec![entry]);
                assert_eq!(written(&d), d, "{version}");
                let mut note = given.clone();
                note["id"] = id.into();
                let index: Vec<Value> = ["pr", "p", "c", "r"].iter().filter_map(|k| given.get(*k).cloned()).collect();
                assert_eq!(
                    to_0_7(&d),
                    json!([{"osc": "/a", "ways": [{"ctrlType": "sysex", "ctrlIndex": index, "_flat": note}]}]),
                    "{version}"
                );
            }
        }
    }
}

// ---- resolution

#[test]
fn a_resolution_that_carries_an_older_target_to_0_7_keeps_every_id() {
    let target = doc("0.6", vec![json!({"osc": "/a", "id": 300}), json!({"osc": "/b", "id": -1})]);
    let file = json!({"$schema": "https://w3id.org/osd/0.7/device.schema.json", "device": {"name": "T", "osc_prefix": "/f"}, "midi_of": "acme/base"});
    let files: BTreeMap<String, String> = [("acme/base".to_string(), target.to_string()), ("acme/synth".to_string(), file.to_string())].into();
    let resolved = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(resolved.device)))).unwrap();
    assert_eq!(
        out["params"]["entries"],
        json!([{"osc": "/a", "ways": [sysex(json!([300]))]}, {"osc": "/b", "ways": [], "_flat": {"id": -1}}])
    );
}

#[test]
fn a_resolution_of_older_files_keeps_every_id_as_given() {
    let target = doc("0.6", vec![json!({"osc": "/a", "id": 300})]);
    let own = vec![json!({"osc": "/b", "id": -1}), json!({"osc": "/c", "pr": 16})];
    let file = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json", "device": {"name": "T", "osc_prefix": "/f"},
        "midi_of": "acme/base", "params": {"entries": own}
    });
    let files: BTreeMap<String, String> = [("acme/base".to_string(), target.to_string()), ("acme/synth".to_string(), file.to_string())].into();
    let resolved = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(resolved.device)))).unwrap();
    assert_eq!(out["params"]["entries"], json!([{"osc": "/a", "id": 300}, {"osc": "/b", "id": -1}, {"osc": "/c", "pr": 16}]));
}

// ---- the text of 0.7

#[test]
fn the_text_of_the_ways_names_the_cases_of_the_numbers_of_a_sysex_way() {
    let spec = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    let section: Vec<&str> = spec.lines().skip_while(|l| !l.starts_with("### 5.3.3 ")).skip(1).take_while(|l| !l.starts_with('#')).collect();
    let cited: Vec<&str> = section.iter().filter(|l| l.starts_with("Cases:")).flat_map(|l| l.split('`').skip(1).step_by(2)).collect();
    let missing: Vec<&str> = [
        "0.7/read/sysex-way-of-one-number-beyond-a-byte-accepted",
        "0.7/read/sysex-way-of-one-negative-number-refused",
        "0.7/read/sysex-way-of-several-numbers-beyond-a-byte-refused",
        "0.7/convert/params-id-beyond-a-byte-becomes-the-number-of-the-way",
        "0.7/convert/params-pr-alone-keeps-its-name-in-a-note",
        "0.7/convert/params-negative-id-alone-stays-a-note",
    ]
    .into_iter()
    .filter(|c| !cited.contains(c) || !common::repo().join("conformance").join(c).join("case.json").is_file())
    .collect();
    assert!(missing.is_empty(), "5.3.3 of spec/0.7.md does not cite, or the suite does not hold: {missing:?}");
}
