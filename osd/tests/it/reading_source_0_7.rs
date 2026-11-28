//! What the `source` of a reading names (`spec/0.7.md`, 5.3.4 and 23.43): which sources judge it
//! in a file read alone and in a resolution, the same rule in proposals 0006 and 0022, and the
//! conformance cases that hold it.

use crate::common;

use serde_json::{json, Value};
use std::collections::BTreeMap;

fn text_of(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The lines of `path` under the heading that starts with `heading`, up to the next heading of
/// its level or above: one paragraph per line.
fn section(path: &str, heading: &str) -> Vec<String> {
    let text = text_of(path);
    let level = heading.chars().take_while(|c| *c == '#').count();
    let above = |l: &&str| {
        let n = l.chars().take_while(|c| *c == '#').count();
        n > 0 && n <= level
    };
    let lines: Vec<String> = text.lines().skip_while(|l| !l.starts_with(heading)).skip(1).take_while(|l| !above(l)).map(str::to_string).collect();
    assert!(!lines.is_empty(), "{path} has no section {heading:?}");
    lines
}

/// One paragraph of `lines` holds every one of `words`.
#[track_caller]
fn one_paragraph_says(lines: &[String], words: &[&str]) {
    assert!(
        lines.iter().any(|l| !l.starts_with("Cases:") && words.iter().all(|w| l.contains(w))),
        "no paragraph says {words:?}:\n{}",
        lines.join("\n")
    );
}

fn readings_section() -> Vec<String> {
    section("spec/0.7.md", "### 5.3.4 ")
}

/// A device file of 0.7 that holds the keys of `extra`.
fn device(extra: Value) -> String {
    let mut d = json!({"$schema": "https://w3id.org/osd/0.7/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}});
    for (k, v) in extra.as_object().unwrap() {
        d[k] = v.clone();
    }
    d.to_string()
}

// ---- 5.3.4: a file without `midi_of` read alone ----

#[test]
fn the_text_judges_a_file_without_midi_of_read_alone_on_its_sources_and_midi_of_sources_a_profile_entry_included() {
    let section = readings_section();
    one_paragraph_says(
        &section,
        &["MUST", "`source`", "command", "control", "`midi_of`", "without being resolved", "`sources`", "`midi_of_sources`", "`profile`"],
    );
    let text = section.join("\n");
    assert!(!text.contains("of the `sources` of the file alone"), "the text still judges a file alone on its `sources` only:\n{text}");
}

// ---- 5.3.4: a `midi_of` file read alone, then resolved ----

#[test]
fn the_text_says_a_reader_does_not_refuse_a_midi_of_file_read_alone_for_a_source_its_lists_do_not_hold() {
    one_paragraph_says(&readings_section(), &["MUST NOT refuse", "`midi_of`", "`source`", "`id`"]);
}

#[test]
fn the_sentence_of_the_refusals_bounds_a_source_that_names_no_id_by_midi_of() {
    let section = readings_section();
    let refusals = section.iter().find(|l| l.starts_with("A reader MUST refuse")).expect("the paragraph of the refusals of 5.3.4");
    assert!(refusals.contains("`source`"), "{refusals}");
    assert!(refusals.contains("`midi_of`"), "the refusal of a `source` that names no `id` has no bound for a `midi_of` file read alone: {refusals}");
}

#[test]
fn the_text_says_the_resolution_judges_each_kept_reading_on_the_sources_written_in_the_files_joined() {
    one_paragraph_says(
        &readings_section(),
        &["MUST", "resol", "`midi_of`", "`source`", "`sources`", "`midi_of_sources`", "file asked for", "target", "`BadValue`", "pointer", "joined device"],
    );
}

#[test]
fn the_text_judges_a_file_without_midi_of_the_end_of_a_chain_included_on_its_own_sources_in_a_resolution() {
    one_paragraph_says(
        &readings_section(),
        &["MUST", "without `midi_of`", "chain", "resol", "`source`", "its own", "kept", "discard", "that file", "pointer"],
    );
}

#[test]
fn the_text_says_a_reading_of_a_midi_of_file_that_the_join_discards_is_not_judged() {
    let section = readings_section();
    assert!(
        section.iter().map(|l| l.to_lowercase()).any(|l| ["reading", "`midi_of`", "replace", "judge", " not "].iter().all(|w| l.contains(w))),
        "no paragraph of 5.3.4 says a reading of a `midi_of` file that the join discards (its command or control replaced) is not judged:\n{}",
        section.join("\n")
    );
}

// ---- 5.3.4 and 23.43: the sources the resolution adds from profiles ----

#[test]
fn the_text_leaves_the_sources_the_resolution_adds_from_profiles_out_of_the_set_a_reading_names() {
    one_paragraph_says(&readings_section(), &["`source`", "profile", "23.42"]);
}

#[test]
fn the_rule_of_the_sources_of_a_profile_says_it_speaks_of_those_added_and_keeps_an_entry_a_file_writes() {
    let rule = section("spec/0.7.md", "### 23.43 ");
    let paragraph = rule
        .iter()
        .find(|l| l.contains("of a reading") && l.contains("source of a profile") && l.contains("judge"))
        .or_else(|| rule.iter().find(|l| l.contains("`source` of a reading")))
        .expect("the paragraph of 23.43 on the `source` of a reading");
    assert!(paragraph.contains("append") || paragraph.contains("add"), "it does not say it speaks of the sources the resolution adds: {paragraph}");
    assert!(paragraph.contains("write") || paragraph.contains("written"), "it does not say what a file writes stays in the set: {paragraph}");
    assert!(paragraph.contains("`profile`"), "it does not name an entry marked `profile`: {paragraph}");
}

#[test]
fn a_resolution_keeps_in_the_set_an_entry_marked_profile_that_a_target_writes_itself() {
    let reading = json!([{"state": "valid", "firmware": "1", "source": "written"}]);
    for list in ["sources", "midi_of_sources"] {
        let mut credit = json!({});
        credit[list] = json!([{"type": "vendor-doc", "id": "written", "profile": "acme"}]);
        let files: BTreeMap<String, String> = [
            ("acme/a".to_string(), device(json!({"midi_of": "acme/b", "commands": [{"osc": "/dump", "readings": reading}]}))),
            ("acme/b".to_string(), device(credit)),
        ]
        .into();
        osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{list}: {f}"));
    }
}

// ---- the end of a chain: judged on its own text ----

#[test]
fn a_resolution_refuses_in_the_end_of_a_chain_a_reading_that_names_a_source_it_does_not_write_even_discarded() {
    let reading = json!([{"state": "valid", "firmware": "1", "source": "only-a"}]);
    let way = json!([{"ctrlType": "cc", "ctrlIndex": [74]}]);
    let command = json!({"commands": [{"osc": "/dump", "readings": reading}]});
    let control = json!({"cc_params": {"entries": [{"osc": "/cutoff", "readings": reading, "ways": way}]}});
    let replacing = [
        json!({"commands": [{"osc": "/dump"}]}),
        json!({"cc_params": {"entries": [{"osc": "/cutoff", "ways": way}]}}),
    ];
    for (end, replaced, pointer) in [
        (command, &replacing[0], "/commands/0/readings/0/source"),
        (control, &replacing[1], "/cc_params/entries/0/readings/0/source"),
    ] {
        let mut asked = replaced.clone();
        asked["midi_of"] = json!("acme/b");
        asked["sources"] = json!([{"type": "vendor-doc", "id": "only-a"}]);
        let files: BTreeMap<String, String> = [
            ("acme/a".to_string(), device(asked)),
            ("acme/b".to_string(), device(json!({"midi_of": "acme/c"}))),
            ("acme/c".to_string(), device(end)),
        ]
        .into();
        let fault = osd::resolve("acme/a", &files).expect_err(pointer);
        assert_eq!(
            (fault.kind, fault.file.as_deref(), fault.pointer.as_str()),
            (osd::FaultKind::BadValue, Some("acme/c"), pointer),
            "{fault}"
        );
    }
}

// ---- the cases ----

const CASES: [&str; 5] = [
    "0.7/read/reading-source-of-midi-of-sources-accepted",
    "0.7/read/midi-of-file-alone-reading-source-unknown-accepted",
    "0.7/resolve/midi-of-chain-reading-names-a-source-of-the-file-asked-accepted",
    "0.7/resolve/midi-of-chain-kept-reading-names-no-source-refused",
    "0.7/resolve/midi-of-chain-end-reading-names-a-source-of-the-file-asked-refused",
];

#[test]
fn the_cases_line_of_the_readings_names_the_cases_of_the_source_of_a_reading() {
    let section = readings_section();
    let cases = section.iter().find(|l| l.starts_with("Cases:")).expect("the Cases line of 5.3.4");
    for case in CASES {
        assert!(common::repo().join("conformance").join(case).join("case.json").is_file(), "{case}");
        assert!(cases.contains(&format!("`{case}`")), "the Cases line of 5.3.4 does not name {case}");
    }
}

#[test]
fn the_version_of_the_suite_that_holds_these_cases_is_at_least_1_18() {
    let version = text_of("conformance/VERSION");
    let mut parts = version.trim().split('.').map(|p| p.parse::<u32>().unwrap());
    let found = (parts.next().unwrap(), parts.next().unwrap());
    assert!(found >= (1, 18), "{version}");
}

// ---- the proposals say the same rule ----

/// The words each proposal says of the set a `source` is judged in: the sources written in the
/// files joined (not those added from profiles), both lists of a file read alone with `profile`
/// included, and no judgement of a `midi_of` file read alone.
#[track_caller]
fn says_the_rule_of_the_source(lines: &[String]) {
    let text = lines.join("\n");
    for word in ["`midi_of`", "`sources`", "`midi_of_sources`", "`profile`", "without being resolved"] {
        assert!(text.contains(word) || (word == "`sources`" && text.contains("`_sources`")), "the text does not say {word}:\n{text}");
    }
    assert!(text.contains("append") || text.contains("add"), "nothing of the sources added from profiles:\n{text}");
    let lower = text.to_lowercase();
    assert!(
        lower.contains("must not refuse") || lower.contains("not judged") || lower.contains("not judge"),
        "nothing says a `midi_of` file read alone is not judged:\n{text}"
    );
    assert!(
        lines.iter().any(|l| l.contains("chain") && l.contains("without `midi_of`")),
        "nothing says a file without `midi_of`, the end of a chain included, is judged on its own sources:\n{text}"
    );
}

#[test]
fn the_proposal_of_the_readings_says_the_rule_of_their_source() {
    says_the_rule_of_the_source(&section(
        "spec/proposals/0006-a-command-carries-its-readings-each-with-its-state-and-its-firmware.md",
        "### How a reading names its firmware and its source",
    ));
}

#[test]
fn the_draft_of_the_origin_of_a_role_judges_its_source_by_the_same_rule() {
    says_the_rule_of_the_source(&section(
        "spec/proposals/0022-a-role-says-where-it-comes-from-and-whether-it-is-confirmed.md",
        "### What the `source` of a role names",
    ));
}

#[test]
fn the_draft_of_the_origin_of_a_role_leaves_no_open_point_on_a_midi_of_file_read_alone() {
    let open = section("spec/proposals/0022-a-role-says-where-it-comes-from-and-whether-it-is-confirmed.md", "### Open points").join("\n");
    assert!(!open.contains("`midi_of` file read alone"), "the point the text of 0.7 settles is still open:\n{open}");
}
