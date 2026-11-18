//! The table of docs/electra-one.md: one line per kind of message the maker documents, with its
//! fate, its reason and the piece of the maker's page; and the numbers the export lays out by.

use crate::common::electra::{COMMAND_KIND, CONTROL_KINDS, KINDS};

const TABLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/electra-one.md");

fn table_lines() -> Vec<Vec<String>> {
    std::fs::read_to_string(TABLE)
        .unwrap_or_else(|e| panic!("{TABLE}: {e}"))
        .lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .map(|l| l.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect())
        .collect()
}

#[test]
fn every_kind_of_message_the_maker_documents_has_a_line_with_its_fate_reason_and_piece() {
    let lines = table_lines();
    for kind in KINDS {
        let line = lines.iter().find(|l| l[0] == kind).unwrap_or_else(|| panic!("no line for the kind {kind} in {TABLE}"));
        assert!(line.len() >= 4, "{kind}: a line has the kind, the fate, the reason and the piece");
        assert!(!line[2].is_empty(), "{kind}: no reason");
        assert!(line[3].contains("presetformat"), "{kind}: the piece of the maker's page is not cited: {:?}", line[3]);
    }
}

#[test]
fn the_fate_is_control_for_the_four_carried_kinds_command_for_sysex_and_lost_for_the_rest() {
    let lines = table_lines();
    for kind in KINDS {
        let line = lines.iter().find(|l| l[0] == kind).unwrap();
        let expected = if CONTROL_KINDS.contains(&kind) {
            "control"
        } else if kind == COMMAND_KIND {
            "command"
        } else {
            "lost"
        };
        assert_eq!(line[1], expected, "{kind}");
    }
}

#[test]
fn the_table_lists_no_kind_the_maker_does_not_document() {
    let known = ["pages", "control sets", "pots", "controls", "devices"];
    for line in table_lines() {
        let name = line[0].as_str();
        if name == "Kind" || name == "Limit" || name.starts_with('-') || known.contains(&name) {
            continue;
        }
        assert!(KINDS.contains(&name), "{name} is in the table and not in the documented kinds");
    }
}

#[test]
fn the_numbers_the_export_lays_out_by_are_in_the_table_with_the_piece() {
    let lines = table_lines();
    for (name, number) in [("pages", 12), ("control sets", 3), ("pots", 12), ("controls", 432), ("devices", 32)] {
        let line = lines.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no line for {name} in {TABLE}"));
        assert_eq!(line[1].parse::<u32>().ok(), Some(number), "{name}");
        assert!(line.len() >= 3 && line[2].contains("presetformat"), "{name}: no piece of the maker's page");
    }
}
