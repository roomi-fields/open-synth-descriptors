//! The flat form of a control, the form of 0.4 to 0.6: the fields of the message on the entry
//! (`cc`, `cc_lsb`, `nrpn_*`, `rpn_*`, `value_bits`, `range`), `also` for the next ways, and
//! `transmit` and `recognize` on the entry (`spec/0.6.md` 22). The reader models the form of 0.7
//! only (`ways`); a device of an older version is read here, in its own form, with the checks and
//! the faults of its version, and lifted to the one form: the same function writes the ways of the
//! converter 0.6 to 0.7 (`spec/proposals/0009`). The other way, [`flatten`], writes a device of an
//! older version back as its text had it; what the ways alone cannot say of its message fields is
//! held beside them ([`FlatFields`]).

use crate::convert::free_key;
use crate::device::{CcEntry, Direction};
use crate::json::{int, obj, object, list, Dict, Object, Opt, Pair, INTEGER, ADDRESS, STRING};
use crate::json::{closed, matching, words, At, Text};
use crate::role::Role;
use crate::{Fault, FaultKind, Version};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const BYTE: crate::json::Int<u8> = int(0, 255);
const U7: crate::json::Int<u8> = int(0, 127);
const CHANNEL: crate::json::Int<u8> = int(0, 15);
const TITLE: Text = matching(|s| !s.is_empty(), "that is not empty");

words! {
    /// Where a control's values start, in the words of 0.4 to 0.6 (`polarity` in 0.7).
    Orientation { ZeroBased = "0-based", Centered = "centered", }
}

object! {
    /// A control of `cc_params` in the flat form.
    FlatCcEntry {
        "osc" osc: String = ADDRESS,
        "title" title: Option<String> = Opt(TITLE),
        "description" description: Option<String> = Opt(STRING),
        "value_description" value_description: Option<String> = Opt(STRING),
        "notes" notes: Option<String> = Opt(STRING),
        "cc" cc: Option<u8> = Opt(U7),
        "cc_lsb" cc_lsb: Option<u8> = Opt(U7),
        "nrpn_msb" nrpn_msb: Option<u8> = Opt(U7),
        "nrpn_lsb" nrpn_lsb: Option<u8> = Opt(U7),
        "nrpn_lsb_range" nrpn_lsb_range: Option<[i64; 2]> = Opt(Pair),
        "rpn_msb" rpn_msb: Option<u8> = Opt(U7),
        "rpn_lsb" rpn_lsb: Option<u8> = Opt(U7),
        "value_bits" value_bits: Option<i64> = Opt(INTEGER),
        "range" range: Option<[i64; 2]> = Opt(Pair),
        "default" default: Option<i64> = Opt(INTEGER),
        "priority" priority: Option<u8> = Opt(int(1, 5)),
        "step_count" step_count: Option<i64> = Opt(crate::json::at_least(2)),
        "transmit" transmit: Option<Direction> = Opt(closed()),
        "recognize" recognize: Option<Direction> = Opt(closed()),
        "values" values: Option<BTreeMap<String, i64>> = Opt(Dict(INTEGER)),
        "orientation" orientation: Option<Orientation> = Opt(closed()),
        "channel" channel: Option<u8> = Opt(CHANNEL),
        "section" section: Option<String> = Opt(STRING),
        "expects" expects: Option<String> = Opt(STRING),
        "transform" transform: Option<String> = Opt(STRING),
        "transform_reverse" transform_reverse: Option<String> = Opt(STRING),
        "role" role: Option<Role> = Opt(obj()),
        "also" also: Option<Vec<FlatWay>> = Opt(list(obj())),
    }
}

object! {
    /// One way of `also`.
    FlatWay {
        "cc" cc: Option<u8> = Opt(U7),
        "cc_lsb" cc_lsb: Option<u8> = Opt(U7),
        "nrpn_msb" nrpn_msb: Option<u8> = Opt(U7),
        "nrpn_lsb" nrpn_lsb: Option<u8> = Opt(U7),
        "rpn_msb" rpn_msb: Option<u8> = Opt(U7),
        "rpn_lsb" rpn_lsb: Option<u8> = Opt(U7),
        "value_bits" value_bits: Option<i64> = Opt(INTEGER),
        "range" range: Option<[i64; 2]> = Opt(Pair),
    }
}

