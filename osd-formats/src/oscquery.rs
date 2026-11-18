//! OSCQuery, the namespace of the OSC addresses a program answers (github.com/Vidvox/OSCQueryProposal):
//! its importer and its exporter. The rules are the proposal's; the fate of each attribute is in
//! docs/oscquery.md.

use crate::credit::{credit_text, uncarried_source_fields};
use crate::midi_csv::check_source;
use crate::{Fault, FaultKind, Loss, Translated};
use osd::{Address, ArgType, Command, Device, Resolved, Segment, Source};
use serde_json::{json, Map, Value};

const SCHEMA_URL: &str = "https://w3id.org/osd/0.6/device.schema.json";

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
        types = Some(tags(s));
    }
    if let Some(r) = o.get("RANGE") {
        let (Some(items), Some(t)) = (r.as_array(), &types) else {
            return Err(invalid(format!("{ptr}/RANGE"), "RANGE is an array with one entry per argument of TYPE"));
        };
        if items.len() != t.len() {
            return Err(invalid(format!("{ptr}/RANGE"), "RANGE has one entry per argument of TYPE"));
        }
        for (i, it) in items.iter().enumerate() {
            if it.is_null() {
                continue;
            }
            let Some(e) = it.as_object() else {
                return Err(invalid(format!("{ptr}/RANGE/{i}"), "a RANGE entry is null or an object"));
            };
            for key in ["MIN", "MAX"] {
                if e.get(key).is_some_and(|v| !v.is_number()) {
                    return Err(invalid(format!("{ptr}/RANGE/{i}/{key}"), format!("{key} is a number")));
                }
            }
            if let (Some(lo), Some(hi)) = (e.get("MIN").and_then(Value::as_f64), e.get("MAX").and_then(Value::as_f64)) {
                if lo > hi {
                    return Err(invalid(format!("{ptr}/RANGE/{i}"), "MIN is above MAX"));
                }
            }
        }
    }
    if let (Some(e), Some(t)) = (o.get("EXTENDED_TYPE"), &types) {
        if e.as_array().map(Vec::len) != Some(t.len()) {
            return Err(invalid(format!("{ptr}/EXTENDED_TYPE"), "EXTENDED_TYPE has one entry per argument of TYPE"));
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

/// The command a method makes, `None` when its type has no argument type in the descriptor.
fn command(o: &Map<String, Value>, ptr: &str, relative: &str, losses: &mut Vec<Loss>) -> Option<Value> {
    let ty = o.get("TYPE").and_then(Value::as_str).unwrap_or("");
    if ty.contains(['[', ']']) {
        loss(losses, format!("{ptr}/TYPE"), "the arrays of the type are not kept: their arguments are read one after the other");
    }
    let mut args = Vec::new();
    let mut used: Vec<String> = Vec::new();
    for (i, tag) in tags(ty).into_iter().enumerate() {
        let kind = match tag {
            'i' | 'h' => "int",
            'f' | 'd' => "float",
            's' | 'S' | 'c' => "string",
            'T' | 'F' => "bool",
            other => {
                loss(losses, format!("{ptr}/TYPE"), format!("the tag {other:?} has no argument type in the descriptor: the method is not imported"));
                return None;
            }
        };
        let mut name = format!("arg{}", i + 1);
        if let Some(n) = o.get("EXTENDED_TYPE").and_then(|e| e.get(i)).and_then(Value::as_str) {
            if name_ok(n) && !used.iter().any(|u| u == n) {
                name = n.to_string();
            } else {
                loss(losses, format!("{ptr}/EXTENDED_TYPE/{i}"), format!("{n:?} is no name for an argument (letters, digits and _, each once): {name} is written"));
            }
        }
        used.push(name.clone());
        let mut arg = Map::new();
        arg.insert("name".into(), json!(name));
        arg.insert("type".into(), json!(kind));
        if let Some(entry) = o.get("RANGE").and_then(|r| r.get(i)).and_then(Value::as_object) {
            let at = format!("{ptr}/RANGE/{i}");
            for key in entry.keys().filter(|k| *k != "MIN" && *k != "MAX") {
                loss(losses, format!("{at}/{}", segment(key)), if key == "VALS" { "the descriptor has no field for the named values of a range" } else { "the descriptor has no field for it" });
            }
            match (entry.get("MIN"), entry.get("MAX")) {
                (Some(lo), Some(hi)) => match (whole(lo), whole(hi)) {
                    (Some(lo), Some(hi)) => {
                        arg.insert("range".into(), json!([lo, hi]));
                    }
                    _ => loss(losses, at, "the bounds are not whole numbers: the descriptor's range holds whole numbers"),
                },
                (None, None) => {}
                _ => loss(losses, at, "a range needs both its MIN and its MAX"),
            }
        }
        args.push(Value::Object(arg));
    }
    let mut c = Map::new();
    c.insert("osc".into(), json!(relative));
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
    if let Some(children) = o.get("CONTENTS").and_then(Value::as_object) {
        for (k, c) in children {
            visit(c, &format!("{ptr}/CONTENTS/{}", segment(k)), prefix, commands, losses);
        }
    }
}

pub(crate) fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
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

/// A method node: ACCESS 2 (written), its tags, its ranges and the names of its arguments.
fn method(path: &str, types: &str, ranges: &[Option<[i64; 2]>], names: Option<Vec<String>>) -> Value {
    let mut m = Map::new();
    m.insert("FULL_PATH".into(), json!(path));
    m.insert("ACCESS".into(), json!(2));
    if !types.is_empty() {
        m.insert("TYPE".into(), json!(types));
        if ranges.iter().any(Option::is_some) {
            m.insert("RANGE".into(), Value::Array(ranges.iter().map(|r| r.map_or(Value::Null, |[lo, hi]| json!({"MIN": lo, "MAX": hi}))).collect()));
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

fn address_text(a: &Address) -> Option<String> {
    match a {
        Address::Text(s) => Some(s.clone()),
        Address::Segments(parts) => parts
            .iter()
            .map(|p| match p {
                Segment::Text(t) => Some(t.as_str()),
                Segment::Capture(_) => None,
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
    let Some(address) = address_text(&c.osc) else {
        loss(losses, at, "the address has a capture: it is a pattern of addresses, not one address of a namespace");
        return None;
    };
    for (field, present) in [
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
        if a.values.is_some() {
            loss(losses, format!("{here}/values"), "the named values of an argument are the VALS of a range, which the export does not write");
        }
        if a.default.is_some() {
            loss(losses, format!("{here}/default"), "the namespace holds the current value, not a default");
        }
        for key in a.remarks.keys() {
            loss(losses, format!("{here}/{}", segment(key)), "the namespace has no attribute for it");
        }
        ranges.push(match a.range {
            Some([lo, hi]) if lo > hi => {
                loss(losses, format!("{here}/range"), "the lowest value is above the highest");
                None
            }
            r => r,
        });
    }
    let types: String = args.iter().map(|a| tag_of(a.ty)).collect();
    let names = (!args.is_empty()).then(|| args.iter().map(|a| a.name.clone()).collect());
    let full = format!("{}{address}", prefix.trim_end_matches('/'));
    Some((address, method(&full, &types, &ranges, names)))
}

pub(crate) fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    let Some(sources) = d.sources.as_ref().filter(|s| !s.is_empty()) else {
        return Err(fault(FaultKind::NoSources, "/_sources", "a device without sources is not written: its credit and licence cannot be"));
    };
    let mut losses = Vec::new();
    let info = &d.device;
    for (pointer, present) in [
        ("/device/revision", info.revision.is_some()),
        ("/device/manufacturer_id", info.manufacturer_id.is_some()),
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
        ("/triggers", d.triggers.is_some()),
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
            loss(&mut losses, format!("/_sources/{n}/{}", segment(&field)), "the DESCRIPTION of the device node carries the credit and the licence of a source, nothing more");
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
            for (field, present) in [
                ("default", e.default.is_some()),
                ("section", e.section.is_some()),
                ("expects", e.expects.is_some()),
                ("orientation", e.orientation.is_some()),
                ("channel", e.channel.is_some()),
                ("transform", e.transform.is_some()),
                ("transform_reverse", e.transform_reverse.is_some()),
                ("role", e.role.is_some()),
                ("nrpn_lsb_range", e.nrpn_lsb_range.is_some()),
            ] {
                if present {
                    loss(&mut losses, format!("{at}/{field}"), "the namespace has no attribute for it");
                }
            }
            for key in e.remarks.keys() {
                loss(&mut losses, format!("{at}/{}", segment(key)), "the namespace has no attribute for it");
            }
            let range = match e.range {
                Some([lo, hi]) if lo > hi => {
                    loss(&mut losses, format!("{at}/range"), "the lowest value is above the highest");
                    None
                }
                r => r,
            };
            let full = format!("{}{}", prefix.trim_end_matches('/'), e.osc);
            put(device, &at, &e.osc, method(&full, "i", &[range], None), &mut losses);
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
                ("default", e.default.is_some()),
                ("section", e.section.is_some()),
                ("expects", e.expects.is_some()),
                ("values", e.values.is_some()),
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
            let range = match e.range {
                Some([lo, hi]) if lo > hi => {
                    loss(&mut losses, format!("{at}/range"), "the lowest value is above the highest");
                    None
                }
                r => r,
            };
            let full = format!("{}{}", prefix.trim_end_matches('/'), e.osc);
            put(device, &at, &e.osc, method(&full, "i", &[range], None), &mut losses);
        }
    }
    for (n, c) in d.commands.iter().flatten().enumerate() {
        if let Some((address, node)) = command_node(n, c, prefix, &mut losses) {
            put(device, &format!("/commands/{n}"), &address, node, &mut losses);
        }
    }
    let mut text = serde_json::to_string_pretty(&root).expect("a namespace is JSON");
    text.push('\n');
    Ok(Translated { value: text, lost: losses })
}
