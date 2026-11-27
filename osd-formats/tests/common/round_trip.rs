//! The round-trip count of an importable format (osd-formats/docs/INTERFACE.md: the report of
//! carried and lost values of the converters): a source file is imported, then the device is
//! exported in the same format. Each value of the source file is counted once under its kind, as
//! carried, lost or structure; a value counted lost has to be named by a loss.
//!
//! - Kinds: a CSV, the column of each filled cell; a JSON file, the attribute path of each leaf
//!   with list indices written N (/controls/N/values/N/min); an OSCQuery namespace, that path
//!   inside its node (/ACCESS, /RANGE/N/MIN); a MIDNAM, Element@Attribute, or the element for its
//!   text.
//! - Structure: the kinds whose fate is structure in the table of the page of the format, and the
//!   number of a message.data of an Electra One preset, the id that names its template.
//! - Lost: a loss of the import covers the value or a block that holds it, or the exported file
//!   does not hold it equal on the paired element. Carried: every other. Equal is as the format
//!   reads it: a number written as text equals that number (a byte of a template written as text
//!   is hexadecimal), an omitted attribute equals the default of the format, F0 and F7 in a
//!   template are the framing, relcc is a relative cc7, the one range (and default) of a CSV row
//!   with a CC and an NRPN of one range reads for both.
//! - Keys: the elements of one key are paired in the order of the files, and an element left
//!   without a pair loses all its values. CSV: a row, by its CC (cc_msb), else its NRPN; of the
//!   triggers CSV, by its note range; else its rank among the rows that have none. Electra One: a
//!   control, a value and a message by the message (type and parameter number, or the bytes of
//!   its template); a template element by its message and its rank; an overlay by the message of
//!   the first value that names it, an item by its number; a page, a group and a device by their
//!   name; a template named by id is read at the message that names it. MIDNAM: a Control by its
//!   type and number, a Note by its number, a PatchBank by its name, a Patch by its bank and its
//!   program change (else its number), a list of value or patch names by the control or bank that uses it, any other by its
//!   parent and its name, number or channel, else its rank. OSCQuery: a node by its address.
//!   MIDI 2.0: an entry by its ctrlType and ctrlIndex, a map by the entry that first names it, an
//!   item by its value.
//! - Naming: a value counted lost is named when a loss of the import covers it, or a loss of the
//!   export lies at, above or under the JSON pointer of what its element became.

#![allow(dead_code)]

use super::xml;
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The values of one kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Figures {
    pub carried: usize,
    pub lost: usize,
    pub structure: usize,
}

impl Figures {
    pub fn of(carried: usize, lost: usize, structure: usize) -> Figures {
        Figures { carried, lost, structure }
    }

    pub fn add(&mut self, other: Figures) {
        self.carried += other.carried;
        self.lost += other.lost;
        self.structure += other.structure;
    }

    pub fn total(&self) -> usize {
        self.carried + self.lost + self.structure
    }
}

/// The count of one file, or of several added together.
#[derive(Clone, Debug, Default)]
pub struct Count {
    pub kinds: BTreeMap<String, Figures>,
    /// Each value counted lost that no loss names: its kind, its content and its place.
    pub unnamed: Vec<String>,
}

impl Count {
    pub fn total(&self) -> Figures {
        let mut total = Figures::default();
        self.kinds.values().for_each(|f| total.add(*f));
        total
    }

    pub fn add(&mut self, other: Count) {
        for (kind, f) in other.kinds {
            self.kinds.entry(kind).or_default().add(f);
        }
        self.unnamed.extend(other.unnamed);
    }

    /// The table of figures, as the page of a format writes it.
    pub fn markdown(&self) -> String {
        let row = |k: &str, f: Figures| format!("| {k} | {} | {} | {} |\n", f.carried, f.lost, f.structure);
        let mut out = String::from("| Kind | Carried | Lost | Structure |\n|---|---|---|---|\n");
        for (kind, f) in &self.kinds {
            out.push_str(&row(&format!("`{kind}`"), *f));
        }
        out.push_str(&row("**Total**", self.total()));
        out
    }
}

/// The source given to the import of a counted file.
fn source() -> osd::Source {
    super::source()
}

/// What an import and the export of its device give.
pub struct Trip {
    pub import_lost: Vec<Loss>,
    pub device: Value,
    pub exported: String,
    pub export_lost: Vec<Loss>,
}

pub fn trip(format: Format, text: &str) -> Trip {
    let imported = import(text, format, &source()).unwrap_or_else(|f| panic!("{}: the file does not import: {f}", format.name()));
    let device = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(imported.value.clone())))).expect("a device is JSON");
    let out = export(&super::resolved(imported.value), format).unwrap_or_else(|f| panic!("{}: the device does not export: {f}", format.name()));
    Trip { import_lost: imported.lost, device, exported: out.value, export_lost: out.lost }
}

/// The count of a file through its format.
pub fn count(format: Format, text: &str) -> Count {
    count_trip(format, text, &trip(format, text), &structure_kinds(format))
}

// ---- the elements ---------------------------------------------------------------------------

struct Leaf {
    kind: String,
    field: String,
    content: String,
    /// The places a loss of the import covers this value at: it, and the blocks that hold it.
    places: Vec<String>,
    structure: bool,
}

#[derive(Default)]
struct Element {
    shape: String,
    key: String,
    values: Vec<Leaf>,
    /// What the element says, by field, as the format reads it: what a source value is compared to.
    reads: BTreeMap<String, String>,
    /// The JSON pointers, in the descriptor imported, of what the element became.
    targets: Vec<String>,
}