object! {
    /// A control of `params` in the flat form.
    FlatParamEntry {
        "osc" osc: String = ADDRESS,
        "title" title: Option<String> = Opt(TITLE),
        "description" description: Option<String> = Opt(STRING),
        "pr" pr: Option<u8> = Opt(BYTE),
        "p" p: Option<u8> = Opt(BYTE),
        "c" c: Option<u8> = Opt(BYTE),
        "r" r: Option<u8> = Opt(BYTE),
        "id" id: Option<i64> = Opt(INTEGER),
        "range" range: Option<[i64; 2]> = Opt(Pair),
        "default" default: Option<i64> = Opt(INTEGER),
        "priority" priority: Option<u8> = Opt(int(1, 5)),
        "step_count" step_count: Option<i64> = Opt(crate::json::at_least(2)),
        "transmit" transmit: Option<Direction> = Opt(closed()),
        "recognize" recognize: Option<Direction> = Opt(closed()),
        "section" section: Option<String> = Opt(STRING),
        "expects" expects: Option<String> = Opt(STRING),
        "values" values: Option<BTreeMap<String, i64>> = Opt(Dict(INTEGER)),
        "transform" transform: Option<String> = Opt(STRING),
        "transform_reverse" transform_reverse: Option<String> = Opt(STRING),
        "role" role: Option<Role> = Opt(obj()),
    }
}

impl FlatWay {
    fn has_cc(&self) -> bool {
        self.cc.is_some() || self.cc_lsb.is_some()
    }
    fn has_nrpn(&self) -> bool {
        self.nrpn_msb.is_some() || self.nrpn_lsb.is_some()
    }
    fn has_rpn(&self) -> bool {
        self.rpn_msb.is_some() || self.rpn_lsb.is_some()
    }
}

impl FlatCcEntry {
    /// The ways in the order written: the flat fields first, then `also`.
    fn ways(&self) -> Vec<FlatWay> {
        let first = FlatWay {
            cc: self.cc,
            cc_lsb: self.cc_lsb,
            nrpn_msb: self.nrpn_msb,
            nrpn_lsb: self.nrpn_lsb,
            rpn_msb: self.rpn_msb,
            rpn_lsb: self.rpn_lsb,
            value_bits: self.value_bits,
            range: self.range,
            remarks: Default::default(),
        };
        std::iter::once(first).chain(self.also.iter().flatten().cloned()).collect()
    }
}

/// A message a control claims: its kind (0 CC, 1 NRPN, 2 RPN), its channel and its numbers. A 14-bit
/// CC claims its CC number and its LSB number, one message each.
type Claim = (u8, Option<u8>, Option<u8>, Option<u8>);

fn claims(w: &FlatWay, channel: Option<u8>) -> Vec<Claim> {
    let mut out: Vec<Claim> = [w.cc, w.cc_lsb].into_iter().flatten().map(|n| (0, channel, Some(n), None)).collect();
    if w.has_nrpn() {
        out.push((1, channel, w.nrpn_msb, w.nrpn_lsb));
    }
    if w.has_rpn() {
        out.push((2, channel, w.rpn_msb, w.rpn_lsb));
    }
    out.sort();
    out.dedup();
    out
}

/// The width of the value of an NRPN or an RPN way (0.6), in the words of the norm: 7 bits in the
/// data MSB or 14 bits over MSB and LSB. A way that gives an NRPN or an RPN says it (`Missing`), a
/// width outside the two words is refused at the field, and a way of another kind has none.
fn check_value_bits(w: &FlatWay, pointer: &str) -> Result<(), Fault> {
    let at = format!("{pointer}/value_bits");
    let fault = |kind, rule: &str| Fault { kind, file: None, pointer: at.clone(), rule: rule.into() };
    match (w.has_nrpn() || w.has_rpn(), w.value_bits) {
        (true, None) => Err(fault(FaultKind::Missing, "value_bits is required with an nrpn or an rpn: 7 or 14")),
        (true, Some(7 | 14)) | (false, None) => Ok(()),
        (true, Some(_)) => Err(fault(FaultKind::BadValue, "7 (the data MSB alone) or 14 (the data MSB and LSB)")),
        (false, Some(_)) => Err(fault(FaultKind::BadValue, "value_bits is the width of an nrpn or an rpn value: this way gives neither")),
    }
}

/// The default of each control: not a field before 0.6; from 0.6, inside the control's `range`
/// (bounds included) when it has one, compared to nothing otherwise.
fn check_defaults(schema: Option<Version>, cc: &[FlatCcEntry], params: &[FlatParamEntry]) -> Result<(), Fault> {
    let cc = cc.iter().enumerate().map(|(n, e)| (format!("/cc_params/entries/{n}/default"), e.default, e.range));
    let params = params.iter().enumerate().map(|(n, e)| (format!("/params/entries/{n}/default"), e.default, e.range));
    for (pointer, default, range) in cc.chain(params) {
        let Some(default) = default else { continue };
        if schema < Some(Version::V0_6) {
            return Err(Fault { kind: FaultKind::UnknownField, file: None, pointer, rule: "default is not a field before 0.6".into() });
        }
        if let Some([lo, hi]) = range {
            if default < lo || default > hi {
                return Err(Fault { kind: FaultKind::BadValue, file: None, pointer, rule: format!("the default {default} is outside the range {lo}..{hi}") });
            }
        }
    }
    Ok(())
}

