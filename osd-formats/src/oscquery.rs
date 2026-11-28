//! OSCQuery, the namespace of the OSC addresses a program answers (github.com/Vidvox/OSCQueryProposal):
//! its importer and its exporter. The rules are the proposal's; the fate of each attribute is in
//! docs/oscquery.md.

use crate::credit::{credit_text, uncarried_source_fields};
use crate::importer::check_source;
use crate::{lose_what_no_way_says, Contract, Fault, FaultKind, Loss, Translated};
use osd::{Address, ArgType, Command, Device, Resolved, Segment, Source, Trigger};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub(crate) const CONTRACT: Contract = Contract {
    name: "oscquery",
    media_type: "application/json",
    extension: "json",
    import,
    export,
    import_into: None,
};

const SCHEMA_URL: &str = "https://w3id.org/osd/0.7/device.schema.json";

/// The type tags of the proposal ("Type tags": the OSC 1.0 and 1.1 tags).
const TAGS: &str = "ifsbhtdScrmTFNI";

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

/// The tags of a TYPE, without the brackets of arrays.
fn tags(ty: &str) -> Vec<char> {
    ty.chars().filter(|c| !matches!(c, '[' | ']')).collect()
}

/// The brackets of a TYPE: a tag, or what stands between two brackets.
enum Shape {
    Tag,
    Array(Vec<Shape>),
}

/// The shapes of a TYPE, `None` when its brackets do not pair.
fn shapes(ty: &str) -> Option<Vec<Shape>> {
    let mut open: Vec<Vec<Shape>> = Vec::new();
    let mut here = Vec::new();
    for c in ty.chars() {
        match c {
            '[' => open.push(std::mem::take(&mut here)),
            ']' => {
                let inner = std::mem::replace(&mut here, open.pop()?);
                here.push(Shape::Array(inner));
            }
            _ => here.push(Shape::Tag),
        }
    }
    open.is_empty().then_some(here)
}

/// What a RANGE or an EXTENDED_TYPE says of one argument, and its place below the attribute
/// (`/1/0`; empty for a value written alone, which holds for each argument).
struct Entry<'a> {
    place: String,
    value: &'a Value,
}

/// The entry of each argument of `ty` in the attribute at `at` (OSCQueryProposal, "Other notes on
/// attributes"): the value itself when it is written alone; else an array holding one entry per
/// tag, or nested as the brackets of the TYPE are.
fn entries<'a>(value: &'a Value, ty: &str, at: &str) -> Result<Vec<Entry<'a>>, Fault> {
    let count = tags(ty).len();
    let Some(items) = value.as_array() else {
        return Ok((0..count).map(|_| Entry { place: String::new(), value }).collect());
    };
    let rule = "one entry per argument of TYPE, in arrays as the brackets of the TYPE when it holds any array";
    if !items.iter().any(Value::is_array) {
        if items.len() != count {
            return Err(invalid(at, rule));
        }
        return Ok(items.iter().enumerate().map(|(i, value)| Entry { place: format!("/{i}"), value }).collect());
    }
    let Some(shapes) = shapes(ty) else {
        return Err(invalid(at, "the brackets of the TYPE do not pair: no array follows them"));
    };
    let mut out = Vec::new();
    follow(items, &shapes, at, "", &mut out)?;
    Ok(out)
}

/// Reads the items of an array beside the shapes of the brackets it stands for.
fn follow<'a>(items: &'a [Value], shapes: &[Shape], at: &str, place: &str, out: &mut Vec<Entry<'a>>) -> Result<(), Fault> {
    if items.len() != shapes.len() {
        return Err(invalid(format!("{at}{place}"), "the array has not the length of the brackets of the TYPE it stands for"));
    }
    for (i, (value, shape)) in items.iter().zip(shapes).enumerate() {
        let place = format!("{place}/{i}");
        match (shape, value.as_array()) {
            (Shape::Tag, None) => out.push(Entry { place, value }),
            (Shape::Array(inner), Some(items)) => follow(items, inner, at, &place, out)?,
            (Shape::Tag, Some(_)) => return Err(invalid(format!("{at}{place}"), "an array where the TYPE has one tag")),
            (Shape::Array(_), None) => return Err(invalid(format!("{at}{place}"), "the TYPE has brackets here: an array is expected")),
        }
    }
    Ok(())
}

/// The rules of one entry of a RANGE: null or an object, numeric bounds, MIN not above MAX.
fn check_bounds(entry: &Value, at: &str) -> Result<(), Fault> {
    if entry.is_null() {
        return Ok(());
    }
    let Some(e) = entry.as_object() else {
        return Err(invalid(at, "a RANGE entry is null or an object"));
    };
    for key in ["MIN", "MAX"] {
        if e.get(key).is_some_and(|v| !v.is_number()) {
            return Err(invalid(format!("{at}/{key}"), format!("{key} is a number")));
        }
    }
    if let (Some(lo), Some(hi)) = (e.get("MIN").and_then(Value::as_f64), e.get("MAX").and_then(Value::as_f64)) {
        if lo > hi {
            return Err(invalid(at, "MIN is above MAX"));
        }
    }
    Ok(())
}

