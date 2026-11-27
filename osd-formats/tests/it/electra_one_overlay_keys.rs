//! The Electra One import names every key of an overlay it reads, and of an item it carries, that
//! the descriptor holds no field for (osd-formats/docs/CADRE.md R8): once each, whatever the number
//! of controls that read the overlay.

use crate::common::electra::*;
use osd_formats::{import, Format, Loss};
use serde_json::{json, Value};

fn imported(p: &Value) -> (Value, Vec<Loss>) {
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("the preset imports");
    (json_of(&t.value), t.lost)
}

/// A preset of one control per `(message, overlay)`, named `C0`, `C1`…, the control referring to
/// the overlay of that index when it names one.
fn preset_reading(controls: Vec<(Value, Option<usize>)>, overlays: Vec<Value>) -> Value {
    let of: Vec<Option<usize>> = controls.iter().map(|(_, o)| *o).collect();
    let controls = controls.into_iter().enumerate().map(|(n, (m, _))| control(n as u64 + 1, &format!("C{n}"), None, m, None)).collect();
    let mut p = preset(&[], controls);
    p["overlays"] = overlays
        .into_iter()
        .enumerate()
        .map(|(k, mut o)| {
            o["id"] = json!(10 + k);
            o
        })
        .collect();
    for (n, o) in of.iter().enumerate() {
        if let Some(k) = o {
            p["controls"][n]["values"][0]["overlayId"] = json!(10 + k);
        }
    }
    p
}

/// The places of the losses under `/overlays`, sorted, each as many times as it is named.
fn overlay_places(lost: &[Loss]) -> Vec<&str> {
    let mut out: Vec<&str> = lost.iter().map(|l| l.place.as_str()).filter(|p| p.starts_with("/overlays")).collect();
    out.sort();
    out
}

fn sorted(mut places: Vec<&str>) -> Vec<&str> {
    places.sort();
    places
}

#[test]
fn a_key_of_a_carried_item_other_than_value_and_label_is_named_at_its_pointer_with_a_reason() {
    let overlay = json!({"items": [
        {"value": 0, "label": "Off", "color": "FF0000"},
        {"value": 1, "label": "On"},
        {"value": 2, "label": "Auto", "color": "00FF00", "bitmap": "AAAA"},
    ]});
    for message in [cc7(1), cc14(1), nrpn(1), rpn(1), json!({"type": "relcc", "deviceId": 1, "parameterNumber": 1, "relative": "twosComplement"})] {
        let (_, lost) = imported(&preset_reading(vec![(message.clone(), Some(0))], vec![overlay.clone()]));
        assert_eq!(
            overlay_places(&lost),
            sorted(vec!["/overlays/0/items/0/color", "/overlays/0/items/2/bitmap", "/overlays/0/items/2/color"]),
            "{message}"
        );
        assert!(lost.iter().all(|l| !l.reason.is_empty()), "{lost:?}");
    }
}

#[test]
fn a_key_of_a_read_overlay_other_than_id_and_items_is_named_at_its_pointer_with_a_reason() {
    let plain = json!({"items": [{"value": 0, "label": "Off"}]});
    let named = json!({"name": "Switch", "color": null, "items": [{"value": 0, "label": "Off"}]});
    let (_, lost) = imported(&preset_reading(vec![(cc7(1), Some(0)), (cc7(2), Some(1))], vec![plain, named]));
    assert_eq!(overlay_places(&lost), sorted(vec!["/overlays/1/color", "/overlays/1/name"]));
    assert!(lost.iter().all(|l| !l.reason.is_empty()), "{lost:?}");
}

#[test]
fn a_key_of_an_overlay_or_of_an_item_is_escaped_as_a_segment_of_a_json_pointer() {
    let overlay = json!({"a/b": 1, "c~d": 2, "items": [{"value": 0, "label": "Off", "e/f~g": 3}]});
    let (_, lost) = imported(&preset_reading(vec![(cc7(1), Some(0))], vec![overlay]));
    assert_eq!(overlay_places(&lost), sorted(vec!["/overlays/0/a~1b", "/overlays/0/c~0d", "/overlays/0/items/0/e~1f~0g"]));
}