/// The precision of a control (0.6): `priority`, `step_count`, `transmit`, `recognize` and the `values`
/// of a CC entry are not fields before 0.6; from 0.6 each named value lies inside the control's
/// `range` (bounds included) when it has one, and is compared to nothing otherwise.
fn check_precision(schema: Option<Version>, cc: &[FlatCcEntry], params: &[FlatParamEntry]) -> Result<(), Fault> {
    let v06 = schema >= Some(Version::V0_6);
    let unknown = |pointer: String, name: &str| Fault { kind: FaultKind::UnknownField, file: None, pointer, rule: format!("{name} is not a field before 0.6") };
    let cc = cc.iter().enumerate().map(|(n, e)| {
        (format!("/cc_params/entries/{n}"), [e.priority.is_some(), e.step_count.is_some(), e.transmit.is_some(), e.recognize.is_some()], e.values.as_ref(), e.range, true)
    });
    let params = params.iter().enumerate().map(|(n, e)| {
        (format!("/params/entries/{n}"), [e.priority.is_some(), e.step_count.is_some(), e.transmit.is_some(), e.recognize.is_some()], e.values.as_ref(), e.range, false)
    });
    for (at, given, values, range, cc) in cc.chain(params) {
        if !v06 {
            for (name, here) in ["priority", "step_count", "transmit", "recognize"].iter().zip(given) {
                if here {
                    return Err(unknown(format!("{at}/{name}"), name));
                }
            }
            if cc && values.is_some() {
                return Err(unknown(format!("{at}/values"), "values"));
            }
            continue;
        }
        if let (Some(values), Some([lo, hi])) = (values, range) {
            for (label, value) in values {
                if *value < lo || *value > hi {
                    let pointer = format!("{at}/values/{}", label.replace('~', "~0").replace('/', "~1"));
                    return Err(Fault { kind: FaultKind::BadValue, file: None, pointer, rule: format!("the value {value} is outside the range {lo}..{hi}") });
                }
            }
        }
    }
    Ok(())
}

/// The ways of a control (0.6): `also` is not a field before 0.6; from 0.6 each way of `also` gives
/// one message kind, and a message (kind, channel, numbers) is claimed once, across the ways of a
/// control as across controls; the fault is at the later place and names the earlier.
fn check_ways(schema: Option<Version>, channel: Option<u8>, cc: &[FlatCcEntry]) -> Result<(), Fault> {
    let v06 = schema >= Some(Version::V0_6);
    let mut claimed: BTreeMap<Claim, String> = BTreeMap::new();
    for (n, e) in cc.iter().enumerate() {
        let at = format!("/cc_params/entries/{n}");
        if !v06 {
            if e.also.is_some() {
                return Err(Fault { kind: FaultKind::UnknownField, file: None, pointer: format!("{at}/also"), rule: "also is not a field before 0.6".into() });
            }
            if e.value_bits.is_some() {
                return Err(Fault { kind: FaultKind::UnknownField, file: None, pointer: format!("{at}/value_bits"), rule: "value_bits is not a field before 0.6".into() });
            }
            continue;
        }
        let channel = e.channel.or(channel);
        for (m, w) in e.ways().iter().enumerate() {
            let pointer = if m == 0 { at.clone() } else { format!("{at}/also/{}", m - 1) };
            let kinds = [w.has_cc(), w.has_nrpn(), w.has_rpn()].iter().filter(|k| **k).count();
            if kinds > 1 || (m > 0 && kinds != 1) {
                let rule = if kinds == 0 { "a way names a message kind: a cc, an nrpn or an rpn" } else { "a way carries one message kind only: a cc, an nrpn or an rpn" };
                return Err(Fault { kind: FaultKind::BadValue, file: None, pointer, rule: rule.into() });
            }
            check_value_bits(w, &pointer)?;
            for claim in claims(w, channel) {
                if let Some(first) = claimed.get(&claim) {
                    let rule = format!("the message is already claimed at {first}: one control is reached by it");
                    return Err(Fault { kind: FaultKind::BadValue, file: None, pointer, rule });
                }
                claimed.insert(claim, pointer.clone());
            }
        }
    }
    Ok(())
}

/// What the ways of a flat entry of `cc_params` do not say of its message fields, as the text gave
/// them: the fields of a message not fully known (which has no way), a block or a range no way
/// carries, and an entry whose ways are all those of `also`.
#[derive(Clone, Debug, PartialEq)]
pub struct FlatFields {
    /// No way is written on the entry itself, though it has an `also`.
    none_on_entry: bool,
    /// The fields of the entry.
    entry: Map<String, Value>,
    /// The elements of `also` with no way, each with its place in the list.
    also: Vec<(usize, Map<String, Value>)>,
}

