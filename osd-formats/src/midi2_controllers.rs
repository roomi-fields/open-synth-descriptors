//! The controller resources of MIDI-CI Property Exchange (M2-117-UM v1.0): `ChCtrlList`,
//! `AllCtrlList` and `CtrlMapList`, in JSON. The importer and the exporter; the fate of each
//! property of the controller object (tables 6 and 12) is in docs/midi2-controllers.md.

use crate::midi_csv::check_source;
use crate::{Fault, FaultKind, Loss, Translated};
use osd::{Descriptor, Device, Resolved, Source};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

const SCHEMA_URL: &str = "https://w3id.org/osd/0.6/device.schema.json";

/// The state of the role vocabulary an import names.
const VOCABULARY_STATE: &str = "1";

fn fault(kind: FaultKind, place: impl Into<String>, rule: impl Into<String>) -> Fault {
    Fault { kind, place: place.into(), rule: rule.into() }
}

fn invalid(place: impl Into<String>, rule: impl Into<String>) -> Fault {
    fault(FaultKind::InvalidFile, place, rule)
}

fn loss(losses: &mut Vec<Loss>, place: impl Into<String>, reason: impl Into<String>) {
    losses.push(Loss { place: place.into(), reason: reason.into() });
}

/// A key as a JSON pointer segment.
fn segment(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

// ---- import -----------------------------------------------------------------------------

/// The ways a control is set, by `ctrlType`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Cc,
    Nrpn,
    Rpn,
}

/// The types of the standard (M2-117-UM v1.0, tables 6 and 12) a descriptor holds no control for.
const STANDARD_TYPES_NOT_CARRIED: [&str; 6] = ["chPress", "pPress", "pBend", "pnrc", "pnac", "pnp"];

/// The number of bits a count of values needs, from the standard's usual widths.
fn bits_of(max: i64) -> i64 {
    match max {
        ..=127 => 7,
        128..=255 => 8,
        256..=16383 => 14,
        _ => (64 - (max as u64).leading_zeros() as i64).clamp(14, 32),
    }
}

/// How a control's value scales to the 32 bits of the norm (M2-117-UM v1.0 section 1.5, M2-115-U v1.0.2):
/// Zero-Extension for an RPN whose index LSB is 0 to 31 (section 4.1), Min-Center-Max for the rest (section 3.1).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scale {
    MinCenterMax,
    ZeroExtension,
}

impl Scale {
    fn of(kind: Kind, index: &[i64]) -> Scale {
        if kind == Kind::Rpn && index.get(1).is_some_and(|l| *l < 32) {
            Scale::ZeroExtension
        } else {
            Scale::MinCenterMax
        }
    }

    /// A value of `src` bits scaled up to `dst` bits (sections 3.3 and 4.3).
    fn up(self, v: u64, src: u32, dst: u32) -> u64 {
        if src >= dst {
            return v;
        }
        let shift = dst - src;
        if self == Scale::ZeroExtension {
            return v << shift;
        }
        if src == 1 {
            return if v == 0 { 0 } else { (1u64 << dst) - 1 };
        }
        let shifted = v << shift;
        if v <= 1u64 << (src - 1) {
            return shifted;
        }
        let repeat_bits = src - 1;
        let mut repeat = v & ((1u64 << repeat_bits) - 1);
        repeat = if shift > repeat_bits { repeat << (shift - repeat_bits) } else { repeat >> (repeat_bits - shift) };
        let mut out = shifted;
        while repeat != 0 {
            out |= repeat;
            repeat >>= repeat_bits;
        }
        out
    }

    /// A value of `src` bits scaled down to `dst` bits: the low bits cut off (section 3.4), or
    /// rounded and clamped for Zero-Extension (section 4.4).
    fn down(self, v: u64, src: u32, dst: u32) -> u64 {
        if src <= dst {
            return v;
        }
        let shift = src - dst;
        match self {
            Scale::MinCenterMax => v >> shift,
            Scale::ZeroExtension => ((v + (1u64 << (shift - 1))) >> shift).min((1u64 << dst) - 1),
        }
    }
}

