//! What the MIDNAM tests share: the table of the verifier of an export, written from the DTD of
//! the MIDI Association (tests/data/MIDINameDocument10.dtd and MIDIEvents10.dtd, the file it
//! includes), the verifier that reads it, the declarations of the DTD files, and the documents
//! and devices the tests build.

#![allow(dead_code)]

use super::xml::{self, Node};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const DTD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/MIDINameDocument10.dtd");
pub const EVENTS_DTD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/MIDIEvents10.dtd");
pub const PUBLIC_ID: &str = "-//MIDI Manufacturers Association//DTD MIDINameDocument 1.0//EN";

const CH: &str = "Channel (1|2|3|4|5|6|7|8|9|10|11|12|13|14|15|16) %ChannelRequired;";
const REQUIRED_NAME: &str = "Name CDATA #REQUIRED";
const IMPLIED_NAME: &str = "Name CDATA #IMPLIED";

/// The parameter entities of the DTD files, each as the file declares it.
pub const ENTITIES: &[(&str, &str)] = &[
    ("namelist", "PatchNameList | NoteNameList | ControlNameList | ValueNameList"),
    ("noteorctl_listorref", "(NoteNameList | UsesNoteNameList)?, (ControlNameList | UsesControlNameList)?"),
    ("ChannelRequired", "#IMPLIED"),
    (
        "MIDIChannelVoiceMessage",
        "(NoteOn | NoteOff | PolyKeyPressure | ControlChange | ProgramChange | ChannelKeyPressure | PitchBendChange)",
    ),
    (
        "MIDIChannelModeMessage",
        "(AllSoundOff | ResetAllControllers | LocalControl | AllNotesOff | OmniOff | OmniOn | MonoMode | PolyMode)",
    ),
    ("MIDIChannelMessage", "(%MIDIChannelVoiceMessage; | %MIDIChannelModeMessage;)"),
    (
        "MIDISystemMessage",
        "(SysEx | MTCQuarterFrame | SongPositionPointer | SongSelect | TuneRequest | TimingClock | Start | Continue | Stop | ActiveSensing | SystemReset)",
    ),
    ("MIDIMessage", "(%MIDIChannelMessage; | %MIDISystemMessage; )"),
];

