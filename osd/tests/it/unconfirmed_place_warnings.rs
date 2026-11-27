//! A control placed twice in one sound transfer, at places that differ: the two places contradict
//! each other and nothing says which one holds. The reader keeps both, takes neither as sure (the
//! control is no setting of a sound, and a comparison does not list it) and warns
//! `UnconfirmedPlace` at the `control` of each. Two identical places do not contradict each other,
//! and places of different transfers are distinct facts.

use osd::{Comparison, Descriptor, Kind, Version};
use serde_json::{json, Value};

const VERSIONS: [Version; 2] = [Version::V0_6, Version::V0_7];

/// The data of the sound is 05 06 07 08.
const MSG: &str = "f0 42 05 06 07 08 f7";

const CUTOFF: &str = "/cc_params/entries/0";
const ATTACK: &str = "/cc_params/entries/1";
const PAN: &str = "/params/entries/0";

fn bytes(hex: &str) -> Vec<u8> {
    hex.split(' ').map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
}

/// A device of `version`: cc 0 filter cutoff, cc 1 filter envelope attack, param 0 pan, and a cc 2
/// without a role.
fn device(version: Version, transfers: Vec<Value>) -> Value {
    let cc = |osc: &str, n: u8| match version {
        Version::V0_7 => json!({"osc": osc, "ways": [{"ctrlType": "cc", "ctrlIndex": [n]}]}),
        _ => json!({"osc": osc, "cc": n}),
    };
    let with_role = |mut entry: Value, parts: Value, setting: &str| {
        entry["role"] = json!({"parts": parts, "setting": setting});
        entry
    };
    let pan = match version {
        Version::V0_7 => json!({"osc": "/pan", "ways": [{"ctrlType": "sysex", "id": 3}]}),
        _ => json!({"osc": "/pan", "id": 3}),
    };
    let uses_lua = transfers
        .iter()
        .flat_map(|t| t["places"].as_array().into_iter().flatten())
        .any(|p| p.get("script").is_some());
    let mut d = json!({
        "$schema": format!("https://w3id.org/osd/{}/device.schema.json", version.name()),
        "role_vocabulary": "1",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"entries": [
            with_role(cc("/cutoff", 74), json!([{"part": "filter"}]), "cutoff"),
            with_role(cc("/attack", 73), json!([{"part": "filter"}, {"part": "envelope"}]), "attack-time"),
            cc("/mute", 9),
        ]},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7",
            "entries": [with_role(pan, json!([{"part": "output"}]), "pan")]},
    });
    if uses_lua {
        d["capabilities"] = json!(["lua"]);
    }
    d["sound_transfers"] = Value::Array(transfers);
    d
}

fn transfer(id: &str, places: Vec<Value>) -> Value {
    json!({"id": id, "carries": "sound", "match_frame": [240, 66, "any", "any", "any", "any", 247],
        "elements": {"data": {"from": 2, "length": 4}}, "places": places})
}

fn place(control: &str, from: i64, length: i64) -> Value {
    json!({"control": control, "from": from, "length": length})
}

fn script(control: &str, code: &str) -> Value {
    json!({"control": control, "script": code})
}

fn read(doc: &Value) -> Descriptor {
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}\n{doc}"))
}

/// The settings of the sound of `msg` in the transfer `id`, as (control, value).
fn settings(d: &Descriptor, id: &str, msg: &str) -> Vec<(String, u64)> {
    let Descriptor::Device(d) = d else { panic!("a device") };
    osd::sound_settings(d, id, &bytes(msg), 0)
        .unwrap_or_else(|f| panic!("{f}"))
        .into_iter()
        .map(|s| (s.control, s.value))
        .collect()
}

fn rows(list: &[(&str, u64)]) -> Vec<(String, u64)> {
    list.iter().map(|(c, v)| (c.to_string(), *v)).collect()
}

/// The warnings of `d`, as (name of the kind, pointer).
fn warned(d: &Descriptor) -> Vec<(String, String)> {
    osd::warnings(d).into_iter().map(|w| (format!("{:?}", w.kind), w.pointer)).collect()
}

