//! The closed list of formats: the present ones at their positions, each with its name; a format
//! added comes after them.

use osd_formats::Format;

#[test]
fn midnam_and_oscquery_come_after_electra_one_in_the_list_of_formats() {
    assert_eq!(&Format::ALL[..5], [Format::MidiCsv, Format::MidiTriggersCsv, Format::ElectraOne, Format::Midnam, Format::OscQuery]);
}

#[test]
fn the_midi2_controller_list_comes_after_oscquery_and_is_named_midi2_controllers() {
    assert_eq!(Format::ALL[5..6], [Format::Midi2Controllers]);
    assert_eq!(Format::Midi2Controllers.name(), "midi2-controllers");
    assert_eq!(Format::from_name("midi2-controllers"), Some(Format::Midi2Controllers));
    assert_eq!(Format::from_name("midi2"), None);
}

#[test]
fn the_deluge_comes_after_the_midi2_controller_list_and_is_named_deluge() {
    assert_eq!(Format::ALL[6..7], [Format::Deluge]);
    assert_eq!(Format::Deluge.name(), "deluge");
    assert_eq!(Format::from_name("deluge"), Some(Format::Deluge));
    assert_eq!(Format::from_name("Deluge"), None);
}

#[test]
fn knobkraft_comes_after_the_deluge_and_is_named_knobkraft() {
    assert_eq!(Format::ALL[7..8], [Format::Knobkraft]);
    assert_eq!(Format::Knobkraft.name(), "knobkraft");
    assert_eq!(Format::from_name("knobkraft"), Some(Format::Knobkraft));
    assert_eq!(Format::from_name("KnobKraft"), None);
    assert_eq!(Format::from_name("knobkraft-orm"), None);
}

#[test]
fn the_interfaces_of_the_crate_and_of_the_command_list_deluge() {
    for path in ["/docs/INTERFACE.md", "/../osd-cli/docs/INTERFACE.md"] {
        let file = format!("{}{path}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(text.contains("`deluge`"), "{file} does not list `deluge`");
    }
}

#[test]
fn the_interfaces_and_the_frame_list_midi2_controllers() {
    for path in ["/docs/INTERFACE.md", "/docs/CADRE.md", "/../osd-cli/docs/INTERFACE.md"] {
        let file = format!("{}{path}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(text.contains("midi2-controllers"), "{file} does not list midi2-controllers");
    }
}

#[test]
fn the_triggers_csv_follows_the_controller_csv_and_is_named_midi_triggers_csv() {
    assert_eq!(&Format::ALL[..2], [Format::MidiCsv, Format::MidiTriggersCsv]);
    assert_eq!(Format::MidiTriggersCsv.name(), "midi-triggers-csv");
    assert_eq!(Format::from_name("midi-triggers-csv"), Some(Format::MidiTriggersCsv));
    assert_eq!(Format::from_name("midi-triggers"), None);
}

#[test]
fn the_interfaces_of_the_crate_and_of_the_command_list_midi_triggers_csv() {
    for path in ["/docs/INTERFACE.md", "/../osd-cli/docs/INTERFACE.md"] {
        let file = format!("{}{path}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(text.contains("`midi-triggers-csv`"), "{file} does not list `midi-triggers-csv`");
    }
}

#[test]
fn each_format_is_named_and_found_by_its_name() {
    assert_eq!(Format::Midnam.name(), "midnam");
    assert_eq!(Format::OscQuery.name(), "oscquery");
    for f in Format::ALL {
        assert_eq!(Format::from_name(f.name()), Some(f), "{}", f.name());
    }
    assert_eq!(Format::from_name("midnam"), Some(Format::Midnam));
    assert_eq!(Format::from_name("oscquery"), Some(Format::OscQuery));
    assert_eq!(Format::from_name("osc-query"), None);
    assert_eq!(Format::from_name("MIDNAM"), None);
}

#[test]
fn the_interfaces_of_the_crate_and_of_the_command_list_midnam_and_oscquery() {
    for path in ["/docs/INTERFACE.md", "/../osd-cli/docs/INTERFACE.md"] {
        let file = format!("{}{path}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
        for word in ["`midnam`", "`oscquery`"] {
            assert!(text.contains(word), "{file} does not list {word}");
        }
    }
}