/// The table of the verifier: one line per element of the DTD files, its content model as the
/// DTD writes it and its attributes (name, type, default).
pub const RULES: &[(&str, &str, &[&str])] = &[
    ("MIDINameDocument", "(Author, (MasterDeviceNames+ | ExtendingDeviceNames+ | StandardDeviceMode+))", &[]),
    ("Author", "(#PCDATA)", &[]),
    (
        "MasterDeviceNames",
        "(((Manufacturer, Model+) | Device), (CustomDeviceMode | SupportsStandardDeviceMode)+, ChannelNameSet*, (%namelist;)*)",
        &[],
    ),
    ("ExtendingDeviceNames", "(((Manufacturer, Model+)? | Device), (%namelist;)+)", &[]),
    ("Manufacturer", "(#PCDATA)", &[]),
    ("Model", "(#PCDATA)", &[]),
    ("Device", "EMPTY", &["Name CDATA #REQUIRED", "UniqueID NMTOKEN #REQUIRED"]),
    (
        "StandardDeviceMode",
        "(DeviceModeEnable?, DeviceModeDisable?, ChannelNameSetAssignments, ChannelNameSet+, (%namelist;)*)",
        &[REQUIRED_NAME],
    ),
    ("CustomDeviceMode", "(DeviceModeEnable?, DeviceModeDisable?, ChannelNameSetAssignments, (%namelist;)*)", &[REQUIRED_NAME]),
    ("DeviceModeEnable", "(MIDICommands)", &[]),
    ("DeviceModeDisable", "(MIDICommands)", &[]),
    ("SupportsStandardDeviceMode", "EMPTY", &[REQUIRED_NAME]),
    ("ChannelNameSetAssignments", "(ChannelNameSetAssign+)", &[]),
    ("ChannelNameSetAssign", "EMPTY", &["Channel NMTOKEN #REQUIRED", "NameSet CDATA #REQUIRED"]),
    ("ChannelNameSet", "(AvailableForChannels, %noteorctl_listorref;, PatchBank+)", &[REQUIRED_NAME]),
    ("AvailableForChannels", "(AvailableChannel+)", &[]),
    ("AvailableChannel", "EMPTY", &["Channel NMTOKEN #REQUIRED", "Available (false|true) \"true\""]),
    ("PatchBank", "(MIDICommands?, (UsesPatchNameList | PatchNameList))", &[IMPLIED_NAME, "ROM (false|true) \"false\""]),
    ("UsesPatchNameList", "EMPTY", &[REQUIRED_NAME]),
    ("PatchNameList", "(Patch*)", &[IMPLIED_NAME]),
    (
        "Patch",
        "(PatchMIDICommands?, ChannelNameSetAssignments?, %noteorctl_listorref; )",
        &["Number CDATA #REQUIRED", "Name CDATA #REQUIRED", "ProgramChange NMTOKEN #IMPLIED"],
    ),
    ("UsesNoteNameList", "EMPTY", &[REQUIRED_NAME]),
    ("NoteNameList", "(NoteGroup | Note)+", &[IMPLIED_NAME]),
    ("NoteGroup", "(Note*)", &[IMPLIED_NAME]),
    ("Note", "EMPTY", &["Number NMTOKEN #REQUIRED", "Name CDATA #REQUIRED"]),
    ("UsesControlNameList", "EMPTY", &[REQUIRED_NAME]),
    ("ControlNameList", "(Control+)", &[IMPLIED_NAME]),
    (
        "Control",
        "(Values?)",
        &["Type (7bit | 14bit | RPN | NRPN) \"7bit\"", "Number NMTOKEN #REQUIRED", "Name CDATA #REQUIRED"],
    ),
    (
        "Values",
        "(ValueNameList | UsesValueNameList)?",
        &["Min NMTOKEN #REQUIRED", "Max NMTOKEN #REQUIRED", "Default NMTOKEN #IMPLIED", "Units NMTOKEN #IMPLIED", "Mapping NMTOKEN #IMPLIED"],
    ),
    ("UsesValueNameList", "EMPTY", &[REQUIRED_NAME]),
    ("ValueNameList", "(Value+)", &[IMPLIED_NAME]),
    ("Value", "EMPTY", &["Number NMTOKEN #REQUIRED", "Name CDATA #REQUIRED"]),
    ("MIDICommands", "(MIDIDelay | %MIDIChannelMessage; | SysEx)+", &[]),
    ("PatchMIDICommands", "(MIDIDelay | %MIDIChannelMessage; )+", &[]),
    ("MIDIDelay", "EMPTY", &["Milliseconds CDATA #REQUIRED"]),
    // The elements of MIDIEvents10.dtd, which MIDINameDocument10.dtd includes.
    ("NoteOn", "EMPTY", &[CH, "Note NMTOKEN #REQUIRED", "Velocity NMTOKEN #REQUIRED"]),
    ("NoteOff", "EMPTY", &[CH, "Note NMTOKEN #REQUIRED", "Velocity NMTOKEN #REQUIRED"]),
    ("PolyKeyPressure", "EMPTY", &[CH, "Note NMTOKEN #REQUIRED", "Pressure NMTOKEN #REQUIRED"]),
    ("ControlChange", "EMPTY", &[CH, "Control NMTOKEN #REQUIRED", "Value NMTOKEN #REQUIRED"]),
    ("ProgramChange", "EMPTY", &[CH, "Number NMTOKEN #REQUIRED"]),
    ("ChannelKeyPressure", "EMPTY", &[CH, "Pressure NMTOKEN #REQUIRED"]),
    ("PitchBendChange", "EMPTY", &[CH, "Value NMTOKEN #REQUIRED"]),
    ("AllSoundOff", "EMPTY", &[CH]),
    ("ResetAllControllers", "EMPTY", &[CH]),
    ("LocalControl", "EMPTY", &[CH, "Value (off|on) #REQUIRED"]),
    ("AllNotesOff", "EMPTY", &[CH]),
    ("OmniOff", "EMPTY", &[CH]),
    ("OmniOn", "EMPTY", &[CH]),
    ("MonoMode", "EMPTY", &[CH, "Value NMTOKEN #REQUIRED"]),
    ("PolyMode", "EMPTY", &[CH]),
    ("ControlChange14", "EMPTY", &[CH, "Control NMTOKEN #REQUIRED", "Value NMTOKEN #REQUIRED"]),
    ("RPNChange", "EMPTY", &[CH, "RPN NMTOKEN #REQUIRED", "Value NMTOKEN #REQUIRED"]),
    ("NRPNChange", "EMPTY", &[CH, "NRPN NMTOKEN #REQUIRED", "Value NMTOKEN #REQUIRED"]),
    ("SysEx", "(#PCDATA | SysExDeviceID | SysExChannel)*", &[]),
    ("SysExDeviceID", "EMPTY", &["Multiplier NMTOKEN \"1\"", "Offset NMTOKEN \"0\""]),
    ("SysExChannel", "EMPTY", &["Multiplier NMTOKEN \"1\"", "Offset NMTOKEN \"0\""]),
    (
        "MTCQuarterFrame",
        "EMPTY",
        &[
            "MessageType (FrameLSNibble | FrameMSNibble | SecsLSNibble | SecsMSNibble | MinsLSNibble | MinsMSNibble | HrsLSNibble | HrsMSNibbleSMPTEType) #REQUIRED",
            "DataNibble NMTOKEN #REQUIRED",
        ],
    ),
    ("SongPositionPointer", "EMPTY", &["Position NMTOKEN #REQUIRED"]),
    ("SongSelect", "EMPTY", &["Number NMTOKEN #REQUIRED"]),
    ("TuneRequest", "EMPTY", &[]),
    ("TimingClock", "EMPTY", &[]),
    ("Start", "EMPTY", &[]),
    ("Continue", "EMPTY", &[]),
    ("Stop", "EMPTY", &[]),
    ("ActiveSensing", "EMPTY", &[]),
    ("SystemReset", "EMPTY", &[]),
];

