//! The round-trip count of each importable format (tests/common/round_trip.rs) on a small file
//! whose figures are known by hand, each with a value carried, one changed, an element absent from
//! the export and a value the import does not read that the export writes equal; and the rule that
//! every value counted lost is named by a loss.

use crate::common::midnam::PUBLIC_ID;
use crate::common::round_trip::{count, count_trip, figures, pretty, structure_kinds, trip, Count};
use crate::common::{csv, row, trigger_csv, trigger_row};
use osd_formats::Format;
use serde_json::{json, Value};

fn assert_figures(format: Format, text: &str, rows: &[(&str, usize, usize, usize)]) {
    let got = count(format, text);
    assert_eq!(got.kinds, figures(rows), "{}:\n{}", format.name(), got.markdown());
}

fn assert_every_lost_value_named(got: &Count) {
    assert!(got.unnamed.is_empty(), "lost and named by no loss:\n{}", got.unnamed.join("\n"));
}

// ---- the MIDI CSV -----------------------------------------------------------------------------

fn midi_csv_file() -> String {
    let base = |manufacturer: &'static str, cells: &[(&'static str, &'static str)]| {
        let mut all = vec![("manufacturer", manufacturer), ("device", "Trundler")];
        all.extend_from_slice(cells);
        row(&all)
    };
    csv(&[
        base("Lofty", &[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("orientation", "0-based")]),
        // Another manufacturer: the export writes the first one.
        base("Other", &[("parameter_name", "Res"), ("cc_msb", "71")]),
        // No message: no row of the export.
        base("Lofty", &[("parameter_name", "Nothing")]),
        // A lone CC bound is not read; the export writes the NRPN's range there, equal.
        base("Lofty", &[("parameter_name", "Env"), ("cc_msb", "1"), ("cc_min_value", "0"), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_min_value", "0"), ("nrpn_max_value", "100")]),
    ])
}

#[test]
fn the_count_of_a_midi_csv_counts_each_filled_cell_once_under_its_column() {
    assert_figures(
        Format::MidiCsv,
        &midi_csv_file(),
        &[
            ("manufacturer", 2, 2, 0),
            ("device", 3, 1, 0),
            ("section", 1, 0, 0),
            ("parameter_name", 3, 1, 0),
            ("cc_msb", 3, 0, 0),
            ("cc_min_value", 1, 1, 0),
            ("cc_max_value", 1, 0, 0),
            ("orientation", 1, 0, 0),
            ("nrpn_msb", 1, 0, 0),
            ("nrpn_lsb", 1, 0, 0),
            ("nrpn_min_value", 1, 0, 0),
            ("nrpn_max_value", 1, 0, 0),
        ],
    );
}

/// Two rows of one CC, of which one comes back, and a row with no message.
fn one_message_twice() -> String {
    let base = |cells: &[(&'static str, &'static str)]| {
        let mut all = vec![("manufacturer", "Lofty"), ("device", "Trundler")];
        all.extend_from_slice(cells);
        row(&all)
    };
    csv(&[base(&[("parameter_name", "First"), ("cc_msb", "5")]), base(&[("parameter_name", "Second"), ("cc_msb", "5")]), base(&[("parameter_name", "Orphan")])])
}

#[test]
fn elements_of_one_key_are_paired_in_order_and_one_left_without_a_pair_loses_all_its_values() {
    assert_figures(Format::MidiCsv, &one_message_twice(), &[("manufacturer", 1, 2, 0), ("device", 1, 2, 0), ("parameter_name", 1, 2, 0), ("cc_msb", 1, 1, 0)]);
}

#[test]
fn every_value_the_midi_csv_loses_is_named() {
    assert_every_lost_value_named(&count(Format::MidiCsv, &midi_csv_file()));
}

#[test]
fn every_value_of_a_midi_csv_row_whose_message_an_earlier_row_holds_is_named() {
    assert_every_lost_value_named(&count(Format::MidiCsv, &one_message_twice()));
}

#[test]
fn every_value_of_a_midi_csv_row_with_half_an_nrpn_and_no_cc_is_named() {
    let half = csv(&[row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Half"), ("nrpn_msb", "1"), ("notes", "n")])]);
    assert_every_lost_value_named(&count(Format::MidiCsv, &half));
}

#[test]
fn every_value_the_midi_csv_loses_between_the_two_defaults_of_a_row_is_named() {
    let both = csv(&[row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("parameter_name", "Both"), ("cc_msb", "74"), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_min_value", "0"), ("nrpn_max_value", "16383"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("nrpn_default_value", "100")])]);
    assert_every_lost_value_named(&count(Format::MidiCsv, &both));
}

#[test]
fn a_value_counted_lost_that_no_loss_names_is_refused() {
    let text = midi_csv_file();
    let mut silent = trip(Format::MidiCsv, &text);
    silent.import_lost.clear();
    silent.export_lost.clear();
    let got = count_trip(Format::MidiCsv, &text, &silent, &structure_kinds(Format::MidiCsv));
    assert!(got.unnamed.iter().any(|u| u.starts_with("manufacturer")), "{:?}", got.unnamed);
    assert!(got.unnamed.iter().any(|u| u.starts_with("parameter_name = \"Nothing\"")), "{:?}", got.unnamed);
}

// ---- the triggers CSV ---------------------------------------------------------------------------

fn triggers_file() -> String {
    let base = |manufacturer: &'static str, cells: &[(&'static str, &'static str)]| {
        let mut all = vec![("manufacturer", manufacturer), ("device", "Trundler")];
        all.extend_from_slice(cells);
        trigger_row(&all)
    };
    trigger_csv(&[
        base("Lofty", &[("section", "Drums"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "36"), ("note_number_default_value", "36")]),
        base("Other", &[("sound_name", "Snare"), ("midi_channel", "omni"), ("note_number_min_value", "38"), ("note_number_max_value", "38")]),
        // A note range of one bound: no row of the export. Its manufacturer is not read, and the
        // export writes an equal one on its other rows.
        base("Lofty", &[("sound_name", "Nothing"), ("note_number_min_value", "40")]),
    ])
}

#[test]
fn the_count_of_a_triggers_csv_counts_each_filled_cell_once_under_its_column() {
    assert_figures(
        Format::MidiTriggersCsv,
        &triggers_file(),
        &[
            ("manufacturer", 1, 2, 0),
            ("device", 2, 1, 0),
            ("section", 1, 0, 0),
            ("sound_name", 2, 1, 0),
            ("midi_channel", 1, 1, 0),
            ("note_number_min_value", 2, 1, 0),
            ("note_number_max_value", 2, 0, 0),
            ("note_number_default_value", 1, 0, 0),
        ],
    );
}

#[test]
fn every_value_the_triggers_csv_loses_is_named() {
    assert_every_lost_value_named(&count(Format::MidiTriggersCsv, &triggers_file()));
}

// ---- the Electra One preset -----------------------------------------------------------------------

fn electra_file() -> String {
    pretty(json!({
        "version": 2,
        "name": "Small",
        "pages": [{"id": 1, "name": "Page 1"}],
        "groups": [{"id": 10, "name": "Filter"}],
        "controls": [
            {"id": 1, "type": "fader", "name": "Cutoff", "color": "FF0000", "groupId": 10,
             "values": [{"id": "value", "message": {"type": "cc7", "deviceId": 1, "parameterNumber": 74}}]},
            {"type": "pad", "name": "Hit", "values": [{"message": {"type": "note", "parameterNumber": 60}}]}
        ]
    }))
}

#[test]
fn the_count_of_an_electra_one_preset_counts_each_leaf_once_under_its_path_and_follows_its_structure() {
    assert_figures(
        Format::ElectraOne,
        &electra_file(),
        &[
            ("/version", 0, 0, 1),
            ("/name", 1, 0, 0),
            ("/pages/N/id", 0, 1, 0),
            ("/pages/N/name", 0, 1, 0),
            ("/groups/N/id", 0, 0, 1),
            ("/groups/N/name", 1, 0, 0),
            ("/controls/N/id", 0, 1, 0),
            ("/controls/N/type", 0, 2, 0),
            ("/controls/N/name", 1, 1, 0),
            ("/controls/N/color", 0, 1, 0),
            ("/controls/N/groupId", 0, 0, 1),
            ("/controls/N/values/N/id", 0, 1, 0),
            ("/controls/N/values/N/message/type", 1, 1, 0),
            ("/controls/N/values/N/message/deviceId", 0, 1, 0),
            ("/controls/N/values/N/message/parameterNumber", 1, 1, 0),
        ],
    );
}

#[test]
fn a_template_named_by_id_counts_its_bytes_against_the_command_it_gives_and_its_id_as_structure() {
    let text = pretty(json!({
        "version": 3, "name": "Named",
        "devices": [{"id": 1, "messages": [{"id": 5, "data": ["F0", "43", {"type": "value"}, "F7"]}]}],
        "controls": [{"type": "fader", "name": "Send", "values": [{"message": {"type": "sysex", "deviceId": 1, "data": 5}}]}]
    }));
    let got = count(Format::ElectraOne, &text);
    assert_eq!(got.kinds["/controls/N/values/N/message/data"], crate::common::round_trip::Figures::of(0, 0, 1), "{}", got.markdown());
    assert_eq!(got.kinds["/devices/N/messages/N/data/N"], crate::common::round_trip::Figures::of(3, 0, 0), "{}", got.markdown());
    assert_eq!(got.kinds["/devices/N/messages/N/data/N/type"], crate::common::round_trip::Figures::of(1, 0, 0), "{}", got.markdown());
}

#[test]
fn every_value_the_electra_one_preset_loses_is_named() {
    assert_every_lost_value_named(&count(Format::ElectraOne, &electra_file()));
}

// ---- MIDNAM -----------------------------------------------------------------------------------

/// The MIDNAM document, one element per line; its Author is the source's attribution.
fn midnam_file() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE MIDINameDocument PUBLIC "{PUBLIC_ID}" "http://www.midi.org/dtds/MIDINameDocument10.dtd">
<MIDINameDocument>
<Author>Someone</Author>
<MasterDeviceNames>
<Manufacturer>Lofty</Manufacturer>
<Model>Trundler</Model>
<CustomDeviceMode Name="Default">
<ChannelNameSetAssignments>
<ChannelNameSetAssign Channel="1" NameSet="Set A"/>
</ChannelNameSetAssignments>
</CustomDeviceMode>
<ChannelNameSet Name="Set A">
<AvailableForChannels>
<AvailableChannel Channel="1" Available="true"/>
</AvailableForChannels>
<ControlNameList Name="Knobs">
<Control Type="7bit" Number="74" Name="Cutoff">
<Values Min="0" Max="127"/>
</Control>
<Control Type="7bit" Number="71" Name="Res">
<Values Min="0" Max="100" Units="percent"/>
</Control>
</ControlNameList>
<PatchBank Name="A">
<MIDICommands>
<ControlChange Control="0" Value="1"/>
<ProgramChange Number="3"/>
</MIDICommands>
<PatchNameList Name="A">
<Patch Number="0" Name="A 0" ProgramChange="0"/>
</PatchNameList>
</PatchBank>
</ChannelNameSet>
</MasterDeviceNames>
</MIDINameDocument>
"#
    )
}

#[test]
fn the_count_of_a_midnam_counts_each_attribute_and_element_text_once() {
    assert_figures(
        Format::Midnam,
        &midnam_file(),
        &[
            ("Author", 0, 1, 0),
            ("Manufacturer", 1, 0, 0),
            ("Model", 1, 0, 0),
            ("CustomDeviceMode@Name", 0, 1, 0),
            ("ChannelNameSetAssign@Channel", 0, 1, 0),
            ("ChannelNameSetAssign@NameSet", 0, 1, 0),
            ("ChannelNameSet@Name", 0, 1, 0),
            ("AvailableChannel@Channel", 0, 1, 0),
            ("AvailableChannel@Available", 0, 1, 0),
            ("ControlNameList@Name", 0, 1, 0),
            ("Control@Type", 2, 0, 0),
            ("Control@Number", 2, 0, 0),
            ("Control@Name", 2, 0, 0),
            ("Values@Min", 2, 0, 0),
            ("Values@Max", 2, 0, 0),
            ("Values@Units", 0, 1, 0),
            ("PatchBank@Name", 1, 0, 0),
            ("ControlChange@Control", 1, 0, 0),
            ("ControlChange@Value", 1, 0, 0),
            ("ProgramChange@Number", 0, 1, 0),
            ("PatchNameList@Name", 1, 0, 0),
            ("Patch@Number", 1, 0, 0),
            ("Patch@Name", 1, 0, 0),
            ("Patch@ProgramChange", 1, 0, 0),
        ],
    );
}

#[test]
fn every_value_the_midnam_loses_is_named() {
    assert_every_lost_value_named(&count(Format::Midnam, &midnam_file()));
}

// ---- OSCQuery ---------------------------------------------------------------------------------

/// A namespace; the DESCRIPTION of its device is the one the export writes for it.
fn oscquery_file() -> String {
    let mut ns = json!({"FULL_PATH": "/", "ACCESS": 0, "CONTENTS": {"synth": {"FULL_PATH": "/synth", "ACCESS": 0, "CONTENTS": {
        "level": {"FULL_PATH": "/synth/level", "ACCESS": 2, "TYPE": "i", "RANGE": [{"MIN": 0, "MAX": 10}]},
        "pan": {"FULL_PATH": "/synth/pan", "ACCESS": 3, "TYPE": "i"},
        "meter": {"FULL_PATH": "/synth/meter", "ACCESS": 1, "TYPE": "f"}
    }}}});
    let written: Value = serde_json::from_str(&trip(Format::OscQuery, &ns.to_string()).exported).unwrap();
    ns["CONTENTS"]["synth"]["DESCRIPTION"] = written["CONTENTS"]["synth"]["DESCRIPTION"].clone();
    pretty(ns)
}

#[test]
fn the_count_of_an_oscquery_namespace_counts_each_leaf_once_under_its_attribute_path() {
    assert_figures(
        Format::OscQuery,
        &oscquery_file(),
        &[
            ("/FULL_PATH", 4, 1, 0),
            ("/ACCESS", 3, 2, 0),
            ("/DESCRIPTION", 0, 1, 0),
            ("/TYPE", 2, 1, 0),
            ("/RANGE/N/MIN", 1, 0, 0),
            ("/RANGE/N/MAX", 1, 0, 0),
        ],
    );
}

#[test]
fn every_value_the_oscquery_namespace_loses_is_named() {
    assert_every_lost_value_named(&count(Format::OscQuery, &oscquery_file()));
}

// ---- the MIDI 2.0 controller list ---------------------------------------------------------------

fn midi2_file() -> String {
    pretty(json!({
        "ChCtrlList": [
            {"title": "Cutoff", "ctrlType": "cc", "ctrlIndex": [74], "numSigBits": 7, "minMax": [0u64, 4294967295u64], "ctrlMapId": "wave"},
            {"title": "Res", "ctrlType": "cc", "ctrlIndex": [71], "numSigBits": 7, "colour": "red"},
            {"title": "Per note", "ctrlType": "pnrc", "ctrlIndex": [1, 2]},
            {"title": "Mine", "ctrlType": "cc", "ctrlIndex": [20], "numSigBits": 7, "paramPath": "/mine/thing"}
        ],
        "CtrlMapList": [{"ctrlMapId": "wave", "ctrlMap": [{"value": 0, "title": "Low"}, {"value": 127, "title": "High"}]}]
    }))
}

#[test]
fn the_count_of_a_midi2_controller_list_counts_each_leaf_once_and_follows_its_map_ids() {
    assert_figures(
        Format::Midi2Controllers,
        &midi2_file(),
        &[
            ("/ChCtrlList/N/title", 2, 2, 0),
            ("/ChCtrlList/N/ctrlType", 2, 2, 0),
            ("/ChCtrlList/N/ctrlIndex/N", 2, 3, 0),
            ("/ChCtrlList/N/numSigBits", 2, 1, 0),
            ("/ChCtrlList/N/minMax/N", 2, 0, 0),
            ("/ChCtrlList/N/ctrlMapId", 0, 0, 1),
            ("/ChCtrlList/N/colour", 0, 1, 0),
            ("/ChCtrlList/N/paramPath", 0, 1, 0),
            ("/CtrlMapList/N/ctrlMapId", 0, 0, 1),
            ("/CtrlMapList/N/ctrlMap/N/value", 2, 0, 0),
            ("/CtrlMapList/N/ctrlMap/N/title", 2, 0, 0),
        ],
    );
}

#[test]
fn every_value_the_midi2_controller_list_loses_is_named() {
    assert_every_lost_value_named(&count(Format::Midi2Controllers, &midi2_file()));
}
