//! How a file is written on a device by MIDI (0.6): `resources[].via.file_write.steps`, the steps
//! `open`, `write` (played once per chunk) and `close`. Each step has the message it sends (`send`,
//! the elements of a frame plus `text`, `file` and `captured`), the reply awaited (`reply`, the
//! recognised elements of a frame plus `capture_json`) with `reply_timeout_ms`, and the pause
//! `delay_ms` before the next step. The `write` step carries `chunk_bytes` and `coding`.

use crate::frame::{FrameToken, FRAME_TOKEN, MATCH_TOKEN};
use crate::json::{at_least, closed, field, keys, put, put_remarks, remarks, At, Codec, Opt, Remarks, Word};
use crate::{Fault, FaultKind};
use serde_json::{Map, Value};

crate::json::words! {
    /// What a step does.
    StepKind { Open = "open", Write = "write", Close = "close", }
}

crate::json::words! {
    /// How the bytes of a chunk are coded in 7 bits.
    Coding { Mcoded7 = "mcoded7", MsbFirst7in8 = "msb_first_7in8", }
}

crate::json::words! {
    /// A datum of the file a message carries.
    FileItem { Path = "path", Size = "size", Offset = "offset", ChunkSize = "chunk_size", Chunk = "chunk", }
}

crate::json::words! {
    /// How a number is written in a message.
    NumberAs { Decimal = "decimal", }
}

/// The bytes of a chunk coded in 7 bits: each group of seven bytes becomes the byte of the sign
/// bits then the seven low parts; the last group, shorter, is coded the same way. `Mcoded7` (M2-103-UM
/// v1.2, 6.1.7) puts the sign of the first byte of the group at bit 6 of the sign byte, the
/// next at bit 5, and so on; `MsbFirst7in8` puts the sign of byte k at bit k.
pub fn encode(coding: Coding, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + bytes.len().div_ceil(7));
    for group in bytes.chunks(7) {
        let signs = group.iter().enumerate().fold(0u8, |acc, (k, b)| {
            let sign = b >> 7;
            acc | match coding {
                Coding::Mcoded7 => sign << (6 - k),
                Coding::MsbFirst7in8 => sign << k,
            }
        });
        out.push(signs);
        out.extend(group.iter().map(|b| b & 0x7F));
    }
    out
}

/// An element of a message a step sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendToken {
    /// A byte or an element of a frame.
    Frame(FrameToken),
    /// ASCII text, written as its bytes.
    Text(String),
    /// A datum of the file; `as_` writes a number as its ASCII digits.
    File { item: FileItem, as_: Option<NumberAs> },
    /// A value the reply of the open step captured.
    Captured { name: String, as_: Option<NumberAs> },
}

/// An element of a reply awaited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyToken {
    /// A byte or a recognised element of a frame.
    Frame(FrameToken),
    /// The value of this key of a JSON text reply, kept under the same name.
    CaptureJson(String),
}

/// A step of a file write.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub step: StepKind,
    pub send: Vec<SendToken>,
    pub reply: Option<Vec<ReplyToken>>,
    pub reply_timeout_ms: Option<i64>,
    pub delay_ms: Option<i64>,
    /// The write step only: the most bytes of the file one write carries.
    pub chunk_bytes: Option<i64>,
    /// The write step only.
    pub coding: Option<Coding>,
    pub remarks: Remarks,
}

const KEYS: &[&str] = &["step", "send", "reply", "reply_timeout_ms", "delay_ms", "chunk_bytes", "coding"];

fn name_ok(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn name(o: &Map<String, Value>, k: &'static str, at: &At) -> Result<String, Fault> {
    let here = At::Key(at, k);
    match &o[k] {
        Value::String(s) if name_ok(s) => Ok(s.clone()),
        _ => Err(here.fault(FaultKind::BadValue, "a name of letters, digits and _")),
    }
}

/// The primary key of an element object among `primaries`, its other keys checked against `allowed`.
fn primary<'a>(o: &Map<String, Value>, at: &At, primaries: &[&'a str], allowed: impl Fn(&str) -> &'static [&'static str]) -> Result<Option<&'a str>, Fault> {
    let present: Vec<&str> = primaries.iter().copied().filter(|k| o.contains_key(*k)).collect();
    match present[..] {
        [] => Ok(None),
        [p] => {
            let ok = allowed(p);
            match o.keys().find(|k| !ok.contains(&k.as_str())) {
                Some(k) => Err(At::Key(at, k).fault(FaultKind::UnknownField, format!("{k} is not a field of {p} here"))),
                None => Ok(Some(p)),
            }
        }
        _ => Err(at.fault(FaultKind::BadValue, "an object of one key of its nature")),
    }
}