/// Refuses a node that breaks the proposal, at the JSON pointer of the value that breaks it.
fn check_node(node: &Value, path: &str, ptr: &str) -> Result<(), Fault> {
    let Some(o) = node.as_object() else {
        return Err(invalid(ptr, "a node of the namespace is a JSON object"));
    };
    match o.get("FULL_PATH").and_then(Value::as_str) {
        Some(p) if p == path => {}
        Some(p) => return Err(invalid(format!("{ptr}/FULL_PATH"), format!("{p:?} is not the path of the node, {path:?}"))),
        None => return Err(invalid(format!("{ptr}/FULL_PATH"), "a node has its FULL_PATH, a string")),
    }
    let access = match o.get("ACCESS") {
        None => None,
        Some(a) => match a.as_i64() {
            Some(n) if (0..=3).contains(&n) => Some(n),
            _ => return Err(invalid(format!("{ptr}/ACCESS"), "ACCESS is an integer from 0 to 3")),
        },
    };
    let mut types = None;
    if let Some(t) = o.get("TYPE") {
        let Some(s) = t.as_str() else {
            return Err(invalid(format!("{ptr}/TYPE"), "TYPE is a string of type tags"));
        };
        if let Some(c) = s.chars().find(|c| !TAGS.contains(*c) && !matches!(c, '[' | ']')) {
            return Err(invalid(format!("{ptr}/TYPE"), format!("{c:?} is not a type tag of the proposal")));
        }
        if access == Some(0) {
            return Err(invalid(format!("{ptr}/TYPE"), "a container (ACCESS 0) has no type"));
        }
        types = Some(s);
    }
    if let Some(r) = o.get("RANGE") {
        let at = format!("{ptr}/RANGE");
        let form = "RANGE is an object that holds for each argument of TYPE, or an array of the entries of its arguments";
        match (r, types) {
            (Value::Object(_), Some(_)) => check_bounds(r, &at)?,
            (Value::Array(_), Some(ty)) => {
                for entry in entries(r, ty, &at)? {
                    check_bounds(entry.value, &format!("{at}{}", entry.place))?;
                }
            }
            _ => return Err(invalid(at, form)),
        }
    }
    if let (Some(e), Some(ty)) = (o.get("EXTENDED_TYPE"), types) {
        let at = format!("{ptr}/EXTENDED_TYPE");
        match e {
            Value::String(_) => {}
            Value::Array(_) => {
                entries(e, ty, &at)?;
            }
            _ => return Err(invalid(at, "EXTENDED_TYPE is a text that holds for each argument of TYPE, or an array of the names of its arguments")),
        }
    }
    match o.get("CONTENTS") {
        None => {}
        Some(Value::Object(children)) => {
            let base = path.trim_end_matches('/');
            for (k, c) in children {
                check_node(c, &format!("{base}/{k}"), &format!("{ptr}/CONTENTS/{}", segment(k)))?;
            }
        }
        Some(_) => return Err(invalid(format!("{ptr}/CONTENTS"), "CONTENTS is an object of nodes")),
    }
    Ok(())
}

/// What an attribute of a node the descriptor cannot hold is lost for.
fn why_lost(attribute: &str) -> &'static str {
    match attribute {
        "DESCRIPTION" => "a text for people: the descriptor has no field for the description of an address",
        "TAGS" => "the descriptor has no field for the tags of an address",
        "UNIT" => "the descriptor has no field for the unit of a value",
        "CRITICAL" => "the descriptor has no field for whether a change is critical",
        "CLIPMODE" => "the descriptor has no field for what a value out of its range becomes",
        "VALUE" => "the current value of an address is a state, not a description",
        "OVERLOADS" => "the descriptor has no field for the other argument lists of an address",
        _ => "the descriptor has no field for it",
    }
}

/// The attributes of a node that nothing carries.
fn lost_attributes(o: &Map<String, Value>, ptr: &str, losses: &mut Vec<Loss>) {
    for key in o.keys().filter(|k| !["FULL_PATH", "ACCESS", "CONTENTS", "TYPE", "RANGE", "EXTENDED_TYPE"].contains(&k.as_str())) {
        loss(losses, format!("{ptr}/{}", segment(key)), why_lost(key));
    }
}

fn name_ok(n: &str) -> bool {
    let mut chars = n.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn whole(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9e15).map(|f| f as i64))
}

/// Why a VALS is not carried: the descriptor's `values` are labels with their integers, and a VALS
/// is the list of permitted values, with either the labels or the integers missing.
fn vals_loss(vals: &Value) -> &'static str {
    match vals.as_array() {
        None => "VALS is a list of permitted values: this is not a list, nothing of it is carried",
        Some(items) if !items.is_empty() && items.iter().all(|v| whole(v).is_some()) => {
            "a list of permitted integers has no label: the descriptor's values are labels with their integers, none is invented"
        }
        Some(items) if !items.is_empty() && items.iter().all(Value::is_string) => {
            "a list of permitted labels has no integer: the descriptor's values are labels with their integers, none is invented"
        }
        Some(_) => "the list holds neither integers alone nor labels alone: the descriptor's values are labels with their integers, nothing is carried",
    }
}

/// The `osc` of a command from a path: the string itself, or the segments when the path holds
/// captures `{name}`; `None` when a brace is no capture (bad name, unclosed, two side by side).
fn osc_of(path: &str) -> Option<Value> {
    if !path.contains(['{', '}']) {
        return Some(json!(path));
    }
    let mut parts: Vec<Value> = Vec::new();
    let mut text = String::new();
    let mut rest = path;
    while let Some(i) = rest.find(['{', '}']) {
        if rest.as_bytes()[i] == b'}' {
            return None;
        }
        text.push_str(&rest[..i]);
        let close = rest[i..].find('}')? + i;
        let name = &rest[i + 1..close];
        if !name_ok(name) || text.is_empty() && matches!(parts.last(), Some(Value::Object(_))) {
            return None;
        }
        if !text.is_empty() {
            parts.push(json!(std::mem::take(&mut text)));
        }
        parts.push(json!({"capture": name}));
        rest = &rest[close + 1..];
    }
    text.push_str(rest);
    if !text.is_empty() {
        parts.push(json!(text));
    }
    Some(Value::Array(parts))
}

