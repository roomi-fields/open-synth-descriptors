//! MIDNAM, the MIDI Association's names document (MIDINameDocument 1.0): its importer and its
//! exporter. The rules are the DTD's (MIDINameDocument10.dtd and MIDIEvents10.dtd); the fate of
//! each element and attribute is in docs/midnam.md.

use crate::credit::{address_words, credit_text, name_segment, uncarried_source_fields};
use crate::midi_csv::check_source;
use crate::midnam_dtd::check_tree;
use crate::xml::{self, escape, Node};
use crate::{Fault, FaultKind, Loss, Translated};
use osd::{Device, Resolved, Source};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

const PUBLIC_ID: &str = "-//MIDI Manufacturers Association//DTD MIDINameDocument 1.0//EN";
const SYSTEM_ID: &str = "http://www.midi.org/dtds/MIDINameDocument10.dtd";
const SCHEMA_URL: &str = "https://w3id.org/osd/0.6/device.schema.json";

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

/// The loss of an element (`line N`, reason led by `Element`) or of one of its attributes
/// (reason led by `Element@Attr`).
fn lost_element(losses: &mut Vec<Loss>, n: &Node, attr: Option<&str>, reason: &str) {
    let what = match attr {
        None => n.name.clone(),
        Some(a) => format!("{}@{a}", n.name),
    };
    loss(losses, format!("line {}", n.line), format!("{what}: {reason}"));
}

/// A whole number written in an attribute.
fn whole(text: &str) -> Option<i64> {
    text.trim().parse().ok()
}

fn at_attr(n: &Node, attr: &str) -> String {
    format!("line {}, {}@{attr}", n.line, n.name)
}