fn number_as(o: &Map<String, Value>, at: &At) -> Result<Option<NumberAs>, Fault> {
    o.get("as").map(|v| closed::<NumberAs>().read(v, &At::Key(at, "as"))).transpose()
}

struct SendCodec;

impl Codec for SendCodec {
    type T = SendToken;
    fn read(&self, v: &Value, at: &At) -> Result<SendToken, Fault> {
        let Value::Object(o) = v else {
            return match v {
                Value::Number(_) => FRAME_TOKEN.read(v, at).map(SendToken::Frame),
                _ => Err(at.fault(FaultKind::BadValue, "a byte (0 to 255), or an object of text, file, captured, arg, device_id, identity or checksum")),
            };
        };
        let allowed = |p: &str| -> &'static [&'static str] {
            match p {
                "text" => &["text"],
                "file" => &["file", "as"],
                _ => &["captured", "as"],
            }
        };
        match primary(o, at, &["text", "file", "captured"], allowed)? {
            None => FRAME_TOKEN.read(v, at).map(SendToken::Frame),
            Some("text") => match &o["text"] {
                Value::String(s) if !s.is_empty() && s.chars().all(|c| (' '..='~').contains(&c)) => Ok(SendToken::Text(s.clone())),
                _ => Err(At::Key(at, "text").fault(FaultKind::BadValue, "a non-empty printable ASCII text")),
            },
            Some("file") => {
                let item = closed::<FileItem>().read(&o["file"], &At::Key(at, "file"))?;
                let as_ = number_as(o, at)?;
                if as_.is_some() && matches!(item, FileItem::Path | FileItem::Chunk) {
                    return Err(At::Key(at, "as").fault(FaultKind::BadValue, "as goes with a number: size, offset or chunk_size"));
                }
                Ok(SendToken::File { item, as_ })
            }
            _ => Ok(SendToken::Captured { name: name(o, "captured", at)?, as_: number_as(o, at)? }),
        }
    }
    fn write(&self, t: &SendToken) -> Option<Value> {
        let mut o = Map::new();
        match t {
            SendToken::Frame(f) => return FRAME_TOKEN.write(f),
            SendToken::Text(s) => {
                o.insert("text".into(), Value::String(s.clone()));
            }
            SendToken::File { item, as_ } => {
                put(&mut o, "file", closed(), item);
                put(&mut o, "as", Opt(closed()), as_);
            }
            SendToken::Captured { name, as_ } => {
                o.insert("captured".into(), Value::String(name.clone()));
                put(&mut o, "as", Opt(closed()), as_);
            }
        }
        Some(Value::Object(o))
    }
}

struct ReplyCodec;

impl Codec for ReplyCodec {
    type T = ReplyToken;
    fn read(&self, v: &Value, at: &At) -> Result<ReplyToken, Fault> {
        match v {
            Value::Object(o) => match primary(o, at, &["capture_json"], |_| &["capture_json"])? {
                Some(_) => Ok(ReplyToken::CaptureJson(name(o, "capture_json", at)?)),
                None => MATCH_TOKEN.read(v, at).map(ReplyToken::Frame),
            },
            Value::String(s) if s != "any" => Err(at.fault(FaultKind::BadValue, "a byte (0 to 255), `any`, or an object of capture, capture_json, arg, device_id, identity or checksum")),
            _ => MATCH_TOKEN.read(v, at).map(ReplyToken::Frame),
        }
    }
    fn write(&self, t: &ReplyToken) -> Option<Value> {
        match t {
            ReplyToken::Frame(f) => MATCH_TOKEN.write(f),
            ReplyToken::CaptureJson(n) => Some(Value::Object(Map::from_iter([("capture_json".to_string(), Value::String(n.clone()))]))),
        }
    }
}

struct List<C>(C);

impl<C: Codec> Codec for List<C> {
    type T = Vec<C::T>;
    fn read(&self, v: &Value, at: &At) -> Result<Self::T, Fault> {
        let a = v.as_array().ok_or_else(|| at.fault(FaultKind::BadValue, "a list"))?;
        a.iter().enumerate().map(|(i, x)| self.0.read(x, &At::Index(at, i))).collect()
    }
    fn write(&self, t: &Self::T) -> Option<Value> {
        Some(Value::Array(t.iter().filter_map(|x| self.0.write(x)).collect()))
    }
}