/// The TYPE of a method that takes an event with no value: the Impulse of OSC 1.1.
const IMPULSE: &str = "I";

/// The command a method of type `I` makes: no argument and no frame. The tag is not kept, and what
/// the node says of its argument is named, a RANGE or an EXTENDED_TYPE written alone included.
fn impulse_command(o: &Map<String, Value>, ptr: &str, relative: &str, losses: &mut Vec<Loss>) -> Option<Value> {
    let osc = command_osc(ptr, relative, losses)?;
    loss(losses, format!("{ptr}/TYPE"), "the impulse tag of TYPE is not kept: the command has no argument, and the export of that command writes no TYPE");
    let unbounded = "an impulse has no value: the command has no argument to bound";
    let holds_a_key = |v: &Value| v.as_object().is_some_and(|e| !e.is_empty());
    match o.get("RANGE") {
        Some(Value::Array(entries)) => {
            for (i, _) in entries.iter().enumerate().filter(|(_, e)| holds_a_key(e)) {
                loss(losses, format!("{ptr}/RANGE/{i}"), unbounded);
            }
        }
        Some(alone) if holds_a_key(alone) => loss(losses, format!("{ptr}/RANGE"), unbounded),
        _ => {}
    }
    let unnamed = "an impulse has no value: the command has no argument to name";
    match o.get("EXTENDED_TYPE") {
        Some(Value::Array(entries)) => {
            for i in 0..entries.len() {
                loss(losses, format!("{ptr}/EXTENDED_TYPE/{i}"), unnamed);
            }
        }
        Some(_) => loss(losses, format!("{ptr}/EXTENDED_TYPE"), unnamed),
        None => {}
    }
    Some(json!({"osc": osc}))
}

/// The `osc` of the command of a method; a path with braces that are no capture makes none.
fn command_osc(ptr: &str, relative: &str, losses: &mut Vec<Loss>) -> Option<Value> {
    let osc = osc_of(relative);
    if osc.is_none() {
        loss(losses, ptr, "the path has braces that are no capture ({name}, letters, digits and _, one per piece of text): the method is not imported");
    }
    osc
}

/// The bounds an entry of a RANGE gives its argument; what it holds beside them, and bounds the
/// descriptor cannot hold, are losses at the entry.
fn bounds_read(entry: &Map<String, Value>, at: &str, losses: &mut Vec<Loss>) -> Option<[i64; 2]> {
    for (key, value) in entry.iter().filter(|(k, _)| *k != "MIN" && *k != "MAX") {
        let reason = if key == "VALS" { vals_loss(value) } else { "the descriptor has no field for it" };
        loss(losses, format!("{at}/{}", segment(key)), reason);
    }
    match (entry.get("MIN"), entry.get("MAX")) {
        (Some(lo), Some(hi)) => match (whole(lo), whole(hi)) {
            (Some(lo), Some(hi)) => return Some([lo, hi]),
            _ => loss(losses, at, "the bounds are not whole numbers: the descriptor's range holds whole numbers"),
        },
        (None, None) => {}
        _ => loss(losses, at, "a range needs both its MIN and its MAX"),
    }
    None
}

/// Names an entry the descriptor takes from another place than the one the export, which writes
/// one entry per argument, writes it back at. A value written alone has no place of its own.
fn lose_place(entry: &Entry, argument: usize, at: &str, losses: &mut Vec<Loss>) {
    if !entry.place.is_empty() && entry.place != format!("/{argument}") {
        let attribute = at.rsplit('/').next().unwrap_or_default();
        loss(losses, format!("{at}{}", entry.place), format!("the arrays are not kept: the export writes this entry at {attribute}/{argument}"));
    }
}

/// Names each null of a RANGE the export does not give back at its place: it writes one entry
/// per argument, null for an argument without bounds, and no RANGE when none has any.
fn lose_nulls(entries: &[Entry], args: &[Value], at: &str, losses: &mut Vec<Loss>) {
    let written = args.iter().any(|a| a.get("range").is_some());
    for entry in entries.iter().filter(|e| e.value.is_null()) {
        let argument = entry.place.strip_prefix('/').and_then(|i| i.parse::<usize>().ok());
        let back = written && argument.and_then(|i| args.get(i)).is_some_and(|a| a.get("range").is_none());
        if !back {
            loss(losses, format!("{at}{}", entry.place), "the export does not write this null back at its place: it writes one entry per argument, and no RANGE when no argument has bounds");
        }
    }
}

