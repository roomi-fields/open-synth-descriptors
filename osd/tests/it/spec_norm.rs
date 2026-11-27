//! The text of 0.6 and the vocabulary are written as a norm: a closed set of keywords in capitals,
//! one keyword and a conformance class per requirement, the four opening sections in order, no
//! keyword in an informative block, and the order of precedence between the text, the schemas and
//! the conformance cases.
//!
//! A block is informative when it is a quotation opening with `NOTE` or `EXAMPLE`, a fenced code
//! block, or a section whose heading ends in `(informative)`. Everything else is normative prose.

use crate::common;

const TEXTS: [&str; 3] = ["spec/0.6.md", "spec/vocabulary/1.md", "spec/vocabulary/2.md"];
const CLASSES: [&str; 3] = ["descriptor", "reader", "converter"];
const SUBJECTS: [&str; 4] = ["descriptor", "reader", "converter", "publisher"];
const KEYWORDS: [&str; 3] = ["MUST", "SHOULD", "MAY"];
const FOREIGN: [&str; 4] = ["SHALL", "REQUIRED", "RECOMMENDED", "OPTIONAL"];

fn text(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[derive(Clone, Copy, PartialEq)]
enum Block {
    Normative,
    Informative,
}

struct Line {
    no: usize,
    text: String,
    block: Block,
    heading: bool,
}

fn heading_level(line: &str) -> usize {
    let n = line.chars().take_while(|c| *c == '#').count();
    if n > 0 && line[n..].starts_with(' ') {
        n
    } else {
        0
    }
}

/// Every line with the kind of block it sits in; inline code and link targets are blanked.
fn lines(path: &str) -> Vec<Line> {
    let mut out = Vec::new();
    let mut fenced = false;
    let mut informative_level: Option<usize> = None;
    let mut in_note = false;
    for (i, raw) in text(path).lines().enumerate() {
        if raw.trim_start().starts_with("```") {
            fenced = !fenced;
            out.push(Line { no: i + 1, text: String::new(), block: Block::Informative, heading: false });
            continue;
        }
        let level = if fenced { 0 } else { heading_level(raw) };
        if level > 0 {
            if informative_level.is_some_and(|l| level <= l) {
                informative_level = None;
            }
            if raw.trim_end().ends_with("(informative)") && informative_level.is_none() {
                informative_level = Some(level);
            }
            in_note = false;
            out.push(Line { no: i + 1, text: raw.to_string(), block: Block::Normative, heading: true });
            continue;
        }
        let quoted = raw.trim_start().strip_prefix('>').map(str::trim_start);
        match quoted {
            Some(q) if q.starts_with("NOTE") || q.starts_with("EXAMPLE") => in_note = true,
            Some(_) if in_note => {}
            _ => in_note = false,
        }
        let block = if fenced || in_note || informative_level.is_some() { Block::Informative } else { Block::Normative };
        out.push(Line { no: i + 1, text: blank_code(raw), block, heading: false });
    }
    out
}

/// The line without its inline code spans and the targets of its links.
fn blank_code(line: &str) -> String {
    let mut out = String::new();
    let mut code = false;
    let mut target = false;
    for c in line.chars() {
        match c {
            '`' => code = !code,
            '(' if !code && out.ends_with(']') => target = true,
            ')' if target => target = false,
            _ if code || target => {}
            _ => out.push(c),
        }
    }
    out
}

fn words(s: &str) -> Vec<&str> {
    s.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|w| !w.is_empty()).collect()
}

fn keyword_count(sentence: &str) -> usize {
    words(sentence).iter().filter(|w| KEYWORDS.contains(w)).count()
}

