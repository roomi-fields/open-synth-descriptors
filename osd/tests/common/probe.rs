//! The probe of the reader against the schemas: valid documents mutated
//! deterministically, and the verdict (accepted or refused) of the reader
//! compared with the verdict of a JSON Schema validator. The targets come
//! from the document; the schema file is only the oracle, and the source of
//! the integer bounds the file declares.

#![allow(dead_code)]

use osd::{Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// A verdict function: the version a document is read in, its kind, the
/// document; true when it accepts.
pub trait Judge: Fn(Version, Kind, &Value) -> bool {}
impl<T: Fn(Version, Kind, &Value) -> bool> Judge for T {}

pub const GENRES: [&str; 8] = [
    "remove-key",
    "add-unknown-key",
    "change-type",
    "integer-bound",
    "unlisted-string",
    "empty-string",
    "empty-array",
    "duplicate-first",
];

/// A document the probe mutates.
pub struct Doc {
    pub name: String,
    pub version: Version,
    pub kind: Kind,
    pub value: Value,
    /// Catalogue documents: at most `CATALOGUE_CAP` mutations, arrays of more
    /// than three elements mutated at their first, second and last only.
    pub bounded: bool,
}

pub const CATALOGUE_CAP: usize = 500;

/// The version a document is read in: 0.4 without `$schema`, else the one
/// its `$schema` names.
pub fn version_of(doc: &Value) -> Version {
    match doc.get("$schema").and_then(Value::as_str) {
        Some(s) if s.contains("/osd/0.5/") => Version::V0_5,
        _ => Version::V0_4,
    }
}

enum Edit {
    Replace(Value),
    Remove(String),
    AddKey(String),
    Empty,
    DuplicateFirst,
}

struct Mutation {
    genre: &'static str,
    /// Where the mutation is told: the key removed, else the value touched.
    pointer: String,
    /// The value the edit applies to (the parent object for a removal).
    target: String,
    edit: Edit,
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// The integer bounds the schema file declares: every minimum, maximum,
/// exclusiveMinimum, exclusiveMaximum, with its neighbours.
pub fn bounds(schema: &Value) -> Vec<i64> {
    fn walk(v: &Value, out: &mut Vec<i64>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    if ["minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum"].contains(&k.as_str()) {
                        if let Some(n) = x.as_i64() {
                            out.extend([n - 1, n, n + 1]);
                        }
                    }
                    walk(x, out);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = vec![-1, 0];
    walk(schema, &mut out);
    out.sort();
    out.dedup();
    out
}

fn other_type(v: &Value) -> Value {
    match v {
        Value::Null => json!("x"),
        Value::Bool(b) => json!(b.to_string()),
        Value::Number(n) => json!(n.to_string()),
        Value::String(_) => json!(0),
        Value::Array(_) => json!({}),
        Value::Object(_) => json!([]),
    }
}

fn collect(v: &Value, at: &str, bounds: &[i64], bounded: bool, out: &mut Vec<Mutation>) {
    let mut push = |genre, pointer: &str, target: &str, edit| {
        out.push(Mutation { genre, pointer: pointer.to_string(), target: target.to_string(), edit })
    };
    push("change-type", at, at, Edit::Replace(other_type(v)));
    match v {
        Value::Object(m) => {
            push("add-unknown-key", at, at, Edit::AddKey("~unknown".into()));
            for k in m.keys() {
                push("remove-key", &format!("{at}/{}", escape(k)), at, Edit::Remove(k.clone()));
            }
            for (k, x) in m {
                collect(x, &format!("{at}/{}", escape(k)), bounds, bounded, out);
            }
        }
        Value::Array(a) => {
            push("empty-array", at, at, Edit::Empty);
            if !a.is_empty() {
                push("duplicate-first", at, at, Edit::DuplicateFirst);
            }
            for (i, x) in a.iter().enumerate() {
                if bounded && a.len() > 3 && i != 0 && i != 1 && i != a.len() - 1 {
                    continue;
                }
                collect(x, &format!("{at}/{i}"), bounds, bounded, out);
            }
        }
        Value::String(s) => {
            if s != "~unlisted" {
                push("unlisted-string", at, at, Edit::Replace(json!("~unlisted")));
            }
            if !s.is_empty() {
                push("empty-string", at, at, Edit::Replace(json!("")));
            }
        }
        Value::Number(n) if n.is_i64() || n.is_u64() => {
            for b in bounds {
                if n.as_i64() != Some(*b) {
                    push("integer-bound", at, at, Edit::Replace(json!(b)));
                }
            }
            push("integer-bound", at, at, Edit::Replace(json!(0.5)));
        }
        _ => {}
    }
}

fn apply(doc: &Value, m: &Mutation) -> Value {
    let mut d = doc.clone();
    let slot = if m.target.is_empty() { Some(&mut d) } else { d.pointer_mut(&m.target) };
    let slot = slot.expect("a mutation targets a value of the document");
    match &m.edit {
        Edit::Replace(v) => *slot = v.clone(),
        Edit::Remove(k) => {
            slot.as_object_mut().unwrap().shift_remove(k);
        }
        Edit::AddKey(k) => {
            slot.as_object_mut().unwrap().insert(k.clone(), json!(0));
        }
        Edit::Empty => *slot = json!([]),
        Edit::DuplicateFirst => {
            let a = slot.as_array_mut().unwrap();
            let first = a[0].clone();
            a.push(first);
        }
    }
    d
}

fn fnv(s: &str) -> usize {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    h as usize
}

/// What a probe did: mutations generated and played, per genre, and the
/// disagreements it found.
#[derive(Default)]
pub struct Outcome {
    pub generated: usize,
    pub played: usize,
    pub played_by_genre: BTreeMap<&'static str, usize>,
    /// Documents skipped because reader and oracle did not both accept them.
    pub not_valid: Vec<String>,
    pub disagreements: Vec<String>,
}

impl Outcome {
    pub fn merge(&mut self, o: Outcome) {
        self.generated += o.generated;
        self.played += o.played;
        for (g, n) in o.played_by_genre {
            *self.played_by_genre.entry(g).or_default() += n;
        }
        self.not_valid.extend(o.not_valid);
        self.disagreements.extend(o.disagreements);
    }
}

/// Mutates each document and compares `reader` with `oracle`; `bounds_of`
/// gives the integer bounds the schema file declares for a version and kind.
pub fn probe(
    docs: &[Doc],
    bounds_of: &dyn Fn(Version, Kind) -> Vec<i64>,
    reader: &dyn Judge,
    oracle: &dyn Judge,
) -> Outcome {
    let mut out = Outcome::default();
    for d in docs {
        if !(reader(d.version, d.kind, &d.value) && oracle(d.version, d.kind, &d.value)) {
            out.not_valid.push(d.name.clone());
            continue;
        }
        let mut all = Vec::new();
        collect(&d.value, "", &bounds_of(d.version, d.kind), d.bounded, &mut all);
        all.sort_by(|a, b| (&a.pointer, a.genre).cmp(&(&b.pointer, b.genre)));
        out.generated += all.len();
        let picked: Vec<&Mutation> = if d.bounded && all.len() > CATALOGUE_CAP {
            let step = all.len().div_ceil(CATALOGUE_CAP);
            let offset = fnv(&d.name) % step;
            all.iter().skip(offset).step_by(step).collect()
        } else {
            all.iter().collect()
        };
        for m in picked {
            let doc = apply(&d.value, m);
            let (r, o) = (reader(d.version, d.kind, &doc), oracle(d.version, d.kind, &doc));
            out.played += 1;
            *out.played_by_genre.entry(m.genre).or_default() += 1;
            if r != o {
                let say = |b| if b { "accepts" } else { "refuses" };
                out.disagreements.push(format!(
                    "version {} {:?} {} mutation {} at \"{}\": reader {}, schema {}",
                    d.version.name(),
                    d.kind,
                    d.name,
                    m.genre,
                    m.pointer,
                    say(r),
                    say(o)
                ));
            }
        }
    }
    out
}
