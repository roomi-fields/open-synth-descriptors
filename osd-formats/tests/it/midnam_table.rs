//! The verifier of a MIDNAM export is a table written from the DTD: it holds a line for each
//! declaration of the DTD files and none other; each line refuses its altered element; and the
//! document of destiny, docs/midnam.md, has a line for each element and attribute.

use crate::common::midnam::*;
use crate::common::partner::outside_section;
use crate::common::xml::Node;
use std::collections::BTreeSet;

const DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/midnam.md");

/// The elements and attributes (`Element@Attribute`) the DTD files declare.
fn declared_names() -> BTreeSet<String> {
    let d = declarations();
    let mut out: BTreeSet<String> = d.elements.keys().cloned().collect();
    for (element, attrs) in &d.attributes {
        for a in attrs {
            out.insert(format!("{element}@{}", a.split(' ').next().unwrap()));
        }
    }
    out
}

fn doc_lines() -> Vec<Vec<String>> {
    let page = std::fs::read_to_string(DOC).unwrap_or_else(|e| panic!("{DOC}: {e}"));
    outside_section(&page)
        .into_iter()
        .filter(|l| l.trim_start().starts_with('|'))
        .map(|l| l.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect())
        .collect()
}

#[test]
fn the_table_has_a_line_for_each_element_the_dtd_declares_and_none_the_dtd_does_not() {
    let d = declarations();
    let table: BTreeSet<&str> = RULES.iter().map(|r| r.0).collect();
    let declared: BTreeSet<&str> = d.elements.keys().map(String::as_str).collect();
    assert_eq!(table.len(), RULES.len(), "an element has two lines");
    assert_eq!(declared.difference(&table).collect::<Vec<_>>(), Vec::<&&str>::new(), "declared by the DTD, without a line");
    assert_eq!(table.difference(&declared).collect::<Vec<_>>(), Vec::<&&str>::new(), "a line without a declaration");
}

#[test]
fn each_line_of_the_table_has_the_content_model_of_its_declaration() {
    let d = declarations();
    for (name, model, _) in RULES {
        assert_eq!(normal(model), d.elements[*name], "{name}");
    }
}

#[test]
fn each_line_of_the_table_has_the_attributes_of_its_declaration_name_type_and_default() {
    let d = declarations();
    for (name, _, attrs) in RULES {
        let table: Vec<String> = attrs.iter().map(|a| normal(a)).collect();
        let declared = d.attributes.get(*name).cloned().unwrap_or_default();
        assert_eq!(table, declared, "{name}");
    }
    for element in d.attributes.keys() {
        assert!(rule(element).is_some(), "{element} has attributes in the DTD and no line");
    }
}

#[test]
fn the_entities_of_the_table_are_those_of_the_dtd_files() {
    let d = declarations();
    for (name, value) in ENTITIES {
        assert_eq!(normal(value), d.entities[*name], "{name}");
    }
    assert_eq!(ENTITIES.len(), d.entities.len(), "an entity of the DTD has no line");
}

#[test]
fn an_element_of_a_rule_with_its_attributes_set_is_accepted_when_its_content_is_empty_or_text() {
    for (name, model, _) in RULES {
        if *model == "EMPTY" || model.starts_with("(#PCDATA") {
            assert_eq!(check_element(&sample(name)), Vec::<String>::new(), "{name}");
        }
    }
}

#[test]
fn an_element_the_dtd_does_not_declare_is_refused() {
    let v = check_element(&Node::new("Bogus", &[]));
    assert!(v.iter().any(|m| m.contains("Bogus") && m.contains("not declared")), "{v:?}");
    for (name, _, _) in RULES {
        let mut n = sample(name);
        n.children.push(Node::new("Bogus", &[]));
        let v = check_element(&n);
        assert!(v.iter().any(|m| m.contains("Bogus") || m.contains("empty")), "{name}: a child Bogus passed: {v:?}");
    }
}

#[test]
fn an_element_without_a_required_attribute_is_refused_for_each_of_its_required_attributes() {
    for (name, _, attrs) in RULES {
        for a in *attrs {
            let (attr, _, default) = attribute_parts(a);
            if default != "#REQUIRED" {
                continue;
            }
            let mut n = sample(name);
            n.attrs.retain(|(k, _)| *k != attr);
            let v = check_element(&n);
            assert!(v.iter().any(|m| m.contains(&attr) && m.contains("missing")), "{name} without {attr}: {v:?}");
        }
    }
}

#[test]
fn an_element_with_an_attribute_the_dtd_does_not_declare_or_a_value_out_of_its_list_is_refused() {
    for (name, _, attrs) in RULES {
        let mut n = sample(name);
        n.attrs.push(("Bogus".into(), "1".into()));
        let v = check_element(&n);
        assert!(v.iter().any(|m| m.contains("Bogus")), "{name}: an attribute Bogus passed");
        for a in *attrs {
            let (attr, ty, _) = attribute_parts(a);
            if ty.starts_with('(') {
                let mut n = sample(name);
                n.attrs.iter_mut().find(|(k, _)| *k == attr).unwrap().1 = "not-in-the-list".into();
                let v = check_element(&n);
                assert!(v.iter().any(|m| m.contains(&attr)), "{name}@{attr}: a value out of the list passed");
            }
        }
    }
}

