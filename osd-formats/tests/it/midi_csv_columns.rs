//! The table of the CSV's columns, in docs/midi-csv.md: one row per column of the header the
//! export writes, its fate (`carried`, `lost` or `conditional`) and its reason or note; each row
//! checked on files that fill the column (osd-formats/docs/CADRE.md R8).

use crate::common::fields::{row_of, table};
use crate::common::{csv, device_from, resolved, row, source, HEADER};
use osd_formats::{export, import, Format};
use serde_json::Value;

const TABLE: &str = "Column";

fn written_header() -> Vec<String> {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "_sources": [{"type": "community-re", "url": "u", "license": "CC0-1.0"}], "device": {"name": "T", "osc_prefix": "/t"}}"#;
    let out = export(&resolved(device_from(text)), Format::MidiCsv).unwrap().value;
    out.lines().next().unwrap().split(',').map(str::to_string).collect()
}

#[test]
fn the_export_writes_the_header_of_the_pencilresearch_template() {
    assert_eq!(written_header(), HEADER.to_vec());
}

#[test]
fn every_column_of_the_header_has_a_row_and_the_table_lists_no_other() {
    let names: Vec<String> = table("midi-csv", TABLE).into_iter().map(|r| r[0].clone()).collect();
    assert_eq!(names, written_header(), "docs/midi-csv.md: one row per column, in the order of the header");
}

/// A row that fills every column, each carried.
fn full() -> Vec<(&'static str, &'static str)> {
    vec![
        ("manufacturer", "Lofty"),
        ("device", "Trundler"),
        ("section", "Filter"),
        ("parameter_name", "Cutoff"),
        ("parameter_description", "Opens the filter"),
        ("cc_msb", "5"),
        ("cc_lsb", "37"),
        ("cc_min_value", "0"),
        ("cc_max_value", "100"),
        ("cc_default_value", "64"),
        ("nrpn_msb", "1"),
        ("nrpn_lsb", "2"),
        ("nrpn_min_value", "0"),
        ("nrpn_max_value", "1000"),
        ("orientation", "centered"),
        ("notes", "a note"),
        ("usage", "0 closed"),
    ]
}

/// The full row with these cells instead.
fn with(cells: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
    let mut out = full();
    for (column, value) in cells {
        match out.iter_mut().find(|(c, _)| c == column) {
            Some(cell) => cell.1 = value,
            None => out.push((column, value)),
        }
    }
    out
}

/// A file of these rows, and the line whose cell of the column is looked at.
type Case = (Vec<Vec<(&'static str, &'static str)>>, usize);

/// For a column: the files that fill it.
fn cases(column: &str) -> Vec<Case> {
    let one = |cells: &[(&'static str, &'static str)]| (vec![with(cells)], 2);
    // A second row of other messages unless the cells say otherwise.
    let second = |cells: &[(&'static str, &'static str)]| {
        let mut other = with(&[("cc_msb", "6"), ("cc_lsb", "38"), ("nrpn_lsb", "3")]);
        for (c, v) in cells {
            other.iter_mut().find(|(k, _)| k == c).unwrap().1 = v;
        }
        (vec![full(), other], 3)
    };
    let full = || (vec![full()], 2);
    // The NRPN alone, its default in its own column.
    let nrpn_alone = |default: &'static str| {
        one(&[("cc_msb", ""), ("cc_lsb", ""), ("cc_min_value", ""), ("cc_max_value", ""), ("cc_default_value", ""), ("nrpn_default_value", default)])
    };
    match column {
        "manufacturer" => vec![full(), second(&[("manufacturer", "Other")])],
        "device" => vec![full(), second(&[("device", "Other")])],
        "cc_msb" => vec![full(), second(&[("cc_msb", "5"), ("cc_lsb", "")])],
        "cc_lsb" => vec![full(), one(&[("cc_lsb", "5")])],
        "cc_min_value" => vec![full(), one(&[("cc_max_value", ""), ("nrpn_min_value", ""), ("nrpn_max_value", "")])],
        "cc_max_value" => vec![full(), one(&[("cc_min_value", ""), ("nrpn_min_value", ""), ("nrpn_max_value", "")])],
        "cc_default_value" => vec![full(), one(&[("cc_default_value", "500")])],
        "nrpn_msb" => vec![full(), second(&[("nrpn_lsb", "2")])],
        "nrpn_lsb" => vec![full(), second(&[("nrpn_msb", ""), ("nrpn_lsb", "9")])],
        "nrpn_min_value" => vec![full(), one(&[("nrpn_max_value", "")])],
        "nrpn_max_value" => vec![full(), one(&[("nrpn_min_value", "")])],
        "nrpn_default_value" => vec![nrpn_alone("500"), nrpn_alone("5000")],
        "orientation" => vec![full(), one(&[("orientation", "sideways")])],
        _ => vec![full()],
    }
}

