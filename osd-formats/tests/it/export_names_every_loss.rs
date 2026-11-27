//! The losses of the MIDI CSV and Electra One exports (osd-formats/docs/CADRE.md R8, docs/CADRE.md R16): on a
//! device that fills every field, each field that importing the file written gives back missing,
//! changed or added is in the losses at its pointer or at a block that holds it, and a field given
//! back unchanged is not named.

use crate::common::device_from;
use crate::common::fields::{broken_rules, held, trip, under, Trip};
use osd_formats::Format;
use serde_json::{json, Value};

const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const LONG: &str = "A name far too long for the controller to show whole, in every place";

/// The device through the format, every difference named and nothing unchanged named.
fn every_loss_is_named(format: Format, doc: &Value) -> Trip {
    let t = trip(format, device_from(&doc.to_string()));
    let broken = broken_rules(&t);
    assert!(broken.is_empty(), "{}:\n{}\nlosses: {:?}", format.name(), broken.join("\n"), t.lost);
    t
}

fn assert_held(t: &Trip, pointers: &[&str]) {
    let silent: Vec<&&str> = pointers.iter().filter(|p| !held(&t.lost, p)).collect();
    assert!(silent.is_empty(), "not in the losses: {silent:?}\n{:?}", t.lost);
}

/// Each field has a loss of its own, at its pointer or within it: the loss of the order of the
/// block that holds it says nothing of what the field loses.
fn assert_lost_at_the_field(t: &Trip, pointers: &[String]) {
    let silent: Vec<&String> = pointers.iter().filter(|p| !t.lost.iter().any(|l| under(&l.place, p) && !l.reason.is_empty())).collect();
    assert!(silent.is_empty(), "no loss at the field: {silent:?}\n{:?}", t.lost);
}

fn source() -> Value {
    json!({"type": "vendor-doc", "id": "s1", "url": "https://example.org/d", "author": "Dee", "firmware": "1.0", "license": "CC0-1.0", "_zz": "n"})
}

/// A device of every block, with a note on each object that takes one. Its first control and its
/// first command fill every field; a control or a command a format drops comes last.
fn full(commands: Value, entries: Value) -> Value {
    json!({
        "$schema": D07,
        "capabilities": ["lua"],
        "role_vocabulary": "1",
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler", "device_id": [16], "rate_limit_hz": 10,
            "kind": "software", "stands_for": "acme/box", "fidelity": "modelled",
            "transport": {"kind": "midi", "_zz": "n"}, "_zz": "n", "_a/b~c": "n"},
        "contexts": {"part": {"values": [{"id": "a"}]}},
        "loads_sounds_of": ["acme/other"],
        "resources": [{"resource": "patch", "canSet": "none", "export": "sysex", "_zz": "n"}],
        "identity": {"manufacturerId": [0, 32, 41], "_zz": "n"},
        "programs": {"banks": [{"name": "A", "size": 64}]},
        "sysex": {"header": [240], "_zz": "n"},
        "cc_params": {"channel": 3, "_zz": "n", "_a/b~c": "n", "entries": entries},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "_zz": "n",
            "entries": [{"osc": "/p", "ways": [{"ctrlType": "sysex", "id": 5}], "_zz": "n"}]},
        "commands": commands,
        "triggers": {"entries": [{"osc": "/kick", "sound": "Kick", "note_range": [36, 36], "_zz": "n"}], "_zz": "n"},
        "midi_out": {"default_channel": 1, "_zz": "n"},
        "midi_in": ["cc"],
        "replies": [{"match_osc": "/y", "emit_osc": "/x", "_zz": "n"}],
        "subscriptions": [{"forward": {"path": "/x", "_zz": "n"}, "_zz": "n"}],
        "custom_commands": [{"osc": "/c", "script": "x", "_zz": "n"}],
        "sources": [source()],
        "midi_of_sources": [{"type": "vendor-doc", "url": "https://example.org/m", "license": "MIT", "_zz": "n"}],
        "_coverage": {"controls": 1},
        "_zz": "n",
        "_a/b~c": "n"
    })
}

/// A control that fills every field, its ways these.
fn full_control(ways: Value) -> Value {
    json!({"osc": "/filter/cutoff", "title": "Cutoff", "section": "Filter", "description": "Opens", "value_description": "0 closed",
        "notes": "n", "scale": [0, 127], "default": 64, "priority": 1, "step_count": 8,
        "values": [{"value": 0, "title": "Off", "_zz": "n"}, {"value": 1, "title": LONG}],
        "polarity": "bipolar", "channel": 2, "expects": "switch", "transform": "x * 2",
        "role": {"parts": [{"part": "filter"}], "setting": "cutoff"},
        "readings": [{"state": "valid", "firmware": "1.0", "_zz": "n"}],
        "ways": ways, "_zz": "n", "_a/b~c": "n"})
}

