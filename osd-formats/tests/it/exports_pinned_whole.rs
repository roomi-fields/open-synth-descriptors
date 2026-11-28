//! The text and the losses of the Electra One, MIDI CSV and triggers CSV exports, and the device
//! and the losses of the Electra One import, each pinned whole (docs/CADRE.md R8,
//! docs/CADRE.md R16): every loss with its place, its reason and its rank in the list, for a
//! device before 0.7, whose losses are ordered on the descriptor as given and compared on its
//! conversion to 0.7, and for a command of 0.7 whose checksum has no length, which the comparison
//! completes and the order does not. The expected files are under tests/data/pinned/.

use crate::common::electra::{cc7, control, other, preset, source};
use crate::common::{device_from, resolved};
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};

const PINNED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/pinned");

fn pinned(file: &str, actual: &str) {
    let path = format!("{PINNED}/{file}");
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    assert_eq!(actual, expected, "{file}");
}

/// One line per loss, in the order of the list: its place, a tab, its reason.
fn lines(lost: &[Loss]) -> String {
    lost.iter().map(|l| format!("{}\t{}\n", l.place, l.reason)).collect()
}

/// The export of the device in the format is the text of `<name>.<extension>` and the losses of
/// `<name>.<extension>.losses`.
fn export_is_pinned(name: &str, format: Format, device: &Value) {
    let device = resolved(device_from(&device.to_string()));
    let out = export(&device, format).unwrap_or_else(|f| panic!("{}: {f}", format.name()));
    let file = format!("{name}.{}.{}", format.name(), format.extension());
    pinned(&file, &out.value);
    pinned(&format!("{file}.losses"), &lines(&out.lost));
}

/// A device of 0.6 of every block, its controls giving whole messages, messages not fully known
/// and a second message, its sections interleaved, its checksums without a length.
fn device_before_0_7() -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [
            {"type": "community-re", "url": "https://example.org/d", "author": "Dee", "firmware": "1.0", "license": "CC0-1.0",
             "parameter_numbers": {"/filter/dump": 12}, "_zz": "n"},
            {"type": "vendor-doc", "url": "https://example.org/e", "author": "Eve", "license": "MIT"}
        ],
        "device": {"name": "A device whose name is far too long for the controller", "vendor": "Lofty", "osc_prefix": "/trundler",
            "device_id": [16], "rate_limit_hz": 10, "_zz": "n", "_a/b~c": "n"},
        "programs": {"banks": [{"name": "A", "size": 64}]},
        "sysex": {"header": [240]},
        "role_vocabulary": "1",
        "cc_params": {"channel": 3, "_zz": "n", "entries": [
            {"osc": "/filter/cutoff", "_name": "Cutoff", "section": "Filter", "_description": "Opens", "_usage": "0 closed", "_notes": "n",
             "cc": 74, "range": [0, 100], "default": 64, "channel": 2, "_zz": "n",
             "also": [{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14}]},
            {"osc": "/lfo/rate", "_name": "Rate", "section": "Lfo", "cc": 1, "cc_lsb": 33, "default": 9000},
            {"osc": "/filter/half", "_name": "Half", "section": "Filter", "nrpn_msb": 1, "value_bits": 14},
            {"osc": "/filter/lsb", "_name": "Lsb", "section": "Filter", "cc_lsb": 38},
            {"osc": "/lfo/pair", "section": "Lfo", "cc": 40, "cc_lsb": 41},
            {"osc": "/filter/beside", "_name": "Beside", "section": "Filter", "cc": 75, "also": [{"nrpn_msb": 2, "value_bits": 14}]},
            {"osc": "/filter/waves", "_name": "Waves", "section": "Filter", "cc": 76,
             "values": {"A label far too long to show": 0, "Saw": 1}},
            {"osc": "/lfo/spread", "_name": "Spread", "section": "Lfo", "nrpn_msb": 3, "nrpn_lsb": 4, "value_bits": 14, "nrpn_lsb_range": [0, 15]},
            {"osc": "/bank", "_name": "Bank", "section": "Last"}
        ]},
        "commands": [
            {"osc": "/lfo/reset", "section": "Lfo", "_a/b~c": "n", "frame": [240, 65, 1, {"checksum": "twos7", "from": 1}, 247]},
            {"osc": "/filter/dump", "section": "Filter", "notes": "n", "_zz": "n",
             "args": [{"name": "value", "type": "u7", "range": [1, 10], "default": 3, "_zz": "n"}],
             "frame": [240, 67, {"arg": "value"}, {"checksum": "sum7", "from": 1}, 247]},
            {"osc": "/filter/added", "section": "Filter", "args": [{"name": "value", "type": "u7"}],
             "frame": [240, 67, {"arg": "value", "add": 1}, {"checksum": "xor7", "from": 1}, 247]}
        ],
        "triggers": {"_zz": "n", "entries": [
            {"osc": "/drums/kick", "sound": "Kick", "section": "Drums", "channel": 9, "note_range": [36, 38], "note": 37,
             "_description": "The big one", "comments": "choke", "velocity_usage": "soft", "_zz": "n"},
            {"osc": "/second", "sound": "Snare", "note_range": [40, 40]}
        ]},
        "_coverage": {"controls": 9},
        "_zz": "n",
        "_a/b~c": "n"
    })
}