fn int_at(o: &Map<String, Value>, key: &str, ptr: &str) -> Result<Option<i64>, Fault> {
    match o.get(key) {
        None => Ok(None),
        Some(v) => v.as_i64().map(Some).ok_or_else(|| invalid(format!("{ptr}/{key}"), format!("{key} is an integer"))),
    }
}

fn index_of(o: &Map<String, Value>, ptr: &str, len: usize) -> Result<Vec<i64>, Fault> {
    let at = format!("{ptr}/ctrlIndex");
    let Some(v) = o.get("ctrlIndex") else {
        return Err(invalid(at, "an entry has its ctrlIndex"));
    };
    let items: Vec<i64> = v
        .as_array()
        .and_then(|a| a.iter().map(Value::as_i64).collect::<Option<Vec<_>>>())
        .filter(|a| a.len() == len)
        .ok_or_else(|| invalid(at.clone(), format!("ctrlIndex is an array of {len} integer(s) for this ctrlType")))?;
    if len == 2 && items.iter().any(|i| !(0..=127).contains(i)) {
        return Err(invalid(at, "the bytes of an NRPN or RPN index are from 0 to 127"));
    }
    if len == 1 && items[0] < 0 {
        return Err(invalid(at, "an index is not negative"));
    }
    Ok(items)
}

/// Whether a role, set on a control of a descriptor naming the vocabulary state, is read strictly.
fn role_reads(role: &Value) -> bool {
    let doc = json!({
        "$schema": SCHEMA_URL,
        "device": {"name": "x", "osc_prefix": "/x"},
        "role_vocabulary": VOCABULARY_STATE,
        "cc_params": {"entries": [{"osc": "/x", "cc": 0, "role": role}]},
    });
    osd::read(&doc.to_string(), osd::Kind::Device).is_ok()
}

fn role_json(role: &osd::Role) -> Value {
    let parts: Vec<Value> = role
        .parts
        .iter()
        .map(|p| {
            let mut o = Map::new();
            o.insert("part".into(), json!(p.part));
            if let Some(i) = p.index {
                o.insert("index".into(), json!(i));
            }
            Value::Object(o)
        })
        .collect();
    json!({"parts": parts, "setting": role.setting})
}

const DIRECTIONS: [&str; 4] = ["absolute", "relative", "both", "none"];

const ENTRY_KEYS: [&str; 15] = [
    "title",
    "description",
    "channel",
    "ctrlType",
    "ctrlIndex",
    "priority",
    "default",
    "transmit",
    "recognize",
    "numSigBits",
    "paramPath",
    "typeHint",
    "ctrlMapId",
    "stepCount",
    "minMax",
];

/// The named values of the map an entry points at.
fn map_values(maps: &[(Value, Value)], id: &Value) -> Option<Value> {
    maps.iter().find(|(i, _)| i == id).map(|(_, v)| v.clone())
}

