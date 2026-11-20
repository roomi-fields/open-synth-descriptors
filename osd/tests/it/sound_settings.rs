//! A sound read setting by setting (0.6): `sound_transfers[].settings`, a list where each entry
//! points to a control (`control`, the JSON pointer of a `cc_params` or `params` entry) and gives
//! its place in the data of the sound: `{from, length}` whole bytes of the data unpacked, the name
//! left out (`from` 0 is the first byte of that data), the value being the unsigned integer of those
//! bytes, the first byte the highest; `bits {from, length}` or `bits_per_byte: 7` read bits and
//! 7-bit words, which the reader does not read; or a Lua `script` (capability `lua`).
//! `osd::sound_settings` gives the settings of a sound with the role of each control
//! (its `paramPath`) in the order of the controls (`cc_params` then `params`);
//! `osd::compare_settings` sets two lists side by side.

use osd::{Comparison, Descriptor, FaultKind, Kind, Setting};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

const MSG: &str = "f0 42 3c 00 01 51 40 41 42 43 44 05 06 07 08 00 f7";

fn bytes(hex: &str) -> Vec<u8> {
    hex.split(' ').map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
}

fn role(part: &str, setting: &str) -> Value {
    json!({"parts": [{"part": part}], "setting": setting})
}

fn env_role(part: &str) -> Value {
    json!({"parts": [{"part": part}, {"part": "envelope"}], "setting": "attack-time"})
}

/// Controls: cc 0 cutoff, cc 1 filter envelope attack, cc 2 amplifier envelope attack, cc 3 pan;
/// param 0 filter cutoff, param 1 pan.
fn device(transfers: Vec<Value>, capabilities: &[&str]) -> Value {
    let mut d = json!({
        "$schema": SCHEMA,
        "role_vocabulary": "1",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [
            {"osc": "/cutoff", "cc": 74, "role": role("filter", "cutoff")},
            {"osc": "/fenv", "cc": 73, "role": env_role("filter")},
            {"osc": "/aenv", "cc": 72, "role": env_role("amplifier")},
            {"osc": "/pan", "cc": 10, "role": role("output", "pan")}
        ]},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [
            {"osc": "/p0", "id": 3, "role": role("filter", "cutoff")},
            {"osc": "/p1", "id": 4, "role": role("output", "pan")}
        ]}
    });
    if !capabilities.is_empty() {
        d["capabilities"] = json!(capabilities);
    }
    d["sound_transfers"] = Value::Array(transfers);
    d
}

fn any(n: usize) -> Vec<Value> {
    vec![json!("any"); n]
}

