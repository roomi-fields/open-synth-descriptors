//! The messages of a device: a frame is a list of elements, each a byte or,
//! in 0.5, a one-key object of its nature (`arg`, `capture`, `device_id`,
//! `identity`, `checksum`) or the word `any`; a `pre` expression is a tree.
//! 0.4 wrote the same things as `{placeholder}` strings and as expression
//! strings: this module reads both forms, and converts the 0.4 forms into
//! the 0.5 structures.

use crate::json::{int, Codec, Int, Word, At};
use crate::{Fault, FaultKind};
use serde_json::{Map, Value};

/// A byte, 0 to 255.
const BYTE: Int<u8> = int(0, 255);
/// A 7-bit MIDI value, 0 to 127: the bound of a `pre` literal and of `add`.
const U7: Int<u8> = int(0, 127);
/// An offset in a message, 0 to 65535.
const OFFSET: Int<u16> = int(0, 65535);
/// A length in bytes, 1 to 65535.
const LENGTH: Int<u16> = int(1, 65535);

crate::json::words! {
    /// How an argument is written in the message; absent, one 7-bit byte.
    Encoding {
        U14Msb = "u14_msb", U14Lsb = "u14_lsb", S32 = "s32", Ascii = "ascii", Bytes = "bytes",
    }
}

crate::json::words! {
    /// Which bytes of the model a device's identity gives.
    IdentityPart { Family = "family", Member = "member", }
}

crate::json::words! {
    /// A checksum algorithm: Roland's two's complement, Waldorf's sum,
    /// Fractal's xor, each on 7 bits.
    Algorithm { Twos7 = "twos7", Sum7 = "sum7", Xor7 = "xor7", }
}

/// An element of a frame written as a structure (0.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FramePart {
    /// An argument of the command, with its encoding or an offset added to
    /// it (the sum wraps on 7 bits).
    Arg { name: String, encoding: Option<Encoding>, add: Option<u8> },
    /// Where a recognised message is read: the value is kept under `name`.
    Capture(String),
    /// The device id.
    DeviceId,
    /// The bytes of the identity's family or member, or the one at `index`.
    Identity { part: IdentityPart, index: Option<u8> },
    /// The checksum of the bytes written from offset `from` (`F0` is 0), on
    /// `length` bytes when given.
    Checksum { algorithm: Algorithm, from: u16, length: Option<u16> },
    /// Any byte (written as the word `any`).
    Any,
}

/// One element of a frame: a byte, a `{placeholder}` string (0.4), or a
/// structure (0.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameToken {
    Byte(u8),
    /// A 0.4 string: `{value}`, `{value:u14_lsb}`, `{dev}`…; in an identity
    /// reply of 0.4, any string.
    Placeholder(String),
    Part(FramePart),
}

/// The places where a frame is written, by what its elements may be.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    /// A message sent.
    Sent,
    /// A message recognised (`replies[].match_frame`).
    Matched,
    /// The start of an identity reply, recognised.
    Reply,
}

impl Place {
    fn recognises(self) -> bool {
        self != Place::Sent
    }
}

/// A placeholder: `^\{.+\}$`, braces around at least one character that is
/// not a line break.
pub(crate) fn placeholder(s: &str) -> bool {
    let inner = s.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
    inner.is_some_and(|i| !i.is_empty() && !i.contains(['\n', '\r', '\u{2028}', '\u{2029}']))
}

pub(crate) struct FrameTokenCodec(pub(crate) Place);

/// The elements of a message sent.
pub(crate) const FRAME_TOKEN: FrameTokenCodec = FrameTokenCodec(Place::Sent);
/// The elements of `replies[].match_frame`.
pub(crate) const MATCH_TOKEN: FrameTokenCodec = FrameTokenCodec(Place::Matched);
/// The elements of `identity.reply`.
pub(crate) const REPLY_TOKEN: FrameTokenCodec = FrameTokenCodec(Place::Reply);

const PRIMARY: [&str; 5] = ["arg", "capture", "device_id", "identity", "checksum"];

