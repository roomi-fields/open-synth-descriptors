//! The trial of 0.7 on a catalogue (a folder of `<maker>/<name>.json`): every
//! driver and maker profile converted to 0.7, read, checked against the
//! schemas of 0.7 and against what its 0.6 text said (`spec/0.7.md`), and
//! every driver resolved. The 0.6 reading of the same catalogue is the
//! reference: what it already refuses is counted, not failed.

use super::{reference_0_6, Folder, Schemas};
use osd::{Descriptor, Fault, FaultKind, Kind, Version};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// What a failure of the trial is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// A file 0.6 accepts and 0.7 refuses.
    Refusal,
    /// A driver 0.6 resolves and 0.7 refuses to resolve.
    Resolution,
    /// A file the reader accepts in 0.7 and the schema of 0.7 refuses.
    Schema,
    /// A value or a source that a file accepted in 0.7 says otherwise than its
    /// 0.6 text: lost, changed or added.
    Loss,
    /// A text accepted in 0.7 that is written back as another JSON, or that a
    /// conversion to 0.7 changes again.
    Unstable,
}

/// One failure: the file, the JSON pointer in it, and why.
#[derive(Clone, Debug)]
pub struct Failure {
    pub check: Check,
    pub file: String,
    pub pointer: String,
    /// The kind of fault of a refusal; none for a schema or a loss.
    pub kind: Option<FaultKind>,
    pub why: String,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pointer = if self.pointer.is_empty() { "(the file)" } else { &self.pointer };
        write!(f, "{:?}: {}: {pointer}: ", self.check, self.file)?;
        if let Some(kind) = self.kind {
            write!(f, "{kind:?}: ")?;
        }
        f.write_str(&self.why)
    }
}

/// What a trial found.
#[derive(Debug, Default)]
pub struct Trial {
    /// The drivers and profiles tried.
    pub files: usize,
    /// The files refused by the 0.6 reading, by kind of fault.
    pub refused_in_0_6: HashMap<FaultKind, usize>,
    /// The files refused by the 0.7 reading, by kind of fault.
    pub refused_in_0_7: HashMap<FaultKind, usize>,
    pub failures: Vec<Failure>,
}

impl std::fmt::Display for Trial {
    /// The counts, of refusals by kind of fault and of differences with the
    /// 0.6 text, then every failure.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} descriptors tried", self.files)?;
        for (version, refused) in [("0.6", &self.refused_in_0_6), ("0.7", &self.refused_in_0_7)] {
            let sorted: BTreeMap<String, usize> = refused.iter().map(|(k, n)| (format!("{k:?}"), *n)).collect();
            writeln!(f, "refused in {version}: {} {sorted:?}", sorted.values().sum::<usize>())?;
        }
        let differences = self.failures.iter().filter(|x| x.check == Check::Loss).count();
        writeln!(f, "differences with the 0.6 text: {differences}")?;
        writeln!(f, "{} failures", self.failures.len())?;
        self.failures.iter().try_for_each(|x| writeln!(f, "{x}"))
    }
}

/// The converted texts of a catalogue in one version, by catalogue id: a file
/// whose conversion is refused is absent.
type Converted = BTreeMap<String, String>;

/// The trial of 0.7 on the catalogue `dir`.
pub fn trial(dir: &Folder) -> Trial {
    let schemas = Schemas::load("0.7");
    let mut out = Trial::default();
    let (mut in_0_6, mut in_0_7) = (Converted::new(), Converted::new());
    let drivers = dir.drivers();
    let all = drivers.iter().map(|id| (Kind::Device, id.clone()));
    let all = all.chain(dir.profiles().into_iter().map(|id| (Kind::Profile, id)));
    for (kind, id) in all {
        out.files += 1;
        let text = dir.text(&id);
        let (old, old_read) = converted_and_read(&text, kind, Version::V0_6, |t| osd::read(t, kind));
        let (new, new_read) = converted_and_read(&text, kind, Version::V0_7, |t| osd::read_as(t, kind, Version::V0_7));
        if let Err(fault) = &old_read {
            *out.refused_in_0_6.entry(fault.kind).or_default() += 1;
        }
        match &new_read {
            Err(fault) => {
                *out.refused_in_0_7.entry(fault.kind).or_default() += 1;
                if old_read.is_ok() {
                    out.failures.push(refusal(Check::Refusal, &id, fault));
                }
            }
            Ok(_) => {
                let new = new.as_deref().expect("a text read was converted");
                match (&old_read, old.as_deref()) {
                    (Ok(_), Some(old)) => out.failures.extend(judge_from(&schemas, &id, kind, &text, old, new)),
                    // No 0.6 text to compare with: the schema still holds.
                    _ => out.failures.extend(schema_failure(&schemas, &id, kind, new)),
                }
            }
        }
        in_0_6.extend(old.map(|t| (id.clone(), t)));
        in_0_7.extend(new.map(|t| (id, t)));
    }
    for id in &drivers {
        // A driver whose own conversion is refused is already judged as a file.
        let Some(_) = in_0_7.get(id) else { continue };
        if let (Ok(_), Err(fault)) = (osd::resolve(id, &in_0_6), osd::resolve(id, &in_0_7)) {
            let file = fault.file.clone().unwrap_or_else(|| id.clone());
            let mut failure = refusal(Check::Resolution, &file, &fault);
            failure.why = format!("resolving {id}: {}", failure.why);
            out.failures.push(failure);
        }
    }
    out
}

