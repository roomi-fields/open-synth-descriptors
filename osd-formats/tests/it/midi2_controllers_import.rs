//! A controller list (ChCtrlList or AllCtrlList of M2-117-UM) imports into an OSD device `osd`
//! reads strictly; what the list cannot say is named in the losses at its JSON pointer, and a
//! list that is not one is refused.

use crate::common::midi2::*;
use crate::common::midnam::lost_at;
use osd_formats::{import, FaultKind, Format};
use serde_json::{json, Value};

#[test]
fn a_list_imports_as_controls_with_their_address_range_default_priority_and_modes() {
    let text = json!([
        {"title": "Cutoff", "ctrlType": "cc", "ctrlIndex": [74], "numSigBits": 7, "minMax": [167772160u64, 4056358001u64], "default": 2352005900u64, "priority": 2,
         "transmit": "absolute", "recognize": "both", "stepCount": 91, "channel": 3, "typeHint": "continuous"},
        {"title": "N", "ctrlType": "nrpn", "ctrlIndex": [3, 9], "numSigBits": 14, "minMax": [0u64, 4294967295u64]},
        {"title": "R", "ctrlType": "rpn", "ctrlIndex": [0, 2], "numSigBits": 14},
    ])
    .to_string();
    let (d, _) = imported(&text);
    let c = controls(&d);
    assert_eq!(c.len(), 3);
    assert_eq!(c[0]["cc"], 74);
    assert_eq!(c[0]["range"], json!([5, 120]));
    assert_eq!(c[0]["default"], 70);
    assert_eq!(c[0]["priority"], 2);
    assert_eq!(c[0]["transmit"], "absolute");
    assert_eq!(c[0]["recognize"], "both");
    assert_eq!(c[0]["step_count"], 91);
    assert_eq!(c[0]["channel"], 2);
    assert_eq!((c[1]["nrpn_msb"].clone(), c[1]["nrpn_lsb"].clone()), (json!(3), json!(9)));
    assert_eq!((c[2]["rpn_msb"].clone(), c[2]["rpn_lsb"].clone()), (json!(0), json!(2)));
}

#[test]
fn a_14_bit_cc_gets_its_lsb_pair() {
    let text = json!([
        {"title": "A", "ctrlType": "cc", "ctrlIndex": [1], "numSigBits": 14, "minMax": [0u64, 4294967295u64]},
        {"title": "B", "ctrlType": "cc", "ctrlIndex": [2], "numSigBits": 7},
    ])
    .to_string();
    let (d, _) = imported(&text);
    let cc = d["cc_params"]["entries"].as_array().unwrap();
    assert_eq!(cc.len(), 2);
    assert_eq!(cc[0]["cc_lsb"], 33);
    assert!(cc[1].get("cc_lsb").is_none(), "{}", cc[1]);
}

#[test]
fn a_ctrl_type_outside_the_nine_of_the_standard_is_refused_at_its_pointer_and_param_is_one() {
    for word in ["param", "sysex", "CC", ""] {
        let list = json!([cc_entry(0), {"title": "P", "ctrlType": word, "ctrlIndex": [12], "minMax": [0, 63]}]);
        let f = refused(&list.to_string());
        assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/1/ctrlType"), "{word:?}");
        let f = refused(&json!({"ChCtrlList": list, "CtrlMapList": []}).to_string());
        assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/ChCtrlList/1/ctrlType"), "{word:?}");
    }
    let f = refused(&json!([{"title": "P", "ctrlType": 3, "ctrlIndex": [1]}]).to_string());
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/0/ctrlType"));
}

#[test]
fn an_entry_of_a_standard_type_the_descriptor_cannot_carry_is_a_loss_at_its_pointer_not_a_refusal() {
    let text = json!({
        "ChCtrlList": [
            cc_entry(1),
            {"title": "Press", "ctrlType": "chPress"},
            {"title": "Poly", "ctrlType": "pPress"},
            {"title": "Bend", "ctrlType": "pBend"},
            {"title": "Reg", "ctrlType": "pnrc", "ctrlIndex": [3]},
            {"title": "Assign", "ctrlType": "pnac", "ctrlIndex": [4]},
            {"title": "Pitch", "ctrlType": "pnp"},
            cc_entry(2),
        ],
        "CtrlMapList": [],
    })
    .to_string();
    let (d, lost) = imported(&text);
    let c = controls(&d);
    assert_eq!(c.len(), 2, "only the two CC are carried: {c:?}");
    assert_eq!((c[0]["cc"].clone(), c[1]["cc"].clone()), (json!(1), json!(2)));
    for n in 1..=6 {
        let at = lost_at(&lost, &format!("/ChCtrlList/{n}"));
        assert_eq!(at.len(), 1, "entry {n}: {lost:?}");
        assert!(at[0].reason.contains("ctrlType"), "{:?}", at[0]);
    }
}