/// An `UnconfirmedPlace` at the `control` of each place `(transfer, place)`.
fn unconfirmed(places: &[(usize, usize)]) -> Vec<(String, String)> {
    places
        .iter()
        .map(|(n, m)| ("UnconfirmedPlace".to_string(), format!("/sound_transfers/{n}/places/{m}/control")))
        .collect()
}

/// The two places of the cutoff differ in each of these ways.
fn contradictions() -> Vec<(&'static str, Value, Value)> {
    vec![
        ("other bytes", place(CUTOFF, 0, 1), place(CUTOFF, 2, 1)),
        ("other bytes, the later first", place(CUTOFF, 2, 1), place(CUTOFF, 0, 1)),
        ("another length", place(CUTOFF, 0, 1), place(CUTOFF, 0, 2)),
        ("bytes then a script", place(CUTOFF, 0, 1), script(CUTOFF, "return 1")),
        ("a script then bytes", script(CUTOFF, "return 1"), place(CUTOFF, 0, 1)),
        ("two scripts", script(CUTOFF, "return 1"), script(CUTOFF, "return 2")),
    ]
}

#[test]
fn a_control_placed_twice_at_places_that_differ_is_no_setting_of_the_sound() {
    for version in VERSIONS {
        for (how, first, second) in contradictions() {
            let doc = device(version, vec![transfer("single", vec![first, place(ATTACK, 1, 1), second, place(PAN, 3, 1)])]);
            let got = settings(&read(&doc), "single", MSG);
            assert_eq!(got, rows(&[(ATTACK, 6), (PAN, 8)]), "{} {how}", version.name());
        }
    }
}

#[test]
fn each_of_the_places_that_differ_is_warned_as_to_be_confirmed_at_its_control() {
    for version in VERSIONS {
        for (how, first, second) in contradictions() {
            let doc = device(version, vec![transfer("single", vec![first, place(ATTACK, 1, 1), second])]);
            assert_eq!(warned(&read(&doc)), unconfirmed(&[(0, 0), (0, 2)]), "{} {how}", version.name());
        }
    }
}

#[test]
fn places_that_differ_are_both_kept_as_written() {
    for version in VERSIONS {
        for (how, first, second) in contradictions() {
            let doc = device(version, vec![transfer("single", vec![first, second])]);
            let out: Value = serde_json::from_str(&osd::write(&read(&doc))).unwrap();
            assert_eq!(out, doc, "{} {how}", version.name());
        }
    }
}

#[test]
fn one_place_that_differs_among_identical_ones_leaves_the_control_out_and_warns_every_place() {
    for version in VERSIONS {
        let places = vec![place(PAN, 3, 1), place(PAN, 3, 1), place(ATTACK, 1, 1), place(PAN, 2, 1)];
        let d = read(&device(version, vec![transfer("single", places)]));
        assert_eq!(settings(&d, "single", MSG), rows(&[(ATTACK, 6)]), "{}", version.name());
        assert_eq!(warned(&d), unconfirmed(&[(0, 0), (0, 1), (0, 3)]), "{}", version.name());
    }
}

#[test]
fn identical_places_give_the_setting_with_no_warning() {
    for version in VERSIONS {
        let places = vec![place(CUTOFF, 0, 2), place(ATTACK, 2, 1), place(CUTOFF, 0, 2), place(CUTOFF, 0, 2)];
        let d = read(&device(version, vec![transfer("single", places)]));
        assert_eq!(settings(&d, "single", MSG), rows(&[(CUTOFF, 0x0506), (ATTACK, 7)]), "{}", version.name());
        assert_eq!(warned(&d), vec![], "{}", version.name());
        let scripts = vec![script(CUTOFF, "return 1"), script(CUTOFF, "return 1")];
        let d = read(&device(version, vec![transfer("single", scripts)]));
        assert_eq!(warned(&d), vec![], "{}", version.name());
    }
}

