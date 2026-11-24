//! What in a message is the sound (0.6): `sound_transfers[]`, per form of transfer of a sound
//! (`id`, `sound_id`, `carries`, `match_frame`, `elements`, and for a bank `count` and `stride`).
//! Each element (`data`, `name`, `channel`, `device_number`, `header`, `checksum`, `bank_number`,
//! `program_number`) is `{from, length}` in the message (F0 = 0) or `{script}` in Lua; `data` may
//! add `coding` (`none`, `msb_first_7in8`). The identity of a sound is the SHA-256 of the data
//! element alone, in 64 lowercase hexadecimal digits (`osd::sound_identity`); the reader says at
//! load which transfers it cannot identify without Lua (`osd::sound_identity_support`).
//!
//! The message of a transfer is as long as its `match_frame`, which holds one token per byte.

use osd::{Descriptor, FaultKind, Kind};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

/// The sound `05 06 07 08`, sha256 of those four bytes.
const IDENTITY: &str = "55e5509f8052998294266ee5b50cb592938191fb5d67f73cac2e60b0276b1bdd";

fn any(n: usize) -> Vec<Value> {
    vec![json!("any"); n]
}

/// A transfer of one sound, 17 bytes: `F0 42 ch 00 01 51 40`, a name of 4 bytes (7..11), a data of
/// 4 bytes (11..15), a checksum (15), `F7`.
fn single(elements: Value) -> Value {
    let mut frame = vec![json!(240), json!(66), json!("any"), json!(0), json!(1), json!(81), json!(64)];
    frame.extend(any(9));
    frame.push(json!(247));
    json!({"id": "single", "sound_id": "program", "carries": "sound", "match_frame": frame, "elements": elements})
}

/// A bank of two sounds, 24 bytes: `F0 42 ch 00 01 51 4C`, then 8 bytes a sound (name 4, data 4), `F7`.
fn bank(elements: Value) -> Value {
    let mut frame = vec![json!(240), json!(66), json!("any"), json!(0), json!(1), json!(81), json!(76)];
    frame.extend(any(16));
    frame.push(json!(247));
    json!({"id": "bank", "sound_id": "program", "carries": "bank", "count": 2, "stride": 8, "match_frame": frame, "elements": elements})
}

fn device(transfers: Vec<Value>, capabilities: &[&str]) -> Value {
    let mut d = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}});
    if !capabilities.is_empty() {
        d["capabilities"] = json!(capabilities);
    }
    d["sound_transfers"] = Value::Array(transfers);
    d
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

fn bytes(hex: &str) -> Vec<u8> {
    hex.split(' ').map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
}

const MSG: &str = "f0 42 3c 00 01 51 40 41 42 43 44 05 06 07 08 00 f7";

fn base() -> Value {
    json!({"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}, "channel": {"from": 2, "length": 1},
           "header": {"from": 3, "length": 4}, "checksum": {"from": 15, "length": 1}})
}

fn identity(doc: &Value, transfer: &str, index: usize, msg: &str) -> String {
    osd::sound_identity(&dev(doc), transfer, &bytes(msg), index).unwrap_or_else(|f| panic!("{f}"))
}

fn support(doc: &Value) -> Vec<(String, bool)> {
    osd::sound_identity_support(&dev(doc)).into_iter().map(|s| (s.pointer, s.without_lua)).collect()
}