fn read_maps(root: &Map<String, Value>, losses: &mut Vec<Loss>) -> Result<Vec<(Value, Value)>, Fault> {
    let mut out = Vec::new();
    let Some(list) = root.get("CtrlMapList") else { return Ok(out) };
    let Some(list) = list.as_array() else {
        return Err(invalid("/CtrlMapList", "CtrlMapList is an array of maps"));
    };
    for (n, m) in list.iter().enumerate() {
        let ptr = format!("/CtrlMapList/{n}");
        let Some(o) = m.as_object() else { return Err(invalid(ptr, "a map is a JSON object")) };
        let Some(id) = o.get("ctrlMapId").filter(|i| i.is_string() || i.is_number()) else {
            return Err(invalid(format!("{ptr}/ctrlMapId"), "a map has its ctrlMapId, a string or a number"));
        };
        let Some(items) = o.get("ctrlMap").and_then(Value::as_array) else {
            return Err(invalid(format!("{ptr}/ctrlMap"), "a map has its ctrlMap, an array of values with their titles"));
        };
        let mut values = Map::new();
        for (k, item) in items.iter().enumerate() {
            let at = format!("{ptr}/ctrlMap/{k}");
            let (Some(value), Some(title)) = (item.get("value").and_then(Value::as_i64), item.get("title").and_then(Value::as_str)) else {
                return Err(invalid(at, "a value of a map has its value, an integer, and its title"));
            };
            if values.insert(title.to_string(), json!(value)).is_some() {
                return Err(invalid(format!("{at}/title"), format!("the title {title:?} names two values of the map")));
            }
            for key in item.as_object().into_iter().flat_map(|i| i.keys()).filter(|k| *k != "value" && *k != "title") {
                loss(losses, format!("{at}/{}", segment(key)), "the descriptor has no field for it");
            }
        }
        for key in o.keys().filter(|k| *k != "ctrlMapId" && *k != "ctrlMap") {
            loss(losses, format!("{ptr}/{}", segment(key)), "the descriptor has no field for it");
        }
        out.push((id.clone(), Value::Object(values)));
    }
    Ok(out)
}

struct Built {
    cc: Vec<Value>,
    any_role: bool,
}