/// The binding of a control: its JSON keys, or the reason the number does not fit its type.
fn binding(kind: &str, number: i64) -> Result<Vec<(&'static str, i64)>, &'static str> {
    match kind {
        "7bit" if (0..=127).contains(&number) => Ok(vec![("cc", number)]),
        "7bit" => Err("a 7-bit control number is from 0 to 127"),
        "14bit" if (0..=95).contains(&number) => Ok(vec![("cc", number), ("cc_lsb", number + 32)]),
        "14bit" => Err("the number of a 14-bit control is its MSB controller: its LSB, 32 above, is from 32 to 127"),
        "NRPN" if (0..=16383).contains(&number) => Ok(vec![("nrpn_msb", number >> 7), ("nrpn_lsb", number & 127)]),
        "NRPN" => Err("an NRPN number is from 0 to 16383"),
        "RPN" if (0..=16383).contains(&number) => Ok(vec![("rpn_msb", number >> 7), ("rpn_lsb", number & 127)]),
        _ => Err("an RPN number is from 0 to 16383"),
    }
}

/// An address not yet used: `base`, else `base_2`, `base_3`…
fn unique(base: String, used: &mut BTreeSet<String>) -> String {
    let mut address = base.clone();
    let mut n = 2;
    while !used.insert(address.clone()) {
        address = format!("{base}_{n}");
        n += 1;
    }
    address
}

fn address_of(name: &str, fallback: String, used: &mut BTreeSet<String>) -> String {
    let seg = name_segment(name);
    unique(format!("/{}", if seg.is_empty() { fallback } else { seg }), used)
}

struct Bank {
    json: Value,
    pitched: bool,
}

fn patch_bank(n: usize, pb: &Node, patch_lists: &[&Node], losses: &mut Vec<Loss>) -> Result<Option<Bank>, Fault> {
    if pb.attr("ROM") == Some("true") {
        lost_element(losses, pb, Some("ROM"), "the descriptor does not say whether a bank is read-only");
    }
    let (mut msb, mut lsb) = (None, None);
    if let Some(commands) = pb.kid("MIDICommands") {
        for c in &commands.children {
            let number = c.attr("Value").and_then(whole).filter(|v| (0..=127).contains(v));
            match (c.name.as_str(), c.attr("Control"), number) {
                ("ControlChange", Some("0"), Some(v)) if msb.is_none() => msb = Some(v),
                ("ControlChange", Some("32"), Some(v)) if lsb.is_none() => lsb = Some(v),
                _ => {
                    lost_element(losses, c, None, "only the Bank Select (controls 0 and 32) of the bank's commands is carried: a bank has an MSB and an LSB");
                    continue;
                }
            }
            if c.attr("Channel").is_some() {
                lost_element(losses, c, Some("Channel"), "the descriptor has no channel for a bank select");
            }
        }
    }
    let list = match (pb.kid("PatchNameList"), pb.kid("UsesPatchNameList")) {
        (Some(l), _) => l,
        (None, Some(u)) => {
            let name = u.attr("Name").unwrap_or("");
            match patch_lists.iter().find(|l| l.attr("Name") == Some(name)) {
                Some(l) => l,
                None => {
                    lost_element(losses, u, None, "no list of patch names of the document has this name: the bank has no patch to read");
                    return Ok(None);
                }
            }
        }
        (None, None) => return Ok(None),
    };
    let name = pb.attr("Name").or_else(|| list.attr("Name")).map(str::to_string);
    let bank_name = match &name {
        Some(n) if !n.is_empty() => n.clone(),
        _ => {
            lost_element(losses, pb, None, &format!("the bank has no name: \"Bank {n}\" is written, the descriptor needs one"));
            format!("Bank {n}")
        }
    };
    let patches: Vec<&Node> = list.kids("Patch").collect();
    if patches.is_empty() {
        lost_element(losses, pb, None, "a bank with no patch: the descriptor's bank holds at least one program");
        return Ok(None);
    }
    let (mut first_number, mut first_pc) = (None, None);
    for (i, p) in patches.iter().enumerate() {
        let number = p.attr("Number").and_then(whole);
        let pc = p.attr("ProgramChange").and_then(whole).filter(|v| (0..=127).contains(v));
        if i == 0 {
            first_number = number;
            first_pc = pc;
        }
        if number != first_number.map(|f| f + i as i64) {
            lost_element(losses, p, Some("Number"), "the numbers of a bank follow one another from the first: this one does not");
        }
        if pc != first_pc.map(|f| f + i as i64) {
            lost_element(losses, p, Some("ProgramChange"), "the program changes of a bank follow one another from the first: this one does not");
        }
        if p.attr("Name") != Some(&patch_name(&bank_name, p.attr("Number").unwrap_or(""))) {
            lost_element(losses, p, Some("Name"), "the descriptor does not name its programs: a patch is named by its bank and number");
        }
        for c in &p.children {
            lost_element(losses, c, None, "what a patch adds to its bank (commands, channels, its own names) is no field of the descriptor");
        }
    }
    let mut bank = Map::new();
    bank.insert("name".into(), json!(bank_name));
    if let Some(v) = msb {
        bank.insert("msb".into(), json!(v));
    }
    if let Some(v) = lsb {
        bank.insert("lsb".into(), json!(v));
    }
    bank.insert("size".into(), json!(patches.len()));
    if let Some(v) = first_pc {
        bank.insert("first_pc".into(), json!(v));
    }
    if let Some(v) = first_number {
        bank.insert("first_number".into(), json!(v));
    }
    Ok(Some(Bank { json: Value::Object(bank), pitched: first_pc.is_some() }))
}

fn control(c: &Node, used: &mut BTreeSet<String>, losses: &mut Vec<Loss>) -> Result<Value, Fault> {
    let kind = c.attr("Type").unwrap_or("7bit");
    let number_text = c.attr("Number").unwrap_or("");
    let number = whole(number_text).ok_or_else(|| invalid(at_attr(c, "Number"), format!("{number_text:?} is not a whole number")))?;
    let keys = binding(kind, number).map_err(|rule| invalid(at_attr(c, "Number"), rule))?;
    let name = c.attr("Name").unwrap_or("");
    let mut entry = Map::new();
    entry.insert("osc".into(), json!(address_of(name, format!("{}_{number}", kind.to_lowercase()), used)));
    for (k, v) in keys {
        entry.insert(k.into(), json!(v));
    }
    if let Some(values) = c.kid("Values") {
        let bound = |attr: &str, losses: &mut Vec<Loss>| {
            let text = values.attr(attr)?;
            let n = whole(text);
            if n.is_none() {
                lost_element(losses, values, Some(attr), &format!("{text:?} is not a whole number: the descriptor's range holds whole numbers"));
            }
            n
        };
        let (lo, hi) = (bound("Min", losses), bound("Max", losses));
        if let (Some(lo), Some(hi)) = (lo, hi) {
            entry.insert("range".into(), json!([lo, hi]));
        }
        if let Some(d) = bound("Default", losses) {
            entry.insert("default".into(), json!(d));
        }
        for attr in ["Units", "Mapping"] {
            if values.attr(attr).is_some() {
                lost_element(losses, values, Some(attr), "the descriptor holds no unit or mapping of a control's value");
            }
        }
        for child in &values.children {
            lost_element(losses, child, None, "the descriptor holds no names for the values of a control");
        }
    }
    if !name.is_empty() {
        entry.insert("_name".into(), json!(name));
    }
    Ok(Value::Object(entry))
}

fn trigger(note: &Node, used: &mut BTreeSet<String>) -> Result<Value, Fault> {
    let text = note.attr("Number").unwrap_or("");
    let number = whole(text).filter(|n| (0..=127).contains(n)).ok_or_else(|| invalid(at_attr(note, "Number"), format!("{text:?} is not a note number from 0 to 127")))?;
    let name = note.attr("Name").unwrap_or("");
    Ok(json!({"osc": address_of(name, format!("note_{number}"), used), "sound": name, "note_range": [number, number], "note": number}))
}

fn notes_of(list: &Node, used: &mut BTreeSet<String>, entries: &mut Vec<Value>, losses: &mut Vec<Loss>) -> Result<(), Fault> {
    for c in &list.children {
        if c.name == "NoteGroup" {
            lost_element(losses, c, None, "the descriptor does not group its sounds: the notes of the group are read one by one");
            for note in c.kids("Note") {
                entries.push(trigger(note, used)?);
            }
        } else {
            entries.push(trigger(c, used)?);
        }
    }
    Ok(())
}

pub(crate) fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let root = xml::parse(text).map_err(|e| invalid(format!("line {}", e.line), e.rule))?;
    if root.name != "MIDINameDocument" {
        return Err(invalid(format!("line {}", root.line), format!("the root element is {}, MIDINameDocument is expected", root.name)));
    }
    let mut found = Vec::new();
    check_tree(&root, &mut found);
    if let Some((line, rule)) = found.into_iter().next() {
        return Err(invalid(format!("line {line}"), rule));
    }
    let masters: Vec<&Node> = root.kids("MasterDeviceNames").collect();
    let Some(master) = masters.first().copied() else {
        return Err(invalid(format!("line {}", root.line), "the document holds no MasterDeviceNames: a device has no name to read in it"));
    };

    let mut losses = Vec::new();
    if let Some(author) = root.kid("Author").filter(|a| !a.text.is_empty()) {
        lost_element(&mut losses, author, None, "the credit of the file is not copied: the source given to the import is the credit");
    }
    for extra in &masters[1..] {
        lost_element(&mut losses, extra, None, "a descriptor describes one device: the first MasterDeviceNames is read");
    }

    let (name, vendor) = if let Some(d) = master.kid("Device") {
        lost_element(&mut losses, d, Some("UniqueID"), "the descriptor has no unique id of a device");
        (d.attr("Name").unwrap_or("").to_string(), None)
    } else {
        let mut models = master.kids("Model");
        let first = models.next();
        for m in models {
            lost_element(&mut losses, m, None, "a descriptor describes one model: the first Model is read");
        }
        let vendor = master.kid("Manufacturer").map(|m| m.text.clone()).filter(|v| !v.is_empty());
        (first.map(|m| m.text.clone()).unwrap_or_default(), vendor)
    };
    if name.is_empty() {
        let line = master.kid("Model").or(master.kid("Device")).map_or(master.line, |m| m.line);
        return Err(invalid(format!("line {line}"), "the device has no name: a descriptor needs one"));
    }

    let (mut control_lists, mut note_lists, mut patch_lists, mut banks_of): (Vec<&Node>, Vec<&Node>, Vec<&Node>, Vec<&Node>) = (vec![], vec![], vec![], vec![]);
    for c in &master.children {
        match c.name.as_str() {
            "CustomDeviceMode" | "SupportsStandardDeviceMode" => {
                lost_element(&mut losses, c, None, "the device modes, which name the sets each channel uses, are no field of the descriptor");
                for k in c.children.iter().filter(|k| k.name == "DeviceModeEnable" || k.name == "DeviceModeDisable") {
                    lost_element(&mut losses, k, None, "the descriptor holds no command that enters or leaves a mode");
                }
            }
            "ChannelNameSet" => {
                for k in &c.children {
                    match k.name.as_str() {
                        "AvailableForChannels" => lost_element(&mut losses, k, None, "the descriptor does not say on which channels its names apply"),
                        "NoteNameList" => note_lists.push(k),
                        "ControlNameList" => control_lists.push(k),
                        "PatchBank" => banks_of.push(k),
                        _ => {}
                    }
                }
            }
            "NoteNameList" => note_lists.push(c),
            "ControlNameList" => control_lists.push(c),
            "PatchNameList" => patch_lists.push(c),
            "ValueNameList" => lost_element(&mut losses, c, None, "the descriptor holds no names for the values of a control"),
            _ => {}
        }
    }

    let mut used = BTreeSet::new();
    let mut entries = Vec::new();
    for list in &control_lists {
        for c in list.kids("Control") {
            entries.push(control(c, &mut used, &mut losses)?);
        }
    }
    let mut triggers = Vec::new();
    for list in &note_lists {
        notes_of(list, &mut used, &mut triggers, &mut losses)?;
    }
    let mut banks = Vec::new();
    let mut pitched = false;
    for (n, pb) in banks_of.iter().enumerate() {
        if let Some(bank) = patch_bank(n + 1, pb, &patch_lists, &mut losses)? {
            pitched |= bank.pitched;
            banks.push(bank.json);
        }
    }
    // A patch list no bank reads names patches the descriptor has nowhere to put.
    for l in patch_lists {
        let by_name = |pb: &&Node| pb.kid("UsesPatchNameList").and_then(|u| u.attr("Name")) == l.attr("Name") && pb.kid("UsesPatchNameList").is_some();
        if !banks_of.iter().any(by_name) && !l.children.is_empty() {
            lost_element(&mut losses, l, None, "no bank reads this list of patch names");
        }
    }

    let mut doc = Map::new();
    doc.insert("$schema".into(), json!(SCHEMA_URL));
    let mut info = Map::new();
    info.insert("name".into(), json!(name));
    if let Some(v) = vendor {
        info.insert("vendor".into(), json!(v));
    }
    let seg = name_segment(&name);
    info.insert("osc_prefix".into(), json!(format!("/{}", if seg.is_empty() { "device" } else { &seg })));
    doc.insert("device".into(), Value::Object(info));
    if !banks.is_empty() {
        doc.insert("programs".into(), json!({"program_change": pitched, "banks": banks}));
    }
    if !entries.is_empty() {
        doc.insert("cc_params".into(), json!({"entries": entries}));
    }
    if !triggers.is_empty() {
        doc.insert("triggers".into(), json!({"entries": triggers}));
    }
    let read = osd::read(&Value::Object(doc).to_string(), osd::Kind::Device)
        .map_err(|f| invalid(format!("line {}", master.line), format!("the document does not make a device descriptor: {} at {}", f.rule, f.pointer)))?;
    let osd::Descriptor::Device(mut device) = read else {
        return Err(invalid(format!("line {}", root.line), "the import does not make a device"));
    };
    device.sources = Some(vec![source.clone()]);
    Ok(Translated { value: *device, lost: losses })
}