#[test]
fn an_element_with_a_child_its_content_model_does_not_allow_is_refused_for_each_rule() {
    for (name, model, _) in RULES {
        if *model == "EMPTY" {
            continue;
        }
        let allowed = allowed_children(name);
        let intruder = RULES.iter().map(|r| r.0).find(|n| !allowed.contains(*n)).unwrap();
        let mut n = sample(name);
        n.children.push(Node::new(intruder, &[]));
        let v = check_element(&n);
        assert!(v.iter().any(|m| m.contains(intruder)), "{name} with a child {intruder}: {v:?}");
    }
}

#[test]
fn the_order_and_the_number_of_children_are_checked_by_the_content_model() {
    let kid = |n: &str| sample(n);
    // Author comes first, then the devices.
    let ok = Node::new("MIDINameDocument", &[]).with(kid("Author")).with(Node::new("MasterDeviceNames", &[]));
    assert_eq!(check_element(&ok), Vec::<String>::new());
    let swapped = Node::new("MIDINameDocument", &[]).with(Node::new("MasterDeviceNames", &[])).with(kid("Author"));
    assert!(!check_element(&swapped).is_empty());
    let none = Node::new("MIDINameDocument", &[]).with(kid("Author"));
    assert!(!check_element(&none).is_empty(), "a document needs a device");
    let two = Node::new("MIDINameDocument", &[]).with(kid("Author")).with(kid("Author")).with(Node::new("MasterDeviceNames", &[]));
    assert!(!check_element(&two).is_empty(), "one Author");
    // A control has at most one Values; a list of controls at least one control.
    let mut c = sample("Control");
    c.children = vec![sample("Values"), sample("Values")];
    assert!(!check_element(&c).is_empty());
    assert!(!check_element(&Node::new("ControlNameList", &[])).is_empty());
    assert_eq!(check_element(&Node::new("PatchNameList", &[])), Vec::<String>::new(), "a patch list may be empty");
}

#[test]
fn a_document_valid_by_the_dtd_has_no_violation_and_one_that_is_not_has_some() {
    assert_eq!(violations(&rich_text()), Vec::<String>::new());
    assert!(!violations("<MIDINameDocument>").is_empty(), "not well-formed");
    let no_type = rich_text().replace("PUBLIC \"-//MIDI Manufacturers Association//DTD MIDINameDocument 1.0//EN\"", "SYSTEM");
    assert!(!violations(&no_type).is_empty(), "not the public document type");
    let mut doc = rich_document();
    doc.children.remove(0);
    assert!(!violations(&crate::common::xml::document_text(&doc)).is_empty(), "no Author");
}

#[test]
fn the_document_of_destiny_has_a_line_for_each_element_and_attribute_with_its_fate_its_reason_and_the_piece() {
    let lines = doc_lines();
    for name in declared_names() {
        let line = lines.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no line for {name} in {DOC}"));
        assert!(line.len() >= 4, "{name}: a line has the name, the fate, the reason and the piece");
        assert!(line[1] == "carried" || line[1] == "lost", "{name}: the fate is carried or lost, not {:?}", line[1]);
        assert!(!line[2].is_empty(), "{name}: no reason");
        assert!(line[3].contains("MIDINameDocument10.dtd") || line[3].contains("MIDIEvents10.dtd"), "{name}: the piece of the DTD is not cited: {:?}", line[3]);
    }
}

#[test]
fn the_document_of_destiny_lists_no_element_or_attribute_the_dtd_does_not_declare() {
    let declared = declared_names();
    for line in doc_lines() {
        let name = line[0].as_str();
        if name == "Element" || name == "Name" || name.starts_with('-') || name.is_empty() {
            continue;
        }
        assert!(declared.contains(name), "{name} is in {DOC} and not in the DTD");
    }
}

#[test]
fn the_fate_of_each_element_and_attribute_is_carried_for_what_the_descriptor_holds_and_lost_for_the_rest() {
    let carried: BTreeSet<&str> = [
        "MIDINameDocument", "MasterDeviceNames", "Manufacturer", "Model", "ControlNameList", "Control", "Control@Type", "Control@Number",
        "Control@Name", "Values", "Values@Min", "Values@Max", "Values@Default", "ValueNameList", "Value", "Value@Number", "Value@Name", "NoteNameList", "Note", "Note@Number", "Note@Name",
        "PatchBank", "PatchBank@Name", "PatchNameList", "Patch", "Patch@Number", "Patch@ProgramChange", "MIDICommands", "ControlChange",
        "ControlChange@Control", "ControlChange@Value",
    ]
    .into_iter()
    .collect();
    let lines = doc_lines();
    for name in declared_names() {
        let line = lines.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no line for {name} in {DOC}"));
        let expected = if carried.contains(name.as_str()) { "carried" } else { "lost" };
        assert_eq!(line[1], expected, "{name}");
    }
}