impl FrameTokenCodec {
    fn expected(&self) -> &'static str {
        if self.0.recognises() {
            "a byte (0 to 255), `any`, or an object of one of arg, capture, device_id, identity, checksum"
        } else {
            "a byte (0 to 255), or an object of one of arg, device_id, identity, checksum"
        }
    }

    fn part(&self, o: &Map<String, Value>, at: &At) -> Result<FramePart, Fault> {
        let present: Vec<&str> = PRIMARY.iter().copied().filter(|k| o.contains_key(*k)).collect();
        let [primary] = present[..] else {
            if present.is_empty() {
                if let Some(k) = o.keys().next() {
                    return Err(At::Key(at, k).fault(FaultKind::UnknownField, format!("{k} is not a frame element")));
                }
                return Err(at.fault(FaultKind::BadValue, self.expected()));
            }
            return Err(at.fault(FaultKind::BadValue, "an object of one key of its nature"));
        };
        let allowed: &[&str] = match primary {
            "arg" => &["arg", "encoding", "add"],
            "identity" => &["identity", "index"],
            "checksum" => &["checksum", "from", "length"],
            _ => &[primary],
        };
        if let Some(k) = o.keys().find(|k| !allowed.contains(&k.as_str())) {
            let rule = format!("{k} is not a field of {primary} here");
            let kind = if PRIMARY.contains(&k.as_str()) { FaultKind::BadValue } else { FaultKind::UnknownField };
            return Err(At::Key(at, k).fault(kind, rule));
        }
        let key = |k: &'static str| At::Key(at, k);
        let name = |k: &'static str| match &o[k] {
            Value::String(s) => Ok(s.clone()),
            _ => Err(key(k).fault(FaultKind::BadValue, "a string")),
        };
        Ok(match primary {
            "arg" => {
                let encoding = o.get("encoding").map(|v| crate::json::closed::<Encoding>().read(v, &key("encoding"))).transpose()?;
                let add = o.get("add").map(|v| U7.read(v, &key("add"))).transpose()?;
                if encoding.is_some() && add.is_some() {
                    return Err(key("add").fault(FaultKind::BadValue, "add goes with a 7-bit argument, without an encoding"));
                }
                FramePart::Arg { name: name("arg")?, encoding, add }
            }
            "capture" => {
                if !self.0.recognises() {
                    return Err(key("capture").fault(FaultKind::BadValue, "capture is for a message recognised"));
                }
                FramePart::Capture(name("capture")?)
            }
            "device_id" => match o["device_id"] {
                Value::Bool(true) => FramePart::DeviceId,
                _ => return Err(key("device_id").fault(FaultKind::BadValue, "true")),
            },
            "identity" => {
                let part = crate::json::closed::<IdentityPart>().read(&o["identity"], &key("identity"))?;
                let index = o.get("index").map(|v| BYTE.read(v, &key("index"))).transpose()?;
                FramePart::Identity { part, index }
            }
            _ => {
                let algorithm = crate::json::closed::<Algorithm>().read(&o["checksum"], &key("checksum"))?;
                let from = match o.get("from") {
                    Some(v) => OFFSET.read(v, &key("from"))?,
                    None => return Err(key("from").fault(FaultKind::Missing, "from is required")),
                };
                let length = o.get("length").map(|v| LENGTH.read(v, &key("length"))).transpose()?;
                FramePart::Checksum { algorithm, from, length }
            }
        })
    }
}

impl Codec for FrameTokenCodec {
    type T = FrameToken;
    fn read(&self, v: &Value, at: &At) -> Result<FrameToken, Fault> {
        match v {
            Value::String(s) if self.0.recognises() && s == "any" => Ok(FrameToken::Part(FramePart::Any)),
            Value::String(s) if placeholder(s) || self.0 == Place::Reply => Ok(FrameToken::Placeholder(s.clone())),
            Value::Number(_) => BYTE.read(v, at).map(FrameToken::Byte),
            // A fault inside the object is reported at the element, where the schema reports it.
            Value::Object(o) => self.part(o, at).map(FrameToken::Part).map_err(|f| Fault { pointer: at.pointer(), ..f }),
            _ => Err(at.fault(FaultKind::BadValue, self.expected())),
        }
    }
    fn write(&self, t: &FrameToken) -> Option<Value> {
        Some(match t {
            FrameToken::Byte(b) => Value::from(*b),
            FrameToken::Placeholder(s) => Value::String(s.clone()),
            FrameToken::Part(p) => write_part(p),
        })
    }
}

fn write_part(p: &FramePart) -> Value {
    let mut o = Map::new();
    match p {
        FramePart::Any => return Value::String("any".into()),
        FramePart::Arg { name, encoding, add } => {
            o.insert("arg".into(), Value::String(name.clone()));
            if let Some(e) = encoding {
                o.insert("encoding".into(), Value::String(e.word().into()));
            }
            if let Some(a) = add {
                o.insert("add".into(), Value::from(*a));
            }
        }
        FramePart::Capture(n) => {
            o.insert("capture".into(), Value::String(n.clone()));
        }
        FramePart::DeviceId => {
            o.insert("device_id".into(), Value::Bool(true));
        }
        FramePart::Identity { part, index } => {
            o.insert("identity".into(), Value::String(part.word().into()));
            if let Some(i) = index {
                o.insert("index".into(), Value::from(*i));
            }
        }
        FramePart::Checksum { algorithm, from, length } => {
            o.insert("checksum".into(), Value::String(algorithm.word().into()));
            o.insert("from".into(), Value::from(*from));
            if let Some(l) = length {
                o.insert("length".into(), Value::from(*l));
            }
        }
    }
    Value::Object(o)
}

