//! The verifier of a MIDNAM document: one line per element of the DTD files of the MIDI
//! Association (MIDINameDocument10.dtd and MIDIEvents10.dtd, which it includes), its content
//! model and its attributes as the DTD writes them. An import reads with it and an export is
//! checked against it. A test of the repository reads the declarations of the DTD files and
//! needs a line here for each, and none without a declaration.

use crate::xml::Node;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

const CH: &str = "Channel (1|2|3|4|5|6|7|8|9|10|11|12|13|14|15|16) %ChannelRequired;";
const REQUIRED_NAME: &str = "Name CDATA #REQUIRED";
const IMPLIED_NAME: &str = "Name CDATA #IMPLIED";

/// The parameter entities of the DTD files, each as the file declares it.
const ENTITIES: &[(&str, &str)] = &[
    ("namelist", "PatchNameList | NoteNameList | ControlNameList | ValueNameList"),
    ("noteorctl_listorref", "(NoteNameList | UsesNoteNameList)?, (ControlNameList | UsesControlNameList)?"),
    ("ChannelRequired", "#IMPLIED"),
    ("MIDIChannelVoiceMessage", "(NoteOn | NoteOff | PolyKeyPressure | ControlChange | ProgramChange | ChannelKeyPressure | PitchBendChange)"),
    ("MIDIChannelModeMessage", "(AllSoundOff | ResetAllControllers | LocalControl | AllNotesOff | OmniOff | OmniOn | MonoMode | PolyMode)"),
    ("MIDIChannelMessage", "(%MIDIChannelVoiceMessage; | %MIDIChannelModeMessage;)"),
    (
        "MIDISystemMessage",
        "(SysEx | MTCQuarterFrame | SongPositionPointer | SongSelect | TuneRequest | TimingClock | Start | Continue | Stop | ActiveSensing | SystemReset)",
    ),
    ("MIDIMessage", "(%MIDIChannelMessage; | %MIDISystemMessage; )"),
];

/// The table: an element, its content model and its attributes (name, type, default).
const RULES: &[(&str, &str, &[&str])] = &[
    ("MIDINameDocument", "(Author, (MasterDeviceNames+ | ExtendingDeviceNames+ | StandardDeviceMode+))", &[]),
    ("Author", "(#PCDATA)", &[]),
    ("MasterDeviceNames", "(((Manufacturer, Model+) | Device), (CustomDeviceMode | SupportsStandardDeviceMode)+, ChannelNameSet*, (%namelist;)*)", &[]),
    ("ExtendingDeviceNames", "(((Manufacturer, Model+)? | Device), (%namelist;)+)", &[]),
    ("Manufacturer", "(#PCDATA)", &[]),
    ("Model", "(#PCDATA)", &[]),
    ("Device", "EMPTY", &["Name CDATA #REQUIRED", "UniqueID NMTOKEN #REQUIRED"]),
    ("StandardDeviceMode", "(DeviceModeEnable?, DeviceModeDisable?, ChannelNameSetAssignments, ChannelNameSet+, (%namelist;)*)", &[REQUIRED_NAME]),
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
    ("Patch", "(PatchMIDICommands?, ChannelNameSetAssignments?, %noteorctl_listorref; )", &["Number CDATA #REQUIRED", "Name CDATA #REQUIRED", "ProgramChange NMTOKEN #IMPLIED"]),
    ("UsesNoteNameList", "EMPTY", &[REQUIRED_NAME]),
    ("NoteNameList", "(NoteGroup | Note)+", &[IMPLIED_NAME]),
    ("NoteGroup", "(Note*)", &[IMPLIED_NAME]),
    ("Note", "EMPTY", &["Number NMTOKEN #REQUIRED", "Name CDATA #REQUIRED"]),
    ("UsesControlNameList", "EMPTY", &[REQUIRED_NAME]),
    ("ControlNameList", "(Control+)", &[IMPLIED_NAME]),
    ("Control", "(Values?)", &["Type (7bit | 14bit | RPN | NRPN) \"7bit\"", "Number NMTOKEN #REQUIRED", "Name CDATA #REQUIRED"]),
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
fn normal(s: &str) -> String {
    let words = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = words.chars().collect();
    let mut out = String::new();
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

fn expand(s: &str) -> String {
    let entities: BTreeMap<&str, String> = ENTITIES.iter().map(|(k, v)| (*k, normal(v))).collect();
    let mut out = normal(s);
    while let Some(a) = out.find('%') {
        let b = out[a..].find(';').expect("an entity reference ends with ;") + a;
        let value = entities.get(&out[a + 1..b]).expect("the table has the entity");
        let value = value.clone();
        out.replace_range(a..=b, &value);
    }
    out
}

/// A content model.
#[derive(Debug)]
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
            (1, _) => items.pop().expect("one item"),
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
        .map(str::to_string)
        .collect()
}

enum Content {
    Empty,
    Mixed(BTreeSet<String>),
    Elements(Cm, BTreeSet<String>),
}

struct Attribute {
    name: String,
    /// The values of an enumerated type.
    values: Option<Vec<String>>,
    token: bool,
    required: bool,
}

struct Rule {
    content: Content,
    model: String,
    attributes: Vec<Attribute>,
}

fn rules() -> &'static BTreeMap<&'static str, Rule> {
    static COMPILED: OnceLock<BTreeMap<&'static str, Rule>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        RULES
            .iter()
            .map(|(name, model, attrs)| {
                let model = expand(model);
                let content = if model == "EMPTY" {
                    Content::Empty
                } else if model.starts_with("(#PCDATA") {
                    Content::Mixed(mentioned(&model))
                } else {
                    let chars: Vec<char> = model.chars().collect();
                    Content::Elements(parse_unit(&chars, &mut 0), mentioned(&model))
                };
                let attributes = attrs
                    .iter()
                    .map(|a| {
                        let expanded = expand(a);
                        let mut w = expanded.splitn(3, ' ');
                        let (n, ty, default) = (w.next().unwrap_or(""), w.next().unwrap_or(""), w.next().unwrap_or(""));
                        Attribute {
                            name: n.to_string(),
                            values: ty.starts_with('(').then(|| ty.trim_matches(|c| c == '(' || c == ')').split('|').map(str::to_string).collect()),
                            token: ty == "NMTOKEN",
                            required: default == "#REQUIRED",
                        }
                    })
                    .collect();
                (*name, Rule { content, model, attributes })
            })
            .collect()
    })
}