/// A declaration text with its white space reduced: one space between words, none around the
/// signs of a content model or a list of values.
pub fn normal(s: &str) -> String {
    let words = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    let chars: Vec<char> = words.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if *c == ' ' {
            let before = out.chars().last();
            let after = chars.get(i + 1);
            if matches!(before, Some('|' | ',' | '(')) || matches!(after, Some('|' | ',' | ')')) {
                continue;
            }
        }
        out.push(*c);
    }
    out
}

/// What the DTD files declare, each as they write it, white space reduced.
#[derive(Debug, Default)]
pub struct Declarations {
    /// element → content model
    pub elements: BTreeMap<String, String>,
    /// element → attributes as `name type default`
    pub attributes: BTreeMap<String, Vec<String>>,
    /// parameter entity → value
    pub entities: BTreeMap<String, String>,
}

pub fn declarations() -> Declarations {
    let mut out = Declarations::default();
    for path in [DTD, EVENTS_DTD] {
        let mut text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        while let Some(a) = text.find("<!--") {
            let b = text[a..].find("-->").unwrap_or_else(|| panic!("{path}: a comment is never closed")) + a + 3;
            text.replace_range(a..b, " ");
        }
        for decl in text.split("<!").skip(1) {
            let body = decl.split('>').next().unwrap_or("");
            let body = normal(body);
            if let Some(rest) = body.strip_prefix("ELEMENT ") {
                let (name, model) = rest.split_once(' ').unwrap_or((rest, ""));
                out.elements.insert(name.to_string(), model.to_string());
            } else if let Some(rest) = body.strip_prefix("ATTLIST ") {
                let (name, list) = rest.split_once(' ').unwrap();
                let mut words = list.split(' ').peekable();
                let mut decls = Vec::new();
                while words.peek().is_some() {
                    let triple: Vec<&str> = words.by_ref().take(3).collect();
                    decls.push(triple.join(" "));
                }
                out.attributes.entry(name.to_string()).or_default().extend(decls);
            } else if let Some(rest) = body.strip_prefix("ENTITY % ") {
                let (name, value) = rest.split_once(' ').unwrap();
                if let Some(v) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                    out.entities.insert(name.to_string(), v.to_string());
                }
            }
        }
    }
    out
}

fn expand(s: &str) -> String {
    let entities: BTreeMap<String, String> = ENTITIES.iter().map(|(k, v)| (k.to_string(), normal(v))).collect();
    let mut out = normal(s);
    while let Some(a) = out.find('%') {
        let b = out[a..].find(';').expect("an entity reference ends with ;") + a;
        let name = out[a + 1..b].to_string();
        let value = entities.get(&name).unwrap_or_else(|| panic!("the table has no entity {name}"));
        out.replace_range(a..=b, value);
    }
    out
}

/// A content model.
#[derive(Clone, Debug)]
enum Cm {
    Name(String),
    Seq(Vec<Cm>),
    Alt(Vec<Cm>),
    Opt(Box<Cm>),
    Star(Box<Cm>),
    Plus(Box<Cm>),
}