/// A sound of 17 bytes: `F0 42 ch 00 01 51 40`, a name of 4 bytes (7..11), a data of 4 bytes
/// (11..15), a checksum (15), `F7`.
fn single(settings: Value) -> Value {
    let mut frame = vec![json!(240), json!(66), json!("any"), json!(0), json!(1), json!(81), json!(64)];
    frame.extend(any(9));
    frame.push(json!(247));
    json!({"id": "single", "sound_id": "program", "carries": "sound", "match_frame": frame,
        "elements": {"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}, "checksum": {"from": 15, "length": 1}},
        "settings": settings})
}

fn place(control: &str, from: i64, length: i64) -> Value {
    json!({"control": control, "from": from, "length": length})
}

fn read(doc: &Value) -> Result<Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn dev(doc: &Value) -> osd::Device {
    match read(doc).unwrap_or_else(|f| panic!("{f}")) {
        Descriptor::Device(d) => *d,
        _ => panic!("a device"),
    }
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

/// The settings of the sound `index` of `msg`, as (control, paramPath, value).
fn settings(doc: &Value, transfer: &str, index: usize, msg: &str) -> Vec<(String, String, u64)> {
    osd::sound_settings(&dev(doc), transfer, &bytes(msg), index)
        .unwrap_or_else(|f| panic!("{f}"))
        .into_iter()
        .map(|s| (s.control, s.param_path, s.value))
        .collect()
}

fn row(control: &str, path: &str, value: u64) -> (String, String, u64) {
    (control.to_string(), path.to_string(), value)
}

fn placed() -> Value {
    // Written in the reverse of the order of the controls.
    single(json!([
        place("/params/entries/1", 3, 1),
        place("/cc_params/entries/1", 2, 1),
        place("/cc_params/entries/0", 0, 2)
    ]))
}

#[test]
fn a_settings_list_is_read_and_written_back_where_it_was() {
    let doc = device(
        vec![single(json!([
            place("/cc_params/entries/0", 0, 2),
            {"control": "/cc_params/entries/1", "from": 2, "length": 1, "bits": {"from": 0, "length": 3}},
            {"control": "/cc_params/entries/2", "from": 0, "length": 2, "bits_per_byte": 7},
            {"control": "/cc_params/entries/3", "script": "return 1"}
        ]))],
        &["lua"],
    );
    let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert_eq!(out, doc);
}

#[test]
fn the_settings_of_a_sound_are_its_controls_found_in_the_data_with_their_role_and_raw_value() {
    let got = settings(&device(vec![placed()], &[]), "single", 0, MSG);
    assert_eq!(
        got,
        vec![
            row("/cc_params/entries/0", "/Filter/Cutoff", 0x0506),
            row("/cc_params/entries/1", "/Filter/Envelope/AttackTime", 7),
            row("/params/entries/1", "/Output/Pan", 8),
        ]
    );
}

#[test]
fn the_settings_follow_the_order_of_the_controls_whatever_the_order_of_the_list() {
    let forward = single(json!([
        place("/cc_params/entries/0", 0, 2),
        place("/cc_params/entries/1", 2, 1),
        place("/params/entries/1", 3, 1)
    ]));
    let a = settings(&device(vec![placed()], &[]), "single", 0, MSG);
    let b = settings(&device(vec![forward], &[]), "single", 0, MSG);
    assert_eq!(a, b);
    let params_first = single(json!([place("/params/entries/0", 1, 1), place("/cc_params/entries/3", 0, 1)]));
    let got = settings(&device(vec![params_first], &[]), "single", 0, MSG);
    assert_eq!(got, vec![row("/cc_params/entries/3", "/Output/Pan", 5), row("/params/entries/0", "/Filter/Cutoff", 6)]);
}

#[test]
fn a_value_is_the_unsigned_integer_of_the_bytes_of_the_place_read_as_they_stand() {
    let doc = device(vec![single(json!([place("/cc_params/entries/0", 0, 4), place("/cc_params/entries/1", 1, 1)]))], &[]);
    let got = settings(&doc, "single", 0, "f0 42 3c 00 01 51 40 41 42 43 44 ff 80 01 02 00 f7");
    assert_eq!(
        got,
        vec![row("/cc_params/entries/0", "/Filter/Cutoff", 0xff80_0102), row("/cc_params/entries/1", "/Filter/Envelope/AttackTime", 0x80)]
    );
}

#[test]
fn the_name_the_header_the_channel_and_the_checksum_are_not_read_as_settings() {
    let doc = device(vec![placed()], &[]);
    let other = "f0 42 3d 00 01 51 40 57 58 59 5a 05 06 07 08 6d f7";
    assert_eq!(settings(&doc, "single", 0, MSG), settings(&doc, "single", 0, other));
}

#[test]
fn a_name_inside_the_data_is_left_out_of_the_data_the_places_count_in() {
    let mut t = placed();
    t["elements"]["data"] = json!({"from": 7, "length": 8});
    t["elements"].as_object_mut().unwrap().remove("checksum");
    let got = settings(&device(vec![t], &[]), "single", 0, MSG);
    assert_eq!(got[0], row("/cc_params/entries/0", "/Filter/Cutoff", 0x0506));
    assert_eq!(got.len(), 3);
}

#[test]
fn the_places_of_a_packed_data_count_in_the_data_unpacked() {
    let mut t = single(json!([place("/cc_params/entries/0", 0, 1), place("/cc_params/entries/1", 2, 2)]));
    t["elements"] = json!({"data": {"from": 7, "length": 8, "packing": "msb_first_7in8"}});
    // 45 05 06 07 08 09 0a 0b unpacks to 85 06 87 08 09 0a 8b.
    let got = settings(&device(vec![t], &[]), "single", 0, "f0 42 3c 00 01 51 40 45 05 06 07 08 09 0a 0b 41 f7");
    assert_eq!(
        got,
        vec![row("/cc_params/entries/0", "/Filter/Cutoff", 0x85), row("/cc_params/entries/1", "/Filter/Envelope/AttackTime", 0x8708)]
    );
}

fn bank(settings: Value) -> Value {
    let mut frame = vec![json!(240), json!(66), json!("any"), json!(0), json!(1), json!(81), json!(76)];
    frame.extend(any(16));
    frame.push(json!(247));
    json!({"id": "bank", "sound_id": "program", "carries": "bank", "count": 2, "stride": 8, "match_frame": frame,
        "elements": {"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}}, "settings": settings})
}

#[test]
fn a_sound_of_a_bank_has_the_settings_of_the_same_data_as_a_single_sound() {
    let list = json!([place("/cc_params/entries/0", 0, 2), place("/params/entries/1", 3, 1)]);
    let doc = device(vec![single(list.clone()), bank(list)], &[]);
    let bank_msg = "f0 42 3c 00 01 51 4c 41 42 43 44 01 02 03 04 45 46 47 48 05 06 07 08 f7";
    let from_bank = settings(&doc, "bank", 1, bank_msg);
    assert_eq!(from_bank, settings(&doc, "single", 0, MSG));
    let first = settings(&doc, "bank", 0, bank_msg);
    assert_eq!(first, vec![row("/cc_params/entries/0", "/Filter/Cutoff", 0x0102), row("/params/entries/1", "/Output/Pan", 4)]);
}

#[test]
fn a_control_whose_value_the_reader_cannot_read_is_absent_never_guessed() {
    let t = single(json!([
        place("/cc_params/entries/0", 0, 2),
        {"control": "/cc_params/entries/1", "from": 2, "length": 1, "bits": {"from": 0, "length": 3}},
        {"control": "/cc_params/entries/2", "from": 0, "length": 2, "bits_per_byte": 7},
        {"control": "/params/entries/0", "script": "return 1"}
    ]));
    // /cc_params/entries/3 and /params/entries/1 have no place declared.
    let got = settings(&device(vec![t], &["lua"]), "single", 0, MSG);
    assert_eq!(got, vec![row("/cc_params/entries/0", "/Filter/Cutoff", 0x0506)]);
}

#[test]
fn a_transfer_without_settings_gives_no_setting() {
    let mut t = placed();
    t.as_object_mut().unwrap().remove("settings");
    assert!(settings(&device(vec![t], &[]), "single", 0, MSG).is_empty());
}

#[test]
fn a_control_without_role_is_absent_from_the_settings() {
    let mut doc = device(vec![placed()], &[]);
    doc["cc_params"]["entries"][1].as_object_mut().unwrap().remove("role");
    let got = settings(&doc, "single", 0, MSG);
    assert_eq!(got, vec![row("/cc_params/entries/0", "/Filter/Cutoff", 0x0506), row("/params/entries/1", "/Output/Pan", 8)]);
}

#[test]
fn settings_are_refused_where_the_sound_cannot_be_found() {
    let doc = device(vec![placed()], &[]);
    let d = dev(&doc);
    let f = osd::sound_settings(&d, "other", &bytes(MSG), 0).expect_err("no such transfer");
    assert_eq!(f.kind, FaultKind::BadValue);
    let f = osd::sound_settings(&d, "single", &bytes("f0 42 3c 00 01 51 41 41 42 43 44 05 06 07 08 00 f7"), 0).expect_err("not recognised");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/sound_transfers/0/match_frame"));
    let f = osd::sound_settings(&d, "single", &bytes(MSG), 1).expect_err("past the count");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/sound_transfers/0"));
}

#[test]
fn settings_are_refused_for_a_transfer_whose_data_is_a_script() {
    let mut t = placed();
    t["elements"]["data"] = json!({"script": "return data"});
    t.as_object_mut().unwrap().remove("settings");
    let doc = device(vec![t], &["lua"]);
    let f = osd::sound_settings(&dev(&doc), "single", &bytes(MSG), 0).expect_err("needs Lua");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/sound_transfers/0"));
}

#[test]
fn a_place_past_the_data_is_refused_where_it_is() {
    let doc = device(vec![single(json!([place("/cc_params/entries/0", 0, 2), place("/cc_params/entries/1", 3, 2)]))], &[]);
    refused(&doc, FaultKind::BadValue, "/sound_transfers/0/settings/1");
}

#[test]
fn a_place_ending_on_the_last_byte_of_the_data_is_read() {
    let doc = device(vec![single(json!([place("/cc_params/entries/1", 3, 1)]))], &[]);
    assert!(read(&doc).is_ok());
}

#[test]
fn a_place_is_judged_against_the_data_unpacked_and_without_its_name() {
    let mut t = single(json!([place("/cc_params/entries/0", 5, 2)]));
    t["elements"] = json!({"data": {"from": 7, "length": 8, "packing": "msb_first_7in8"}});
    // 8 packed bytes are 7 data bytes: 5 + 2 reaches the end.
    assert!(read(&device(vec![t.clone()], &[])).is_ok());
    t["settings"] = json!([place("/cc_params/entries/0", 6, 2)]);
    refused(&device(vec![t], &[]), FaultKind::BadValue, "/sound_transfers/0/settings/0");
    let mut named = single(json!([place("/cc_params/entries/0", 3, 2)]));
    named["elements"] = json!({"data": {"from": 7, "length": 8}, "name": {"from": 7, "length": 4}});
    refused(&device(vec![named], &[]), FaultKind::BadValue, "/sound_transfers/0/settings/0");
}

#[test]
fn a_place_in_the_second_transfer_is_refused_at_that_transfer() {
    let ok = single(json!([place("/cc_params/entries/0", 0, 2)]));
    let mut bad = single(json!([place("/cc_params/entries/0", 0, 9)]));
    bad["id"] = json!("other");
    refused(&device(vec![ok, bad], &[]), FaultKind::BadValue, "/sound_transfers/1/settings/0");
}

#[test]
fn a_place_of_no_width_is_refused_at_its_length() {
    let doc = device(vec![single(json!([place("/cc_params/entries/0", 0, 0)]))], &[]);
    refused(&doc, FaultKind::BadValue, "/sound_transfers/0/settings/0/length");
}

#[test]
fn a_place_and_a_script_together_are_refused_at_the_setting() {
    let both = json!({"control": "/cc_params/entries/0", "from": 0, "length": 2, "script": "return 1"});
    refused(&device(vec![single(json!([both]))], &["lua"]), FaultKind::BadValue, "/sound_transfers/0/settings/0");
}

#[test]
fn a_script_requires_the_lua_capability_and_the_capability_requires_a_script() {
    let scripted = single(json!([{"control": "/cc_params/entries/0", "script": "return 1"}]));
    refused(&device(vec![scripted.clone()], &[]), FaultKind::BadValue, "/capabilities");
    assert!(read(&device(vec![scripted.clone()], &["lua"])).is_ok());
    assert_eq!(dev(&device(vec![scripted], &["lua"])).derived_capabilities(), vec![osd::Capability::Lua]);
    refused(&device(vec![single(json!([place("/cc_params/entries/0", 0, 2)]))], &["lua"]), FaultKind::BadValue, "/capabilities");
}

#[test]
fn an_unknown_key_of_a_setting_is_refused_as_an_unknown_field() {
    let doc = device(vec![single(json!([{"control": "/cc_params/entries/0", "from": 0, "length": 1, "colour": 1}]))], &[]);
    refused(&doc, FaultKind::UnknownField, "/sound_transfers/0/settings/0/colour");
}

#[test]
fn a_setting_without_its_control_is_refused_as_missing() {
    let doc = device(vec![single(json!([{"from": 0, "length": 1}]))], &[]);
    refused(&doc, FaultKind::Missing, "/sound_transfers/0/settings/0/control");
}

#[test]
fn a_0_4_and_a_0_5_descriptor_refuse_the_settings_as_an_unknown_field() {
    let transfers = json!([single(json!([place("/cc_params/entries/0", 0, 2)]))]);
    let v05 = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}, "sound_transfers": transfers});
    let f = osd::read(&v05.to_string(), Kind::Device).expect_err("0.5 refuses");
    assert_eq!(f.kind, FaultKind::UnknownField);
    let v04 = json!({"device": {"name": "T", "osc_prefix": "/t"}, "sound_transfers": transfers});
    let f = osd::read(&v04.to_string(), Kind::Device).expect_err("0.4 refuses");
    assert_eq!(f.kind, FaultKind::UnknownField);
}

#[test]
fn converting_a_0_5_descriptor_to_0_6_adds_no_settings() {
    let v05 = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/c", "cc": 1}]}});
    let out = osd::convert_to(&v05.to_string(), Kind::Device, osd::Version::V0_6).unwrap();
    assert!(!out.contains("settings"));
    let out: Value = serde_json::from_str(&out).unwrap();
    assert!(out.get("sound_transfers").is_none());
}

fn s(control: &str, path: &str, value: u64) -> Setting {
    Setting { control: control.to_string(), param_path: path.to_string(), value }
}

fn outcomes(a: &[Setting], b: &[Setting]) -> Vec<(String, Comparison)> {
    osd::compare_settings(a, b).into_iter().map(|c| (c.control, c.outcome)).collect()
}

#[test]
fn two_sounds_of_one_device_compare_setting_by_setting_and_by_role() {
    let doc = device(vec![placed()], &[]);
    let a = osd::sound_settings(&dev(&doc), "single", &bytes(MSG), 0).unwrap();
    let b = osd::sound_settings(&dev(&doc), "single", &bytes("f0 42 3c 00 01 51 40 41 42 43 44 05 06 07 09 00 f7"), 0).unwrap();
    let got = osd::compare_settings(&a, &b);
    assert_eq!(
        got.iter().map(|c| (c.control.as_str(), c.param_path.as_str(), c.outcome)).collect::<Vec<_>>(),
        vec![
            ("/cc_params/entries/0", "/Filter/Cutoff", Comparison::Equal),
            ("/cc_params/entries/1", "/Filter/Envelope/AttackTime", Comparison::Equal),
            ("/params/entries/1", "/Output/Pan", Comparison::Differ),
        ]
    );
    assert_eq!((got[2].a, got[2].b), (Some(8), Some(9)));
    assert_eq!((got[0].a, got[0].b), (Some(0x0506), Some(0x0506)));
}

#[test]
fn a_setting_absent_from_one_sound_is_said_absent_never_equal() {
    let a = vec![s("/cc_params/entries/0", "/Filter/Cutoff", 5), s("/cc_params/entries/1", "/Filter/Envelope/AttackTime", 7)];
    let b = vec![s("/cc_params/entries/0", "/Filter/Cutoff", 5)];
    assert_eq!(
        outcomes(&a, &b),
        vec![("/cc_params/entries/0".to_string(), Comparison::Equal), ("/cc_params/entries/1".to_string(), Comparison::Absent)]
    );
    let got = osd::compare_settings(&b, &a);
    assert_eq!((got[1].a, got[1].b, got[1].outcome), (None, Some(7), Comparison::Absent));
}

#[test]
fn a_setting_absent_from_both_sounds_is_not_listed_and_two_empty_sounds_compare_to_nothing() {
    assert!(osd::compare_settings(&[], &[]).is_empty());
    let a = vec![s("/params/entries/1", "/Output/Pan", 1)];
    let got = outcomes(&a, &[]);
    assert_eq!(got, vec![("/params/entries/1".to_string(), Comparison::Absent)]);
}

/// The case of `conformance/0.6/read/<name>`, as its `case.json` states it.
fn case(name: &str) -> Value {
    let path = crate::common::repo().join("conformance/0.6/read").join(name).join("case.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("no conformance case {name}: {e}"));
    serde_json::from_str(&text).unwrap()
}

fn claimed<'a>(case: &'a Value, transfer: &str, index: u64) -> &'a Value {
    case["settings"]
        .as_array()
        .and_then(|l| l.iter().find(|c| c["transfer"] == transfer && c["index"] == index))
        .map(|c| &c["settings"])
        .unwrap_or_else(|| panic!("the case states no settings for {transfer} {index}"))
}

#[test]
fn a_conformance_case_shows_the_settings_of_a_single_sound_with_the_controls_the_reader_leaves_out() {
    let c = case("sound-settings-placed");
    let listed = claimed(&c, "single", 0).as_array().unwrap();
    assert!(listed.len() >= 3);
    for s in listed {
        assert!(s["control"].is_string() && s["param_path"].is_string() && s["value"].is_u64(), "{s}");
    }
    let settings = c["result"]["sound_transfers"][0]["settings"].as_array().unwrap();
    assert!(settings.len() > listed.len(), "the case declares controls that stay absent");
    assert!(settings.iter().any(|s| s.get("script").is_some()), "one absent control is a script");
    assert!(settings.iter().any(|s| s.get("bits").is_some() || s.get("bits_per_byte").is_some()), "one absent control is read by bits");
}

#[test]
fn a_conformance_case_shows_a_sound_of_a_bank_with_the_settings_of_a_single_sound_of_the_same_data() {
    let c = case("sound-settings-bank-sound");
    let transfers = c["result"]["sound_transfers"].as_array().unwrap();
    assert!(transfers.iter().any(|t| t["carries"] == "sound") && transfers.iter().any(|t| t["carries"] == "bank"));
    let single = claimed(&c, "single", 0);
    assert_eq!(single, claimed(&c, "bank", 1));
    assert!(!single.as_array().unwrap().is_empty());
    assert_ne!(single, claimed(&c, "bank", 0), "another sound of the bank has other settings");
}

#[test]
fn a_conformance_case_compares_two_sounds_of_which_one_setting_differs() {
    let c = case("sound-settings-compare");
    let compared = c["comparisons"].as_array().expect("the case states a comparison");
    let result = compared[0]["result"].as_array().unwrap();
    let differ = result.iter().filter(|r| r["outcome"] == "differ").count();
    let equal = result.iter().filter(|r| r["outcome"] == "equal").count();
    assert_eq!((differ, equal >= 1), (1, true));
}

#[test]
fn two_equal_settings_with_the_same_value_are_equal_and_zero_is_a_value() {
    let a = vec![s("/params/entries/1", "/Output/Pan", 0)];
    assert_eq!(outcomes(&a, &a.clone()), vec![("/params/entries/1".to_string(), Comparison::Equal)]);
    let b = vec![s("/params/entries/1", "/Output/Pan", 1)];
    assert_eq!(outcomes(&a, &b), vec![("/params/entries/1".to_string(), Comparison::Differ)]);
}