/// A violation of the DTD: the line where it is and what it is.
pub(crate) type Violation = (usize, String);

/// The violations of the DTD in one element (its attributes and its children, not its
/// descendants). A child the content does not allow is named at its own line.
fn check_element(n: &Node) -> Vec<Violation> {
    let Some(rule) = rules().get(n.name.as_str()) else {
        return vec![(n.line, format!("{}: the element is not declared by the DTD", n.name))];
    };
    let mut out = Vec::new();
    for (k, v) in &n.attrs {
        match rule.attributes.iter().find(|a| &a.name == k) {
            None => out.push((n.line, format!("{}: the attribute {k} is not declared by the DTD", n.name))),
            Some(a) => {
                if let Some(values) = &a.values {
                    if !values.contains(v) {
                        out.push((n.line, format!("{}: the attribute {k} is {v:?}, one of {} is expected", n.name, values.join(", "))));
                    }
                } else if a.token && (v.is_empty() || v.contains(char::is_whitespace)) {
                    out.push((n.line, format!("{}: the attribute {k} is {v:?}, a name token is expected", n.name)));
                }
            }
        }
    }
    for a in rule.attributes.iter().filter(|a| a.required) {
        if n.attr(&a.name).is_none() {
            out.push((n.line, format!("{}: the required attribute {} is missing", n.name, a.name)));
        }
    }
    let names: Vec<&str> = n.children.iter().map(|c| c.name.as_str()).collect();
    let outside = |allowed: &BTreeSet<String>, out: &mut Vec<Violation>, what: &str| {
        for c in &n.children {
            if !allowed.contains(&c.name) {
                out.push((c.line, format!("{}: the child {} is not allowed {what}", n.name, c.name)));
            }
        }
    };
    match &rule.content {
        Content::Empty => {
            if !names.is_empty() || !n.text.is_empty() {
                out.push((n.children.first().map_or(n.line, |c| c.line), format!("{}: the content must be empty", n.name)));
            }
        }
        Content::Mixed(allowed) => outside(allowed, &mut out, "in the text content"),
        Content::Elements(cm, allowed) => {
            if !n.text.is_empty() {
                out.push((n.line, format!("{}: text is not allowed in the element content", n.name)));
            }
            let before = out.len();
            outside(allowed, &mut out, "here");
            if out.len() == before && !ends(cm, &names, 0).contains(&names.len()) {
                out.push((n.line, format!("{}: the children ({}) do not match {}", n.name, names.join(", "), rule.model)));
            }
        }
    }
    out
}

/// The violations of the DTD in an element and its descendants, in document order.
pub(crate) fn check_tree(n: &Node, out: &mut Vec<Violation>) {
    out.extend(check_element(n));
    for c in &n.children {
        check_tree(c, out);
    }
}