/// The text of `text` in `target`, when it converts, and its reading.
fn converted_and_read(
    text: &str,
    kind: Kind,
    target: Version,
    read: impl Fn(&str) -> Result<Descriptor, Fault>,
) -> (Option<String>, Result<Descriptor, Fault>) {
    match osd::convert_to(text, kind, target) {
        Ok(t) => {
            let d = read(&t);
            (Some(t), d)
        }
        Err(f) => (None, Err(f)),
    }
}

fn refusal(check: Check, file: &str, fault: &Fault) -> Failure {
    Failure { check, file: file.into(), pointer: fault.pointer.clone(), kind: Some(fault.kind), why: fault.rule.clone() }
}

/// What is wrong with `new`, the 0.7 text converted from `old`, a text of 0.6 of
/// the file `file`.
pub fn judge(schemas: &Schemas, file: &str, kind: Kind, old: &str, new: &str) -> Vec<Failure> {
    judge_from(schemas, file, kind, old, old, new)
}

/// What is wrong with `new`, the 0.7 text of the file `file` that the reader
/// accepts, `given` being its text as the catalogue holds it and `old` its 0.6
/// text: the schema of 0.7 refuses it, it differs
/// from what `old` says, it is written back as another JSON, or converting it
/// again to 0.7 changes it.
pub fn judge_from(schemas: &Schemas, file: &str, kind: Kind, given: &str, old: &str, new: &str) -> Vec<Failure> {
    let mut out: Vec<Failure> = schema_failure(schemas, file, kind, new).into_iter().collect();
    let failure = |check, pointer: String, why: String| Failure { check, file: file.into(), pointer, kind: None, why };
    let (before, after) = (json(old), json(new));
    let lost = reference_0_6::document(kind, &json(given), &before, &after);
    out.extend(lost.into_iter().map(|(pointer, why)| failure(Check::Loss, pointer, why)));
    match osd::read_as(new, kind, Version::V0_7) {
        Ok(d) => {
            let back = json(&osd::write(&d));
            if back != after {
                let (pointer, why) = first_difference(&after, &back, "");
                out.push(failure(Check::Unstable, pointer, format!("read then written: {why}")));
            }
        }
        Err(fault) => out.push(refusal(Check::Refusal, file, &fault)),
    }
    match osd::convert_to(new, kind, Version::V0_7) {
        Ok(again) if again == new => {}
        Ok(again) => {
            let (pointer, why) = first_difference(&after, &json(&again), "");
            out.push(failure(Check::Unstable, pointer, format!("converted to 0.7 again, it changes: {why}")));
        }
        Err(fault) => out.push(failure(Check::Unstable, fault.pointer.clone(), format!("converted to 0.7 again: {fault}"))),
    }
    out
}

/// The first error the schema of 0.7 finds in the text, at its pointer.
fn schema_failure(schemas: &Schemas, file: &str, kind: Kind, text: &str) -> Option<Failure> {
    let (pointer, why) = schemas.errors(kind, &json(text)).into_iter().next()?;
    Some(Failure { check: Check::Schema, file: file.into(), pointer, kind: None, why })
}

fn json(text: &str) -> Value {
    serde_json::from_str(text).expect("a text the reader accepts is JSON")
}

/// Where two documents first differ: a JSON pointer, and how.
pub fn first_difference(a: &Value, b: &Value, at: &str) -> (String, String) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                if x.get(k) != y.get(k) {
                    let at = format!("{at}/{}", k.replace('~', "~0").replace('/', "~1"));
                    return match (x.get(k), y.get(k)) {
                        (Some(p), Some(q)) => first_difference(p, q, &at),
                        (p, q) => (at, format!("{p:?} became {q:?}")),
                    };
                }
            }
            (at.to_string(), "differs".into())
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            let i = x.iter().zip(y).position(|(p, q)| p != q).unwrap_or(0);
            match (x.get(i), y.get(i)) {
                (Some(p), Some(q)) => first_difference(p, q, &format!("{at}/{i}")),
                _ => (at.to_string(), "differs".into()),
            }
        }
        _ => (at.to_string(), format!("{a} became {b}")),
    }
}
