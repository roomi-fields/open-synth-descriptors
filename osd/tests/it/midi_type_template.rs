//! `MidiType::template` gives the address and arguments the specification
//! fixes for each `midi_in` type, under the device's prefix.

use osd::{FaultKind, Kind, MidiType};
use serde_json::{json, Value};

const ALL: [MidiType; 7] = [
    MidiType::NoteOn,
    MidiType::NoteOff,
    MidiType::Cc,
    MidiType::Pitchbend,
    MidiType::Aftertouch,
    MidiType::PolyAftertouch,
    MidiType::ProgramChange,
];

/// The word the specification gives each type in a 0.4 `midi_in` and in the table.
fn word(t: MidiType) -> &'static str {
    match t {
        MidiType::NoteOn => "note_on",
        MidiType::NoteOff => "note_off",
        MidiType::Cc => "cc",
        MidiType::Pitchbend => "pitchbend",
        MidiType::Aftertouch => "aftertouch",
        MidiType::PolyAftertouch => "poly_aftertouch",
        MidiType::ProgramChange => "program_change",
    }
}

#[test]
fn each_type_reports_at_the_address_and_arguments_the_specification_fixes() {
    assert_eq!(MidiType::NoteOn.template(), "/note/on {note} {velocity} {channel}");
    assert_eq!(MidiType::NoteOff.template(), "/note/off {note} {velocity} {channel}");
    assert_eq!(MidiType::Cc.template(), "/cc/{num} {value} {channel}");
    assert_eq!(MidiType::Pitchbend.template(), "/pitchbend {value_u14} {channel}");
    assert_eq!(MidiType::Aftertouch.template(), "/aftertouch {value} {channel}");
    assert_eq!(MidiType::PolyAftertouch.template(), "/poly_aftertouch {note} {value} {channel}");
    assert_eq!(MidiType::ProgramChange.template(), "/program_change {program} {channel}");
}

#[test]
fn the_template_of_a_type_carries_no_device_prefix_and_starts_with_a_slash() {
    for t in ALL {
        assert!(t.template().starts_with('/'), "{}", word(t));
    }
}

#[test]
fn the_templates_of_the_specification_table_are_the_ones_the_reader_returns() {
    let doc = std::fs::read_to_string(crate::common::repo().join("spec/0.6.md")).unwrap();
    let section = doc.split("Addresses, arguments and `midi_in`").nth(1).expect("the section exists");
    let section = section.split("\n## ").next().unwrap();
    let rows: Vec<(String, String)> = section
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            let cells: Vec<&str> = l.trim().trim_matches('|').split("` | `").collect();
            assert_eq!(cells.len(), 2, "{l}");
            (cells[0].trim().trim_matches('`').to_string(), cells[1].trim().trim_matches('`').to_string())
        })
        .collect();
    let mut named: Vec<&str> = rows.iter().map(|(t, _)| t.as_str()).collect();
    let mut known: Vec<&str> = ALL.iter().map(|t| word(*t)).collect();
    named.sort();
    known.sort();
    assert_eq!(named, known, "the table names the seven types and no other");
    for (word_, text) in &rows {
        let t = ALL.iter().find(|t| word(**t) == word_).unwrap();
        assert_eq!(t.template(), text, "{word_}");
    }
}

fn convert(midi_in: Value) -> Result<Value, osd::Fault> {
    let doc = json!({"device": {"name": "T", "osc_prefix": "/t"}, "midi_in": midi_in});
    let out = osd::convert(&doc.to_string(), Kind::Device)?;
    Ok(serde_json::from_str(&out).unwrap())
}

#[test]
fn the_conversion_accepts_the_canonical_template_of_each_type() {
    for t in ALL {
        let out = convert(json!({ word(t): t.template() })).unwrap_or_else(|f| panic!("{}: {f}", word(t)));
        assert_eq!(out["midi_in"], json!([word(t)]));
    }
}

#[test]
fn the_conversion_refuses_a_template_other_than_the_canonical_one_of_its_type() {
    for t in ALL {
        let f = convert(json!({ word(t): format!("{} extra", t.template()) })).unwrap_err();
        assert_eq!(f.kind, FaultKind::NotConvertible, "{}", word(t));
    }
    let f = convert(json!({"note_on": MidiType::NoteOff.template()})).unwrap_err();
    assert_eq!(f.kind, FaultKind::NotConvertible);
}