// ---- export -----------------------------------------------------------------------------

/// What a control is written with.
enum Binding {
    Cc(u8),
    Cc14(u8),
    Nrpn(u16),
    Rpn(u16),
}

impl Binding {
    fn type_and_number(&self) -> (&'static str, String) {
        match self {
            Binding::Cc(n) => ("7bit", n.to_string()),
            Binding::Cc14(n) => ("14bit", n.to_string()),
            Binding::Nrpn(n) => ("NRPN", n.to_string()),
            Binding::Rpn(n) => ("RPN", n.to_string()),
        }
    }

    fn span(&self) -> i64 {
        match self {
            Binding::Cc(_) => 127,
            _ => 16383,
        }
    }
}

struct Writer {
    out: String,
    depth: usize,
}

impl Writer {
    fn tag(&mut self, name: &str, attrs: &[(&str, String)], close: &str) {
        let attrs: String = attrs.iter().map(|(k, v)| format!(" {k}=\"{}\"", escape(v))).collect();
        self.out.push_str(&format!("{}<{name}{attrs}{close}>\n", "  ".repeat(self.depth)));
    }

    fn open(&mut self, name: &str, attrs: &[(&str, String)]) {
        self.tag(name, attrs, "");
        self.depth += 1;
    }

    fn end(&mut self, name: &str) {
        self.depth -= 1;
        self.out.push_str(&format!("{}</{name}>\n", "  ".repeat(self.depth)));
    }

