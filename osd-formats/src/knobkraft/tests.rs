//! The table of the licences allowed under the AGPL-3.0 is a table of the code, and the document
//! of the format carries one line per licence of it, with its piece.

use super::AGPL_COMPATIBLE;

#[test]
fn each_licence_of_the_compatible_table_has_its_line_with_its_piece_in_the_document() {
    let doc = include_str!("../../docs/knobkraft.md");
    assert!(!AGPL_COMPATIBLE.is_empty());
    for id in AGPL_COMPATIBLE.iter() {
        let start = format!("| `{id}` |");
        let line = doc.lines().find(|l| l.starts_with(&start)).unwrap_or_else(|| panic!("knobkraft.md has no line for {id}"));
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        assert!(cells.len() >= 2 && !cells[1].is_empty(), "{id} has no piece: {line}");
    }
}

#[test]
fn the_compatible_table_holds_the_licences_the_decision_names() {
    for id in ["AGPL-3.0", "GPL-3.0", "MIT", "Apache-2.0", "CC-BY-4.0"] {
        assert!(AGPL_COMPATIBLE.iter().any(|c| c.eq_ignore_ascii_case(id)), "{id} is not in the table");
    }
}
