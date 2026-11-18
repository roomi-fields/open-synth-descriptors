//! An Electra One preset imports into an OSD device: its controls and commands, its look and
//! the kinds it cannot carry in the losses, its refusals.

use crate::common::electra::*;
use osd_formats::{import, FaultKind, Format};
use serde_json::{json, Value};

fn imported(p: &Value) -> (Value, Vec<osd_formats::Loss>) {
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("the preset imports");
    (json_of(&t.value), t.lost)
}

fn entries(d: &Value) -> Vec<Value> {
    d["cc_params"]["entries"].as_array().cloned().unwrap_or_default()
}

#[test]
fn the_format_is_named_electra_one_and_comes_after_midi_csv() {
    assert_eq!(Format::ALL, [Format::MidiCsv, Format::ElectraOne]);
    assert_eq!(Format::ElectraOne.name(), "electra-one");
    assert_eq!(Format::from_name("electra-one"), Some(Format::ElectraOne));
}

#[test]
fn a_cc7_control_becomes_a_control_with_its_name_its_group_as_section_and_its_range() {
    let p = preset(&[(1, "Filter")], vec![control(1, "Cutoff", Some(1), cc7(74), Some((10, 100)))]);
    let (d, _) = imported(&p);
    assert_eq!(d["$schema"], "https://w3id.org/osd/0.5/device.schema.json");
    let e = entries(&d);
    assert_eq!(e.len(), 1);
    assert_eq!(e[0]["cc"], 74);
    assert_eq!(e[0]["_name"], "Cutoff");
    assert_eq!(e[0]["section"], "Filter");
    assert_eq!(e[0]["range"], json!([10, 100]));
    assert!(e[0]["osc"].as_str().unwrap().starts_with('/'));
}

#[test]
fn a_control_with_no_group_has_no_section_and_one_with_no_range_has_none() {
    let p = preset(&[], vec![control(1, "Bare", None, cc7(7), None)]);
    let (d, _) = imported(&p);
    let e = entries(&d);
    assert!(e[0].get("section").is_none(), "{e:?}");
    assert!(e[0].get("range").is_none(), "{e:?}");
}

#[test]
fn the_controls_keep_the_order_of_the_preset_and_the_addresses_stay_distinct() {
    let p = preset(
        &[(1, "A"), (2, "B")],
        vec![
            control(1, "Same", Some(1), cc7(1), None),
            control(2, "Same", Some(1), cc7(2), None),
            control(3, "Other", Some(2), cc7(3), None),
        ],
    );
    let (d, _) = imported(&p);
    let e = entries(&d);
    assert_eq!(e.iter().map(|x| x["cc"].as_i64().unwrap()).collect::<Vec<_>>(), vec![1, 2, 3]);
    let mut osc: Vec<&str> = e.iter().map(|x| x["osc"].as_str().unwrap()).collect();
    osc.sort();
    osc.dedup();
    assert_eq!(osc.len(), 3);
}

#[test]
fn a_cc14_control_is_a_cc_with_its_lsb_32_above() {
    let (d, _) = imported(&preset(&[], vec![control(1, "Wide", None, cc14(5), Some((0, 16383)))]));
    let e = entries(&d);
    assert_eq!((e[0]["cc"].clone(), e[0]["cc_lsb"].clone()), (json!(5), json!(37)));
    assert_eq!(e[0]["range"], json!([0, 16383]));
}

#[test]
fn an_nrpn_control_splits_its_number_into_msb_and_lsb() {
    for (n, msb, lsb) in [(129u64, 1, 1), (0, 0, 0), (16383, 127, 127), (300, 2, 44)] {
        let (d, _) = imported(&preset(&[], vec![control(1, "N", None, nrpn(n), None)]));
        let e = entries(&d);
        assert_eq!((e[0]["nrpn_msb"].clone(), e[0]["nrpn_lsb"].clone()), (json!(msb), json!(lsb)), "{n}");
        assert!(e[0].get("cc").is_none());
    }
}

#[test]
fn an_rpn_control_splits_its_number_into_msb_and_lsb() {
    for (n, msb, lsb) in [(2u64, 0, 2), (130, 1, 2)] {
        let (d, _) = imported(&preset(&[], vec![control(1, "R", None, rpn(n), None)]));
        let e = entries(&d);
        assert_eq!((e[0]["rpn_msb"].clone(), e[0]["rpn_lsb"].clone()), (json!(msb), json!(lsb)), "{n}");
    }
}

#[test]
fn a_sysex_message_becomes_a_command_with_its_bytes_name_and_section() {
    let p = preset(
        &[(1, "Utility")],
        vec![
            control(1, "Dump request", Some(1), sysex(&[240, 126, 127, 6, 1, 247]), None),
            control(2, "Cutoff", Some(1), cc7(74), None),
            control(3, "Reset", None, sysex(&[240, 1, 247]), None),
        ],
    );
    let (d, lost) = imported(&p);
    let commands = d["commands"].as_array().unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0]["frame"], json!([240, 126, 127, 6, 1, 247]));
    assert_eq!(commands[0]["section"], "Utility");
    assert!(commands[0]["osc"].as_str().unwrap().starts_with('/'));
    assert_eq!(commands[1]["frame"], json!([240, 1, 247]));
    assert!(commands[1].get("section").is_none());
    assert_eq!(entries(&d).len(), 1);
    assert!(lost.iter().all(|l| !l.place.starts_with("/controls/0/values/0/message")), "{lost:?}");
}

