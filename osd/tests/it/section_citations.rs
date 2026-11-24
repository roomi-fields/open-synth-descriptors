//! A rule of the format is cited by its numbered section of `spec/0.6.md`: every section has one
//! number and a title, a section number is not written twice, a citation names a section of the
//! text, the informative table of the annex covers the former numbers of resolution without a gap,
//! and no bare rule identifier is left in the packages: a rule of a charter is cited with its
//! document.

use crate::common;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const SPEC: &str = "spec/0.6.md";
const PACKAGES: [&str; 3] = ["osd", "osd-cli", "osd-formats"];

fn spec_text() -> String {
    std::fs::read_to_string(common::repo().join(SPEC)).unwrap()
}

/// The headings of a text outside fenced code: (line number, level, text after the hashes).
fn headings(text: &str) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let mut fenced = false;
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        let n = line.chars().take_while(|c| *c == '#').count();
        if !fenced && n > 0 && line[n..].starts_with(' ') {
            out.push((i + 1, n, line[n + 1..].trim().to_string()));
        }
    }
    out
}

fn is_number(word: &str) -> bool {
    let mut parts = word.split('.');
    let first = parts.next().unwrap_or("");
    !first.is_empty() && first.chars().all(|c| c.is_ascii_digit()) && parts.all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// The number a heading carries: `23.25`, or `B.2` / `A` in an annex; none when it has none.
fn number_of(heading: &str) -> Option<String> {
    let mut words = heading.split_whitespace();
    let first = words.next()?;
    if first == "Annex" {
        return words.next().map(str::to_string);
    }
    if is_number(first) {
        return Some(first.to_string());
    }
    if first.len() > 2 && first.as_bytes()[0].is_ascii_uppercase() && first.as_bytes()[1] == b'.' && is_number(&first[2..]) {
        return Some(first.to_string());
    }
    None
}

/// Faults of the numbering of a text: a section without number or title, a number present twice.
fn numbering_faults(text: &str) -> Vec<String> {
    let mut faults = Vec::new();
    let mut seen = BTreeSet::new();
    for (no, level, h) in headings(text) {
        if level == 1 {
            continue;
        }
        match number_of(&h) {
            None => faults.push(format!("line {no}: the section `{h}` has no number")),
            Some(n) => {
                let title = h.split_once(' ').map_or("", |(_, t)| t).trim();
                let title = if h.starts_with("Annex") { title.split_once(' ').map_or("", |(_, t)| t) } else { title };
                if title.is_empty() {
                    faults.push(format!("line {no}: the section {n} has no title"));
                }
                if !seen.insert(n.clone()) {
                    faults.push(format!("line {no}: the section number {n} is present twice"));
                }
            }
        }
    }
    faults
}

fn section_numbers(text: &str) -> BTreeSet<String> {
    headings(text).into_iter().filter(|(_, l, _)| *l > 1).filter_map(|(_, _, h)| number_of(&h)).collect()
}

fn clean(word: &str) -> &str {
    word.trim_matches(|c: char| matches!(c, '(' | ')' | ',' | ';' | ':' | '.' | '"' | '`' | '*'))
}

/// The section numbers a line cites: `see N.N`, `section N.N`, `sections N.N`, `§ N.N`, and
/// `N.N of `spec/0.6.md``.
fn citations(line: &str) -> Vec<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let mut out = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let c = clean(w);
        if !c.contains('.') || !is_number(c) {
            continue;
        }
        let before = i.checked_sub(1).map(|j| clean(words[j]).to_lowercase()).unwrap_or_default();
        let after_of = words.get(i + 1).is_some_and(|x| *x == "of") && words.get(i + 2).is_some_and(|x| clean(x) == "spec/0.6.md");
        if matches!(before.as_str(), "see" | "section" | "sections" | "§") || after_of {
            out.push(c.to_string());
        }
    }
    out
}

fn citation_faults(text: &str, sections: &BTreeSet<String>, only_with_spec: bool) -> Vec<String> {
    let mut faults = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if only_with_spec && !line.contains(SPEC) {
            continue;
        }
        for c in citations(line) {
            if !sections.contains(&c) {
                faults.push(format!("line {}: cites the section {c}, which the text does not have", i + 1));
            }
        }
    }
    faults
}

fn files_of(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_str().unwrap().to_string();
        if p.is_dir() {
            if name != "target" && !name.starts_with('.') {
                files_of(&p, out);
            }
        } else if matches!(p.extension().and_then(|e| e.to_str()), Some("rs" | "md")) && name != "CHANGELOG.md" {
            out.push(p);
        }
    }
}

/// The files the guards read: Rust and Markdown of the three packages, and the text of the format.
fn guarded_files() -> Vec<PathBuf> {
    let root = common::repo();
    let mut out = Vec::new();
    for p in PACKAGES {
        files_of(&root.join(p), &mut out);
    }
    out.push(root.join("spec/0.6.md"));
    out.retain(|p| p.file_name().unwrap() != "section_citations.rs");
    out
}