impl CcEntry {
    /// The places, relative to the entry, of the flat message fields of a text before 0.7 that none
    /// of the ways says: the number given of each message not fully known (`nrpn_msb`, `cc_lsb`,
    /// `also/0/rpn_lsb`), the flat fields first, then the `nrpn_lsb_range` of the entry when no way
    /// takes it. Empty for an entry of 0.7, and for one whose ways say all its messages. What reads
    /// the ways alone, an exporter for one, names these as what it does not carry.
    pub fn fields_no_way_says(&self) -> Vec<String> {
        let Some(held) = &self.flat else { return Vec::new() };
        let numbers = |e: &Map<String, Value>| -> Vec<String> {
            let incomplete = messages_of(e).into_iter().filter_map(Result::err);
            incomplete.flat_map(|m| m.into_iter().map(|(k, _)| k).filter(|k| k != "value_bits" && k != "range")).collect()
        };
        let mut places = numbers(&held.entry);
        if held.entry.contains_key("nrpn_lsb_range") {
            places.push("nrpn_lsb_range".to_string());
        }
        for (n, element) in &held.also {
            places.extend(numbers(element).into_iter().map(|k| format!("also/{n}/{k}")));
        }
        places
    }
}

impl FlatFields {
    /// What is to be held beside `lifted`, the 0.7 form of the flat entry `e`, so that [`flat_cc`]
    /// writes `e` back as given; `None` when its ways say it all.
    fn of(e: &Map<String, Value>, lifted: &Map<String, Value>, schema: Option<Version>) -> Option<FlatFields> {
        let also = e.get("also").and_then(Value::as_array);
        let none_on_entry = also.is_some_and(|a| !a.is_empty()) && !messages_of(e).iter().any(Result::is_ok);
        let incomplete = also.into_iter().flatten().filter_map(Value::as_object).enumerate().filter(|(_, a)| messages_of(a).iter().any(Result::is_err));
        let mut held = FlatFields { none_on_entry, entry: Map::new(), also: incomplete.map(|(n, a)| (n, a.clone())).collect() };
        // What the ways alone write on the entry.
        let derived = flat_cc(lifted.clone(), schema, Some(&FlatFields { none_on_entry, entry: Map::new(), also: Vec::new() }));
        held.entry = e.iter().filter(|(k, v)| WAY_KEYS.contains(&k.as_str()) && derived.get(*k) != Some(*v)).map(|(k, v)| (k.clone(), v.clone())).collect();
        let ways_moved = none_on_entry && lifted.get("ways").and_then(Value::as_array).is_some_and(|w| !w.is_empty());
        (ways_moved || !held.entry.is_empty() || !held.also.is_empty()).then_some(held)
    }
}

/// The entries of `cc_params` and `params` of a device of a version before 0.7, read in the flat form,
/// refused with the faults of their version, and the device `o` with those entries written as 0.7
/// writes them: the reader then reads the one form. With it, what to hold so that [`flatten`] writes
/// an entry back as given: per entry of `cc_params`, its fields; per entry of `params`, the key of
/// the note that holds the `id` its ways cannot carry.
#[allow(clippy::type_complexity)]
pub(crate) fn lift(o: &Map<String, Value>, schema: Option<Version>, at: &At) -> Result<(Map<String, Value>, Vec<Option<Box<FlatFields>>>, Vec<Option<String>>), Fault> {
    fn entries<'a>(o: &'a Map<String, Value>, table: &str) -> Option<&'a Vec<Value>> {
        o.get(table)?.get("entries")?.as_array()
    }
    let mut cc = Vec::new();
    if let Some(list) = entries(o, "cc_params") {
        let base = At::Key(&At::Key(at, "cc_params"), "entries");
        for (n, v) in list.iter().enumerate() {
            cc.push(FlatCcEntry::read(v, &At::Index(&base, n))?);
        }
    }
    let mut params = Vec::new();
    if let Some(list) = entries(o, "params") {
        let base = At::Key(&At::Key(at, "params"), "entries");
        for (n, v) in list.iter().enumerate() {
            params.push(FlatParamEntry::read(v, &At::Index(&base, n))?);
        }
    }
    let channel = o.get("cc_params").and_then(|c| c.get("channel")).and_then(Value::as_i64).and_then(|c| u8::try_from(c).ok()).filter(|c| *c < 16);
    check_defaults(schema, &cc, &params)?;
    check_precision(schema, &cc, &params)?;
    check_ways(schema, channel, &cc)?;
    let mut out = o.clone();
    let raw = |table: &str, n: usize| entries(o, table).and_then(|l| l.get(n)).and_then(|e| e.get("values")).cloned();
    let written = |mut e: Map<String, Value>, values: Option<Value>| {
        // The named values keep the order of the text.
        if let Some(v @ Value::Object(_)) = values {
            e.insert("values".into(), v);
        }
        e
    };
    // Before 0.6 the value of an NRPN or an RPN is the 14 bits of the data MSB and LSB, the one coding
    // there was: the way says it, as the converter 0.5 to 0.6 writes it.
    let full_width = |mut e: Map<String, Value>| {
        if schema < Some(Version::V0_6) {
            if let Some(Value::Array(ways)) = e.get_mut("ways") {
                for w in ways.iter_mut().filter_map(Value::as_object_mut) {
                    if matches!(w.get("ctrlType").and_then(Value::as_str), Some("nrpn" | "rpn")) {
                        w.insert("value_bits".into(), 14.into());
                    }
                }
            }
        }
        e
    };
    let mut kept = Vec::new();
    let mut cc_lifted = Vec::new();
    for (n, e) in cc.iter().enumerate() {
        let flat = written(object_of(e.write()), raw("cc_params", n));
        let lifted = polarity_of(full_width(ways_of_cc(flat.clone())));
        kept.push(FlatFields::of(&flat, &lifted, schema).map(Box::new));
        cc_lifted.push(lifted);
    }
    let (params_lifted, kept_ids): (Vec<_>, Vec<_>) =
        params.iter().enumerate().map(|(n, e)| ways_of_param(written(object_of(e.write()), raw("params", n)))).unzip();
    for (table, lifted) in [("cc_params", cc_lifted), ("params", params_lifted)] {
        if let Some(Value::Object(t)) = out.get_mut(table) {
            if t.get("entries").is_some_and(Value::is_array) {
                t.insert("entries".into(), Value::Array(lifted.into_iter().map(Value::Object).collect()));
            }
        }
    }
    Ok((out, kept, kept_ids))
}

