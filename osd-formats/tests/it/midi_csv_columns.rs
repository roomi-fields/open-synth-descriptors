//! The table of the CSV's columns, in docs/midi-csv.md: one line per column of the header the
//! export writes, saying where it goes or why it is lost.

use crate::common::{device_from, resolved, HEADER};
use osd_formats::{export, Format};

const TABLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/midi-csv.md");

fn written_header() -> Vec<String> {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "_sources": [{"type": "community-re", "url": "u", "license": "CC0-1.0"}], "device": {"name": "T", "osc_prefix": "/t"}}"#;
    let out = export(&resolved(device_from(text)), Format::MidiCsv).unwrap().value;
    out.lines().next().unwrap().split(',').map(str::to_string).collect()
}

/// The cells of the lines of a Markdown table, the first cell stripped of its backticks.
fn table_lines() -> Vec<Vec<String>> {
    std::fs::read_to_string(TABLE)
        .unwrap_or_else(|e| panic!("{TABLE}: {e}"))
        .lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .map(|l| l.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect())
        .collect()
}

#[test]
fn the_export_writes_the_header_of_the_pencilresearch_template() {
    assert_eq!(written_header(), HEADER.to_vec());
}

#[test]
fn every_column_of_the_header_has_a_line_saying_its_fate_and_its_reason_when_lost() {
    let lines = table_lines();
    for column in written_header() {
        let line = lines.iter().find(|l| l[0] == column).unwrap_or_else(|| panic!("no line for the column {column} in {TABLE}"));
        assert!(line.len() >= 2 && !line[1].is_empty(), "{column}: no fate");
        assert!(line.len() >= 3 && !line[2].is_empty(), "{column}: no reason or note");
    }
}

#[test]
fn the_table_lists_no_column_the_header_does_not_have() {
    let header = written_header();
    let names: Vec<String> = table_lines().into_iter().map(|l| l[0].clone()).collect();
    for name in names.iter().filter(|n| !["Column", "---"].contains(&n.as_str()) && !n.starts_with('-')) {
        assert!(header.contains(name), "{name} is in the table and not in the header");
    }
}