impl Element {
    fn new(shape: impl Into<String>, key: impl Into<String>) -> Element {
        Element { shape: shape.into(), key: key.into(), ..Element::default() }
    }

    fn push(&mut self, kind: impl Into<String>, field: impl Into<String>, content: String, places: Vec<String>) {
        let field = field.into();
        self.reads.insert(field.clone(), content.clone());
        self.values.push(Leaf { kind: kind.into(), field, content, places, structure: false });
    }
}

/// A content as the format reads it: a number written as text equals that number.
fn normal(text: &str) -> String {
    match text.trim().parse::<f64>() {
        Ok(n) if n.is_finite() => format!("{n}"),
        _ => text.to_string(),
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => normal(s),
        Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), |n| format!("{n}")),
        other => other.to_string(),
    }
}

fn escaped(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn under(place: &str, pointer: &str) -> bool {
    place == pointer || place.strip_prefix(pointer).is_some_and(|r| r.starts_with('/'))
}

/// Hands out, for a key, the descriptor items that have it, one after the other.
#[derive(Default)]
struct Items {
    by_key: BTreeMap<String, Vec<String>>,
    taken: BTreeMap<String, usize>,
}

impl Items {
    fn add(&mut self, key: String, pointer: String) {
        self.by_key.entry(key).or_default().push(pointer);
    }

    fn take(&mut self, key: &str) -> Option<String> {
        let n = self.taken.entry(key.to_string()).or_default();
        let found = self.by_key.get(key).and_then(|l| l.get(*n)).cloned();
        *n += 1;
        found
    }
}

fn list<'a>(v: &'a Value, pointer: &str) -> impl Iterator<Item = (usize, &'a Value)> {
    v.pointer(pointer).and_then(Value::as_array).into_iter().flatten().enumerate()
}

fn index_of(way: &Value) -> Vec<i64> {
    way["ctrlIndex"].as_array().into_iter().flatten().filter_map(Value::as_i64).collect()
}

/// Which of the two files is read: a list shared by several users counts once in the source, and
/// is held by each of them in the export.
#[derive(Clone, Copy, PartialEq)]
enum Side {
    Source,
    Export,
}

fn users_of(side: Side, users: &[String]) -> &[String] {
    match side {
        Side::Source => &users[..users.len().min(1)],
        Side::Export => users,
    }
}

// ---- the two CSV ------------------------------------------------------------------------------

/// The records of a CSV text, each with the line it starts on.
fn records(text: &str) -> Vec<(usize, Vec<String>)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let (mut out, mut cells, mut cell) = (Vec::new(), Vec::new(), String::new());
    let (mut quoted, mut line, mut start) = (false, 1usize, 1usize);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            line += 1;
        }
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            ('"', true) => quoted = false,
            ('"', false) if cell.is_empty() => quoted = true,
            (',', false) => cells.push(std::mem::take(&mut cell)),
            ('\r', false) if chars.peek() == Some(&'\n') => {}
            ('\n', false) => {
                if !(cells.is_empty() && cell.is_empty()) {
                    cells.push(std::mem::take(&mut cell));
                    out.push((start, std::mem::take(&mut cells)));
                }
                start = line;
            }
            _ => cell.push(c),
        }
    }
    if !(cells.is_empty() && cell.is_empty()) {
        cells.push(cell);
        out.push((start, cells));
    }
    out
}

fn csv_elements(format: Format, text: &str, device: Option<&Value>) -> Vec<Element> {
    let rows = records(text);
    let header = rows.first().map(|r| r.1.clone()).unwrap_or_default();
    let triggers = format == Format::MidiTriggersCsv;
    let mut items = Items::default();
    if let Some(d) = device {
        if triggers {
            for (n, e) in list(d, "/triggers/entries") {
                let key = format!("note:{}:{}", scalar(&e["note_range"][0]), scalar(&e["note_range"][1]));
                items.add(key, format!("/triggers/entries/{n}"));
            }
        } else {
            for (n, e) in list(d, "/cc_params/entries") {
                for (_, w) in list(e, "/ways") {
                    let i = index_of(w);
                    let key = match w["ctrlType"].as_str() {
                        Some("cc") => format!("cc:{}", i[0]),
                        Some("nrpn") => format!("nrpn:{}:{}", i[0], i[1]),
                        _ => continue,
                    };
                    items.add(key, format!("/cc_params/entries/{n}"));
                }
            }
        }
    }
    let mut out = Vec::new();
    for (line, cells) in rows.iter().skip(1) {
        let cell = |name: &str| header.iter().position(|h| h == name).and_then(|i| cells.get(i)).map(|c| c.trim()).filter(|c| !c.is_empty()).map(normal);
        let pair = |kind: &str, a: &str, b: &str| Some(format!("{kind}:{}:{}", cell(a)?, cell(b)?));
        let cc = cell("cc_msb").map(|n| format!("cc:{n}"));
        let nrpn = pair("nrpn", "nrpn_msb", "nrpn_lsb");
        let key = if triggers { pair("note", "note_number_min_value", "note_number_max_value") } else { cc.clone().or_else(|| nrpn.clone()) };
        let mut e = Element::new("row", key.unwrap_or_else(|| "rank".to_string()));
        for (column, content) in header.iter().zip(cells) {
            if !content.is_empty() {
                let places = vec![format!("line {line}"), format!("line {line}, column {column}")];
                e.push(column.clone(), column.clone(), normal(content), places);
            }
        }
        // A row with a CC and an NRPN says one range, and one default, for both of one range.
        if cc.is_some() && nrpn.is_some() {
            let cross = |e: &mut Element, a: &str, b: &str| {
                for (from, to) in [(a, b), (b, a)] {
                    if !e.reads.contains_key(to) {
                        if let Some(v) = e.reads.get(from).cloned() {
                            e.reads.insert(to.to_string(), v);
                        }
                    }
                }
            };
            cross(&mut e, "cc_min_value", "nrpn_min_value");
            cross(&mut e, "cc_max_value", "nrpn_max_value");
            if e.reads.get("cc_min_value") == e.reads.get("nrpn_min_value") && e.reads.get("cc_max_value") == e.reads.get("nrpn_max_value") {
                cross(&mut e, "cc_default_value", "nrpn_default_value");
            }
        }
        // The row becomes one item, found by the first of its messages the descriptor holds.
        let found: Vec<String> = if triggers {
            items.take(&e.key).into_iter().collect()
        } else {
            [cc, nrpn].into_iter().flatten().filter_map(|k| items.take(&k)).collect()
        };
        e.targets = found;
        out.push(e);
    }
    out
}