#[test]
fn places_of_different_transfers_are_distinct_facts_each_given_with_no_warning() {
    for version in VERSIONS {
        let doc = device(version, vec![
            transfer("first", vec![place(CUTOFF, 0, 1)]),
            transfer("second", vec![place(CUTOFF, 2, 1)]),
        ]);
        let d = read(&doc);
        assert_eq!(settings(&d, "first", MSG), rows(&[(CUTOFF, 5)]), "{}", version.name());
        assert_eq!(settings(&d, "second", MSG), rows(&[(CUTOFF, 7)]), "{}", version.name());
        assert_eq!(warned(&d), vec![], "{}", version.name());
    }
}

#[test]
fn a_control_unconfirmed_in_one_transfer_is_still_a_setting_of_another() {
    for version in VERSIONS {
        let doc = device(version, vec![
            transfer("sure", vec![place(CUTOFF, 0, 1)]),
            transfer("unsure", vec![place(CUTOFF, 0, 1), place(CUTOFF, 2, 1)]),
        ]);
        let d = read(&doc);
        assert_eq!(settings(&d, "sure", MSG), rows(&[(CUTOFF, 5)]), "{}", version.name());
        assert_eq!(settings(&d, "unsure", MSG), vec![], "{}", version.name());
        assert_eq!(warned(&d), unconfirmed(&[(1, 0), (1, 1)]), "{}", version.name());
    }
}

#[test]
fn the_unconfirmed_places_are_warned_in_the_order_of_the_transfers_and_of_their_places() {
    for version in VERSIONS {
        let doc = device(version, vec![
            transfer("first", vec![place(PAN, 0, 1), place(CUTOFF, 1, 1), place(PAN, 2, 1), place(CUTOFF, 3, 1)]),
            transfer("second", vec![place(ATTACK, 0, 1), place(ATTACK, 1, 1)]),
        ]);
        let want = unconfirmed(&[(0, 0), (0, 1), (0, 2), (0, 3), (1, 0), (1, 1)]);
        assert_eq!(warned(&read(&doc)), want, "{}", version.name());
    }
}

#[test]
fn a_control_without_a_role_placed_at_places_that_differ_is_warned_too() {
    for version in VERSIONS {
        let mute = "/cc_params/entries/2";
        let doc = device(version, vec![transfer("single", vec![place(mute, 0, 1), place(mute, 1, 1)])]);
        assert_eq!(warned(&read(&doc)), unconfirmed(&[(0, 0), (0, 1)]), "{}", version.name());
    }
}

#[test]
fn a_comparison_does_not_list_a_control_whose_places_differ() {
    for version in VERSIONS {
        let places = vec![place(CUTOFF, 0, 1), place(ATTACK, 1, 1), place(CUTOFF, 2, 1), place(PAN, 3, 1)];
        let d = read(&device(version, vec![transfer("single", places)]));
        let Descriptor::Device(d) = &d else { panic!("a device") };
        let of = |msg: &str| osd::sound_settings(d, "single", &bytes(msg), 0).unwrap();
        let got: Vec<(String, Comparison)> = osd::compare_settings(&of(MSG), &of("f0 42 09 06 0a 09 f7"))
            .into_iter()
            .map(|c| (c.control, c.outcome))
            .collect();
        let want = vec![(ATTACK.to_string(), Comparison::Equal), (PAN.to_string(), Comparison::Differ)];
        assert_eq!(got, want, "{}", version.name());
    }
}

#[test]
fn a_converted_device_gives_the_same_settings_and_warnings_as_before_the_conversion() {
    let places = vec![place(CUTOFF, 0, 1), place(ATTACK, 1, 1), place(CUTOFF, 2, 1), place(PAN, 3, 1), place(PAN, 3, 1)];
    let old = device(Version::V0_6, vec![transfer("single", places)]);
    let new = osd::convert_to(&old.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"));
    let new = osd::read(&new, Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    for d in [read(&old), new] {
        assert_eq!(settings(&d, "single", MSG), rows(&[(ATTACK, 6), (PAN, 8)]));
        assert_eq!(warned(&d), unconfirmed(&[(0, 0), (0, 2)]));
    }
}