/// Whether the elements of a frame, read in version `v05` or not, are of
/// that version: a 0.4 string in 0.5 and a structure in 0.4 are refused at
/// their address.
pub(crate) fn check_frame(tokens: &[FrameToken], place: Place, pointer: &str, v05: bool) -> Result<(), Fault> {
    for (i, t) in tokens.iter().enumerate() {
        let rule = match (t, v05) {
            (FrameToken::Placeholder(_), true) => "a byte or an object of its nature (a {placeholder} is 0.4)",
            // In 0.4 an identity reply holds any string, `any` included.
            (FrameToken::Part(p), false) if *p != FramePart::Any || place != Place::Reply => {
                "a byte or a {placeholder} (an object or the word any is 0.5)"
            }
            _ => continue,
        };
        return Err(at_pointer(format!("{pointer}/{i}"), rule));
    }
    Ok(())
}

fn at_pointer(pointer: String, rule: &str) -> Fault {
    Fault { kind: FaultKind::BadValue, file: None, pointer, rule: rule.to_string() }
}

// ---- pre: expression trees -------------------------------------------------

crate::json::words! {
    /// A binary operation of an expression. `add`, `sub` and `mul` give a
    /// value bounded to 0..127; `gt`, `lt`, `eq` and `ne` give 0 or 1.
    BinOp { Add = "add", Sub = "sub", Mul = "mul", Gt = "gt", Lt = "lt", Eq = "eq", Ne = "ne", }
}

/// A value computed from the arguments before the message is built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// An expression string (0.4).
    Text(String),
    /// A constant, 0 to 127.
    Literal(u8),
    /// An argument or a value bound before.
    Arg(String),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// `if` condition, then, else: the condition is not 0.
    If(Box<Expr>, Box<Expr>, Box<Expr>),
}

pub(crate) struct ExprCodec;
pub(crate) const EXPR: ExprCodec = ExprCodec;

impl Codec for ExprCodec {
    type T = Expr;
    fn read(&self, v: &Value, at: &At) -> Result<Expr, Fault> {
        match v {
            Value::String(s) => Ok(Expr::Text(s.clone())),
            _ => tree(v, at),
        }
    }
    fn write(&self, t: &Expr) -> Option<Value> {
        Some(write_expr(t))
    }
}

fn tree(v: &Value, at: &At) -> Result<Expr, Fault> {
    match v {
        Value::Number(_) => U7.read(v, at).map(Expr::Literal),
        Value::Object(o) => {
            let mut keys = o.keys();
            let (Some(key), None) = (keys.next(), keys.next()) else {
                return Err(at.fault(FaultKind::BadValue, "an object of one key: arg, add, sub, mul, gt, lt, eq, ne or if"));
            };
            let here = At::Key(at, key);
            if key == "arg" {
                return match &o[key] {
                    Value::String(s) => Ok(Expr::Arg(s.clone())),
                    _ => Err(here.fault(FaultKind::BadValue, "a string")),
                };
            }
            if key == "if" {
                let mut a = operands(&o[key], &here, 3)?.into_iter();
                let (c, t, e) = (a.next().unwrap(), a.next().unwrap(), a.next().unwrap());
                return Ok(Expr::If(Box::new(c), Box::new(t), Box::new(e)));
            }
            let Some(op) = BinOp::WORDS.iter().find(|(w, _)| w == key).map(|(_, o)| *o) else {
                return Err(here.fault(FaultKind::UnknownField, format!("{key} is not an operation")));
            };
            let mut a = operands(&o[key], &here, 2)?.into_iter();
            Ok(Expr::Binary(op, Box::new(a.next().unwrap()), Box::new(a.next().unwrap())))
        }
        _ => Err(at.fault(FaultKind::BadValue, "an integer from 0 to 127, or an object of one key")),
    }
}

fn operands(v: &Value, at: &At, n: usize) -> Result<Vec<Expr>, Fault> {
    let a = v.as_array().filter(|a| a.len() == n).ok_or_else(|| at.fault(FaultKind::BadValue, format!("a list of {n}")))?;
    a.iter().enumerate().map(|(i, x)| tree(x, &At::Index(at, i))).collect()
}