// ---- the Electra One preset -------------------------------------------------------------------

const MESSAGE: &str = "/controls/N/values/N/message";

/// A byte of a template: a number, or a text read as hexadecimal.
fn byte(v: &Value) -> String {
    match v.as_str().and_then(|s| u8::from_str_radix(s, 16).ok()) {
        Some(b) => b.to_string(),
        None => scalar(v),
    }
}

fn framing(b: &str) -> bool {
    b == "240" || b == "247"
}

/// The template a SysEx message names by id, with its pointer and the kind of its data.
fn named_template<'a>(preset: &'a Value, message: &Value) -> Option<(String, &'a Value)> {
    let id = message.get("data").filter(|d| d.is_number())?;
    for (d, device) in list(preset, "/devices") {
        if message.get("deviceId").is_some_and(|i| Some(i) != device.get("id")) {
            continue;
        }
        for (m, template) in list(device, "/messages") {
            if template.get("id") == Some(id) {
                return Some((format!("/devices/{d}/messages/{m}"), template));
            }
        }
    }
    None
}

fn message_key(preset: &Value, message: &Value) -> String {
    let kind = message.get("type").and_then(Value::as_str).unwrap_or("sysex");
    if kind != "sysex" {
        let kind = if kind == "relcc" { "cc7" } else { kind };
        return format!("{kind}:{}", message.get("parameterNumber").map_or_else(|| "0".to_string(), scalar));
    }
    let data = named_template(preset, message).and_then(|(_, t)| t.get("data")).or_else(|| message.get("data"));
    match data.and_then(Value::as_array) {
        Some(data) => {
            let parts: Vec<String> = data
                .iter()
                .map(|v| v.get("type").and_then(Value::as_str).map_or_else(|| byte(v), str::to_string))
                .filter(|b| !framing(b))
                .collect();
            format!("sysex:{}", parts.join(" "))
        }
        None => format!("sysex:id:{}", data.map_or_else(String::new, scalar)),
    }
}

/// The key of the item a command became: its frame, as a template says it.
fn frame_key(command: &Value) -> String {
    let parts: Vec<String> = list(command, "/frame")
        .map(|(_, t)| match t {
            Value::Object(o) if o.contains_key("arg") => "value".to_string(),
            Value::Object(o) if o.contains_key("checksum") => "checksum".to_string(),
            other => byte(other),
        })
        .filter(|b| !framing(b))
        .collect();
    format!("sysex:{}", parts.join(" "))
}

/// A JSON object of a preset as an element, the objects it holds as elements of their own, keyed
/// by it and their name, id or rank.
fn object(v: &Value, [pointer, kind, shape]: [&str; 3], key: &str, targets: &[String], skip: &[&str], out: &mut Vec<Element>) -> Element {
    let mut e = Element::new(shape, key);
    e.targets = targets.to_vec();
    for (k, v) in v.as_object().into_iter().flatten().filter(|(k, _)| !skip.contains(&k.as_str())) {
        members(v, &format!("{pointer}/{}", escaped(k)), &format!("{kind}/{k}"), &format!("{shape}/{k}"), k, &mut e, out);
    }
    e
}

fn members(v: &Value, pointer: &str, kind: &str, shape: &str, field: &str, e: &mut Element, out: &mut Vec<Element>) {
    match v {
        Value::Object(_) => {
            let key = format!("{}>{field}", e.key);
            let child = object(v, [pointer, kind, shape], &key, &e.targets.clone(), &[], out);
            out.push(child);
        }
        Value::Array(a) => {
            for (i, item) in a.iter().enumerate() {
                let (pointer, kind, shape) = (format!("{pointer}/{i}"), format!("{kind}/N"), format!("{shape}/N"));
                if item.is_object() {
                    let own = item.get("name").or_else(|| item.get("id")).map_or_else(|| i.to_string(), scalar);
                    let key = format!("{}>{field}:{own}", e.key);
                    let child = object(item, [&pointer, &kind, &shape], &key, &e.targets.clone(), &[], out);
                    out.push(child);
                } else {
                    members(item, &pointer, &kind, &shape, &format!("{field}/{i}"), e, out);
                }
            }
        }
        leaf => e.push(kind, field, scalar(leaf), vec![pointer.to_string()]),
    }
}

