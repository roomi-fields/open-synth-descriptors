//! The document of destiny docs/oscquery.md: one line per attribute of a node of the namespace
//! and per key of a range, with its fate, its reason and the piece of the proposal; and the
//! rules of the checker of an export.

use crate::common::oscquery::*;
use crate::common::partner::outside_section;
use serde_json::json;

const DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/oscquery.md");

fn doc_lines() -> Vec<Vec<String>> {
    let page = std::fs::read_to_string(DOC).unwrap_or_else(|e| panic!("{DOC}: {e}"));
    outside_section(&page)
        .into_iter()
        .filter(|l| l.trim_start().starts_with('|'))
        .map(|l| l.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect())
        .collect()
}

fn documented() -> Vec<String> {
    ATTRIBUTES.iter().chain(RANGE_KEYS.iter()).map(|s| s.to_string()).collect()
}

#[test]
fn every_attribute_of_a_node_and_every_key_of_a_range_has_a_line_with_its_fate_its_reason_and_the_piece() {
    let lines = doc_lines();
    for name in documented() {
        let line = lines.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no line for {name} in {DOC}"));
        assert!(line.len() >= 4, "{name}: a line has the name, the fate, the reason and the piece");
        assert!(line[1] == "carried" || line[1] == "lost", "{name}: the fate is carried or lost, not {:?}", line[1]);
        assert!(!line[2].is_empty(), "{name}: no reason");
        assert!(line[3].contains("OSCQueryProposal"), "{name}: the piece of the proposal is not cited: {:?}", line[3]);
    }
}

#[test]
fn the_document_lists_no_attribute_the_proposal_does_not_have() {
    let known = documented();
    for line in doc_lines() {
        let name = line[0].as_str();
        if name == "Attribute" || name == "Name" || name.starts_with('-') || name.is_empty() {
            continue;
        }
        assert!(known.iter().any(|k| k == name), "{name} is in {DOC} and not in the proposal");
    }
}

#[test]
fn the_fate_is_carried_for_the_address_the_types_the_ranges_and_the_names_and_lost_for_the_rest() {
    let lines = doc_lines();
    for name in documented() {
        let line = lines.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no line for {name} in {DOC}"));
        let expected = if CARRIED.contains(&name.as_str()) { "carried" } else { "lost" };
        assert_eq!(line[1], expected, "{name}");
    }
}

#[test]
fn the_line_of_vals_says_the_list_of_permitted_values_is_written_and_what_has_no_place_in_it() {
    let lines = doc_lines();
    let line = lines.iter().find(|l| l[0] == "VALS").expect("a line for VALS");
    let reason = line[2].to_lowercase();
    assert!(reason.contains("label"), "the labels of the values have no place in a VALS: {reason:?}");
    assert!(!reason.contains("no field for the named values"), "the line still says the export writes nothing: {reason:?}");
}

/// The sentences of a section of the page, the section named by its title.
fn sentences(title: &str) -> Vec<String> {
    let page = std::fs::read_to_string(DOC).unwrap_or_else(|e| panic!("{DOC}: {e}"));
    let heading = format!("## {title}");
    let body: Vec<&str> = page.lines().skip_while(|l| l.trim() != heading).skip(1).take_while(|l| !l.starts_with("## ")).collect();
    assert!(!body.is_empty(), "no section {title:?} in {DOC}");
    body.iter().flat_map(|l| l.split(". ")).map(str::to_string).collect()
}

/// A sentence of the section holds every one of these words.
fn says(title: &str, words: &[&str]) {
    let all = sentences(title);
    assert!(all.iter().any(|s| words.iter().all(|w| s.contains(w))), "no sentence of {title:?} holds all of {words:?}");
}

/// The section holds every one of these words, in any case.
fn holds(title: &str, words: &[&str]) {
    let text = sentences(title).join(" ").to_lowercase();
    for word in words {
        assert!(text.contains(&word.to_lowercase()), "the section {title:?} does not say {word:?}");
    }
}