fn write_expr(e: &Expr) -> Value {
    let one = |k: &str, v: Value| Value::Object(Map::from_iter([(k.to_string(), v)]));
    match e {
        Expr::Text(s) => Value::String(s.clone()),
        Expr::Literal(n) => Value::from(*n),
        Expr::Arg(n) => one("arg", Value::String(n.clone())),
        Expr::Binary(op, a, b) => one(op.word(), Value::Array(vec![write_expr(a), write_expr(b)])),
        Expr::If(c, t, f) => one("if", Value::Array(vec![write_expr(c), write_expr(t), write_expr(f)])),
    }
}

/// Whether an expression, read in version `v05` or not, is of that version.
pub(crate) fn check_expr(e: &Expr, pointer: &str, v05: bool) -> Result<(), Fault> {
    match (matches!(e, Expr::Text(_)), v05) {
        (true, true) => Err(at_pointer(pointer.into(), "an expression tree (a string is 0.4)")),
        (false, false) => Err(at_pointer(pointer.into(), "an expression string (a tree is 0.5)")),
        _ => Ok(()),
    }
}

// ---- conversion of the 0.4 forms ---------------------------------------------

fn not_convertible(at: &At, why: impl Into<String>) -> Fault {
    at.fault(FaultKind::NotConvertible, why)
}

/// The structure of a 0.4 element at `at`, in a frame of `place`.
pub(crate) fn convert_token(t: &FrameToken, place: Place, at: &At) -> Result<FrameToken, Fault> {
    let FrameToken::Placeholder(s) = t else { return Ok(t.clone()) };
    if !placeholder(s) {
        return Err(not_convertible(at, format!("{s:?} is not a {{placeholder}}")));
    }
    convert_placeholder(&s[1..s.len() - 1], place, at).map(FrameToken::Part).map_err(|why| not_convertible(at, why))
}

fn name_ok(n: &str) -> bool {
    !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn number(s: &str) -> Option<u32> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(h) => u32::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    }
}

fn convert_placeholder(inner: &str, place: Place, _at: &At) -> Result<FramePart, String> {
    let (name, spec) = match inner.split_once(':') {
        Some((n, s)) => (n, Some(s)),
        None => (inner, None),
    };
    match (name, spec) {
        ("dev", None) => Ok(FramePart::DeviceId),
        ("any", None) if place.recognises() => Ok(FramePart::Any),
        ("any", None) => Err("{any} is for a message recognised".into()),
        ("checksum", None) => Err("{checksum} names no algorithm (twos, sum or xor)".into()),
        ("function", _) => Err("{function} has no structure in 0.5".into()),
        ("checksum", Some(spec)) => {
            let (algorithm, rest) = spec.split_once('@').ok_or("a checksum is {checksum:<algorithm>@<from>}")?;
            let algorithm = match algorithm {
                "twos" => Algorithm::Twos7,
                "sum" => Algorithm::Sum7,
                "xor" => Algorithm::Xor7,
                other => return Err(format!("the checksum algorithm {other:?} is not twos, sum or xor")),
            };
            let (from, length) = match rest.split_once('+') {
                Some((f, l)) => (f, Some(l)),
                None => (rest, None),
            };
            let from = number(from).and_then(|n| u16::try_from(n).ok()).ok_or("the checksum offset is a number")?;
            let length = length
                .map(|l| number(l).and_then(|n| u16::try_from(n).ok()).filter(|n| *n > 0).ok_or("the checksum length is a number above 0"))
                .transpose()?;
            Ok(FramePart::Checksum { algorithm, from, length })
        }
        ("family" | "member", spec) => {
            let part = if name == "family" { IdentityPart::Family } else { IdentityPart::Member };
            let index = spec
                .map(|s| number(s).and_then(|n| u8::try_from(n).ok()).ok_or("the identity index is a number from 0 to 255"))
                .transpose()?;
            Ok(FramePart::Identity { part, index })
        }
        (name, spec) if name_ok(name) => match spec {
            None if place.recognises() => Ok(FramePart::Capture(name.into())),
            None | Some("u7") => Ok(FramePart::Arg { name: name.into(), encoding: None, add: None }),
            Some(spec) => {
                if let Some(add) = spec.strip_prefix("u7+") {
                    let add = number(add).and_then(|n| u8::try_from(n).ok()).filter(|n| *n < 128).ok_or("the added number is from 0 to 127")?;
                    return Ok(FramePart::Arg { name: name.into(), encoding: None, add: Some(add) });
                }
                let encoding = Encoding::WORDS
                    .iter()
                    .find(|(w, _)| *w == spec)
                    .map(|(_, e)| *e)
                    .ok_or_else(|| format!("the encoding {spec:?} is not u7, u14_msb, u14_lsb, s32, ascii or bytes"))?;
                Ok(FramePart::Arg { name: name.into(), encoding: Some(encoding), add: None })
            }
        },
        _ => Err(format!("{{{inner}}} is not a form of 0.4 that has a structure")),
    }
}