#[allow(clippy::too_many_arguments)]
fn read_entry(
    e: &Value,
    ptr: &str,
    maps: &[(Value, Value)],
    used: &mut BTreeSet<String>,
    built: &mut Built,
    losses: &mut Vec<Loss>,
) -> Result<(), Fault> {
    let Some(o) = e.as_object() else { return Err(invalid(ptr, "an entry of the list is a JSON object")) };
    let kind = match o.get("ctrlType").map(|t| t.as_str()) {
        None => return Err(invalid(format!("{ptr}/ctrlType"), "an entry has its ctrlType")),
        Some(Some("cc")) => Kind::Cc,
        Some(Some("nrpn")) => Kind::Nrpn,
        Some(Some("rpn")) => Kind::Rpn,
        Some(Some(t)) if STANDARD_TYPES_NOT_CARRIED.contains(&t) => {
            loss(losses, ptr, format!("the ctrlType {t} is a type of the standard that the descriptor has no field for: the entry is not carried"));
            return Ok(());
        }
        Some(_) => return Err(invalid(format!("{ptr}/ctrlType"), "ctrlType is one of the nine types of the standard: cc, chPress, pPress, nrpn, rpn, pBend, pnrc, pnac, pnp")),
    };
    let index = index_of(o, ptr, if matches!(kind, Kind::Nrpn | Kind::Rpn) { 2 } else { 1 })?;
    if kind == Kind::Cc && index[0] > 127 {
        return Err(invalid(format!("{ptr}/ctrlIndex"), "a Control Change number is from 0 to 127"));
    }

    let mut entry = Map::new();
    let natural = match kind {
        Kind::Cc => {
            entry.insert("cc".into(), json!(index[0]));
            7
        }
        Kind::Nrpn => {
            entry.insert("nrpn_msb".into(), json!(index[0]));
            entry.insert("nrpn_lsb".into(), json!(index[1]));
            14
        }
        Kind::Rpn => {
            entry.insert("rpn_msb".into(), json!(index[0]));
            entry.insert("rpn_lsb".into(), json!(index[1]));
            14
        }
    };

    let bits = match int_at(o, "numSigBits", ptr)? {
        Some(b) if (1..=32).contains(&b) => Some(b),
        Some(_) => return Err(invalid(format!("{ptr}/numSigBits"), "numSigBits is from 1 to 32")),
        None => None,
    };
    // The values are unsigned 32-bit (M2-117-UM v1.0); they scale down to numSigBits bits (32 when not provided).
    let scale = Scale::of(kind, &index);
    let width = bits.unwrap_or(32) as u32;
    let down = |v: i64| scale.down(v as u64, 32, width) as i64;
    let in_32 = |v: i64| (0..=u32::MAX as i64).contains(&v);
    let range = match o.get("minMax") {
        None => None,
        Some(v) => {
            let pair = v.as_array().filter(|a| a.len() == 2).and_then(|a| Some([a[0].as_i64()?, a[1].as_i64()?]));
            match pair {
                Some([lo, hi]) if lo <= hi && in_32(lo) && in_32(hi) => Some([down(lo), down(hi)]),
                _ => return Err(invalid(format!("{ptr}/minMax"), "minMax is [lowest, highest], two unsigned 32-bit integers in order")),
            }
        }
    };
    if kind != Kind::Cc {
        // numSigBits 7 is the data MSB alone; any other width of an NRPN or an RPN is read as the 14 bits of MSB and LSB.
        entry.insert("value_bits".into(), json!(if bits == Some(7) { 7 } else { 14 }));
    }
    let mut range_out = range;
    if kind == Kind::Cc && bits == Some(14) {
        if index[0] < 32 {
            entry.insert("cc_lsb".into(), json!(index[0] + 32));
        } else {
            loss(losses, format!("{ptr}/numSigBits"), "a 14-bit Control Change pairs a number below 32 with its LSB at +32: the pair is not kept");
        }
    } else if let (None, Some(b)) = (range, bits) {
        if b != natural {
            range_out = Some([0, (1i64 << b) - 1]);
        }
    }
    if let Some(r) = range_out {
        entry.insert("range".into(), json!(r));
    }
    if let Some(d) = int_at(o, "default", ptr)? {
        if !in_32(d) {
            return Err(invalid(format!("{ptr}/default"), "default is an unsigned 32-bit integer"));
        }
        let d = down(d);
        if let Some([lo, hi]) = range_out {
            if d < lo || d > hi {
                return Err(invalid(format!("{ptr}/default"), format!("the default {d} is outside the range {lo}..{hi}")));
            }
        }
        entry.insert("default".into(), json!(d));
    }
    if let Some(p) = int_at(o, "priority", ptr)? {
        if !(1..=5).contains(&p) {
            return Err(invalid(format!("{ptr}/priority"), "priority is from 1 to 5"));
        }
        entry.insert("priority".into(), json!(p));
    }
    if let Some(s) = int_at(o, "stepCount", ptr)? {
        if s < 2 {
            return Err(invalid(format!("{ptr}/stepCount"), "stepCount is 2 or more"));
        }
        entry.insert("step_count".into(), json!(s));
    }
    for (from, to) in [("transmit", "transmit"), ("recognize", "recognize")] {
        if let Some(v) = o.get(from) {
            match v.as_str().filter(|s| DIRECTIONS.contains(s)) {
                Some(s) => {
                    entry.insert(to.into(), json!(s));
                }
                None => return Err(invalid(format!("{ptr}/{from}"), format!("{from} is absolute, relative, both or none"))),
            }
        }
    }
    if let Some(c) = int_at(o, "channel", ptr)? {
        if !(1..=256).contains(&c) {
            return Err(invalid(format!("{ptr}/channel"), "channel is from 1 to 256, the norm counting from 1"));
        }
        if c > 16 {
            let group = (c - 1) / 16 + 1;
            return Err(invalid(
                format!("{ptr}/channel"),
                format!("channel {c} falls in Group {group} of the Function Block: the descriptor's channel has no Group, it is from 1 to 16"),
            ));
        }
        entry.insert("channel".into(), json!(c - 1));
    }
    match o.get("typeHint").map(|t| t.as_str()) {
        None => {}
        Some(Some("continuous")) => {
            entry.insert("expects".into(), json!("continuous"));
        }
        Some(Some("toggle")) => {
            entry.insert("expects".into(), json!("switch"));
        }
        Some(Some("momentary")) => {
            entry.insert("expects".into(), json!("momentary"));
        }
        Some(Some("valueSelect")) => {
            entry.insert("expects".into(), json!("discrete"));
        }
        Some(Some("relative")) => loss(losses, format!("{ptr}/typeHint"), "relative has no expects word: transmit and recognize carry it"),
        Some(_) => return Err(invalid(format!("{ptr}/typeHint"), "typeHint is continuous, toggle, momentary, relative or valueSelect")),
    }
    if let Some(id) = o.get("ctrlMapId") {
        let Some(values) = map_values(maps, id) else {
            return Err(invalid(format!("{ptr}/ctrlMapId"), format!("the map {id} is not in the CtrlMapList")));
        };
        if let Some([lo, hi]) = range_out {
            if values.as_object().into_iter().flatten().any(|(_, v)| v.as_i64().is_some_and(|v| v < lo || v > hi)) {
                return Err(invalid(format!("{ptr}/ctrlMapId"), format!("a value of the map {id} is outside the range {lo}..{hi}")));
            }
        }
        entry.insert("values".into(), values);
    }

    // The path of the control: a role, else a section, else nothing.
    if let Some(p) = o.get("paramPath") {
        let Some(path) = p.as_str() else { return Err(invalid(format!("{ptr}/paramPath"), "paramPath is a string")) };
        let role = osd::role_from_param_path(path, VOCABULARY_STATE).ok().map(|r| role_json(&r)).filter(role_reads);
        match role {
            Some(r) => {
                entry.insert("role".into(), r);
                built.any_role = true;
            }
            None => {
                loss(losses, ptr, format!("the paramPath {path:?} has no known role"));
                if path.starts_with('/') {
                    entry.insert("section".into(), json!(path));
                }
            }
        }
    }

    // What nothing carries.
    if o.contains_key("title") {
        loss(losses, format!("{ptr}/title"), "the descriptor holds no title: the label belongs to the role and to the tool that lays it out");
    }
    if o.contains_key("description") {
        loss(losses, format!("{ptr}/description"), "free text for people is not read by a program: the descriptor has no field for it");
    }
    for key in o.keys().filter(|k| !ENTRY_KEYS.contains(&k.as_str())) {
        loss(losses, format!("{ptr}/{}", segment(key)), "the descriptor has no field for it");
    }

    // The address of the control in the descriptor: its own, from what sets it.
    let base = match kind {
        Kind::Cc => format!("/cc/{}", index[0]),
        Kind::Nrpn => format!("/nrpn/{}/{}", index[0], index[1]),
        Kind::Rpn => format!("/rpn/{}/{}", index[0], index[1]),
    };
    let mut osc = base.clone();
    let mut k = 2;
    while !used.insert(osc.clone()) {
        osc = format!("{base}_{k}");
        k += 1;
    }
    let mut full = Map::new();
    full.insert("osc".into(), json!(osc));
    full.extend(entry);
    built.cc.push(Value::Object(full));
    Ok(())
}

