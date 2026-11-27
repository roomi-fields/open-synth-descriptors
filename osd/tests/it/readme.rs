//! The README is short, shows one descriptor, names the three audiences and points to
//! the text, the reference, the site and the changelog; the rules it once held live in
//! the text of the format, the interface of the command and the governance.

use crate::common;

fn read(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn readme() -> String {
    read("README.md")
}

#[test]
fn the_readme_holds_at_most_90_lines() {
    let lines = readme().lines().count();
    assert!(lines <= 90, "the README has {lines} lines");
}

#[test]
fn the_readme_has_no_normative_word_of_rfc_2119_in_capitals() {
    let text = readme();
    // "MUST NOT" and "SHALL NOT" are caught by their first word.
    for word in ["MUST", "SHALL", "SHOULD", "MAY", "REQUIRED", "RECOMMENDED"] {
        let found = text.split(|c: char| !c.is_ascii_alphabetic()).any(|w| w == word);
        assert!(!found, "the README carries the normative word {word}");
    }
}

fn inline_example() -> String {
    let text = readme();
    let after = text.split("```json\n").nth(1).expect("the README has a ```json block");
    after.split("\n```").next().unwrap().to_string()
}

#[test]
fn the_inline_example_is_a_0_6_descriptor_the_reader_validates_without_fault() {
    let example = inline_example();
    let value: serde_json::Value = serde_json::from_str(&example).expect("the example is JSON");
    let schema = value["$schema"].as_str().expect("the example names its $schema");
    assert!(schema.contains("/0.6/"), "the example is not 0.6: {schema}");
    let kind = [
        ("device", osd::Kind::Device),
        ("profile", osd::Kind::Profile),
        ("sound", osd::Kind::Sound),
        ("fingerprint", osd::Kind::Fingerprint),
        ("firmware", osd::Kind::Firmware),
    ]
    .into_iter()
    .find(|(name, _)| schema.ends_with(&format!("{name}.schema.json")))
    .unwrap_or_else(|| panic!("no kind in {schema}"))
    .1;
    osd::read(&example, kind).unwrap_or_else(|f| panic!("the example is refused: {f}"));
}

#[test]
fn the_readme_carries_four_badges_with_the_links_that_name_their_subject() {
    let text = readme();
    let all: Vec<&str> = text.lines().filter(|l| l.contains("![")).collect();
    let all = all.join("\n");
    assert!(
        all.contains("github.com/roomi-fields/open-synth-descriptors/actions/workflows/check.yml"),
        "the check badge does not point to the check.yml flow"
    );
    assert!(all.contains("crates.io") && all.contains("osd-cli"), "the crate badge does not name osd-cli on crates.io");
    assert!(
        all.contains("npmjs.com") && all.contains("@roomi-fields/osd"),
        "the npm badge does not name @roomi-fields/osd on npm"
    );
    assert!(all.contains("CC0"), "the licence badge does not name CC0");
    assert!(text.matches("![").count() >= 4, "fewer than four badges");
}

#[test]
fn the_readme_names_the_three_audiences() {
    let lower = readme().to_lowercase();
    for audience in ["author", "reader", "implementer"] {
        assert!(lower.contains(audience), "the README does not name the {audience}");
    }
}

#[test]
fn the_readme_points_to_the_text_the_reference_the_site_and_the_changelog() {
    let text = readme();
    for target in ["spec/0.6.md", "reference/", "https://roomi-fields.github.io/open-synth-descriptors/", "spec/CHANGELOG.md"] {
        assert!(text.contains(target), "the README does not point to {target}");
    }
}

#[test]
fn the_readme_points_where_what_it_dropped_now_lives() {
    let text = readme();
    for target in ["osd-cli/docs/INTERFACE.md", "docs/RELEASE.md", "GOVERNANCE.md"] {
        assert!(text.contains(target), "the README does not point to {target}");
    }
}

#[test]
fn every_relative_link_of_the_readme_resolves_to_a_file_or_a_folder() {
    let text = readme();
    let mut checked = 0;
    for part in text.split("](").skip(1) {
        let target = part.split(')').next().unwrap();
        if target.starts_with("http") || target.starts_with('#') || target.starts_with("mailto:") {
            continue;
        }
        let path = target.split('#').next().unwrap();
        assert!(common::repo().join(path).exists(), "{path} does not exist");
        checked += 1;
    }
    assert!(checked >= 4, "only {checked} relative links");
}

#[test]
fn the_readme_no_longer_carries_what_other_documents_hold() {
    let text = readme();
    assert!(!text.contains("## Where MIDI ends and OSD starts"), "the MIDI/OSD table is back");
    assert!(!text.contains("| Version | Published |"), "the list of versions is back");
    assert!(!text.contains("- **0.4**"), "the 0.1-0.4 history is back");
    assert!(!text.contains("osd import <format>"), "the description of each word is back");
    assert!(!text.contains("three independent readers"), "the conditions of 1.0 are back");
    assert!(!text.contains("0.6 is a draft") && !text.contains("0.6, draft"));
}

#[test]
fn the_text_of_the_format_has_one_address_and_its_old_places_are_gone() {
    for gone in ["docs/INTERFACE.md", "docs/controls.md", "docs/vocabulary.md"] {
        assert!(!common::repo().join(gone).exists(), "{gone} still exists beside spec/");
    }
    for here in ["spec/0.6.md", "spec/vocabulary/1.md"] {
        assert!(common::repo().join(here).is_file(), "{here} is missing");
    }
}

#[test]
fn state_1_of_the_vocabulary_is_stated_under_checksum_and_nowhere_unpublished() {
    let sums = read("schemas/SHA256SUMS");
    assert!(sums.contains("vocabulary/1/vocabulary.schema.json"), "state 1 is not recorded under checksum");
    for path in ["spec/0.6.md", "spec/vocabulary/1.md", "README.md", "CONTRIBUTING.md"] {
        let text = read(path);
        for false_claim in ["not published yet", "not under checksum"] {
            assert!(!text.contains(false_claim), "{path} says {false_claim:?} of state 1");
        }
    }
}

fn spec() -> String {
    read("spec/0.6.md")
}

#[test]
fn the_text_states_0_6_and_0_5_as_published_and_frozen() {
    let text = spec();
    assert!(text.contains("Version 0.6 is published and frozen."));
    assert!(text.contains("Version 0.5 is published and frozen."));
    assert!(!text.contains("0.6 (draft)"));
    assert!(!text.contains("0.6 is a draft"));
}

#[test]
fn the_text_states_that_a_published_version_never_changes_and_that_a_later_0_x_may_add_and_rename() {
    let text = spec();
    assert!(text.contains("never changes"), "spec/0.6.md misses the frozen rule");
    assert!(text.contains("converts"), "spec/0.6.md misses the conversion");
    assert!(text.contains("may still add and rename"), "spec/0.6.md misses the rule of a later 0.x");
}

#[test]
fn the_governance_names_the_three_readers_that_hold_1_0() {
    // The text is wrapped: a phrase may span a line break.
    let text = read("GOVERNANCE.md").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(text.contains("1.0"), "GOVERNANCE.md misses the condition of 1.0");
    for needle in ["two readers written without `osd`", "in other languages", "round trip"] {
        assert!(text.contains(needle), "GOVERNANCE.md misses {needle}");
    }
}

#[test]
fn the_five_schema_links_of_the_table_of_kinds_point_to_schemas_0_6() {
    let text = spec();
    for kind in ["device", "profile", "sound", "fingerprint", "firmware"] {
        let link = format!("schemas/0.6/{kind}.schema.json)");
        assert!(text.contains(&link), "spec/0.6.md misses {link}");
        let old = format!("schemas/0.5/{kind}.schema.json)");
        assert!(!text.contains(&old), "{kind} still links 0.5");
    }
}
