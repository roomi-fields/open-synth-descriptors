//! The Electra One preset (JSON, versions 2 and 3): its importer and its exporter. The rules
//! are those of the maker's page "Preset format description"
//! (docs.electra.one/developers/presetformat.html, documentation 5.0.0); the fate of each kind
//! of message and the numbers the export lays out by are in docs/electra-one.md.

use crate::importer::{check_source, read_back, slug, unique, way_claims};
use crate::losses::{segment, Became, Control};
use crate::{lose_what_no_way_says, Contract, Fault, FaultKind, Loss, Translated};
use osd::{Address, Algorithm, Arg, ArgType, CcEntry, CcParams, Command, Device, DeviceInfo, Encoding, FramePart, FrameToken, Resolved, Source, Version, Way};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTRACT: Contract = Contract {
    name: "electra-one",
    media_type: "application/json",
    extension: "epr",
    import,
    export,
    import_into: None,
};

/// Every kind of message the maker documents (page, "type" of a message).
const KINDS: [&str; 18] = [
    "cc7", "cc14", "nrpn", "rpn", "sysex", "note", "program", "start", "stop", "tune", "atpoly", "atchannel", "pitchbend", "spp",
    "relcc", "macro", "virtual", "none",
];

/// The numbers the export lays out by (page, "Size limits" and "Model differences").
const PAGES: usize = 12;
const CONTROL_SETS: usize = 3;
const POTS: usize = 12;
const CONTROLS: usize = 432;
const SLOTS_PER_PAGE: usize = CONTROL_SETS * POTS;
const COLUMNS: usize = 6;

fn fault(kind: FaultKind, place: impl Into<String>, rule: impl Into<String>) -> Fault {
    Fault { kind, place: place.into(), rule: rule.into() }
}

fn invalid(place: impl Into<String>, rule: impl Into<String>) -> Fault {
    fault(FaultKind::InvalidFile, place, rule)
}

fn loss(losses: &mut Vec<Loss>, place: impl Into<String>, reason: impl Into<String>) {
    losses.push(Loss { place: place.into(), reason: reason.into() });
}

// ---- import -----------------------------------------------------------------------------

/// Why a kind of message is not carried.
fn why_lost(kind: &str) -> &'static str {
    match kind {
        "note" => "a note is played, the descriptor has no field for sending one from a control",
        "program" => "a program change is a recall, which `programs` describes, not a control",
        "start" | "stop" | "tune" => "a realtime message is no control of the descriptor",
        "atpoly" | "atchannel" | "pitchbend" => "the descriptor has no control bound to this channel message",
        "spp" => "a song position is no control of the descriptor",
        "macro" => "a macro holds a parameter of the performance, which the descriptor has no field for",
        "virtual" | "none" => "the value sends nothing: no MIDI binding to describe",
        _ => "the descriptor has no field for it",
    }
}

/// The `min` and `max` of a value, `at` being its pointer: whole numbers.
fn bounds_of(value: &Map<String, Value>, at: &str) -> Result<(Option<i64>, Option<i64>), Fault> {
    let bound = |key: &str| match value.get(key) {
        None => Ok(None),
        Some(b) => b.as_i64().map(Some).ok_or_else(|| invalid(format!("{at}/{key}"), "a bound is a whole number")),
    };
    Ok((bound("min")?, bound("max")?))
}

/// A bound of a range and the pointer it is read at.
type Bound = (i64, String);

/// A range needs both bounds: a lone one gives none and is a loss.
fn both(low: Option<Bound>, high: Option<Bound>, losses: &mut Vec<Loss>) -> Option<(Bound, Bound)> {
    match (low, high) {
        (Some(low), Some(high)) => Some((low, high)),
        (None, None) => None,
        (Some((_, at)), None) => {
            loss(losses, at, "a range needs both bounds: the maximum is missing");
            None
        }
        (None, Some((_, at))) => {
            loss(losses, at, "a range needs both bounds: the minimum is missing");
            None
        }
    }
}

/// The MIDI range of a message (page, "Message", `min`, `max`, `onValue` and `offValue`): on a
/// pad, `offValue` and `onValue` when it gives them; else each bound is the message's `min` or
/// `max`, else the value's. A range equal to `scale`, what the message carries, is none when the
/// value gives no bound. `at` is the pointer of the value; each bound that is carried is noted in
/// `read`, and any other is named in the losses, here or as a key the import does not read.
fn message_range(
    message: &Map<String, Value>,
    value: &Map<String, Value>,
    at: &str,
    pad: bool,
    scale: [i64; 2],
    read: &mut BTreeSet<String>,
    losses: &mut Vec<Loss>,
) -> Result<Option<[i64; 2]>, Fault> {
    let of_message = format!("{at}/message");
    let (value_min, value_max) = bounds_of(value, at)?;
    let (min, max) = bounds_of(message, &of_message)?;
    let bound = |n: Option<i64>, at: &str, key: &str| n.map(|n| (n, format!("{at}/{key}")));
    let own = [bound(min, &of_message, "min"), bound(max, &of_message, "max")];
    let shown = [bound(value_min, at, "min"), bound(value_max, at, "max")];
    let pressed = ["offValue", "onValue"].map(|key| bound(message.get(key).and_then(Value::as_i64), &of_message, key));
    let chosen = if pad && ["offValue", "onValue"].iter().any(|k| message.contains_key(*k)) {
        match pressed {
            [Some(off), Some(on)] if off.0 < on.0 => Some((off, on)),
            _ => {
                for key in ["offValue", "onValue"].iter().filter(|k| message.contains_key(**k)) {
                    loss(losses, format!("{of_message}/{key}"), "a pad gives a range with both its values, the off value below the on value");
                }
                lose_bounds(value, at, "the pad gives no range: no range takes the bound", losses);
                return Ok(None);
            }
        }
    } else {
        let [low, high] = [0, 1].map(|side| own[side].clone().or_else(|| shown[side].clone()));
        let chosen = both(low, high, losses);
        if chosen.is_none() {
            // A bound of the value beside the lone one of the message.
            for (side, key) in ["min", "max"].iter().enumerate() {
                if own[side].is_some() && shown[side].is_some() {
                    loss(losses, format!("{at}/{key}"), "no range: no range takes the bound");
                }
            }
        }
        chosen
    };
    let Some((low, high)) = chosen else { return Ok(None) };
    let range = [low.0, high.0];
    read.extend([low.1, high.1]);
    for (side, bounds) in [&own, &shown].into_iter().flat_map(|b| b.iter().enumerate()) {
        match bounds {
            Some((n, at)) if *n == range[side] => {
                read.insert(at.clone());
            }
            Some((_, at)) => loss(losses, at.clone(), format!("the range carried is {}..{}, the MIDI range of the message: this bound differs from it", range[0], range[1])),
            None => {}
        }
    }
    let kept = range != scale || shown.iter().any(Option::is_some);
    if !kept && pad {
        // No range says the off and on values, and nothing says the control is a pad: what takes
        // a range is written back as a fader, which has neither.
        for key in ["offValue", "onValue"].iter().filter(|k| message.contains_key(**k)) {
            loss(losses, format!("{of_message}/{key}"), "the pad gives the whole scale of its message: no range is written, nor that the control is a pad");
        }
    }
    Ok(kept.then_some(range))
}

/// Names each bound a value holds when nothing written takes it.
fn lose_bounds(value: &Map<String, Value>, at: &str, reason: &str, losses: &mut Vec<Loss>) {
    for key in ["min", "max"].into_iter().filter(|k| value.contains_key(*k)) {
        loss(losses, format!("{at}/{key}"), reason);
    }
}