/// The host of a URL: what stands between its scheme and its first `/`.
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("").split(':').next().unwrap_or("");
    (!host.is_empty()).then(|| host.to_string())
}

pub(crate) fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let root: Value = serde_json::from_str(text).map_err(|e| invalid("", format!("the file is not valid JSON: {e}")))?;
    let mut losses = Vec::new();
    let (list, base, maps) = match &root {
        Value::Array(a) => (a, String::new(), Vec::new()),
        Value::Object(o) => {
            let key = ["ChCtrlList", "AllCtrlList"].into_iter().find(|k| o.contains_key(*k));
            let Some(key) = key else {
                return Err(invalid("", "an object of controllers holds a ChCtrlList or an AllCtrlList"));
            };
            let Some(list) = o[key].as_array() else {
                return Err(invalid(format!("/{key}"), format!("{key} is an array of entries")));
            };
            let maps = read_maps(o, &mut losses)?;
            for k in o.keys().filter(|k| !["ChCtrlList", "AllCtrlList", "CtrlMapList"].contains(&k.as_str())) {
                loss(&mut losses, format!("/{}", segment(k)), "the descriptor has no field for it");
            }
            (list, format!("/{key}"), maps)
        }
        _ => return Err(invalid("", "a controller list is a JSON array of entries, or an object holding one")),
    };
    let mut built = Built { cc: Vec::new(), any_role: false };
    let mut used = BTreeSet::new();
    for (n, e) in list.iter().enumerate() {
        read_entry(e, &format!("{base}/{n}"), &maps, &mut used, &mut built, &mut losses)?;
    }

    let vendor = source
        .author
        .clone()
        .filter(|a| !a.trim().is_empty())
        .or_else(|| source.url.as_deref().and_then(host_of))
        .or_else(|| source.attribution.clone().filter(|a| !a.trim().is_empty()));
    let mut device = Map::new();
    device.insert("name".into(), json!("MIDI 2.0 device"));
    if let Some(v) = &vendor {
        device.insert("vendor".into(), json!(v));
        loss(&mut losses, "/vendor", "identity not in the list: the controller list says nothing of who makes the device, the vendor is the source's author");
    }
    device.insert("osc_prefix".into(), json!("/midi2"));
    loss(&mut losses, "/name", "identity not in the list: the controller list says nothing of the device's name, a fixed name stands");

    let mut doc = Map::new();
    doc.insert("$schema".into(), json!(SCHEMA_URL));
    doc.insert("device".into(), Value::Object(device));
    if built.any_role {
        doc.insert("role_vocabulary".into(), json!(VOCABULARY_STATE));
    }
    if !built.cc.is_empty() {
        doc.insert("cc_params".into(), json!({"entries": built.cc}));
    }
    let read = osd::read(&Value::Object(doc).to_string(), osd::Kind::Device)
        .map_err(|f| invalid(base.clone(), format!("the list does not make a device descriptor: {} at {}", f.rule, f.pointer)))?;
    let Descriptor::Device(mut d) = read else {
        return Err(invalid("", "the import does not make a device"));
    };
    d.sources = Some(vec![source.clone()]);
    Ok(Translated { value: *d, lost: losses })
}

