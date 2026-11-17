//! The README announces the current version and links its schemas (OSD-b91.2.9).

use crate::common;

fn readme() -> String {
    std::fs::read_to_string(common::repo().join("README.md")).unwrap()
}

#[test]
fn the_readme_states_the_status_as_0_5_published_and_frozen() {
    assert!(readme().contains("**Status: 0.5, published and frozen.**"));
    assert!(!readme().contains("0.5, draft"));
}

#[test]
fn the_list_of_versions_says_0_5_is_published_and_frozen() {
    let text = readme();
    assert!(text.contains("- **0.5**: published and frozen."));
    assert!(!text.contains("**0.5**: draft."));
}

#[test]
fn the_promise_paragraph_states_the_frozen_version_and_the_conversion() {
    let text = readme();
    for needle in ["never changes", "converts", "1.0"] {
        assert!(text.contains(needle), "missing {needle}");
    }
    assert!(!text.contains("may rename or remove fields"));
    assert!(!text.to_lowercase().contains("remove"));
}

#[test]
fn the_promise_paragraph_names_the_three_readers_that_hold_1_0() {
    let text = readme();
    for needle in ["osc-bridge", "SyntHub", "round trip"] {
        assert!(text.contains(needle), "missing {needle}");
    }
    assert!(text.contains("may still add and rename"));
}

#[test]
fn criterion_5_the_five_schema_links_of_the_table_point_to_schemas_0_5() {
    let text = readme();
    for kind in ["device", "profile", "sound", "fingerprint", "firmware"] {
        let link = format!("[`{kind}.schema.json`](schemas/0.5/{kind}.schema.json)");
        assert!(text.contains(&link), "missing {link}");
        let old = format!("](schemas/0.4/{kind}.schema.json)");
        assert!(!text.contains(&old), "{kind} still links 0.4");
    }
}
