//! The table of the attributes of a preset in docs/electra-one.md: one row per attribute of the
//! maker's page, its fate (`carried`, `lost`, `conditional`, or `structure` for what the import follows
//! and does not name), its reason and the section of the page; each row checked on presets that
//! hold the attribute (osd-formats/docs/CADRE.md R8, docs/CADRE.md R9).

use crate::common::electra::*;
use crate::common::fields::{held, page, row_of, table};
use osd_formats::{import, Format};
use serde_json::{json, Value};

const HEADER: &str = "Preset attribute";

/// A preset that holds every attribute, each where the import reads it.
fn full() -> Value {
    let mut p = json!({
        "version": 3, "name": "A preset", "projectId": "p1",
        "pages": [{"id": 1, "name": "Page 1"}],
        "devices": [{"id": 1, "name": "Synth", "port": 1, "channel": 1, "interfaces": [], "runningStatus": false, "rate": 20, "patch": [],
            "messages": [{"id": 5, "direction": "out", "data": [67, {"type": "value"}]}]}],
        "overlays": [{"id": 1, "items": [{"value": 10, "label": "Low", "bitmap": "AAAA", "color": "FF0000"}, {"value": 100, "label": "High"}]}],
        "groups": [{"id": 11, "pageId": 1, "name": "Filter", "bounds": [0, 0, 300, 100], "color": "FF0000", "variant": "default",
            "font": "default", "values": []}],
        "controls": []
    });
    p["controls"] = controls();
    p
}

/// The controls of that preset.
fn controls() -> Value {
    json!([cutoff(),
        {"id": 102, "type": "pad", "name": "Switch", "values": [{"id": "value",
             "message": {"deviceId": 1, "type": "cc7", "parameterNumber": 75, "offValue": 0, "onValue": 1}}]},
        {"id": 103, "type": "pad", "name": "By id", "values": [{"id": "value",
             "message": {"deviceId": 1, "type": "sysex", "direction": "out", "data": 5}}]},
        {"id": 104, "type": "fader", "name": "Inline", "values": [{"id": "value",
             "message": {"deviceId": 1, "type": "sysex", "data": [67,
                 {"type": "value", "rules": [{"parameterBitPosition": 0, "byteBitPosition": 0, "bitWidth": 7}]},
                 {"type": "checksum", "algorithm": "roland", "start": 1, "length": 2}]}}]}
    ])
}

/// The first control, which holds every attribute of a control, of its value and of its message.
fn cutoff() -> Value {
    let mut c = json!({"id": 101, "type": "fader", "name": "Cutoff", "color": "FF0000", "variant": "dial", "mode": "default", "font": "default",
        "bounds": [0, 0, 146, 56], "pageId": 1, "controlSetId": 1, "groupId": 11, "visible": true,
        "inputs": [{"potId": 1, "valueId": "value"}], "events": [], "valueRefs": []});
    let mut value = json!({"id": "value", "name": "v", "min": 10, "max": 100, "defaultValue": 64, "placeholder": "x", "sensitivity": 1,
        "formatter": "f", "function": "g", "overlayId": 1});
    value["message"] = json!({"deviceId": 1, "type": "cc7", "id": "m", "parameterNumber": 74, "min": 10, "max": 100, "value": 1,
        "lsbFirst": false, "resetRpn": false, "signMode": "twosComplement", "bitWidth": 7, "relative": false,
        "relativeMode": "twosComplement", "accelerated": false});
    c["values"] = json!([value]);
    c
}

const FIRST: &str = "/controls/0/values/0";
const MESSAGE: &str = "/controls/0/values/0/message";
const PAD: &str = "/controls/1/values/0/message";
const BY_ID: &str = "/controls/2/values/0/message";
const INLINE: &str = "/controls/3/values/0/message";

/// The full preset changed: each pointer set to its value, or removed for `null`.
fn changed(edits: &[(String, Value)]) -> Value {
    let mut p = full();
    for (pointer, value) in edits {
        if value.is_null() {
            crate::common::fields::remove(&mut p, pointer);
        } else {
            crate::common::fields::set(&mut p, pointer, value.clone());
        }
    }
    p
}

/// What is shown of an attribute: the fates its row may say, the presets that hold it with its
/// pointer, and a change of it that changes the device imported (for a carried one).
struct Shown {
    fates: &'static [&'static str],
    held_by: Vec<(Value, String)>,
    witness: Option<(String, Value)>,
}

fn at(pointer: impl Into<String>) -> (Value, String) {
    (full(), pointer.into())
}