#[test]
fn a_sound_transfer_with_its_elements_is_read_and_written_back_where_it_was() {
    let doc = device(vec![single(base()), bank(json!({"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}}))], &[]);
    let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert_eq!(out, doc);
}

#[test]
fn every_element_of_a_transfer_is_read_by_its_name() {
    let all = json!({"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}, "channel": {"from": 2, "length": 1},
        "device_number": {"from": 3, "length": 1}, "header": {"from": 4, "length": 3}, "checksum": {"from": 15, "length": 1},
        "bank_number": {"from": 1, "length": 1}, "program_number": {"from": 0, "length": 1}});
    let doc = device(vec![single(all)], &[]);
    let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert_eq!(out, doc);
}

#[test]
fn a_descriptor_without_sound_transfers_reads_and_writes_as_before() {
    let doc = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}});
    let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert!(out.get("sound_transfers").is_none());
    assert!(osd::sound_identity_support(&dev(&doc)).is_empty());
}

#[test]
fn a_0_4_and_a_0_5_descriptor_refuse_sound_transfers_as_an_unknown_field() {
    let transfers = json!([single(base())]);
    let v05 = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}, "sound_transfers": transfers});
    let f = osd::read(&v05.to_string(), Kind::Device).expect_err("0.5 refuses");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/sound_transfers"));
    let v04 = json!({"device": {"name": "T", "osc_prefix": "/t"}, "sound_transfers": transfers});
    let f = osd::read(&v04.to_string(), Kind::Device).expect_err("0.4 refuses");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/sound_transfers"));
}

#[test]
fn converting_a_0_5_descriptor_to_0_6_adds_no_sound_transfers() {
    let v05 = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"},
        "commands": [{"osc": "/x", "frame": [240, 247]}]});
    let out = osd::convert_to(&v05.to_string(), Kind::Device, osd::Version::V0_6).unwrap();
    let out: Value = serde_json::from_str(&out).unwrap();
    let mut want = v05.clone();
    want["$schema"] = json!(SCHEMA);
    assert_eq!(out, want);
}