fn pointer_number(message: &Map<String, Value>, place: &str, max: u64) -> Result<u64, Fault> {
    match message.get("parameterNumber") {
        None => Ok(0),
        Some(v) => match v.as_u64() {
            Some(n) if n <= max => Ok(n),
            _ => Err(invalid(format!("{place}/parameterNumber"), format!("the parameter number is a whole number from 0 to {max} for this type"))),
        },
    }
}

/// One byte of a template: a number, or a string read as hexadecimal.
fn template_byte(v: &Value, place: &str) -> Result<Option<u8>, Fault> {
    match v {
        Value::Number(n) => match n.as_u64() {
            Some(b) if b <= 255 => Ok(Some(b as u8)),
            _ => Err(invalid(place, "a byte is a number from 0 to 255")),
        },
        Value::String(s) => u8::from_str_radix(s, 16).map(Some).map_err(|_| invalid(place, "a byte written as a string is hexadecimal")),
        Value::Object(_) => Ok(None),
        _ => Err(invalid(place, "an element of a SysEx template is a number, a string or an object")),
    }
}

/// What one element of a SysEx template becomes.
enum Element {
    Byte(u8),
    /// A part of the frame, with the keys of the element it is read from.
    Part(FramePart),
    /// No form in OSD: why, naming the control.
    Lost(String),
}

/// The bit of the value an element of the template starts at, when it is a `value`.
fn value_bit_position(e: &Value) -> Option<u64> {
    let o = e.as_object()?;
    if o.get("type").and_then(Value::as_str) != Some("value") {
        return None;
    }
    match o.get("rules").and_then(Value::as_array).and_then(|r| r.first()) {
        Some(r) => Some(r.get("parameterBitPosition").and_then(Value::as_u64).unwrap_or(0)),
        None => Some(0),
    }
}

/// A whole number from a number or a text of digits.
fn whole(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Notes in `read` the keys of a template element, at `at`, that its part of the frame takes.
fn read_element(element: &Value, at: &str, read: &mut BTreeSet<String>) {
    let keys: &[&str] = match element.get("type").and_then(Value::as_str) {
        Some("checksum") => &["type", "algorithm", "start", "length"],
        Some("value") => &["type", "rules", "rules/0/parameterBitPosition", "rules/0/byteBitPosition", "rules/0/bitWidth"],
        _ => &[],
    };
    read.extend(keys.iter().map(|k| format!("{at}/{k}")));
}

/// The template a message names by `id` (page, "Device", `messages`): its pointer and itself,
/// among the messages of the device the message's `deviceId` names.
fn named_template<'a>(root: &'a Map<String, Value>, device: Option<&Value>, id: &Value) -> Option<(String, &'a Map<String, Value>)> {
    let devices = root.get("devices")?.as_array()?;
    let (d, held) = devices.iter().enumerate().find(|(_, d)| device.is_some() && d.get("id") == device)?;
    let messages = held.get("messages")?.as_array()?;
    let (m, template) = messages.iter().enumerate().find(|(_, m)| m.get("id") == Some(id))?;
    Some((format!("/devices/{d}/messages/{m}"), template.as_object()?))
}

/// One element of a template (page, "SysEx template"): a byte, a string read as hexadecimal, or
/// an object by its `type`. A `function` is only named, never read.
fn template_element(v: &Value, place: &str, wide: bool, control: &str) -> Result<Element, Fault> {
    let Value::Object(o) = v else {
        return Ok(match template_byte(v, place)? {
            Some(b) => Element::Byte(b),
            None => unreachable!("a number or a string is a byte"),
        });
    };
    let lost = |why: String| Ok(Element::Lost(format!("the control \"{control}\": {why}")));
    match o.get("type").and_then(Value::as_str) {
        Some("checksum") => {
            let algorithm = match o.get("algorithm") {
                None => Algorithm::Twos7,
                Some(Value::String(a)) => match a.as_str() {
                    "roland" => Algorithm::Twos7,
                    "waldorf" => Algorithm::Sum7,
                    "fractal" => Algorithm::Xor7,
                    other => return lost(format!("the checksum algorithm \"{other}\" is not one OSD writes (roland, waldorf, fractal)")),
                },
                Some(other) => return lost(format!("the checksum algorithm {other} is not one OSD writes (roland, waldorf, fractal)")),
            };
            let from = match o.get("start") {
                None => Some(0),
                Some(s) => whole(s).filter(|n| *n <= 65535),
            };
            let Some(from) = from else {
                return lost(format!("the checksum start {} is not a whole number", o["start"]));
            };
            // The page: `length`, default 0, "How many bytes to sum". OSD writes no checksum of
            // no byte, and a frame without `length` sums every byte from `from`.
            let length = match o.get("length") {
                None => return lost("a checksum without `length` sums no byte (the default of the page is 0), which OSD cannot write: it is treated as `length: 0`".to_string()),
                Some(l) => match whole(l).filter(|n| (1..=65535).contains(n)) {
                    Some(n) => Some(n as u16),
                    None => return lost(format!("the checksum length {l} is not a whole number of bytes")),
                },
            };
            Ok(Element::Part(FramePart::Checksum { algorithm, from: from as u16, length }))
        }
        Some("value") => {
            let rules = o.get("rules").and_then(Value::as_array).cloned().unwrap_or_default();
            let rule = match rules.as_slice() {
                [] => Map::new(),
                [Value::Object(r)] => r.clone(),
                _ => return lost("a byte that packs several parameters of the value has no form in OSD".to_string()),
            };
            if let Some(key) = rule.keys().find(|k| !["parameterBitPosition", "byteBitPosition", "bitWidth"].contains(&k.as_str())) {
                return lost(format!("the value rule `{key}` (another parameter than the value) has no form in OSD"));
            }
            let get = |k: &str, d: u64| rule.get(k).map_or(Some(d), Value::as_u64);
            let (Some(pp), Some(bp), Some(width)) = (get("parameterBitPosition", 0), get("byteBitPosition", 0), get("bitWidth", 7)) else {
                return lost("a value rule is not a whole number".to_string());
            };
            if bp != 0 || width != 7 || !(pp == 0 || pp == 7) {
                return lost(format!("the value byte takes {width} bits from bit {pp} to bit {bp}: OSD writes whole 7-bit bytes"));
            }
            let encoding = match (wide, pp) {
                (false, _) => None,
                (true, 7) => Some(Encoding::U14Msb),
                (true, _) => Some(Encoding::U14Lsb),
            };
            Ok(Element::Part(FramePart::Arg { name: "value".to_string(), encoding, add: None }))
        }
        Some("parameter") => lost("a byte built from the parameter number (`parameter`) has no form in OSD".to_string()),
        Some("function") => {
            let name = o.get("name").or_else(|| o.get("function")).and_then(Value::as_str).unwrap_or("(unnamed)");
            lost(format!("a byte built by the Lua function `{name}` has no form in OSD; the function is not read"))
        }
        Some(other) => lost(format!("the template element of type `{other}` has no form in OSD")),
        None => lost("a template element without a type has no form in OSD".to_string()),
    }
}

/// The SHA-256 of `data` as lower-case hexadecimal.
fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