/// The command a method makes, `None` when its type has no argument type in the descriptor.
fn command(o: &Map<String, Value>, ptr: &str, relative: &str, losses: &mut Vec<Loss>) -> Option<Value> {
    let ty = o.get("TYPE").and_then(Value::as_str).unwrap_or("");
    if ty == IMPULSE {
        return impulse_command(o, ptr, relative, losses);
    }
    if ty.contains(['[', ']']) {
        loss(losses, format!("{ptr}/TYPE"), "the arrays of the type are not kept: their arguments are read one after the other");
    }
    let count = tags(ty).len();
    let (names_at, ranges_at) = (format!("{ptr}/EXTENDED_TYPE"), format!("{ptr}/RANGE"));
    // The check of the node has read both attributes beside the TYPE.
    let of_arguments = |key: &str, at: &str| o.get(key).and_then(|v| entries(v, ty, at).ok()).unwrap_or_default();
    let (names, ranges) = (of_arguments("EXTENDED_TYPE", &names_at), of_arguments("RANGE", &ranges_at));
    // A RANGE written alone is read once, for all the arguments.
    let alone = o.get("RANGE").and_then(Value::as_object).filter(|_| count > 0).and_then(|e| bounds_read(e, &ranges_at, losses));
    // Why a text written alone does not name every argument.
    let mut unnamed = None;
    let mut args = Vec::new();
    let mut used: Vec<String> = Vec::new();
    // The tags the export writes otherwise: several tags read as one argument type.
    let mut rewritten = Vec::new();
    for (i, tag) in tags(ty).into_iter().enumerate() {
        let (kind, read_as) = match tag {
            'i' | 'h' => ("int", ArgType::Int),
            'f' | 'd' => ("float", ArgType::Float),
            's' | 'S' | 'c' => ("string", ArgType::String),
            'T' | 'F' => ("bool", ArgType::Bool),
            other => {
                loss(losses, format!("{ptr}/TYPE"), format!("the tag {other:?} has no argument type in the descriptor: the method is not imported"));
                return None;
            }
        };
        if tag_of(read_as) != tag {
            rewritten.push(format!("{tag} comes back as {}", tag_of(read_as)));
        }
        let mut name = format!("arg{}", i + 1);
        if let Some(entry) = names.get(i) {
            match entry.value.as_str() {
                Some(n) if name_ok(n) && !used.iter().any(|u| u == n) => {
                    name = n.to_string();
                    lose_place(entry, i, &names_at, losses);
                }
                Some(n) => {
                    let reason = format!("{n:?} is no name for this argument (letters, digits and _, each once): {name} is written");
                    if entry.place.is_empty() {
                        unnamed.get_or_insert(reason);
                    } else {
                        loss(losses, format!("{names_at}{}", entry.place), reason);
                    }
                }
                // A value written alone is a text: this entry has a place in an array.
                None => loss(losses, format!("{names_at}{}", entry.place), format!("an entry that is no text names no argument: {name} is written")),
            }
        }
        used.push(name.clone());
        let mut arg = Map::new();
        arg.insert("name".into(), json!(name));
        arg.insert("type".into(), json!(kind));
        if let Some(entry) = ranges.get(i) {
            let range = if entry.place.is_empty() {
                alone
            } else {
                let at = format!("{ranges_at}{}", entry.place);
                let range = entry.value.as_object().and_then(|e| bounds_read(e, &at, losses));
                if range.is_some() {
                    lose_place(entry, i, &ranges_at, losses);
                }
                range
            };
            if let Some(range) = range {
                arg.insert("range".into(), json!(range));
            }
        }
        args.push(Value::Object(arg));
    }
    lose_nulls(&ranges, &args, &ranges_at, losses);
    if count == 0 {
        if o.contains_key("TYPE") {
            loss(losses, format!("{ptr}/TYPE"), "a TYPE of no tag gives no argument: the export writes no TYPE");
        }
        for (key, at) in [("EXTENDED_TYPE", &names_at), ("RANGE", &ranges_at)] {
            if o.contains_key(key) {
                loss(losses, at.as_str(), "the TYPE has no argument that takes it");
            }
        }
    } else if let Some(reason) = unnamed {
        loss(losses, names_at, reason);
    }
    let osc = command_osc(ptr, relative, losses)?;
    if !rewritten.is_empty() {
        loss(losses, format!("{ptr}/TYPE"), format!("the descriptor does not keep these type tags, and the export writes others: {}", rewritten.join(", ")));
    }
    let mut c = Map::new();
    c.insert("osc".into(), osc);
    if !args.is_empty() {
        c.insert("args".into(), Value::Array(args));
    }
    Some(Value::Object(c))
}

fn visit(node: &Value, ptr: &str, prefix: &str, commands: &mut Vec<Value>, losses: &mut Vec<Loss>) {
    let Some(o) = node.as_object() else { return };
    lost_attributes(o, ptr, losses);
    let has_contents = o.contains_key("CONTENTS");
    match o.get("ACCESS").and_then(Value::as_i64) {
        Some(2 | 3) => {
            if o["ACCESS"] == 3 {
                loss(losses, format!("{ptr}/ACCESS"), "the read side of a method (its value, its changes) is not carried: only the command that writes it is");
            }
            let full = o.get("FULL_PATH").and_then(Value::as_str).unwrap_or("");
            if let Some(c) = command(o, ptr, full.strip_prefix(prefix).unwrap_or(full), losses) {
                commands.push(c);
            }
        }
        Some(1) => loss(losses, ptr, "a read-only method sends nothing: it is no command"),
        Some(_) => {}
        None if !has_contents => loss(losses, format!("{ptr}/ACCESS"), "the node has no ACCESS: it is neither a container nor a method"),
        None => {}
    }
    let before = commands.len();
    if let Some(children) = o.get("CONTENTS").and_then(Value::as_object) {
        for (k, c) in children {
            visit(c, &format!("{ptr}/CONTENTS/{}", segment(k)), prefix, commands, losses);
        }
    }
    let container = match o.get("ACCESS").and_then(Value::as_i64) {
        Some(access) => access == 0,
        None => has_contents,
    };
    if container && commands.len() == before {
        loss(losses, ptr, "no method of this container gives a command: the export, which writes the containers of the commands, does not write it");
    }
}

fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let root: Value = serde_json::from_str(text).map_err(|e| invalid("", format!("the file is not valid JSON: {e}")))?;
    check_node(&root, "/", "")?;
    let top = root.get("CONTENTS").and_then(Value::as_object);
    let one = top.filter(|t| t.len() == 1).and_then(|t| t.iter().next());
    let Some((key, device)) = one else {
        return Err(invalid("/CONTENTS", "a namespace of one device has one container at its root: its name and its address prefix"));
    };
    let is_container = device["ACCESS"].as_i64().is_none_or(|a| a == 0);
    if !is_container {
        return Err(invalid("/CONTENTS", "the node at the root is a method: a device's namespace is a container"));
    }
    let prefix = format!("/{key}");
    let top_ptr = format!("/CONTENTS/{}", segment(key));
    let mut losses = Vec::new();
    if let Some(o) = root.as_object() {
        lost_attributes(o, "", &mut losses);
    }
    if let Some(o) = device.as_object() {
        lost_attributes(o, &top_ptr, &mut losses);
    }
    let mut commands = Vec::new();
    if let Some(children) = device.get("CONTENTS").and_then(Value::as_object) {
        for (k, c) in children {
            visit(c, &format!("{top_ptr}/CONTENTS/{}", segment(k)), &prefix, &mut commands, &mut losses);
        }
    }
    let mut doc = Map::new();
    doc.insert("$schema".into(), json!(SCHEMA_URL));
    doc.insert("device".into(), json!({"name": key, "osc_prefix": prefix}));
    if !commands.is_empty() {
        doc.insert("commands".into(), Value::Array(commands));
    }
    let read = osd::read(&Value::Object(doc).to_string(), osd::Kind::Device)
        .map_err(|f| invalid(top_ptr.clone(), format!("the namespace does not make a device descriptor: {} at {}", f.rule, f.pointer)))?;
    let osd::Descriptor::Device(mut d) = read else {
        return Err(invalid("", "the import does not make a device"));
    };
    d.sources = Some(vec![source.clone()]);
    Ok(Translated { value: *d, lost: losses })
}

// ---- export -----------------------------------------------------------------------------

fn container(path: &str) -> Value {
    json!({"FULL_PATH": path, "ACCESS": 0, "CONTENTS": {}})
}

/// The RANGE entry of an argument: its bounds and the list of its permitted values.
#[derive(Default)]
struct Bounds {
    range: Option<[i64; 2]>,
    vals: Option<Vec<Value>>,
}

impl Bounds {
    fn node(&self) -> Value {
        let mut o = Map::new();
        if let Some([lo, hi]) = self.range {
            o.insert("MIN".into(), json!(lo));
            o.insert("MAX".into(), json!(hi));
        }
        if let Some(v) = &self.vals {
            o.insert("VALS".into(), Value::Array(v.clone()));
        }
        if o.is_empty() {
            Value::Null
        } else {
            Value::Object(o)
        }
    }
}

/// The bounds a range gives, or else those MIDI fixes for the kind of controller; a lowest value
/// above the highest is lost.
fn bounds_of(range: Option<[i64; 2]>, fixed: Option<i64>, at: &str, losses: &mut Vec<Loss>) -> Option<[i64; 2]> {
    match range {
        Some([lo, hi]) if lo > hi => {
            loss(losses, format!("{at}/range"), "the lowest value is above the highest");
            None
        }
        Some(r) => Some(r),
        None => fixed.map(|max| [0, max]),
    }
}

/// The highest value MIDI fixes for the kind of a controller: 127 for a 7-bit controller, 16383
/// for a 14-bit one, an NRPN or an RPN; none for an entry of no such kind.
fn span_of(way: &osd::Way) -> Option<i64> {
    match (way.ctrl_type.as_str(), way.ctrl_index.as_ref().map(Vec::len)) {
        ("cc", Some(1)) => Some(127),
        ("cc" | "nrpn" | "rpn", _) => Some(16383),
        _ => None,
    }
}

/// The way a namespace carries of a control, the first in the order written (its range is the
/// control's), and the named values of the control as that way carries them; every other way is a
/// loss at its place.
fn carried_way<'a>(ways: &'a [osd::Way], values: Option<&osd::NamedValues>, entry: &impl Scaled, at: &str, losses: &mut Vec<Loss>) -> (Option<&'a osd::Way>, Option<BTreeMap<String, i64>>) {
    for m in 1..ways.len() {
        loss(losses, format!("{at}/ways/{m}"), "a namespace says one range per address: the first way gives it, the other ways name messages it does not say");
    }
    let first = ways.first();
    let named = values.map(|v| v.iter().map(|n| (n.title.clone(), first.map_or(n.value, |w| entry.in_way(n.value, w)))).collect());
    (first, named)
}

/// The fields of an entry with no way that the node does not carry: its `range` alone is the node's.
fn lost_without_a_way(fields: &[(&str, bool)], at: &str, losses: &mut Vec<Loss>) {
    for (field, _) in fields.iter().filter(|(_, present)| *present) {
        loss(losses, format!("{at}/{field}"), "the entry gives no message: the namespace says its range alone");
    }
}

/// A control that converts a value of its reference scale to the scale of one of its ways.
trait Scaled {
    fn in_way(&self, value: i64, way: &osd::Way) -> i64;
}

impl Scaled for osd::CcEntry {
    fn in_way(&self, value: i64, way: &osd::Way) -> i64 {
        osd::CcEntry::in_way(self, value, way)
    }
}

