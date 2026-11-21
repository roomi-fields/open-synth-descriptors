//! The vocabulary of roles: numbered states of terms, each a part of an
//! instrument or a setting of a part, written as data under
//! `schemas/vocabulary/<state>/terms.json`. A published state never changes;
//! the next state only adds. A state is read strictly, like a descriptor, and
//! a role is checked against the state its descriptor names. The README of the
//! vocabulary (`docs/vocabulary.md`) states the rule of entry.

use crate::json::{closed, int, list, list_of, matching, obj, object, words, At, Bool, Codec, Object, Opt, STRING};
use crate::{Fault, FaultKind};
use serde_json::Value;
use std::sync::OnceLock;

/// The states of the vocabulary this crate holds, the folders of
/// `schemas/vocabulary/`.
pub const VOCABULARY_STATES: &[&str] = &["1"];

const STATE_1: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/vocabulary-state-1.json"));

/// The published state `state` of the vocabulary, or none when this crate
/// does not hold it.
pub fn vocabulary_state(state: &str) -> Option<&'static Vocabulary> {
    static ONE: OnceLock<Vocabulary> = OnceLock::new();
    match state {
        "1" => Some(ONE.get_or_init(|| read_vocabulary(STATE_1).expect("state 1 of the vocabulary is valid"))),
        _ => None,
    }
}

words! {
    /// What a term is: a part of the instrument, or what is set on a part.
    TermKind {
        Part = "part",
        Setting = "setting",
    }
}

const NOT_EMPTY: crate::json::Text = matching(|s| !s.is_empty(), "that is not empty");

object! {
    /// A controller of the MIDI 1.0 table of defined controllers.
    Controller {
        "controller" number: u8 = int(0, 127),
        "name" name: String = NOT_EMPTY,
    }
}

object! {
    /// A word a maker uses for a term, with where it comes from.
    MakerWord {
        "word" word: String = NOT_EMPTY,
        /// The specification (the MIDI 1.0 table, Property Exchange) or the
        /// maker that uses the word.
        "origin" origin: String = NOT_EMPTY,
    }
}

object! {
    /// A row of a table of a published document (M2-104-UM table 22,
    /// M2-117-UM tables 6 and 12) a term is taken from.
    DocumentSource {
        "document" document: String = NOT_EMPTY,
        "table" table: String = NOT_EMPTY,
        /// The number of the row, when the table numbers its rows.
        "number" number: Option<u8> = Opt(int(0, 127)),
        "name" name: String = NOT_EMPTY,
    }
}

/// Where a term comes from: a controller of the MIDI 1.0 table, or a row of
/// a table of a published document.
#[derive(Clone, Debug, PartialEq)]
pub enum TermSource {
    Controller(Controller),
    Document(DocumentSource),
}

/// A source is read by its form: an object with `document` cites a document.
pub struct SourceCodec;

impl Codec for SourceCodec {
    type T = TermSource;
    fn read(&self, v: &Value, at: &At) -> Result<TermSource, Fault> {
        if v.as_object().is_some_and(|o| o.contains_key("document")) {
            DocumentSource::read(v, at).map(TermSource::Document)
        } else {
            Controller::read(v, at).map(TermSource::Controller)
        }
    }
    fn write(&self, t: &TermSource) -> Option<Value> {
        Some(match t {
            TermSource::Controller(c) => c.write(),
            TermSource::Document(d) => d.write(),
        })
    }
}

object! {
    /// A device that uses a term, with its maker.
    SharedDevice {
        "maker" maker: String = NOT_EMPTY,
        "device" device: String = NOT_EMPTY,
    }
}