/// Why the import does not read the key at `pointer`.
fn why_unread(pointer: &str) -> &'static str {
    let mut path = pointer.split('/').skip(1);
    let (block, key) = (path.next().unwrap_or_default(), pointer.rsplit('/').next().unwrap_or_default());
    let at = |part: &str| pointer.contains(part);
    match block {
        "pages" => "the pages are the look of the preset",
        "devices" => "the ports, channels and devices of the preset are the setup of the controller",
        "groups" if key == "name" => "no control or command carried is in the group: the descriptor has no section for its name",
        "groups" => "the look of the group on the controller",
        "controls" if key == "deviceId" => "the device a message is sent to is the setting of the controller",
        "controls" if key == "name" => "no control or command is carried from this control: nothing takes its name",
        "controls" if key == "id" => "the id of a control or of a value of the preset: the descriptor has no field for it",
        "controls" if at("/message/") && ["min", "max", "onValue", "offValue"].contains(&key) => "no range takes this value of the message",
        "controls" if at("/message/") => "how the controller encodes or sends the value: the descriptor has no field for it",
        "controls" if at("/values/") => "how the value is shown on the controller",
        "controls" => "the look or the behaviour of the control on the controller: the descriptor says what a device speaks, not a screen",
        _ => "the descriptor has no field for it",
    }
}

/// Names each key of the preset, at any depth, that is not in `read` and that no loss holds.
fn name_unread(value: &Value, at: &str, read: &BTreeSet<String>, held: &BTreeSet<String>, losses: &mut Vec<Loss>) {
    let items: Vec<(String, &Value)> = match value {
        Value::Object(o) => o.iter().map(|(k, v)| (format!("{at}/{}", segment(k)), v)).collect(),
        Value::Array(a) => a.iter().enumerate().map(|(i, v)| (format!("{at}/{i}"), v)).collect(),
        _ => return,
    };
    for (pointer, item) in items {
        let under = format!("{pointer}/");
        let read_under = || read.range(under.clone()..).next().is_some_and(|r| r.starts_with(&under));
        // A block a loss holds is named already, unless another control reads under it.
        if held.contains(&pointer) && !read_under() {
            continue;
        }
        if value.is_object() && !read.contains(&pointer) && !held.contains(&pointer) {
            let reason = why_unread(&pointer);
            loss(losses, pointer, reason);
        } else {
            name_unread(item, &pointer, read, held, losses);
        }
    }
}

fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    read_back(read_preset(text, source)?.0)
}

/// The control of the preset each control, and each command, of the device read comes from.
#[derive(Default)]
struct From {
    entries: Vec<usize>,
    commands: Vec<usize>,
}