impl Scaled for osd::ParamEntry {
    fn in_way(&self, value: i64, way: &osd::Way) -> i64 {
        osd::ParamEntry::in_way(self, value, way)
    }
}

/// The VALS of the named values of an integer argument (its integers, ascending, each once) or of
/// a text argument (its labels, ascending); the loss says what the list leaves out.
fn vals_of(values: &BTreeMap<String, i64>, text: bool, at: &str, losses: &mut Vec<Loss>) -> Vec<Value> {
    if text {
        loss(losses, at, "the integers behind the labels have no place in a VALS of labels");
        return values.keys().map(|l| json!(l)).collect();
    }
    loss(losses, at, "the labels have no place in a VALS of integers: it is the list of permitted values");
    let mut numbers: Vec<i64> = values.values().copied().collect();
    numbers.sort_unstable();
    numbers.dedup();
    numbers.into_iter().map(|n| json!(n)).collect()
}

/// A method node: ACCESS 2 (written), its tags, its ranges and the names of its arguments.
fn method(path: &str, types: &str, ranges: &[Bounds], names: Option<Vec<String>>) -> Value {
    let mut m = Map::new();
    m.insert("FULL_PATH".into(), json!(path));
    m.insert("ACCESS".into(), json!(2));
    if !types.is_empty() {
        m.insert("TYPE".into(), json!(types));
        if ranges.iter().any(|b| b.range.is_some() || b.vals.is_some()) {
            m.insert("RANGE".into(), Value::Array(ranges.iter().map(Bounds::node).collect()));
        }
        if let Some(n) = names {
            m.insert("EXTENDED_TYPE".into(), json!(n));
        }
    }
    Value::Object(m)
}

/// Puts `node` at `address` under the device node, creating the containers on the way; `false`
/// when a method is already on the way or at the place.
fn place(device: &mut Value, prefix: &str, address: &str, node: Value) -> bool {
    let segs: Vec<&str> = address.split('/').filter(|s| !s.is_empty()).collect();
    let Some((last, parents)) = segs.split_last() else { return false };
    let mut here = device;
    let mut path = prefix.trim_end_matches('/').to_string();
    for s in parents {
        path = format!("{path}/{s}");
        if here["ACCESS"].as_i64().is_some_and(|a| a > 0) {
            return false;
        }
        let contents = here["CONTENTS"].as_object_mut().expect("a container has its CONTENTS");
        let child = contents.entry((*s).to_string()).or_insert_with(|| container(&path));
        here = child;
    }
    if here["ACCESS"].as_i64().is_some_and(|a| a > 0) {
        return false;
    }
    let contents = here["CONTENTS"].as_object_mut().expect("a container has its CONTENTS");
    if contents.contains_key(*last) {
        return false;
    }
    contents.insert((*last).to_string(), node);
    true
}

/// The address as the namespace writes it: each capture stays `{name}` in its segment.
fn address_text(a: &Address) -> String {
    match a {
        Address::Text(s) => s.clone(),
        Address::Segments(parts) => parts
            .iter()
            .map(|p| match p {
                Segment::Text(t) => t.clone(),
                Segment::Capture(n) => format!("{{{n}}}"),
            })
            .collect(),
    }
}

fn tag_of(t: ArgType) -> char {
    match t {
        ArgType::Float => 'f',
        ArgType::Bool => 'T',
        ArgType::String | ArgType::Enum => 's',
        _ => 'i',
    }
}

fn command_node(n: usize, c: &Command, prefix: &str, losses: &mut Vec<Loss>) -> Option<(String, Value)> {
    let at = format!("/commands/{n}");
    let address = address_text(&c.osc);
    for (field, present) in [
        ("title", c.title.is_some()),
        ("description", c.description.is_some()),
        ("frame", c.frame.is_some()),
        ("forward", c.forward.is_some()),
        ("pre", c.pre.is_some()),
        ("script", c.script.is_some()),
        ("notes", c.notes.is_some()),
        ("section", c.section.is_some()),
    ] {
        if present {
            loss(losses, format!("{at}/{field}"), "a namespace says what an address takes, not what it sends");
        }
    }
    for key in c.remarks.keys() {
        loss(losses, format!("{at}/{}", segment(key)), "the namespace has no attribute for it");
    }
    let args = c.args.as_deref().unwrap_or_default();
    let mut ranges = Vec::new();
    for (m, a) in args.iter().enumerate() {
        let here = format!("{at}/args/{m}");
        if a.default.is_some() {
            loss(losses, format!("{here}/default"), "the namespace holds the current value, not a default");
        }
        for key in a.remarks.keys() {
            loss(losses, format!("{here}/{}", segment(key)), "the namespace has no attribute for it");
        }
        let vals = match (&a.values, a.ty) {
            (Some(_), ArgType::Float | ArgType::Bool) => {
                loss(losses, format!("{here}/values"), "a VALS holds the integers of an integer argument or the labels of a text argument: this argument is neither");
                None
            }
            (Some(v), ty) => Some(vals_of(v, matches!(ty, ArgType::String | ArgType::Enum), &format!("{here}/values"), losses)),
            (None, _) => None,
        };
        ranges.push(Bounds { range: bounds_of(a.range, None, &here, losses), vals });
    }
    let types: String = args.iter().map(|a| tag_of(a.ty)).collect();
    let names = (!args.is_empty()).then(|| args.iter().map(|a| a.name.clone()).collect());
    let full = format!("{}{address}", prefix.trim_end_matches('/'));
    Some((address, method(&full, &types, &ranges, names)))
}