/// The data of a SysEx template, on the element of the message that sends it. at: the pointer
/// and the kind of the data; also: the message that names it, whose losses cover it too.
fn template_data(data: &[Value], at: &str, kind: &str, also: Option<&str>, message: &mut Element, out: &mut Vec<Element>) {
    let mut rank = 0;
    let places = |tail: &str| -> Vec<String> { std::iter::once(format!("{at}{tail}")).chain(also.map(|a| format!("{a}/data{tail}"))).collect() };
    for (i, item) in data.iter().enumerate() {
        if item.is_object() {
            let key = format!("{}>data:{rank}", message.key);
            let shape = format!("{MESSAGE}/data/N");
            let mut e = Element::new(shape.clone(), key.clone());
            e.targets = message.targets.clone();
            for (k, v) in item.as_object().into_iter().flatten() {
                if k == "rules" {
                    for (j, rule) in v.as_array().into_iter().flatten().enumerate() {
                        let mut r = Element::new(format!("{shape}/rules/N"), format!("{key}>rules:{j}"));
                        r.targets = message.targets.clone();
                        for (rk, rv) in rule.as_object().into_iter().flatten() {
                            r.push(format!("{kind}/N/rules/N/{rk}"), rk.clone(), scalar(rv), places(&format!("/{i}/rules/{j}/{}", escaped(rk))));
                        }
                        out.push(r);
                    }
                } else {
                    e.push(format!("{kind}/N/{k}"), k.clone(), scalar(v), places(&format!("/{i}/{}", escaped(k))));
                }
            }
            out.push(e);
            rank += 1;
        } else if framing(&byte(item)) {
            message.push(format!("{kind}/N"), "framing", "framing".to_string(), places(&format!("/{i}")));
        } else {
            message.push(format!("{kind}/N"), format!("data/{rank}"), byte(item), places(&format!("/{i}")));
            rank += 1;
        }
    }
}

fn electra_elements(side: Side, text: &str, device: Option<&Value>) -> Vec<Element> {
    let preset: Value = serde_json::from_str(text).expect("a preset is JSON");
    let mut items = Items::default();
    if let Some(d) = device {
        for (n, e) in list(d, "/cc_params/entries") {
            for (_, w) in list(e, "/ways") {
                let i = index_of(w);
                let key = match (w["ctrlType"].as_str(), i.as_slice()) {
                    (Some("cc"), [cc, _]) => format!("cc14:{cc}"),
                    (Some("cc"), [cc]) => format!("cc7:{cc}"),
                    (Some(kind @ ("nrpn" | "rpn")), [m, l]) => format!("{kind}:{}", m * 128 + l),
                    _ => continue,
                };
                items.add(key, format!("/cc_params/entries/{n}"));
            }
        }
        for (n, c) in list(d, "/commands") {
            items.add(frame_key(c), format!("/commands/{n}"));
        }
    }
    // What each control became, and the keys of the messages, the overlays and the groups.
    let controls: Vec<(usize, &Value)> = list(&preset, "/controls").collect();
    let mut of_control: Vec<Vec<(String, Vec<String>)>> = Vec::new();
    for (_, c) in &controls {
        let values = list(c, "/values")
            .map(|(k, v)| match v.get("message") {
                Some(m) => {
                    let key = message_key(&preset, m);
                    let target = items.take(&key).into_iter().collect();
                    (key, target)
                }
                None => (format!("{}>value:{k}", c.get("name").map_or_else(String::new, scalar)), Vec::new()),
            })
            .collect();
        of_control.push(values);
    }
    let mut out = Vec::new();
    for ((n, c), values) in controls.iter().zip(&of_control) {
        let at = format!("/controls/{n}");
        let targets: Vec<String> = values.iter().flat_map(|v| v.1.clone()).collect();
        let key = values.first().map_or_else(|| format!("name:{}", c.get("name").map_or_else(|| n.to_string(), scalar)), |v| v.0.clone());
        let control = object(c, [&at, "/controls/N", "/controls/N"], &key, &targets, &["values"], &mut out);
        out.push(control);
        for ((k, v), (key, targets)) in list(c, "/values").zip(values) {
            let at = format!("{at}/values/{k}");
            let value = object(v, [&at, "/controls/N/values/N", "/controls/N/values/N"], key, targets, &["message"], &mut out);
            out.push(value);
            let Some(m) = v.get("message") else { continue };
            let at = format!("{at}/message");
            let kind = m.get("type").and_then(Value::as_str);
            let named = named_template(&preset, m);
            let inline = m.get("data").and_then(Value::as_array);
            let skip: &[&str] = if inline.is_some() { &["data"] } else { &[] };
            let mut message = object(m, [&at, MESSAGE, MESSAGE], key, targets, skip, &mut out);
            for leaf in &mut message.values {
                if leaf.field == "data" && named.is_some() {
                    leaf.structure = true;
                }
                if leaf.field == "type" && leaf.content == "relcc" {
                    leaf.content = "cc7".to_string();
                }
            }
            if kind == Some("relcc") {
                message.reads.insert("type".to_string(), "cc7".to_string());
                message.reads.insert("relative".to_string(), "true".to_string());
            }
            message.reads.entry("relative".to_string()).or_insert_with(|| "false".to_string());
            if let Some(data) = inline {
                template_data(data, &format!("{at}/data"), &format!("{MESSAGE}/data"), None, &mut message, &mut out);
                message.reads.insert("framing".to_string(), "framing".to_string());
            }
            if let Some((pointer, template)) = named {
                // Counted once, at the first message that names it.
                let first = controls.iter().flat_map(|(_, c)| list(c, "/values")).filter_map(|(_, v)| v.get("message")).find(|o| named_template(&preset, o).is_some_and(|t| t.0 == pointer));
                if side == Side::Export || first.is_some_and(|f| std::ptr::eq(f, m)) {
                    if let Some(data) = template.get("data").and_then(Value::as_array) {
                        let kind = "/devices/N/messages/N/data";
                        template_data(data, &format!("{pointer}/data"), kind, Some(&at), &mut message, &mut out);
                    }
                }
            }
            out.push(message);
        }
    }
    // The targets of the controls that satisfy a condition.
    let users = |wanted: &dyn Fn(&Value) -> bool| -> Vec<String> {
        controls.iter().zip(&of_control).filter(|((_, c), _)| wanted(c)).flat_map(|(_, v)| v.iter().flat_map(|v| v.1.clone())).collect()
    };
    for (n, o) in list(&preset, "/overlays") {
        let id = o.get("id");
        let named: Vec<(String, Vec<String>)> = controls
            .iter()
            .zip(&of_control)
            .flat_map(|((_, c), keys)| list(c, "/values").zip(keys).filter(|((_, v), _)| id.is_some() && v.get("overlayId") == id).map(|(_, k)| k.clone()).collect::<Vec<_>>())
            .collect();
        let keys: Vec<String> = named.iter().map(|k| k.0.clone()).collect();
        let targets: Vec<String> = named.iter().flat_map(|k| k.1.iter().map(|t| format!("{t}/values"))).collect();
        let unnamed = [format!("rank:{n}")];
        let keys = if keys.is_empty() { &unnamed[..] } else { users_of(side, &keys) };
        for key in keys {
            let at = format!("/overlays/{n}");
            let overlay = object(o, [&at, "/overlays/N", "/overlays/N"], key, &targets, &["items"], &mut out);
            out.push(overlay);
            for (m, item) in list(o, "/items") {
                let own = item.get("value").map_or_else(|| m.to_string(), scalar);
                let item = object(item, [&format!("{at}/items/{m}"), "/overlays/N/items/N", "/overlays/N/items/N"], &format!("{key}>{own}"), &targets, &[], &mut out);
                out.push(item);
            }
        }
    }
    let device_block = vec!["/device".to_string()];
    for block in ["pages", "groups", "devices"] {
        for (n, v) in list(&preset, &format!("/{block}")) {
            let at = format!("/{block}/{n}");
            let kind = format!("/{block}/N");
            let key = v.get("name").map_or_else(|| n.to_string(), scalar);
            let targets = if block == "groups" {
                users(&|c| c.get("groupId").is_some() && c.get("groupId") == v.get("id")).iter().map(|t| format!("{t}/section")).collect()
            } else {
                device_block.clone()
            };
            let skip: &[&str] = if block == "devices" { &["messages"] } else { &[] };
            let e = object(v, [&at, &kind, &kind], &key, &targets, skip, &mut out);
            if block == "devices" {
                for (m, template) in list(v, "/messages") {
                    let pointer = format!("{at}/messages/{m}");
                    let used = controls.iter().flat_map(|(_, c)| list(c, "/values")).filter_map(|(_, v)| v.get("message")).any(|o| named_template(&preset, o).is_some_and(|t| t.0 == pointer));
                    let own = template.get("id").map_or_else(|| m.to_string(), scalar);
                    let skip: &[&str] = if used { &["data"] } else { &[] };
                    let kind = "/devices/N/messages/N";
                    let t = object(template, [&pointer, kind, kind], &format!("{}>{own}", e.key), &targets, skip, &mut out);
                    out.push(t);
                }
            }
            out.push(e);
        }
    }
    let root = object(&preset, ["", "", ""], "", &device_block, &["pages", "groups", "devices", "overlays", "controls"], &mut out);
    out.push(root);
    out
}