/// The device a preset gives, before the reader of `osd` judges it, and where it comes from.
fn read_preset(text: &str, source: &Source) -> Result<(Translated<Device>, From), Fault> {
    let document: Value = serde_json::from_str(text).map_err(|e| invalid("", format!("the file is not valid JSON: {e}")))?;
    let document = &document;
    let Some(root) = document.as_object() else {
        return Err(invalid("", "a preset is a JSON object"));
    };
    match root.get("version").and_then(Value::as_i64) {
        Some(2 | 3) => {}
        Some(v) => return Err(invalid("/version", format!("version {v}: the maker documents 2 and 3"))),
        None => return Err(invalid("/version", "a preset has a version, 2 or 3")),
    }
    let Some(controls) = root.get("controls").and_then(Value::as_array) else {
        return Err(invalid("/controls", "a preset has an array of controls"));
    };
    let groups: Vec<(Value, Option<String>)> = root
        .get("groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|g| (g.get("id").cloned().unwrap_or(Value::Null), g.get("name").and_then(Value::as_str).map(str::to_string)))
        .collect();

    let mut losses = Vec::new();
    // The keys the import follows or carries, by pointer: every other one is named at the end.
    let mut read: BTreeSet<String> = ["version", "name", "controls", "groups", "devices", "overlays"].iter().map(|k| format!("/{k}")).collect();
    for n in 0..groups.len() {
        read.insert(format!("/groups/{n}/id"));
    }
    for (d, device) in root.get("devices").and_then(Value::as_array).into_iter().flatten().enumerate() {
        read.insert(format!("/devices/{d}/messages"));
        for m in 0..device.get("messages").and_then(Value::as_array).map_or(0, Vec::len) {
            read.insert(format!("/devices/{d}/messages/{m}/id"));
        }
    }
    let overlays: Vec<Value> = root.get("overlays").and_then(Value::as_array).cloned().unwrap_or_default();
    // The overlays a carried control refers to: their labels are its `values`.
    let mut overlays_read: BTreeSet<usize> = BTreeSet::new();
    // The items, by overlay and index, that the `values` of at least one control hold.
    let mut items_carried: BTreeSet<(usize, usize)> = BTreeSet::new();

    let mut from = From::default();
    let (mut entries, mut commands, mut used) = (Vec::new(), Vec::new(), BTreeSet::new());
    // The messages the controls written claim: no channel is written, `deviceId` being the setup
    // of the controller.
    let mut taken = BTreeSet::new();
    let mut parameter_numbers = Map::new();
    for (n, c) in controls.iter().enumerate() {
        let here = format!("/controls/{n}");
        let Some(c) = c.as_object() else {
            return Err(invalid(here, "a control is a JSON object"));
        };
        read.extend(["groupId", "values"].iter().map(|k| format!("{here}/{k}")));
        let pad = c.get("type").and_then(Value::as_str) == Some("pad");
        let name = c.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let group = match c.get("groupId") {
            None | Some(Value::Null) => None,
            Some(id) => match groups.iter().position(|(g, _)| g == id) {
                Some(g) => Some(g),
                None => return Err(invalid(format!("{here}/groupId"), "no group of the preset has this id")),
            },
        };
        let section = group.and_then(|g| groups[g].1.clone());
        // What a control or a command written takes of its control: the name and the section.
        let named: Vec<String> = std::iter::once(format!("{here}/name")).chain(group.map(|g| format!("/groups/{g}/name"))).collect();
        let Some(values) = c.get("values").and_then(Value::as_array) else {
            loss(&mut losses, format!("{here}/values"), "the control has no value: no message to carry");
            continue;
        };
        for (m, v) in values.iter().enumerate() {
            let at = format!("{here}/values/{m}");
            let Some(v) = v.as_object() else {
                return Err(invalid(at, "a value is a JSON object"));
            };
            let overlay_at = v.get("overlayId").and_then(|id| overlays.iter().position(|o| o.get("id") == Some(id)));
            let Some(message) = v.get("message") else {
                loss(&mut losses, at, "the value has no message");
                continue;
            };
            read.insert(format!("{at}/message"));
            if overlay_at.is_some() {
                read.insert(format!("{at}/overlayId"));
            }
            let value_at = at;
            let at = format!("{value_at}/message");
            let Some(message) = message.as_object() else {
                return Err(invalid(at, "a message is a JSON object"));
            };
            let kind = match message.get("type") {
                None => "sysex",
                Some(Value::String(k)) if KINDS.contains(&k.as_str()) => k.as_str(),
                Some(_) => return Err(invalid(format!("{at}/type"), "the type is one of the documented message types")),
            };
            let title = |name: &str| (!name.is_empty()).then(|| name.to_string());
            match kind {
                "cc7" | "cc14" | "nrpn" | "rpn" | "relcc" => {
                    let max = match kind {
                        "cc7" | "relcc" => 127,
                        "cc14" => 31,
                        _ => 16383,
                    };
                    let number = pointer_number(message, &at, max)?;
                    bounds_of(v, &value_at)?;
                    let split = |n: u64| vec![(n >> 7) as i64, (n & 127) as i64];
                    // A preset does not say the width of an NRPN or RPN value: the full 14 bits.
                    let mut way = match kind {
                        "nrpn" | "rpn" => Way { ctrl_type: kind.into(), ctrl_index: Some(split(number)), value_bits: Some(14), ..Default::default() },
                        "cc14" => Way { ctrl_type: "cc".into(), ctrl_index: Some(vec![number as i64, number as i64 + 32]), value_bits: Some(14), ..Default::default() },
                        _ => Way { ctrl_type: "cc".into(), ctrl_index: Some(vec![number as i64]), value_bits: Some(7), ..Default::default() },
                    };
                    let relative = message.get("relative").and_then(Value::as_bool);
                    if kind == "relcc" || relative == Some(true) {
                        (way.transmit, way.recognize) = (Some(osd::Direction::Relative), Some(osd::Direction::Relative));
                    }
                    // A message is claimed once: a control whose message an earlier one holds is
                    // not written.
                    let claims = way_claims(&way, None);
                    if claims.iter().any(|c| taken.contains(c)) {
                        loss(&mut losses, at, "the message is already claimed by an earlier control: this control is not written");
                        lose_bounds(v, &value_at, "the bound of a control that is not written: an earlier one claims its message", &mut losses);
                        continue;
                    }
                    taken.extend(claims);
                    way.range = message_range(message, v, &value_at, pad, way.scale(), &mut read, &mut losses)?;
                    read.extend(["type", "parameterNumber"].iter().map(|k| format!("{at}/{k}")));
                    // A relative CC is relative by its type: its `relative` key says nothing more.
                    if relative.is_some() && (kind != "relcc" || relative == Some(true)) {
                        read.insert(format!("{at}/relative"));
                    }
                    read.extend(named.iter().cloned());
                    let path = {
                        let (s, n) = (slug(section.as_deref().unwrap_or("")), slug(&name));
                        let joined: Vec<&str> = [s.as_str(), n.as_str()].into_iter().filter(|x| !x.is_empty()).collect();
                        if joined.is_empty() {
                            format!("{kind}-{number}")
                        } else {
                            joined.join("/")
                        }
                    };
                    let scale = way.scale();
                    // The literal survives the addition of a field: `..Default::default()` closes it.
                    #[allow(clippy::needless_update)]
                    let mut entry = CcEntry {
                        osc: unique(format!("/{path}"), &mut used),
                        title: title(&name),
                        ways: vec![way],
                        default: None,
                        priority: None,
                        step_count: None,
                        values: None,
                        polarity: None,
                        channel: None,
                        section: section.clone(),
                        expects: None,
                        transform: None,
                        transform_reverse: None,
                        role: None,
                        remarks: Default::default(),
                        ..Default::default()
                    };
                    if let Some(o) = overlay_at {
                        overlays_read.insert(o);
                        read.extend(["id", "items"].iter().map(|k| format!("/overlays/{o}/{k}")));
                        let control = if name.is_empty() { format!("the control at {here}") } else { format!("the control \"{name}\"") };
                        entry.values = overlay_values(&overlays[o], o, scale, &control, &mut items_carried, &mut losses);
                    }
                    entries.push(entry);
                    from.entries.push(n);
                }
                "sysex" => {
                    let mut frame = vec![FrameToken::Byte(0xF0)];
                    let mut args: Option<Arg> = None;
                    let mut carried = true;
                    // The template: written in the message, or named by its id.
                    let mut direction = message.get("direction").map(|d| (d.clone(), format!("{at}/direction")));
                    let template = match message.get("data") {
                        Some(Value::Array(data)) => Ok((format!("{at}/data"), data)),
                        Some(id @ Value::Number(_)) => match named_template(root, message.get("deviceId"), id) {
                            Some((held, template)) => {
                                let own = template.get("direction").map(|d| (d.clone(), format!("{held}/direction")));
                                direction = direction.or(own);
                                match template.get("data") {
                                    Some(Value::Array(data)) => Ok((format!("{held}/data"), data)),
                                    _ => Err(format!("the template {id} of the device has no list of data: nothing to send")),
                                }
                            }
                            None => Err(format!("no template of id {id} is in the messages of the device the message names")),
                        },
                        Some(_) => Err("the data is neither a template nor the id of one".to_string()),
                        None => Err("the message has no data: nothing to send".to_string()),
                    };
                    let sent = direction.as_ref().is_none_or(|(d, _)| d != "in");
                    match &template {
                        _ if !sent => {
                            carried = false;
                            loss(&mut losses, &at, "the template is received only (`direction` is `in`): it is never sent; it is lost in the round trip");
                        }
                        Ok((held, data)) => {
                            let wide = data.iter().any(|e| value_bit_position(e) == Some(7));
                            let mut elements = BTreeSet::new();
                            for (i, e) in data.iter().enumerate() {
                                match template_element(e, &format!("{held}/{i}"), wide, &name)? {
                                    Element::Byte(b) => {
                                        // The maker drops F0 and F7 wherever they appear and frames the message.
                                        if b != 0xF0 && b != 0xF7 {
                                            frame.push(FrameToken::Byte(b));
                                        }
                                    }
                                    Element::Part(part) => {
                                        if matches!(part, FramePart::Arg { .. }) {
                                            args = Some(Arg {
                                                name: "value".to_string(),
                                                ty: if wide { ArgType::U14 } else { ArgType::U7 },
                                                range: None,
                                                values: None,
                                                default: None,
                                                remarks: Default::default(),
                                            });
                                        }
                                        read_element(e, &format!("{held}/{i}"), &mut elements);
                                        frame.push(FrameToken::Part(part));
                                    }
                                    Element::Lost(reason) => {
                                        carried = false;
                                        loss(&mut losses, &at, format!("{reason}; it is lost in the round trip"));
                                    }
                                }
                            }
                            if carried {
                                read.append(&mut elements);
                            }
                        }
                        Err(why) => {
                            carried = false;
                            loss(&mut losses, &at, format!("the control \"{name}\": {why}; it is lost in the round trip"));
                        }
                    }
                    if !carried {
                        lose_bounds(v, &value_at, "the bound of a message that is not carried: no command takes it", &mut losses);
                        continue;
                    }
                    match args.as_mut() {
                        Some(arg) => {
                            let scale = [0, if arg.ty == ArgType::U14 { 16383 } else { 127 }];
                            arg.range = message_range(message, v, &value_at, pad, scale, &mut read, &mut losses)?;
                        }
                        None => lose_bounds(v, &value_at, "the template sends no value: the command has no argument to take the bound", &mut losses),
                    }
                    read.extend(["type", "data", "direction"].iter().map(|k| format!("{at}/{k}")));
                    if message.get("parameterNumber").is_some_and(Value::is_u64) {
                        read.insert(format!("{at}/parameterNumber"));
                    }
                    // The template and the direction that decided it is sent, where a device holds them.
                    read.extend(template.ok().map(|(held, _)| held));
                    read.extend(direction.map(|(_, at)| at));
                    read.extend(named.iter().cloned());
                    frame.push(FrameToken::Byte(0xF7));
                    let path = {
                        let (s, n) = (slug(section.as_deref().unwrap_or("")), slug(&name));
                        let joined: Vec<&str> = [s.as_str(), n.as_str()].into_iter().filter(|x| !x.is_empty()).collect();
                        if joined.is_empty() {
                            "sysex".to_string()
                        } else {
                            joined.join("/")
                        }
                    };
                    let osc = unique(format!("/{path}"), &mut used);
                    if let Some(n) = message.get("parameterNumber").and_then(Value::as_u64) {
                        parameter_numbers.insert(osc.clone(), json!(n));
                    }
                    // The literal survives the addition of a field: `..Default::default()` closes it.
                    #[allow(clippy::needless_update)]
                    let command = Command {
                        osc: Address::Text(osc),
                        title: title(&name),
                        section: section.clone(),
                        notes: None,
                        args: args.map(|a| vec![a]),
                        pre: None,
                        frame: Some(frame),
                        forward: None,
                        script: None,
                        remarks: Default::default(),
                        ..Default::default()
                    };
                    commands.push(command);
                    from.commands.push(n);
                }
                _ => {
                    loss(
                        &mut losses,
                        at,
                        format!("the {kind} message is not carried: {}; it is lost in the round trip", why_lost(kind)),
                    );
                    lose_bounds(v, &value_at, "the bound of a message that is not carried: nothing takes it", &mut losses);
                }
            }
        }
    }

    for (n, _) in overlays.iter().enumerate().filter(|(n, _)| !overlays_read.contains(n)) {
        loss(&mut losses, format!("/overlays/{n}"), "the list of labels is the look of the preset: no control of the descriptor uses it");
    }
    read.extend(items_carried.iter().flat_map(|(o, i)| ["value", "label"].map(|k| format!("/overlays/{o}/items/{i}/{k}"))));
    let held: BTreeSet<String> = losses.iter().map(|l| l.place.clone()).collect();
    name_unread(document, "", &read, &held, &mut losses);
    let name = root.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or("Electra One preset").to_string();
    let prefix = match slug(&name) {
        s if s.is_empty() => "/electra-one-preset".to_string(),
        s => format!("/{s}"),
    };
    // The caller's source (credit, revision) carries the hash of the text received and the
    // parameter number of each command.
    let mut source = source.clone();
    source.extra.insert("sha256".to_string(), Value::String(sha256_hex(text.as_bytes())));
    if !parameter_numbers.is_empty() {
        source.extra.insert("parameter_numbers".to_string(), Value::Object(parameter_numbers));
    }
    let device = Device {
        schema: Some(Version::V0_7),
        device: DeviceInfo {
            name,
            vendor: None,
            revision: None,
            osc_prefix: prefix,
            device_id: None,
            rate_limit_hz: None,
            kind: None,
            transport: None,
            stands_for: None,
            fidelity: None,
            remarks: Default::default(),
        },
        capabilities: None,
        midi_of: None,
        contexts: None,
        loads_sounds_of: None,
        sound_transfers: None,
        resources: None,
        identity: None,
        programs: None,
        sysex: None,
        cc_params: (!entries.is_empty()).then(|| CcParams { channel: None, entries, remarks: Default::default() }),
        params: None,
        commands: (!commands.is_empty()).then_some(commands),
        triggers: None,
        midi_out: None,
        midi_in: None,
        replies: None,
        subscriptions: None,
        custom_commands: None,
        sources: Some(vec![source]),
        midi_of_sources: None,
        coverage: None,
        role_vocabulary: None,
        remarks: Default::default(),
    };
    Ok((Translated { value: device, lost: losses }, from))
}

/// The named values of a control from the overlay of index `o`, in its order: each item with a
/// text label and a whole value inside `scale`, the scale of the way of the control, bounds
/// included, and noted in `carried`. Any other item is a loss that names the control; `None`
/// when no value remains.
fn overlay_values(overlay: &Value, o: usize, [low, high]: [i64; 2], control: &str, carried: &mut BTreeSet<(usize, usize)>, losses: &mut Vec<Loss>) -> Option<osd::NamedValues> {
    let items = match overlay.get("items") {
        None => return None,
        Some(Value::Array(items)) => items,
        Some(_) => {
            loss(losses, format!("/overlays/{o}/items"), format!("{control}: the items of the overlay are no list"));
            return None;
        }
    };
    let mut values = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let reason = match (item.get("label").and_then(Value::as_str), item.get("value").and_then(Value::as_i64)) {
            (None, _) => "the item has no text label".to_string(),
            (_, None) => "the item has no whole value".to_string(),
            (_, Some(value)) if value < low || value > high => format!("the value {value} is outside the scale {low}..{high}"),
            (Some(title), Some(value)) => {
                values.push(osd::NamedValue { value, title: title.to_string(), remarks: Default::default() });
                carried.insert((o, i));
                continue;
            }
        };
        loss(losses, format!("/overlays/{o}/items/{i}"), format!("{control}: {reason}"));
    }
    (!values.is_empty()).then(|| osd::NamedValues(values))
}