/// The method of a trigger, an event with no value; every other field of the entry is named at
/// its place. The descriptor has no field for the arguments of a trigger: none is written.
fn trigger_node(n: usize, t: &Trigger, prefix: &str, losses: &mut Vec<Loss>) -> Value {
    let at = format!("/triggers/entries/{n}");
    let [low, high] = t.note_range;
    let note_range = if t.note.is_none() && low < high {
        "the namespace has no attribute for it: the method does not say which note of the range to play"
    } else {
        "the namespace has no attribute for it"
    };
    for (field, present, reason) in [
        ("title", t.title.is_some(), "the namespace has no attribute for it"),
        ("description", t.description.is_some(), "the namespace has no attribute for it"),
        ("sound", true, "the namespace has no attribute for it: the import reads the method back as a command, not as a trigger"),
        ("section", t.section.is_some(), "the namespace has no attribute for it"),
        ("channel", t.channel.is_some(), "the namespace has no attribute for it"),
        ("channel_mode", t.channel_mode.is_some(), "the namespace has no attribute for it"),
        ("note_range", true, note_range),
        ("note", t.note.is_some(), "the namespace has no attribute for it"),
        ("velocity_usage", t.velocity_usage.is_some(), "the namespace has no attribute for it"),
        ("comments", t.comments.is_some(), "the namespace has no attribute for it"),
    ] {
        if present {
            loss(losses, format!("{at}/{field}"), reason);
        }
    }
    for key in t.remarks.keys() {
        loss(losses, format!("{at}/{}", segment(key)), "the namespace has no attribute for it");
    }
    let full = format!("{}{}", prefix.trim_end_matches('/'), t.osc);
    method(&full, IMPULSE, &[], None)
}

fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    let Some(sources) = d.sources.as_ref().filter(|s| !s.is_empty()) else {
        return Err(fault(FaultKind::NoSources, "/sources", "a device without sources is not written: its credit and licence cannot be"));
    };
    let mut losses = Vec::new();
    let info = &d.device;
    for (pointer, present) in [
        ("/device/revision", info.revision.is_some()),
        ("/identity/manufacturerId", d.identity.as_ref().is_some_and(|i| i.manufacturer_id.is_some())),
        ("/device/device_id", info.device_id.is_some()),
        ("/device/rate_limit_hz", info.rate_limit_hz.is_some()),
        ("/device/kind", info.kind.is_some()),
        ("/device/transport", info.transport.is_some()),
    ] {
        if present {
            loss(&mut losses, pointer, "the namespace has no attribute for it");
        }
    }
    for key in info.remarks.keys() {
        loss(&mut losses, format!("/device/{}", segment(key)), "the namespace has no attribute for it");
    }
    for (pointer, present) in [
        ("/capabilities", d.capabilities.is_some()),
        ("/role_vocabulary", d.role_vocabulary.is_some()),
        ("/midi_of", d.midi_of.is_some()),
        ("/loads_sounds_of", d.loads_sounds_of.is_some()),
        ("/identity", d.identity.is_some()),
        ("/programs", d.programs.is_some()),
        ("/sysex", d.sysex.is_some()),
        ("/midi_out", d.midi_out.is_some()),
        ("/midi_in", d.midi_in.is_some()),
        ("/replies", d.replies.is_some()),
        ("/subscriptions", d.subscriptions.is_some()),
        ("/custom_commands", d.custom_commands.is_some()),
        ("/_coverage", d.coverage.is_some()),
    ] {
        if present {
            loss(&mut losses, pointer, "a namespace lists the addresses a program answers: it holds nothing of this");
        }
    }
    for key in d.remarks.keys() {
        loss(&mut losses, format!("/{}", segment(key)), "the namespace has no attribute for it");
    }
    for (n, s) in sources.iter().enumerate() {
        for field in uncarried_source_fields(s) {
            loss(&mut losses, format!("/sources/{n}/{}", segment(&field)), "the DESCRIPTION of the device node carries the credit and the licence of a source, nothing more");
        }
    }

    // The device node, with the credit of its sources.
    let prefix = info.osc_prefix.as_str();
    let mut description = info.name.clone();
    if let Some(v) = info.vendor.as_ref().filter(|v| !v.is_empty()) {
        description.push_str(&format!(" by {v}"));
    }
    description.push_str(&format!(". Source: {}", credit_text(sources)));
    let mut root = container("/");
    let mut device_node = container(prefix);
    device_node["DESCRIPTION"] = json!(description);
    {
        let segs: Vec<&str> = prefix.split('/').filter(|s| !s.is_empty()).collect();
        let Some((last, parents)) = segs.split_last() else {
            // A prefix of no segment: the device is the root.
            root["DESCRIPTION"] = json!(description);
            return finish(root, d, losses);
        };
        let mut here = &mut root;
        let mut path = String::new();
        for s in parents {
            path = format!("{path}/{s}");
            let c = here["CONTENTS"].as_object_mut().expect("a container has its CONTENTS");
            here = c.entry((*s).to_string()).or_insert_with(|| container(&path));
        }
        here["CONTENTS"].as_object_mut().expect("a container has its CONTENTS").insert((*last).to_string(), device_node);
    }
    finish(root, d, losses)
}

