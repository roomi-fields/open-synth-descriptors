//! The named values of a control cross MIDNAM as a ValueNameList (Value Number and Name) the
//! control uses, and the span MIDI fixes for a controller is the range of a control that gives
//! none.

use crate::common::midnam::*;
use crate::common::xml::{document_text, Node};
use crate::common::{electra::json_of, resolved, source};
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

/// The (Number, Name) pairs of the list a control uses, inline or by name, sorted.
fn names_of(root: &Node, control: &Node) -> Vec<(String, String)> {
    let values = control.kid("Values").expect("a control has its Values");
    let list: &Node = if let Some(inline) = values.kid("ValueNameList") {
        inline
    } else {
        let uses = values.kid("UsesValueNameList").expect("the control uses a list of value names");
        let name = uses.attr("Name").expect("the reference names its list");
        root.all("ValueNameList").into_iter().find(|l| l.attr("Name") == Some(name)).unwrap_or_else(|| panic!("no ValueNameList named {name:?}"))
    };
    let mut out: Vec<(String, String)> =
        list.kids("Value").iter().map(|v| (v.attr("Number").unwrap().to_string(), v.attr("Name").unwrap().to_string())).collect();
    out.sort();
    out
}

/// The `values` of an entry of 0.7, a list of `{value, title}`, as (value, title) sorted.
fn listed(values: &Value) -> Vec<(i64, String)> {
    let mut out: Vec<(i64, String)> = values
        .as_array()
        .unwrap_or_else(|| panic!("the values are a list of {{value, title}}: {values}"))
        .iter()
        .map(|v| (v["value"].as_i64().unwrap(), v["title"].as_str().unwrap().to_string()))
        .collect();
    out.sort();
    out
}

fn named_values(items: &[(i64, &str)]) -> Vec<(i64, String)> {
    let mut out: Vec<(i64, String)> = items.iter().map(|(n, t)| (*n, t.to_string())).collect();
    out.sort();
    out
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = items.iter().map(|(n, l)| (n.to_string(), l.to_string())).collect();
    out.sort();
    out
}

fn controls(root: &Node) -> Vec<&Node> {
    root.all("Control")
}

fn entries(d: &Value) -> Vec<Value> {
    d["cc_params"]["entries"].as_array().cloned().unwrap_or_default()
}

fn value(number: &str, name: &str) -> Node {
    Node::new("Value", &[("Number", number), ("Name", name)])
}

/// A document of the MIDNAM kind: the given controls in its control list, and the given lists
/// of value names beside it.
fn document_of(controls: Vec<Node>, lists: Vec<Node>) -> String {
    let mut doc = rich_document();
    let master = doc.at_mut(&[1]);
    assert_eq!(master.at_mut(&[5]).name, "ControlNameList");
    master.at_mut(&[5]).children = controls;
    master.children.extend(lists);
    document_text(&doc)
}

fn control_with(kind: &str, number: &str, name: &str, min: &str, max: &str, children: Vec<Node>) -> Node {
    let mut values = Node::new("Values", &[("Min", min), ("Max", max)]);
    values.children = children;
    Node::new("Control", &[("Type", kind), ("Number", number), ("Name", name)]).with(values)
}

fn line_of(text: &str, needle: &str) -> usize {
    text.lines().position(|l| l.contains(needle)).unwrap_or_else(|| panic!("{needle} is not in the document")) + 1
}

fn lost_on_line(lost: &[osd_formats::Loss], line: usize) -> Vec<&osd_formats::Loss> {
    lost.iter().filter(|l| l.place == format!("line {line}") || l.place.starts_with(&format!("line {line},"))).collect()
}

// ---- export ----

#[test]
fn the_values_of_a_control_are_written_as_a_list_of_value_names_the_control_uses() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/osc/wave", "cc": 70, "range": [0, 2], "values": {"Saw": 2, "Sine": 0, "Square": 1}},
    ]}}));
    let root = parsed(&exported(&d).0);
    let control = controls(&root)[0];
    assert_eq!(names_of(&root, control), pairs(&[("0", "Sine"), ("1", "Square"), ("2", "Saw")]));
    let range = control.kid("Values").unwrap();
    assert_eq!((range.attr("Min"), range.attr("Max")), (Some("0"), Some("2")));
}