object! {
    /// A term of the vocabulary.
    Term {
        "name" name: String = matching(is_name, "of lowercase letters, digits and hyphens, starting with a letter"),
        "type" kind: TermKind = closed(),
        "definition" definition: String = matching(is_one_sentence, "of one sentence"),
        /// The parts a setting applies to.
        "applies_to" applies_to: Option<Vec<String>> = Opt(list(STRING)),
        /// The parts a part may stand under in a path.
        "stands_under" stands_under: Option<Vec<String>> = Opt(list_of(STRING, 1, usize::MAX)),
        /// Whether a part may open a path; true when `stands_under` is absent, false when it is present.
        "at_top" at_top: Option<bool> = Opt(Bool),
        "words" words: Vec<MakerWord> = list(obj()),
        /// Where the term comes from: MIDI 1.0 controllers whose composition uses it, rows of a published document.
        "source" source: Option<Vec<TermSource>> = Opt(list_of(SourceCodec, 1, usize::MAX)),
        /// Devices of two makers at least that share the term.
        "shared_by" shared_by: Option<Vec<SharedDevice>> = Opt(list(obj())),
        /// The term of the same state that replaces this one.
        "replaced_by" replaced_by: Option<String> = Opt(STRING),
    }
    check check_term
}

object! {
    /// A state of the vocabulary.
    Vocabulary {
        "$schema" schema: Option<String> = Opt(STRING),
        "state" state: String = matching(is_state, "of digits, the number of the state"),
        "terms" terms: Vec<Term> = list(obj()),
    }
    check check_state
}

fn is_name(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|c| c.is_ascii_lowercase()) && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_state(s: &str) -> bool {
    !s.starts_with('0') && !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// One sentence: not empty, and no `.`, `!` or `?` followed by a space.
fn is_one_sentence(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let end = |c: char| matches!(c, '.' | '!' | '?');
    chars.first().is_some_and(|c| !c.is_whitespace() && !end(*c))
        && chars.last().is_some_and(|c| !c.is_whitespace())
        && !chars.windows(2).any(|w| end(w[0]) && w[1].is_whitespace())
}

fn fault(kind: FaultKind, pointer: String, rule: impl Into<String>) -> Fault {
    Fault { kind, file: None, pointer, rule: rule.into() }
}

fn check_term(t: &Term, at: &At) -> Result<(), Fault> {
    let here = at.pointer();
    match (t.kind, &t.applies_to) {
        (TermKind::Setting, None) => return Err(fault(FaultKind::Missing, format!("{here}/applies_to"), "applies_to is required for a setting")),
        (TermKind::Setting, Some(a)) if a.is_empty() => {
            return Err(fault(FaultKind::BadValue, format!("{here}/applies_to"), "at least one part"))
        }
        (TermKind::Part, Some(_)) => return Err(fault(FaultKind::BadValue, format!("{here}/applies_to"), "a part applies to nothing")),
        _ => {}
    }
    if t.kind == TermKind::Setting {
        if t.stands_under.is_some() {
            return Err(fault(FaultKind::BadValue, format!("{here}/stands_under"), "a setting stands under no part: it applies to parts"));
        }
        if t.at_top.is_some() {
            return Err(fault(FaultKind::BadValue, format!("{here}/at_top"), "a setting does not open a path"));
        }
    }
    if t.source.is_none() && t.shared_by.is_none() {
        return Err(fault(FaultKind::Missing, format!("{here}/source"), "a term has a source or says where it is shared (shared_by)"));
    }
    if let Some(shared) = &t.shared_by {
        let makers: std::collections::BTreeSet<&str> = shared.iter().map(|d| d.maker.as_str()).collect();
        let devices: std::collections::BTreeSet<(&str, &str)> = shared.iter().map(|d| (d.maker.as_str(), d.device.as_str())).collect();
        if makers.len() < 2 || devices.len() < 2 {
            return Err(fault(FaultKind::BadValue, format!("{here}/shared_by"), "at least two devices of two makers"));
        }
    }
    Ok(())
}

/// The segment of a term in a `paramPath` (M2-117-UM): each word between hyphens
/// starts with a capital letter and the hyphens go (`attack-time` is `AttackTime`).
pub(crate) fn segment(name: &str) -> String {
    name.split('-')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
        })
        .collect()
}