/// `orientation` of a flat entry as 0.7 says it: `centered` is `bipolar`, `0-based` is `unipolar`, at the
/// same place; an entry without it gets no `polarity`.
fn polarity_of(e: Map<String, Value>) -> Map<String, Value> {
    e.into_iter()
        .map(|(k, v)| match (k.as_str(), v.as_str()) {
            ("orientation", Some("centered")) => ("polarity".to_string(), Value::from("bipolar")),
            ("orientation", Some("0-based")) => ("polarity".to_string(), Value::from("unipolar")),
            _ => (k, v),
        })
        .collect()
}

/// The `orientation` a flat entry says for the `polarity` of 0.7, at the same place.
fn orientation_of(e: Map<String, Value>) -> Map<String, Value> {
    e.into_iter()
        .map(|(k, v)| match (k.as_str(), v.as_str()) {
            ("polarity", Some("bipolar")) => ("orientation".to_string(), Value::from("centered")),
            ("polarity", Some("unipolar")) => ("orientation".to_string(), Value::from("0-based")),
            _ => (k, v),
        })
        .collect()
}

fn object_of(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(o) => o,
        _ => Map::new(),
    }
}

/// The keys a flat way gives, moved off `e` (a flat entry, or a way of `also`).
const WAY_KEYS: [&str; 9] = ["cc", "cc_lsb", "nrpn_msb", "nrpn_lsb", "nrpn_lsb_range", "rpn_msb", "rpn_lsb", "value_bits", "range"];

fn number(e: &Map<String, Value>, key: &str) -> Option<i64> {
    e.get(key).and_then(Value::as_i64)
}

/// The messages of the flat fields of `e`, one per message kind they name, in the order CC, NRPN,
/// RPN: one at most from 0.6, where an entry or a way of `also` mixes no kinds. `Ok` is the way
/// (without direction) of a message fully known: a `cc`, or both numbers of an NRPN or an RPN.
/// A message not fully known (an NRPN or an RPN given by one number, a `cc_lsb` without its `cc`)
/// has no way (`spec/0.7.md`, what the converter writes): `Err` holds the numbers it gives, with
/// the `value_bits` and the `range` of `e`.
fn messages_of(e: &Map<String, Value>) -> Vec<Result<Map<String, Value>, Map<String, Value>>> {
    let mut messages = Vec::new();
    for (kind, first, second) in [("cc", "cc", "cc_lsb"), ("nrpn", "nrpn_msb", "nrpn_lsb"), ("rpn", "rpn_msb", "rpn_lsb")] {
        let (msb, lsb) = (number(e, first), number(e, second));
        if msb.is_none() && lsb.is_none() {
            continue;
        }
        let index = match (msb, lsb) {
            (Some(m), Some(l)) => vec![m, l],
            (Some(m), None) if kind == "cc" => vec![m],
            _ => {
                let keys = [first, second, "value_bits", "range"];
                messages.push(Err(keys.iter().filter_map(|k| e.get(*k).map(|v| ((*k).to_string(), v.clone()))).collect()));
                continue;
            }
        };
        let bits = match kind {
            "cc" => (index.len() == 2).then_some(14),
            _ => number(e, "value_bits"),
        };
        let mut w = Map::new();
        w.insert("ctrlType".into(), kind.into());
        w.insert("ctrlIndex".into(), Value::from(index));
        if let Some(b) = bits {
            w.insert("value_bits".into(), b.into());
        }
        if let Some(r) = e.get("range") {
            w.insert("range".into(), r.clone());
        }
        if let (Some(r), "nrpn") = (e.get("nrpn_lsb_range"), kind) {
            w.insert("nrpn_lsb_range".into(), r.clone());
        }
        messages.push(Ok(w));
    }
    messages
}