// ---- MIDNAM -----------------------------------------------------------------------------------

/// The attributes a document may omit, with the default of the DTD.
const DEFAULTS: [(&str, &str, &str); 3] = [("Control", "Type", "7bit"), ("PatchBank", "ROM", "false"), ("AvailableChannel", "Available", "true")];

/// The numeric character references the test reader leaves as they are.
fn decoded(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("&#") {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let code = rest.find(';').and_then(|end| {
            let digits = &rest[2..end];
            let n = digits.strip_prefix('x').map_or_else(|| digits.parse().ok(), |h| u32::from_str_radix(h, 16).ok());
            Some((char::from_u32(n?)?, end))
        });
        match code {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn descendants<'a>(n: &'a xml::Node, out: &mut Vec<&'a xml::Node>) {
    out.push(n);
    n.children.iter().for_each(|c| descendants(c, out));
}

fn control_key(n: &xml::Node) -> String {
    format!("control:{}:{}", n.attr("Type").unwrap_or("7bit"), normal(n.attr("Number").unwrap_or("")))
}

struct Midnam {
    side: Side,
    /// The controls that use each named list of value names, and the banks that use each named
    /// list of patch names: their keys, and what each became.
    values: BTreeMap<String, Vec<(String, Vec<String>)>>,
    patches: BTreeMap<String, Vec<(String, Vec<String>)>>,
    controls: Items,
    notes: Items,
    banks: Items,
    out: Vec<Element>,
}

impl Midnam {
    fn walk(&mut self, n: &xml::Node, key: &str, targets: &[String], places: &[String]) {
        let mut places = places.to_vec();
        places.push(format!("{}:{}", n.line, n.name));
        let mut e = Element::new(n.name.clone(), key);
        e.targets = targets.to_vec();
        for (attr, content) in &n.attrs {
            let mut here = places.clone();
            here.push(format!("{}:{}@{attr}", n.line, n.name));
            e.push(format!("{}@{attr}", n.name), format!("@{attr}"), normal(&decoded(content)), here);
        }
        let text = n.text.trim();
        if !text.is_empty() {
            e.push(n.name.clone(), "text", normal(&decoded(text)), places.clone());
        }
        for (_, attr, default) in DEFAULTS.iter().filter(|d| d.0 == n.name) {
            e.reads.entry(format!("@{attr}")).or_insert_with(|| default.to_string());
        }
        self.out.push(e);
        for (i, c) in n.children.iter().enumerate() {
            let rank = n.children[..i].iter().filter(|o| o.name == c.name).count();
            let own = c.attr("Name").or_else(|| c.attr("Channel")).or_else(|| c.attr("Control")).or_else(|| c.attr("Number"));
            let child = format!("{key}/{}:{}", c.name, own.map_or_else(|| rank.to_string(), normal));
            match c.name.as_str() {
                "Control" => {
                    let k = control_key(c);
                    let target: Vec<String> = self.controls.take(&k).into_iter().collect();
                    self.walk(c, &k, &target, &places);
                }
                "Note" => {
                    let k = format!("note:{}", normal(c.attr("Number").unwrap_or("")));
                    let target: Vec<String> = self.notes.take(&k).into_iter().collect();
                    self.walk(c, &k, &target, &places);
                }
                "PatchBank" => {
                    let name = c.attr("Name").map_or_else(|| rank.to_string(), str::to_string);
                    let target: Vec<String> = self.banks.take(&name).into_iter().collect();
                    self.walk(c, &format!("bank:{name}"), &target, &places);
                }
                "Patch" => {
                    let bank = key.split("/list").next().unwrap_or(key);
                    // A patch by the program change that recalls it, else by its number.
                    let own = c.attr("ProgramChange").map_or_else(|| format!("number:{}", normal(c.attr("Number").unwrap_or(""))), |p| format!("pc:{}", normal(p)));
                    self.walk(c, &format!("{bank}/Patch:{own}"), targets, &places);
                }
                "Values" | "MIDICommands" => self.walk(c, &format!("{key}/{}", c.name), targets, &places),
                "ValueNameList" | "PatchNameList" => {
                    // Inline, the list of the control or bank that holds it; named, of those that use it.
                    let inline = n.name == "Values" || n.name == "PatchBank";
                    if inline {
                        let user = key.strip_suffix("/Values").unwrap_or(key);
                        self.walk(c, &format!("{user}/list"), targets, &places);
                    } else {
                        let uses = if c.name == "ValueNameList" { &self.values } else { &self.patches };
                        let users = uses.get(c.attr("Name").unwrap_or("")).cloned().unwrap_or_default();
                        if users.is_empty() {
                            self.walk(c, &child, &[], &places);
                        }
                        let taken = if self.side == Side::Source { users.len().min(1) } else { users.len() };
                        for (user, targets) in users.into_iter().take(taken) {
                            self.walk(c, &format!("{user}/list"), &targets, &places);
                        }
                    }
                }
                "Value" => self.walk(c, &format!("{key}/Value:{}", normal(c.attr("Number").unwrap_or(""))), targets, &places),
                "Manufacturer" => self.walk(c, &child, &["/device/vendor".to_string()], &places),
                "Model" => self.walk(c, &child, &["/device/name".to_string()], &places),
                "MasterDeviceNames" => self.walk(c, &child, &["/device".to_string()], &places),
                "Author" => self.walk(c, &child, &[], &places),
                _ => {
                    // Lists and groups pass nothing down to what they hold, which has its own key.
                    let held = if ["ControlNameList", "NoteNameList", "NoteGroup"].contains(&c.name.as_str()) { &[] } else { targets };
                    let held: Vec<String> = held.iter().filter(|t| t.as_str() != "/device").cloned().collect();
                    self.walk(c, &child, &held, &places);
                }
            }
        }
    }
}

fn midnam_elements(side: Side, text: &str, device: Option<&Value>) -> Vec<Element> {
    let document = xml::parse(text.strip_prefix('\u{feff}').unwrap_or(text)).unwrap_or_else(|e| panic!("the MIDNAM does not read: {e}"));
    let mut m = Midnam { side, values: BTreeMap::new(), patches: BTreeMap::new(), controls: Items::default(), notes: Items::default(), banks: Items::default(), out: Vec::new() };
    if let Some(d) = device {
        for (n, e) in list(d, "/cc_params/entries") {
            for (_, w) in list(e, "/ways") {
                let key = match (w["ctrlType"].as_str(), index_of(w).as_slice()) {
                    (Some("cc"), [cc, _]) => format!("control:14bit:{cc}"),
                    (Some("cc"), [cc]) => format!("control:7bit:{cc}"),
                    (Some("nrpn"), [h, l]) => format!("control:NRPN:{}", h * 128 + l),
                    (Some("rpn"), [h, l]) => format!("control:RPN:{}", h * 128 + l),
                    _ => continue,
                };
                m.controls.add(key, format!("/cc_params/entries/{n}"));
            }
        }
        for (n, t) in list(d, "/triggers/entries") {
            m.notes.add(format!("note:{}", scalar(&t["note_range"][0])), format!("/triggers/entries/{n}"));
        }
        for (n, b) in list(d, "/programs/banks") {
            m.banks.add(b["name"].as_str().unwrap_or("").to_string(), format!("/programs/banks/{n}"));
        }
    }
    // Who uses each named list.
    let mut all = Vec::new();
    descendants(&document.root, &mut all);
    let (mut controls, mut banks) = (Items { by_key: m.controls.by_key.clone(), ..Items::default() }, Items { by_key: m.banks.by_key.clone(), ..Items::default() });
    for c in all.iter().filter(|n| n.name == "Control") {
        let key = control_key(c);
        let targets: Vec<String> = controls.take(&key).into_iter().map(|t| format!("{t}/values")).collect();
        if let Some(name) = c.kid("Values").and_then(|v| v.kid("UsesValueNameList")).and_then(|u| u.attr("Name")) {
            m.values.entry(name.to_string()).or_default().push((key, targets));
        }
    }
    for (rank, b) in all.iter().filter(|n| n.name == "PatchBank").enumerate() {
        let bank = b.attr("Name").map_or_else(|| rank.to_string(), str::to_string);
        let targets: Vec<String> = banks.take(&bank).into_iter().collect();
        if let Some(name) = b.kid("UsesPatchNameList").and_then(|u| u.attr("Name")) {
            m.patches.entry(name.to_string()).or_default().push((format!("bank:{bank}"), targets));
        }
    }
    m.walk(&document.root, "", &[], &[]);
    m.out
}

/// The place a loss of the MIDNAM import covers: its line and what leads its reason.
fn midnam_place(l: &Loss) -> String {
    let line = l.place.strip_prefix("line ").and_then(|r| r.split(',').next()).unwrap_or("");
    format!("{line}:{}", l.reason.split(':').next().unwrap_or(""))
}

// ---- OSCQuery ---------------------------------------------------------------------------------

fn leaves(v: &Value, pointer: &str, kind: &str, field: &str, e: &mut Element) {
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                leaves(v, &format!("{pointer}/{}", escaped(k)), &format!("{kind}/{k}"), &format!("{field}/{k}"), e);
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                leaves(v, &format!("{pointer}/{i}"), &format!("{kind}/N"), &format!("{field}/{i}"), e);
            }
        }
        leaf => e.push(kind, field, scalar(leaf), vec![pointer.to_string()]),
    }
}