// ---- export -----------------------------------------------------------------------------

/// The keys of an entry the export carries, by what it writes from them.
const CC_CARRIED: [&str; 19] = [
    "osc", "cc", "cc_lsb", "nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb", "value_bits", "range", "default", "priority", "step_count", "transmit", "recognize", "values", "channel", "section", "expects",
    "role",
];

/// The maps written: equal value lists share one id.
#[derive(Default)]
struct Maps {
    lists: Vec<(Vec<(i64, String)>, String)>,
}

impl Maps {
    fn id_of(&mut self, values: &Map<String, Value>) -> String {
        let mut list: Vec<(i64, String)> = values.iter().filter_map(|(k, v)| Some((v.as_i64()?, k.clone()))).collect();
        list.sort();
        if let Some((_, id)) = self.lists.iter().find(|(l, _)| *l == list) {
            return id.clone();
        }
        let id = format!("map{}", self.lists.len() + 1);
        self.lists.push((list, id.clone()));
        id
    }

    fn to_json(&self) -> Vec<Value> {
        self.lists
            .iter()
            .map(|(l, id)| json!({"ctrlMapId": id, "ctrlMap": l.iter().map(|(v, t)| json!({"value": v, "title": t})).collect::<Vec<_>>()}))
            .collect()
    }
}

