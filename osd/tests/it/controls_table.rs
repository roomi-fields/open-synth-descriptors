// @lit spec/0.6.md schemas/0.6/device.schema.json
//! spec/0.6.md holds one table, `Property | OSD field | Reason`, one row per property of
//! MIDI-CI Property Exchange Controller Resources (M2-117-UM v1.0, tables 6 and 12). A property
//! without a row fails, and so does a carried field the control entries of the 0.6 schema do not hold.

use crate::common;

use serde_json::Value;
use std::collections::BTreeSet;

const PROPERTIES: [&str; 15] = [
    "title", "description", "channel", "ctrlType", "ctrlIndex", "priority", "default", "transmit", "recognize", "numSigBits", "paramPath",
    "typeHint", "ctrlMapId", "stepCount", "minMax",
];

/// The backticked words of a cell.
fn ticks(cell: &str) -> Vec<String> {
    cell.split('`').skip(1).step_by(2).map(str::to_string).collect()
}

/// The rows of the table: the cells of each line after the header `Property | OSD field | Reason`.
fn rows() -> Vec<[String; 3]> {
    let text = std::fs::read_to_string(common::repo().join("spec/0.6.md")).unwrap();
    let mut in_table = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let cells: Vec<String> = line.trim().trim_matches('|').split('|').map(|c| c.trim().to_string()).collect();
        if cells.len() == 3 && cells[0] == "Property" && cells[1] == "OSD field" && cells[2] == "Reason" {
            in_table = true;
        } else if in_table && line.trim_start().starts_with('|') {
            if cells.len() == 3 && !cells[0].starts_with("---") && !cells[0].starts_with(":-") {
                out.push([cells[0].clone(), cells[1].clone(), cells[2].clone()]);
            }
        } else if in_table {
            break;
        }
    }
    out
}

/// The properties of every control entry (CC and param) of the 0.6 device schema.
fn entry_fields() -> BTreeSet<String> {
    fn walk(v: &Value, out: &mut BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(Value::Object(p)) = o.get("properties") {
                    if p.contains_key("osc") && (p.contains_key("cc") || p.contains_key("p")) {
                        out.extend(p.keys().cloned());
                    }
                }
                o.values().for_each(|c| walk(c, out));
            }
            Value::Array(a) => a.iter().for_each(|c| walk(c, out)),
            _ => {}
        }
    }
    let text = std::fs::read_to_string(common::repo().join("schemas/0.6/device.schema.json")).unwrap();
    let mut out = BTreeSet::new();
    walk(&serde_json::from_str(&text).unwrap(), &mut out);
    out
}

#[test]
fn the_controls_table_has_a_row_for_each_property_of_table_5() {
    let rows = rows();
    let have: Vec<String> = rows.iter().map(|r| r[0].clone()).collect();
    let missing: Vec<String> = PROPERTIES.iter().filter(|p| !have.contains(&format!("`{p}`"))).map(|p| format!("{p}: no row")).collect();
    common::report("tables 6 and 12 properties without a row", missing);
    assert_eq!(rows.len(), PROPERTIES.len(), "one row per property, no more: {have:?}");
}

#[test]
fn each_row_names_a_field_or_not_carried_and_gives_its_reason() {
    let mut failures = Vec::new();
    for [property, field, reason] in rows() {
        if reason.is_empty() {
            failures.push(format!("{property}: no reason"));
        }
        if !field.contains('`') && field != "not carried" {
            failures.push(format!("{property}: the field is neither in backticks nor 'not carried': {field}"));
        }
    }
    common::report("rows without field or reason", failures);
}

#[test]
fn each_carried_field_is_a_field_of_the_control_entries_of_the_0_6_schema() {
    let held = entry_fields();
    let mut failures = Vec::new();
    for [property, field, _] in rows() {
        for f in ticks(&field) {
            if !held.contains(&f) {
                failures.push(format!("{property}: `{f}` is not a field of a control entry"));
            }
        }
    }
    common::report("carried fields the schema does not hold", failures);
}

#[test]
fn the_fields_added_for_the_standard_are_the_ones_the_table_names() {
    let rows = rows();
    let field = |p: &str| rows.iter().find(|r| r[0] == format!("`{p}`")).map(|r| r[1].clone()).unwrap_or_default();
    assert_eq!(field("priority"), "`priority`");
    assert_eq!(field("stepCount"), "`step_count`");
    assert_eq!(field("transmit"), "`transmit`");
    assert_eq!(field("recognize"), "`recognize`");
    assert_eq!(field("default"), "`default`");
    assert_eq!(field("minMax"), "`range`");
    let map = rows.iter().find(|r| r[0] == "`ctrlMapId`").unwrap();
    assert!(map[1].contains("`values`") && format!("{} {}", map[1], map[2]).contains("carried by `values`"), "{map:?}");
}

#[test]
fn the_controls_doc_gives_the_expects_and_type_hint_correspondence_both_ways() {
    let text = std::fs::read_to_string(common::repo().join("spec/0.6.md")).unwrap();
    for pair in ["continuous", "switch", "toggle", "momentary", "discrete", "valueSelect", "trigger", "clock", "relative"] {
        assert!(text.contains(pair), "spec/0.6.md does not mention {pair}");
    }
    for field in ["transmit", "recognize", "values"] {
        assert!(text.contains(&format!("`{field}`")), "{field}");
    }
}

#[test]
fn the_channel_row_says_the_two_countings_and_the_conversion() {
    let rows = rows();
    let row = rows.iter().find(|r| r[0] == "`channel`").unwrap().join(" ");
    assert!(row.contains("1-256"), "{row}");
    assert!(row.contains("0 to 15"), "{row}");
    assert!(row.contains("Group"), "{row}");
}

#[test]
fn the_default_and_minmax_rows_say_the_32_bit_scale_with_its_norm_and_the_conversion() {
    let rows = rows();
    for name in ["default", "minMax"] {
        let row = rows.iter().find(|r| r[0] == format!("`{name}`")).unwrap().join(" ");
        assert!(row.contains("unsigned 32"), "{name}: {row}");
        assert!(row.contains("M2-115-U"), "{name}: {row}");
        assert!(row.contains("Min-Center-Max") && row.contains("Zero-Extension"), "{name}: {row}");
    }
}

#[test]
fn the_relative_section_names_the_relative_registered_and_assignable_controllers_and_no_longer_leaves_the_coding_to_the_device() {
    let text = std::fs::read_to_string(common::repo().join("spec/0.6.md")).unwrap();
    assert!(text.contains("Relative Registered"), "Relative Registered Controllers");
    assert!(text.contains("Assignable Controllers"), "Relative Assignable Controllers");
    assert!(!text.contains("that coding is the device's"), "the norm does say it");
}

#[test]
fn a_row_that_says_same_as_the_norm_cites_the_norm() {
    for r in &rows() {
        let joined = r.join(" ");
        if joined.to_lowercase().contains("same") {
            assert!(joined.contains("M2-117-UM"), "{}: says the same as the norm without citing it: {joined}", r[0]);
        }
    }
}