#[test]
fn the_identity_is_the_sha256_of_the_data_in_64_lowercase_hexadecimal_digits() {
    let id = identity(&device(vec![single(base())], &[]), "single", 0, MSG);
    assert_eq!(id, IDENTITY);
    assert_eq!(id.len(), 64);
    assert!(id.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
}

#[test]
fn the_same_sound_renamed_has_the_same_identity() {
    let doc = device(vec![single(base())], &[]);
    assert_eq!(identity(&doc, "single", 0, "f0 42 3c 00 01 51 40 57 58 59 5a 05 06 07 08 00 f7"), IDENTITY);
}

#[test]
fn the_same_sound_on_another_channel_header_and_checksum_has_the_same_identity() {
    let doc = device(vec![single(base())], &[]);
    assert_eq!(identity(&doc, "single", 0, "f0 42 3d 00 01 51 40 41 42 43 44 05 06 07 08 00 f7"), IDENTITY);
    assert_eq!(identity(&doc, "single", 0, "f0 42 3c 00 02 51 40 41 42 43 44 05 06 07 08 6d f7"), IDENTITY);
}

#[test]
fn a_device_number_a_bank_number_and_a_program_number_stay_out_of_the_identity() {
    let all = json!({"data": {"from": 11, "length": 4}, "device_number": {"from": 2, "length": 1}, "bank_number": {"from": 3, "length": 1}, "program_number": {"from": 4, "length": 1}});
    let doc = device(vec![single(all)], &[]);
    assert_eq!(identity(&doc, "single", 0, "f0 42 09 07 33 51 40 41 42 43 44 05 06 07 08 00 f7"), IDENTITY);
}

#[test]
fn a_sound_with_other_data_has_another_identity() {
    let doc = device(vec![single(base())], &[]);
    assert_ne!(identity(&doc, "single", 0, "f0 42 3c 00 01 51 40 41 42 43 44 05 06 07 09 00 f7"), IDENTITY);
}

#[test]
fn the_sound_alone_and_the_first_sound_of_a_bank_with_the_same_data_have_one_identity() {
    let doc = device(vec![single(base()), bank(json!({"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}}))], &[]);
    let b = "f0 42 3c 00 01 51 4c 41 42 43 44 05 06 07 08 45 46 47 48 05 06 07 09 f7";
    assert_eq!(identity(&doc, "bank", 0, b), IDENTITY);
    assert_eq!(identity(&doc, "single", 0, MSG), IDENTITY);
}

#[test]
fn the_elements_of_the_sound_i_of_a_bank_are_shifted_by_i_times_the_stride() {
    let doc = device(vec![bank(json!({"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}}))], &[]);
    let b = "f0 42 3c 00 01 51 4c 41 42 43 44 05 06 07 08 45 46 47 48 05 06 07 09 f7";
    assert_eq!(identity(&doc, "bank", 0, b), IDENTITY);
    let second = identity(&doc, "bank", 1, b);
    assert_ne!(second, IDENTITY);
    // The second sound with the data 05 06 07 08 has the identity of the first.
    assert_eq!(identity(&doc, "bank", 1, "f0 42 3c 00 01 51 4c 41 42 43 44 00 00 00 00 45 46 47 48 05 06 07 08 f7"), IDENTITY);
}

#[test]
fn a_name_in_the_data_is_blanked_before_the_identity_so_the_same_sound_renamed_has_one_identity() {
    let doc = device(vec![single(json!({"data": {"from": 7, "length": 8}, "name": {"in_data": {"from": 0, "length": 4}}}))], &[]);
    let id = identity(&doc, "single", 0, MSG);
    assert_eq!(id, "8326846b5e0221e2401c5aaf7d227577fe81536e5a4217a2c7c74feb8c6c9b83", "the data 00 00 00 00 05 06 07 08");
    assert_eq!(identity(&doc, "single", 0, "f0 42 3c 00 01 51 40 57 58 59 5a 05 06 07 08 00 f7"), id);
    assert_ne!(identity(&doc, "single", 0, "f0 42 3c 00 01 51 40 41 42 43 44 05 06 07 09 00 f7"), id);
    // A name in the middle of the data blanks its bytes and keeps the length: 41 42 00 00 05 06 07 08.
    let mid = device(vec![single(json!({"data": {"from": 7, "length": 8}, "name": {"in_data": {"from": 2, "length": 2}}}))], &[]);
    assert_eq!(identity(&mid, "single", 0, MSG), "99072e22f9e2aa94a5d4f4801086240605ad0c4fe1703f6fb4e4babf199e7dda");
}

#[test]
fn a_name_in_a_packed_data_counts_in_the_unpacked_data_and_straddles_two_groups() {
    let mut t = single(json!({"data": {"from": 7, "length": 16, "coding": "msb_first_7in8"}, "name": {"in_data": {"from": 5, "length": 4}}}));
    let mut frame = vec![json!(240), json!(66), json!("any"), json!(0), json!(1), json!(81), json!(64)];
    frame.extend(any(16));
    frame.push(json!(247));
    t["match_frame"] = json!(frame);
    let doc = device(vec![t], &[]);
    let base = identity(&doc, "single", 0, "f0 42 3c 00 01 51 40 21 01 12 23 34 45 06 67 26 78 09 1a 2b 3c 5d 6e f7");
    let renamed = identity(&doc, "single", 0, "f0 42 3c 00 01 51 40 41 01 12 23 34 45 41 42 24 33 04 1a 2b 3c 5d 6e f7");
    let other_setting = identity(&doc, "single", 0, "f0 42 3c 00 01 51 40 21 01 12 23 34 45 06 67 26 78 09 1a 2b 3d 5d 6e f7");
    assert_eq!(base, "2c54974e9dc2a3343b500797f20be2c6f741290b12450f6ebda0de5ee4e3898b");
    assert_eq!(renamed, base);
    assert_eq!(other_setting, "26960d38590fd65da75a7385a7965377991c29aacd0c71e8275e79afc62df49f");
}

#[test]
fn a_name_in_the_data_that_goes_past_the_unpacked_data_is_refused_at_the_name() {
    let past = device(vec![single(json!({"data": {"from": 7, "length": 8}, "name": {"in_data": {"from": 6, "length": 4}}}))], &[]);
    refused(&past, FaultKind::BadValue, "/sound_transfers/0/elements/name");
    // 8 packed bytes hold 7 data bytes: the name 5..9 goes past them.
    let packed = device(vec![single(json!({"data": {"from": 7, "length": 8, "coding": "msb_first_7in8"}, "name": {"in_data": {"from": 5, "length": 4}}}))], &[]);
    refused(&packed, FaultKind::BadValue, "/sound_transfers/0/elements/name");
    // Ending on the last byte of the unpacked data is read.
    assert!(read(&device(vec![single(json!({"data": {"from": 7, "length": 8, "coding": "msb_first_7in8"}, "name": {"in_data": {"from": 3, "length": 4}}}))], &[])).is_ok());
}

#[test]
fn a_name_given_by_positions_of_the_message_that_covers_the_data_is_refused() {
    let inside = device(vec![single(json!({"data": {"from": 7, "length": 8}, "name": {"from": 9, "length": 2}}))], &[]);
    refused(&inside, FaultKind::BadValue, "/sound_transfers/0/elements/name");
    let packed = device(vec![single(json!({"data": {"from": 7, "length": 8, "coding": "msb_first_7in8"}, "name": {"from": 7, "length": 4}}))], &[]);
    refused(&packed, FaultKind::BadValue, "/sound_transfers/0/elements/name");
    let straddle = device(vec![single(json!({"data": {"from": 7, "length": 4}, "name": {"from": 9, "length": 4}}))], &[]);
    refused(&straddle, FaultKind::BadValue, "/sound_transfers/0/elements/name");
}

#[test]
fn a_name_that_mixes_in_data_with_positions_or_a_script_is_refused_at_the_name() {
    for extra in [json!({"from": 7, "length": 4}), json!({"length": 4}), json!({"script": "return msg"})] {
        let mut name = json!({"in_data": {"from": 0, "length": 4}});
        name.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let doc = device(vec![single(json!({"data": {"from": 7, "length": 8}, "name": name}))], &["lua"]);
        refused(&doc, FaultKind::BadValue, "/sound_transfers/0/elements/name");
    }
}

#[test]
fn a_name_in_the_data_of_a_data_given_by_a_script_is_refused_at_the_name() {
    let doc = device(vec![single(json!({"data": {"script": "return msg"}, "name": {"in_data": {"from": 0, "length": 4}}}))], &["lua"]);
    refused(&doc, FaultKind::BadValue, "/sound_transfers/0/elements/name");
}

#[test]
fn a_name_in_the_data_keeps_the_identity_computable_without_lua_and_is_written_back() {
    let doc = device(vec![single(json!({"data": {"from": 7, "length": 8}, "name": {"in_data": {"from": 0, "length": 4}}}))], &[]);
    assert_eq!(support(&doc), vec![("/sound_transfers/0".to_string(), true)]);
    let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert_eq!(out, doc);
}

#[test]
fn a_data_packed_in_7_bits_is_unpacked_before_the_identity() {
    // 45 | 05 06 07 08 09 0a 0b: the high bits of bytes 0, 2 and 6 are set: 85 06 87 08 09 0a 8b.
    let packed = json!({"data": {"from": 7, "length": 8, "coding": "msb_first_7in8"}});
    let doc = device(vec![single(packed)], &[]);
    let id = identity(&doc, "single", 0, "f0 42 3c 00 01 51 40 45 05 06 07 08 09 0a 0b 41 f7");
    assert_eq!(id, "6946b9a650125289f56fbe6e4a9f3858c4648696ed8a443abbeb1ff1bc70f805");
}

#[test]
fn a_last_group_of_a_packed_data_may_be_partial() {
    // KORG NTS-1, same note: 11 bytes of MIDI hold 9 bytes of data. The first group is 45 | 05..0b
    // (85 06 87 08 09 0a 8b); the last, 03 0c 0d, is a byte of high bits and two bytes: 8c 8d.
    let doc = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}, "sound_transfers": [
        {"id": "s", "carries": "sound", "match_frame": [240, "any", "any", "any", "any", "any", "any", "any", "any", "any", "any", "any", 247],
         "elements": {"data": {"from": 1, "length": 11, "coding": "msb_first_7in8"}}}]});
    let id = osd::sound_identity(&dev(&doc), "s", &bytes("f0 45 05 06 07 08 09 0a 0b 03 0c 0d f7"), 0).unwrap();
    assert_eq!(id, "bd2e70e1f1a63819d3687337923943794790aafe13ea4bdb661f903ee6ff02b3");
}

#[test]
fn a_data_with_coding_none_is_the_bytes_of_the_message() {
    let explicit = device(vec![single(json!({"data": {"from": 11, "length": 4, "coding": "none"}}))], &[]);
    assert_eq!(identity(&explicit, "single", 0, MSG), IDENTITY);
}

#[test]
fn a_coding_word_outside_the_list_is_refused_at_its_address() {
    for word in ["lsb_first_7in8", "msb_last_7in8", "7bit", "mcoded7", ""] {
        let doc = device(vec![single(json!({"data": {"from": 7, "length": 8, "coding": word}}))], &[]);
        refused(&doc, FaultKind::BadValue, "/sound_transfers/0/elements/data/coding");
    }
}

#[test]
fn the_word_packing_of_0_5_drafts_is_no_field_of_the_data_element() {
    let doc = device(vec![single(json!({"data": {"from": 7, "length": 8, "packing": "msb_first_7in8"}}))], &[]);
    refused(&doc, FaultKind::UnknownField, "/sound_transfers/0/elements/data/packing");
}

#[test]
fn a_coding_is_a_field_of_the_data_element_only() {
    let doc = device(vec![single(json!({"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4, "coding": "none"}}))], &[]);
    refused(&doc, FaultKind::UnknownField, "/sound_transfers/0/elements/name/coding");
}

#[test]
fn a_reader_without_lua_knows_at_load_for_which_transfers_it_cannot_compute_the_identity() {
    // A data script: no identity without Lua.
    let doc = device(vec![single(base()), single(json!({"data": {"script": "return msg"}})), bank(json!({"data": {"script": "return msg"}}))], &["lua"]);
    let mut doc = doc;
    doc["sound_transfers"][1]["id"] = json!("scripted");
    assert_eq!(
        support(&doc),
        vec![("/sound_transfers/0".to_string(), true), ("/sound_transfers/1".to_string(), false), ("/sound_transfers/2".to_string(), false)]
    );
}

#[test]
fn a_name_script_is_taken_as_outside_the_data_so_positions_for_the_data_keep_the_identity_computable() {
    let doc = device(vec![single(json!({"data": {"from": 7, "length": 8}, "name": {"script": "return ''"}}))], &["lua"]);
    assert_eq!(support(&doc), vec![("/sound_transfers/0".to_string(), true)]);
}

#[test]
fn a_name_script_outside_the_data_leaves_the_identity_computable() {
    let doc = device(vec![single(json!({"data": {"from": 11, "length": 4}, "name": {"script": "return ''"}}))], &["lua"]);
    assert_eq!(support(&doc), vec![("/sound_transfers/0".to_string(), true)]);
    assert_eq!(identity(&doc, "single", 0, MSG), IDENTITY);
}

#[test]
fn a_script_on_a_transport_part_leaves_the_identity_computable() {
    for part in ["channel", "device_number", "header", "checksum", "bank_number", "program_number"] {
        let doc = device(vec![single(json!({"data": {"from": 11, "length": 4}, part: {"script": "return 0"}}))], &["lua"]);
        assert_eq!(support(&doc), vec![("/sound_transfers/0".to_string(), true)], "{part}");
        assert_eq!(identity(&doc, "single", 0, MSG), IDENTITY, "{part}");
    }
}

#[test]
fn an_identity_that_needs_lua_is_not_computed_by_the_reader() {
    let doc = device(vec![single(json!({"data": {"script": "return msg"}}))], &["lua"]);
    assert!(osd::sound_identity(&dev(&doc), "single", &bytes(MSG), 0).is_err());
}

#[test]
fn a_message_that_does_not_match_the_frame_or_a_transfer_unnamed_gives_no_identity() {
    let doc = device(vec![single(base())], &[]);
    assert!(osd::sound_identity(&dev(&doc), "single", &bytes("f0 43 3c 00 01 51 40 41 42 43 44 05 06 07 08 00 f7"), 0).is_err());
    assert!(osd::sound_identity(&dev(&doc), "single", &bytes("f0 42 3c"), 0).is_err());
    assert!(osd::sound_identity(&dev(&doc), "nothing", &bytes(MSG), 0).is_err());
}

#[test]
fn a_script_requires_the_lua_capability_and_the_capability_requires_a_script() {
    let scripted = single(json!({"data": {"script": "return msg"}}));
    refused(&device(vec![scripted.clone()], &[]), FaultKind::BadValue, "/capabilities");
    refused(&device(vec![single(base())], &["lua"]), FaultKind::BadValue, "/capabilities");
    assert!(read(&device(vec![scripted], &["lua"])).is_ok());
    for part in ["name", "channel", "checksum"] {
        let doc = device(vec![single(json!({"data": {"from": 11, "length": 4}, part: {"script": "return 0"}}))], &[]);
        refused(&doc, FaultKind::BadValue, "/capabilities");
    }
}

#[test]
fn derived_capabilities_include_lua_when_an_element_is_a_script() {
    let d = dev(&device(vec![single(json!({"data": {"script": "return msg"}}))], &["lua"]));
    assert_eq!(d.derived_capabilities(), vec![osd::Capability::Lua]);
    assert!(dev(&device(vec![single(base())], &[])).derived_capabilities().is_empty());
}

#[test]
fn an_element_reaching_past_the_message_is_refused_where_it_is() {
    // The message has 17 bytes: an element may end at 17.
    assert!(read(&device(vec![single(json!({"data": {"from": 13, "length": 4}}))], &[])).is_ok());
    refused(&device(vec![single(json!({"data": {"from": 14, "length": 4}}))], &[]), FaultKind::BadValue, "/sound_transfers/0/elements/data");
    refused(&device(vec![single(json!({"data": {"from": 11, "length": 4}, "name": {"from": 16, "length": 4}}))], &[]), FaultKind::BadValue, "/sound_transfers/0/elements/name");
    for part in ["channel", "device_number", "header", "checksum", "bank_number", "program_number"] {
        let doc = device(vec![single(json!({"data": {"from": 11, "length": 4}, part: {"from": 17, "length": 1}}))], &[]);
        refused(&doc, FaultKind::BadValue, &format!("/sound_transfers/0/elements/{part}"));
    }
}

#[test]
fn the_last_sound_of_a_bank_must_stay_inside_the_message_too() {
    // 24 bytes, two sounds, stride 8: data at 11 and 19, ending at 23.
    assert!(read(&device(vec![bank(json!({"data": {"from": 11, "length": 4}}))], &[])).is_ok());
    refused(&device(vec![bank(json!({"data": {"from": 17, "length": 4}}))], &[]), FaultKind::BadValue, "/sound_transfers/0/elements/data");
    let mut wide = bank(json!({"data": {"from": 11, "length": 4}}));
    wide["stride"] = json!(14);
    refused(&device(vec![wide], &[]), FaultKind::BadValue, "/sound_transfers/0/elements/data");
}

#[test]
fn an_empty_data_is_refused_at_its_length() {
    refused(&device(vec![single(json!({"data": {"from": 11, "length": 0}}))], &[]), FaultKind::BadValue, "/sound_transfers/0/elements/data/length");
}

#[test]
fn a_data_overlapping_the_header_the_checksum_or_a_transport_part_is_refused() {
    refused(&device(vec![single(json!({"data": {"from": 9, "length": 4}, "header": {"from": 3, "length": 7}}))], &[]), FaultKind::BadValue, "/sound_transfers/0/elements/data");
    refused(&device(vec![single(json!({"data": {"from": 11, "length": 5}, "checksum": {"from": 15, "length": 1}}))], &[]), FaultKind::BadValue, "/sound_transfers/0/elements/data");
    for part in ["channel", "device_number", "bank_number", "program_number"] {
        let doc = device(vec![single(json!({"data": {"from": 11, "length": 4}, part: {"from": 14, "length": 1}}))], &[]);
        refused(&doc, FaultKind::BadValue, "/sound_transfers/0/elements/data");
    }
    // Touching without sharing a byte is not an overlap.
    assert!(read(&device(vec![single(json!({"data": {"from": 11, "length": 4}, "checksum": {"from": 15, "length": 1}, "header": {"from": 3, "length": 8}}))], &[])).is_ok());
}

#[test]
fn two_transport_parts_that_overlap_are_refused_at_the_later_one() {
    let doc = device(vec![single(json!({"data": {"from": 11, "length": 4}, "channel": {"from": 2, "length": 1}, "device_number": {"from": 2, "length": 1}}))], &[]);
    refused(&doc, FaultKind::BadValue, "/sound_transfers/0/elements/device_number");
    let doc = device(vec![single(json!({"data": {"from": 11, "length": 4}, "header": {"from": 3, "length": 4}, "program_number": {"from": 6, "length": 2}}))], &[]);
    refused(&doc, FaultKind::BadValue, "/sound_transfers/0/elements/program_number");
}

#[test]
fn an_element_holding_positions_and_a_script_is_refused_where_it_is() {
    let doc = device(vec![single(json!({"data": {"from": 11, "length": 4, "script": "return msg"}}))], &["lua"]);
    refused(&doc, FaultKind::BadValue, "/sound_transfers/0/elements/data");
}

/// The conformance cases of 0.6 for the sound transfers (`conformance/0.6/read/<case>`): a
/// descriptor with its `case.json`. A result case states `identity_support` (one entry per
/// transfer, `{pointer, without_lua}`) and, when it computes, `identities` (`{transfer, index,
/// message, identity}`); a refusal states its fault and whether only the reader refuses it.
/// `(case, without_lua per transfer | fault kind, fault pointer, schema_accepts)`.
const RESULT_CASES: [(&str, &[bool]); 8] = [
    ("sound-transfer-identity", &[true, true]),
    ("sound-transfer-name-in-data", &[true]),
    ("sound-transfer-name-in-packed-data", &[true]),
    ("sound-transfer-packed-7bit", &[true]),
    ("sound-transfer-data-script", &[false]),
    ("sound-transfer-name-script-data-by-positions", &[true]),
    ("sound-transfer-transport-scripts", &[true]),
    ("sound-transfer-name-script-outside-data", &[true]),
];

const REFUSED_CASES: [(&str, &str, &str, bool); 14] = [
    ("sound-transfer-element-outside-refused", "BadValue", "/sound_transfers/0/elements/data", true),
    ("sound-transfer-name-outside-refused", "BadValue", "/sound_transfers/0/elements/name", true),
    ("sound-transfer-data-empty-refused", "BadValue", "/sound_transfers/0/elements/data/length", false),
    ("sound-transfer-data-overlaps-header-refused", "BadValue", "/sound_transfers/0/elements/data", true),
    ("sound-transfer-data-overlaps-checksum-refused", "BadValue", "/sound_transfers/0/elements/data", true),
    ("sound-transfer-data-overlaps-transport-refused", "BadValue", "/sound_transfers/0/elements/data", true),
    ("sound-transfer-transport-parts-overlap-refused", "BadValue", "/sound_transfers/0/elements/device_number", true),
    ("sound-transfer-positions-and-script-refused", "BadValue", "/sound_transfers/0/elements/data", false),
    ("sound-transfer-coding-outside-list-refused", "BadValue", "/sound_transfers/0/elements/data/coding", false),
    ("sound-transfer-name-in-data-outside-refused", "BadValue", "/sound_transfers/0/elements/name", true),
    ("sound-transfer-name-positions-over-data-refused", "BadValue", "/sound_transfers/0/elements/name", true),
    ("sound-transfer-name-in-data-with-positions-refused", "BadValue", "/sound_transfers/0/elements/name", false),
    ("sound-transfer-name-in-data-script-refused", "BadValue", "/sound_transfers/0/elements/name", true),
    ("capabilities-sound-transfer-script-missing-refused", "BadValue", "/capabilities", true),
];

fn case_of(name: &str) -> (Value, Value) {
    let dir = crate::common::repo().join("conformance/0.6/read").join(name);
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap_or_else(|e| panic!("{name}/{f}: {e}"));
    (serde_json::from_str(&read("case.json")).unwrap(), serde_json::from_str(&read("descriptor.json")).unwrap())
}

#[test]
fn the_conformance_cases_of_sound_transfers_state_each_result_and_each_refusal() {
    let mut failures = Vec::new();
    for (name, want) in RESULT_CASES {
        let (case, doc) = case_of(name);
        let got: Vec<bool> = case["identity_support"].as_array().map_or(vec![], |a| a.iter().map(|s| s["without_lua"] == json!(true)).collect());
        if got != want || case.get("fault").is_some() || doc["sound_transfers"].as_array().map(Vec::len) != Some(want.len()) {
            failures.push(format!("{name}: states {got:?}, expected {want:?} for the transfers of its descriptor"));
        }
    }
    for (name, kind, pointer, reader_only) in REFUSED_CASES {
        let (case, _) = case_of(name);
        if case["fault"] != json!({"kind": kind, "pointer": pointer}) || (case.get("schema_accepts") == Some(&json!(true))) != reader_only {
            failures.push(format!("{name}: states {}, expected {kind} at {pointer}", case["fault"]));
        }
    }
    crate::common::report("sound transfer cases fail", failures);
}

#[test]
fn a_conformance_case_gives_one_identity_for_a_message_the_same_sound_renamed_on_another_channel_and_in_a_bank() {
    let (case, doc) = case_of("sound-transfer-identity");
    let ids = case["identities"].as_array().expect("identities");
    assert!(ids.len() >= 4);
    assert!(ids.iter().all(|i| i["identity"] == json!(IDENTITY)), "one identity expected");
    let messages: std::collections::BTreeSet<&str> = ids.iter().map(|i| i["message"].as_str().unwrap()).collect();
    assert!(messages.len() >= 3, "a message, the same sound renamed, on another channel: {messages:?}");
    assert!(ids.iter().any(|i| i["transfer"] == "bank") && ids.iter().any(|i| i["transfer"] == "single"));
    let ts = doc["sound_transfers"].as_array().unwrap();
    assert_eq!(ts[0]["sound_id"], ts[1]["sound_id"]);
    assert_eq!((ts[0]["carries"].as_str(), ts[1]["carries"].as_str()), (Some("sound"), Some("bank")));
}

#[test]
fn the_conformance_cases_of_the_name_in_the_data_and_of_the_packing_state_their_identity() {
    let (case, doc) = case_of("sound-transfer-name-in-data");
    let blanked = "8326846b5e0221e2401c5aaf7d227577fe81536e5a4217a2c7c74feb8c6c9b83";
    assert!(case["identities"].as_array().unwrap().iter().all(|i| i["identity"] == json!(blanked)));
    let e =&doc["sound_transfers"][0]["elements"];
    assert_eq!(e["name"], json!({"in_data": {"from": 0, "length": 4}}));
    let (case, doc) = case_of("sound-transfer-name-in-packed-data");
    let ids = case["identities"].as_array().unwrap();
    assert_eq!(ids.len(), 3);
    assert_eq!(ids[0]["identity"], ids[1]["identity"], "the same sound renamed");
    assert_ne!(ids[0]["identity"], ids[2]["identity"], "one setting differs");
    assert_eq!(doc["sound_transfers"][0]["elements"]["data"]["coding"], "msb_first_7in8");
    let (case, doc) = case_of("sound-transfer-packed-7bit");
    assert_eq!(doc["sound_transfers"][0]["elements"]["data"]["coding"], "msb_first_7in8");
    assert_eq!(case["identities"][0]["identity"], "6946b9a650125289f56fbe6e4a9f3858c4648696ed8a443abbeb1ff1bc70f805");
}

#[test]
fn an_unknown_element_or_key_or_a_carries_outside_the_list_is_refused() {
    refused(&device(vec![single(json!({"data": {"from": 11, "length": 4}, "volume": {"from": 1, "length": 1}}))], &[]), FaultKind::UnknownField, "/sound_transfers/0/elements/volume");
    refused(&device(vec![single(json!({"data": {"from": 11, "length": 4, "colour": 1}}))], &[]), FaultKind::UnknownField, "/sound_transfers/0/elements/data/colour");
    let mut t = single(base());
    t["carries"] = json!("patch");
    refused(&device(vec![t], &[]), FaultKind::BadValue, "/sound_transfers/0/carries");
}
