//! Reading JSON strictly and writing it back: every key named by its
//! schema, every value checked, every refusal a fault at its JSON pointer.
//!
//! An object of the format is declared once with [`object!`]: its keys, the
//! Rust field each one fills and the codec that checks its value. A codec
//! reads a value and writes it back unchanged, so a descriptor read then
//! written equals its text, key order aside and an integer written without
//! a fraction (`1.0` comes back `1`).

use crate::{Fault, FaultKind};
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;
use std::marker::PhantomData;

/// The keys of an object that start with `_`: notes for people, never read
/// by a program, kept as they are.
pub type Remarks = BTreeMap<String, Value>;

/// Where a value sits in its document; rendered as a JSON pointer only when
/// a fault needs it.
#[derive(Clone, Copy)]
pub enum At<'a> {
    Root,
    Key(&'a At<'a>, &'a str),
    Index(&'a At<'a>, usize),
}

impl At<'_> {
    /// The JSON pointer (RFC 6901) of this place.
    pub fn pointer(&self) -> String {
        match self {
            At::Root => String::new(),
            At::Key(up, k) => format!("{}/{}", up.pointer(), k.replace('~', "~0").replace('/', "~1")),
            At::Index(up, i) => format!("{}/{i}", up.pointer()),
        }
    }

    pub fn fault(&self, kind: FaultKind, rule: impl Into<String>) -> Fault {
        Fault { kind, file: None, pointer: self.pointer(), rule: rule.into() }
    }

    fn bad(&self, rule: impl Into<String>) -> Fault {
        self.fault(FaultKind::BadValue, rule)
    }
}

/// How one value of the format is read and written back.
pub trait Codec {
    type T;
    fn read(&self, v: &Value, at: &At) -> Result<Self::T, Fault>;
    /// The value written; `None` leaves the key out.
    fn write(&self, t: &Self::T) -> Option<Value>;
    /// The value of an absent key; `None` when the key is required.
    fn absent(&self) -> Option<Self::T> {
        None
    }
}

/// An object of the format.
pub trait Object: Sized {
    /// The keys its schema names.
    const KEYS: &'static [&'static str];
    fn read(v: &Value, at: &At) -> Result<Self, Fault>;
    fn write(&self) -> Value;
}

/// A word of a closed list.
pub trait Word: Sized + Copy + PartialEq + 'static {
    const WORDS: &'static [(&'static str, Self)];

    /// The word as the format writes it.
    fn word(self) -> &'static str {
        Self::WORDS.iter().find(|(_, w)| *w == self).map(|(s, _)| *s).unwrap_or_default()
    }
}

/// The object `v`, refused when it holds a key `keys` does not name and
/// that does not start with `_`.
pub fn keys<'v>(v: &'v Value, at: &At, keys: &[&str]) -> Result<&'v Map<String, Value>, Fault> {
    let o = v.as_object().ok_or_else(|| at.bad("an object"))?;
    if let Some(k) = o.keys().find(|k| !k.starts_with('_') && !keys.contains(&k.as_str())) {
        return Err(At::Key(at, k).fault(FaultKind::UnknownField, format!("{k} is not a field here")));
    }
    Ok(o)
}

/// The value of `key` in `o`, read by `c`.
pub fn field<C: Codec>(o: &Map<String, Value>, key: &str, at: &At, c: C) -> Result<C::T, Fault> {
    let here = At::Key(at, key);
    match o.get(key) {
        Some(v) => c.read(v, &here),
        None => c.absent().ok_or_else(|| here.fault(FaultKind::Missing, format!("{key} is required"))),
    }
}