#[test]
fn the_page_names_the_parameters_among_the_addresses_the_export_writes() {
    says("What the export writes", &["command", "trigger", "param"]);
}

#[test]
fn the_page_says_a_trigger_is_an_impulse_method_with_no_velocity_argument() {
    holds("What the export writes", &["`I`", "velocity"]);
}

#[test]
fn the_page_says_a_node_is_a_method_or_a_container_never_both_and_the_trigger_left_out_is_a_loss() {
    says("What the export writes", &["method", "container", "both"]);
    says("What the export writes", &["trigger", "loss"]);
}

#[test]
fn the_page_says_the_method_of_a_trigger_does_not_say_which_note_of_a_range_to_play() {
    holds("What the export writes", &["which note"]);
}

#[test]
fn the_page_lists_the_arguments_of_the_services_triggers_among_the_differences_with_the_reason() {
    holds("Differences from the service", &["velocity", "note", "channel", "OVERLOADS", "no field"]);
}

#[test]
fn the_page_says_an_impulse_method_is_read_back_as_a_command() {
    says("What the import reads", &["`I`", "command"]);
}

#[test]
fn the_line_of_type_says_the_impulse_tag_is_read_as_a_command_and_not_kept() {
    let lines = doc_lines();
    let line = lines.iter().find(|l| l[0] == "TYPE").expect("a line for TYPE");
    assert!(line[2].contains("`I`") && line[2].contains("command"), "{:?}", line[2]);
}

#[test]
fn the_checker_accepts_a_namespace_of_the_proposal_and_refuses_each_kind_of_break() {
    let ok = namespace(vec![("a", method("/synth/a", "if", &[Some((0, 1)), None]))]);
    assert_eq!(violations(&ok.to_string()), Vec::<String>::new());
    let mut cases = Vec::new();
    let mut bad = ok.clone();
    bad["CONTENTS"]["synth"]["CONTENTS"]["a"]["FULL_PATH"] = json!("/synth/b");
    cases.push(("a FULL_PATH that is not the path", bad));
    let mut bad = ok.clone();
    bad["CONTENTS"]["synth"]["CONTENTS"]["a"]["TYPE"] = json!("iz");
    cases.push(("a type that is no tag", bad));
    let mut bad = ok.clone();
    bad["CONTENTS"]["synth"]["CONTENTS"]["a"]["RANGE"] = json!([null]);
    cases.push(("a RANGE of the wrong length", bad));
    let mut bad = ok.clone();
    bad["CONTENTS"]["synth"]["CONTENTS"]["a"]["ACCESS"] = json!(9);
    cases.push(("an ACCESS out of 0 to 3", bad));
    let mut bad = ok.clone();
    bad["CONTENTS"]["synth"]["CONTENTS"]["a"]["BOGUS"] = json!(1);
    cases.push(("an attribute the proposal does not have", bad));
    let mut bad = ok.clone();
    bad["CONTENTS"]["synth"]["TYPE"] = json!("i");
    cases.push(("a container with a type", bad));
    let mut bad = ok.clone();
    bad["CONTENTS"]["synth"]["CONTENTS"]["a"]["RANGE"] = json!([{"MIN": 0, "MAX": 1, "VALS": {"Off": 0}}, null]);
    cases.push(("a VALS that is not a list", bad));
    let with_vals = namespace(vec![("a", json!({"FULL_PATH": "/synth/a", "ACCESS": 2, "TYPE": "i", "RANGE": [{"MIN": 0, "MAX": 2, "VALS": [0, 2]}]}))]);
    assert_eq!(violations(&with_vals.to_string()), Vec::<String>::new());
    for (what, ns) in cases {
        assert!(!violations(&ns.to_string()).is_empty(), "{what}");
    }
    assert!(!violations("{").is_empty());
}