/// Whether the message of the kind of the flat fields `keys` that `e` names is fully known.
pub(crate) fn is_complete(e: &Map<String, Value>, keys: [&str; 2]) -> bool {
    // A CC is said by its number alone, an NRPN and an RPN by both.
    e.contains_key(keys[0]) && (keys[0] == "cc" || e.contains_key(keys[1]))
}

/// The messages not fully known of the flat entry `e` of 0.6, in the order written, the flat fields
/// then `also`, as the note `_incomplete` keeps them: the flat message with the numbers it gives,
/// its `value_bits` and, when the entry keeps a way (`way_kept`), its `range`, which else stays on
/// the entry; an element of `also` whole.
pub(crate) fn incomplete_of(e: &Map<String, Value>, way_kept: bool) -> Vec<Map<String, Value>> {
    let flat = messages_of(e).into_iter().filter_map(Result::err).map(|mut m| {
        if !way_kept {
            m.remove("range");
        }
        m
    });
    let also = e.get("also").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_object);
    flat.chain(also.filter(|a| messages_of(a).iter().any(Result::is_err)).cloned()).collect()
}

/// The highest value a way of the 0.7 form can carry: its `range`, else what its width implies.
fn maximum_of(w: &Map<String, Value>) -> i64 {
    scale_of(w)[1]
}

fn scale_of(w: &Map<String, Value>) -> [i64; 2] {
    if let Some([a, b]) = w.get("range").and_then(Value::as_array).map(|r| [r[0].as_i64().unwrap_or(0), r[1].as_i64().unwrap_or(0)]) {
        return [a, b];
    }
    let bits = w.get("value_bits").and_then(Value::as_i64).unwrap_or(7);
    if bits == 14 { [0, 16383] } else { [0, 127] }
}

/// A flat entry of `cc_params` as the 0.7 form: its ways, in order, with the entry's direction on
/// each; `values` as a list; `scale` where the reference is not that of the first message. An entry
/// with no way (no message, or none fully known) keeps its `range`, `transmit`, `recognize` and
/// `nrpn_lsb_range`.
fn ways_of_cc(mut e: Map<String, Value>) -> Map<String, Value> {
    let mut ways: Vec<Map<String, Value>> = Vec::new();
    // The scale of the first message, when that message has no way.
    let mut unsaid_first = None;
    let also = e.get("also").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_object);
    for (n, source) in std::iter::once(&e).chain(also).enumerate() {
        for message in messages_of(source) {
            match message {
                Ok(mut w) => {
                    if n > 0 {
                        w.extend(source.iter().filter(|(k, _)| k.starts_with('_')).map(|(k, v)| (k.clone(), v.clone())));
                    }
                    ways.push(w);
                }
                Err(fields) if ways.is_empty() => {
                    unsaid_first.get_or_insert_with(|| scale_of(&fields));
                }
                Err(_) => {}
            }
        }
    }
    let partial = ways.is_empty();
    for k in WAY_KEYS.iter().chain(["also"].iter()) {
        if !(partial && matches!(*k, "range" | "nrpn_lsb_range")) {
            e.remove(*k);
        }
    }
    let direction: Vec<(String, Value)> = if partial {
        Vec::new()
    } else {
        ["transmit", "recognize"].iter().filter_map(|k| e.remove(*k).map(|v| (k.to_string(), v))).collect()
    };
    let measured = e.contains_key("default") || e.contains_key("values") || e.contains_key("step_count");
    if measured && !partial {
        let scale = match unsaid_first {
            // The reference the ways give by themselves is that of the most precise one.
            Some(first) => {
                let given = ways.iter().map(scale_of).reduce(|best, s| if s[1] > best[1] { s } else { best });
                (given != Some(first)).then_some(first)
            }
            None => ways[1..].iter().any(|w| maximum_of(w) > maximum_of(&ways[0])).then(|| scale_of(&ways[0])),
        };
        if let Some(scale) = scale {
            e.insert("scale".into(), Value::from(scale.to_vec()));
        }
    }
    for w in &mut ways {
        for (k, v) in &direction {
            w.insert(k.clone(), v.clone());
        }
    }
    e.insert("ways".into(), Value::Array(ways.into_iter().map(Value::Object).collect()));
    named_values(&mut e);
    e
}