fn parse_unit(s: &[char], i: &mut usize) -> Cm {
    let base = if s[*i] == '(' {
        *i += 1;
        let mut items = vec![parse_unit(s, i)];
        let mut sep = None;
        while s[*i] != ')' {
            sep = Some(s[*i]);
            *i += 1;
            items.push(parse_unit(s, i));
        }
        *i += 1;
        match (items.len(), sep) {
            (1, _) => items.pop().unwrap(),
            (_, Some('|')) => Cm::Alt(items),
            _ => Cm::Seq(items),
        }
    } else {
        let start = *i;
        while *i < s.len() && !matches!(s[*i], ',' | '|' | ')' | '?' | '*' | '+') {
            *i += 1;
        }
        Cm::Name(s[start..*i].iter().collect())
    };
    match s.get(*i) {
        Some('?') => {
            *i += 1;
            Cm::Opt(Box::new(base))
        }
        Some('*') => {
            *i += 1;
            Cm::Star(Box::new(base))
        }
        Some('+') => {
            *i += 1;
            Cm::Plus(Box::new(base))
        }
        _ => base,
    }
}

/// The positions after `names[from..]` is matched by `cm`.
fn ends(cm: &Cm, names: &[&str], from: usize) -> BTreeSet<usize> {
    match cm {
        Cm::Name(n) => {
            if names.get(from) == Some(&n.as_str()) {
                BTreeSet::from([from + 1])
            } else {
                BTreeSet::new()
            }
        }
        Cm::Seq(items) => {
            let mut at = BTreeSet::from([from]);
            for it in items {
                at = at.iter().flat_map(|p| ends(it, names, *p)).collect();
            }
            at
        }
        Cm::Alt(items) => items.iter().flat_map(|it| ends(it, names, from)).collect(),
        Cm::Opt(it) => {
            let mut e = ends(it, names, from);
            e.insert(from);
            e
        }
        Cm::Star(it) | Cm::Plus(it) => {
            let mut all = BTreeSet::new();
            let mut frontier = BTreeSet::from([from]);
            if matches!(cm, Cm::Star(_)) {
                all.insert(from);
            }
            while !frontier.is_empty() {
                let mut next = BTreeSet::new();
                for p in &frontier {
                    for e in ends(it, names, *p) {
                        if e != *p && all.insert(e) {
                            next.insert(e);
                        }
                    }
                }
                frontier = next;
            }
            all
        }
    }
}

/// The element names a content model mentions.
fn mentioned(model: &str) -> BTreeSet<String> {
    model
        .split(['(', ')', '|', ',', '?', '*', '+'])
        .filter(|w| !w.is_empty() && !w.starts_with('#') && *w != "EMPTY")
        .map(|w| w.to_string())
        .collect()
}

pub fn rule(name: &str) -> Option<&'static (&'static str, &'static str, &'static [&'static str])> {
    RULES.iter().find(|r| r.0 == name)
}

/// The names of an attribute declaration `name type default`.
pub fn attribute_parts(decl: &str) -> (String, String, String) {
    let expanded = expand(decl);
    let mut w = expanded.splitn(3, ' ');
    (w.next().unwrap().to_string(), w.next().unwrap().to_string(), w.next().unwrap().to_string())
}