// ---- export -----------------------------------------------------------------------------

enum Message {
    Cc7(u8),
    Cc14(u8),
    Relcc(u8),
    Nrpn(u16),
    Rpn(u16),
    /// A template: its bytes and its elements.
    Sysex(Vec<Value>),
}

/// The longest `name` of a control, of a group, of a preset and of an overlay label (page,
/// "Preset format description": the `maxLength` of each `name` and of an item's `label`).
const CONTROL_NAME_MAX: usize = 40;
const GROUP_NAME_MAX: usize = 40;
const PRESET_NAME_MAX: usize = 20;
const LABEL_MAX: usize = 20;

/// What a control is on the screen of the controller.
enum Look {
    Fader,
    Pad,
    /// A list: its positions and their labels, in the order of the values.
    List(Vec<(i64, String)>),
}

/// Where the descriptor has what a control of the preset is written from.
#[derive(Clone, Copy)]
enum Origin {
    /// The control of this index in `cc_params.entries`, and the way written.
    Entry(usize, usize),
    Command(usize),
}

impl Origin {
    fn pointer(&self) -> String {
        match self {
            Origin::Entry(n, _) => format!("/cc_params/entries/{n}"),
            Origin::Command(n) => format!("/commands/{n}"),
        }
    }

    /// Whether both are of one list, this one first in it.
    fn comes_before(&self, other: &Origin) -> bool {
        match (self, other) {
            (Origin::Entry(a, _), Origin::Entry(b, _)) | (Origin::Command(a), Origin::Command(b)) => a < b,
            _ => false,
        }
    }
}

/// What came back of what each control of the preset was written from: `items` in the order
/// written, `from` the control of the preset each control and command read comes from.
fn became(items: &[Item], from: &From) -> Became {
    let mut out = Became::default();
    for (back, &control) in from.entries.iter().enumerate() {
        if let Origin::Entry(given, way) = items[control].source {
            out.controls.push(Control { given, back, ways: vec![(way, 0)] });
        }
    }
    for (back, &control) in from.commands.iter().enumerate() {
        if let Origin::Command(given) = items[control].source {
            out.commands.push((given, back));
        }
    }
    out
}

/// The device with the length of each checksum written out: one without it covers every byte from
/// its start up to itself, which is the length the preset writes and the import reads.
fn with_checksum_lengths(d: &Device) -> Device {
    let mut d = d.clone();
    for frame in d.commands.iter_mut().flatten().filter_map(|c| c.frame.as_mut()) {
        for (position, token) in frame.iter_mut().enumerate() {
            if let FrameToken::Part(FramePart::Checksum { from, length: length @ None, .. }) = token {
                *length = position.checked_sub(usize::from(*from)).and_then(|n| u16::try_from(n).ok()).filter(|n| *n > 0);
            }
        }
    }
    d
}

/// A control to place: what it says, where the descriptor has it.
struct Item {
    source: Origin,
    name: String,
    section: Option<String>,
    /// The range the descriptor gives: the bounds of the value.
    range: Option<[i64; 2]>,
    /// What the message carries, its own bounds: the range, else the scale of the way or of the
    /// argument; none for a command without argument.
    scale: Option<[i64; 2]>,
    /// Whether the way written sends relative values.
    relative: bool,
    message: Message,
    look: Look,
}

