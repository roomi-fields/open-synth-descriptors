//! The Electra One preset (JSON, versions 2 and 3): its importer and its exporter. The rules
//! are those of the maker's page "Preset format description"
//! (docs.electra.one/developers/presetformat.html, documentation 5.0.0); the fate of each kind
//! of message and the numbers the export lays out by are in docs/electra-one.md.

use crate::midi_csv::{check_source, slug, unique};
use crate::{Fault, FaultKind, Loss, Translated};
use osd::{Address, Algorithm, Arg, ArgType, CcEntry, CcParams, Command, Device, DeviceInfo, Encoding, FramePart, FrameToken, Resolved, Source, Version};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

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

const LOOK: &str = "the look or the behaviour of the control on the controller: the descriptor says what a device speaks, not a screen";

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

/// Why a kind of message is not carried.
fn why_lost(kind: &str) -> &'static str {
    match kind {
        "note" => "a note is played, the descriptor has no field for sending one from a control",
        "program" => "a program change is a recall, which `programs` describes, not a control",
        "start" | "stop" | "tune" => "a realtime message is no control of the descriptor",
        "atpoly" | "atchannel" | "pitchbend" => "the descriptor has no control bound to this channel message",
        "spp" => "a song position is no control of the descriptor",
        "relcc" => "a relative CC steps a value, the descriptor has no field for steps",
        "macro" => "a macro holds a parameter of the performance, which the descriptor has no field for",
        "virtual" | "none" => "the value sends nothing: no MIDI binding to describe",
        _ => "the descriptor has no field for it",
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
            let length = match o.get("length") {
                None => None,
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

pub(crate) fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let root: Value = serde_json::from_str(text).map_err(|e| invalid("", format!("the file is not valid JSON: {e}")))?;
    let Some(root) = root.as_object() else {
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
    let overlays: Vec<Value> = root.get("overlays").and_then(Value::as_array).cloned().unwrap_or_default();
    // The overlays a carried control refers to: their labels are its `values`.
    let mut overlays_read: BTreeSet<usize> = BTreeSet::new();
    let mut any_values = false;
    for (key, reason) in [
        ("pages", "the pages are the look of the preset"),
        ("devices", "the ports and channels of the preset's devices (and each message's deviceId) are the setup of the controller"),
    ] {
        if root.contains_key(key) {
            loss(&mut losses, format!("/{key}"), reason);
        }
    }
    for key in root.keys().filter(|k| !["version", "name", "controls", "groups", "pages", "devices", "overlays"].contains(&k.as_str())) {
        loss(&mut losses, format!("/{}", segment(key)), "the descriptor has no field for it");
    }
    for (n, g) in root.get("groups").and_then(Value::as_array).into_iter().flatten().enumerate() {
        for key in g.as_object().into_iter().flat_map(|o| o.keys()).filter(|k| !["id", "name"].contains(&k.as_str())) {
            loss(&mut losses, format!("/groups/{n}/{}", segment(key)), "the look of the group on the controller");
        }
    }

    let (mut entries, mut commands, mut used) = (Vec::new(), Vec::new(), BTreeSet::new());
    let mut parameter_numbers = Map::new();
    for (n, c) in controls.iter().enumerate() {
        let here = format!("/controls/{n}");
        let Some(c) = c.as_object() else {
            return Err(invalid(here, "a control is a JSON object"));
        };
        for key in c.keys().filter(|k| !["id", "name", "groupId", "values"].contains(&k.as_str())) {
            loss(&mut losses, format!("{here}/{}", segment(key)), LOOK);
        }
        let name = c.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let section = match c.get("groupId") {
            None | Some(Value::Null) => None,
            Some(id) => match groups.iter().find(|(g, _)| g == id) {
                Some((_, name)) => name.clone(),
                None => return Err(invalid(format!("{here}/groupId"), "no group of the preset has this id")),
            },
        };
        let Some(values) = c.get("values").and_then(Value::as_array) else {
            loss(&mut losses, format!("{here}/values"), "the control has no value: no message to carry");
            continue;
        };
        for (m, v) in values.iter().enumerate() {
            let at = format!("{here}/values/{m}");
            let Some(v) = v.as_object() else {
                return Err(invalid(at, "a value is a JSON object"));
            };
            let carried_kind = matches!(
                v.get("message").and_then(|m| m.get("type")).and_then(Value::as_str),
                Some("cc7" | "cc14" | "nrpn" | "rpn")
            );
            let overlay_at = carried_kind
                .then(|| v.get("overlayId"))
                .flatten()
                .and_then(|id| overlays.iter().position(|o| o.get("id") == Some(id)));
            for key in v.keys().filter(|k| !["id", "min", "max", "message"].contains(&k.as_str())) {
                if key == "overlayId" && overlay_at.is_some() {
                    continue;
                }
                loss(&mut losses, format!("{at}/{}", segment(key)), "how the value is shown on the controller");
            }
            let labels: Option<BTreeMap<String, i64>> = overlay_at.map(|o| {
                overlays_read.insert(o);
                overlays[o]
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|i| Some((i.get("label")?.as_str()?.to_string(), i.get("value")?.as_i64()?)))
                    .collect()
            });
            let Some(message) = v.get("message") else {
                loss(&mut losses, at, "the value has no message");
                continue;
            };
            let at = format!("{at}/message");
            let Some(message) = message.as_object() else {
                return Err(invalid(at, "a message is a JSON object"));
            };
            let kind = match message.get("type") {
                None => "sysex",
                Some(Value::String(k)) if KINDS.contains(&k.as_str()) => k.as_str(),
                Some(_) => return Err(invalid(format!("{at}/type"), "the type is one of the documented message types")),
            };
            let remark = |name: &str| (!name.is_empty()).then(|| Value::String(name.to_string()));
            match kind {
                "cc7" | "cc14" | "nrpn" | "rpn" => {
                    let max = match kind {
                        "cc7" => 127,
                        "cc14" => 31,
                        _ => 16383,
                    };
                    let number = pointer_number(message, &at, max)?;
                    let bound = |key: &str| match v.get(key) {
                        None => Ok(None),
                        Some(b) => b.as_i64().map(Some).ok_or_else(|| invalid(format!("{}/{key}", at.trim_end_matches("/message")), "a bound is a whole number")),
                    };
                    let range = match (bound("min")?, bound("max")?) {
                        (Some(a), Some(b)) => Some([a, b]),
                        (None, None) => None,
                        (Some(_), None) => {
                            loss(&mut losses, at.replace("/message", "/min"), "a range needs both bounds: the maximum is missing");
                            None
                        }
                        (None, Some(_)) => {
                            loss(&mut losses, at.replace("/message", "/max"), "a range needs both bounds: the minimum is missing");
                            None
                        }
                    };
                    for key in message.keys().filter(|k| !["type", "parameterNumber", "deviceId"].contains(&k.as_str())) {
                        loss(&mut losses, format!("{at}/{}", segment(key)), "how the controller encodes the value: the descriptor has no field for it");
                    }
                    let path = {
                        let (s, n) = (slug(section.as_deref().unwrap_or("")), slug(&name));
                        let joined: Vec<&str> = [s.as_str(), n.as_str()].into_iter().filter(|x| !x.is_empty()).collect();
                        if joined.is_empty() {
                            format!("{kind}-{number}")
                        } else {
                            joined.join("/")
                        }
                    };
                    let split = |n: u64| (Some((n >> 7) as u8), Some((n & 127) as u8));
                    let (nrpn, rpn) = match kind {
                        "nrpn" => (split(number), (None, None)),
                        "rpn" => ((None, None), split(number)),
                        _ => ((None, None), (None, None)),
                    };
                    let mut entry = CcEntry {
                        osc: unique(format!("/{path}"), &mut used),
                        cc: matches!(kind, "cc7" | "cc14").then(|| number as u8),
                        cc_lsb: (kind == "cc14").then(|| number as u8 + 32),
                        nrpn_msb: nrpn.0,
                        nrpn_lsb: nrpn.1,
                        nrpn_lsb_range: None,
                        rpn_msb: rpn.0,
                        rpn_lsb: rpn.1,
                        // A preset does not say the width of an NRPN or RPN value: the full 14 bits.
                        value_bits: (kind == "nrpn" || kind == "rpn").then_some(14),
                        range,
                        also: None,
                        default: None,
                        priority: None,
                        step_count: None,
                        transmit: None,
                        recognize: None,
                        values: None,
                        orientation: None,
                        channel: None,
                        section: section.clone(),
                        expects: None,
                        transform: None,
                        transform_reverse: None,
                        role: None,
                        remarks: Default::default(),
                    };
                    if let Some(r) = remark(&name) {
                        entry.remarks.insert("_name".to_string(), r);
                    }
                    entry.values = labels.filter(|l| !l.is_empty());
                    any_values |= entry.values.is_some() || entry.value_bits.is_some();
                    entries.push(entry);
                }
                "sysex" => {
                    let mut frame = vec![FrameToken::Byte(0xF0)];
                    let mut args: Option<Arg> = None;
                    let mut carried = true;
                    match message.get("data") {
                        Some(Value::Array(data)) => {
                            let wide = data.iter().any(|e| value_bit_position(e) == Some(7));
                            for (i, e) in data.iter().enumerate() {
                                match template_element(e, &format!("{at}/data/{i}"), wide, &name)? {
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
                                        frame.push(FrameToken::Part(part));
                                    }
                                    Element::Lost(reason) => {
                                        carried = false;
                                        loss(&mut losses, &at, format!("{reason}; it is lost in the round trip"));
                                    }
                                }
                            }
                        }
                        Some(_) => {
                            carried = false;
                            loss(&mut losses, &at, "the template is named by its id in the device's messages: the descriptor holds the bytes only; it is lost in the round trip");
                        }
                        None => {
                            carried = false;
                            loss(&mut losses, &at, "the message has no data: nothing to send; it is lost in the round trip");
                        }
                    }
                    if !carried {
                        continue;
                    }
                    for key in message.keys().filter(|k| !["type", "data", "deviceId", "parameterNumber"].contains(&k.as_str())) {
                        loss(&mut losses, format!("{at}/{}", segment(key)), "how the controller sends the template: the descriptor has no field for it");
                    }
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
                    let mut command = Command {
                        osc: Address::Text(osc),
                        section: section.clone(),
                        notes: None,
                        args: args.map(|a| vec![a]),
                        pre: None,
                        frame: Some(frame),
                        forward: None,
                        script: None,
                        remarks: Default::default(),
                    };
                    if let Some(r) = remark(&name) {
                        command.remarks.insert("_name".to_string(), r);
                    }
                    commands.push(command);
                }
                _ => loss(
                    &mut losses,
                    at,
                    format!("the {kind} message is not carried: {}; it is lost in the round trip", why_lost(kind)),
                ),
            }
        }
    }

    for (n, _) in overlays.iter().enumerate().filter(|(n, _)| !overlays_read.contains(n)) {
        loss(&mut losses, format!("/overlays/{n}"), "the list of labels is the look of the preset: no control of the descriptor uses it");
    }
    let name = root.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or("Electra One preset").to_string();
    let prefix = match slug(&name) {
        s if s.is_empty() => "/electra-one-preset".to_string(),
        s => format!("/{s}"),
    };
    // The caller's source (credit, revision, date) carries the hash of the text received and the
    // parameter number of each command.
    let mut source = source.clone();
    source.extra.insert("sha256".to_string(), Value::String(sha256_hex(text.as_bytes())));
    if !parameter_numbers.is_empty() {
        source.extra.insert("parameter_numbers".to_string(), Value::Object(parameter_numbers));
    }
    let device = Device {
        schema: Some(if any_values { Version::V0_6 } else { Version::V0_5 }),
        device: DeviceInfo {
            name,
            vendor: None,
            revision: None,
            osc_prefix: prefix,
            manufacturer_id: None,
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
        coverage: None,
        role_vocabulary: None,
        remarks: Default::default(),
    };
    Ok(Translated { value: device, lost: losses })
}

// ---- export -----------------------------------------------------------------------------

enum Message {
    Cc7(u8),
    Cc14(u8),
    Nrpn(u16),
    Rpn(u16),
    Sysex(Vec<u8>),
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

/// A control to place: what it says, where the descriptor has it.
struct Item {
    pointer: String,
    name: String,
    section: Option<String>,
    range: Option<[i64; 2]>,
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
fn look_of(e: &CcEntry) -> (Look, bool) {
    let range = e.range.filter(|[lo, hi]| lo <= hi);
    if let Some(values) = e.values.as_ref().filter(|v| !v.is_empty()) {
        let mut items: Vec<(i64, String)> = values.iter().map(|(label, n)| (*n, label.clone())).collect();
        items.sort_by_key(|(n, _)| *n);
        return (Look::List(items), true);
    }
    match e.expects.as_deref() {
        Some("switch" | "momentary" | "trigger") => (Look::Pad, true),
        Some("discrete") => match range {
            Some([lo, hi]) if hi - lo == 1 => (Look::Pad, true),
            Some([lo, hi]) if hi - lo >= 2 => (Look::List((lo..=hi).map(|n| (n, n.to_string())).collect()), true),
            _ => (Look::Fader, false),
        },
        _ => (Look::Fader, false),
    }
}

fn text_remark(remarks: &osd::Remarks, key: &str, at: &str, losses: &mut Vec<Loss>) -> Option<String> {
    match remarks.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) | None => None,
        Some(_) => {
            loss(losses, format!("{at}/{key}"), "a name is text: this remark is not text");
            None
        }
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
    let name = text_remark(&e.remarks, "_name", &at, losses)
        .or_else(|| Some(readable(&e.role.as_ref()?.setting)).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| last_segment(&e.osc));
    let (look, expects_read) = look_of(e);
    for (field, present, reason) in [
        ("orientation", e.orientation.is_some(), "the preset has no field for an orientation"),
        ("channel", e.channel.is_some(), "the channel of a preset belongs to its devices, which the export does not describe"),
        ("expects", e.expects.is_some() && !expects_read, "the preset has no field for it: only switch, momentary, trigger and discrete choose a pad or a list"),
        ("transform", e.transform.is_some(), "the preset has no field for it"),
        ("transform_reverse", e.transform_reverse.is_some(), "the preset has no field for it"),
        ("role", e.role.is_some(), "the preset has no field for it"),
        ("nrpn_lsb_range", e.nrpn_lsb_range.is_some(), "the preset has no field for it"),
    ] {
        if present {
            loss(losses, format!("{at}/{field}"), reason);
        }
    }
    for key in e.remarks.keys().filter(|k| *k != "_name") {
        loss(losses, format!("{at}/{}", segment(key)), "the preset has no field for it");
    }
    // The ways in the order the descriptor wrote them: the first with a complete binding is
    // written, with its own range; the export never reorders them.
    let ways = e.ways();
    let place = |k: usize| if k == 0 { at.clone() } else { format!("{at}/also/{}", k - 1) };
    let bindings: Vec<Vec<(Message, &'static str, &'static str)>> = ways.iter().enumerate().map(|(k, w)| way_bindings(w, &place(k), losses)).collect();
    let Some(chosen) = bindings.iter().position(|b| !b.is_empty()) else {
        loss(losses, at, "the control has no complete MIDI binding to write");
        return None;
    };
    let mut bindings = bindings;
    let mut written = std::mem::take(&mut bindings[chosen]);
    let (message, written_kind, _) = written.remove(0);
    for (_, kind, field) in written {
        loss(losses, format!("{}/{field}", place(chosen)), format!("a control of the preset sends one message: the {written_kind} way is written, the {kind} binding is not"));
    }
    for (k, b) in bindings.iter().enumerate().skip(chosen + 1) {
        if !b.is_empty() {
            let kinds: Vec<&str> = b.iter().map(|(_, kind, _)| *kind).collect();
            loss(
                losses,
                place(k),
                format!("a control of the preset sends one message: the {written_kind} way is written, the {} way is not", kinds.join(" and ")),
            );
        }
    }
    Some(Item { pointer: at, name, section: e.section.clone(), range: ways[chosen].range, message, look })
}

/// The complete bindings of one way, each with the kind the preset names it and its field; what
/// is incomplete is a loss at the way's place.
fn way_bindings(w: &osd::Way, at: &str, losses: &mut Vec<Loss>) -> Vec<(Message, &'static str, &'static str)> {
    let mut bindings = Vec::new();
    match (w.cc, w.cc_lsb) {
        (Some(cc), Some(lsb)) if cc <= 31 && lsb == cc + 32 => bindings.push((Message::Cc14(cc), "cc14", "cc")),
        (Some(cc), lsb) => {
            if lsb.is_some() {
                loss(losses, format!("{at}/cc_lsb"), "a 14-bit CC of the preset pairs a CC from 0 to 31 with the CC 32 above it");
            }
            bindings.push((Message::Cc7(cc), "cc7", "cc"));
        }
        (None, Some(_)) => loss(losses, format!("{at}/cc_lsb"), "an LSB without its CC"),
        (None, None) => {}
    }
    for (msb, lsb, field, make) in [
        (w.nrpn_msb, w.nrpn_lsb, "nrpn", Message::Nrpn as fn(u16) -> Message),
        (w.rpn_msb, w.rpn_lsb, "rpn", Message::Rpn as fn(u16) -> Message),
    ] {
        match (msb, lsb) {
            (Some(m), Some(l)) => bindings.push((make(u16::from(m) << 7 | u16::from(l)), field, field)),
            (None, None) => {}
            (Some(_), None) => loss(losses, format!("{at}/{field}_msb"), format!("an {field} needs its MSB and its LSB")),
            (None, Some(_)) => loss(losses, format!("{at}/{field}_lsb"), format!("an {field} needs its MSB and its LSB")),
        }
    }
    bindings
}

fn command_item(n: usize, c: &Command, losses: &mut Vec<Loss>) -> Option<Item> {
    let at = format!("/commands/{n}");
    let Some(frame) = &c.frame else {
        loss(losses, at, "the command has no frame: it is no MIDI message to write");
        return None;
    };
    let name = text_remark(&c.remarks, "_name", &at, losses).unwrap_or_else(|| last_segment(&address_text(&c.osc)));
    let mut bytes = Vec::new();
    for t in frame {
        match t {
            FrameToken::Byte(b) => bytes.push(*b),
            _ => {
                loss(losses, format!("{at}/frame"), "the frame has an argument, a capture or a computed byte: the preset template written holds fixed bytes only");
                return None;
            }
        }
    }
    for (field, present) in [
        ("notes", c.notes.is_some()),
        ("args", c.args.is_some()),
        ("pre", c.pre.is_some()),
        ("forward", c.forward.is_some()),
        ("script", c.script.is_some()),
    ] {
        if present {
            loss(losses, format!("{at}/{field}"), "the preset has no field for it");
        }
    }
    for key in c.remarks.keys().filter(|k| *k != "_name") {
        loss(losses, format!("{at}/{}", segment(key)), "the preset has no field for it");
    }
    Some(Item { pointer: at, name, section: c.section.clone(), range: None, message: Message::Sysex(bytes), look: Look::Pad })
}

fn message_json(m: &Message) -> Value {
    match m {
        Message::Cc7(n) => json!({"type": "cc7", "deviceId": 1, "parameterNumber": n}),
        Message::Cc14(n) => json!({"type": "cc14", "deviceId": 1, "parameterNumber": n}),
        Message::Nrpn(n) => json!({"type": "nrpn", "deviceId": 1, "parameterNumber": n}),
        Message::Rpn(n) => json!({"type": "rpn", "deviceId": 1, "parameterNumber": n}),
        Message::Sysex(bytes) => json!({"type": "sysex", "deviceId": 1, "data": bytes}),
    }
}

pub(crate) fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    if d.sources.as_ref().is_none_or(|s| s.is_empty()) {
        return Err(fault(FaultKind::NoSources, "/_sources", "a device without sources is not written: its credit and licence cannot be"));
    }
    let mut losses = Vec::new();
    loss(&mut losses, "/_sources", "the preset has no field for the credit and the licence of its sources");

    let info = &d.device;
    for (pointer, present) in [
        ("/device/vendor", info.vendor.is_some()),
        ("/device/revision", info.revision.is_some()),
        ("/device/manufacturer_id", info.manufacturer_id.is_some()),
        ("/device/device_id", info.device_id.is_some()),
        ("/device/rate_limit_hz", info.rate_limit_hz.is_some()),
        ("/device/kind", info.kind.is_some()),
        ("/device/transport", info.transport.is_some()),
    ] {
        if present {
            loss(&mut losses, pointer, "the preset has no field for it");
        }
    }
    for key in info.remarks.keys() {
        loss(&mut losses, format!("/device/{}", segment(key)), "the preset has no field for it");
    }
    for (pointer, present) in [
        ("/capabilities", d.capabilities.is_some()),
        ("/midi_of", d.midi_of.is_some()),
        ("/loads_sounds_of", d.loads_sounds_of.is_some()),
        ("/identity", d.identity.is_some()),
        ("/programs", d.programs.is_some()),
        ("/sysex", d.sysex.is_some()),
        ("/params", d.params.is_some()),
        ("/triggers", d.triggers.is_some()),
        ("/midi_out", d.midi_out.is_some()),
        ("/midi_in", d.midi_in.is_some()),
        ("/replies", d.replies.is_some()),
        ("/subscriptions", d.subscriptions.is_some()),
        ("/custom_commands", d.custom_commands.is_some()),
        ("/_coverage", d.coverage.is_some()),
    ] {
        if present {
            loss(&mut losses, pointer, "the preset carries controls and SysEx messages only");
        }
    }
    for key in d.remarks.keys() {
        loss(&mut losses, format!("/{}", segment(key)), "the preset has no field for it");
    }

    let mut items = Vec::new();
    if let Some(cc) = &d.cc_params {
        if cc.channel.is_some() {
            loss(&mut losses, "/cc_params/channel", "the channel of a preset belongs to its devices, which the export does not describe");
        }
        for key in cc.remarks.keys() {
            loss(&mut losses, format!("/cc_params/{}", segment(key)), "the preset has no field for it");
        }
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
        loss(&mut losses, i.pointer, format!("a preset holds at most {CONTROLS} controls: this one comes after them"));
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
        let range = item.range.or_else(|| match &item.look {
            Look::List(l) => Some([l.first()?.0, l.last()?.0]),
            _ => None,
        });
        if let Some([lo, hi]) = range {
            value.insert("min".to_string(), json!(lo));
            value.insert("max".to_string(), json!(hi));
        }
        value.insert("message".to_string(), message_json(&item.message));
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
    Ok(Translated { value: text, lost: losses })
}