/// The sentences of the normative prose that are not table rows: each unit (a paragraph or a list
/// item) cut where a full stop meets a capital.
fn sentences(path: &str) -> Vec<(usize, String)> {
    let mut units: Vec<(usize, String)> = Vec::new();
    let mut open = false;
    for l in lines(path) {
        let t = l.text.trim();
        let starts_unit = t.starts_with("- ") || t.starts_with("* ") || (t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.contains(". "));
        if l.block == Block::Informative || l.heading || t.is_empty() || t.starts_with('|') {
            open = false;
            continue;
        }
        if open && !starts_unit {
            let last = units.last_mut().unwrap();
            last.1.push(' ');
            last.1.push_str(t);
        } else {
            units.push((l.no, t.to_string()));
        }
        open = true;
    }
    let mut out = Vec::new();
    for (no, unit) in units {
        let chars: Vec<char> = unit.chars().collect();
        let mut start = 0;
        for i in 0..chars.len() {
            let cut = chars[i] == '.' && chars.get(i + 1) == Some(&' ') && chars.get(i + 2).is_some_and(|c| c.is_uppercase());
            if cut || i + 1 == chars.len() {
                let s: String = chars[start..=i].iter().collect();
                out.push((no, s.trim().to_string()));
                start = i + 1;
            }
        }
    }
    out
}

/// The sentence that gives the keywords their meaning quotes the RFCs; it is the one sentence
/// that names the keywords themselves.
fn is_keyword_sentence(s: &str) -> bool {
    s.contains("RFC 8174")
}

#[test]
fn the_text_and_the_vocabulary_use_only_the_five_keywords_in_capitals() {
    let mut failures = Vec::new();
    for path in TEXTS {
        for l in lines(path).into_iter().filter(|l| l.block == Block::Normative && !l.heading) {
            for w in words(&l.text) {
                let lower = matches!(w, "must" | "should" | "may");
                if FOREIGN.contains(&w) || lower {
                    failures.push(format!("{path}:{}: `{w}`", l.no));
                }
            }
        }
    }
    common::report("keywords outside the closed set", failures);
}

#[test]
fn a_note_an_example_and_an_informative_section_carry_no_keyword() {
    let mut failures = Vec::new();
    let mut informative = 0;
    for path in TEXTS {
        for l in lines(path).into_iter().filter(|l| l.block == Block::Informative) {
            informative += 1;
            for w in words(&l.text) {
                if KEYWORDS.contains(&w) || FOREIGN.contains(&w) {
                    failures.push(format!("{path}:{}: `{w}` in informative content", l.no));
                }
            }
        }
    }
    assert!(informative > 0, "the text marks no informative block");
    common::report("keywords in informative content", failures);
}

#[test]
fn a_requirement_of_the_text_carries_one_keyword_and_names_its_class() {
    let mut failures = Vec::new();
    let mut seen = 0;
    for path in TEXTS {
        for (no, s) in sentences(path) {
            if is_keyword_sentence(&s) {
                continue;
            }
            let n = keyword_count(&s);
            if n == 0 {
                continue;
            }
            seen += 1;
            if n > 1 {
                failures.push(format!("{path}:{no}: {n} keywords in `{s}`"));
                continue;
            }
            let ws = words(&s);
            let at = ws.iter().position(|w| KEYWORDS.contains(w)).unwrap();
            let before: Vec<String> = ws[..at].iter().map(|w| w.to_lowercase()).collect();
            if !SUBJECTS.iter().any(|c| before.iter().any(|w| w == c)) {
                failures.push(format!("{path}:{no}: no class before the keyword in `{s}`"));
            }
        }
    }
    assert!(seen >= 20, "{seen} requirements: the text has none to judge");
    common::report("requirements without one keyword and a class", failures);
}

fn title_of(heading: &str) -> String {
    heading[3..].trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ').trim().to_string()
}

#[test]
fn the_text_opens_with_scope_conformance_normative_references_and_terms() {
    let titles: Vec<String> = text("spec/0.6.md").lines().filter(|l| l.starts_with("## ")).map(title_of).collect();
    let head: Vec<&str> = titles.iter().take(4).map(String::as_str).collect();
    assert_eq!(head, ["Scope", "Conformance", "Normative references", "Terms and definitions"]);
}