/// A flat entry of `params` as the 0.7 form: one `sysex` way; an entry with no number has none and
/// keeps its `range`, `transmit` and `recognize`. So does an entry whose one number is a negative
/// `id`, which no way carries: the entry keeps it in a note, whose key is returned.
fn ways_of_param(mut e: Map<String, Value>) -> (Map<String, Value>, Option<String>) {
    let given: Vec<(&str, Value)> = ["pr", "p", "c", "r"].iter().filter_map(|k| e.remove(*k).map(|v| (*k, v))).collect();
    let mut id = e.remove("id");
    let mut id_note = None;
    if given.is_empty() && id.as_ref().and_then(Value::as_i64).is_some_and(|n| n < 0) {
        let key = free_key(&e, "_flat");
        e.insert(key.clone(), Value::Object(id.take().map(|v| ("id".to_string(), v)).into_iter().collect()));
        id_note = Some(key);
    }
    // The numbers `[pr, p, c, r]` as the entry gives them, else `[id]`. One number alone reads as an
    // `id`: `pr` alone is said by the note.
    let prefix = given.iter().zip(["pr", "p", "c", "r"]).all(|((k, _), want)| *k == want);
    let exact = if given.is_empty() { true } else { prefix && given.len() > 1 && id.is_none() };
    let mut flat = Map::new();
    if !exact {
        flat.extend(given.iter().map(|(k, v)| ((*k).to_string(), v.clone())));
        flat.extend(id.iter().map(|v| ("id".to_string(), v.clone())));
    }
    let index: Vec<Value> = if given.is_empty() { id.into_iter().collect() } else { given.into_iter().map(|(_, v)| v).collect() };
    let mut ways = Vec::new();
    if !index.is_empty() {
        let mut w = Map::new();
        w.insert("ctrlType".into(), "sysex".into());
        w.insert("ctrlIndex".into(), Value::Array(index));
        if !exact {
            // What the numbers alone cannot say: which fields of 0.6 the entry gave, kept as a note.
            w.insert("_flat".into(), Value::Object(flat));
        }
        for k in ["range", "transmit", "recognize"] {
            if let Some(v) = e.remove(k) {
                w.insert(k.into(), v);
            }
        }
        ways.push(Value::Object(w));
    }
    e.insert("ways".into(), Value::Array(ways));
    named_values(&mut e);
    (e, id_note)
}

/// `values` of 0.6, a dictionary of label to integer, as the list of `{value, title}` of 0.7.
fn named_values(e: &mut Map<String, Value>) {
    if let Some(Value::Object(dict)) = e.get("values") {
        let list = dict
            .iter()
            .map(|(title, value)| {
                let mut item = Map::new();
                item.insert("value".into(), value.clone());
                item.insert("title".into(), Value::String(title.clone()));
                Value::Object(item)
            })
            .collect();
        e.insert("values".into(), Value::Array(list));
    }
}

/// The text of a device of an older version as it had it: the entries of `cc_params` and `params`
/// written in the flat form again; `kept` holds, per entry of `cc_params`, what its ways alone do
/// not say, and `kept_ids`, per entry of `params`, the key of the note that holds its `id`.
pub(crate) fn flatten(v: &mut Value, schema: Option<Version>, kept: &[Option<&FlatFields>], kept_ids: &[Option<&str>]) {
    for table in ["cc_params", "params"] {
        let Some(Value::Array(entries)) = v.get_mut(table).and_then(|t| t.get_mut("entries")) else { continue };
        for (n, entry) in entries.iter_mut().enumerate() {
            let Value::Object(e) = entry else { continue };
            *e = if table == "cc_params" {
                flat_cc(std::mem::take(e), schema, kept.get(n).copied().flatten())
            } else {
                flat_param(std::mem::take(e), kept_ids.get(n).copied().flatten())
            };
        }
    }
}