    fn empty(&mut self, name: &str, attrs: &[(&str, String)]) {
        self.tag(name, attrs, "/");
    }

    fn text(&mut self, name: &str, text: &str) {
        self.out.push_str(&format!("{}<{name}>{}</{name}>\n", "  ".repeat(self.depth), escape(text)));
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

struct Control {
    name: String,
    binding: Binding,
    range: Option<[i64; 2]>,
    default: Option<i64>,
}

fn controls_of(d: &Device, losses: &mut Vec<Loss>) -> Vec<Control> {
    let mut out = Vec::new();
    let Some(cc) = &d.cc_params else { return out };
    if cc.channel.is_some() {
        loss(losses, "/cc_params/channel", "MIDNAM names the controls of a channel set, it does not give a channel to them");
    }
    for key in cc.remarks.keys() {
        loss(losses, format!("/cc_params/{}", segment(key)), "the DTD has no element for it");
    }
    for (n, e) in cc.entries.iter().enumerate() {
        let at = format!("/cc_params/entries/{n}");
        let name = text_remark(&e.remarks, "_name", &at, losses).unwrap_or_else(|| address_words(&e.osc));
        for (field, present) in [
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
                loss(losses, format!("{at}/{field}"), "a Control of the DTD has a type, a number, a name and values: nothing for it");
            }
        }
        for key in e.remarks.keys().filter(|k| *k != "_name") {
            loss(losses, format!("{at}/{}", segment(key)), "the DTD has no element for it");
        }
        let mut bindings: Vec<(Binding, &'static str)> = Vec::new();
        match (e.cc, e.cc_lsb) {
            (Some(cc), Some(lsb)) if cc <= 95 && lsb == cc + 32 => bindings.push((Binding::Cc14(cc), "cc")),
            (Some(cc), lsb) => {
                if lsb.is_some() {
                    loss(losses, format!("{at}/cc_lsb"), "a 14-bit control of the DTD pairs a controller with the one 32 above it");
                }
                bindings.push((Binding::Cc(cc), "cc"));
            }
            (None, Some(_)) => loss(losses, format!("{at}/cc_lsb"), "an LSB without its CC"),
            (None, None) => {}
        }
        for (msb, lsb, field, make) in [
            (e.nrpn_msb, e.nrpn_lsb, "nrpn", Binding::Nrpn as fn(u16) -> Binding),
            (e.rpn_msb, e.rpn_lsb, "rpn", Binding::Rpn as fn(u16) -> Binding),
        ] {
            match (msb, lsb) {
                (Some(m), Some(l)) => bindings.push((make(u16::from(m) << 7 | u16::from(l)), field)),
                (None, None) => {}
                (Some(_), None) => loss(losses, format!("{at}/{field}_msb"), format!("an {field} needs its MSB and its LSB")),
                (None, Some(_)) => loss(losses, format!("{at}/{field}_lsb"), format!("an {field} needs its MSB and its LSB")),
            }
        }
        if bindings.is_empty() {
            loss(losses, at, "the control has no complete MIDI binding to write");
            continue;
        }
        let (binding, _) = bindings.remove(0);
        for (_, field) in bindings {
            loss(losses, format!("{at}/{field}"), "a Control of the DTD has one type and one number: the first binding is written");
        }
        let range = match e.range {
            Some([lo, hi]) if lo > hi => {
                loss(losses, format!("{at}/range"), "the lowest value is above the highest");
                None
            }
            r => r,
        };
        out.push(Control { name, binding, range, default: e.default });
    }
    out
}

/// A bank's name when a patch is not named by the descriptor.
fn patch_name(bank: &str, number: &str) -> String {
    format!("{bank} {number}")
}

pub(crate) fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    let Some(sources) = d.sources.as_ref().filter(|s| !s.is_empty()) else {
        return Err(fault(FaultKind::NoSources, "/_sources", "a device without sources is not written: its credit and licence cannot be"));
    };
    let mut losses = Vec::new();
    let info = &d.device;
    loss(&mut losses, "/device/osc_prefix", "MIDNAM names a device's controls, it does not address them");
    if info.vendor.is_none() {
        loss(&mut losses, "/device/vendor", "the descriptor names no vendor: the Manufacturer is written empty, nothing is invented");
    }
    for (pointer, present) in [
        ("/device/revision", info.revision.is_some()),
        ("/device/manufacturer_id", info.manufacturer_id.is_some()),
        ("/device/device_id", info.device_id.is_some()),
        ("/device/rate_limit_hz", info.rate_limit_hz.is_some()),
        ("/device/kind", info.kind.is_some()),
        ("/device/transport", info.transport.is_some()),
    ] {
        if present {
            loss(&mut losses, pointer, "the DTD has no element for it");
        }
    }
    for key in info.remarks.keys() {
        loss(&mut losses, format!("/device/{}", segment(key)), "the DTD has no element for it");
    }
    for (pointer, present) in [
        ("/capabilities", d.capabilities.is_some()),
        ("/role_vocabulary", d.role_vocabulary.is_some()),
        ("/midi_of", d.midi_of.is_some()),
        ("/loads_sounds_of", d.loads_sounds_of.is_some()),
        ("/identity", d.identity.is_some()),
        ("/sysex", d.sysex.is_some()),
        ("/midi_out", d.midi_out.is_some()),
        ("/midi_in", d.midi_in.is_some()),
        ("/replies", d.replies.is_some()),
        ("/subscriptions", d.subscriptions.is_some()),
        ("/custom_commands", d.custom_commands.is_some()),
        ("/_coverage", d.coverage.is_some()),
    ] {
        if present {
            loss(&mut losses, pointer, "MIDNAM names controls, notes and patches: it holds nothing of this");
        }
    }
    for key in d.remarks.keys() {
        loss(&mut losses, format!("/{}", segment(key)), "the DTD has no element for it");
    }
    for (n, s) in sources.iter().enumerate() {
        for field in uncarried_source_fields(s) {
            loss(&mut losses, format!("/_sources/{n}/{}", segment(&field)), "the Author carries the credit and the licence of a source, nothing more");
        }
    }
    if let Some(p) = &d.params {
        for field in ["get_frame", "set_frame", "value_type"] {
            loss(&mut losses, format!("/params/{field}"), "a SysEx parameter is no control of the DTD");
        }
        for key in p.remarks.keys() {
            loss(&mut losses, format!("/params/{}", segment(key)), "the DTD has no element for it");
        }
        for n in 0..p.entries.len() {
            loss(&mut losses, format!("/params/entries/{n}"), "a SysEx parameter is no control of the DTD: a Control is a Control Change, an NRPN or an RPN");
        }
    }
    for n in 0..d.commands.iter().flatten().count() {
        loss(&mut losses, format!("/commands/{n}"), "a command is a message sent on demand: MIDNAM names what a device holds, it sends nothing");
    }

    let controls = controls_of(d, &mut losses);

    // The notes of the triggers.
    let mut notes: Vec<(i64, &str)> = Vec::new();
    if let Some(t) = &d.triggers {
        for key in t.remarks.keys() {
            loss(&mut losses, format!("/triggers/{}", segment(key)), "the DTD has no element for it");
        }
        for (n, e) in t.entries.iter().enumerate() {
            let at = format!("/triggers/entries/{n}");
            let number = match (e.note, e.note_range) {
                (Some(note), range) => {
                    if range != [i64::from(note), i64::from(note)] {
                        loss(&mut losses, format!("{at}/note_range"), "a Note has one number: the note of the trigger is written");
                    }
                    Some(i64::from(note))
                }
                (None, [lo, hi]) if lo == hi => Some(lo),
                (None, _) => {
                    loss(&mut losses, format!("{at}/note_range"), "the trigger gives a range and no note: a Note has one number, none is chosen");
                    None
                }
            };
            for (field, present) in [
                ("section", e.section.is_some()),
                ("channel", e.channel.is_some()),
                ("channel_mode", e.channel_mode.is_some()),
                ("velocity_usage", e.velocity_usage.is_some()),
                ("comments", e.comments.is_some()),
            ] {
                if present {
                    loss(&mut losses, format!("{at}/{field}"), "a Note of the DTD has a number and a name: nothing for it");
                }
            }
            for key in e.remarks.keys() {
                loss(&mut losses, format!("{at}/{}", segment(key)), "the DTD has no element for it");
            }
            if let Some(number) = number {
                notes.push((number, e.sound.as_str()));
            }
        }
    }

    let mut w = Writer { out: String::new(), depth: 0 };
    w.out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    w.out.push_str(&format!("<!DOCTYPE MIDINameDocument PUBLIC \"{PUBLIC_ID}\" \"{SYSTEM_ID}\">\n"));
    w.open("MIDINameDocument", &[]);
    w.text("Author", &credit_text(sources));
    w.open("MasterDeviceNames", &[]);
    w.text("Manufacturer", info.vendor.as_deref().unwrap_or(""));
    w.text("Model", &info.name);
    w.open("CustomDeviceMode", &[("Name", "Default".into())]);
    w.open("ChannelNameSetAssignments", &[]);
    for c in 1..=16 {
        w.empty("ChannelNameSetAssign", &[("Channel", c.to_string()), ("NameSet", "Names".into())]);
    }
    w.end("ChannelNameSetAssignments");
    w.end("CustomDeviceMode");

    w.open("ChannelNameSet", &[("Name", "Names".into())]);
    w.open("AvailableForChannels", &[]);
    for c in 1..=16 {
        w.empty("AvailableChannel", &[("Channel", c.to_string()), ("Available", "true".into())]);
    }
    w.end("AvailableForChannels");
    if !notes.is_empty() {
        w.empty("UsesNoteNameList", &[("Name", "Notes".into())]);
    }
    if !controls.is_empty() {
        w.empty("UsesControlNameList", &[("Name", "Controls".into())]);
    }
    let banks = d.programs.as_ref().map(|p| p.banks.iter().flatten().collect::<Vec<_>>()).unwrap_or_default();
    if let Some(p) = &d.programs {
        for (field, present) in [
            ("profile", p.profile.is_some()),
            ("selects", p.selects.is_some()),
            ("requires", p.requires.is_some()),
            ("before", p.before.is_some()),
            ("after", p.after.is_some()),
            ("recall_script", p.recall_script.is_some()),
            ("delay_ms", p.delay_ms.is_some()),
            ("dump", p.dump.is_some()),
            ("notes", p.notes.is_some()),
        ] {
            if present {
                loss(&mut losses, format!("/programs/{field}"), "a PatchBank names patches and selects a bank: nothing for how a recall is made");
            }
        }
        if p.program_change == Some(false) {
            loss(&mut losses, "/programs/program_change", "a Patch of the DTD carries a program change number: the device's choice not to use it is not said");
        }
        for key in p.remarks.keys() {
            loss(&mut losses, format!("/programs/{}", segment(key)), "the DTD has no element for it");
        }
    }
    if banks.is_empty() {
        w.open("PatchBank", &[]);
        w.empty("PatchNameList", &[]);
        w.end("PatchBank");
    }
    for (n, b) in banks.iter().enumerate() {
        let at = format!("/programs/banks/{n}");
        for (field, present) in [("display", b.display.is_some()), ("channel", b.channel.is_some()), ("selects", b.selects.is_some())] {
            if present {
                loss(&mut losses, format!("{at}/{field}"), "a PatchBank has a name and a bank select: nothing for it");
            }
        }
        for key in b.remarks.keys() {
            loss(&mut losses, format!("{at}/{}", segment(key)), "the DTD has no element for it");
        }
        w.open("PatchBank", &[("Name", b.name.clone())]);
        if b.msb.is_some() || b.lsb.is_some() {
            w.open("MIDICommands", &[]);
            for (control, value) in [(0, b.msb), (32, b.lsb)] {
                if let Some(v) = value {
                    w.empty("ControlChange", &[("Control", control.to_string()), ("Value", v.to_string())]);
                }
            }
            w.end("MIDICommands");
        }
        w.open("PatchNameList", &[("Name", b.name.clone())]);
        let first = b.first_number.unwrap_or(0);
        for i in 0..b.size {
            let number = first + i;
            let mut attrs = vec![("Number", number.to_string()), ("Name", patch_name(&b.name, &number.to_string()))];
            if let Some(pc) = b.first_pc.map(|f| i64::from(f) + i).filter(|pc| *pc <= 127) {
                attrs.push(("ProgramChange", pc.to_string()));
            }
            w.empty("Patch", &attrs);
        }
        w.end("PatchNameList");
        w.end("PatchBank");
    }
    w.end("ChannelNameSet");

    if !notes.is_empty() {
        w.open("NoteNameList", &[("Name", "Notes".into())]);
        for (number, name) in &notes {
            w.empty("Note", &[("Number", number.to_string()), ("Name", (*name).to_string())]);
        }
        w.end("NoteNameList");
    }
    if !controls.is_empty() {
        w.open("ControlNameList", &[("Name", "Controls".into())]);
        for c in &controls {
            let (kind, number) = c.binding.type_and_number();
            w.open("Control", &[("Type", kind.into()), ("Number", number), ("Name", c.name.clone())]);
            let mut values = vec![("Min", c.range.map_or(0, |r| r[0]).to_string()), ("Max", c.range.map_or(c.binding.span(), |r| r[1]).to_string())];
            if let Some(default) = c.default {
                values.push(("Default", default.to_string()));
            }
            w.empty("Values", &values);
            w.end("Control");
        }
        w.end("ControlNameList");
    }
    w.end("MasterDeviceNames");
    w.end("MIDINameDocument");
    Ok(Translated { value: w.out, lost: losses })
}
