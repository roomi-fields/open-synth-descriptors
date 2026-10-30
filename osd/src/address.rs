//! The addresses of a descriptor that take captures. 0.4 wrote them as
//! strings with `{name}` inside; 0.5 writes an address without capture as a
//! string without brace, and one with captures as an array of segments,
//! each a string without brace or `{"capture": "name"}`. This module reads
//! both forms, checks each against its version, and converts the 0.4
//! strings into segments.

use crate::json::{At, Codec};
use crate::{Fault, FaultKind};
use serde_json::{Map, Value};

/// The most segments an address holds: the schema bounds the check that two
/// captures are not side by side.
const MAX_SEGMENTS: usize = 16;

/// One piece of an address in segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    /// Text, taken as it is.
    Text(String),
    /// The value of the field of this name.
    Capture(String),
}

/// An address that may take captures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Address {
    /// A string: in 0.5 without brace, in 0.4 with `{name}` for a capture.
    Text(String),
    /// The concatenation of its segments (0.5), never split on a slash.
    Segments(Vec<Segment>),
}

/// The places where an address is written, by what it may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AddressPlace {
    /// `commands[].osc`: recognised, starts with `/`.
    Command,
    /// `replies[].match_osc`: recognised.
    Recognised,
    /// `forward.path`: emitted.
    Emitted,
    /// `replies[].emit_osc`: emitted, without space (its arguments are `emit_args`).
    Reported,
}

impl AddressPlace {
    /// Where an address is recognised, two captures side by side are
    /// ambiguous: the first takes everything.
    fn recognises(self) -> bool {
        matches!(self, AddressPlace::Command | AddressPlace::Recognised)
    }
}