#[test]
fn a_key_of_an_overlay_several_controls_read_is_named_once() {
    let overlay = json!({"name": "Shared", "items": [{"value": 0, "label": "Off", "color": "FF0000"}, {"value": 200, "label": "Far", "color": "00FF00"}]});
    // The first control carries the first item alone, the two others both.
    let p = preset_reading(vec![(cc7(1), Some(0)), (nrpn(1), Some(0)), (nrpn(2), Some(0))], vec![overlay]);
    let (_, lost) = imported(&p);
    assert_eq!(
        overlay_places(&lost),
        sorted(vec!["/overlays/0/items/0/color", "/overlays/0/items/1", "/overlays/0/items/1/color", "/overlays/0/name"])
    );
}

#[test]
fn an_item_no_control_carries_is_named_whole_and_none_of_its_keys() {
    let overlay = json!({"items": [
        {"value": 0, "label": "Off", "color": "FF0000"},
        {"value": 200, "label": "Far", "color": "00FF00"},
        {"value": 1, "color": "0000FF"},
        {"label": "no value", "color": "0000FF"},
    ]});
    let (_, lost) = imported(&preset_reading(vec![(cc7(1), Some(0))], vec![overlay]));
    assert_eq!(
        overlay_places(&lost),
        sorted(vec!["/overlays/0/items/0/color", "/overlays/0/items/1", "/overlays/0/items/2", "/overlays/0/items/3"])
    );
}

#[test]
fn the_keys_of_a_read_overlay_are_named_when_no_item_of_it_is_carried() {
    let outside = json!({"name": "Far", "items": [{"value": 200, "label": "Far", "color": "00FF00"}]});
    let no_list = json!({"name": "Text", "items": "Off, On"});
    let no_items = json!({"name": "Bare"});
    let empty = json!({"name": "Empty", "items": []});
    let p = preset_reading(
        vec![(cc7(1), Some(0)), (cc7(2), Some(1)), (cc7(3), Some(2)), (cc7(4), Some(3)), (cc7(5), Some(1))],
        vec![outside, no_list, no_items, empty],
    );
    let (_, lost) = imported(&p);
    let places: Vec<&str> = overlay_places(&lost).into_iter().filter(|p| *p != "/overlays/1/items").collect();
    assert_eq!(
        places,
        sorted(vec!["/overlays/0/items/0", "/overlays/0/name", "/overlays/1/name", "/overlays/2/name", "/overlays/3/name"])
    );
    assert!(overlay_places(&lost).contains(&"/overlays/1/items"), "{lost:?}");
}

#[test]
fn an_overlay_no_written_control_reads_is_named_whole_and_none_of_its_keys() {
    let overlay = || json!({"name": "Unread", "items": [{"value": 0, "label": "Off", "color": "FF0000"}]});
    // 0: read by no control; 1: by a control left out for its message, claimed; 2: by a command.
    let p = preset_reading(
        vec![(cc7(1), None), (cc7(1), Some(1)), (sysex(&[240, 1, 247]), Some(2))],
        vec![overlay(), overlay(), overlay()],
    );
    let (_, lost) = imported(&p);
    assert_eq!(overlay_places(&lost), ["/overlays/0", "/overlays/1", "/overlays/2"]);
}

#[test]
fn the_keys_named_lost_change_neither_the_values_carried_nor_the_descriptor() {
    let bare = json!({"items": [{"value": 0, "label": "Off"}, {"value": 1, "label": "On"}]});
    let dressed = json!({"name": "Switch", "items": [{"value": 0, "label": "Off", "color": "FF0000"}, {"value": 1, "label": "On", "bitmap": "AA"}]});
    let of = |overlay: Value| imported(&preset_reading(vec![(cc7(1), Some(0))], vec![overlay])).0;
    let (mut before, mut after) = (of(bare), of(dressed));
    assert_eq!(after["cc_params"]["entries"][0]["values"], json!([{"value": 0, "title": "Off"}, {"value": 1, "title": "On"}]));
    // The two texts differ, so their hashes do.
    for d in [&mut before, &mut after] {
        d["sources"][0].as_object_mut().unwrap().remove("sha256");
    }
    assert_eq!(after, before);
}