#[test]
fn every_kind_the_importer_does_not_carry_is_a_loss_with_its_reason_and_no_control() {
    for kind in KINDS.iter().filter(|k| !CONTROL_KINDS.contains(k) && **k != COMMAND_KIND) {
        let p = preset(&[], vec![control(1, "Kept", None, cc7(1), None), control(2, "Lost", None, other(kind), None)]);
        let (d, lost) = imported(&p);
        assert_eq!(entries(&d).len(), 1, "{kind}");
        assert!(d.get("commands").and_then(|c| c.as_array()).is_none_or(|c| c.is_empty()), "{kind}");
        let loss = lost.iter().find(|l| l.place == "/controls/1/values/0/message").unwrap_or_else(|| panic!("{kind}: {lost:?}"));
        assert!(loss.reason.contains(kind), "{kind}: {}", loss.reason);
        assert!(loss.reason.contains("round trip"), "{kind}: {}", loss.reason);
    }
}

#[test]
fn the_look_of_the_preset_is_in_the_losses_and_nothing_is_invented_for_it() {
    let mut p = preset(&[(1, "Filter")], vec![control(1, "Cutoff", Some(1), cc7(74), None)]);
    p["pages"] = json!([{"id": 1, "name": "Page 1"}, {"id": 2, "name": "Page 2"}]);
    let t = import(&p.to_string(), Format::ElectraOne, &source()).unwrap();
    let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    for expected in ["/pages", "/controls/0/bounds", "/controls/0/color", "/controls/0/type", "/controls/0/pageId"] {
        assert!(places.contains(&expected), "{expected} not in {places:?}");
    }
    assert!(t.lost.iter().all(|l| !l.reason.is_empty()));
    let text = json_of(&t.value).to_string();
    for look in ["FF0000", "Page 1", "146"] {
        assert!(!text.contains(look), "{look} is invented in {text}");
    }
}

#[test]
fn the_descriptor_carries_the_credit_and_licence_given() {
    let (d, _) = imported(&preset(&[], vec![control(1, "C", None, cc7(1), None)]));
    assert_eq!(d["_sources"][0]["url"], "https://app.electra.one/preset/abc");
    assert_eq!(d["_sources"][0]["license"], "CC-BY-4.0");
}

#[test]
fn an_import_without_a_named_source_or_a_licence_is_refused() {
    let text = preset(&[], vec![control(1, "C", None, cc7(1), None)]).to_string();
    let mut no_license = source();
    no_license.license = None;
    assert_eq!(import(&text, Format::ElectraOne, &no_license).unwrap_err().kind, FaultKind::SourceIncomplete);
    let mut no_credit = source();
    (no_credit.url, no_credit.author, no_credit.attribution) = (None, None, None);
    assert_eq!(import(&text, Format::ElectraOne, &no_credit).unwrap_err().kind, FaultKind::SourceIncomplete);
}

#[test]
fn a_text_that_is_not_json_is_refused_as_an_invalid_file() {
    let f = import("{not json", Format::ElectraOne, &source()).unwrap_err();
    assert_eq!(f.kind, FaultKind::InvalidFile);
}

#[test]
fn a_preset_without_its_version_or_its_controls_is_refused_at_their_pointer() {
    let mut no_version = preset(&[], vec![]);
    no_version.as_object_mut().unwrap().remove("version");
    let f = import(&no_version.to_string(), Format::ElectraOne, &source()).unwrap_err();
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/version"));
    let mut no_controls = preset(&[], vec![]);
    no_controls.as_object_mut().unwrap().remove("controls");
    let f = import(&no_controls.to_string(), Format::ElectraOne, &source()).unwrap_err();
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/controls"));
}

#[test]
fn a_version_the_maker_does_not_document_is_refused_and_versions_2_and_3_are_read() {
    for v in [1, 4] {
        let mut p = preset(&[], vec![]);
        p["version"] = json!(v);
        let f = import(&p.to_string(), Format::ElectraOne, &source()).unwrap_err();
        assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/version"), "{v}");
    }
    let mut p = preset(&[], vec![control(1, "C", None, cc7(1), None)]);
    p["version"] = json!(2);
    assert!(import(&p.to_string(), Format::ElectraOne, &source()).is_ok());
}

#[test]
fn a_message_type_the_maker_does_not_document_is_refused_at_its_pointer() {
    let p = preset(&[], vec![control(1, "A", None, cc7(1), None), control(2, "B", None, other("teleport"), None)]);
    let f = import(&p.to_string(), Format::ElectraOne, &source()).unwrap_err();
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/controls/1/values/0/message/type"));
}

#[test]
fn a_parameter_number_outside_its_kind_is_refused_at_its_pointer() {
    for (message, name) in [(cc7(128), "cc7"), (nrpn(16384), "nrpn")] {
        let p = preset(&[], vec![control(1, "A", None, message, None)]);
        let f = import(&p.to_string(), Format::ElectraOne, &source()).unwrap_err();
        assert_eq!(f.kind, FaultKind::InvalidFile, "{name}");
        assert_eq!(f.place, "/controls/0/values/0/message/parameterNumber", "{name}");
    }
}
