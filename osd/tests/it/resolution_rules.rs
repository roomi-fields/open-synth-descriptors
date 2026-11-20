//! The rules of resolution are written in the specification (`docs/INTERFACE.md`,
//! section Resolution), each tied to the conformance cases that hold it, and the
//! reader's comments cite them by number.

use crate::common;

use std::collections::BTreeSet;

fn text(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

struct Rule {
    number: u32,
    heading: String,
    cases_line: Option<String>,
}

/// The lines of the section `## Resolution` of the specification.
fn section() -> Vec<String> {
    let spec = text("docs/INTERFACE.md");
    let mut lines = spec.lines().skip_while(|l| l.trim_end() != "## Resolution").map(str::to_string);
    let head = lines.next();
    assert!(head.is_some(), "docs/INTERFACE.md has no `## Resolution` section");
    lines.take_while(|l| !l.starts_with("## ")).collect()
}

/// `R<n>` read at the start of `word`, when it is one.
fn rule_number(word: &str) -> Option<u32> {
    let digits: String = word.strip_prefix('R')?.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The rules of the section: each a `### R<n> — <one sentence>` heading and its `Cases:` line.
fn rules() -> Vec<Rule> {
    let mut out: Vec<Rule> = Vec::new();
    for line in section() {
        if let Some(rest) = line.strip_prefix("### ") {
            let bad = || panic!("a rule heading is `### R<n> — <sentence>`: {line}");
            let Some((tag, title)) = rest.split_once(" — ") else { bad() };
            let Some(number) = rule_number(tag).filter(|_| tag[1..].chars().all(|c| c.is_ascii_digit())) else { bad() };
            assert!(!title.trim().is_empty(), "rule R{number} has no sentence");
            out.push(Rule { number, heading: title.trim().to_string(), cases_line: None });
        } else if let Some(cases) = line.strip_prefix("Cases:") {
            if let Some(rule) = out.last_mut() {
                assert!(rule.cases_line.is_none(), "rule R{} has two `Cases:` lines", rule.number);
                rule.cases_line = Some(cases.to_string());
            }
        }
    }
    out
}

/// The cases a `Cases:` line cites: each `<version>/<case>` between backticks.
fn cited(line: &str) -> Vec<String> {
    line.split('`').skip(1).step_by(2).map(str::to_string).collect()
}

fn existing_cases() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for version in ["0.4", "0.5", "0.6"] {
        for case in std::fs::read_dir(common::repo().join(format!("conformance/{version}/resolve"))).unwrap() {
            out.insert(format!("{version}/{}", case.unwrap().file_name().to_string_lossy()));
        }
    }
    out
}

#[test]
fn the_specification_has_a_resolution_section_of_numbered_rules() {
    let rules = rules();
    assert!(!rules.is_empty(), "the Resolution section has no rule");
    let numbers: Vec<u32> = rules.iter().map(|r| r.number).collect();
    let want: Vec<u32> = (1..=rules.len() as u32).collect();
    assert_eq!(numbers, want, "rules are numbered R1, R2, … in order");
    for r in &rules {
        assert!(!r.heading.contains(". "), "R{} states one idea in one sentence: {}", r.number, r.heading);
    }
}

#[test]
fn the_resolution_section_states_the_limit_of_0_5_and_the_cases_of_0_6() {
    let lines = section();
    assert!(!lines.iter().any(|l| l.contains("no cases of its own")), "the section still says 0.6 has no cases of its own");
    assert!(lines.iter().any(|l| l.contains("conformance/0.6/resolve/")), "no line names the cases of 0.6");
    let limit = lines.iter().any(|l| l.contains("0.5") && l.contains("capabilities") && l.contains("get_frame"));
    assert!(limit, "no line states the limit of 0.5: a midi_of file may not declare capabilities nor give get_frame in params");
}

#[test]
fn the_rules_of_what_a_midi_of_file_adds_cite_the_cases_of_0_6_that_hold_them() {
    let rules = rules();
    let cases_of = |n: u32| cited(rules.iter().find(|r| r.number == n).and_then(|r| r.cases_line.as_deref()).unwrap_or(""));
    let want: [(u32, &[&str]); 5] = [
        (3, &["0.6/midi-of-custom-commands-with-lua", "0.6/midi-of-script-without-lua-refused", "0.6/midi-of-script-with-empty-capabilities-refused"]),
        (4, &["0.6/midi-of-params-entries-added", "0.6/midi-of-params-without-target-frames-refused"]),
        (7, &["0.6/midi-of-key-outside-the-list-refused", "0.6/midi-of-0-5-file-on-0-6-target-refused"]),
        (8, &["0.6/midi-of-params-target-key-refused"]),
        (12, &["0.6/midi-of-capabilities-declared-over-target-lua", "0.6/midi-of-lua-without-script-refused", "0.6/midi-of-0-6-file-on-0-5-target"]),
    ];
    let mut failures = Vec::new();
    for (n, names) in want {
        let got = cases_of(n);
        failures.extend(names.iter().filter(|c| !got.iter().any(|g| g == *c)).map(|c| format!("R{n} does not cite `{c}`")));
    }
    common::report("rules that miss their 0.6 cases", failures);
}

/// What a case of `conformance/0.6/resolve/` expects: `Err((kind, file, pointer))` for a fault, `Ok` for a result.
fn expected(case: &str) -> Result<(), (String, String, String)> {
    let path = format!("conformance/0.6/resolve/{case}/case.json");
    let spec: serde_json::Value = serde_json::from_str(&text(&path)).unwrap_or_else(|e| panic!("{path}: {e}"));
    match spec.get("fault") {
        None => {
            assert!(spec.get("result").is_some(), "{path} expects neither a result nor a fault");
            Ok(())
        }
        Some(f) => {
            let s = |k: &str| f[k].as_str().unwrap_or_else(|| panic!("{path}: fault has no `{k}`")).to_string();
            Err((s("kind"), s("file"), s("pointer")))
        }
    }
}

#[test]
fn the_refusals_of_the_role_vocabulary_state_and_of_a_missing_stands_for_are_rules_that_cite_their_cases() {
    let rules = rules();
    let cases_of = |n: u32| cited(rules.iter().find(|r| r.number == n).and_then(|r| r.cases_line.as_deref()).unwrap_or(""));
    let want: [(u32, &[&str]); 2] = [
        (33, &["0.6/midi-of-role-vocabulary-state-differs-refused", "0.6/midi-of-role-vocabulary-kept-from-target"]),
        (34, &["0.6/stands-for-absent-from-catalogue-refused", "0.6/midi-of-inherited-stands-for-absent-refused"]),
    ];
    let mut failures = Vec::new();
    for (n, names) in want {
        if !rules.iter().any(|r| r.number == n) {
            failures.push(format!("the section Resolution has no R{n}"));
            continue;
        }
        let got = cases_of(n);
        failures.extend(names.iter().filter(|c| !got.iter().any(|g| g == *c)).map(|c| format!("R{n} does not cite `{c}`")));
    }
    common::report("rules without their cases", failures);
}

#[test]
fn the_cases_of_the_role_vocabulary_state_and_of_stands_for_state_their_fault_in_the_file_asked_for() {
    let fault = |kind: &str, pointer: &str| Err((kind.to_string(), "acme/a".to_string(), pointer.to_string()));
    assert_eq!(expected("midi-of-role-vocabulary-state-differs-refused"), fault("BadValue", "/role_vocabulary"));
    assert_eq!(expected("midi-of-role-vocabulary-kept-from-target"), Ok(()));
    assert_eq!(expected("stands-for-absent-from-catalogue-refused"), fault("BadValue", "/device/stands_for"));
    assert_eq!(expected("midi-of-inherited-stands-for-absent-refused"), fault("BadValue", "/device/stands_for"));
}

#[test]
fn the_sentence_of_the_section_device_that_refuses_a_missing_stands_for_refers_to_its_rule() {
    let spec = text("docs/INTERFACE.md");
    let line = spec.lines().find(|l| l.contains("`stands_for`") && l.contains("absent from the catalogue") && l.contains("/device/stands_for")).expect("no sentence refuses a stands_for absent from the catalogue");
    assert!(line.contains("R34"), "that sentence does not refer to R34");
}

#[test]
fn each_rule_names_the_conformance_cases_that_hold_it_and_each_cited_case_exists() {
    let existing = existing_cases();
    let mut failures = Vec::new();
    for r in rules() {
        let Some(line) = &r.cases_line else {
            failures.push(format!("R{} has no `Cases:` line", r.number));
            continue;
        };
        let cases = cited(line);
        if cases.is_empty() {
            failures.push(format!("R{} cites no case", r.number));
        }
        for c in cases.iter().filter(|c| !existing.contains(*c)) {
            failures.push(format!("R{} cites `{c}`, which is no resolution case", r.number));
        }
    }
    common::report("rules without their cases", failures);
}

#[test]
fn each_resolution_case_of_0_4_0_5_and_0_6_is_named_by_a_rule() {
    assert!(!rules().is_empty(), "the Resolution section has no rule");
    let named: BTreeSet<String> = rules().iter().filter_map(|r| r.cases_line.as_deref()).flat_map(cited).collect();
    let failures = existing_cases().into_iter().filter(|c| !named.contains(c)).map(|c| format!("`{c}` is named by no rule")).collect();
    common::report("cases without a rule", failures);
}

/// The doc comment (`///`) written right above the line that starts with `anchor`.
fn doc_above(source: &str, anchor: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let at = lines.iter().position(|l| l.trim_start().starts_with(anchor)).unwrap_or_else(|| panic!("osd/src/resolve.rs has no `{anchor}`"));
    let mut doc: Vec<&str> = Vec::new();
    for l in lines[..at].iter().rev() {
        let t = l.trim_start();
        if t.starts_with("///") {
            doc.push(t);
        } else if !t.starts_with("#[") {
            break;
        }
    }
    doc.reverse();
    doc.join("\n")
}

/// The module header (`//!`) of a source.
fn module_doc(source: &str) -> String {
    source.lines().take_while(|l| l.starts_with("//!")).collect::<Vec<_>>().join("\n")
}

/// Why `comment` does not cite the section Resolution by the number of an existing rule, if it does not.
fn uncited(comment: &str, numbers: &BTreeSet<u32>) -> Option<String> {
    if !comment.contains("Resolution") || !comment.contains("docs/INTERFACE.md") {
        return Some("does not name the section Resolution of docs/INTERFACE.md".into());
    }
    let refs: Vec<u32> = comment.split(|c: char| !c.is_ascii_alphanumeric()).filter_map(rule_number).collect();
    if refs.is_empty() {
        return Some("cites no rule by its number (R<n>)".into());
    }
    refs.iter().find(|n| !numbers.contains(n)).map(|n| format!("cites R{n}, which the specification does not have"))
}

#[test]
fn the_comments_of_the_resolver_refer_to_the_rules_of_the_specification_by_number() {
    let numbers: BTreeSet<u32> = rules().iter().map(|r| r.number).collect();
    assert!(!numbers.is_empty(), "the Resolution section has no rule");
    let source = text("osd/src/resolve.rs");
    let mut failures = Vec::new();
    let mut check = |what: &str, comment: String| {
        if let Some(why) = uncited(&comment, &numbers) {
            failures.push(format!("{what}: {why}"));
        }
    };
    check("module header", module_doc(&source));
    for anchor in ["pub fn resolve", "fn follow", "fn complete", "const IDENTITY", "const OWN_LISTS", "const OWN_TABLES", "const OWN_BLOCKS"] {
        check(anchor, doc_above(&source, anchor));
    }
    common::report("comments that do not cite the specification", failures);
}

#[test]
fn the_resolve_line_of_the_reader_interface_refers_to_the_rules_of_the_specification() {
    let numbers: BTreeSet<u32> = rules().iter().map(|r| r.number).collect();
    assert!(!numbers.is_empty(), "the Resolution section has no rule");
    let interface = text("osd/docs/INTERFACE.md");
    let line = interface.lines().find(|l| l.starts_with("| `resolve` |")).expect("no `resolve` line in osd/docs/INTERFACE.md");
    if let Some(why) = uncited(line, &numbers) {
        panic!("the `resolve` line {why}");
    }
}