/// The fates the row of a column may say.
fn fates(column: &str) -> &'static [&'static str] {
    match column {
        "section" | "parameter_name" | "parameter_description" | "notes" | "usage" => &["carried"],
        // The loss of half an NRPN is at the cell that is empty.
        "nrpn_lsb" => &["carried", "conditional"],
        _ => &["conditional"],
    }
}

/// A value of the column other than that of the full row.
fn other_value(column: &str) -> &'static str {
    match column {
        "nrpn_lsb" => "7",
        _ => "Other",
    }
}

fn imported(rows: &[Vec<(&str, &str)>]) -> (Value, Vec<osd_formats::Loss>) {
    let rows: Vec<Vec<String>> = rows.iter().map(|r| row(r)).collect();
    let t = import(&csv(&rows), Format::MidiCsv, &source()).unwrap_or_else(|f| panic!("{f}: {rows:?}"));
    let d = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(t.value)))).unwrap();
    (d, t.lost)
}

fn check(column: &str) {
    let line = row_of("midi-csv", TABLE, column);
    let fate = line.get(1).map(String::as_str).unwrap_or_default();
    assert!(fates(column).contains(&fate), "docs/midi-csv.md, `{column}`: the fate is one of {:?}, not {fate:?}", fates(column));
    assert!(line.get(2).is_some_and(|r| !r.is_empty()), "docs/midi-csv.md, `{column}`: no reason or note");
    let named: Vec<bool> = cases(column)
        .iter()
        .map(|(rows, n)| {
            let (_, lost) = imported(rows);
            lost.iter().any(|l| l.place == format!("line {n}, column {column}") || l.place == format!("line {n}"))
        })
        .collect();
    if fate == "conditional" {
        assert!(named.contains(&true) && named.contains(&false), "{column} is said conditional and does not show both cases: {named:?}");
    } else {
        assert!(named.iter().all(|n| !*n), "{column} is said {fate} and is in the losses: {named:?}");
        let (before, _) = imported(&[full()]);
        let (after, _) = imported(&[with(&[(HEADER.iter().find(|h| **h == column).unwrap(), other_value(column))])]);
        assert_ne!(before, after, "{column}: changing the cell changes nothing");
    }
}

macro_rules! rows {
    ($($test:ident => $column:literal,)+) => {
        const TESTED: &[&str] = &[$($column),+];

        $(
            #[test]
            fn $test() {
                check($column);
            }
        )+
    };
}

#[test]
fn each_column_of_the_header_has_its_test_and_every_file_of_a_test_imports() {
    assert_eq!(TESTED, HEADER);
    for column in HEADER {
        for (rows, _) in cases(column) {
            imported(&rows);
        }
    }
}

rows!(
    the_row_of_manufacturer_says_what_the_import_does => "manufacturer",
    the_row_of_device_says_what_the_import_does => "device",
    the_row_of_section_says_what_the_import_does => "section",
    the_row_of_parameter_name_says_what_the_import_does => "parameter_name",
    the_row_of_parameter_description_says_what_the_import_does => "parameter_description",
    the_row_of_cc_msb_says_what_the_import_does => "cc_msb",
    the_row_of_cc_lsb_says_what_the_import_does => "cc_lsb",
    the_row_of_cc_min_value_says_what_the_import_does => "cc_min_value",
    the_row_of_cc_max_value_says_what_the_import_does => "cc_max_value",
    the_row_of_cc_default_value_says_what_the_import_does => "cc_default_value",
    the_row_of_nrpn_msb_says_what_the_import_does => "nrpn_msb",
    the_row_of_nrpn_lsb_says_what_the_import_does => "nrpn_lsb",
    the_row_of_nrpn_min_value_says_what_the_import_does => "nrpn_min_value",
    the_row_of_nrpn_max_value_says_what_the_import_does => "nrpn_max_value",
    the_row_of_nrpn_default_value_says_what_the_import_does => "nrpn_default_value",
    the_row_of_orientation_says_what_the_import_does => "orientation",
    the_row_of_notes_says_what_the_import_does => "notes",
    the_row_of_usage_says_what_the_import_does => "usage",
);