fn variant(pointer: impl Into<String>, edits: &[(&str, Value)]) -> (Value, String) {
    let edits: Vec<(String, Value)> = edits.iter().map(|(p, v)| (p.to_string(), v.clone())).collect();
    (changed(&edits), pointer.into())
}

fn shown(fates: &'static [&'static str], held_by: Vec<(Value, String)>) -> Shown {
    Shown { fates, held_by, witness: None }
}

fn witnessed(fates: &'static [&'static str], held_by: Vec<(Value, String)>, pointer: impl Into<String>, value: Value) -> Shown {
    Shown { fates, held_by, witness: Some((pointer.into(), value)) }
}

const LOST: &[&str] = &["lost"];
const MAY_LOSE: &[&str] = &["lost", "conditional"];
const CARRIED: &[&str] = &["carried", "conditional"];
const CONDITIONAL: &[&str] = &["conditional"];
const STRUCTURE: &[&str] = &["structure"];

/// A rule of a `value` element of the template of the inline message.
fn rule(key: &str, lost: Value) -> Shown {
    let pointer = format!("{INLINE}/data/1/rules/0/{key}");
    let held_by = vec![at(pointer.clone()), variant(pointer.clone(), &[(&pointer, lost.clone())])];
    witnessed(CARRIED, held_by, pointer, lost)
}

/// An element of this form added to the template of the inline message: it is lost.
fn element(element: Value, key: &str) -> Shown {
    let place = format!("{INLINE}/data/1");
    let pointer = if key.is_empty() { place.clone() } else { format!("{place}/{key}") };
    shown(LOST, vec![variant(pointer, &[(&place, element)])])
}

fn checksum(key: &str, refused: Value) -> Shown {
    let pointer = format!("{INLINE}/data/2/{key}");
    shown(CONDITIONAL, vec![at(pointer.clone()), variant(pointer.clone(), &[(&pointer, refused)])])
}