#[test]
fn a_14_bit_an_nrpn_and_an_rpn_control_write_their_values_too() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 5, "cc_lsb": 37, "values": {"Low": 0, "High": 16383}},
        {"osc": "/b", "nrpn_msb": 2, "nrpn_lsb": 44, "value_bits": 14, "values": {"Off": 0, "On": 1}},
        {"osc": "/c", "rpn_msb": 0, "rpn_lsb": 1, "value_bits": 14, "values": {"Down": 0, "Up": 5}},
    ]}}));
    let root = parsed(&exported(&d).0);
    let got: Vec<_> = controls(&root).iter().map(|c| names_of(&root, c)).collect();
    assert_eq!(got, vec![pairs(&[("0", "Low"), ("16383", "High")]), pairs(&[("0", "Off"), ("1", "On")]), pairs(&[("0", "Down"), ("5", "Up")])]);
}

#[test]
fn a_control_without_values_writes_no_list_of_value_names_and_no_reference() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127]}]}}));
    let root = parsed(&exported(&d).0);
    assert!(root.all("ValueNameList").is_empty() && root.all("UsesValueNameList").is_empty() && root.all("Value").is_empty());
}

#[test]
fn equal_lists_of_values_are_written_once_and_shared_and_different_ones_apart() {
    let wave = json!({"Saw": 2, "Sine": 0, "Square": 1});
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/one", "cc": 10, "range": [0, 2], "values": wave},
        {"osc": "/two", "cc": 11, "range": [0, 2], "values": wave},
        {"osc": "/three", "cc": 12, "range": [0, 1], "values": {"Off": 0, "On": 1}},
        {"osc": "/four", "cc": 13, "range": [0, 2], "values": wave},
    ]}}));
    let root = parsed(&exported(&d).0);
    assert_eq!(root.all("ValueNameList").len(), 2, "one list for the three equal ones, one for the other");
    let got: Vec<_> = controls(&root).iter().map(|c| names_of(&root, c)).collect();
    let saw = pairs(&[("0", "Sine"), ("1", "Square"), ("2", "Saw")]);
    assert_eq!(got, vec![saw.clone(), saw.clone(), pairs(&[("0", "Off"), ("1", "On")]), saw]);
}

#[test]
fn two_labels_on_the_same_integer_are_both_written() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 3], "values": {"Low": 0, "Mid": 1, "Middle": 1, "High": 3}}]}}));
    let root = parsed(&exported(&d).0);
    assert_eq!(names_of(&root, controls(&root)[0]), pairs(&[("0", "Low"), ("1", "Mid"), ("1", "Middle"), ("3", "High")]));
}

#[test]
fn a_label_with_xml_signs_is_escaped_and_read_back_as_it_was() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 1], "values": {"A & <B> \"c\"": 0, "Plain": 1}}]}}));
    let root = parsed(&exported(&d).0);
    assert_eq!(names_of(&root, controls(&root)[0]), pairs(&[("0", "A & <B> \"c\""), ("1", "Plain")]));
}

#[test]
fn a_control_with_values_and_no_range_has_the_span_of_its_type_and_its_values() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "values": {"Off": 0, "On": 127}}]}}));
    let root = parsed(&exported(&d).0);
    let control = controls(&root)[0];
    let range = control.kid("Values").unwrap();
    assert_eq!((range.attr("Min"), range.attr("Max")), (Some("0"), Some("127")));
    assert_eq!(names_of(&root, control), pairs(&[("0", "Off"), ("127", "On")]));
}

#[test]
fn the_values_of_a_control_are_carried_and_not_in_the_losses() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 1], "values": {"Off": 0, "On": 1}}]}}));
    let (_, lost) = exported(&d);
    assert!(lost_at(&lost, "/cc_params/entries/0/values").is_empty(), "{lost:?}");
}

// ---- import ----

#[test]
fn a_list_of_value_names_inside_the_values_becomes_the_values_of_the_entry() {
    let text = document_of(vec![control_with("7bit", "70", "Wave", "0", "2", vec![Node::new("ValueNameList", &[]).with(value("0", "Sine")).with(value("2", "Saw"))])], vec![]);
    let (d, lost) = imported(&text);
    assert_eq!(listed(&entries(&d)[0]["values"]), named_values(&[(0, "Sine"), (2, "Saw")]));
    assert!(lost_on_line(&lost, line_of(&text, "Name=\"Saw\"")).is_empty(), "{lost:?}");
}

#[test]
fn a_control_that_uses_a_list_by_name_reads_that_list_and_two_controls_may_share_one() {
    let list = Node::new("ValueNameList", &[("Name", "Wave")]).with(value("0", "Sine")).with(value("1", "Square")).with(value("2", "Saw"));
    let uses = || vec![Node::new("UsesValueNameList", &[("Name", "Wave")])];
    let text = document_of(
        vec![control_with("7bit", "70", "Wave A", "0", "2", uses()), control_with("NRPN", "300", "Wave B", "0", "2", uses())],
        vec![list],
    );
    let (d, lost) = imported(&text);
    let want = named_values(&[(0, "Sine"), (1, "Square"), (2, "Saw")]);
    assert_eq!((listed(&entries(&d)[0]["values"]), listed(&entries(&d)[1]["values"])), (want.clone(), want));
    assert!(lost_on_line(&lost, line_of(&text, "<ValueNameList")).is_empty(), "a list a control uses is not lost: {lost:?}");
}