/// The device before 0.7 with a note the chain of `osd` refuses to carry to 0.7.
fn device_before_0_7_the_chain_refuses() -> Value {
    let mut doc = device_before_0_7();
    doc["_midi_of_sources"] = json!("taken from the maker's manual");
    doc
}

/// A device of 0.7 whose commands give a checksum without a length, one of them written, with a
/// loss on each of its texts, its argument and its place in the preset.
fn device_of_0_7_with_checksums_without_length() -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "community-re", "url": "https://example.org/d", "license": "CC0-1.0",
            "parameter_numbers": {"/filter/dump": 12}}],
        "device": {"name": "Trundler", "osc_prefix": "/trundler"},
        "cc_params": {"entries": [
            {"osc": "/lfo/rate", "title": "Rate", "section": "Lfo", "ways": [{"ctrlType": "cc", "ctrlIndex": [1], "value_bits": 7}]},
            {"osc": "/filter/cutoff", "title": "Cutoff", "section": "Filter", "ways": [{"ctrlType": "cc", "ctrlIndex": [74], "value_bits": 7}]}
        ]},
        "commands": [
            {"osc": "/filter/dump", "id": 3, "title": "A command whose title is far too long to show", "section": "Filter",
             "description": "Asks", "notes": "n",
             "args": [{"name": "value", "type": "u7", "range": [1, 10], "values": {"on": 1}, "default": 3, "_zz": "n"}],
             "readings": [{"state": "valid", "firmware": "1.0"}],
             "frame": [240, 65, 16, {"arg": "value"}, {"checksum": "twos7", "from": 1}, 247], "_zz": "n", "_a/b~c": "n"},
            {"osc": "/lfo/reset", "section": "Lfo", "notes": "n", "frame": [240, 65, 1, {"checksum": "sum7", "from": 0}, 247]},
            {"osc": "/filter/added", "title": "Added", "section": "Filter", "args": [{"name": "value", "type": "u7"}],
             "frame": [240, 67, {"arg": "value", "add": 1}, {"checksum": "xor7", "from": 1}, 247]},
            {"osc": "/filter/empty", "title": "Empty", "section": "Filter", "notes": "n",
             "frame": [240, {"checksum": "xor7", "from": 1}, 247]},
            {"osc": "/filter/ahead", "title": "Ahead", "section": "Filter",
             "frame": [240, 1, {"checksum": "xor7", "from": 9}, 247]}
        ]
    })
}

#[test]
fn the_chain_of_osd_carries_one_device_before_0_7_to_0_7_and_refuses_the_other() {
    let converts = |doc: Value| osd::convert_to(&doc.to_string(), osd::Kind::Device, osd::Version::V0_7).map(drop);
    assert_eq!(converts(device_before_0_7()), Ok(()));
    let refused = converts(device_before_0_7_the_chain_refuses()).expect_err("the chain refuses the note");
    assert_eq!(refused.pointer, "/_midi_of_sources");
}