fn export_entry(e: &Map<String, Value>, ptr: &str, block_channel: Option<i64>, role_path: Option<&str>, maps: &mut Maps, losses: &mut Vec<Loss>) -> Option<Value> {
    for key in e.keys().filter(|k| !CC_CARRIED.contains(&k.as_str())) {
        loss(losses, format!("{ptr}/{}", segment(key)), "the Property Exchange controller resources have no property for it");
    }
    let num = |k: &str| e.get(k).and_then(Value::as_i64);
    let (ctrl_type, index, natural) = if let Some(cc) = num("cc") {
        ("cc", vec![cc], 7)
    } else if let (Some(a), Some(b)) = (num("nrpn_msb"), num("nrpn_lsb")) {
        ("nrpn", vec![a, b], num("value_bits").unwrap_or(14))
    } else if let (Some(a), Some(b)) = (num("rpn_msb"), num("rpn_lsb")) {
        ("rpn", vec![a, b], num("value_bits").unwrap_or(14))
    } else {
        loss(losses, ptr, "the control has no complete address (cc, nrpn_msb and nrpn_lsb, or rpn_msb and rpn_lsb): it is not written");
        return None;
    };

    let osc = e.get("osc").and_then(Value::as_str).unwrap_or("");
    let section = e.get("section").and_then(Value::as_str);
    let slash_section = section.filter(|s| s.starts_with('/'));
    let param_path = role_path.or(if role_path.is_none() { slash_section } else { None });
    let title = param_path.map_or_else(|| osc.to_string(), str::to_string);

    let mut o = Map::new();
    o.insert("title".into(), json!(title));
    if let Some(c) = num("channel").or(block_channel) {
        o.insert("channel".into(), json!(c + 1));
    }
    o.insert("ctrlType".into(), json!(ctrl_type));
    o.insert("ctrlIndex".into(), json!(index));
    for (from, to) in [("priority", "priority"), ("transmit", "transmit"), ("recognize", "recognize")] {
        if let Some(v) = e.get(from) {
            o.insert(to.into(), v.clone());
        }
    }
    let mut lsb_pair = false;
    if let Some(lsb) = num("cc_lsb") {
        if ctrl_type == "cc" && lsb == index[0] + 32 {
            lsb_pair = true;
        } else {
            loss(losses, format!("{ptr}/cc_lsb"), "a 14-bit Control Change is written as numSigBits 14 on its MSB: an LSB other than the MSB + 32 has no property");
        }
    }
    let range = match e.get("range").and_then(Value::as_array).filter(|a| a.len() == 2).and_then(|a| Some([a[0].as_i64()?, a[1].as_i64()?])) {
        Some([lo, hi]) if lo > hi => {
            loss(losses, format!("{ptr}/range"), "the lowest value is above the highest");
            None
        }
        Some([lo, hi]) if lo < 0 || hi < 0 => {
            loss(losses, format!("{ptr}/range"), "minMax is unsigned 32-bit values: a negative bound is not written");
            None
        }
        r => r,
    };
    let bits = if lsb_pair {
        14
    } else if let Some([lo, hi]) = range {
        bits_of(hi.max(lo))
    } else {
        natural
    };
    o.insert("numSigBits".into(), json!(bits));
    // The values are written as unsigned 32-bit values (M2-117-UM v1.0), scaled up from numSigBits bits.
    let kind = match ctrl_type {
        "rpn" => Kind::Rpn,
        "nrpn" => Kind::Nrpn,
        _ => Kind::Cc,
    };
    let scale = Scale::of(kind, &index);
    let up = |v: i64| scale.up(v as u64, bits as u32, 32);
    if let Some([lo, hi]) = range {
        o.insert("minMax".into(), json!([up(lo), up(hi)]));
    }
    if let Some(d) = e.get("default") {
        match d.as_i64() {
            Some(d) if d < 0 => loss(losses, format!("{ptr}/default"), "default is an unsigned 32-bit value: a negative default is not written"),
            Some(d) if bits < 32 && d >= 1i64 << bits => loss(losses, format!("{ptr}/default"), format!("the default {d} does not fit in numSigBits {bits}: it is not written")),
            Some(d) => {
                o.insert("default".into(), json!(up(d)));
            }
            None => loss(losses, format!("{ptr}/default"), "the default is not an integer: it is not written"),
        }
    }
    if let Some(s) = e.get("step_count") {
        o.insert("stepCount".into(), s.clone());
    }
    if let Some(p) = param_path {
        o.insert("paramPath".into(), json!(p));
    }
    if section.is_some() {
        if role_path.is_some() {
            loss(losses, format!("{ptr}/section"), "the control has a role: its paramPath says the role, the section has no property");
        } else if slash_section.is_none() {
            loss(losses, format!("{ptr}/section"), "a free section is not a path: it is never written as a paramPath");
        }
    }
    match e.get("expects").and_then(Value::as_str) {
        None => {}
        Some("continuous") => {
            o.insert("typeHint".into(), json!("continuous"));
        }
        Some("switch") => {
            o.insert("typeHint".into(), json!("toggle"));
        }
        Some("momentary") => {
            o.insert("typeHint".into(), json!("momentary"));
        }
        Some("discrete") => {
            o.insert("typeHint".into(), json!("valueSelect"));
        }
        Some("trigger") => {
            o.insert("typeHint".into(), json!("momentary"));
            loss(losses, format!("{ptr}/expects"), "the standard has no trigger: it is written as momentary");
        }
        Some("clock") => loss(losses, format!("{ptr}/expects"), "the standard has no clock: the control carries no typeHint"),
        Some(other) => loss(losses, format!("{ptr}/expects"), format!("{other:?} has no typeHint word: the control carries no typeHint")),
    }
    if let Some(values) = e.get("values").and_then(Value::as_object).filter(|v| !v.is_empty()) {
        o.insert("ctrlMapId".into(), json!(maps.id_of(values)));
    }
    Some(Value::Object(o))
}