fn oscquery_elements(text: &str, device: Option<&Value>) -> Vec<Element> {
    let root: Value = serde_json::from_str(text).expect("a namespace is JSON");
    let mut targets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(d) = device {
        let prefix = d["device"]["osc_prefix"].as_str().unwrap_or("").to_string();
        for (n, c) in list(d, "/commands") {
            let osc = match &c["osc"] {
                Value::Array(parts) => parts.iter().map(|p| p.as_str().map_or_else(|| format!("{{{}}}", p["capture"].as_str().unwrap_or("")), str::to_string)).collect(),
                other => other.as_str().unwrap_or("").to_string(),
            };
            targets.entry(format!("{prefix}{osc}")).or_default().push(format!("/commands/{n}"));
        }
        for at in ["/".to_string(), prefix] {
            targets.entry(at).or_default().push("/device".to_string());
        }
    }
    let mut out = Vec::new();
    fn walk(node: &Value, pointer: &str, path: &str, targets: &BTreeMap<String, Vec<String>>, out: &mut Vec<Element>) {
        let mut e = Element::new("node", path);
        e.targets = targets.get(path).cloned().unwrap_or_default();
        for (k, v) in node.as_object().into_iter().flatten().filter(|(k, _)| *k != "CONTENTS") {
            leaves(v, &format!("{pointer}/{}", escaped(k)), &format!("/{k}"), k, &mut e);
        }
        out.push(e);
        for (k, c) in node.get("CONTENTS").and_then(Value::as_object).into_iter().flatten() {
            walk(c, &format!("{pointer}/CONTENTS/{}", escaped(k)), &format!("{}/{k}", path.trim_end_matches('/')), targets, out);
        }
    }
    walk(&root, "", "/", &targets, &mut out);
    out
}