fn section_of(title: &str) -> String {
    let mut inside = false;
    let mut out = String::new();
    for l in text("spec/0.6.md").lines() {
        if l.starts_with("## ") {
            inside = title_of(l) == title;
            continue;
        }
        if inside {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
}

#[test]
fn the_normative_references_name_rfc_2119_and_rfc_8174() {
    let refs = section_of("Normative references");
    assert!(refs.contains("RFC 2119"), "no RFC 2119");
    assert!(refs.contains("RFC 8174"), "no RFC 8174");
}

#[test]
fn the_conformance_section_names_the_classes_and_the_keyword_sentence() {
    let c = section_of("Conformance");
    for class in CLASSES {
        assert!(c.contains(&format!("A {class}")), "no class `{class}`");
    }
    assert!(c.contains("RFC 2119") && c.contains("RFC 8174"), "no keyword sentence citing both RFCs");
    assert!(c.contains("all capitals"), "the keyword sentence does not say the capitals carry the meaning");
}

#[test]
fn each_class_has_a_requirement_on_an_input_that_does_not_conform() {
    let c = section_of("Conformance");
    for class in CLASSES {
        let found = c.split(['.', '\n']).any(|s| {
            words(s).iter().any(|x| x.eq_ignore_ascii_case(class))
                && keyword_count(s) == 1
                && (s.contains("not conform") || s.contains("refus") || s.contains("fault") || s.contains("report"))
        });
        assert!(found, "no requirement of the conformance section says what a {class} does on an input that does not conform");
    }
}

#[test]
fn a_reader_passes_every_case_of_each_version_it_reads_with_lua_and_identity_cases_optional() {
    let c = section_of("Conformance");
    let low = c.to_lowercase();
    assert!(low.contains("every conformance case") || low.contains("every case"), "no sentence on every case");
    assert!(low.contains("version it reads") || low.contains("versions it reads"), "no sentence on the versions a reader reads");
    assert!(c.contains("Lua"), "the Lua cases are not named");
    assert!(low.contains("identity"), "the identity cases are not named");
    assert!(low.contains("optional") || c.contains("MAY"), "the optional cases are not marked");
}

#[test]
fn a_reader_without_lua_reports_a_descriptor_that_holds_some() {
    let c = section_of("Conformance");
    let hit = c.split('.').any(|s| s.contains("Lua") && s.contains("reader") && s.contains("MUST") && s.to_lowercase().contains("report"));
    assert!(hit, "no requirement says a reader without Lua MUST report a descriptor that holds Lua");
}

#[test]
fn the_schema_prevails_for_shape_and_the_cases_for_meaning() {
    let c = section_of("Conformance").to_lowercase();
    let s = c.split('.').find(|s| s.contains("schema") && s.contains("prevail")).expect("no sentence on the schema prevailing");
    assert!(s.contains("shape") || s.contains("form"), "the schema does not prevail for the form: {s}");
    let m = c.split('.').find(|s| s.contains("conformance case") && s.contains("prevail")).expect("no sentence on the cases prevailing");
    assert!(m.contains("meaning"), "the cases do not prevail for the meaning: {m}");
}

#[test]
fn the_charter_of_the_specification_says_the_text_yields_to_the_schema_and_the_cases() {
    let cadre = text("docs/CADRE.md");
    let r1 = cadre.lines().find(|l| l.starts_with("- **R1**")).expect("no docs/CADRE.md R1");
    assert!(!r1.contains("no other document states what the format requires"), "docs/CADRE.md R1 still says no other document states the format");
    let low = r1.to_lowercase();
    assert!(low.contains("schema") && low.contains("conformance cases"), "docs/CADRE.md R1 does not name the schema and the cases");
    assert!(low.contains("yields") || low.contains("cedes") || low.contains("gives way"), "docs/CADRE.md R1 does not say the text yields");
}