#[test]
fn a_list_of_value_names_no_control_uses_is_named_in_the_losses_at_its_line() {
    let used = Node::new("ValueNameList", &[("Name", "Used")]).with(value("0", "Off"));
    let unused = Node::new("ValueNameList", &[("Name", "Orphan")]).with(value("0", "Nothing"));
    let text = document_of(vec![control_with("7bit", "70", "Wave", "0", "1", vec![Node::new("UsesValueNameList", &[("Name", "Used")])])], vec![used, unused]);
    let (d, lost) = imported(&text);
    assert_eq!(listed(&entries(&d)[0]["values"]), named_values(&[(0, "Off")]));
    let orphan = lost_on_line(&lost, line_of(&text, "Name=\"Orphan\""));
    assert_eq!(orphan.len(), 1, "{lost:?}");
    assert!(!orphan[0].reason.is_empty());
    assert!(lost_on_line(&lost, line_of(&text, "Name=\"Used\"")).is_empty(), "{lost:?}");
}

#[test]
fn a_reference_to_a_list_the_document_does_not_hold_is_named_in_the_losses_and_gives_no_values() {
    let text = document_of(vec![control_with("7bit", "70", "Wave", "0", "1", vec![Node::new("UsesValueNameList", &[("Name", "Nowhere")])])], vec![]);
    let (d, lost) = imported(&text);
    assert!(entries(&d)[0].get("values").is_none(), "{d}");
    let found = lost_on_line(&lost, line_of(&text, "<UsesValueNameList"));
    assert_eq!(found.len(), 1, "{lost:?}");
    assert!(!found[0].reason.is_empty());
}

#[test]
fn a_value_whose_number_is_not_a_whole_number_is_named_with_its_reason_and_the_others_are_carried() {
    let list = Node::new("ValueNameList", &[]).with(value("0", "Zero")).with(value("1.5", "Half")).with(value("2", "Two"));
    let text = document_of(vec![control_with("7bit", "70", "Wave", "0", "2", vec![list])], vec![]);
    let (d, lost) = imported(&text);
    assert_eq!(listed(&entries(&d)[0]["values"]), named_values(&[(0, "Zero"), (2, "Two")]));
    let found = lost_on_line(&lost, line_of(&text, "Name=\"Half\""));
    assert_eq!(found.len(), 1, "{lost:?}");
    assert!(found[0].reason.contains("whole number"), "{:?}", found[0]);
    assert!(lost_on_line(&lost, line_of(&text, "Name=\"Zero\"")).is_empty() && lost_on_line(&lost, line_of(&text, "Name=\"Two\"")).is_empty(), "{lost:?}");
}

#[test]
fn a_list_with_no_value_it_can_carry_gives_an_entry_without_values() {
    let list = Node::new("ValueNameList", &[]).with(value("1.5", "Half"));
    let text = document_of(vec![control_with("7bit", "70", "Wave", "0", "2", vec![list])], vec![]);
    let (d, _) = imported(&text);
    let values = entries(&d)[0].get("values").cloned();
    assert!(values.is_none() || values == Some(json!([])), "{d}");
}

#[test]
fn a_control_without_a_list_of_value_names_has_no_values() {
    let (d, _) = imported(&control_document("7bit", "74", "0", "127", "Cutoff", None));
    assert!(entries(&d)[0].get("values").is_none(), "{d}");
}

#[test]
fn two_names_on_the_same_number_are_both_read() {
    let list = Node::new("ValueNameList", &[]).with(value("1", "Mid")).with(value("1", "Middle"));
    let text = document_of(vec![control_with("7bit", "70", "Wave", "0", "3", vec![list])], vec![]);
    let (d, _) = imported(&text);
    assert_eq!(listed(&entries(&d)[0]["values"]), named_values(&[(1, "Mid"), (1, "Middle")]));
}

// ---- the span of a controller ----

#[test]
fn a_range_equal_to_the_span_of_its_type_is_not_written_back_and_is_no_loss() {
    for (kind, number, max) in [("7bit", "74", "127"), ("14bit", "5", "16383"), ("NRPN", "300", "16383"), ("RPN", "130", "16383")] {
        let text = control_document(kind, number, "0", max, "Some Control", Some("5"));
        let (d, lost) = imported(&text);
        let e = &entries(&d)[0];
        assert!(e["ways"][0].get("range").is_none(), "{kind}: {e}");
        assert_eq!(e["default"], 5, "{kind}: the default stays");
        assert!(lost_on_line(&lost, line_of(&text, "<Values")).is_empty(), "{kind}: {lost:?}");
    }
}

