//! Each proposal has one number, so that it is cited without doubt: a file of `spec/proposals`,
//! the template aside, is named `<n>-<name>.md` (the form of section 1 of `spec/PROCESS.md`), no
//! two share the value of `<n>`, the title of its first line, `# <n> — <title>` as in the
//! template, carries the same number, and a link from a proposal to another one resolves and shows
//! the number of its target.
// @lit spec/proposals/**

use crate::common;
use std::collections::BTreeMap;

const FOLDER: &str = "spec/proposals";
const TEMPLATE: &str = "TEMPLATE.md";

/// A number by its value, whatever its leading zeros; none when it is not made of digits.
fn value(digits: &str) -> Option<&str> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(digits.trim_start_matches('0'))
}

/// The number of a file named `<n>-<name>.md`.
fn number_of_name(file: &str) -> Option<&str> {
    let (n, name) = file.strip_suffix(".md")?.split_once('-')?;
    value(n).filter(|_| !name.is_empty())
}

/// The number of a first line `# <n> — <title>`.
fn number_of_title(text: &str) -> Option<&str> {
    let (n, title) = text.lines().next()?.strip_prefix("# ")?.split_once(" — ")?;
    value(n).filter(|_| !title.trim().is_empty())
}

/// What the numbering refuses in a folder of proposals given as (file name, text).
fn numbering_faults(files: &[(String, String)]) -> Vec<String> {
    let mut faults = Vec::new();
    let mut taken: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (file, text) in files.iter().filter(|(file, _)| file != TEMPLATE) {
        let Some(number) = number_of_name(file) else {
            faults.push(format!("{file}: the name is not <n>-<name>.md, <n> made of digits (spec/PROCESS.md section 1)"));
            continue;
        };
        taken.entry(number).or_default().push(file);
        match number_of_title(text) {
            Some(title) if title == number => {}
            _ => faults.push(format!(
                "{file}: the first line is not `# <n> — <title>` with the number of the file name, as in {TEMPLATE}"
            )),
        }
    }
    for sharing in taken.values_mut().filter(|sharing| sharing.len() > 1) {
        sharing.sort_unstable();
        faults.push(format!(
            "{}: one number, several proposals. One number per proposal: the one already on main keeps it, \
             the one entering takes the next free number",
            sharing.join(", ")
        ));
    }
    faults
}

/// The links of a text to a file beside it: (label, target).
fn local_links(text: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    for (at, _) in text.match_indices("](") {
        let Some(open) = text[..at].rfind('[') else { continue };
        let Some(length) = text[at + 2..].find(')') else { continue };
        let target = &text[at + 2..at + 2 + length];
        let target = target.split('#').next().unwrap_or(target);
        if target.ends_with(".md") && !target.contains('/') {
            out.push((&text[open + 1..at], target));
        }
    }
    out
}

/// The links between proposals that name no file of the folder, or whose label says another number
/// than their target.
fn link_faults(files: &[(String, String)]) -> Vec<String> {
    let mut faults = Vec::new();
    for (file, text) in files.iter().filter(|(file, _)| file != TEMPLATE) {
        for (label, target) in local_links(text) {
            let Some(number) = number_of_name(target) else { continue };
            if !files.iter().any(|(other, _)| other == target) {
                faults.push(format!("{file}: the link to {target} names no proposal"));
                continue;
            }
            let said = label
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|word| word.len() == 4)
                .filter_map(value)
                .find(|said| *said != number);
            if let Some(said) = said {
                faults.push(format!("{file}: the label [{label}] says {said}, its link goes to {target}"));
            }
        }
    }
    faults
}

/// Every file of `spec/proposals` with its text.
fn proposals() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(common::repo().join(FOLDER))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{FOLDER}/{name}: {e}"));
            (name, text)
        })
        .collect();
    files.sort();
    files
}

/// A fabricated proposal whose title carries the given number.
fn proposal(file: &str, title_number: &str) -> (String, String) {
    (file.to_string(), format!("# {title_number} — A title\n\n## Status\n"))
}