fn wayless() -> Value {
    json!({"osc": "/bank", "title": "Bank", "section": "Last", "ways": [], "scale": [0, 7], "range": [0, 3], "transmit": "absolute",
        "recognize": "none", "nrpn_lsb_range": [0, 15], "_zz": "n"})
}

fn full_command(osc: &str, frame: Value) -> Value {
    json!({"osc": osc, "id": 3, "title": "Dump", "section": "Filter", "description": "Asks", "notes": "n",
        "args": [{"name": "value", "type": "u7", "range": [1, 10], "values": {"on": 1}, "default": 3, "_zz": "n"}],
        "readings": [{"state": "valid", "firmware": "1.0", "_zz": "n"}],
        "pre": [{"bind": "x", "expr": 1, "_zz": "n"}], "frame": frame, "forward": {"path": "/x", "_zz": "n"}, "script": "x", "_zz": "n"})
}

/// The smallest device of these blocks.
fn small(device: Value, extra: Value) -> Value {
    let mut doc = json!({"$schema": D07, "device": device, "sources": [{"type": "vendor-doc", "url": "https://example.org/d", "license": "CC0-1.0"}]});
    for (k, v) in extra.as_object().unwrap() {
        doc[k.as_str()] = v.clone();
    }
    doc
}

/// Where a place stands in the descriptor given: the position of each of its segments, a list item
/// by its index and a field by its rank in the order `osd` writes the object; none for a place the
/// descriptor does not hold.
fn position(given: &Value, place: &str) -> Option<Vec<usize>> {
    let (mut at, mut out) = (given, Vec::new());
    for segment in place.split('/').skip(1) {
        let key = segment.replace("~1", "/").replace("~0", "~");
        let (rank, next) = match at {
            Value::Array(a) => {
                let i: usize = key.parse().ok()?;
                (i, a.get(i)?)
            }
            Value::Object(o) => (o.keys().position(|k| *k == key)?, o.get(&key)?),
            _ => return None,
        };
        out.push(rank);
        at = next;
    }
    Some(out)
}

/// The losses at places the descriptor holds come in its order: list items by their index, a block
/// before what lies under it, the fields of an object as `osd` writes them.
fn assert_in_the_order_of_the_descriptor(t: &Trip) {
    let held: Vec<(Vec<usize>, &str)> = t.lost.iter().filter_map(|l| Some((position(&t.given, &l.place)?, l.place.as_str()))).collect();
    let out_of_order: Vec<String> = held.windows(2).filter(|w| w[0].0 > w[1].0).map(|w| format!("{} comes after {}", w[1].1, w[0].1)).collect();
    let places: Vec<&str> = held.iter().map(|h| h.1).collect();
    assert!(out_of_order.is_empty(), "{}\nlosses: {places:#?}", out_of_order.join("\n"));
}

// ---- the MIDI CSV ---------------------------------------------------------------------------

