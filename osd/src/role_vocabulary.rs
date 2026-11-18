//! The vocabulary of roles: numbered states of terms, each a part of an
//! instrument or a setting of a part, written as data under
//! `schemas/vocabulary/<state>/terms.json`. A published state never changes;
//! the next state only adds. A state is read strictly, like a descriptor, and
//! a role is checked against the state its descriptor names. The README of the
//! vocabulary (`docs/vocabulary.md`) states the rule of entry.

use crate::json::{closed, int, list, list_of, matching, obj, object, words, At, Object, Opt, STRING};
use crate::{Fault, FaultKind};
use serde_json::Value;
use std::sync::OnceLock;

/// The states of the vocabulary this crate holds, the folders of
/// `schemas/vocabulary/`.
pub const VOCABULARY_STATES: &[&str] = &["1"];

const STATE_1: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../schemas/vocabulary/1/terms.json"));

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
    /// A term of the vocabulary.
    Term {
        "name" name: String = matching(is_name, "of lowercase letters, digits and hyphens, starting with a letter"),
        "type" kind: TermKind = closed(),
        "definition" definition: String = matching(is_one_sentence, "of one sentence"),
        /// The parts a setting applies to.
        "applies_to" applies_to: Option<Vec<String>> = Opt(list(STRING)),
        "words" words: Vec<MakerWord> = list(obj()),
        /// The controllers whose composition uses the term.
        "source" source: Vec<Controller> = list_of(obj(), 1, usize::MAX),
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
        (TermKind::Setting, None) => Err(fault(FaultKind::Missing, format!("{here}/applies_to"), "applies_to is required for a setting")),
        (TermKind::Setting, Some(a)) if a.is_empty() => {
            Err(fault(FaultKind::BadValue, format!("{here}/applies_to"), "at least one part"))
        }
        (TermKind::Part, Some(_)) => Err(fault(FaultKind::BadValue, format!("{here}/applies_to"), "a part applies to nothing")),
        _ => Ok(()),
    }
}

/// A name or a word in the form compared: lowercase words joined by one space.
fn norm(s: &str) -> String {
    s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

impl Term {
    fn controllers(&self) -> impl Iterator<Item = u8> + '_ {
        self.source.iter().map(|c| c.number)
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
    // The compositions the state holds: a part and a setting that applies to it.
    let compositions = |except: usize| -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (p, tp) in v.terms.iter().enumerate().filter(|(p, t)| *p != except && t.kind == TermKind::Part) {
            for (s, ts) in v.terms.iter().enumerate().filter(|(s, t)| *s != except && t.kind == TermKind::Setting) {
                if ts.applies_to.iter().flatten().any(|a| *a == tp.name) {
                    out.push((p, s));
                }
            }
        }
        out
    };
    for (i, t) in v.terms.iter().enumerate() {
        let here = format!("/terms/{i}");
        // One name.
        if v.terms[..i].iter().any(|e| e.name == t.name) {
            return Err(fault(FaultKind::BadValue, format!("{here}/name"), format!("{} is the name of an earlier term", t.name)));
        }
        // A function enters once: no term whose name or word is a part and a setting it holds,
        // nor a setting of a part whose controllers that part and another setting already compose.
        let forms: Vec<(String, String)> = compositions(i)
            .into_iter()
            .map(|(p, s)| (format!("{} {}", norm(&v.terms[p].name), norm(&v.terms[s].name)), format!("{} + {}", v.terms[p].name, v.terms[s].name)))
            .collect();
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
                    if t.controllers().all(|c| shared.contains(&c)) {
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