/// The keys of the flat fields of a way `w` of the 0.7 form.
fn flat_way(w: &Map<String, Value>, first: bool) -> Map<String, Value> {
    let mut out = Map::new();
    let index: Vec<Value> = w.get("ctrlIndex").and_then(Value::as_array).cloned().unwrap_or_default();
    let kind = w.get("ctrlType").and_then(Value::as_str).unwrap_or_default();
    let names: &[&str] = match kind {
        "cc" => &["cc", "cc_lsb"],
        "nrpn" => &["nrpn_msb", "nrpn_lsb"],
        "rpn" => &["rpn_msb", "rpn_lsb"],
        _ => &[],
    };
    for (name, n) in names.iter().zip(index) {
        out.insert((*name).into(), n);
    }
    if kind != "cc" {
        if let Some(b) = w.get("value_bits") {
            out.insert("value_bits".into(), b.clone());
        }
    }
    if let Some(r) = w.get("range") {
        out.insert("range".into(), r.clone());
    }
    if first {
        if let Some(r) = w.get("nrpn_lsb_range") {
            out.insert("nrpn_lsb_range".into(), r.clone());
        }
    }
    if !first {
        out.extend(w.iter().filter(|(k, _)| k.starts_with('_')).map(|(k, v)| (k.clone(), v.clone())));
    }
    out
}

/// How many of the first ways are written on the entry itself, the others in `also`: none when
/// the entry's ways are all those of `also`; before 0.6, where there is no `also`, one way of each kind; from
/// 0.6, the first.
fn ways_on_entry(ways: &[Map<String, Value>], schema: Option<Version>, kept: Option<&FlatFields>) -> usize {
    if kept.is_some_and(|f| f.none_on_entry) {
        return 0;
    }
    if schema >= Some(Version::V0_6) {
        return ways.len().min(1);
    }
    let kind = |w: &Map<String, Value>| w.get("ctrlType").cloned();
    ways.iter().enumerate().take_while(|(n, w)| ways[..*n].iter().all(|earlier| kind(earlier) != kind(w))).count()
}

fn flat_cc(mut e: Map<String, Value>, schema: Option<Version>, kept: Option<&FlatFields>) -> Map<String, Value> {
    let ways: Vec<Map<String, Value>> = match e.remove("ways") {
        Some(Value::Array(w)) => w.into_iter().filter_map(|w| if let Value::Object(o) = w { Some(o) } else { None }).collect(),
        _ => Vec::new(),
    };
    e.remove("scale");
    if let Some(first) = ways.first() {
        for k in ["transmit", "recognize"] {
            if let Some(v) = first.get(k) {
                e.insert(k.into(), v.clone());
            }
        }
    }
    let (on_entry, in_also) = ways.split_at(ways_on_entry(&ways, schema, kept));
    for w in on_entry {
        // The ways of one entry share its range: the first says it.
        for (k, v) in flat_way(w, true) {
            e.entry(k).or_insert(v);
        }
    }
    let mut also: Vec<Value> = in_also.iter().map(|w| Value::Object(flat_way(w, false))).collect();
    if let Some(held) = kept {
        for (n, element) in &held.also {
            also.insert((*n).min(also.len()), Value::Object(element.clone()));
        }
    }
    if !also.is_empty() {
        e.insert("also".into(), Value::Array(also));
    }
    // Before 0.6 no control gives a width: the 14 bits of an NRPN or an RPN are the one coding.
    if schema < Some(Version::V0_6) {
        e.remove("value_bits");
    }
    for (k, v) in kept.iter().flat_map(|f| &f.entry) {
        e.entry(k.clone()).or_insert_with(|| v.clone());
    }
    dictionary(&mut e);
    orientation_of(e)
}

fn flat_param(mut e: Map<String, Value>, id_note: Option<&str>) -> Map<String, Value> {
    let first = match e.remove("ways") {
        Some(Value::Array(w)) => w.into_iter().find_map(|w| if let Value::Object(o) = w { Some(o) } else { None }),
        _ => None,
    };
    e.remove("scale");
    if let Some(Value::Object(held)) = id_note.and_then(|key| e.remove(key)) {
        e.extend(held);
    }
    if let Some(w) = first {
        let index: Vec<Value> = w.get("ctrlIndex").and_then(Value::as_array).cloned().unwrap_or_default();
        if let Some(Value::Object(given)) = w.get("_flat") {
            e.extend(given.iter().map(|(k, v)| (k.clone(), v.clone())));
        } else if index.len() == 1 {
            e.insert("id".into(), index[0].clone());
        } else {
            for (k, n) in ["pr", "p", "c", "r"].iter().zip(index) {
                e.insert((*k).into(), n);
            }
        }
        for k in ["range", "transmit", "recognize"] {
            if let Some(v) = w.get(k) {
                e.insert(k.into(), v.clone());
            }
        }
    }
    dictionary(&mut e);
    e
}

/// `values` of 0.7, a list of `{value, title}`, as the dictionary of 0.6.
fn dictionary(e: &mut Map<String, Value>) {
    if let Some(Value::Array(list)) = e.get("values") {
        let dict: Map<String, Value> = list
            .iter()
            .filter_map(|i| Some((i.get("title")?.as_str()?.to_string(), i.get("value")?.clone())))
            .collect();
        e.insert("values".into(), Value::Object(dict));
    }
}