/// A name or a word in the form compared: lowercase words joined by one space.
fn norm(s: &str) -> String {
    s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

impl Term {
    /// The MIDI 1.0 controllers the term cites.
    fn controllers(&self) -> impl Iterator<Item = u8> + '_ {
        self.source.iter().flatten().filter_map(|s| match s {
            TermSource::Controller(c) => Some(c.number),
            TermSource::Document(_) => None,
        })
    }

    /// Whether a path may open with this part: `at_top`, which is true when `stands_under`
    /// is absent and false when it is present, unless the part writes it.
    pub(crate) fn opens_a_path(&self) -> bool {
        self.at_top.unwrap_or(self.stands_under.is_none())
    }
}

impl Vocabulary {
    /// The chains of parts that follow `stands_under`, from any part down: each part stands
    /// under the one before it. A chain holds a part once.
    fn chains(&self) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        let mut stack: Vec<Vec<usize>> = (0..self.terms.len()).filter(|i| self.terms[*i].kind == TermKind::Part).map(|i| vec![i]).collect();
        while let Some(chain) = stack.pop() {
            let last = &self.terms[*chain.last().expect("a chain holds a part")];
            if chain.len() < 4 {
                for (i, t) in self.terms.iter().enumerate() {
                    if t.kind == TermKind::Part && !chain.contains(&i) && t.stands_under.iter().flatten().any(|u| *u == last.name) {
                        let mut longer = chain.clone();
                        longer.push(i);
                        stack.push(longer);
                    }
                }
            }
            out.push(chain);
        }
        out
    }
}