// ---- the MIDI 2.0 controller list ---------------------------------------------------------------

fn entry_key(kind: &str, index: &[i64]) -> String {
    let index = if kind == "cc" { &index[..index.len().min(1)] } else { index };
    format!("{kind}:{index:?}")
}

fn midi2_elements(side: Side, text: &str, device: Option<&Value>) -> Vec<Element> {
    let root: Value = serde_json::from_str(text).expect("a controller list is JSON");
    let mut items = Items::default();
    if let Some(d) = device {
        for (n, e) in list(d, "/cc_params/entries") {
            for (_, w) in list(e, "/ways") {
                items.add(entry_key(w["ctrlType"].as_str().unwrap_or(""), &index_of(w)), format!("/cc_params/entries/{n}"));
            }
        }
    }
    let mut out = Vec::new();
    let mut maps: Vec<(Value, String, Vec<String>)> = Vec::new();
    let lists: Vec<String> = match &root {
        Value::Array(_) => vec![String::new()],
        _ => ["ChCtrlList", "AllCtrlList"].iter().filter(|k| root.get(**k).is_some()).map(|k| format!("/{k}")).collect(),
    };
    for at in &lists {
        for (n, entry) in list(&root, at) {
            let key = entry_key(entry["ctrlType"].as_str().unwrap_or(""), &index_of(entry));
            let mut e = Element::new("entry", key.clone());
            e.targets = items.take(&key).into_iter().collect();
            for (k, v) in entry.as_object().into_iter().flatten() {
                leaves(v, &format!("{at}/{n}/{}", escaped(k)), &format!("{at}/N/{k}"), k, &mut e);
            }
            if let Some(id) = entry.get("ctrlMapId") {
                maps.push((id.clone(), key, e.targets.iter().map(|t| format!("{t}/values")).collect()));
            }
            out.push(e);
        }
    }
    let mut rest = Element::new("root", "");
    rest.targets = vec!["/device".to_string()];
    for (k, v) in root.as_object().into_iter().flatten() {
        if !["ChCtrlList", "AllCtrlList", "CtrlMapList"].contains(&k.as_str()) {
            leaves(v, &format!("/{}", escaped(k)), &format!("/{k}"), k, &mut rest);
        }
    }
    out.push(rest);
    for (n, map) in list(&root, "/CtrlMapList") {
        let users: Vec<&(Value, String, Vec<String>)> = maps.iter().filter(|m| Some(&m.0) == map.get("ctrlMapId")).collect();
        let targets: Vec<String> = users.iter().flat_map(|u| u.2.clone()).collect();
        let mut keys: Vec<String> = users.iter().map(|u| u.1.clone()).collect();
        if keys.is_empty() {
            keys.push(format!("rank:{n}"));
        }
        for key in users_of(side, &keys) {
            let mut e = Element::new("map", key.clone());
            e.targets = targets.clone();
            for (k, v) in map.as_object().into_iter().flatten().filter(|(k, _)| *k != "ctrlMap") {
                leaves(v, &format!("/CtrlMapList/{n}/{}", escaped(k)), &format!("/CtrlMapList/N/{k}"), k, &mut e);
            }
            out.push(e);
            for (m, item) in list(map, "/ctrlMap") {
                let own = item.get("value").map_or_else(|| m.to_string(), scalar);
                let mut e = Element::new("item", format!("{key}>{own}"));
                e.targets = targets.clone();
                for (k, v) in item.as_object().into_iter().flatten() {
                    leaves(v, &format!("/CtrlMapList/{n}/ctrlMap/{m}/{}", escaped(k)), &format!("/CtrlMapList/N/ctrlMap/N/{k}"), k, &mut e);
                }
                out.push(e);
            }
        }
    }
    out
}