/// The violations of the DTD in one element (its attributes and its children, not its
/// descendants).
pub fn check_element(n: &Node) -> Vec<String> {
    let Some((_, model, attrs)) = rule(&n.name) else {
        return vec![format!("{}: the element is not declared by the DTD", n.name)];
    };
    let mut out = Vec::new();
    let decls: Vec<(String, String, String)> = attrs.iter().map(|a| attribute_parts(a)).collect();
    for (k, v) in &n.attrs {
        match decls.iter().find(|d| &d.0 == k) {
            None => out.push(format!("{}: the attribute {k} is not declared by the DTD", n.name)),
            Some((_, ty, _)) => {
                if ty.starts_with('(') {
                    let values: Vec<&str> = ty.trim_matches(|c| c == '(' || c == ')').split('|').collect();
                    if !values.contains(&v.as_str()) {
                        out.push(format!("{}: the attribute {k} is {v:?}, one of {ty} is expected", n.name));
                    }
                } else if ty == "NMTOKEN" && (v.is_empty() || v.contains(char::is_whitespace)) {
                    out.push(format!("{}: the attribute {k} is {v:?}, a name token is expected", n.name));
                }
            }
        }
    }
    for (name, _, default) in &decls {
        if default == "#REQUIRED" && n.attr(name).is_none() {
            out.push(format!("{}: the required attribute {name} is missing", n.name));
        }
    }
    let model = expand(model);
    let names: Vec<&str> = n.children.iter().map(|c| c.name.as_str()).collect();
    if model == "EMPTY" {
        if !names.is_empty() || !n.text.is_empty() {
            out.push(format!("{}: the content must be empty, it has {}", n.name, names.join(", ")));
        }
    } else if model.starts_with("(#PCDATA") {
        let allowed = mentioned(&model);
        for c in &names {
            if !allowed.contains(*c) {
                out.push(format!("{}: the child {c} is not allowed in the text content", n.name));
            }
        }
    } else {
        if !n.text.is_empty() {
            out.push(format!("{}: text is not allowed in the element content", n.name));
        }
        let chars: Vec<char> = model.chars().collect();
        let cm = parse_unit(&chars, &mut 0);
        if !ends(&cm, &names, 0).contains(&names.len()) {
            out.push(format!("{}: the children ({}) do not match {model}", n.name, names.join(", ")));
        }
    }
    out
}

/// The violations of the DTD in an element and its descendants, each with its line.
pub fn check_tree(n: &Node) -> Vec<String> {
    let mut out: Vec<String> = check_element(n).into_iter().map(|v| format!("line {}: {v}", n.line)).collect();
    for c in &n.children {
        out.extend(check_tree(c));
    }
    out
}

/// The violations of the DTD in a document text: well-formed, the public document type of
/// MIDNAM, and every element valid.
pub fn violations(text: &str) -> Vec<String> {
    let doc = match xml::parse(text) {
        Ok(d) => d,
        Err(e) => return vec![format!("not well-formed: {e}")],
    };
    let mut out = Vec::new();
    match &doc.doctype {
        Some(d) if d.starts_with("MIDINameDocument") && d.contains(PUBLIC_ID) => {}
        other => out.push(format!("the document type is not the public MIDINameDocument 1.0: {other:?}")),
    }
    if doc.root.name != "MIDINameDocument" {
        out.push(format!("the root is {}, MIDINameDocument is expected", doc.root.name));
    }
    out.extend(check_tree(&doc.root));
    out
}

/// A value the DTD accepts for an attribute declaration.
pub fn sample_value(decl: &str) -> String {
    let (_, ty, _) = attribute_parts(decl);
    if ty.starts_with('(') {
        ty.trim_matches(|c| c == '(' || c == ')').split('|').next().unwrap().to_string()
    } else if ty == "NMTOKEN" {
        "1".to_string()
    } else {
        "x".to_string()
    }
}

/// The element of a rule with every attribute set to a value the DTD accepts.
pub fn sample(name: &str) -> Node {
    let (_, _, attrs) = rule(name).unwrap();
    let mut n = Node { name: name.to_string(), ..Node::default() };
    for a in *attrs {
        n.attrs.push((attribute_parts(a).0, sample_value(a)));
    }
    n
}

/// The element names a rule's content model allows.
pub fn allowed_children(name: &str) -> BTreeSet<String> {
    mentioned(&expand(rule(name).unwrap().1))
}

// ---- documents and devices the tests build ----