fn of(attribute: &str) -> Shown {
    let note = json!({"deviceId": 1, "type": "note", "parameterNumber": 60});
    let (last, parent) = match attribute.rsplit_once('.') {
        Some((parent, last)) => (last, parent),
        None => (attribute, ""),
    };
    let plain = |pointer: String| shown(LOST, vec![at(pointer)]);
    match attribute {
        "version" => shown(STRUCTURE, vec![at("/version")]),
        "name" => witnessed(CARRIED, vec![at("/name"), variant("/name", &[("/name", json!(""))])], "/name", json!("Another")),
        "projectId" => plain("/projectId".into()),
        "pages" => plain("/pages".into()),
        "devices" => witnessed(CARRIED, vec![at("/devices"), variant("/devices", &[("/controls/2", json!({"id": 103, "type": "pad", "name": "None"}))])],
            "/devices/0/messages/0/data", json!([67, 9])),
        "overlays" => witnessed(CARRIED, vec![at("/overlays")], "/overlays/0/items/0/label", json!("Other")),
        "groups" => witnessed(CARRIED, vec![at("/groups")], "/groups/0/name", json!("Other")),
        "controls" => witnessed(CARRIED, vec![at("/controls")], "/controls/0/name", json!("Other")),
        "devices[].id" => shown(MAY_LOSE, vec![at("/devices/0/id")]),
        "devices[].messages" => witnessed(CARRIED, vec![at("/devices/0/messages")], "/devices/0/messages/0/data", json!([67, 9])),
        "devices[].messages[].id" => shown(STRUCTURE, vec![at("/devices/0/messages/0/id")]),
        "devices[].messages[].direction" => witnessed(&["carried", "lost", "conditional"],
            vec![at("/devices/0/messages/0/direction")], "/devices/0/messages/0/direction", json!("in")),
        "devices[].messages[].data" => witnessed(CARRIED, vec![at("/devices/0/messages/0/data")], "/devices/0/messages/0/data", json!([67, 9])),
        "overlays[].id" => shown(STRUCTURE, vec![at("/overlays/0/id")]),
        "overlays[].items" => witnessed(CARRIED, vec![at("/overlays/0/items")], "/overlays/0/items/0/label", json!("Other")),
        "overlays[].items[].value" => witnessed(CARRIED,
            vec![at("/overlays/0/items/0/value"), variant("/overlays/0/items/0/value", &[("/overlays/0/items/0/value", json!(500))])],
            "/overlays/0/items/0/value", json!(11)),
        "overlays[].items[].label" => witnessed(CARRIED,
            vec![at("/overlays/0/items/0/label"), variant("/overlays/0/items/0/label", &[("/overlays/0/items/0/value", json!(500))])],
            "/overlays/0/items/0/label", json!("Other")),
        "groups[].id" => shown(STRUCTURE, vec![at("/groups/0/id")]),
        "groups[].name" => shown(CONDITIONAL, vec![
            at("/groups/0/name"),
            variant("/groups/1/name", &[("/groups/1", json!({"id": 12, "pageId": 1, "name": "Nobody"}))]),
        ]),
        "controls[].id" => plain("/controls/0/id".into()),
        "controls[].name" => witnessed(CARRIED, vec![at("/controls/0/name"), variant("/controls/0/name", &[(MESSAGE, note.clone())])],
            "/controls/0/name", json!("Other")),
        "controls[].groupId" => shown(STRUCTURE, vec![at("/controls/0/groupId")]),
        "controls[].values" => witnessed(CARRIED, vec![at("/controls/0/values")], "/controls/0/values/0/message/parameterNumber", json!(7)),
        "controls[].values[].id" => plain(format!("{FIRST}/id")),
        "controls[].values[].min" | "controls[].values[].max" => shown(CONDITIONAL, vec![
            at(format!("{FIRST}/{last}")),
            variant(format!("{FIRST}/{last}"), &[(&format!("{FIRST}/{last}"), json!(55))]),
        ]),
        "controls[].values[].overlayId" => shown(STRUCTURE, vec![at(format!("{FIRST}/overlayId"))]),
        "controls[].values[].message" => witnessed(CARRIED, vec![at(MESSAGE), variant(MESSAGE, &[(MESSAGE, note.clone())])],
            format!("{MESSAGE}/parameterNumber"), json!(7)),
        "controls[].values[].message.deviceId" => shown(MAY_LOSE, vec![at(format!("{MESSAGE}/deviceId")), at(format!("{INLINE}/deviceId"))]),
        "controls[].values[].message.type" => witnessed(CARRIED,
            vec![at(format!("{MESSAGE}/type")), variant(format!("{MESSAGE}/type"), &[(MESSAGE, note.clone())])],
            format!("{MESSAGE}/type"), json!("nrpn")),
        "controls[].values[].message.parameterNumber" => witnessed(CARRIED,
            vec![at(format!("{MESSAGE}/parameterNumber")), variant(format!("{MESSAGE}/parameterNumber"), &[(MESSAGE, note.clone())])],
            format!("{MESSAGE}/parameterNumber"), json!(7)),
        "controls[].values[].message.min" | "controls[].values[].message.max" => shown(CONDITIONAL, vec![
            at(format!("{MESSAGE}/{last}")),
            // A lone bound.
            variant(format!("{MESSAGE}/{last}"), &[
                (FIRST, json!({"id": "value", "message": {"deviceId": 1, "type": "cc7", "parameterNumber": 74, last: 20}})),
            ]),
        ]),
        "controls[].values[].message.onValue" | "controls[].values[].message.offValue" => shown(CONDITIONAL, vec![
            at(format!("{PAD}/{last}")),
            variant(format!("{PAD}/{last}"), &[(PAD, json!({"deviceId": 1, "type": "cc7", "parameterNumber": 75, last: 20}))]),
        ]),
        "controls[].values[].message.relative" => witnessed(CARRIED,
            vec![at(format!("{MESSAGE}/relative")), variant(format!("{MESSAGE}/relative"), &[(&format!("{MESSAGE}/relative"), json!(true))])],
            format!("{MESSAGE}/relative"), json!(true)),
        "controls[].values[].message.relativeMode" => plain(format!("{MESSAGE}/relativeMode")),
        "controls[].values[].message.data" => witnessed(CARRIED, vec![
            at(format!("{BY_ID}/data")),
            at(format!("{INLINE}/data")),
            variant(format!("{INLINE}/data"), &[(&format!("{INLINE}/data"), json!([67, {"type": "parameter"}]))]),
        ], format!("{INLINE}/data"), json!([67, 9])),
        "controls[].values[].message.direction" => witnessed(&["carried", "lost", "conditional"],
            vec![at(format!("{BY_ID}/direction"))], format!("{BY_ID}/direction"), json!("in")),
        "template.value.rules[].type" => shown(&["carried", "lost", "conditional"],
            vec![variant(format!("{INLINE}/data/1/rules/0/type"), &[(&format!("{INLINE}/data/1/rules/0/type"), json!("cc7"))])]),
        "template.value.rules[].parameterNumber" => shown(MAY_LOSE,
            vec![variant(format!("{INLINE}/data/1/rules/0/parameterNumber"), &[(&format!("{INLINE}/data/1/rules/0/parameterNumber"), json!(12))])]),
        "template.value.rules[].parameterBitPosition" => rule("parameterBitPosition", json!(14)),
        "template.value.rules[].byteBitPosition" => rule("byteBitPosition", json!(4)),
        "template.value.rules[].bitWidth" => rule("bitWidth", json!(3)),
        "template.parameter.parameterBitPosition" | "template.parameter.byteBitPosition" | "template.parameter.bitWidth" => {
            element(json!({"type": "parameter", last: 0}), last)
        }
        "template.checksum.algorithm" => checksum("algorithm", json!("yamaha")),
        "template.checksum.start" => checksum("start", json!("abc")),
        "template.checksum.length" => checksum("length", json!(0)),
        "template.function.name" => element(json!({"type": "function", "name": "pickByte"}), "name"),
        "template.any" => element(json!({"type": "any"}), ""),
        _ => match parent {
            "devices[]" => plain(format!("/devices/0/{last}")),
            "overlays[].items[]" => plain(format!("/overlays/0/items/0/{last}")),
            "groups[]" => plain(format!("/groups/0/{last}")),
            "controls[]" => plain(format!("/controls/0/{last}")),
            "controls[].values[]" => plain(format!("{FIRST}/{last}")),
            // What else a message holds says how the controller encodes or sends it.
            "controls[].values[].message" => shown(MAY_LOSE, vec![at(format!("{MESSAGE}/{last}"))]),
            _ => panic!("no case for the attribute `{attribute}`"),
        },
    }
}