/// A capture or argument name: `[A-Za-z_][A-Za-z0-9_]*`.
pub(crate) fn name_ok(n: &str) -> bool {
    let mut chars = n.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The name of an object `{"<key>": "name"}` of that single key.
pub(crate) fn named(o: &Map<String, Value>, key: &str) -> Option<String> {
    match (o.len(), o.get(key)) {
        (1, Some(Value::String(n))) if name_ok(n) => Some(n.clone()),
        _ => None,
    }
}

fn bad(at: &At, rule: impl Into<String>) -> Fault {
    at.fault(FaultKind::BadValue, rule)
}

fn at_pointer(pointer: &str, rule: &str) -> Fault {
    Fault { kind: FaultKind::BadValue, file: None, pointer: pointer.to_string(), rule: rule.to_string() }
}

pub(crate) struct AddressCodec(pub(crate) AddressPlace);

impl AddressCodec {
    fn segments(&self, a: &[Value], at: &At) -> Result<Vec<Segment>, Fault> {
        if a.is_empty() || a.len() > MAX_SEGMENTS {
            return Err(bad(at, format!("an array of 1 to {MAX_SEGMENTS} segments")));
        }
        let mut out = Vec::new();
        for (i, x) in a.iter().enumerate() {
            let here = At::Index(at, i);
            out.push(match x {
                Value::String(s) if s.contains(['{', '}']) => {
                    return Err(bad(&here, "a string without brace (a capture is {\"capture\": name})"));
                }
                Value::String(s) if self.0 == AddressPlace::Reported && s.contains(' ') => {
                    return Err(bad(&here, "a string without space (the arguments are emit_args)"));
                }
                Value::String(s) => Segment::Text(s.clone()),
                Value::Object(o) => match named(o, "capture") {
                    Some(n) => Segment::Capture(n),
                    None => return Err(bad(&here, "a capture {\"capture\": name}, the name from letters, digits and _")),
                },
                _ => return Err(bad(&here, "a string without brace, or a capture {\"capture\": name}")),
            });
        }
        if !out.iter().any(|s| matches!(s, Segment::Capture(_))) {
            return Err(bad(at, "at least one capture (an address without capture is a string)"));
        }
        if self.0 == AddressPlace::Command && !matches!(&out[0], Segment::Text(s) if s.starts_with('/')) {
            return Err(bad(&At::Index(at, 0), "a string starting with /"));
        }
        if self.0.recognises() {
            if let Some(i) = out.windows(2).position(|w| w.iter().all(|s| matches!(s, Segment::Capture(_)))) {
                return Err(bad(&At::Index(at, i + 1), "not a capture next to a capture: where an address is recognised they are ambiguous"));
            }
        }
        Ok(out)
    }
}

impl Codec for AddressCodec {
    type T = Address;
    fn read(&self, v: &Value, at: &At) -> Result<Address, Fault> {
        match v {
            Value::String(s) if self.0 == AddressPlace::Command && !s.starts_with('/') => Err(bad(at, "a string starting with /")),
            Value::String(s) => Ok(Address::Text(s.clone())),
            Value::Array(a) => self.segments(a, at).map(Address::Segments),
            _ => Err(bad(at, "a string, or an array of segments")),
        }
    }
    fn write(&self, t: &Address) -> Option<Value> {
        Some(match t {
            Address::Text(s) => Value::String(s.clone()),
            Address::Segments(l) => Value::Array(
                l.iter()
                    .map(|s| match s {
                        Segment::Text(t) => Value::String(t.clone()),
                        Segment::Capture(n) => Value::Object(Map::from_iter([("capture".to_string(), Value::String(n.clone()))])),
                    })
                    .collect(),
            ),
        })
    }
}

/// Whether an address, read in version `v05` or not, is of that version: a
/// 0.5 string holds no brace (nor space where it is reported), and segments
/// are not 0.4.
pub(crate) fn check_address(a: &Address, place: AddressPlace, pointer: &str, v05: bool) -> Result<(), Fault> {
    match (a, v05) {
        (Address::Text(s), true) if s.contains(['{', '}']) => {
            Err(at_pointer(pointer, "a string without brace (an address with captures is an array of segments)"))
        }
        (Address::Text(s), true) if place == AddressPlace::Reported && s.contains(' ') => {
            Err(at_pointer(pointer, "a string without space (the arguments are emit_args)"))
        }
        (Address::Segments(_), false) => Err(at_pointer(pointer, "a string (segments are 0.5)")),
        _ => Ok(()),
    }
}

/// The segments of a 0.4 string `s`, or why it has none; a string without
/// brace stays a string.
pub(crate) fn split_template(s: &str, recognised: bool) -> Result<Address, String> {
    if !s.contains(['{', '}']) {
        return Ok(Address::Text(s.to_string()));
    }
    let mut out: Vec<Segment> = Vec::new();
    let mut text = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(['{', '}']) {
        text.push_str(&rest[..i]);
        if rest[i..].starts_with('}') {
            return Err("a } closes no {".into());
        }
        let after = &rest[i + 1..];
        let Some(j) = after.find('}') else { return Err("a { is never closed".into()) };
        let name = &after[..j];
        if !name_ok(name) {
            return Err(format!("{{{name}}} is not a name (letters, digits and _, not starting with a digit)"));
        }
        if !text.is_empty() {
            out.push(Segment::Text(std::mem::take(&mut text)));
        }
        if recognised && matches!(out.last(), Some(Segment::Capture(_))) {
            return Err("two captures side by side are ambiguous where an address is recognised".into());
        }
        out.push(Segment::Capture(name.to_string()));
        rest = &after[j + 1..];
    }
    text.push_str(rest);
    if !text.is_empty() {
        out.push(Segment::Text(text));
    }
    Ok(Address::Segments(out))
}

/// The structure of a 0.4 address at `at`.
pub(crate) fn convert_address(a: &Address, place: AddressPlace, at: &At) -> Result<Address, Fault> {
    let Address::Text(s) = a else { return Ok(a.clone()) };
    split_template(s, place.recognises())
        .map_err(|why| at.fault(FaultKind::NotConvertible, format!("the address {s:?}: {why}")))
}