#[test]
fn a_list_of_only_standard_types_the_descriptor_cannot_carry_still_imports() {
    let (d, lost) = imported(&json!([{"ctrlType": "pBend"}, {"ctrlType": "chPress"}]).to_string());
    assert!(controls(&d).is_empty(), "{d}");
    assert_eq!(lost_at(&lost, "/0").len(), 1, "{lost:?}");
    assert_eq!(lost_at(&lost, "/1").len(), 1, "{lost:?}");
}

#[test]
fn an_import_never_makes_a_params_block_so_no_frame_is_made_up() {
    let text = json!([
        cc_entry(1),
        {"ctrlType": "nrpn", "ctrlIndex": [1, 2]},
        {"ctrlType": "rpn", "ctrlIndex": [0, 2]},
        {"ctrlType": "pnp"},
        {"ctrlType": "pnrc", "ctrlIndex": [1]},
    ])
    .to_string();
    let (d, lost) = imported(&text);
    assert!(d.get("params").is_none(), "{d}");
    assert!(!lost.iter().any(|l| l.place.starts_with("/params")), "{lost:?}");
}

#[test]
fn the_type_hints_become_expects_words_and_a_map_becomes_the_named_values() {
    let text = json!({
        "ChCtrlList": [
            {"title": "A", "ctrlType": "cc", "ctrlIndex": [1], "typeHint": "toggle"},
            {"title": "B", "ctrlType": "cc", "ctrlIndex": [2], "typeHint": "momentary"},
            {"title": "C", "ctrlType": "cc", "ctrlIndex": [3], "typeHint": "valueSelect", "ctrlMapId": "m1", "numSigBits": 7, "minMax": [0, 67108864]},
        ],
        "CtrlMapList": [{"ctrlMapId": "m1", "ctrlMap": [{"value": 0, "title": "Saw"}, {"value": 1, "title": "Square"}, {"value": 2, "title": "Tri"}]}],
    })
    .to_string();
    let (d, _) = imported(&text);
    let c = controls(&d);
    assert_eq!(c[0]["expects"], "switch");
    assert_eq!(c[1]["expects"], "momentary");
    assert_eq!(c[2]["expects"], "discrete");
    assert_eq!(c[2]["values"], json!({"Saw": 0, "Square": 1, "Tri": 2}));
}

#[test]
fn a_chctrllist_array_an_allctrllist_array_and_the_object_all_import() {
    let entries = json!([cc_entry(1), cc_entry(2)]);
    for text in [entries.to_string(), json!({"ChCtrlList": entries, "CtrlMapList": []}).to_string()] {
        let (d, _) = imported(&text);
        assert_eq!(controls(&d).len(), 2, "{text}");
    }
}

#[test]
fn a_param_path_of_known_terms_becomes_a_role_and_the_device_names_its_vocabulary() {
    let mut a = cc_entry(1);
    a["paramPath"] = json!("/Amplifier/Envelope/AttackTime");
    let mut b = cc_entry(2);
    b["paramPath"] = json!("/Oscillator/1/Level");
    let (d, lost) = imported(&json!([a, b]).to_string());
    let c = controls(&d);
    assert_eq!(c[0]["role"], json!({"parts": [{"part": "amplifier"}, {"part": "envelope"}], "setting": "attack-time"}));
    assert_eq!(c[1]["role"], json!({"parts": [{"part": "oscillator", "index": 1}], "setting": "level"}));
    assert_eq!(d["role_vocabulary"], "1");
    assert!(c[0].get("section").is_none());
    assert!(!lost.iter().any(|l| l.reason.contains("no known role")), "{lost:?}");
}

#[test]
fn a_param_path_with_no_known_role_is_kept_as_the_section_and_named_in_the_losses() {
    let mut a = cc_entry(1);
    a["paramPath"] = json!("/Mystery/Knob");
    let mut b = cc_entry(2);
    b["paramPath"] = json!("/Filter/Cutoff");
    let mut c = cc_entry(3);
    c["paramPath"] = json!("Not/Absolute");
    let (d, lost) = imported(&json!([a, b, c]).to_string());
    let ctl = controls(&d);
    assert_eq!(ctl[0]["section"], "/Mystery/Knob");
    assert!(ctl[0].get("role").is_none());
    assert!(ctl[1].get("role").is_some());
    assert!(ctl[2].get("section").is_none(), "{}", ctl[2]);
    let found = lost_at(&lost, "/0");
    assert!(found.iter().any(|l| l.reason.contains("no known role")), "{lost:?}");
    assert!(lost_at(&lost, "/1").iter().all(|l| !l.reason.contains("no known role")), "{lost:?}");
    assert!(lost_at(&lost, "/2").iter().any(|l| l.reason.contains("no known role")), "{lost:?}");
}