#[test]
fn every_proposal_of_the_folder_has_a_number_of_its_own_in_its_name_and_its_title() {
    let files = proposals();
    assert!(files.iter().any(|(file, _)| file == TEMPLATE), "the folder read is not the one of the proposals");
    assert!(files.len() > 1, "no proposal read in {FOLDER}");
    let faults = numbering_faults(&files);
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn proposals_with_distinct_numbers_are_accepted_and_the_template_is_left_aside() {
    let files = [
        proposal("0001-one.md", "0001"),
        proposal("0002-two-with-7-bit-words.md", "0002"),
        proposal("0010-ten.md", "0010"),
        proposal("10000-past-four-digits.md", "10000"),
        proposal(TEMPLATE, "<n>"),
    ];
    assert_eq!(numbering_faults(&files), Vec::<String>::new());
}

#[test]
fn two_proposals_of_one_number_are_refused_naming_both_and_saying_which_one_keeps_it() {
    let files = [
        proposal("0014-places-of-a-sound.md", "0014"),
        proposal("0014-the-unit-of-a-setting.md", "0014"),
        proposal("0016-the-clock.md", "0016"),
    ];
    let faults = numbering_faults(&files);
    assert_eq!(faults.len(), 1, "{faults:?}");
    for said in [
        "0014-places-of-a-sound.md",
        "0014-the-unit-of-a-setting.md",
        "One number per proposal",
        "already on main keeps it",
        "takes the next free number",
    ] {
        assert!(faults[0].contains(said), "{said}: {faults:?}");
    }
    assert!(!faults[0].contains("0016-the-clock.md"), "{faults:?}");
}

#[test]
fn every_number_taken_more_than_once_is_refused_with_each_file_that_takes_it() {
    let files = [
        proposal("0015-a.md", "0015"),
        proposal("0015-b.md", "0015"),
        proposal("0015-c.md", "0015"),
        proposal("0017-d.md", "0017"),
        proposal("0017-e.md", "0017"),
    ];
    let faults = numbering_faults(&files);
    assert_eq!(faults.len(), 2, "{faults:?}");
    for file in ["0015-a.md", "0015-b.md", "0015-c.md"] {
        assert!(faults[0].contains(file), "{file}: {faults:?}");
    }
    for file in ["0017-d.md", "0017-e.md"] {
        assert!(faults[1].contains(file), "{file}: {faults:?}");
    }
}

#[test]
fn a_number_is_compared_by_its_value_whatever_its_leading_zeros() {
    let files = [proposal("0007-padded.md", "0007"), proposal("7-bare.md", "7")];
    let faults = numbering_faults(&files);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(faults[0].contains("0007-padded.md") && faults[0].contains("7-bare.md"), "{faults:?}");
}

#[test]
fn a_file_whose_name_is_not_a_number_then_a_name_is_refused() {
    for file in [
        "a-title.md",
        "0019.md",
        "0019-.md",
        "-a-title.md",
        "0019a-a-title.md",
        "0019_a_title.md",
        "0019-a-title.txt",
        "0019-a-title",
        "README.md",
        "template.md",
    ] {
        let faults = numbering_faults(&[proposal(file, "0019")]);
        assert_eq!(faults.len(), 1, "{file}: {faults:?}");
        assert!(faults[0].starts_with(&format!("{file}:")), "{file}: {faults:?}");
    }
}

#[test]
fn a_proposal_whose_title_carries_another_number_than_its_name_is_refused() {
    let files = [proposal("0019-the-unit-of-a-setting.md", "0014"), proposal("0016-the-clock.md", "0016")];
    let faults = numbering_faults(&files);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(faults[0].starts_with("0019-the-unit-of-a-setting.md:"), "{faults:?}");
}

#[test]
fn a_proposal_whose_first_line_is_not_a_numbered_title_is_refused() {
    for text in [
        "",
        "# A title\n",
        "# 0019\n",
        "# 0019 A title\n",
        "## 0019 — A title\n",
        "\n# 0019 — A title\n",
        "Some words.\n\n# 0019 — A title\n",
    ] {
        let faults = numbering_faults(&[("0019-a-title.md".to_string(), text.to_string())]);
        assert_eq!(faults.len(), 1, "{text:?}: {faults:?}");
        assert!(faults[0].starts_with("0019-a-title.md:"), "{text:?}: {faults:?}");
    }
}

#[test]
fn the_drafts_written_with_a_shared_number_each_have_one_the_first_on_main_keeping_it() {
    let files = proposals();
    let names: Vec<&str> = files.iter().map(|(file, _)| file.as_str()).collect();
    for kept_or_taken in [
        "0014-places-of-a-sound-read-bits-signed-values-and-7-bit-words.md",
        "0015-a-range-of-notes-on-a-way.md",
        "0017-midi-and-osc-on-one-device.md",
        "0019-the-unit-of-a-setting.md",
        "0020-a-named-value-carries-a-term-of-the-vocabulary.md",
        "0021-a-source-or-destination-of-modulation-carries-its-meaning.md",
    ] {
        assert!(names.contains(&kept_or_taken), "{kept_or_taken} is not in {FOLDER}: {names:#?}");
    }
}

#[test]
fn every_link_from_a_proposal_to_another_names_a_proposal_and_shows_its_number() {
    let faults = link_faults(&proposals());
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn a_link_to_a_former_name_or_a_label_with_a_former_number_is_refused() {
    let target = proposal("0020-a-named-value.md", "0020");
    let citing = |line: &str| ("0021-modulation.md".to_string(), format!("# 0021 — Modulation\n\n{line}\n"));
    for line in [
        "See [draft 0015, a named value](0015-a-named-value.md).",
        "See [draft 0015, a named value](0020-a-named-value.md).",
    ] {
        let faults = link_faults(&[target.clone(), citing(line)]);
        assert_eq!(faults.len(), 1, "{line}: {faults:?}");
        assert!(faults[0].starts_with("0021-modulation.md:"), "{line}: {faults:?}");
    }
    for line in [
        "See [draft 0020, a named value](0020-a-named-value.md#the-change).",
        "See [the draft on named values](0020-a-named-value.md).",
        "See [the process](../PROCESS.md) and [the template](TEMPLATE.md).",
    ] {
        assert_eq!(link_faults(&[target.clone(), citing(line)]), Vec::<String>::new(), "{line}");
    }
}