// ---- the count ----------------------------------------------------------------------------------

fn elements(format: Format, side: Side, text: &str, device: Option<&Value>) -> Vec<Element> {
    match format {
        Format::MidiCsv | Format::MidiTriggersCsv => csv_elements(format, text, device),
        Format::ElectraOne => electra_elements(side, text, device),
        Format::Midnam => midnam_elements(side, text, device),
        Format::OscQuery => oscquery_elements(text, device),
        Format::Midi2Controllers => midi2_elements(side, text, device),
        other => panic!("{}: not an importable format", other.name()),
    }
}

/// The count of a file, of what its import and the export of its device gave.
pub fn count_trip(format: Format, text: &str, trip: &Trip, structure: &BTreeSet<String>) -> Count {
    let given = elements(format, Side::Source, text, Some(&trip.device));
    let mut back: BTreeMap<(String, String), std::collections::VecDeque<Element>> = BTreeMap::new();
    for e in elements(format, Side::Export, &trip.exported, None) {
        back.entry((e.shape.clone(), e.key.clone())).or_default().push_back(e);
    }
    let covering: Vec<String> = trip
        .import_lost
        .iter()
        .map(|l| if format == Format::Midnam { midnam_place(l) } else { l.place.clone() })
        .collect();
    let mut count = Count::default();
    for e in given {
        let pair = back.get_mut(&(e.shape.clone(), e.key.clone())).and_then(|l| l.pop_front());
        for v in e.values {
            let figures = count.kinds.entry(v.kind.clone()).or_default();
            if v.structure || structure.contains(&v.kind) {
                figures.structure += 1;
                continue;
            }
            let covered = covering.iter().any(|c| v.places.iter().any(|p| under(p, c)));
            let held = pair.as_ref().is_some_and(|p| p.reads.get(&v.field) == Some(&v.content));
            if !covered && held {
                figures.carried += 1;
                continue;
            }
            figures.lost += 1;
            let named = covered || trip.export_lost.iter().any(|l| e.targets.iter().any(|t| under(&l.place, t) || under(t, &l.place)));
            if !named {
                count.unnamed.push(format!("{} = {:?} at {}", v.kind, v.content, v.places.last().map_or("", String::as_str)));
            }
        }
    }
    count
}

// ---- structure --------------------------------------------------------------------------------

/// The kinds the table of the page of a format gives the fate structure.
pub fn structure_kinds(format: Format) -> BTreeSet<String> {
    let header = match format {
        Format::ElectraOne => "Preset attribute",
        Format::Midnam => "Element",
        Format::OscQuery => "Attribute",
        Format::Midi2Controllers => "Property",
        _ => return BTreeSet::new(),
    };
    let mut out = BTreeSet::new();
    for row in super::fields::table(format.name(), header).into_iter().filter(|r| r.len() > 1 && r[1] == "structure") {
        let name = row[0].as_str();
        match format {
            Format::ElectraOne => {
                out.insert(format!("/{}", name.replace("[]", "/N").replace('.', "/")));
            }
            Format::Midi2Controllers => {
                for at in ["/ChCtrlList/N", "/AllCtrlList/N", "/N", "/CtrlMapList/N"] {
                    out.insert(format!("{at}/{name}"));
                }
            }
            Format::OscQuery => {
                out.insert(format!("/{name}"));
            }
            _ => {
                out.insert(name.to_string());
            }
        }
    }
    out
}

/// The figures of kinds written as (kind, carried, lost, structure).
pub fn figures(rows: &[(&str, usize, usize, usize)]) -> BTreeMap<String, Figures> {
    rows.iter().map(|(k, c, l, s)| (k.to_string(), Figures::of(*c, *l, *s))).collect()
}

/// A namespace, a list or a preset as text, one value per line.
pub fn pretty(v: Value) -> String {
    serde_json::to_string_pretty(&json!(v)).expect("JSON")
}