#[test]
fn the_losses_of_the_csv_export_come_in_the_order_of_the_descriptor_the_lost_order_of_the_ways_in_its_place() {
    let reordered = json!({"osc": "/a", "title": "A", "priority": 1, "ways": [
        {"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14, "range": [0, 127]},
        {"ctrlType": "cc", "ctrlIndex": [74], "range": [0, 127]},
    ]});
    let after = json!({"osc": "/b", "title": "B", "priority": 2, "ways": [{"ctrlType": "cc", "ctrlIndex": [75]}]});
    let doc = small(json!({"name": "Trundler", "osc_prefix": "/trundler"}), json!({"cc_params": {"entries": [reordered, after]}}));
    let t = every_loss_is_named(Format::MidiCsv, &doc);
    assert_held(&t, &["/cc_params/entries/0", "/cc_params/entries/1/priority", "/sources"]);
    assert_in_the_order_of_the_descriptor(&t);
}

#[test]
fn the_losses_of_a_full_device_come_in_the_order_of_the_descriptor_whoever_names_them() {
    let ways = json!([
        {"ctrlType": "cc", "ctrlIndex": [74], "value_bits": 7, "range": [0, 127], "transmit": "both", "_zz": "n"},
        {"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 7},
        {"ctrlType": "pBend"},
    ]);
    let rate = json!({"osc": "/lfo/rate", "title": "Rate", "section": "Lfo", "ways": [{"ctrlType": "cc", "ctrlIndex": [75]}]});
    let doc = full(json!([full_command("/filter/dump", json!([240, 1, {"arg": "value"}, 247])), {"osc": "/osc-only", "title": "Osc only"}]), json!([full_control(ways), rate, wayless()]));
    for format in [Format::MidiCsv, Format::ElectraOne] {
        assert_in_the_order_of_the_descriptor(&every_loss_is_named(format, &doc));
    }
}

#[test]
fn the_losses_of_the_electra_export_come_in_the_order_of_the_descriptor_the_moved_controls_in_their_place() {
    let entry = |n: u64, section: &str| {
        json!({"osc": format!("/{}/c{n}", section.to_lowercase()), "title": format!("C{n}"), "section": section,
            "description": "Opens", "ways": [{"ctrlType": "cc", "ctrlIndex": [n], "value_bits": 7}]})
    };
    let command = |name: &str, section: &str| json!({"osc": format!("/{}/{name}", section.to_lowercase()), "title": name, "section": section, "notes": "n", "frame": [240, 1, 247]});
    let doc = small(
        json!({"name": "Trundler", "osc_prefix": "/trundler"}),
        json!({"cc_params": {"entries": [entry(1, "A"), entry(2, "B"), entry(3, "A")]}, "commands": [command("first", "B"), command("second", "A")]}),
    );
    let t = trip(Format::ElectraOne, device_from(&doc.to_string()));
    assert_held(&t, &["/cc_params/entries/2", "/commands/1", "/sources"]);
    assert_in_the_order_of_the_descriptor(&t);
}

#[test]
fn the_csv_export_names_every_field_of_a_full_device_it_does_not_give_back_and_no_other() {
    let ways = json!([
        {"ctrlType": "cc", "ctrlIndex": [74], "value_bits": 7, "range": [0, 127], "transmit": "both", "recognize": "absolute",
         "in": {"part": "a"}, "slice": [0, 63], "_zz": "n"},
        {"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 7, "nrpn_lsb_range": [0, 10], "_zz": "n"},
        {"ctrlType": "pBend"},
    ]);
    let rate = json!({"osc": "/lfo/rate", "title": "Rate", "section": "Lfo", "ways": [{"ctrlType": "cc", "ctrlIndex": [75]}]});
    let doc = full(json!([full_command("/filter/dump", json!([240, 247]))]), json!([full_control(ways), rate, wayless()]));
    every_loss_is_named(Format::MidiCsv, &doc);
}

#[test]
fn the_csv_export_names_every_field_the_import_builds_otherwise() {
    let entries = json!([
        // No title, an address the import would not build, and two ways of which one alone has a
        // range: the row writes the scale of the other.
        {"osc": "/x/y", "ways": [{"ctrlType": "cc", "ctrlIndex": [20], "range": [0, 100]}, {"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14}]},
        // A default whose column puts the NRPN first.
        {"osc": "/b", "title": "B", "default": 9000, "ways": [{"ctrlType": "cc", "ctrlIndex": [21]}, {"ctrlType": "nrpn", "ctrlIndex": [1, 3], "value_bits": 14}]},
        // An NRPN then a CC of one scale: the CC comes back first.
        {"osc": "/c", "title": "C", "ways": [{"ctrlType": "nrpn", "ctrlIndex": [1, 4], "value_bits": 14, "range": [0, 127]}, {"ctrlType": "cc", "ctrlIndex": [22], "range": [0, 127]}]},
    ]);
    let doc = small(json!({"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/other"}), json!({"cc_params": {"entries": entries}}));
    let t = every_loss_is_named(Format::MidiCsv, &doc);
    assert_held(&t, &["/device/osc_prefix", "/cc_params/entries/0/osc", "/cc_params/entries/0/title"]);
}

#[test]
fn the_csv_export_names_at_its_own_pointer_each_field_a_control_whose_ways_come_back_in_another_order_loses() {
    let ways = json!([
        {"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14, "range": [0, 127]},
        {"ctrlType": "cc", "ctrlIndex": [74], "range": [0, 127]},
    ]);
    // The order is lost with no default, and with one whose column puts the NRPN first.
    for (ways, default) in [(ways.clone(), None), (json!([{"ctrlType": "cc", "ctrlIndex": [74]}, {"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14}]), Some(9000))] {
        let mut entry = json!({"osc": "/a", "title": "A", "priority": 1, "expects": "switch", "values": [{"value": 0, "title": "Off"}], "ways": ways});
        if let Some(default) = default {
            entry["default"] = json!(default);
        }
        let doc = small(json!({"name": "Trundler", "osc_prefix": "/trundler"}), json!({"cc_params": {"entries": [entry]}}));
        let t = every_loss_is_named(Format::MidiCsv, &doc);
        assert_lost_at_the_field(&t, &["priority", "expects", "values"].map(|f| format!("/cc_params/entries/0/{f}")));
    }
}

#[test]
fn the_csv_export_that_writes_no_row_names_the_name_and_the_manufacturer_of_the_device() {
    let device = json!({"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"});
    for extra in [
        json!({}),
        json!({"cc_params": {"entries": []}}),
        json!({"commands": [{"osc": "/dump", "frame": [240, 247]}]}),
        // A row with no CC and no NRPN is no row an import reads.
        json!({"cc_params": {"entries": [wayless()]}}),
    ] {
        let t = every_loss_is_named(Format::MidiCsv, &small(device.clone(), extra.clone()));
        assert_held(&t, &["/device/name", "/device/vendor"]);
    }
}

#[test]
fn the_csv_export_names_the_values_role_priority_steps_readings_and_scale_of_a_control_and_the_blocks_of_the_device() {
    let entry = json!({"osc": "/a", "title": "A", "priority": 1, "step_count": 8, "values": [{"value": 0, "title": "Off"}],
        "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}, "readings": [{"state": "valid", "firmware": "1.0"}], "scale": [0, 127],
        "ways": [{"ctrlType": "cc", "ctrlIndex": [1], "transmit": "both", "recognize": "both", "in": {"part": "a"}, "slice": [0, 63], "_zz": "n"}]});
    let doc = small(json!({"name": "Trundler", "osc_prefix": "/trundler"}), json!({
        "role_vocabulary": "1", "contexts": {"part": {"values": [{"id": "a"}]}},
        "resources": [{"resource": "patch", "canSet": "none", "export": "sysex"}],
        "sound_transfers": crate::common::knobkraft::base()["sound_transfers"].clone(),
        "cc_params": {"entries": [entry]}}));
    let t = every_loss_is_named(Format::MidiCsv, &doc);
    let fields = ["priority", "step_count", "values", "role", "readings", "scale", "ways/0/transmit", "ways/0/recognize", "ways/0/in", "ways/0/slice", "ways/0/_zz"];
    let pointers: Vec<String> = fields.iter().map(|f| format!("/cc_params/entries/0/{f}")).collect();
    let mut pointers: Vec<&str> = pointers.iter().map(String::as_str).collect();
    pointers.extend(["/contexts", "/sound_transfers", "/resources"]);
    assert_held(&t, &pointers);
}

// ---- the Electra One ------------------------------------------------------------------------

#[test]
fn the_electra_export_names_every_field_of_a_full_device_it_does_not_give_back_and_no_other() {
    let ways = json!([
        {"ctrlType": "cc", "ctrlIndex": [74], "value_bits": 7, "range": [0, 127], "transmit": "both", "recognize": "absolute",
         "in": {"part": "a"}, "slice": [0, 63], "_zz": "n"},
        {"ctrlType": "pBend"},
    ]);
    let rate = json!({"osc": "/lfo/rate", "title": "Rate", "section": "Lfo", "ways": [{"ctrlType": "cc", "ctrlIndex": [75], "value_bits": 7}]});
    let commands = json!([
        full_command("/filter/dump", json!([240, 1, {"arg": "value"}, 247])),
        full_command("/filter/added", json!([240, 1, {"arg": "value", "add": 1}, 247])),
        {"osc": "/osc-only", "title": "Osc only", "_zz": "n"},
    ]);
    let doc = full(commands, json!([full_control(ways), rate, wayless()]));
    every_loss_is_named(Format::ElectraOne, &doc);
}

#[test]
fn the_electra_export_names_what_it_cuts_renames_and_writes_otherwise() {
    let way = |n: u64| json!([{"ctrlType": "cc", "ctrlIndex": [n], "value_bits": 7}]);
    let entries = json!([
        {"osc": "/s/cutoff", "title": "Cutoff", "section": "S", "ways": way(1)},
        // The second control of this name in its section is renamed.
        {"osc": "/s/cutoff-2", "title": "Cutoff", "section": "S", "ways": way(2)},
        {"osc": "/s/bare", "section": "S", "ways": way(3)},
        {"osc": "/elsewhere", "title": "There", "section": "S", "ways": way(4)},
        {"osc": "/s/width", "title": "Width", "section": "S", "ways": [{"ctrlType": "cc", "ctrlIndex": [5]}]},
        {"osc": "/s/fine", "title": "Fine", "section": "S", "ways": [{"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 7}]},
        {"osc": "/s/steps", "title": "Steps", "section": "S", "expects": "discrete",
         "ways": [{"ctrlType": "cc", "ctrlIndex": [6], "value_bits": 7, "range": [0, 3]}]},
        {"osc": "/s/waves", "title": "Waves", "section": "S", "values": [{"value": 0, "title": "A label far too long to show"}, {"value": 1, "title": "Saw"}],
         "ways": way(7)},
        {"osc": "/long", "title": LONG, "section": LONG, "ways": way(8)},
    ]);
    let commands = json!([{"osc": "/s/bare-dump", "section": "S", "frame": [240, 1, 247]}]);
    let doc = small(json!({"name": LONG, "osc_prefix": "/trundler"}), json!({"cc_params": {"entries": entries}, "commands": commands}));
    let t = every_loss_is_named(Format::ElectraOne, &doc);
    assert_held(&t, &[
        "/device/name", "/device/osc_prefix", "/cc_params/entries/1/title", "/cc_params/entries/2/title", "/cc_params/entries/3/osc",
        "/cc_params/entries/4/ways/0/value_bits", "/cc_params/entries/5/ways/0/value_bits", "/cc_params/entries/6/expects",
        "/cc_params/entries/7/values/0/title", "/cc_params/entries/8/title", "/cc_params/entries/8/section", "/commands/0/title",
    ]);
}

#[test]
fn the_electra_export_names_the_width_of_a_7_bit_nrpn_or_rpn_way_which_the_import_reads_on_14_bits() {
    for kind in ["nrpn", "rpn"] {
        let entry = json!({"osc": "/a", "title": "A", "ways": [{"ctrlType": kind, "ctrlIndex": [1, 2], "value_bits": 7}]});
        let doc = small(json!({"name": "Trundler", "osc_prefix": "/trundler"}), json!({"cc_params": {"entries": [entry]}}));
        let t = every_loss_is_named(Format::ElectraOne, &doc);
        assert_held(&t, &["/cc_params/entries/0/ways/0/value_bits"]);
    }
}

#[test]
fn the_electra_export_names_at_its_own_pointer_each_field_a_control_or_a_command_laid_out_in_another_order_loses() {
    // The sections interleave: the preset lays each group out together, so the third control and
    // the second command are written before ones that precede them.
    let entry = |n: u64, section: &str| {
        json!({"osc": format!("/{}/c{n}", section.to_lowercase()), "title": format!("C{n}"), "section": section,
            "description": "Opens", "notes": "n", "priority": 1, "ways": [{"ctrlType": "cc", "ctrlIndex": [n], "value_bits": 7}]})
    };
    let command = |name: &str, section: &str| {
        json!({"osc": format!("/{}/{name}", section.to_lowercase()), "title": name, "section": section,
            "description": "Asks", "notes": "n", "frame": [240, 1, 247]})
    };
    let entries = json!([entry(1, "A"), entry(2, "B"), entry(3, "A")]);
    let commands = json!([command("first", "B"), command("second", "A")]);
    let doc = small(json!({"name": "Trundler", "osc_prefix": "/trundler"}), json!({"cc_params": {"entries": entries}, "commands": commands}));
    // Not `every_loss_is_named`, which compares the lists place by place: here the places change.
    let t = trip(Format::ElectraOne, device_from(&doc.to_string()));
    for moved in ["/cc_params/entries/1/title", "/commands/0/title"] {
        assert_ne!(t.back.pointer(moved), t.given.pointer(moved), "the preset moves its items: {moved}");
    }
    assert_held(&t, &["/cc_params/entries/2", "/commands/1"]);
    let mut pointers = Vec::new();
    for n in 0..3 {
        pointers.extend(["description", "notes", "priority"].map(|f| format!("/cc_params/entries/{n}/{f}")));
    }
    for n in 0..2 {
        pointers.extend(["description", "notes"].map(|f| format!("/commands/{n}/{f}")));
    }
    assert_lost_at_the_field(&t, &pointers);
}

#[test]
fn a_device_each_format_gives_back_whole_names_its_sources_only() {
    for format in [Format::MidiCsv, Format::ElectraOne] {
        let t = every_loss_is_named(format, &crate::common::fields::base(format));
        let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
        assert_eq!(places, ["/sources"], "{}", format.name());
    }
}
