//! The document of destiny docs/deluge.md: one line per tag and attribute the Deluge reader
//! accepts for a device definition (midi_instrument.cpp, `readCCLabelsFromFile` at 605-625 and
//! the tag names it dispatches on), with its fate, its reason and its piece.

const DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/deluge.md");

/// The tags and attributes the reader accepts: `midiDevice`, `ccLabels` and its attributes
/// (each a CC number from 0 to 119, written `0-119`), `hideUnlabeledCC` and its `value`.
const ACCEPTED: [&str; 5] = ["midiDevice", "ccLabels", "0-119", "hideUnlabeledCC", "value"];

fn doc_lines() -> Vec<Vec<String>> {
    std::fs::read_to_string(DOC)
        .unwrap_or_else(|e| panic!("{DOC}: {e}"))
        .lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .map(|l| l.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect())
        .collect()
}

#[test]
fn every_tag_and_attribute_the_reader_accepts_has_a_line_with_its_fate_its_reason_and_the_piece() {
    let lines = doc_lines();
    for name in ACCEPTED {
        let line = lines.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no line for {name} in {DOC}"));
        assert!(line.len() >= 4, "{name}: a line has the name, the fate, the reason and the piece");
        assert!(line[1] == "carried" || line[1] == "lost", "{name}: the fate is carried or lost, not {:?}", line[1]);
        assert!(!line[2].is_empty(), "{name}: no reason");
        assert!(line[3].contains("midi_instrument.cpp"), "{name}: the piece of the firmware is not cited: {:?}", line[3]);
    }
}

#[test]
fn the_document_lists_nothing_the_reader_does_not_accept() {
    for line in doc_lines() {
        let name = line[0].as_str();
        if name == "Element" || name.starts_with('-') || name.is_empty() {
            continue;
        }
        assert!(ACCEPTED.contains(&name), "{name} is in {DOC} and the reader does not accept it");
    }
}

#[test]
fn the_export_writes_the_device_the_labels_and_their_numbers_and_never_the_hide_flag() {
    let lines = doc_lines();
    for name in ["midiDevice", "ccLabels", "0-119"] {
        let line = lines.iter().find(|l| l[0] == name).unwrap();
        assert_eq!(line[1], "carried", "{name}");
    }
    for name in ["hideUnlabeledCC", "value"] {
        let line = lines.iter().find(|l| l[0] == name).unwrap();
        assert_eq!(line[1], "lost", "{name}");
    }
}
