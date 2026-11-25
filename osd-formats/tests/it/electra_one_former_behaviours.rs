//! The table "What the former importer kept" of docs/electra-one.md: one row per behaviour of
//! SyntHub's former Electra One importer, said kept with the name of its case, or lost with its
//! loss named.

use crate::common::electra::{FORMER_BEHAVIOURS, FORMER_POLICY};

const TABLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/electra-one.md");
const TESTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/it");

fn rows() -> Vec<Vec<String>> {
    std::fs::read_to_string(TABLE)
        .unwrap_or_else(|e| panic!("{TABLE}: {e}"))
        .lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .map(|l| l.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect())
        .collect()
}

fn row(name: &str) -> Vec<String> {
    rows().into_iter().find(|r| r[0] == name).unwrap_or_else(|| panic!("no row for the behaviour {name:?} in {TABLE}"))
}

/// Whether a test function of this name is written under tests/it/.
fn case_exists(name: &str) -> bool {
    std::fs::read_dir(TESTS).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "rs")).any(|p| {
        let text = std::fs::read_to_string(&p).unwrap();
        text.contains(&format!("\nfn {name}()"))
    })
}

#[test]
fn every_behaviour_of_the_former_importer_has_a_row_saying_kept_or_lost_with_its_piece() {
    for name in FORMER_BEHAVIOURS.iter().chain([&FORMER_POLICY]) {
        let r = row(name);
        assert!(r.len() >= 4, "{name}: a row has the behaviour, its fate, its case or loss and its piece");
        assert!(r[1] == "kept" || r[1] == "lost", "{name}: the fate is kept or lost, not {:?}", r[1]);
        assert!(!r[3].is_empty(), "{name}: no piece");
    }
}

#[test]
fn a_kept_behaviour_names_a_case_that_exists_and_a_lost_one_names_its_loss() {
    for name in FORMER_BEHAVIOURS.iter().chain([&FORMER_POLICY]) {
        let r = row(name);
        assert!(!r[2].is_empty(), "{name}: neither a case nor a loss named");
        if r[1] == "kept" {
            assert!(case_exists(&r[2]), "{name}: the case {:?} is not a test of tests/it/", r[2]);
        } else {
            assert!(r[2].starts_with('/') || r[2].starts_with("none"), "{name}: a loss is named by its pointer, or `none` with its reason: {:?}", r[2]);
        }
    }
}

#[test]
fn the_conversions_of_the_former_importer_are_kept_and_only_the_re_import_policy_is_out_of_the_format() {
    for name in ["checksum", "byte computed from the value", "parameter number of a command", "revision and hash of the preset"] {
        assert_eq!(row(name)[1], "kept", "{name}");
    }
    assert_eq!(row("byte computed from the parameter number")[1], "lost");
    assert_eq!(row(FORMER_POLICY)[1], "lost");
    assert!(row(FORMER_POLICY)[2].starts_with("none"), "the silent address is no loss of a conversion: {:?}", row(FORMER_POLICY));
}

#[test]
fn the_row_of_the_date_of_the_preset_is_lost_at_its_pointer_because_0_7_has_no_field_for_it() {
    let r = row("date of the preset");
    assert_eq!(r[1], "lost");
    assert_eq!(r[2], "/sources/0/imported_at");
    assert!(r[3].contains("0.7 has no field for the date of an import"), "{r:?}");
    assert!(r[3].contains("sources[].imported_at"), "{r:?}");
    assert!(r[3].contains("0008"), "{r:?}");
}

#[test]
fn the_row_of_a_lua_function_names_the_loss_at_the_message_of_the_control() {
    for name in [
        "lua function of one return",
        "lua function with a branch",
        "lua function calling an API of the controller",
        "lua function not defined",
    ] {
        let r = row(name);
        assert_eq!(r[1], "lost", "{name}");
        assert!(r[2].contains("/controls/N/values/M/message"), "{name}: {:?}", r[2]);
    }
}