#[test]
fn the_losses_of_an_object_are_placed_by_its_json_pointer() {
    let mut a = cc_entry(1);
    a["paramPath"] = json!("/Mystery/Knob");
    let text = json!({"ChCtrlList": [cc_entry(0), a], "CtrlMapList": []}).to_string();
    let (_, lost) = imported(&text);
    assert!(lost_at(&lost, "/ChCtrlList/1").iter().any(|l| l.reason.contains("no known role")), "{lost:?}");
    assert_eq!(lost_at(&lost, "/ChCtrlList/0/title").len(), 1, "{lost:?}");
}

#[test]
fn the_title_is_named_lost_at_its_pointer_for_each_entry() {
    let (_, lost) = imported(&json!([cc_entry(1), cc_entry(2)]).to_string());
    assert_eq!(lost_at(&lost, "/0/title").len(), 1, "{lost:?}");
    assert_eq!(lost_at(&lost, "/1/title").len(), 1, "{lost:?}");
}

#[test]
fn the_identity_is_the_author_as_vendor_else_the_host_of_the_url_and_a_fixed_name_and_the_losses_say_so() {
    let (d, lost) = imported(&json!([cc_entry(1)]).to_string());
    assert_eq!(d["device"]["vendor"], "A maker");
    assert_eq!(d["device"]["name"], "MIDI 2.0 device");
    for place in ["/vendor", "/name"] {
        let l = lost_at(&lost, place);
        assert_eq!(l.len(), 1, "{place}: {lost:?}");
        assert!(l[0].reason.contains("identity not in the list"), "{:?}", l[0]);
    }
    let mut s = source();
    s.author = None;
    let t = import(&json!([cc_entry(1)]).to_string(), Format::Midi2Controllers, &s).unwrap();
    assert_eq!(crate::common::electra::json_of(&t.value)["device"]["vendor"], "example.org");
}

#[test]
fn the_import_carries_the_source_given_and_every_control_has_its_own_address() {
    let (d, _) = imported(&json!([cc_entry(1), cc_entry(2), cc_entry(3)]).to_string());
    assert_eq!(d["_sources"].as_array().unwrap().len(), 1);
    assert_eq!(d["_sources"][0]["license"], "CC-BY-4.0");
    assert_eq!(d["_sources"][0]["author"], "A maker");
    assert_eq!(d["$schema"], "https://w3id.org/osd/0.6/device.schema.json");
    let mut oscs: Vec<String> = controls(&d).iter().map(|e| e["osc"].as_str().unwrap().to_string()).collect();
    oscs.sort();
    oscs.dedup();
    assert_eq!(oscs.len(), 3);
}

#[test]
fn the_order_of_the_controls_and_of_the_losses_follows_the_source() {
    let (d, lost) = imported(&json!([cc_entry(9), cc_entry(3), cc_entry(5)]).to_string());
    let cc: Vec<i64> = controls(&d).iter().map(|e| e["cc"].as_i64().unwrap()).collect();
    assert_eq!(cc, [9, 3, 5]);
    let titles: Vec<&str> = lost.iter().filter(|l| l.place.ends_with("/title")).map(|l| l.place.as_str()).collect();
    assert_eq!(titles, ["/0/title", "/1/title", "/2/title"]);
}

#[test]
fn a_map_the_object_does_not_hold_an_entry_without_type_or_index_and_a_json_that_is_no_list_are_refused_at_their_pointer() {
    let mut e = cc_entry(1);
    e["ctrlMapId"] = json!("nope");
    let f = refused(&json!({"ChCtrlList": [cc_entry(0), e], "CtrlMapList": []}).to_string());
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/ChCtrlList/1/ctrlMapId"));

    let mut e = cc_entry(1);
    e.as_object_mut().unwrap().remove("ctrlType");
    let f = refused(&json!({"ChCtrlList": [e]}).to_string());
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/ChCtrlList/0/ctrlType"));

    let mut e = cc_entry(1);
    e.as_object_mut().unwrap().remove("ctrlIndex");
    let f = refused(&json!([cc_entry(0), e]).to_string());
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/1/ctrlIndex"));

    for text in ["{", "3", "\"list\"", "{\"Other\": []}"] {
        assert_eq!(refused(text).kind, FaultKind::InvalidFile, "{text}");
    }
}

#[test]
fn a_source_without_url_author_or_attribution_or_without_license_is_refused() {
    let text = json!([cc_entry(1)]).to_string();
    let mut s = source();
    s.license = None;
    assert_eq!(import(&text, Format::Midi2Controllers, &s).unwrap_err().kind, FaultKind::SourceIncomplete);
    let mut s = source();
    (s.url, s.author, s.attribution) = (None, None, None);
    assert_eq!(import(&text, Format::Midi2Controllers, &s).unwrap_err().kind, FaultKind::SourceIncomplete);
}