/// The tree of a 0.4 expression string at `at`: `+ - *` (`*` binds
/// tighter), `> < == !=` (lower), then `c ? a : b`; numbers, names and
/// parentheses. A literal above 127 is a
/// fault: the bound 0..127 is the specification's, and the evaluation of the tree bounds its result.
pub(crate) fn convert_expr(e: &Expr, at: &At) -> Result<Expr, Fault> {
    let Expr::Text(s) = e else { return Ok(e.clone()) };
    let mut p = Parser { chars: s.chars().collect(), at: 0 };
    let tree = p.ternary().and_then(|t| if p.peek().is_some() { Err(p.error("what follows the expression")) } else { Ok(t) });
    tree.map_err(|why| not_convertible(at, format!("the expression {s:?}: {why}")))
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn peek(&mut self) -> Option<char> {
        while self.chars.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
        self.chars.get(self.at).copied()
    }

    fn error(&self, what: &str) -> String {
        format!("{what} at character {}", self.at)
    }

    fn eat(&mut self, s: &str) -> bool {
        self.peek();
        let end = self.at + s.chars().count();
        if self.chars.get(self.at..end).is_some_and(|w| w.iter().copied().eq(s.chars())) {
            self.at = end;
            true
        } else {
            false
        }
    }

    fn ternary(&mut self) -> Result<Expr, String> {
        let c = self.comparison()?;
        if !self.eat("?") {
            return Ok(c);
        }
        let a = self.ternary()?;
        if !self.eat(":") {
            return Err(self.error("`:` expected"));
        }
        let b = self.ternary()?;
        Ok(Expr::If(Box::new(c), Box::new(a), Box::new(b)))
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.sum()?;
        loop {
            let op = if self.eat("==") {
                BinOp::Eq
            } else if self.eat("!=") {
                BinOp::Ne
            } else if self.eat(">") {
                BinOp::Gt
            } else if self.eat("<") {
                BinOp::Lt
            } else {
                return Ok(left);
            };
            left = Expr::Binary(op, Box::new(left), Box::new(self.sum()?));
        }
    }

    fn sum(&mut self) -> Result<Expr, String> {
        let mut left = self.product()?;
        loop {
            let op = if self.eat("+") {
                BinOp::Add
            } else if self.eat("-") {
                BinOp::Sub
            } else {
                return Ok(left);
            };
            left = Expr::Binary(op, Box::new(left), Box::new(self.product()?));
        }
    }

    fn product(&mut self) -> Result<Expr, String> {
        let mut left = self.primary()?;
        while self.eat("*") {
            left = Expr::Binary(BinOp::Mul, Box::new(left), Box::new(self.primary()?));
        }
        Ok(left)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some('(') => {
                self.at += 1;
                let e = self.ternary()?;
                if !self.eat(")") {
                    return Err(self.error("`)` expected"));
                }
                Ok(e)
            }
            Some(c) if c.is_ascii_digit() => {
                let start = self.at;
                while self.chars.get(self.at).is_some_and(|c| c.is_ascii_alphanumeric()) {
                    self.at += 1;
                }
                let word: String = self.chars[start..self.at].iter().collect();
                let n = number(&word).ok_or_else(|| format!("{word:?} is not an integer"))?;
                let n = u8::try_from(n).ok().filter(|n| *n < 128).ok_or_else(|| format!("the literal {word:?} is above 127"))?;
                Ok(Expr::Literal(n))
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                let start = self.at;
                while self.chars.get(self.at).is_some_and(|c| c.is_alphanumeric() || *c == '_') {
                    self.at += 1;
                }
                Ok(Expr::Arg(self.chars[start..self.at].iter().collect()))
            }
            _ => Err(self.error("a number, a name or `(` expected")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::placeholder;

    #[test]
    fn a_placeholder_is_braces_around_one_character_or_more() {
        assert!(placeholder("{value}") && placeholder("{}}") && placeholder("{x}"));
        assert!(!placeholder("{}") && !placeholder("value") && !placeholder("{a\nb}") && !placeholder("{value"));
    }
}