/// The methods of the device under its prefix, then the text.
fn finish(mut root: Value, d: &Device, mut losses: Vec<Loss>) -> Result<Translated<String>, Fault> {
    let prefix = d.device.osc_prefix.as_str();
    let mut device_at = &mut root;
    for s in prefix.split('/').filter(|s| !s.is_empty()) {
        device_at = &mut device_at["CONTENTS"][s];
    }
    let device = device_at;
    let put = |device: &mut Value, at: &str, address: &str, node: Value, losses: &mut Vec<Loss>| {
        if !place(device, prefix, address, node) {
            loss(losses, at, "the address is already taken by another node of the namespace");
        }
    };
    if let Some(cc) = &d.cc_params {
        if cc.channel.is_some() {
            loss(&mut losses, "/cc_params/channel", "the namespace has no attribute for a channel");
        }
        for key in cc.remarks.keys() {
            loss(&mut losses, format!("/cc_params/{}", segment(key)), "the namespace has no attribute for it");
        }
        for (n, e) in cc.entries.iter().enumerate() {
            let at = format!("/cc_params/entries/{n}");
            lose_what_no_way_says(e, n, &mut losses);
            for (field, present) in [
                ("title", e.title.is_some()),
                ("description", e.description.is_some()),
                ("value_description", e.value_description.is_some()),
                ("notes", e.notes.is_some()),
                ("default", e.default.is_some()),
                ("section", e.section.is_some()),
                ("expects", e.expects.is_some()),
                ("polarity", e.polarity.is_some()),
                ("channel", e.channel.is_some()),
                ("transform", e.transform.is_some()),
                ("transform_reverse", e.transform_reverse.is_some()),
                ("role", e.role.is_some()),
            ] {
                if present {
                    loss(&mut losses, format!("{at}/{field}"), "the namespace has no attribute for it");
                }
            }
            for key in e.remarks.keys() {
                loss(&mut losses, format!("{at}/{}", segment(key)), "the namespace has no attribute for it");
            }
            let (first, named) = carried_way(&e.ways, e.values.as_ref(), e, &at, &mut losses);
            let range = match first {
                Some(w) => bounds_of(w.range, span_of(w), &format!("{at}/ways/0"), &mut losses),
                None => {
                    let fields = [("scale", e.scale.is_some()), ("transmit", e.transmit.is_some()), ("recognize", e.recognize.is_some()), ("nrpn_lsb_range", e.nrpn_lsb_range.is_some())];
                    lost_without_a_way(&fields, &at, &mut losses);
                    bounds_of(e.range, None, &at, &mut losses)
                }
            };
            let vals = named.as_ref().map(|v| vals_of(v, false, &format!("{at}/values"), &mut losses));
            let full = format!("{}{}", prefix.trim_end_matches('/'), e.osc);
            put(device, &at, &e.osc, method(&full, "i", &[Bounds { range, vals }], None), &mut losses);
        }
    }
    if let Some(p) = &d.params {
        for field in ["get_frame", "set_frame"] {
            loss(&mut losses, format!("/params/{field}"), "a namespace says what an address takes, not what it sends");
        }
        for key in p.remarks.keys() {
            loss(&mut losses, format!("/params/{}", segment(key)), "the namespace has no attribute for it");
        }
        for (n, e) in p.entries.iter().enumerate() {
            let at = format!("/params/entries/{n}");
            for (field, present) in [
                ("title", e.title.is_some()),
                ("description", e.description.is_some()),
                ("default", e.default.is_some()),
                ("section", e.section.is_some()),
                ("expects", e.expects.is_some()),
                ("transform", e.transform.is_some()),
                ("transform_reverse", e.transform_reverse.is_some()),
                ("role", e.role.is_some()),
            ] {
                if present {
                    loss(&mut losses, format!("{at}/{field}"), "the namespace has no attribute for it");
                }
            }
            for key in e.remarks.keys() {
                loss(&mut losses, format!("{at}/{}", segment(key)), "the namespace has no attribute for it");
            }
            let (first, named) = carried_way(&e.ways, e.values.as_ref(), e, &at, &mut losses);
            let range = match first {
                Some(w) => bounds_of(w.range, None, &format!("{at}/ways/0"), &mut losses),
                None => {
                    let fields = [("scale", e.scale.is_some()), ("transmit", e.transmit.is_some()), ("recognize", e.recognize.is_some())];
                    lost_without_a_way(&fields, &at, &mut losses);
                    bounds_of(e.range, None, &at, &mut losses)
                }
            };
            let vals = named.as_ref().map(|v| vals_of(v, false, &format!("{at}/values"), &mut losses));
            let full = format!("{}{}", prefix.trim_end_matches('/'), e.osc);
            put(device, &at, &e.osc, method(&full, "i", &[Bounds { range, vals }], None), &mut losses);
        }
    }
    for (n, c) in d.commands.iter().flatten().enumerate() {
        if let Some((address, node)) = command_node(n, c, prefix, &mut losses) {
            put(device, &format!("/commands/{n}"), &address, node, &mut losses);
        }
    }
    // The triggers come last: an address a control, a parameter or a command takes stays theirs.
    if let Some(triggers) = &d.triggers {
        for key in triggers.remarks.keys() {
            loss(&mut losses, format!("/triggers/{}", segment(key)), "the namespace has no attribute for it");
        }
        for (n, t) in triggers.entries.iter().enumerate() {
            let node = trigger_node(n, t, prefix, &mut losses);
            put(device, &format!("/triggers/entries/{n}"), &t.osc, node, &mut losses);
        }
    }
    let mut text = serde_json::to_string_pretty(&root).expect("a namespace is JSON");
    text.push('\n');
    Ok(Translated { value: text, lost: losses })
}