#[test]
fn the_electra_export_of_a_device_before_0_7_returns_this_text_and_these_losses_in_this_order() {
    export_is_pinned("before-0-7", Format::ElectraOne, &device_before_0_7());
}

#[test]
fn the_electra_export_of_a_device_before_0_7_the_chain_refuses_returns_this_text_and_these_losses_in_this_order() {
    export_is_pinned("before-0-7-refused", Format::ElectraOne, &device_before_0_7_the_chain_refuses());
}

#[test]
fn the_electra_export_of_a_checksum_without_length_returns_this_text_and_these_losses_in_this_order() {
    export_is_pinned("checksum-without-length", Format::ElectraOne, &device_of_0_7_with_checksums_without_length());
}

#[test]
fn the_midi_csv_export_of_a_device_before_0_7_returns_this_text_and_these_losses_in_this_order() {
    export_is_pinned("before-0-7", Format::MidiCsv, &device_before_0_7());
}

#[test]
fn the_midi_csv_export_of_a_device_before_0_7_the_chain_refuses_returns_this_text_and_these_losses_in_this_order() {
    export_is_pinned("before-0-7-refused", Format::MidiCsv, &device_before_0_7_the_chain_refuses());
}

#[test]
fn the_triggers_export_of_a_device_before_0_7_returns_this_text_and_these_losses_in_this_order() {
    export_is_pinned("before-0-7", Format::MidiTriggersCsv, &device_before_0_7());
}

#[test]
fn the_triggers_export_of_a_device_before_0_7_the_chain_refuses_returns_this_text_and_these_losses_in_this_order() {
    export_is_pinned("before-0-7-refused", Format::MidiTriggersCsv, &device_before_0_7_the_chain_refuses());
}

/// A preset with a control of several kinds, a list, a SysEx template with its value and
/// checksum, and keys the import does not read at each level.
fn rich_preset() -> Value {
    let mut cutoff = control(1, "Cutoff", Some(1), cc7(74), Some((0, 100)));
    cutoff["visible"] = json!(false);
    cutoff["unknown~key/x"] = json!(1);
    cutoff["values"][0]["formatter"] = json!("show");
    cutoff["values"][0]["message"]["lsbFirst"] = json!(true);
    let mut waves = control(2, "Waves", Some(1), cc7(75), Some((0, 1)));
    waves["type"] = json!("list");
    waves["values"][0]["overlayId"] = json!(1);
    let template = json!({"type": "sysex", "deviceId": 1, "min": 0, "max": 100, "data": [
        67, {"type": "value", "rules": [{"parameterBitPosition": 0, "byteBitPosition": 0, "bitWidth": 7}], "extra": 1},
        {"type": "checksum", "algorithm": "waldorf", "start": 1, "length": 2, "extra": 1}]});
    let dump = control(3, "Dump", None, template, None);
    let bend = control(4, "Bend", None, other("pitchbend"), None);
    let mut doc = preset(&[(1, "Filter")], vec![cutoff, waves, dump, bend]);
    doc["overlays"] = json!([{"id": 1, "items": [{"value": 0, "label": "Saw", "color": "FFFFFF"}, {"value": 1, "label": "Square"}]}]);
    doc["devices"][0]["rate"] = json!(10);
    doc["groups"][0]["color"] = json!("FF0000");
    doc["somethingElse"] = json!({"a": [1, 2]});
    doc
}

#[test]
fn the_electra_import_of_a_preset_returns_this_device_and_these_losses_in_this_order() {
    let t = import(&rich_preset().to_string(), Format::ElectraOne, &source()).expect("imports");
    let written = osd::write(&osd::Descriptor::Device(Box::new(t.value)));
    pinned("preset.imported.json", &written);
    pinned("preset.imported.json.losses", &lines(&t.lost));
}