pub(crate) fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    if d.sources.as_ref().is_none_or(|s| s.is_empty()) {
        return Err(fault(FaultKind::NoSources, "/_sources", "a device without sources is not written: its credit and licence cannot be"));
    }
    let descriptor = Descriptor::Device(Box::new(d.clone()));
    let doc: Value = serde_json::from_str(&osd::write(&descriptor)).expect("a descriptor is JSON");
    let role_paths: Vec<(String, String)> = osd::roles(&descriptor).into_iter().map(|r| (r.pointer, r.param_path)).collect();
    let mut losses = Vec::new();

    // What the list holds nothing of.
    if let Some(info) = doc.get("device").and_then(Value::as_object) {
        for key in info.keys() {
            loss(&mut losses, format!("/device/{}", segment(key)), "the controller list says nothing of the device itself");
        }
    }
    if let Some(top) = doc.as_object() {
        for key in top.keys().filter(|k| !["$schema", "device", "role_vocabulary", "cc_params", "params", "_sources"].contains(&k.as_str())) {
            loss(&mut losses, format!("/{}", segment(key)), "the controller list holds controls only: nothing of this");
        }
    }
    loss(&mut losses, "/_sources", "the controller resources have no place for a credit or a licence: the file carries none");

    let mut maps = Maps::default();
    let mut list = Vec::new();
    if let Some(b) = doc.get("cc_params").and_then(Value::as_object) {
        let block_channel = b.get("channel").and_then(Value::as_i64);
        for key in b.keys().filter(|k| *k != "entries" && *k != "channel") {
            loss(&mut losses, format!("/cc_params/{}", segment(key)), "the controller list has no property for it");
        }
        for (n, e) in b.get("entries").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let Some(e) = e.as_object() else { continue };
            let ptr = format!("/cc_params/entries/{n}");
            let role = role_paths.iter().find(|(p, _)| *p == format!("{ptr}/role")).map(|(_, r)| r.as_str());
            if let Some(v) = export_entry(e, &ptr, block_channel, role, &mut maps, &mut losses) {
                list.push(v);
            }
        }
    }
    // A SysEx param has no ctrlType in the standard (nine types, M2-117-UM v1.0): none is written.
    if let Some(b) = doc.get("params").and_then(Value::as_object) {
        let count = b.get("entries").and_then(Value::as_array).map_or(0, Vec::len);
        loss(&mut losses, "/params", format!("the standard has no ctrlType for a setting set by SysEx: the {count} entries of the block are not written"));
    }
    let out = json!({"ChCtrlList": list, "CtrlMapList": maps.to_json()});
    let mut text = serde_json::to_string_pretty(&out).expect("a list is JSON");
    text.push('\n');
    Ok(Translated { value: text, lost: losses })
}
