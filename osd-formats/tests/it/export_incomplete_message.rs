//! The export of a device of 0.5 or 0.6 whose control gives a message not fully known (an NRPN or
//! an RPN given by one number, a `cc_lsb` without its `cc`): no format writes a number the source
//! does not give (docs/CADRE.md R9), and each names in its losses, at the pointer of its field,
//! the message it does not carry (docs/CADRE.md R4, docs/CADRE.md R8). A format that has a column for each number of a message
//! may carry the number given, in its own column.

use crate::common::deluge::labels_read;
use crate::common::midnam::exported as exported_midnam;
use crate::common::{device_from, parse, resolved, HEADER};
use osd_formats::{export, Format, Loss, Translated};
use serde_json::{json, Value};

/// The formats that write the controls of `cc_params`.
const FORMATS: [Format; 6] = [Format::MidiCsv, Format::ElectraOne, Format::Midnam, Format::OscQuery, Format::Midi2Controllers, Format::Deluge];

/// A device of `version`, with one source and this one control.
fn device(version: &str, entry: &Value) -> osd::Resolved {
    let doc = json!({
        "$schema": format!("https://w3id.org/osd/{version}/device.schema.json"),
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "_sources": [{"type": "community-re", "url": "https://example.org/x", "license": "CC0-1.0"}],
        "cc_params": {"entries": [entry]},
    });
    resolved(device_from(&doc.to_string()))
}

fn exported(d: &osd::Resolved, format: Format) -> Translated<String> {
    export(d, format).unwrap_or_else(|f| panic!("{format:?}: {f}"))
}

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

/// Every object of a JSON value, itself included.
fn objects(v: &Value) -> Vec<&serde_json::Map<String, Value>> {
    match v {
        Value::Object(o) => std::iter::once(o).chain(o.values().flat_map(objects)).collect(),
        Value::Array(l) => l.iter().flat_map(objects).collect(),
        _ => Vec::new(),
    }
}

/// The messages a file of `format` writes for its controls, as (kind, number), for the formats
/// that say a message by one kind and one number or list of numbers.
fn messages_written(text: &str, format: Format) -> Vec<(String, Value)> {
    let of = |v: &Value, kind: &str, number: &str| -> Vec<(String, Value)> {
        objects(v)
            .into_iter()
            .filter_map(|o| Some((o.get(kind)?.as_str()?.to_string(), o.get(number)?.clone())))
            .collect()
    };
    match format {
        Format::ElectraOne => of(&json_of(text), "type", "parameterNumber"),
        Format::Midi2Controllers => of(&json_of(text), "ctrlType", "ctrlIndex"),
        Format::Deluge => labels_read(text).into_iter().map(|(n, _)| ("cc".to_string(), json!([n]))).collect(),
        _ => Vec::new(),
    }
}

/// The cells of the four message columns of the one row of a CSV export.
fn csv_numbers(text: &str) -> [String; 4] {
    let records = parse(text);
    assert_eq!(records.len(), 2, "{text}");
    ["cc_msb", "cc_lsb", "nrpn_msb", "nrpn_lsb"].map(|c| records[1][HEADER.iter().position(|h| *h == c).unwrap()].clone())
}

fn names(lost: &[Loss], place: &str) -> bool {
    let under = format!("{place}/");
    lost.iter().any(|l| (l.place == place || l.place.starts_with(&under)) && !l.reason.is_empty())
}

/// For each (version, control): no format writes a number for the control but `complete`, the CC
/// it gives whole; each format names the field `field` in its losses, except the CSV when it
/// writes `given` in the column `column` of that number.
fn assert_nothing_invented_and_nothing_silently_lost(devices: &[(&str, Value)], complete: Option<u32>, field: &str, column: &str, given: i64) {
    let place = format!("/cc_params/entries/0/{field}");
    let mut silent = Vec::new();
    for (version, entry) in devices {
        let d = device(version, entry);
        for format in FORMATS {
            let out = exported(&d, format);
            let at = format!("{version} {format:?} {entry}");
            let mut carried = false;
            match format {
                Format::MidiCsv => {
                    let cells = csv_numbers(&out.value);
                    for (name, cell) in ["cc_msb", "cc_lsb", "nrpn_msb", "nrpn_lsb"].into_iter().zip(&cells) {
                        let whole = complete.filter(|_| name == "cc_msb").map(|n| n.to_string());
                        let half = (name == column).then(|| given.to_string());
                        assert!(cell.is_empty() || Some(cell) == whole.as_ref() || Some(cell) == half.as_ref(), "{at}: {name} = {cell:?}\n{}", out.value);
                        carried |= Some(cell) == half.as_ref();
                    }
                    if let Some(n) = complete {
                        assert_eq!(cells[0], n.to_string(), "{at}: the whole CC is written\n{}", out.value);
                    }
                }
                Format::Midnam => {
                    let text = exported_midnam(&d).0;
                    assert!(!text.contains("Type=\"NRPN\"") && !text.contains("Type=\"RPN\""), "{at}: {text}");
                    assert!(!text.contains(&format!("Number=\"{given}\"")), "{at}: {text}");
                }
                _ => {
                    for (kind, number) in messages_written(&out.value, format) {
                        let whole = complete.is_some_and(|n| kind.starts_with("cc") && (number == json!(n) || number == json!([n])));
                        assert!(whole, "{at}: a {kind} message {number} is written\n{}", out.value);
                    }
                }
            }
            if !carried && !names(&out.lost, &place) {
                silent.push(format!("{version} {format:?}: {:?}", out.lost.iter().map(|l| l.place.as_str()).collect::<Vec<_>>()));
            }
        }
    }
    assert!(silent.is_empty(), "{devices:?}: nothing in the losses names {place}:\n{}", silent.join("\n"));
}

#[test]
fn an_export_names_the_nrpn_of_one_number_it_does_not_carry_beside_a_whole_cc() {
    let devices = [("0.5", json!({"osc": "/a", "cc": 74, "nrpn_msb": 1}))];
    assert_nothing_invented_and_nothing_silently_lost(&devices, Some(74), "nrpn_msb", "nrpn_msb", 1);
}

#[test]
fn an_export_names_the_nrpn_of_one_number_that_is_the_only_message_of_a_control() {
    let devices = [
        ("0.5", json!({"osc": "/a", "nrpn_msb": 1})),
        ("0.6", json!({"osc": "/a", "nrpn_msb": 1, "value_bits": 14})),
    ];
    assert_nothing_invented_and_nothing_silently_lost(&devices, None, "nrpn_msb", "nrpn_msb", 1);
}

#[test]
fn an_export_names_the_cc_lsb_without_its_cc_it_does_not_carry() {
    let devices = [("0.5", json!({"osc": "/a", "cc_lsb": 38})), ("0.6", json!({"osc": "/a", "cc_lsb": 38}))];
    assert_nothing_invented_and_nothing_silently_lost(&devices, None, "cc_lsb", "cc_lsb", 38);
}

#[test]
fn an_export_names_the_incomplete_message_of_an_also_beside_a_whole_cc() {
    // The field named is `also`, which holds the message, or a place under it.
    let devices = [("0.6", json!({"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "value_bits": 14}]}))];
    assert_nothing_invented_and_nothing_silently_lost(&devices, Some(74), "also", "nrpn_msb", 1);
}