/// A MIDNAM document valid by the DTD that holds each element an importer reads, and some it
/// can only lose.
pub fn rich_document() -> Node {
    let a = |n: &str, attrs: &[(&str, &str)]| Node::new(n, attrs);
    let mut author = a("Author", &[]);
    author.text = "Someone, CC0-1.0".into();
    let mut manufacturer = a("Manufacturer", &[]);
    manufacturer.text = "Lofty".into();
    let mut model = a("Model", &[]);
    model.text = "Trundler".into();
    let commands = |cc: &[(&str, &str)]| {
        let mut c = a("MIDICommands", &[]);
        for (control, value) in cc {
            c.children.push(a("ControlChange", &[("Control", control), ("Value", value)]));
        }
        c
    };
    let master = a("MasterDeviceNames", &[])
        .with(manufacturer)
        .with(model)
        .with(
            a("CustomDeviceMode", &[("Name", "Default")])
                .with(a("DeviceModeEnable", &[]).with(commands(&[("1", "2")])))
                .with(a("ChannelNameSetAssignments", &[]).with(a("ChannelNameSetAssign", &[("Channel", "1"), ("NameSet", "Names")]))),
        )
        .with(
            a("ChannelNameSet", &[("Name", "Names")])
                .with(a("AvailableForChannels", &[]).with(a("AvailableChannel", &[("Channel", "1"), ("Available", "true")])))
                .with(a("UsesNoteNameList", &[("Name", "Notes")]))
                .with(a("UsesControlNameList", &[("Name", "Controls")]))
                .with(
                    a("PatchBank", &[("Name", "Bank A")])
                        .with(commands(&[("0", "1"), ("32", "2")]))
                        .with(
                            a("PatchNameList", &[("Name", "Bank A")])
                                .with(a("Patch", &[("Number", "0"), ("Name", "Bank A 0"), ("ProgramChange", "0")]))
                                .with(a("Patch", &[("Number", "1"), ("Name", "Warm Pad"), ("ProgramChange", "1")])),
                        ),
                ),
        )
        .with(a("NoteNameList", &[("Name", "Notes")]).with(a("Note", &[("Number", "36"), ("Name", "Kick")])))
        .with(
            a("ControlNameList", &[("Name", "Controls")])
                .with(
                    a("Control", &[("Type", "7bit"), ("Number", "74"), ("Name", "Filter Cutoff")])
                        .with(a("Values", &[("Min", "0"), ("Max", "127"), ("Default", "64"), ("Units", "dB"), ("Mapping", "linear")])),
                )
                .with(a("Control", &[("Type", "NRPN"), ("Number", "300"), ("Name", "Fine")]).with(a("Values", &[("Min", "0"), ("Max", "16383")])))
                .with(
                    a("Control", &[("Type", "14bit"), ("Number", "5"), ("Name", "Wide")]).with(
                        a("Values", &[("Min", "0"), ("Max", "16383")])
                            .with(a("ValueNameList", &[]).with(a("Value", &[("Number", "0"), ("Name", "Off")]))),
                    ),
                ),
        );
    a("MIDINameDocument", &[]).with(author).with(master)
}

pub fn rich_text() -> String {
    xml::document_text(&rich_document())
}

/// A document of the MIDNAM kind with one control: its type, number, range and Name.
pub fn control_document(kind: &str, number: &str, min: &str, max: &str, name: &str, default: Option<&str>) -> String {
    let mut values = vec![("Min", min), ("Max", max)];
    if let Some(d) = default {
        values.push(("Default", d));
    }
    let mut doc = rich_document();
    let list = doc.at_mut(&[1, 5]);
    assert_eq!(list.name, "ControlNameList");
    list.children = vec![Node::new("Control", &[("Type", kind), ("Number", number), ("Name", name)]).with(Node::new("Values", &values))];
    xml::document_text(&doc)
}

/// A device from the JSON of some of its blocks: a name, a vendor, a prefix and one source
/// with its credit, 0.6; the blocks given replace those.
pub fn device_with(extra: Value) -> osd::Resolved {
    let mut doc = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [{"type": "community-re", "url": "https://example.org/d", "author": "Dee", "license": "CC-BY-4.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
    });
    for (k, v) in extra.as_object().expect("an object of blocks") {
        if v.is_null() {
            doc.as_object_mut().unwrap().remove(k);
        } else {
            doc[k] = v.clone();
        }
    }
    super::resolved(super::device_from(&doc.to_string()))
}

pub fn exported(device: &osd::Resolved) -> (String, Vec<osd_formats::Loss>) {
    let t = osd_formats::export(device, osd_formats::Format::Midnam).expect("the device exports");
    (t.value, t.lost)
}

/// The document an export wrote, read; it is also checked by the DTD.
pub fn parsed(text: &str) -> Node {
    let v = violations(text);
    assert!(v.is_empty(), "the export is not valid by the DTD: {v:?}\n{text}");
    xml::parse(text).unwrap().root
}

/// The device an import returned, as JSON.
pub fn imported(text: &str) -> (Value, Vec<osd_formats::Loss>) {
    let t = osd_formats::import(text, osd_formats::Format::Midnam, &super::source()).unwrap_or_else(|f| panic!("{f}\n{text}"));
    (super::electra::json_of(&t.value), t.lost)
}

pub fn lost_at<'a>(lost: &'a [osd_formats::Loss], place: &str) -> Vec<&'a osd_formats::Loss> {
    lost.iter().filter(|l| l.place == place).collect()
}