/// The bare rule identifiers of a line: `R` and digits as a word, not allowed by the line or by
/// the text before it.
fn bare_identifiers(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let at_word = bytes[i] == b'R' && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'));
        if at_word {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let word_end = j == bytes.len() || !(bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_');
            if j > i + 1 && word_end {
                let id = &line[i..j];
                let defined = line.trim_start().starts_with(&format!("- **{id}**"));
                let quoted = line.contains(&format!("**{id}**"));
                let documented = line[..i].ends_with("docs/CADRE.md ");
                if !(defined || quoted || documented) {
                    out.push(id.to_string());
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn identifier_faults(path: &str, text: &str) -> Vec<String> {
    let mut faults = Vec::new();
    let mut in_annex_table = false;
    for (i, line) in text.lines().enumerate() {
        if path == SPEC {
            if line.starts_with('#') {
                in_annex_table = line.starts_with("### B.2");
            }
            if in_annex_table {
                continue;
            }
        }
        for id in bare_identifiers(line) {
            faults.push(format!("{path}:{}: {id} is not written with its document (docs/CADRE.md {id})", i + 1));
        }
    }
    faults
}

#[test]
fn every_section_of_the_text_has_a_number_and_a_title_and_no_number_is_written_twice() {
    let faults = numbering_faults(&spec_text());
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn a_section_number_present_twice_is_refused() {
    let text = "# T\n\n## 1 One\n\n### 1.1 A\n\n### 1.1 B\n";
    let faults = numbering_faults(text);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(faults[0].contains("1.1") && faults[0].contains("twice"), "{faults:?}");
    assert!(numbering_faults("# T\n\n## 1 One\n\n### 1.1 A\n\n### 1.2 B\n").is_empty());
}

#[test]
fn a_section_without_number_or_title_is_refused() {
    let faults = numbering_faults("# T\n\n## Scope\n\n## 2\n");
    assert_eq!(faults.len(), 2, "{faults:?}");
    // a heading inside a fenced block is not a section
    assert!(numbering_faults("# T\n\n## 1 One\n\n```\n## no number\n```\n").is_empty());
}

#[test]
fn every_citation_of_a_section_in_the_text_names_a_section_of_the_text() {
    let text = spec_text();
    let faults = citation_faults(&text, &section_numbers(&text), false);
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn a_citation_of_a_section_that_does_not_exist_is_refused_in_each_recognised_form() {
    let sections: BTreeSet<String> = ["1.1", "23.25"].iter().map(|s| s.to_string()).collect();
    for line in [
        "as the text says, see 9.9.",
        "the rule of section 9.9 holds",
        "the rules of sections 9.9 and 1.1 hold",
        "the rule of § 9.9 holds",
        "9.9 of `spec/0.6.md` says so",
    ] {
        let faults = citation_faults(line, &sections, false);
        assert_eq!(faults.len(), 1, "{line}: {faults:?}");
    }
    for line in ["see 1.1.", "section 23.25 of `spec/0.6.md`", "§ 1.1", "1.1 of `spec/0.6.md`", "version 9.9 is out"] {
        assert!(citation_faults(line, &sections, false).is_empty(), "{line}");
    }
}

#[test]
fn every_citation_of_a_section_in_the_rust_comments_and_messages_of_the_packages_names_a_section() {
    let sections = section_numbers(&spec_text());
    let root = common::repo();
    let mut faults = Vec::new();
    for f in guarded_files().into_iter().filter(|f| f.extension().is_some_and(|e| e == "rs")) {
        let text = std::fs::read_to_string(&f).unwrap();
        let rel = f.strip_prefix(&root).unwrap().display().to_string();
        faults.extend(citation_faults(&text, &sections, true).into_iter().map(|m| format!("{rel}: {m}")));
    }
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn the_table_of_the_annex_covers_the_former_numbers_one_to_thirty_nine_with_the_sections_that_state_them() {
    let text = spec_text();
    let sections = section_numbers(&text);
    for n in 1..=39 {
        let s = format!("23.{}", n + 2);
        assert!(sections.contains(&s), "the section {s} of the former number {n} does not exist");
    }
    let start = text.find("### B.2").expect("no section B.2");
    let rows: Vec<(u32, u32, String, String)> = text[start..]
        .lines()
        .filter(|l| l.starts_with("| R"))
        .map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
            let (a, b) = cells[0].split_once(" to ").unwrap_or_else(|| panic!("no range in {l}"));
            let num = |s: &str| s.trim_start_matches('R').parse::<u32>().unwrap_or_else(|_| panic!("bad number in {l}"));
            let (sa, sb) = cells[1].split_once(" to ").unwrap_or_else(|| panic!("no sections in {l}"));
            (num(a), num(b), sa.to_string(), sb.to_string())
        })
        .collect();
    assert_eq!(rows.len(), 4, "the table has four ranges: {rows:?}");
    let mut next = 1;
    for (a, b, sa, sb) in &rows {
        assert_eq!(*a, next, "a gap or an overlap before R{a}");
        assert!(b >= a);
        assert_eq!(*sa, format!("23.{}", a + 2), "the first section of R{a} to R{b}");
        assert_eq!(*sb, format!("23.{}", b + 2), "the last section of R{a} to R{b}");
        next = b + 1;
    }
    assert_eq!(next, 40, "the ranges end at R39");
}

#[test]
fn a_bare_rule_identifier_is_refused_and_one_written_with_its_document_is_not() {
    let r = "R12";
    assert_eq!(bare_identifiers(&format!("within {r} of it")), vec![r.to_string()]);
    assert_eq!(bare_identifiers(&format!("(osd/docs/CADRE.md {r})")), Vec::<String>::new());
    assert_eq!(bare_identifiers(&format!("- **{r}** Validating one descriptor")), Vec::<String>::new());
    assert_eq!(bare_identifiers(&format!("assert!(cadre.contains(\"**{r}**\"));")), Vec::<String>::new());
    assert_eq!(bare_identifiers("HR12 and R12x and R"), Vec::<String>::new());
}

#[test]
fn no_package_file_or_text_of_the_format_holds_a_bare_rule_identifier() {
    let root = common::repo();
    let mut faults = Vec::new();
    for f in guarded_files() {
        let rel = f.strip_prefix(&root).unwrap().display().to_string();
        let text = std::fs::read_to_string(&f).unwrap();
        faults.extend(identifier_faults(&rel, &text));
    }
    assert!(faults.is_empty(), "{faults:#?}");
}
