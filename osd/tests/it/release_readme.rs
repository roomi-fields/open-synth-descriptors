//! The README gives the three ways to install; the publication README says
//! who publishes first and what a failure does.

use crate::common;

fn read(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn the_readme_gives_the_three_ways_to_install_the_command() {
    let readme = read("README.md");
    assert!(readme.contains("cargo install osd-cli"), "cargo");
    assert!(readme.contains("npm install -g @roomi-fields/osd"), "npm");
    assert!(readme.contains("releases"), "a downloaded binary, from the releases page");
    assert!(readme.contains("SHA256SUMS"), "the binary is checked against its checksum file");
}

#[test]
fn the_workflow_checks_each_way_the_readme_gives() {
    let workflow = read(".github/workflows/release.yml");
    assert!(workflow.contains("cargo install"));
    assert!(workflow.contains("@roomi-fields/osd"));
    assert!(workflow.contains("releases/download") || workflow.contains("gh release download"));
}

#[test]
fn the_publication_readme_says_the_owner_publishes_each_new_name_by_hand_first() {
    let text = read("docs/RELEASE.md");
    for name in ["osd", "osd-formats", "osd-cli", "@roomi-fields/osd", "crates.io", "npm"] {
        assert!(text.contains(name), "missing {name}");
    }
    assert!(text.contains("by hand"));
    assert!(text.contains("trusted publisher"));
}

#[test]
fn the_publication_readme_says_what_a_failure_stops_and_that_nothing_is_rolled_back() {
    let text = read("docs/RELEASE.md");
    for needle in ["check", "crates.io", "yank", "rolls nothing back"] {
        assert!(text.contains(needle), "missing {needle}");
    }
}
