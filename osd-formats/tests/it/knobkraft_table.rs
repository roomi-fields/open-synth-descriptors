//! The document of the KnobKraft format (docs/knobkraft.md) says, for each function of the
//! interface of an adaptation, whether the export writes it or names it lost, with its piece (the
//! section and version of the programming guide), and what of an adaptation is data of a device
//! and what is code only its host runs.

use crate::common::knobkraft::{exported, device, LEFT_TO_HAND, WRITTEN};
use osd_formats::{export, Format};
use serde_json::json;

fn read(path: &str) -> String {
    let file = format!("{}/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"))
}

/// The cells of the table line of a function: `| `name` | fate | reason | piece |`.
fn line_of(doc: &str, function: &str) -> Vec<String> {
    let start = format!("| `{function}` |");
    let line = doc.lines().find(|l| l.starts_with(&start)).unwrap_or_else(|| panic!("knobkraft.md has no line for {function}"));
    line.trim_matches('|').split('|').map(|c| c.trim().to_string()).collect()
}

#[test]
fn each_function_of_the_interface_has_its_fate_its_reason_and_its_piece_in_the_document() {
    let doc = read("docs/knobkraft.md");
    for (functions, fate) in [(WRITTEN.as_slice(), "written"), (LEFT_TO_HAND.as_slice(), "lost")] {
        for f in functions {
            let cells = line_of(&doc, f);
            assert_eq!(cells.len(), 4, "{f}: {cells:?}");
            assert_eq!(cells[1], fate, "{f}");
            assert!(!cells[2].is_empty(), "{f} has no reason");
            // The piece reads `Adaptation Programming Guide, § <section>, <revision>`: the guide,
            // its section (a name after `§`) and the revision (a hex commit of 7 characters or more
            // of the pinned KnobKraft checkout).
            let parts: Vec<&str> = cells[3].split(',').map(str::trim).collect();
            assert_eq!(parts.len(), 3, "{f}: the piece is `guide, § section, revision`: {cells:?}");
            assert_eq!(parts[0], "Adaptation Programming Guide", "{f}: {cells:?}");
            assert!(parts[1].strip_prefix('§').is_some_and(|s| !s.trim().is_empty()), "{f} has no section: {cells:?}");
            assert!(parts[2].len() >= 7 && parts[2].chars().all(|c| c.is_ascii_hexdigit()), "{f} has no revision: {cells:?}");
        }
    }
}

#[test]
fn the_document_does_not_list_a_function_the_export_neither_writes_nor_names_lost() {
    let doc = read("docs/knobkraft.md");
    for line in doc.lines().filter(|l| l.starts_with("| `")) {
        let name = line.trim_start_matches("| `").split('`').next().unwrap();
        // The lines of the licence table are named by an SPDX identifier, which starts upper case.
        if name.starts_with(|c: char| c.is_ascii_uppercase()) {
            continue;
        }
        assert!(WRITTEN.contains(&name) || LEFT_TO_HAND.contains(&name), "{name} is in the document and not in the export");
    }
}

#[test]
fn every_function_the_export_names_lost_is_a_line_of_the_document() {
    let doc = read("docs/knobkraft.md");
    let (_, lost) = exported(&device(json!({"programs": null})));
    for l in lost.iter().filter(|l| WRITTEN.contains(&l.place.as_str()) || LEFT_TO_HAND.contains(&l.place.as_str())) {
        line_of(&doc, &l.place);
    }
}

#[test]
fn the_document_names_knobkraft_orm_its_author_and_the_guides_it_cites() {
    let doc = read("docs/knobkraft.md");
    for word in ["KnobKraft Orm", "Christof Ruch", "https://github.com/christofmuc/KnobKraft-orm", "Adaptation Programming Guide", "Adaptation Testing Guide", "AGPL"] {
        assert!(doc.contains(word), "knobkraft.md does not say {word}");
    }
}

#[test]
fn the_document_says_what_of_an_adaptation_is_data_of_a_device_and_what_is_code_only_the_host_runs() {
    let doc = read("docs/knobkraft.md").to_lowercase();
    assert!(doc.contains("data of a device"), "no word on what is data of a device");
    assert!(doc.contains("code that only the host runs"), "no word on what is code only the host runs");
    assert!(doc.contains("import") && doc.contains("refused"), "no word on the refused import");
}

#[test]
fn the_interface_and_the_frame_list_knobkraft_and_the_interface_points_to_its_document() {
    for path in ["docs/INTERFACE.md", "docs/CADRE.md"] {
        assert!(read(path).contains("`knobkraft`") || read(path).contains("KnobKraft"), "{path} does not list KnobKraft");
    }
    let interface = read("docs/INTERFACE.md");
    assert!(interface.contains("`knobkraft`"), "INTERFACE.md does not name `knobkraft`");
    assert!(interface.contains("(knobkraft.md)"), "INTERFACE.md does not point to knobkraft.md");
    assert!(interface.contains("NoSoundTransfers"), "INTERFACE.md does not name the fault of a device without sound transfers");
}

#[test]
fn the_role_of_the_frame_lists_knobkraft_and_the_frame_keeps_refusing_to_run_code() {
    let cadre = read("docs/CADRE.md");
    assert!(cadre.contains("**R12**"));
    let role = cadre.lines().find(|l| l.starts_with("- **R1**")).unwrap();
    assert!(role.contains("KnobKraft"), "R1 does not list KnobKraft: {role}");
}

#[test]
fn the_export_is_the_same_text_each_time() {
    let d = device(json!({}));
    assert_eq!(export(&d, Format::Knobkraft).unwrap().value, export(&d, Format::Knobkraft).unwrap().value);
}