fn check_state(v: &Vocabulary, _: &At) -> Result<(), Fault> {
    if let Some(s) = &v.schema {
        let want = format!("https://w3id.org/osd/vocabulary/{}/vocabulary.schema.json", v.state);
        if *s != want {
            return Err(fault(FaultKind::BadValue, "/$schema".into(), format!("`{want}`")));
        }
    }
    let part = |name: &str| v.terms.iter().find(|t| t.name == name && t.kind == TermKind::Part);
    // The compositions the state holds: a chain of parts, each under the one before, and a
    // setting that applies to the last (a part alone is a chain of one).
    let chains = v.chains();
    let compositions = |except: usize| -> Vec<(String, String)> {
        let mut out = Vec::new();
        for chain in chains.iter().filter(|c| !c.contains(&except)) {
            let last = &v.terms[*chain.last().expect("a chain holds a part")].name;
            for ts in v.terms.iter().enumerate().filter(|(s, t)| *s != except && t.kind == TermKind::Setting && t.applies_to.iter().flatten().any(|a| a == last)).map(|(_, t)| t) {
                let names: Vec<&str> = chain.iter().map(|p| v.terms[*p].name.as_str()).chain(std::iter::once(ts.name.as_str())).collect();
                out.push((names.iter().map(|n| norm(n)).collect::<Vec<_>>().join(" "), names.join(" + ")));
            }
        }
        out
    };
    // Two terms are one segment of a path when their CamelCase segments are the same.
    let segments: Vec<String> = v.terms.iter().map(|t| segment(&t.name)).collect();
    for (i, t) in v.terms.iter().enumerate() {
        let here = format!("/terms/{i}");
        // One name.
        if v.terms[..i].iter().any(|e| e.name == t.name) {
            return Err(fault(FaultKind::BadValue, format!("{here}/name"), format!("{} is the name of an earlier term", t.name)));
        }
        // One segment: `x1` and `x-1` would both be `X1` in a path.
        if let Some(e) = segments[..i].iter().position(|s| *s == segments[i]) {
            return Err(fault(FaultKind::BadValue, format!("{here}/name"), format!("{} says the segment {} of {}", t.name, segments[i], v.terms[e].name)));
        }
        // A function enters once: no term whose name or word is a path of parts and a setting it holds,
        // nor a setting of a part whose controllers that part and another setting already compose.
        let forms = compositions(i);
        for form in std::iter::once(t.name.as_str()).chain(t.words.iter().map(|w| w.word.as_str())) {
            if let Some((_, what)) = forms.iter().find(|(f, _)| *f == norm(form)) {
                return Err(fault(FaultKind::BadValue, here, format!("`{form}` is {what}, which the state already holds")));
            }
        }
        if t.kind == TermKind::Setting {
            for tp in t.applies_to.iter().flatten().filter_map(|a| part(a)) {
                // Against the settings before it: the later term is the one that says it again.
                for other in v.terms[..i].iter().filter(|o| o.kind == TermKind::Setting) {
                    if !other.applies_to.iter().flatten().any(|a| *a == tp.name) {
                        continue;
                    }
                    let shared: Vec<u8> = tp.controllers().filter(|c| other.controllers().any(|o| o == *c)).collect();
                    if t.controllers().next().is_some() && t.controllers().all(|c| shared.contains(&c)) {
                        return Err(fault(
                            FaultKind::BadValue,
                            here,
                            format!("{} is {} + {}, which the state already holds", t.name, tp.name, other.name),
                        ));
                    }
                }
            }
        }
        // One word, one term.
        if let Some(e) = v.terms[..i].iter().find(|e| e.words.iter().any(|w| norm(&w.word) == norm(&t.name))) {
            return Err(fault(FaultKind::BadValue, format!("{here}/name"), format!("{} is a maker's word of {}", t.name, e.name)));
        }
        for (k, w) in t.words.iter().enumerate() {
            let word = norm(&w.word);
            let earlier_word = t.words[..k].iter().any(|x| norm(&x.word) == word)
                || v.terms[..i].iter().any(|e| e.words.iter().any(|x| norm(&x.word) == word));
            let a_name = v.terms[..i].iter().any(|e| norm(&e.name) == word);
            if earlier_word || a_name {
                return Err(fault(FaultKind::BadValue, format!("{here}/words/{k}/word"), format!("`{}` already means a term", w.word)));
            }
        }
        for (k, a) in t.applies_to.iter().flatten().enumerate() {
            if part(a).is_none() {
                return Err(fault(FaultKind::BadValue, format!("{here}/applies_to/{k}"), format!("{a} is not a part of the state")));
            }
        }
        for (k, a) in t.stands_under.iter().flatten().enumerate() {
            if part(a).is_none() {
                return Err(fault(FaultKind::BadValue, format!("{here}/stands_under/{k}"), format!("{a} is not a part of the state")));
            }
        }
        if let Some(r) = &t.replaced_by {
            if *r == t.name || !v.terms.iter().any(|e| e.name == *r) {
                return Err(fault(FaultKind::BadValue, format!("{here}/replaced_by"), format!("{r} is not another term of the state")));
            }
        }
    }
    Ok(())
}

/// A state of the vocabulary read from its text, every field checked.
pub fn read_vocabulary(text: &str) -> Result<Vocabulary, Fault> {
    let v: Value = serde_json::from_str(text).map_err(|e| At::Root.fault(FaultKind::Syntax, e.to_string()))?;
    Vocabulary::read(&v, &At::Root)
}

impl Vocabulary {
    /// The term named `name`.
    pub fn term(&self, name: &str) -> Option<&Term> {
        self.terms.iter().find(|t| t.name == name)
    }

    /// Whether `next` only adds to this state: each term stays, with its type;
    /// a term a later state replaces stays and says `replaced_by`.
    pub fn check_successor(&self, next: &Vocabulary) -> Result<(), Fault> {
        for t in &self.terms {
            match next.term(&t.name) {
                Some(n) if n.kind == t.kind => {}
                Some(_) => return Err(fault(FaultKind::BadValue, "/terms".into(), format!("{} changes its type", t.name))),
                None => return Err(fault(FaultKind::BadValue, "/terms".into(), format!("{} is not in the next state: a term stays", t.name))),
            }
        }
        Ok(())
    }
}