/// What the import of a preset gives but for its source, which holds the hash of the text.
fn device_of(p: &Value) -> Value {
    let t = import(&p.to_string(), Format::ElectraOne, &source()).unwrap_or_else(|f| panic!("{f}\n{p}"));
    let mut d = json_of(&t.value);
    d.as_object_mut().unwrap().remove("sources");
    d
}

fn check(attribute: &str) {
    let shown = of(attribute);
    let row = row_of("electra-one", HEADER, attribute);
    let fate = row.get(1).map(String::as_str).unwrap_or_default();
    assert!(shown.fates.contains(&fate), "docs/electra-one.md, `{attribute}`: the fate is one of {:?}, not {fate:?}", shown.fates);
    assert!(row.get(2).is_some_and(|r| !r.is_empty()), "docs/electra-one.md, `{attribute}`: no reason");
    assert!(row.get(3).is_some_and(|r| !r.is_empty()), "docs/electra-one.md, `{attribute}`: no section of the page");
    let named: Vec<bool> = shown
        .held_by
        .iter()
        .map(|(p, pointer)| {
            let t = import(&p.to_string(), Format::ElectraOne, &source()).unwrap_or_else(|f| panic!("{attribute}: {f}\n{p}"));
            held(&t.lost, pointer)
        })
        .collect();
    let places: Vec<&String> = shown.held_by.iter().map(|(_, p)| p).collect();
    match fate {
        "lost" => assert!(named.iter().all(|n| *n), "{attribute} is said lost and is not in the losses: {places:?} {named:?}"),
        "conditional" => assert!(
            named.contains(&true) && named.contains(&false),
            "{attribute} is said conditional and does not show both cases: {places:?} {named:?}"
        ),
        _ => assert!(named.iter().all(|n| !*n), "{attribute} is said {fate} and is in the losses: {places:?} {named:?}"),
    }
    if fate == "carried" {
        let (pointer, value) = shown.witness.unwrap_or_else(|| panic!("{attribute}: nothing shows it carried"));
        assert_ne!(device_of(&full()), device_of(&changed(&[(pointer.clone(), value)])), "{attribute}: changing {pointer} changes nothing");
    }
}

/// The page states what the list is copied from, and its table has a row for each attribute of the
/// list, each with its test and presets that import.
#[test]
fn every_attribute_the_maker_documents_has_its_row() {
    assert_eq!(TESTED, ATTRIBUTES, "an attribute without its test");
    for attribute in ATTRIBUTES {
        for (p, _) in of(attribute).held_by {
            device_of(&p);
        }
    }
    let text = page("electra-one");
    assert!(text.contains(PAGE_ADDRESS) && text.contains(PAGE_VERSION), "docs/electra-one.md states {PAGE_ADDRESS} {PAGE_VERSION}");
    let rows: Vec<String> = table("electra-one", HEADER).into_iter().map(|r| r[0].clone()).collect();
    let missing: Vec<&&str> = ATTRIBUTES.iter().filter(|a| !rows.iter().any(|r| r == **a)).collect();
    assert!(missing.is_empty(), "docs/electra-one.md: no row for {missing:?}");
}