/// The notes of `o`: its keys starting with `_` that `keys` does not name.
pub fn remarks(o: &Map<String, Value>, keys: &[&str]) -> Remarks {
    o.iter().filter(|(k, _)| k.starts_with('_') && !keys.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()
}

pub fn put<C: Codec>(o: &mut Map<String, Value>, key: &str, c: C, t: &C::T) {
    if let Some(v) = c.write(t) {
        o.insert(key.to_string(), v);
    }
}

pub fn put_remarks(o: &mut Map<String, Value>, r: &Remarks) {
    o.extend(r.iter().map(|(k, v)| (k.clone(), v.clone())));
}

/// Declares an object of the format: a struct with one field per key, its
/// notes for people, and its [`Object`] reading and writing. A `check`
/// function, given the object read, refuses what its fields alone cannot.
macro_rules! object {
    (
        $(#[$meta:meta])*
        $name:ident {
            $( $(#[$fmeta:meta])* $key:literal $field:ident: $ty:ty = $codec:expr, )*
        }
        $(check $check:path)?
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            $( $(#[$fmeta])* pub $field: $ty, )*
            /// The keys starting with `_` that the schema does not name: notes
            /// for people, kept as they are.
            pub remarks: $crate::json::Remarks,
        }

        impl $crate::json::Object for $name {
            const KEYS: &'static [&'static str] = &[$($key),*];

            fn read(v: &serde_json::Value, at: &$crate::json::At) -> Result<Self, $crate::Fault> {
                let o = $crate::json::keys(v, at, Self::KEYS)?;
                let out = $name {
                    $( $field: $crate::json::field(o, $key, at, $codec)?, )*
                    remarks: $crate::json::remarks(o, Self::KEYS),
                };
                $( $check(&out, at)?; )?
                Ok(out)
            }

            fn write(&self) -> serde_json::Value {
                let mut o = serde_json::Map::new();
                $( $crate::json::put(&mut o, $key, $codec, &self.$field); )*
                $crate::json::put_remarks(&mut o, &self.remarks);
                serde_json::Value::Object(o)
            }
        }
    };
}
pub(crate) use object;

/// Declares a closed list of words as an enum.
macro_rules! words {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident = $word:literal, )* }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name { $( $(#[$vmeta])* $variant, )* }

        impl $crate::json::Word for $name {
            const WORDS: &'static [(&'static str, Self)] = &[$(($word, Self::$variant)),*];
        }
    };
}
pub(crate) use words;

/// An optional key: absent is `None`.
pub struct Opt<C>(pub C);

impl<C: Codec> Codec for Opt<C> {
    type T = Option<C::T>;
    fn read(&self, v: &Value, at: &At) -> Result<Self::T, Fault> {
        self.0.read(v, at).map(Some)
    }
    fn write(&self, t: &Self::T) -> Option<Value> {
        t.as_ref().and_then(|t| self.0.write(t))
    }
    fn absent(&self) -> Option<Self::T> {
        Some(None)
    }
}

/// A value that may be `null`, read as `None`.
pub struct Nullable<C>(pub C);

impl<C: Codec> Codec for Nullable<C> {
    type T = Option<C::T>;
    fn read(&self, v: &Value, at: &At) -> Result<Self::T, Fault> {
        if v.is_null() { Ok(None) } else { self.0.read(v, at).map(Some) }
    }
    fn write(&self, t: &Self::T) -> Option<Value> {
        match t {
            Some(t) => self.0.write(t),
            None => Some(Value::Null),
        }
    }
}

/// An integer, within its bounds, held in the Rust type `T`.
pub struct Int<T> {
    min: Option<i64>,
    max: Option<i64>,
    t: PhantomData<T>,
}

/// An integer from `min` to `max`.
pub const fn int<T>(min: i64, max: i64) -> Int<T> {
    Int { min: Some(min), max: Some(max), t: PhantomData }
}

/// An integer of at least `min`.
pub const fn at_least(min: i64) -> Int<i64> {
    Int { min: Some(min), max: None, t: PhantomData }
}

/// Any integer.
pub const INTEGER: Int<i64> = Int { min: None, max: None, t: PhantomData };

impl<T: TryFrom<i64> + Into<i64> + Copy> Codec for Int<T> {
    type T = T;
    fn read(&self, v: &Value, at: &At) -> Result<T, Fault> {
        let range = || match (self.min, self.max) {
            (Some(a), Some(b)) => format!("an integer from {a} to {b}"),
            (Some(a), None) => format!("an integer of at least {a}"),
            _ => "an integer".to_string(),
        };
        let n = v.as_i64().or_else(|| whole(v.as_f64()?)).ok_or_else(|| at.bad(range()))?;
        if self.min.is_some_and(|m| n < m) || self.max.is_some_and(|m| n > m) {
            return Err(at.bad(range()));
        }
        T::try_from(n).map_err(|_| at.bad(range()))
    }
    fn write(&self, t: &T) -> Option<Value> {
        Some(Value::from((*t).into()))
    }
}

/// The integer a number with a zero fraction is (`1.0`, `1e2`), as JSON
/// Schema reads it; `None` for a fraction or a number beyond `i64`.
fn whole(x: f64) -> Option<i64> {
    const LIMIT: f64 = 9_223_372_036_854_775_808.0; // 2^63
    (x.fract() == 0.0 && (-LIMIT..LIMIT).contains(&x)).then_some(x as i64)
}

/// A number, kept as written (an integer stays an integer).
pub struct Num {
    pub above: Option<f64>,
}

impl Codec for Num {
    type T = Number;
    fn read(&self, v: &Value, at: &At) -> Result<Number, Fault> {
        let n = v.as_number().ok_or_else(|| at.bad("a number"))?;
        if let Some(a) = self.above {
            if n.as_f64().is_none_or(|x| x <= a) {
                return Err(at.bad(format!("a number above {a}")));
            }
        }
        Ok(n.clone())
    }
    fn write(&self, t: &Number) -> Option<Value> {
        Some(Value::Number(t.clone()))
    }
}

pub struct Bool;

impl Codec for Bool {
    type T = bool;
    fn read(&self, v: &Value, at: &At) -> Result<bool, Fault> {
        v.as_bool().ok_or_else(|| at.bad("true or false"))
    }
    fn write(&self, t: &bool) -> Option<Value> {
        Some(Value::Bool(*t))
    }
}

/// A pattern a string matches, and what it is in words.
pub type Pattern = (fn(&str) -> bool, &'static str);

/// A string, with the pattern its schema gives (`what` says it in words)
/// and its greatest length in characters.
pub struct Text {
    pub pattern: Option<Pattern>,
    pub max: Option<usize>,
}

/// Any string.
pub const STRING: Text = Text { pattern: None, max: None };

/// A string of at most `max` characters.
pub const fn at_most(max: usize) -> Text {
    Text { pattern: None, max: Some(max) }
}

/// A string matching `test`, which `what` names.
pub const fn matching(test: fn(&str) -> bool, what: &'static str) -> Text {
    Text { pattern: Some((test, what)), max: None }
}

impl Codec for Text {
    type T = String;
    fn read(&self, v: &Value, at: &At) -> Result<String, Fault> {
        let s = v.as_str().ok_or_else(|| at.bad("a string"))?;
        if let Some((test, what)) = self.pattern {
            if !test(s) {
                return Err(at.bad(format!("a string {what}")));
            }
        }
        if let Some(max) = self.max {
            if s.chars().count() > max {
                return Err(at.bad(format!("a string of at most {max} characters")));
            }
        }
        Ok(s.to_string())
    }
    fn write(&self, t: &String) -> Option<Value> {
        Some(Value::String(t.clone()))
    }
}

/// An address read the same way from one device to the next: `^/`.
pub const ADDRESS: Text = matching(|s| s.starts_with('/'), "starting with /");

/// A word of the closed list `T`.
pub struct Closed<T>(PhantomData<T>);

pub const fn closed<T>() -> Closed<T> {
    Closed(PhantomData)
}

impl<T: Word> Codec for Closed<T> {
    type T = T;
    fn read(&self, v: &Value, at: &At) -> Result<T, Fault> {
        let s = v.as_str();
        T::WORDS.iter().find(|(w, _)| Some(*w) == s).map(|(_, t)| *t).ok_or_else(|| {
            let words: Vec<&str> = T::WORDS.iter().map(|(w, _)| *w).collect();
            at.bad(format!("one of {}", words.join(", ")))
        })
    }
    fn write(&self, t: &T) -> Option<Value> {
        Some(Value::String(t.word().to_string()))
    }
}

/// An object of the format, read by its own [`Object`] reading.
pub struct Obj<T>(PhantomData<T>);

pub const fn obj<T>() -> Obj<T> {
    Obj(PhantomData)
}

impl<T: Object> Codec for Obj<T> {
    type T = T;
    fn read(&self, v: &Value, at: &At) -> Result<T, Fault> {
        T::read(v, at)
    }
    fn write(&self, t: &T) -> Option<Value> {
        Some(t.write())
    }
}

/// A list of values read by `C`, with its least and greatest length.
pub struct List<C> {
    item: C,
    min: usize,
    max: usize,
}

pub const fn list<C>(item: C) -> List<C> {
    List { item, min: 0, max: usize::MAX }
}

pub const fn list_of<C>(item: C, min: usize, max: usize) -> List<C> {
    List { item, min, max }
}

impl<C: Codec> Codec for List<C> {
    type T = Vec<C::T>;
    fn read(&self, v: &Value, at: &At) -> Result<Self::T, Fault> {
        let a = v.as_array().ok_or_else(|| at.bad("a list"))?;
        if a.len() < self.min || a.len() > self.max {
            return Err(at.bad(match (self.min, self.max) {
                (a, b) if a == b => format!("a list of {a}"),
                (a, usize::MAX) => format!("a list of at least {a}"),
                (_, b) => format!("a list of at most {b}"),
            }));
        }
        a.iter().enumerate().map(|(i, x)| self.item.read(x, &At::Index(at, i))).collect()
    }
    fn write(&self, t: &Self::T) -> Option<Value> {
        Some(Value::Array(t.iter().filter_map(|x| self.item.write(x)).collect()))
    }
}

/// Two integers: the lowest and highest value.
pub struct Pair;

impl Codec for Pair {
    type T = [i64; 2];
    fn read(&self, v: &Value, at: &At) -> Result<[i64; 2], Fault> {
        let l = list_of(INTEGER, 2, 2).read(v, at)?;
        Ok([l[0], l[1]])
    }
    fn write(&self, t: &[i64; 2]) -> Option<Value> {
        Some(Value::from(t.to_vec()))
    }
}

/// An object whose keys are free and whose values are read by `C`.
pub struct Dict<C>(pub C);

impl<C: Codec> Codec for Dict<C> {
    type T = BTreeMap<String, C::T>;
    fn read(&self, v: &Value, at: &At) -> Result<Self::T, Fault> {
        let o = v.as_object().ok_or_else(|| at.bad("an object"))?;
        o.iter().map(|(k, x)| Ok((k.clone(), self.0.read(x, &At::Key(at, k))?))).collect()
    }
    fn write(&self, t: &Self::T) -> Option<Value> {
        Some(Value::Object(t.iter().filter_map(|(k, x)| Some((k.clone(), self.0.write(x)?))).collect()))
    }
}

/// Any value, kept as it is.
pub struct Any;

impl Codec for Any {
    type T = Value;
    fn read(&self, v: &Value, _: &At) -> Result<Value, Fault> {
        Ok(v.clone())
    }
    fn write(&self, t: &Value) -> Option<Value> {
        Some(t.clone())
    }
}