#[test]
fn a_range_that_is_not_the_span_of_its_type_is_kept() {
    for (kind, number, min, max, want) in [
        ("7bit", "74", "0", "16383", json!([0, 16383])),
        ("7bit", "74", "1", "127", json!([1, 127])),
        ("14bit", "5", "0", "127", json!([0, 127])),
        ("NRPN", "300", "0", "127", json!([0, 127])),
        ("RPN", "130", "0", "16382", json!([0, 16382])),
    ] {
        let (d, _) = imported(&control_document(kind, number, min, max, "Some Control", None));
        assert_eq!(entries(&d)[0]["ways"][0]["range"], want, "{kind} {min}-{max}");
    }
}

#[test]
fn a_control_without_a_range_exports_and_comes_back_equal_to_what_was_exported() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 1},
        {"osc": "/b", "cc": 2, "cc_lsb": 34},
        {"osc": "/c", "nrpn_msb": 1, "nrpn_lsb": 1, "value_bits": 14},
        {"osc": "/d", "rpn_msb": 0, "rpn_lsb": 0, "value_bits": 14},
    ]}}));
    let out = export(&d, Format::Midnam).unwrap();
    let back = json_of(&import(&out.value, Format::Midnam, &source()).unwrap().value);
    for e in entries(&back) {
        assert!(e["ways"][0].get("range").is_none(), "a range the descriptor did not give is not invented: {e}");
    }
}

#[test]
fn a_range_equal_to_the_span_of_its_type_comes_back_without_range_with_no_loss_named() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 1, "range": [0, 127]},
        {"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 1, "value_bits": 14, "range": [0, 16383]},
    ]}}));
    let out = export(&d, Format::Midnam).unwrap();
    let t = import(&out.value, Format::Midnam, &source()).unwrap();
    for e in entries(&json_of(&t.value)) {
        assert!(e["ways"][0].get("range").is_none(), "{e}");
    }
    let values_lines: Vec<usize> = out.value.lines().enumerate().filter(|(_, l)| l.contains("<Values")).map(|(n, _)| n + 1).collect();
    assert_eq!(values_lines.len(), 2);
    for line in values_lines {
        assert!(lost_on_line(&t.lost, line).is_empty(), "{:?}", t.lost);
    }
}

// ---- the round trip ----

#[test]
fn the_values_come_back_the_same_after_an_export_and_an_import() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 70, "range": [0, 2], "values": {"Saw": 2, "Sine": 0, "Square": 1}},
        {"osc": "/b", "cc": 71, "range": [0, 2], "values": {"Saw": 2, "Sine": 0, "Square": 1}},
        {"osc": "/c", "cc": 5, "cc_lsb": 37, "values": {"Low": 0, "High": 16383}},
        {"osc": "/d", "nrpn_msb": 2, "nrpn_lsb": 44, "value_bits": 14, "range": [0, 3], "values": {"A": 0, "B": 1, "C": 1}},
        {"osc": "/e", "cc": 72, "range": [0, 127]},
    ]}}));
    let out = export(&d, Format::Midnam).unwrap();
    let back = json_of(&import(&out.value, Format::Midnam, &source()).unwrap().value);
    // MIDNAM lists a value name by its number: the lists are compared in the order of the numbers.
    let by_number = |e: &Value| {
        e.get("values").cloned().map(|mut v| {
            v.as_array_mut().unwrap().sort_by_key(|n| n["value"].as_i64());
            v
        })
    };
    let got: Vec<Option<Value>> = entries(&back).iter().map(by_number).collect();
    let want: Vec<Option<Value>> = entries(&json_of(&d.device)).iter().map(by_number).collect();
    assert_eq!(got, want);
}

#[test]
fn a_second_round_trip_of_values_gives_the_same_lists() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 70, "range": [0, 2], "values": {"Saw": 2, "Sine": 0}},
        {"osc": "/b", "cc": 71, "range": [0, 2], "values": {"Saw": 2, "Sine": 0}},
    ]}}));
    let first = export(&d, Format::Midnam).unwrap().value;
    let again = export(&resolved(import(&first, Format::Midnam, &source()).unwrap().value), Format::Midnam).unwrap().value;
    let lists = |t: &str| parsed(t).all("ValueNameList").iter().map(|l| l.to_xml()).collect::<Vec<_>>();
    assert_eq!(lists(&first), lists(&again));
    assert_eq!(lists(&first).len(), 1);
}