/// The first `max` characters of a text.
fn cut(s: &str, max: usize) -> String {
    s.chars().take(max).collect::<String>().trim_end().to_string()
}

/// A text within `max` characters that is not in `taken`: cut, then numbered from 2 when taken.
fn unique_cut(base: &str, max: usize, taken: &mut BTreeSet<String>) -> String {
    let mut candidate = cut(base, max);
    let mut k = 2;
    while taken.contains(&candidate) {
        let suffix = format!(" {k}");
        candidate = format!("{}{suffix}", cut(base, max - suffix.chars().count()));
        k += 1;
    }
    taken.insert(candidate.clone());
    candidate
}

/// A word of an address or of a role as a person reads it: `cutoff-freq` → `Cutoff freq`.
fn readable(word: &str) -> String {
    let spaced: Vec<&str> = word.split(['_', '-', ' ']).filter(|w| !w.is_empty()).collect();
    let text = spaced.join(" ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The last segment of an address, made readable.
fn last_segment(address: &str) -> String {
    readable(address.rsplit('/').find(|s| !s.is_empty()).unwrap_or(""))
}

/// The type of control the descriptor's own words ask for (page, control `type`: fader, list, pad).
fn look_of(e: &CcEntry, way: &Way) -> Look {
    let range = way.range.filter(|[lo, hi]| lo <= hi);
    if let Some(values) = e.values.as_ref().filter(|v| !v.is_empty()) {
        // The labels are the named values as the way written carries them.
        let mut items: Vec<(i64, String)> = values.iter().map(|v| (e.in_way(v.value, way), v.title.clone())).collect();
        items.sort_by_key(|(n, _)| *n);
        return Look::List(items);
    }
    match e.expects.as_deref() {
        Some("switch" | "momentary" | "trigger") => Look::Pad,
        Some("discrete") => match range {
            Some([lo, hi]) if hi - lo == 1 => Look::Pad,
            Some([lo, hi]) if hi - lo >= 2 => Look::List((lo..=hi).map(|n| (n, n.to_string())).collect()),
            _ => Look::Fader,
        },
        _ => Look::Fader,
    }
}

fn address_text(a: &Address) -> String {
    match a {
        Address::Text(s) => s.clone(),
        Address::Segments(parts) => parts
            .iter()
            .map(|p| match p {
                osd::Segment::Text(t) => t.clone(),
                osd::Segment::Capture(c) => format!("{{{c}}}"),
            })
            .collect(),
    }
}

fn entry_item(n: usize, e: &CcEntry, losses: &mut Vec<Loss>) -> Option<Item> {
    let at = format!("/cc_params/entries/{n}");
    lose_what_no_way_says(e, n, losses);
    let name = e.title.clone()
        .or_else(|| Some(readable(&e.role.as_ref()?.setting)).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| last_segment(&e.osc));
    // The ways in the order the descriptor wrote them: the first the preset can carry is written,
    // with its own range; the export never reorders them, and every other way is a loss at its place.
    let ways = &e.ways;
    let place = |k: usize| format!("{at}/ways/{k}");
    let mut bindings: Vec<Option<(Message, &'static str)>> = ways.iter().map(way_binding).collect();
    let Some(chosen) = bindings.iter().position(Option::is_some) else {
        for (k, w) in ways.iter().enumerate() {
            loss(losses, place(k), format!("the preset has no message for a {} way", w.ctrl_type));
        }
        loss(losses, at, "the control has no complete MIDI binding to write");
        return None;
    };
    let look = look_of(e, &ways[chosen]);
    let (message, written_kind) = bindings[chosen].take().expect("the chosen way has a binding");
    if let Some(note) = way_note(&ways[chosen]) {
        loss(losses, format!("{}/ctrlIndex", place(chosen)), note);
    }
    for (k, b) in bindings.iter().enumerate().filter(|(k, _)| *k != chosen) {
        let reason = match b {
            Some((_, kind)) => format!("a control of the preset sends one message: the {written_kind} way is written, the {kind} way is not"),
            None => format!("the preset has no message for a {} way: the {written_kind} way is written", ways[k].ctrl_type),
        };
        loss(losses, place(k), reason);
    }
    let way = &ways[chosen];
    // A 14-bit pair the preset cannot say is written as its MSB alone, on its own scale.
    let scale = match &message {
        Message::Cc7(_) if way.ctrl_index.as_ref().is_some_and(|i| i.len() == 2) => [0, 127],
        _ => way.scale(),
    };
    let range = way.range.filter(|_| way_note(way).is_none());
    let relative = way.transmit == Some(osd::Direction::Relative);
    Some(Item { source: Origin::Entry(n, chosen), name, section: e.section.clone(), range, scale: Some(scale), relative, message, look })
}

/// The message of a preset a way is, with the kind the preset names it; `None` for a way of a kind
/// the preset has no message for. A 14-bit CC the preset cannot pair (a CC above 31, or an LSB not 32
/// above) is written as its MSB alone, with a loss.
fn way_binding(w: &Way) -> Option<(Message, &'static str)> {
    match (w.ctrl_type.as_str(), w.ctrl_index.as_deref().unwrap_or_default()) {
        ("cc", [cc, lsb]) if *cc <= 31 && *lsb == cc + 32 => Some((Message::Cc14(*cc as u8), "cc14")),
        ("cc", [cc]) if w.transmit == Some(osd::Direction::Relative) => Some((Message::Relcc(*cc as u8), "relcc")),
        ("cc", [cc, _] | [cc]) => Some((Message::Cc7(*cc as u8), "cc7")),
        ("nrpn", [m, l]) => Some((Message::Nrpn((*m as u16) << 7 | *l as u16), "nrpn")),
        ("rpn", [m, l]) => Some((Message::Rpn((*m as u16) << 7 | *l as u16), "rpn")),
        _ => None,
    }
}

/// Why the way written does not say all its numbers.
fn way_note(w: &Way) -> Option<&'static str> {
    match (w.ctrl_type.as_str(), w.ctrl_index.as_deref().unwrap_or_default()) {
        ("cc", [cc, lsb]) if !(*cc <= 31 && *lsb == cc + 32) => Some("a 14-bit CC of the preset pairs a CC from 0 to 31 with the CC 32 above it"),
        _ => None,
    }
}

/// The value element of a template that takes the 7 bits of the value from bit `from`.
fn value_element(from: u8) -> Value {
    json!({"type": "value", "rules": [{"parameterBitPosition": from, "byteBitPosition": 0, "bitWidth": 7}]})
}

/// The template a frame is, with the argument `value` it sends: fixed bytes between F0 and F7,
/// the argument as a u7, or as a u14 by its two halves, with nothing added, and checksums. `None`
/// for any other frame: the maker's template would not send its bytes.
fn template_of<'a>(c: &'a Command, frame: &[FrameToken]) -> Option<(Vec<Value>, Option<&'a Arg>)> {
    let inner = frame.strip_prefix(&[FrameToken::Byte(0xF0)])?.strip_suffix(&[FrameToken::Byte(0xF7)])?;
    if inner.iter().any(|t| matches!(t, FrameToken::Byte(0xF0 | 0xF7))) {
        return None;
    }
    let arg = c.args.iter().flatten().find(|a| a.name == "value");
    let mut data = Vec::with_capacity(frame.len());
    // The halves of the value the frame sends: its low bits, its high bits.
    let mut halves = [false; 2];
    for (position, token) in frame.iter().enumerate() {
        data.push(match token {
            FrameToken::Byte(b) => json!(b),
            FrameToken::Part(FramePart::Arg { name, encoding, add: None }) if name == "value" => {
                let from = match (arg?.ty, encoding) {
                    (ArgType::U7, None) => 0,
                    (ArgType::U14, Some(Encoding::U14Lsb)) => 0,
                    (ArgType::U14, Some(Encoding::U14Msb)) => 7,
                    _ => return None,
                };
                halves[usize::from(from == 7)] = true;
                value_element(from)
            }
            FrameToken::Part(FramePart::Checksum { algorithm, from, length }) => {
                let algorithm = match algorithm {
                    Algorithm::Twos7 => "roland",
                    Algorithm::Sum7 => "waldorf",
                    Algorithm::Xor7 => "fractal",
                    #[allow(unreachable_patterns)]
                    _ => return None,
                };
                // Without a length, every byte from the start up to the checksum.
                let written = position.checked_sub(usize::from(*from)).filter(|n| *n > 0)?;
                let length = length.map_or(written, |l| written.min(usize::from(l)));
                json!({"type": "checksum", "algorithm": algorithm, "start": from, "length": length})
            }
            _ => return None,
        });
    }
    // A 14-bit value is sent by its two halves.
    let sends_value = halves != [false; 2];
    if sends_value && arg.is_some_and(|a| a.ty == ArgType::U14) && halves != [true; 2] {
        return None;
    }
    Some((data, arg.filter(|_| sends_value)))
}

fn command_item(n: usize, c: &Command, losses: &mut Vec<Loss>) -> Option<Item> {
    let at = format!("/commands/{n}");
    let Some(frame) = &c.frame else {
        loss(losses, at, "the command has no frame: it is no MIDI message to write");
        return None;
    };
    let Some((data, arg)) = template_of(c, frame) else {
        loss(
            losses,
            format!("{at}/frame"),
            "the preset template written holds fixed bytes between F0 and F7, the argument `value` (u7, or u14 as u14_lsb and u14_msb, nothing added) and checksums: this frame is none such",
        );
        for (m, a) in c.args.iter().flatten().enumerate() {
            let fields = [("name", true), ("type", true), ("range", a.range.is_some()), ("values", a.values.is_some()), ("default", a.default.is_some())];
            let held = fields.into_iter().filter_map(|(field, present)| present.then_some(field.to_string()));
            for field in held.chain(a.remarks.keys().map(|k| segment(k))) {
                loss(losses, format!("{at}/args/{m}/{field}"), "the command is not written: the preset holds no part of its argument");
            }
        }
        return None;
    };
    let name = c.title.clone().unwrap_or_else(|| last_segment(&address_text(&c.osc)));
    let range = arg.and_then(|a| a.range);
    let scale = arg.map(|a| a.range.unwrap_or([0, if a.ty == ArgType::U14 { 16383 } else { 127 }]));
    // A pad sends its on and off values only: a command with an argument is a fader.
    let look = if arg.is_some() { Look::Fader } else { Look::Pad };
    Some(Item { source: Origin::Command(n), name, section: c.section.clone(), range, scale, relative: false, message: Message::Sysex(data), look })
}

/// Why the import of the preset written does not give a field back, by its pointer; `added`: the
/// descriptor given does not hold it.
fn why_not_back(pointer: &str, added: bool) -> String {
    let field = pointer.rsplit('/').next().unwrap_or_default();
    let item = |list: &str| pointer.strip_prefix(list).is_some_and(|n| !n.is_empty() && !n.contains('/'));
    let reason = match field {
        _ if item("/cc_params/entries/") || item("/commands/") => "the preset written gives no control, or no command, back as this one at this place",
        "osc_prefix" => "the preset has no field for it: the import builds the prefix from the name of the preset",
        "osc" => "the preset has no field for an address: the import builds it from the group and the name",
        "name" if pointer == "/device/name" => "the preset cuts the name",
        "title" | "section" => "the preset needs a name, cuts it and keeps two names of a group apart: the import reads what is written",
        "value_bits" => "the preset does not say the width of a value: the import reads 7 bits for a CC of one number, 14 for a 14-bit CC, an NRPN or an RPN",
        "range" => "the message range and the value bounds written do not give it back as it is",
        "transmit" | "recognize" => "a message of the preset is relative or not, for both directions",
        "channel" => "the channel of a preset belongs to its devices, which the export does not describe",
        "expects" => "the preset has no field for it: it only chooses a pad or a list",
        _ if added => "the import of the preset adds it: the descriptor given does not hold it",
        _ if pointer.starts_with("/cc_params") || pointer.starts_with("/commands") || pointer.starts_with("/device") => "the preset has no field for it",
        _ => "the preset carries controls and SysEx messages only",
    };
    reason.to_string()
}

/// The message of a control to place (page, "Message").
fn message_json(item: &Item) -> Value {
    let (kind, key, written) = match &item.message {
        Message::Cc7(n) => ("cc7", "parameterNumber", json!(n)),
        Message::Relcc(n) => ("relcc", "parameterNumber", json!(n)),
        Message::Cc14(n) => ("cc14", "parameterNumber", json!(n)),
        Message::Nrpn(n) => ("nrpn", "parameterNumber", json!(n)),
        Message::Rpn(n) => ("rpn", "parameterNumber", json!(n)),
        Message::Sysex(data) => ("sysex", "data", json!(data)),
    };
    let mut m = json!({"type": kind, "deviceId": 1});
    m[key] = written;
    match item.message {
        Message::Relcc(_) => m["relative"] = json!("twosComplement"),
        _ if item.relative => m["relative"] = json!(true),
        _ => {}
    }
    if let Some([low, high]) = item.scale {
        m["min"] = json!(low);
        m["max"] = json!(high);
    }
    if matches!(item.look, Look::Pad) {
        match item.scale {
            Some([low, high]) => {
                m["offValue"] = json!(low);
                m["onValue"] = json!(high);
            }
            None => m["onValue"] = json!(127),
        }
    }
    m
}

/// Names each command written whose parameter number a source keeps (`parameter_numbers`, by the
/// address of the command, as the import writes it): the preset written has none for a SysEx
/// message. Like the lost order, these losses stand for no field of the command.
fn lose_parameter_numbers(d: &Device, items: &[Item], losses: &mut Vec<Loss>) {
    for i in items {
        let Origin::Command(n) = &i.source else { continue };
        let Some(command) = d.commands.iter().flatten().nth(*n) else { continue };
        let address = address_text(&command.osc);
        let kept = d.sources.iter().flatten().any(|s| s.extra.get("parameter_numbers").and_then(|p| p.get(&address)).is_some());
        if kept {
            loss(losses, i.source.pointer(), "the parameter number a source keeps for this command (parameter_numbers) is not written in the preset");
        }
    }
}

fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    if d.sources.as_ref().is_none_or(|s| s.is_empty()) {
        return Err(fault(FaultKind::NoSources, "/sources", "a device without sources is not written: its credit and licence cannot be"));
    }
    let mut losses = Vec::new();
    loss(&mut losses, "/sources", "the preset has no field for the credit and the licence of its sources");

    let info = &d.device;
    if d.identity.as_ref().is_some_and(|i| i.manufacturer_id.is_some()) {
        loss(&mut losses, "/identity/manufacturerId", "the preset has no field for it");
    }
    let mut items = Vec::new();
    if let Some(cc) = &d.cc_params {
        for (n, e) in cc.entries.iter().enumerate() {
            items.extend(entry_item(n, e, &mut losses));
        }
    }
    for (n, c) in d.commands.iter().flatten().enumerate() {
        items.extend(command_item(n, c, &mut losses));
    }

    // By section, in the order the descriptor first names each; the descriptor's order within one.
    let mut sections: Vec<Option<String>> = Vec::new();
    for i in &items {
        if !sections.contains(&i.section) {
            sections.push(i.section.clone());
        }
    }
    items.sort_by_key(|i| sections.iter().position(|s| *s == i.section));
    // Slots: 12 pots, 3 control sets, then the next page; a control 120 wide on a grid of 6. A
    // group is a rectangle of whole cells: one that spreads over two rows or more begins on a new
    // row, and the next group begins on a new row after it.
    let mut slots: Vec<usize> = Vec::new();
    let mut chunks: Vec<(usize, usize)> = Vec::new();
    let mut cursor = 0usize;
    let mut overflow = items.len();
    let mut from = 0;
    'runs: while from < items.len() {
        let mut to = from;
        while to < items.len() && items[to].section == items[from].section {
            to += 1;
        }
        let mut k = from;
        while k < to {
            let (col, left) = (cursor % COLUMNS, to - k);
            if col != 0 && left > COLUMNS - col {
                cursor += COLUMNS - col;
            }
            if cursor >= CONTROLS {
                overflow = k;
                break 'runs;
            }
            let take = left.min(SLOTS_PER_PAGE - cursor % SLOTS_PER_PAGE);
            slots.extend(cursor..cursor + take);
            chunks.push((k, k + take));
            let last = cursor + take - 1;
            cursor = if (cursor % SLOTS_PER_PAGE) / COLUMNS == (last % SLOTS_PER_PAGE) / COLUMNS { cursor + take } else { (last / COLUMNS + 1) * COLUMNS };
            k += take;
        }
        from = to;
    }
    for i in items.drain(overflow..) {
        loss(&mut losses, i.source.pointer(), format!("a preset holds at most {CONTROLS} controls: this one comes after them"));
    }
    // A control written before one the descriptor holds earlier in its list comes back in another
    // order. These losses say the place only: they join the list after the comparison, so that
    // none of them stands for a field the control loses.
    let mut moved = Vec::new();
    for (k, i) in items.iter().enumerate() {
        if items[k + 1..].iter().any(|later| later.source.comes_before(&i.source)) {
            loss(&mut moved, i.source.pointer(), "the preset lays its controls out by group: the import gives this one before one that precedes it");
        }
    }
    let placed = items.len();
    let cell = |slot: usize| ((slot % COLUMNS) as i64 * 130, 24 + ((slot % SLOTS_PER_PAGE) / COLUMNS) as i64 * 70);
    let mut groups: Vec<(Option<String>, usize, [i64; 4])> = Vec::new();
    let mut group_of: Vec<Option<usize>> = vec![None; placed];
    for (a, b) in &chunks {
        if items[*a].section.is_none() {
            continue;
        }
        let mut bounds = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
        for slot in &slots[*a..*b] {
            let (x, y) = cell(*slot);
            bounds = [bounds[0].min(x), bounds[1].min(y), bounds[2].max(x + 120), bounds[3].max(y + 56)];
        }
        groups.push((items[*a].section.clone(), slots[*a] / SLOTS_PER_PAGE, bounds));
        for g in &mut group_of[*a..*b] {
            *g = Some(placed + groups.len());
        }
    }
    let mut controls = Vec::new();
    let mut overlays: Vec<Value> = Vec::new();
    let mut names: BTreeMap<Option<String>, BTreeSet<String>> = BTreeMap::new();
    for (i, item) in items.iter().enumerate() {
        let (page, slot) = (slots[i] / SLOTS_PER_PAGE, slots[i] % SLOTS_PER_PAGE);
        let (set, pot) = (slot / POTS, slot % POTS);
        let (x, y) = cell(slots[i]);
        let group = group_of[i];
        let mut value = Map::new();
        value.insert("id".to_string(), json!("value"));
        if let Some([lo, hi]) = item.range {
            value.insert("min".to_string(), json!(lo));
            value.insert("max".to_string(), json!(hi));
        }
        value.insert("message".to_string(), message_json(item));
        if let Look::List(list) = &item.look {
            let mut taken = BTreeSet::new();
            let items: Vec<Value> = list.iter().map(|(n, label)| json!({"value": n, "label": unique_cut(label, LABEL_MAX, &mut taken)})).collect();
            overlays.push(json!({"id": overlays.len() + 1, "items": items}));
            value.insert("overlayId".to_string(), json!(overlays.len()));
        }
        let name = unique_cut(&item.name, CONTROL_NAME_MAX, names.entry(item.section.clone()).or_default());
        let mut control = Map::new();
        control.insert("id".to_string(), json!(i + 1));
        let kind = match &item.look {
            Look::Fader => "fader",
            Look::Pad => "pad",
            Look::List(_) => "list",
        };
        control.insert("type".to_string(), json!(kind));
        control.insert("name".to_string(), json!(name));
        control.insert("color".to_string(), json!("FFFFFF"));
        control.insert("bounds".to_string(), json!([x, y, 120, 56]));
        control.insert("pageId".to_string(), json!(page + 1));
        control.insert("controlSetId".to_string(), json!(set + 1));
        if let Some(g) = group {
            control.insert("groupId".to_string(), json!(g));
        }
        control.insert("inputs".to_string(), json!([{"potId": pot + 1, "valueId": "value"}]));
        control.insert("values".to_string(), json!([Value::Object(value)]));
        controls.push(Value::Object(control));
    }
    let pages = slots.last().map_or(1, |s| s / SLOTS_PER_PAGE + 1).clamp(1, PAGES);
    let groups: Vec<Value> = groups
        .iter()
        .enumerate()
        .map(|(k, (name, page, b))| {
            let name = name.as_deref().map(|n| cut(n, GROUP_NAME_MAX));
            json!({"id": placed + 1 + k, "pageId": page + 1, "name": name, "bounds": [b[0], b[1], b[2] - b[0], b[3] - b[1]]})
        })
        .collect();
    let preset_name = cut(&info.name, PRESET_NAME_MAX);
    let id = match slug(&info.name) {
        s if s.is_empty() => "osd".to_string(),
        s => s,
    };
    let preset = json!({
        "version": 3,
        "name": preset_name,
        "projectId": id,
        "pages": (1..=pages).map(|p| json!({"id": p, "name": format!("Page {p}")})).collect::<Vec<_>>(),
        "devices": [{"id": 1, "name": preset_name, "port": 1, "channel": 1}],
        "overlays": overlays,
        "groups": groups,
        "controls": controls,
    });
    let mut text = serde_json::to_string_pretty(&preset).expect("a preset is JSON");
    text.push('\n');
    // Every field the preset read back does not give is named.
    let back = read_preset(&text, &Source::default()).ok();
    let became = back.as_ref().map_or_else(Became::default, |(_, from)| became(&items, from));
    let given = with_checksum_lengths(d);
    crate::losses::name_what_does_not_come_back(&given, back.as_ref().map(|(t, _)| &t.value), &became, &mut losses, why_not_back);
    losses.append(&mut moved);
    lose_parameter_numbers(d, &items, &mut losses);
    crate::losses::in_the_order_of(d, &mut losses);
    Ok(Translated { value: text, lost: losses })
}