macro_rules! rows {
    ($($test:ident => $attribute:literal,)+) => {
        const TESTED: &[&str] = &[$($attribute),+];

        $(
            #[test]
            fn $test() {
                check($attribute);
            }
        )+
    };
}

rows!(
    the_row_of_version_says_what_the_import_does => "version",
    the_row_of_name_says_what_the_import_does => "name",
    the_row_of_project_id_says_what_the_import_does => "projectId",
    the_row_of_pages_says_what_the_import_does => "pages",
    the_row_of_devices_says_what_the_import_does => "devices",
    the_row_of_overlays_says_what_the_import_does => "overlays",
    the_row_of_groups_says_what_the_import_does => "groups",
    the_row_of_controls_says_what_the_import_does => "controls",
    the_row_of_devices_id_says_what_the_import_does => "devices[].id",
    the_row_of_devices_name_says_what_the_import_does => "devices[].name",
    the_row_of_devices_port_says_what_the_import_does => "devices[].port",
    the_row_of_devices_channel_says_what_the_import_does => "devices[].channel",
    the_row_of_devices_interfaces_says_what_the_import_does => "devices[].interfaces",
    the_row_of_devices_running_status_says_what_the_import_does => "devices[].runningStatus",
    the_row_of_devices_rate_says_what_the_import_does => "devices[].rate",
    the_row_of_devices_patch_says_what_the_import_does => "devices[].patch",
    the_row_of_devices_messages_says_what_the_import_does => "devices[].messages",
    the_row_of_devices_messages_id_says_what_the_import_does => "devices[].messages[].id",
    the_row_of_devices_messages_direction_says_what_the_import_does => "devices[].messages[].direction",
    the_row_of_devices_messages_data_says_what_the_import_does => "devices[].messages[].data",
    the_row_of_overlays_id_says_what_the_import_does => "overlays[].id",
    the_row_of_overlays_items_says_what_the_import_does => "overlays[].items",
    the_row_of_overlays_items_value_says_what_the_import_does => "overlays[].items[].value",
    the_row_of_overlays_items_label_says_what_the_import_does => "overlays[].items[].label",
    the_row_of_overlays_items_bitmap_says_what_the_import_does => "overlays[].items[].bitmap",
    the_row_of_overlays_items_color_says_what_the_import_does => "overlays[].items[].color",
    the_row_of_groups_id_says_what_the_import_does => "groups[].id",
    the_row_of_groups_page_id_says_what_the_import_does => "groups[].pageId",
    the_row_of_groups_name_says_what_the_import_does => "groups[].name",
    the_row_of_groups_bounds_says_what_the_import_does => "groups[].bounds",
    the_row_of_groups_color_says_what_the_import_does => "groups[].color",
    the_row_of_groups_variant_says_what_the_import_does => "groups[].variant",
    the_row_of_groups_font_says_what_the_import_does => "groups[].font",
    the_row_of_groups_values_says_what_the_import_does => "groups[].values",
    the_row_of_controls_id_says_what_the_import_does => "controls[].id",
    the_row_of_controls_type_says_what_the_import_does => "controls[].type",
    the_row_of_controls_name_says_what_the_import_does => "controls[].name",
    the_row_of_controls_color_says_what_the_import_does => "controls[].color",
    the_row_of_controls_variant_says_what_the_import_does => "controls[].variant",
    the_row_of_controls_mode_says_what_the_import_does => "controls[].mode",
    the_row_of_controls_font_says_what_the_import_does => "controls[].font",
    the_row_of_controls_bounds_says_what_the_import_does => "controls[].bounds",
    the_row_of_controls_page_id_says_what_the_import_does => "controls[].pageId",
    the_row_of_controls_control_set_id_says_what_the_import_does => "controls[].controlSetId",
    the_row_of_controls_group_id_says_what_the_import_does => "controls[].groupId",
    the_row_of_controls_visible_says_what_the_import_does => "controls[].visible",
    the_row_of_controls_inputs_says_what_the_import_does => "controls[].inputs",
    the_row_of_controls_values_says_what_the_import_does => "controls[].values",
    the_row_of_controls_events_says_what_the_import_does => "controls[].events",
    the_row_of_controls_value_refs_says_what_the_import_does => "controls[].valueRefs",
    the_row_of_controls_values_id_says_what_the_import_does => "controls[].values[].id",
    the_row_of_controls_values_name_says_what_the_import_does => "controls[].values[].name",
    the_row_of_controls_values_min_says_what_the_import_does => "controls[].values[].min",
    the_row_of_controls_values_max_says_what_the_import_does => "controls[].values[].max",
    the_row_of_controls_values_default_value_says_what_the_import_does => "controls[].values[].defaultValue",
    the_row_of_controls_values_placeholder_says_what_the_import_does => "controls[].values[].placeholder",
    the_row_of_controls_values_sensitivity_says_what_the_import_does => "controls[].values[].sensitivity",
    the_row_of_controls_values_formatter_says_what_the_import_does => "controls[].values[].formatter",
    the_row_of_controls_values_function_says_what_the_import_does => "controls[].values[].function",
    the_row_of_controls_values_overlay_id_says_what_the_import_does => "controls[].values[].overlayId",
    the_row_of_controls_values_message_says_what_the_import_does => "controls[].values[].message",
    the_row_of_controls_values_message_device_id_says_what_the_import_does => "controls[].values[].message.deviceId",
    the_row_of_controls_values_message_type_says_what_the_import_does => "controls[].values[].message.type",
    the_row_of_controls_values_message_id_says_what_the_import_does => "controls[].values[].message.id",
    the_row_of_controls_values_message_parameter_number_says_what_the_import_does => "controls[].values[].message.parameterNumber",
    the_row_of_controls_values_message_min_says_what_the_import_does => "controls[].values[].message.min",
    the_row_of_controls_values_message_max_says_what_the_import_does => "controls[].values[].message.max",
    the_row_of_controls_values_message_on_value_says_what_the_import_does => "controls[].values[].message.onValue",
    the_row_of_controls_values_message_off_value_says_what_the_import_does => "controls[].values[].message.offValue",
    the_row_of_controls_values_message_value_says_what_the_import_does => "controls[].values[].message.value",
    the_row_of_controls_values_message_lsb_first_says_what_the_import_does => "controls[].values[].message.lsbFirst",
    the_row_of_controls_values_message_reset_rpn_says_what_the_import_does => "controls[].values[].message.resetRpn",
    the_row_of_controls_values_message_sign_mode_says_what_the_import_does => "controls[].values[].message.signMode",
    the_row_of_controls_values_message_bit_width_says_what_the_import_does => "controls[].values[].message.bitWidth",
    the_row_of_controls_values_message_relative_says_what_the_import_does => "controls[].values[].message.relative",
    the_row_of_controls_values_message_relative_mode_says_what_the_import_does => "controls[].values[].message.relativeMode",
    the_row_of_controls_values_message_accelerated_says_what_the_import_does => "controls[].values[].message.accelerated",
    the_row_of_controls_values_message_data_says_what_the_import_does => "controls[].values[].message.data",
    the_row_of_controls_values_message_direction_says_what_the_import_does => "controls[].values[].message.direction",
    the_row_of_template_value_rules_type_says_what_the_import_does => "template.value.rules[].type",
    the_row_of_template_value_rules_parameter_number_says_what_the_import_does => "template.value.rules[].parameterNumber",
    the_row_of_template_value_rules_parameter_bit_position_says_what_the_import_does => "template.value.rules[].parameterBitPosition",
    the_row_of_template_value_rules_byte_bit_position_says_what_the_import_does => "template.value.rules[].byteBitPosition",
    the_row_of_template_value_rules_bit_width_says_what_the_import_does => "template.value.rules[].bitWidth",
    the_row_of_template_parameter_parameter_bit_position_says_what_the_import_does => "template.parameter.parameterBitPosition",
    the_row_of_template_parameter_byte_bit_position_says_what_the_import_does => "template.parameter.byteBitPosition",
    the_row_of_template_parameter_bit_width_says_what_the_import_does => "template.parameter.bitWidth",
    the_row_of_template_checksum_algorithm_says_what_the_import_does => "template.checksum.algorithm",
    the_row_of_template_checksum_start_says_what_the_import_does => "template.checksum.start",
    the_row_of_template_checksum_length_says_what_the_import_does => "template.checksum.length",
    the_row_of_template_function_name_says_what_the_import_does => "template.function.name",
    the_row_of_template_any_says_what_the_import_does => "template.any",
);