#[test]
fn the_import_reads_strictly_whatever_the_entries_carry() {
    let entries: Vec<Value> = (0..5)
        .map(|n| {
            let mut e = cc_entry(n);
            e["paramPath"] = json!(["/Filter/Cutoff", "/Odd/Thing", "/Amplifier/Envelope/AttackTime", "/Delay/DelayTime", "/Lfo/Rate"][n as usize]);
            e
        })
        .collect();
    // json_of reads the device strictly and panics when it is refused.
    let (d, _) = imported(&json!(entries).to_string());
    assert_eq!(controls(&d).len(), 5);
}

fn one(entry: Value) -> Value {
    let (d, _) = imported(&json!([entry]).to_string());
    controls(&d)[0].clone()
}

// M2-117-UM v1.0, table 12: channel is 1-256, counted across the Groups of a Function Block (16 per Group).
#[test]
fn the_channel_of_the_norm_is_read_back_to_osds_0_to_15() {
    for (norm, osd) in [(1, 0), (2, 1), (16, 15)] {
        let c = one(json!({"title": "A", "ctrlType": "cc", "ctrlIndex": [1], "channel": norm}));
        assert_eq!(c["channel"], osd, "channel {norm}");
    }
}

#[test]
fn a_channel_beyond_16_is_refused_at_its_pointer_with_the_group_it_falls_in() {
    for (channel, group) in [(17, 2), (32, 2), (33, 3), (256, 16)] {
        let f = refused(&json!([cc_entry(0), {"title": "A", "ctrlType": "cc", "ctrlIndex": [1], "channel": channel}]).to_string());
        assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/1/channel"), "channel {channel}");
        assert!(f.to_string().contains(&format!("Group {group}")), "channel {channel}: {f}");
    }
}

#[test]
fn a_channel_of_zero_or_below_is_refused_the_norm_counting_from_one() {
    for channel in [0, -1] {
        let f = refused(&json!([{"title": "A", "ctrlType": "cc", "ctrlIndex": [1], "channel": channel}]).to_string());
        assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/0/channel"), "channel {channel}");
    }
}

// M2-115-U v1.0.2 section 3.4, table 8: Min-Center-Max scales down by cutting off the low bits.
#[test]
fn a_7_bit_range_and_default_are_scaled_down_from_32_bits_by_cutting_the_low_bits() {
    let c = one(json!({"title": "A", "ctrlType": "cc", "ctrlIndex": [1], "numSigBits": 7, "minMax": [167772160u64, 4294967295u64], "default": 3238268993u64}));
    assert_eq!(c["range"], json!([5, 127]));
    assert_eq!(c["default"], 96);
}

#[test]
fn a_14_bit_nrpn_is_scaled_down_to_its_14_bits() {
    let c = one(json!({"title": "A", "ctrlType": "nrpn", "ctrlIndex": [1, 2], "numSigBits": 14, "minMax": [1310720u64, 4294967295u64], "default": 2147483648u64}));
    assert_eq!(c["range"], json!([5, 16383]));
    assert_eq!(c["default"], 8192);
    assert_eq!(c["value_bits"], 14);
}

#[test]
fn a_32_bit_range_and_default_are_read_as_they_are() {
    let c = one(json!({"title": "A", "ctrlType": "cc", "ctrlIndex": [20], "numSigBits": 32, "minMax": [0u64, 4294967295u64], "default": 3000000000u64}));
    assert_eq!(c["range"], json!([0u64, 4294967295u64]));
    assert_eq!(c["default"], 3000000000u64);
}

// M2-115-U v1.0.2 section 4.4, table 10: Zero-Extension scales down with rounding, clamped to the maximum.
#[test]
fn an_rpn_of_lsb_below_32_is_scaled_down_with_rounding_and_clamped() {
    let rpn = |lsb: i64, value: u64| {
        one(json!({"title": "A", "ctrlType": "rpn", "ctrlIndex": [0, lsb], "numSigBits": 7, "minMax": [0u64, 4261412864u64], "default": value}))["default"].clone()
    };
    assert_eq!(rpn(2, 0x15FF0000), 11, "rounded up");
    assert_eq!(rpn(2, 0x14000000), 10);
    assert_eq!(rpn(2, 0xFE000000), 127);
    assert_eq!(rpn(0, 0xFE000000), 127);
    // min-center-max cuts off: the same 32-bit value is 10, not 11
    assert_eq!(rpn(32, 0x15FF0000), 10);
    let c = one(json!({"title": "A", "ctrlType": "rpn", "ctrlIndex": [0, 2], "numSigBits": 7, "minMax": [0u64, 4294967295u64]}));
    assert_eq!(c["range"], json!([0, 127]), "the clamp");
}