fn step_read(v: &Value, at: &At) -> Result<Step, Fault> {
    let o = keys(v, at, KEYS)?;
    let kind = field(o, "step", at, closed::<StepKind>())?;
    let send = field(o, "send", at, List(SendCodec))?;
    let reply = field(o, "reply", at, Opt(List(ReplyCodec)))?;
    let reply_timeout_ms = field(o, "reply_timeout_ms", at, Opt(at_least(0)))?;
    let delay_ms = field(o, "delay_ms", at, Opt(at_least(0)))?;
    let bad = |k: &'static str, rule: &str| At::Key(at, k).fault(FaultKind::BadValue, rule);
    match (&reply, reply_timeout_ms) {
        (Some(_), None) => return Err(At::Key(at, "reply_timeout_ms").fault(FaultKind::Missing, "reply_timeout_ms is required with a reply")),
        (None, Some(_)) => return Err(bad("reply_timeout_ms", "a reply time goes with a reply")),
        (Some(_), Some(t)) if t < delay_ms.unwrap_or(0) => {
            return Err(bad("reply_timeout_ms", &format!("the reply time {t} ms is shorter than the pause delay_ms {} ms", delay_ms.unwrap_or(0))));
        }
        _ => {}
    }
    let (chunk_bytes, coding) = if kind == StepKind::Write {
        (field(o, "chunk_bytes", at, at_least(1))?, field(o, "coding", at, closed::<Coding>())?)
    } else {
        for k in ["chunk_bytes", "coding"] {
            if o.contains_key(k) {
                return Err(bad(if k == "coding" { "coding" } else { "chunk_bytes" }, "a field of the write step only"));
            }
        }
        (0, Coding::Mcoded7)
    };
    let write = kind == StepKind::Write;
    Ok(Step {
        step: kind,
        send,
        reply,
        reply_timeout_ms,
        delay_ms,
        chunk_bytes: write.then_some(chunk_bytes),
        coding: write.then_some(coding),
        remarks: remarks(o, KEYS),
    })
}

fn step_write(s: &Step) -> Value {
    let mut o = Map::new();
    put(&mut o, "step", closed(), &s.step);
    put(&mut o, "send", List(SendCodec), &s.send);
    put(&mut o, "reply", Opt(List(ReplyCodec)), &s.reply);
    put(&mut o, "reply_timeout_ms", Opt(at_least(0)), &s.reply_timeout_ms);
    put(&mut o, "delay_ms", Opt(at_least(0)), &s.delay_ms);
    put(&mut o, "chunk_bytes", Opt(at_least(1)), &s.chunk_bytes);
    put(&mut o, "coding", Opt(closed()), &s.coding);
    put_remarks(&mut o, &s.remarks);
    Value::Object(o)
}

/// The list of steps: one `open`, one `write` and one `close`.
pub struct StepsCodec;

impl Codec for StepsCodec {
    type T = Vec<Step>;
    fn read(&self, v: &Value, at: &At) -> Result<Vec<Step>, Fault> {
        let a = v.as_array().ok_or_else(|| at.fault(FaultKind::BadValue, "a list"))?;
        let steps: Vec<Step> = a.iter().enumerate().map(|(i, x)| step_read(x, &At::Index(at, i))).collect::<Result<_, _>>()?;
        for (i, s) in steps.iter().enumerate() {
            if steps[..i].iter().any(|p| p.step == s.step) {
                return Err(At::Index(at, i).fault(FaultKind::BadValue, format!("a second {} step: one open, one write and one close", s.step.word())));
            }
        }
        for kind in [StepKind::Open, StepKind::Write, StepKind::Close] {
            if !steps.iter().any(|s| s.step == kind) {
                return Err(at.fault(FaultKind::BadValue, format!("a file write has an {} step: one open, one write and one close", kind.word())));
            }
        }
        let captures: Vec<&str> = steps
            .iter()
            .filter(|s| s.step == StepKind::Open)
            .flat_map(|s| s.reply.iter().flatten())
            .filter_map(|t| if let ReplyToken::CaptureJson(n) = t { Some(n.as_str()) } else { None })
            .collect();
        for (i, s) in steps.iter().enumerate() {
            for (j, t) in s.send.iter().enumerate() {
                if let SendToken::Captured { name, .. } = t {
                    if !captures.contains(&name.as_str()) {
                        let here = At::Index(at, i);
                        let send = At::Key(&here, "send");
                        let el = At::Index(&send, j);
                        return Err(At::Key(&el, "captured").fault(FaultKind::BadValue, format!("{name} is not captured by capture_json in the reply of the open step")));
                    }
                }
            }
        }
        Ok(steps)
    }
    fn write(&self, t: &Vec<Step>) -> Option<Value> {
        Some(Value::Array(t.iter().map(step_write).collect()))
    }
}

#[cfg(test)]
mod tests;
