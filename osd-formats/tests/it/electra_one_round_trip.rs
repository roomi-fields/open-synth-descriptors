//! A preset imported then exported keeps every binding of the carried kinds, and none of the
//! others.

use crate::common::electra::*;
use crate::common::resolved;
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

/// The messages of a preset's controls as (type, parameter number or bytes, min, max).
fn bindings(p: &Value) -> Vec<(Value, Value, Value, Value)> {
    p["controls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let v = &c["values"][0];
            let m = &v["message"];
            let what = if m["type"] == "sysex" { json!(m["data"]) } else { m["parameterNumber"].clone() };
            (m["type"].clone(), what, v.get("min").cloned().unwrap_or(Value::Null), v.get("max").cloned().unwrap_or(Value::Null))
        })
        .collect()
}

fn through(p: &Value) -> (Value, Vec<osd_formats::Loss>) {
    let imported = import(&p.to_string(), Format::ElectraOne, &source()).expect("imports");
    let t = export(&resolved(imported.value), Format::ElectraOne).expect("exports");
    (serde_json::from_str(&t.value).unwrap(), imported.lost)
}

#[test]
fn every_control_of_a_carried_kind_comes_back_with_the_same_message_and_range() {
    let p = preset(
        &[(1, "Filter"), (2, "Mod")],
        vec![
            control(1, "Cutoff", Some(1), cc7(74), Some((10, 100))),
            control(2, "Wide", Some(1), cc14(5), Some((0, 16383))),
            control(3, "Fine", Some(2), nrpn(300), None),
            control(4, "Bend range", Some(2), rpn(2), Some((0, 24))),
            control(5, "Bare", None, cc7(7), None),
        ],
    );
    let (back, _) = through(&p);
    assert_eq!(bindings(&back), bindings(&p));
    assert_eq!(violations(&back), Vec::<String>::new());
}

#[test]
fn the_names_and_sections_come_back_in_the_same_order() {
    let p = preset(&[(1, "Filter")], vec![control(1, "Cutoff", Some(1), cc7(74), None), control(2, "Bare", None, cc7(7), None)]);
    let (back, _) = through(&p);
    let names: Vec<&str> = back["controls"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Cutoff", "Bare"]);
    assert_eq!(back["groups"][0]["name"], "Filter");
    assert!(back["controls"][1].get("groupId").is_none());
}

#[test]
fn a_sysex_message_comes_back_with_the_same_bytes() {
    let p = preset(&[], vec![control(1, "Dump", None, sysex(&[240, 126, 127, 6, 1, 247]), None)]);
    let (back, _) = through(&p);
    assert_eq!(bindings(&back), bindings(&p));
}

#[test]
fn a_message_kind_lost_on_import_is_not_in_the_export_and_its_loss_says_it_is_lost_in_the_round_trip() {
    for kind in KINDS.iter().filter(|k| !CONTROL_KINDS.contains(k) && **k != COMMAND_KIND) {
        let p = preset(&[], vec![control(1, "Kept", None, cc7(1), None), control(2, "Lost", None, other(kind), None)]);
        let (back, lost) = through(&p);
        let controls = back["controls"].as_array().unwrap();
        assert_eq!(controls.len(), 1, "{kind}");
        assert!(controls.iter().all(|c| c["values"][0]["message"]["type"] != json!(kind)), "{kind}");
        assert!(lost.iter().any(|l| l.reason.contains("round trip")), "{kind}: {lost:?}");
    }
}
