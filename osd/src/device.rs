//! The device descriptor of OSD 0.4 (`schemas/0.4/device.schema.json`) and
//! the maker profile (`schemas/0.4/profile.schema.json`), whose blocks are
//! the device's.

use crate::json::{
    at_least, closed, int, list, obj, object, words, Any, At, Bool, Codec, Dict, Nullable, Object, Opt, Pair,
    Word, ADDRESS, INTEGER, STRING,
};
use crate::address::{
    check_address, convert_address, name_ok, named, split_template, Address, AddressCodec, AddressPlace,
};
use crate::frame::{
    check_expr, check_frame, convert_expr, convert_token, Expr, FrameToken, Place, EXPR, FRAME_TOKEN, MATCH_TOKEN,
    REPLY_TOKEN,
};
use crate::vocabulary;
use crate::{Capability, Fault, FaultKind, SchemaUrlFrom05, Version};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

/// A byte, 0 to 255.
const BYTE: crate::json::Int<u8> = int(0, 255);
/// A 7-bit MIDI value, 0 to 127.
const U7: crate::json::Int<u8> = int(0, 127);
/// A MIDI channel, 0 to 15 (channel 1 is 0).
const CHANNEL: crate::json::Int<u8> = int(0, 15);

object! {
    /// A device descriptor: how a synthesizer, drum machine, controller or
    /// music software speaks MIDI or OSC, from one source and for one
    /// firmware.
    Device {
        "$schema" schema: Option<Version> = Opt(SchemaUrlFrom05("device")),
        "device" device: DeviceInfo = obj(),
        /// What a reader needs beyond the declarative fields (0.5): the list
        /// the reader derives from the fields present, nothing else.
        "capabilities" capabilities: Option<Vec<Capability>> = Opt(list(closed())),
        /// The driver whose MIDI this firmware answers to unchanged
        /// (`maker/file`): this file then only says what differs.
        "midi_of" midi_of: Option<String> = Opt(STRING),
        /// Devices (their address prefix without `/`) whose sounds this one
        /// loads as they are.
        "loads_sounds_of" loads_sounds_of: Option<Vec<String>> = Opt(list(STRING)),
        "identity" identity: Option<Identity> = Opt(obj()),
        "programs" programs: Option<Programs> = Opt(obj()),
        "sysex" sysex: Option<Sysex> = Opt(obj()),
        "cc_params" cc_params: Option<CcParams> = Opt(obj()),
        "params" params: Option<Params> = Opt(obj()),
        "commands" commands: Option<Vec<Command>> = Opt(list(obj())),
        "triggers" triggers: Option<Triggers> = Opt(obj()),
        "midi_out" midi_out: Option<MidiOut> = Opt(obj()),
        "midi_in" midi_in: Option<MidiIn> = Opt(MidiInCodec),
        "replies" replies: Option<Vec<Reply>> = Opt(list(obj())),
        "subscriptions" subscriptions: Option<Vec<Subscription>> = Opt(list(obj())),
        /// osc-bridge extension: addresses handled by a script only.
        "custom_commands" custom_commands: Option<Vec<CustomCommand>> = Opt(list(obj())),
        /// Where the data comes from, with its credit, licence and the
        /// firmware it documents.
        "_sources" sources: Option<Vec<Source>> = Opt(list(obj())),
        /// What the source covers, counted.
        "_coverage" coverage: Option<BTreeMap<String, Value>> = Opt(Dict(Any)),
    }
    check check_device
}

object! {
    /// A maker profile: what several devices of a maker or of a family
    /// share, taken up by the device blocks that name it.
    Profile {
        "$schema" schema: Option<Version> = Opt(SchemaUrlFrom05("profile")),
        "identity" identity: Option<Identity> = Opt(obj()),
        "programs" programs: Option<Programs> = Opt(obj()),
    }
    check check_profile
}

object! {
    /// The bytes every SysEx message of the device starts and ends with.
    Sysex {
        "header" header: Option<Vec<u8>> = Opt(list(BYTE)),
        "footer" footer: Option<Vec<u8>> = Opt(list(BYTE)),
    }
}

object! {
    /// Controls set by Control Change, 14-bit CC pairs, NRPN or RPN.
    CcParams {
        "channel" channel: Option<u8> = Opt(CHANNEL),
        "entries" entries: Vec<CcEntry> = list(obj()),
    }
}

words! {
    /// The size of a SysEx parameter's value.
    ValueType { U7 = "u7", U14 = "u14", S32 = "s32", }
}

object! {
    /// Parameters read and written by SysEx through one shared message shape.
    Params {
        "get_frame" get_frame: Vec<FrameToken> = list(FRAME_TOKEN),
        "set_frame" set_frame: Vec<FrameToken> = list(FRAME_TOKEN),
        "value_type" value_type: ValueType = closed(),
        "entries" entries: Vec<ParamEntry> = list(obj()),
    }
}

object! {
    /// Drum and sample sounds fired by a note.
    Triggers {
        "entries" entries: Vec<Trigger> = list(obj()),
    }
}

object! {
    /// Playing it: notes, pitch bend, aftertouch, program change.
    MidiOut {
        "default_channel" default_channel: Option<u8> = Opt(CHANNEL),
        /// Added to every note number sent.
        "note_offset" note_offset: Option<i64> = Opt(INTEGER),
    }
}

words! {
    /// A MIDI message a device sends while played, in the order of the schema.
    MidiType {
        NoteOn = "note_on", NoteOff = "note_off", Cc = "cc", Pitchbend = "pitchbend", Aftertouch = "aftertouch",
        PolyAftertouch = "poly_aftertouch", ProgramChange = "program_change",
    }
}

/// The OSC address each MIDI message is reported at, under the device's
/// prefix, with its OSC arguments after the space in this order: fixed by
/// the specification.
pub(crate) const MIDI_TEMPLATES: [(MidiType, &str); 7] = [
    (MidiType::NoteOn, "/note/on {note} {velocity} {channel}"),
    (MidiType::NoteOff, "/note/off {note} {velocity} {channel}"),
    (MidiType::Cc, "/cc/{num} {value} {channel}"),
    (MidiType::Pitchbend, "/pitchbend {value_u14} {channel}"),
    (MidiType::Aftertouch, "/aftertouch {value} {channel}"),
    (MidiType::PolyAftertouch, "/poly_aftertouch {note} {value} {channel}"),
    (MidiType::ProgramChange, "/program_change {program} {channel}"),
];

/// What the device sends while played that is reported (an osc-bridge
/// extension).
#[derive(Clone, Debug, PartialEq)]
pub enum MidiIn {
    /// 0.4: the address each message is reported at.
    Templates(MidiTemplates),
    /// 0.5: the types of the messages reported, whose addresses the
    /// specification fixes ([`MIDI_TEMPLATES`]).
    Types(Vec<MidiType>),
}

pub(crate) struct MidiInCodec;

impl Codec for MidiInCodec {
    type T = MidiIn;
    fn read(&self, v: &Value, at: &At) -> Result<MidiIn, Fault> {
        match v {
            Value::Array(_) => {
                let types = list(closed::<MidiType>()).read(v, at)?;
                if let Some(i) = (1..types.len()).find(|i| types[..*i].contains(&types[*i])) {
                    return Err(at.fault(FaultKind::BadValue, format!("each type once ({} is twice)", types[i].word())));
                }
                Ok(MidiIn::Types(types))
            }
            Value::Object(_) => obj::<MidiTemplates>().read(v, at).map(MidiIn::Templates),
            _ => Err(at.fault(FaultKind::BadValue, "a list of message types")),
        }
    }
    fn write(&self, t: &MidiIn) -> Option<Value> {
        match t {
            MidiIn::Templates(x) => Some(x.write()),
            MidiIn::Types(l) => list(closed::<MidiType>()).write(l),
        }
    }
}

object! {
    /// What the device sends while played, and the address (an osc-bridge
    /// extension) each message is reported at, in 0.4.
    MidiTemplates {
        "note_on" note_on: Option<String> = Opt(STRING),
        "note_off" note_off: Option<String> = Opt(STRING),
        "cc" cc: Option<String> = Opt(STRING),
        "pitchbend" pitchbend: Option<String> = Opt(STRING),
        "aftertouch" aftertouch: Option<String> = Opt(STRING),
        "poly_aftertouch" poly_aftertouch: Option<String> = Opt(STRING),
        "program_change" program_change: Option<String> = Opt(STRING),
    }
}

words! {
    /// When a subscription is sent.
    SubscriptionOn { Startup = "startup", }
}

object! {
    /// An OSC message a software needs at start to report its changes.
    Subscription {
        "on" on: Option<SubscriptionOn> = Opt(closed()),
        "forward" forward: Forward = obj(),
    }
}

words! {
    /// The profile a script command runs under.
    ScriptProfile { Default = "default", PresetIngest = "preset_ingest", }
}

object! {
    /// osc-bridge extension: an address handled by a script only.
    CustomCommand {
        "osc" osc: String = ADDRESS,
        /// A Lua script that builds the message.
        "script" script: String = STRING,
        "profile" profile: Option<ScriptProfile> = Opt(closed()),
        "wrap_sysex" wrap_sysex: Option<bool> = Opt(Bool),
    }
}

// ---- the frames and expressions of a descriptor, by version ----------------

/// Every frame of an identity, with the JSON pointer of its place.
fn identity_frames<'a>(i: &'a Identity, at: &str, out: &mut Vec<(String, Place, &'a [FrameToken])>) {
    if let Some(r) = &i.request {
        out.push((format!("{at}/request"), Place::Sent, r));
    }
    if let Some(r) = &i.reply {
        out.push((format!("{at}/reply"), Place::Reply, r));
    }
}

fn programs_frames<'a>(p: &'a Programs, at: &str, out: &mut Vec<(String, Place, &'a [FrameToken])>) {
    for (n, f) in p.before.iter().flatten().enumerate() {
        out.push((format!("{at}/before/{n}"), Place::Sent, f));
    }
    for (n, f) in p.after.iter().flatten().enumerate() {
        out.push((format!("{at}/after/{n}"), Place::Sent, f));
    }
    if let Some(r) = p.dump.as_ref().and_then(|d| d.request.as_ref()) {
        out.push((format!("{at}/dump/request"), Place::Sent, r));
    }
}

/// The fields that enter with 0.5: in 0.4 each is an unknown field.
fn check_05_fields(i: Option<&Identity>, p: Option<&Programs>, v05: bool) -> Result<(), Fault> {
    if v05 {
        return Ok(());
    }
    let present: [(&str, bool); 7] = [
        ("/identity/firmware/script", i.and_then(|i| i.firmware.as_ref()).and_then(|f| f.as_ref()).is_some_and(|f| f.script.is_some())),
        ("/identity/to", i.is_some_and(|i| i.to.is_some())),
        ("/identity/script", i.is_some_and(|i| i.script.is_some())),
        ("/programs/after", p.is_some_and(|p| p.after.is_some())),
        ("/programs/recall_script", p.is_some_and(|p| p.recall_script.is_some())),
        ("/programs/dump/script", p.is_some_and(|p| p.dump.as_ref().is_some_and(|d| d.script.is_some()))),
        ("", false),
    ];
    match present.iter().find(|(_, here)| *here) {
        Some((pointer, _)) => Err(Fault {
            kind: FaultKind::UnknownField,
            file: None,
            pointer: (*pointer).into(),
            rule: format!("{} is not a field of 0.4", pointer.rsplit('/').next().unwrap_or_default()),
        }),
        None => Ok(()),
    }
}

impl Device {
    /// The capabilities the fields present require: Lua when the device
    /// carries any field that holds Lua (the osc-bridge extension fields of
    /// 0.4 and four of 0.5: `identity.script`, `identity.firmware.script`,
    /// `programs.recall_script`, `programs.dump.script`). `identity.to` and
    /// `programs.after` hold none.
    pub fn derived_capabilities(&self) -> Vec<Capability> {
        let d = self;
        let cc = d.cc_params.iter().flat_map(|t| &t.entries).any(|e| e.transform.is_some() || e.transform_reverse.is_some());
        let params = d.params.iter().flat_map(|t| &t.entries).any(|e| e.transform.is_some() || e.transform_reverse.is_some());
        let commands = d.commands.iter().flatten().any(|c| c.script.is_some());
        let replies = d.replies.iter().flatten().any(|r| r.script.is_some());
        let custom = d.custom_commands.as_ref().is_some_and(|c| !c.is_empty());
        let identity = d.identity.as_ref().is_some_and(|i| {
            i.script.is_some() || i.firmware.iter().flatten().any(|f| f.script.is_some() || f.decode == "lua")
        });
        let programs = d.programs.as_ref().is_some_and(|p| p.recall_script.is_some() || p.dump.as_ref().is_some_and(|x| x.script.is_some()));
        if cc || params || commands || replies || custom || identity || programs { vec![Capability::Lua] } else { Vec::new() }
    }
}

/// A declared `capabilities` equals the derived list.
fn check_capabilities(d: &Device) -> Result<(), Fault> {
    match &d.capabilities {
        Some(declared) if *declared != d.derived_capabilities() => Err(Fault {
            kind: FaultKind::BadValue,
            file: None,
            pointer: "/capabilities".into(),
            rule: format!(
                "the list the fields present require is [{}]",
                d.derived_capabilities().iter().map(|c| c.word()).collect::<Vec<_>>().join(", ")
            ),
        }),
        None if d.schema == Some(Version::V0_5) && !d.derived_capabilities().is_empty() => Err(Fault {
            kind: FaultKind::BadValue,
            file: None,
            pointer: "/capabilities".into(),
            rule: "the fields present require lua: declare it".into(),
        }),
        _ => Ok(()),
    }
}

fn check_identity_vocabularies(i: &Identity) -> Result<(), Fault> {
    if let Some(Some(f)) = &i.firmware {
        vocabulary::check_decode(&f.decode, f.field.is_some(), f.script.is_some(), "/identity/firmware")?;
    }
    Ok(())
}

fn check_programs_vocabularies(p: &Programs) -> Result<(), Fault> {
    if let Some(s) = &p.selects {
        vocabulary::check_selects(s, "/programs/selects")?;
    }
    for (n, b) in p.banks.iter().flatten().enumerate() {
        if let Some(s) = &b.selects {
            vocabulary::check_selects(s, &format!("/programs/banks/{n}/selects"))?;
        }
    }
    Ok(())
}

/// The closed vocabularies of a 0.5 device.
fn check_vocabularies(d: &Device) -> Result<(), Fault> {
    if let Some(i) = &d.identity {
        check_identity_vocabularies(i)?;
    }
    if let Some(p) = &d.programs {
        check_programs_vocabularies(p)?;
    }
    if let Some(c) = &d.cc_params {
        for (n, e) in c.entries.iter().enumerate() {
            if let Some(x) = &e.expects {
                vocabulary::check_expects(x, &format!("/cc_params/entries/{n}/expects"))?;
            }
        }
    }
    if let Some(p) = &d.params {
        for (n, e) in p.entries.iter().enumerate() {
            if let Some(x) = &e.expects {
                vocabulary::check_expects(x, &format!("/params/entries/{n}/expects"))?;
            }
        }
    }
    for (n, s) in d.sources.iter().flatten().enumerate() {
        vocabulary::check_source(&s.kind, s.license.as_deref(), &format!("/_sources/{n}"))?;
    }
    Ok(())
}

/// Whether the frames and expressions of a descriptor are the forms of its
/// version: strings in 0.4 (no `$schema`), structures in 0.5.
fn check_device(d: &Device, _: &At) -> Result<(), Fault> {
    let v05 = d.schema == Some(Version::V0_5);
    check_05_fields(d.identity.as_ref(), d.programs.as_ref(), v05)?;
    if !v05 && d.capabilities.is_some() {
        return Err(Fault { kind: FaultKind::UnknownField, file: None, pointer: "/capabilities".into(), rule: "capabilities is not a field of 0.4".into() });
    }
    if v05 {
        check_vocabularies(d)?;
        check_capabilities(d)?;
    }
    let mut frames = Vec::new();
    if let Some(i) = &d.identity {
        identity_frames(i, "/identity", &mut frames);
    }
    if let Some(p) = &d.programs {
        programs_frames(p, "/programs", &mut frames);
    }
    if let Some(p) = &d.params {
        frames.push(("/params/get_frame".into(), Place::Sent, &p.get_frame));
        frames.push(("/params/set_frame".into(), Place::Sent, &p.set_frame));
    }
    for (n, c) in d.commands.iter().flatten().enumerate() {
        check_address(&c.osc, AddressPlace::Command, &format!("/commands/{n}/osc"), v05)?;
        if let Some(f) = &c.forward {
            f.check(&format!("/commands/{n}/forward"), v05)?;
        }
        if let Some(f) = &c.frame {
            frames.push((format!("/commands/{n}/frame"), Place::Sent, f));
        }
        for (m, p) in c.pre.iter().flatten().enumerate() {
            check_expr(&p.expr, &format!("/commands/{n}/pre/{m}/expr"), v05)?;
        }
    }
    for (n, s) in d.subscriptions.iter().flatten().enumerate() {
        s.forward.check(&format!("/subscriptions/{n}/forward"), v05)?;
    }
    match (&d.midi_in, v05) {
        (Some(MidiIn::Templates(_)), true) => {
            return Err(Fault {
                kind: FaultKind::BadValue,
                file: None,
                pointer: "/midi_in".into(),
                rule: "a list of message types (an object of addresses is 0.4)".into(),
            })
        }
        (Some(MidiIn::Types(_)), false) => {
            return Err(Fault {
                kind: FaultKind::BadValue,
                file: None,
                pointer: "/midi_in".into(),
                rule: "an object of addresses (a list of message types is 0.5)".into(),
            })
        }
        _ => {}
    }
    for (n, r) in d.replies.iter().flatten().enumerate() {
        let at = format!("/replies/{n}");
        if let Some(m) = &r.match_osc {
            check_address(m, AddressPlace::Recognised, &format!("{at}/match_osc"), v05)?;
        }
        check_address(&r.emit_osc, AddressPlace::Reported, &format!("{at}/emit_osc"), v05)?;
        match (&r.emit_args, v05) {
            (Some(_), false) => {
                return Err(Fault {
                    kind: FaultKind::UnknownField,
                    file: None,
                    pointer: format!("{at}/emit_args"),
                    rule: "emit_args is not a field of 0.4 (the arguments follow the first space of emit_osc)".into(),
                })
            }
            (Some(a), true) => check_scalars(a, &format!("{at}/emit_args"), v05)?,
            _ => {}
        }
        if let Some(f) = &r.match_frame {
            frames.push((format!("/replies/{n}/match_frame"), Place::Matched, f));
        }
    }
    for (pointer, place, tokens) in frames {
        check_frame(tokens, place, &pointer, v05)?;
    }
    Ok(())
}

fn check_profile(p: &Profile, _: &At) -> Result<(), Fault> {
    let v05 = p.schema == Some(Version::V0_5);
    check_05_fields(p.identity.as_ref(), p.programs.as_ref(), v05)?;
    if v05 {
        if let Some(i) = &p.identity {
            check_identity_vocabularies(i)?;
        }
        if let Some(x) = &p.programs {
            check_programs_vocabularies(x)?;
        }
    }
    let mut frames = Vec::new();
    if let Some(i) = &p.identity {
        identity_frames(i, "/identity", &mut frames);
    }
    if let Some(x) = &p.programs {
        programs_frames(x, "/programs", &mut frames);
    }
    for (pointer, place, tokens) in frames {
        check_frame(tokens, place, &pointer, v05)?;
    }
    Ok(())
}

fn convert_frame(tokens: &mut [FrameToken], place: Place, at: &At) -> Result<(), Fault> {
    for (i, t) in tokens.iter_mut().enumerate() {
        *t = convert_token(t, place, &At::Index(at, i))?;
    }
    Ok(())
}

fn convert_identity(i: &mut Identity, at: &At) -> Result<(), Fault> {
    if let Some(Some(f)) = &i.firmware {
        vocabulary::convert_decode(&f.decode, f.field.is_some(), f.script.is_some(), &At::Key(at, "firmware").pointer())?;
    }
    if let Some(r) = &mut i.request {
        convert_frame(r, Place::Sent, &At::Key(at, "request"))?;
    }
    if let Some(r) = &mut i.reply {
        convert_frame(r, Place::Reply, &At::Key(at, "reply"))?;
    }
    Ok(())
}

fn convert_programs(p: &mut Programs, at: &At) -> Result<(), Fault> {
    if let Some(s) = &mut p.selects {
        *s = vocabulary::convert_selects(s, &At::Key(at, "selects").pointer())?;
    }
    let banks = At::Key(at, "banks");
    for (n, b) in p.banks.iter_mut().flatten().enumerate() {
        if let Some(s) = &mut b.selects {
            *s = vocabulary::convert_selects(s, &At::Key(&At::Index(&banks, n), "selects").pointer())?;
        }
    }
    let before = At::Key(at, "before");
    for (n, f) in p.before.iter_mut().flatten().enumerate() {
        convert_frame(f, Place::Sent, &At::Index(&before, n))?;
    }
    let after = At::Key(at, "after");
    for (n, f) in p.after.iter_mut().flatten().enumerate() {
        convert_frame(f, Place::Sent, &At::Index(&after, n))?;
    }
    if let Some(r) = p.dump.as_mut().and_then(|d| d.request.as_mut()) {
        convert_frame(r, Place::Sent, &At::Key(&At::Key(at, "dump"), "request"))?;
    }
    Ok(())
}

impl Device {
    /// The device with every 0.4 frame element and `pre` expression turned
    /// into its 0.5 structure; the first that has none is a fault at its
    /// address.
    pub(crate) fn into_structures(mut self) -> Result<Device, Fault> {
        let root = At::Root;
        if let Some(i) = &mut self.identity {
            convert_identity(i, &At::Key(&root, "identity"))?;
        }
        if let Some(p) = &mut self.programs {
            convert_programs(p, &At::Key(&root, "programs"))?;
        }
        if let Some(c) = &mut self.cc_params {
            let at = At::Key(&At::Key(&root, "cc_params"), "entries");
            for (n, e) in c.entries.iter_mut().enumerate() {
                if let Some(x) = &mut e.expects {
                    *x = vocabulary::convert_expects(x, &At::Key(&At::Index(&at, n), "expects").pointer())?;
                }
            }
        }
        if let Some(p) = &mut self.params {
            let at = At::Key(&At::Key(&root, "params"), "entries");
            for (n, e) in p.entries.iter_mut().enumerate() {
                if let Some(x) = &mut e.expects {
                    *x = vocabulary::convert_expects(x, &At::Key(&At::Index(&at, n), "expects").pointer())?;
                }
            }
        }
        let sources = At::Key(&root, "_sources");
        for (n, s) in self.sources.iter_mut().flatten().enumerate() {
            let at = At::Index(&sources, n);
            s.kind = vocabulary::convert_source_type(&s.kind, &At::Key(&at, "type").pointer())?;
            if let Some(l) = &s.license {
                vocabulary::convert_license(l, &At::Key(&at, "license").pointer())?;
            }
        }
        if let Some(p) = &mut self.params {
            let at = At::Key(&root, "params");
            convert_frame(&mut p.get_frame, Place::Sent, &At::Key(&at, "get_frame"))?;
            convert_frame(&mut p.set_frame, Place::Sent, &At::Key(&at, "set_frame"))?;
        }
        let commands = At::Key(&root, "commands");
        for (n, c) in self.commands.iter_mut().flatten().enumerate() {
            let at = At::Index(&commands, n);
            if let Some(f) = &mut c.frame {
                convert_frame(f, Place::Sent, &At::Key(&at, "frame"))?;
            }
            let pre = At::Key(&at, "pre");
            for (m, p) in c.pre.iter_mut().flatten().enumerate() {
                p.expr = convert_expr(&p.expr, &At::Key(&At::Index(&pre, m), "expr"))?;
            }
        }
        for (n, c) in self.commands.iter_mut().flatten().enumerate() {
            let at = At::Index(&commands, n);
            c.osc = convert_address(&c.osc, AddressPlace::Command, &At::Key(&at, "osc"))?;
            if let Some(f) = c.forward.take() {
                c.forward = Some(f.into_structures(&At::Key(&at, "forward"))?);
            }
        }
        let subscriptions = At::Key(&root, "subscriptions");
        for (n, s) in self.subscriptions.iter_mut().flatten().enumerate() {
            let at = At::Key(&At::Index(&subscriptions, n), "forward");
            s.forward = s.forward.clone().into_structures(&at)?;
        }
        if let Some(MidiIn::Templates(t)) = &self.midi_in {
            self.midi_in = Some(MidiIn::Types(t.types(&At::Key(&root, "midi_in"))?));
        }
        let replies = At::Key(&root, "replies");
        for (n, r) in self.replies.iter_mut().flatten().enumerate() {
            let at = At::Index(&replies, n);
            if let Some(f) = &mut r.match_frame {
                convert_frame(f, Place::Matched, &At::Key(&at, "match_frame"))?;
            }
            if let Some(m) = &r.match_osc {
                r.match_osc = Some(convert_address(m, AddressPlace::Recognised, &At::Key(&at, "match_osc"))?);
            }
            convert_reported(r, &At::Key(&at, "emit_osc"))?;
        }
        let derived = self.derived_capabilities();
        self.capabilities = (!derived.is_empty()).then_some(derived);
        Ok(self)
    }
}

impl MidiTemplates {
    /// The types of the messages it names, in the order of the schema; a
    /// template other than the canonical one of its type is a fault at the
    /// type.
    fn types(&self, at: &At) -> Result<Vec<MidiType>, Fault> {
        let templates = [
            (MidiType::NoteOn, &self.note_on),
            (MidiType::NoteOff, &self.note_off),
            (MidiType::Cc, &self.cc),
            (MidiType::Pitchbend, &self.pitchbend),
            (MidiType::Aftertouch, &self.aftertouch),
            (MidiType::PolyAftertouch, &self.poly_aftertouch),
            (MidiType::ProgramChange, &self.program_change),
        ];
        let mut out = Vec::new();
        for (t, template) in templates {
            let Some(template) = template else { continue };
            let canonical = MIDI_TEMPLATES.iter().find(|(x, _)| *x == t).map(|(_, c)| *c).unwrap_or_default();
            if template != canonical {
                return Err(At::Key(at, t.word()).fault(
                    FaultKind::NotConvertible,
                    format!("the template of {} is {template:?}, the specification fixes {canonical:?}", t.word()),
                ));
            }
            out.push(t);
        }
        Ok(out)
    }
}

/// The 0.5 form of a reply's `emit_osc`: the text up to the first space is
/// the address, each `{name}` after it a captured field in `emit_args`.
fn convert_reported(r: &mut Reply, at: &At) -> Result<(), Fault> {
    let Address::Text(whole) = &r.emit_osc else { return Ok(()) };
    let not = |why: String| at.fault(FaultKind::NotConvertible, format!("the reply {whole:?}: {why}"));
    let (address, rest) = whole.split_once(' ').unwrap_or((whole, ""));
    let address = split_template(address, false).map_err(not)?;
    let mut args = Vec::new();
    for a in rest.split_whitespace() {
        match a.strip_prefix('{').and_then(|x| x.strip_suffix('}')) {
            Some(n) if name_ok(n) => args.push(Scalar::Capture(n.to_string())),
            _ => return Err(not(format!("the argument {a:?} is not one {{name}}"))),
        }
    }
    r.emit_osc = address;
    if !args.is_empty() {
        r.emit_args = Some(args);
    }
    Ok(())
}

impl Profile {
    /// The profile with every 0.4 frame element turned into its 0.5
    /// structure.
    pub(crate) fn into_structures(mut self) -> Result<Profile, Fault> {
        let root = At::Root;
        if let Some(i) = &mut self.identity {
            convert_identity(i, &At::Key(&root, "identity"))?;
        }
        if let Some(p) = &mut self.programs {
            convert_programs(p, &At::Key(&root, "programs"))?;
        }
        Ok(self)
    }
}

words! {
    /// Hardware or software.
    DeviceKind { Hardware = "hardware", Software = "software", }
}

words! {
    /// How a device is reached.
    TransportKind { Midi = "midi", Osc = "osc", }
}

object! {
    /// The device itself: its name, maker, address prefix and SysEx ids.
    DeviceInfo {
        /// The model's name, as its maker writes it.
        "name" name: String = STRING,
        "vendor" vendor: Option<String> = Opt(STRING),
        /// The version this file describes (firmware or document).
        "revision" revision: Option<String> = Opt(STRING),
        /// The device's own address prefix (`/minilogue`).
        "osc_prefix" osc_prefix: String = ADDRESS,
        /// The maker's SysEx id: one byte, or three starting with 0.
        "manufacturer_id" manufacturer_id: Option<Vec<u8>> = Opt(list(U7)),
        "device_id" device_id: Option<Vec<u8>> = Opt(list(U7)),
        /// Most messages a second the device takes; 0: no limit known.
        "rate_limit_hz" rate_limit_hz: Option<i64> = Opt(at_least(0)),
        "kind" kind: Option<DeviceKind> = Opt(closed()),
        "transport" transport: Option<Transport> = Opt(obj()),
    }
}

object! {
    /// How a software is reached (OSC host and ports).
    Transport {
        "kind" kind: TransportKind = closed(),
        "host" host: Option<String> = Opt(STRING),
        "port" port: Option<i64> = Opt(INTEGER),
        "reply_port" reply_port: Option<i64> = Opt(INTEGER),
        "rate_limit_hz" rate_limit_hz: Option<i64> = Opt(INTEGER),
        "passthrough_prefix" passthrough_prefix: Option<String> = Opt(STRING),
    }
}

words! {
    /// The ports an identify request goes to.
    IdentityTo { EveryPort = "every_port", NamedPorts = "named_ports", }
}

words! {
    /// How an identity reply's bytes are unpacked.
    Unpack { Lsb8To7 = "lsb_8to7", }
}

object! {
    /// How to recognise the device and its firmware: its answer to an
    /// identify request, its port names, its USB identity, what it sends by
    /// itself.
    Identity {
        /// A maker profile this extends (`maker` or `maker/family`).
        "profile" profile: Option<String> = Opt(STRING),
        /// The identify request, when not the Universal Identity Request.
        "request" request: Option<Vec<FrameToken>> = Opt(list(FRAME_TOKEN)),
        /// Where the request goes (0.5).
        "to" to: Option<IdentityTo> = Opt(closed()),
        /// A Lua script that says whether a reply is the device's (0.5).
        "script" script: Option<String> = Opt(STRING),
        "unpack" unpack: Option<Unpack> = Opt(closed()),
        /// The start of the reply, recognised element by element.
        "reply" reply: Option<Vec<FrameToken>> = Opt(list(REPLY_TOKEN)),
        "reply_text" reply_text: Option<String> = Opt(STRING),
        "family" family: Option<Vec<u8>> = Opt(list(BYTE)),
        "member" member: Option<Vec<u8>> = Opt(list(BYTE)),
        "device_id" device_id: Option<DeviceIdRange> = Opt(obj()),
        /// Where the reply carries the firmware version; `Some(None)` is
        /// written `null`, which removes a profile's rule.
        "firmware" firmware: Option<Option<FirmwareRule>> = Opt(Nullable(obj())),
        /// Its port names, for a device that does not answer.
        "ports" ports: Option<Vec<String>> = Opt(list(STRING)),
        /// Its USB identities, one per model version or mode.
        "usb" usb: Option<Vec<UsbId>> = Opt(list(obj())),
        "sends" sends: Option<Sends> = Opt(obj()),
    }
}

object! {
    /// The device id a device answers to.
    DeviceIdRange {
        "default" default: Option<i64> = Opt(INTEGER),
        "min" min: Option<i64> = Opt(INTEGER),
        "max" max: Option<i64> = Opt(INTEGER),
    }
}

object! {
    /// Where an identity reply carries the firmware version, and how to read it.
    FirmwareRule {
        /// How the bytes read as a version: `raw_bytes`, `json`,
        /// `korg_minor_major`, `yamaha_tenths`, `ascii` or `lua` (0.5 holds
        /// no other word).
        "decode" decode: String = STRING,
        "at" at: Vec<i64> = list(INTEGER),
        /// With decode `json`: the key whose text is the version.
        "field" field: Option<String> = Opt(STRING),
        /// With decode `lua`: the script that reads the version (0.5).
        "script" script: Option<String> = Opt(STRING),
    }
}

object! {
    /// A bank of programs.
    Bank {
        "name" name: String = STRING,
        "msb" msb: Option<u8> = Opt(U7),
        "lsb" lsb: Option<u8> = Opt(U7),
        "size" size: i64 = at_least(1),
        "first_pc" first_pc: Option<u8> = Opt(U7),
        "first_number" first_number: Option<i64> = Opt(INTEGER),
        /// How the device numbers it (`A01-H16`).
        "display" display: Option<String> = Opt(STRING),
        "channel" channel: Option<u8> = Opt(CHANNEL),
        "selects" selects: Option<String> = Opt(STRING),
    }
}

object! {
    /// Its programs and banks: how to select one, how they are numbered, how
    /// a sound is taken from it.
    Programs {
        "profile" profile: Option<String> = Opt(STRING),
        /// What a program change selects (sound, pattern, kit…).
        "selects" selects: Option<String> = Opt(STRING),
        "program_change" program_change: Option<bool> = Opt(Bool),
        /// Settings on the device needed first, as its manual names them.
        "requires" requires: Option<Vec<String>> = Opt(list(STRING)),
        "banks" banks: Option<Vec<Bank>> = Opt(list(obj())),
        /// Messages sent before a recall.
        "before" before: Option<Vec<Vec<FrameToken>>> = Opt(list(list(FRAME_TOKEN))),
        /// Messages sent after a recall (0.5).
        "after" after: Option<Vec<Vec<FrameToken>>> = Opt(list(list(FRAME_TOKEN))),
        /// A Lua script that writes the recall itself (0.5).
        "recall_script" recall_script: Option<String> = Opt(STRING),
        "delay_ms" delay_ms: Option<i64> = Opt(at_least(0)),
        "dump" dump: Option<Dump> = Opt(obj()),
        "notes" notes: Option<String> = Opt(STRING),
    }
}

words! {
    /// How the current sound is taken from the device.
    DumpKind { None = "none", Request = "request", EditBuffer = "edit_buffer", Manual = "manual", }
}

object! {
    /// How the current sound is taken from the device.
    Dump {
        "kind" kind: DumpKind = closed(),
        "request" request: Option<Vec<FrameToken>> = Opt(list(FRAME_TOKEN)),
        /// A Lua script that writes the request (0.5).
        "script" script: Option<String> = Opt(STRING),
    }
}

words! {
    /// Where a control's values start.
    Orientation { ZeroBased = "0-based", Centered = "centered", }
}

object! {
    /// A control set by Control Change, a 14-bit CC pair, NRPN or RPN.
    CcEntry {
        /// The control's address, after the device's prefix (`/filter/cutoff`).
        "osc" osc: String = ADDRESS,
        "cc" cc: Option<u8> = Opt(U7),
        "cc_lsb" cc_lsb: Option<u8> = Opt(U7),
        "nrpn_msb" nrpn_msb: Option<u8> = Opt(U7),
        "nrpn_lsb" nrpn_lsb: Option<u8> = Opt(U7),
        "nrpn_lsb_range" nrpn_lsb_range: Option<[i64; 2]> = Opt(Pair),
        "rpn_msb" rpn_msb: Option<u8> = Opt(U7),
        "rpn_lsb" rpn_lsb: Option<u8> = Opt(U7),
        /// Lowest and highest value.
        "range" range: Option<[i64; 2]> = Opt(Pair),
        "orientation" orientation: Option<Orientation> = Opt(closed()),
        "channel" channel: Option<u8> = Opt(CHANNEL),
        "section" section: Option<String> = Opt(STRING),
        /// What the control expects: continuous (default), switch,
        /// momentary, trigger, clock or discrete; other words are allowed.
        "expects" expects: Option<String> = Opt(STRING),
        /// osc-bridge extension: a Lua expression applied to the value.
        "transform" transform: Option<String> = Opt(STRING),
        /// osc-bridge extension: its reverse.
        "transform_reverse" transform_reverse: Option<String> = Opt(STRING),
    }
}

object! {
    /// A parameter read and written by SysEx.
    ParamEntry {
        "osc" osc: String = ADDRESS,
        "pr" pr: Option<u8> = Opt(BYTE),
        "p" p: Option<u8> = Opt(BYTE),
        "c" c: Option<u8> = Opt(BYTE),
        "r" r: Option<u8> = Opt(BYTE),
        "id" id: Option<i64> = Opt(INTEGER),
        "range" range: Option<[i64; 2]> = Opt(Pair),
        "section" section: Option<String> = Opt(STRING),
        "expects" expects: Option<String> = Opt(STRING),
        /// Named values: each label with its value.
        "values" values: Option<BTreeMap<String, i64>> = Opt(Dict(INTEGER)),
        "transform" transform: Option<String> = Opt(STRING),
        "transform_reverse" transform_reverse: Option<String> = Opt(STRING),
    }
}

words! {
    /// The type of a command's argument.
    ArgType {
        U7 = "u7", U14 = "u14", S32 = "s32", Int = "int", Float = "float", Bool = "bool",
        String = "string", Enum = "enum",
    }
}

object! {
    /// An argument of a command.
    Arg {
        "name" name: String = STRING,
        "type" ty: ArgType = closed(),
        "range" range: Option<[i64; 2]> = Opt(Pair),
        "values" values: Option<BTreeMap<String, i64>> = Opt(Dict(INTEGER)),
        /// Its value when none is given, of any type.
        "default" default: Option<Value> = Opt(Any),
    }
}

/// An argument of an OSC message: a number (kept as written), a boolean or
/// a string; in 0.5 also an argument of the command or a field captured in
/// the message recognised.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Number(Number),
    Bool(bool),
    /// In 0.4 a string, `{name}` taking an argument; in 0.5 a literal
    /// without brace.
    String(String),
    /// 0.5: `{"arg": "name"}`, the value of an argument of the command.
    Arg(String),
    /// 0.5: `{"capture": "name"}`, a field captured in the message.
    Capture(String),
}

pub(crate) struct ScalarCodec;

impl Codec for ScalarCodec {
    type T = Scalar;
    fn read(&self, v: &Value, at: &At) -> Result<Scalar, Fault> {
        match v {
            Value::Number(n) => Ok(Scalar::Number(n.clone())),
            Value::Bool(b) => Ok(Scalar::Bool(*b)),
            Value::String(s) => Ok(Scalar::String(s.clone())),
            Value::Object(o) => match (named(o, "arg"), named(o, "capture")) {
                (Some(n), _) => Ok(Scalar::Arg(n)),
                (_, Some(n)) => Ok(Scalar::Capture(n)),
                _ => Err(at.fault(FaultKind::BadValue, "{\"arg\": name} or {\"capture\": name}, the name from letters, digits and _")),
            },
            _ => Err(at.fault(FaultKind::BadValue, "a number, true or false, a string, or an object of arg or capture")),
        }
    }
    fn write(&self, t: &Scalar) -> Option<Value> {
        let one = |k: &str, n: &String| Value::Object(serde_json::Map::from_iter([(k.to_string(), Value::String(n.clone()))]));
        Some(match t {
            Scalar::Number(n) => Value::Number(n.clone()),
            Scalar::Bool(b) => Value::Bool(*b),
            Scalar::String(s) => Value::String(s.clone()),
            Scalar::Arg(n) => one("arg", n),
            Scalar::Capture(n) => one("capture", n),
        })
    }
}

/// Whether the arguments, read in version `v05` or not, are of that
/// version: in 0.5 a string holds no brace, in 0.4 there are no objects.
fn check_scalars(args: &[Scalar], pointer: &str, v05: bool) -> Result<(), Fault> {
    for (i, a) in args.iter().enumerate() {
        let rule = match (a, v05) {
            (Scalar::String(s), true) if s.contains(['{', '}']) => {
                "a literal without brace (an argument of the command is {\"arg\": name})"
            }
            (Scalar::Arg(_) | Scalar::Capture(_), false) => "a number, true or false, or a string (an object is 0.5)",
            _ => continue,
        };
        return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("{pointer}/{i}"), rule: rule.into() });
    }
    Ok(())
}

/// The argument a 0.4 string stands for: `{name}` is `arg`, a string without
/// brace is a literal, any other brace cannot be converted.
fn convert_arg(a: &Scalar, at: &At) -> Result<Scalar, Fault> {
    let Scalar::String(s) = a else { return Ok(a.clone()) };
    if !s.contains(['{', '}']) {
        return Ok(a.clone());
    }
    match s.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        Some(n) if name_ok(n) => Ok(Scalar::Arg(n.to_string())),
        _ => Err(at.fault(FaultKind::NotConvertible, format!("the argument {s:?} is neither a literal nor one {{name}}"))),
    }
}

object! {
    /// The OSC message sent to a software; in 0.4 `{name}` takes an
    /// argument, in 0.5 `{"arg": "name"}`.
    Forward {
        "path" path: Address = AddressCodec(AddressPlace::Emitted),
        "args" args: Option<Vec<Scalar>> = Opt(list(ScalarCodec)),
    }
}

impl Forward {
    fn check(&self, pointer: &str, v05: bool) -> Result<(), Fault> {
        check_address(&self.path, AddressPlace::Emitted, &format!("{pointer}/path"), v05)?;
        check_scalars(self.args.as_deref().unwrap_or_default(), &format!("{pointer}/args"), v05)
    }

    fn into_structures(mut self, at: &At) -> Result<Forward, Fault> {
        self.path = convert_address(&self.path, AddressPlace::Emitted, &At::Key(at, "path"))?;
        let args = At::Key(at, "args");
        for (i, a) in self.args.iter_mut().flatten().enumerate() {
            *a = convert_arg(a, &At::Index(&args, i))?;
        }
        Ok(self)
    }
}

object! {
    /// A value computed from the arguments before the message is built.
    Pre {
        "bind" bind: String = STRING,
        "expr" expr: Expr = EXPR,
    }
}

object! {
    /// A named message: a SysEx utility, or an OSC message of a software.
    Command {
        "osc" osc: Address = AddressCodec(AddressPlace::Command),
        "section" section: Option<String> = Opt(STRING),
        "notes" notes: Option<String> = Opt(STRING),
        "args" args: Option<Vec<Arg>> = Opt(list(obj())),
        "pre" pre: Option<Vec<Pre>> = Opt(list(obj())),
        "frame" frame: Option<Vec<FrameToken>> = Opt(list(FRAME_TOKEN)),
        "forward" forward: Option<Forward> = Opt(obj()),
        /// osc-bridge extension: a Lua script that changes the message
        /// before it is sent.
        "script" script: Option<String> = Opt(STRING),
    }
}

words! {
    /// How a trigger's channel is chosen.
    ChannelMode { Fixed = "fixed", Basic = "basic", Omni = "omni", Auto = "auto", }
}

object! {
    /// A drum or sample sound fired by a note.
    Trigger {
        "osc" osc: String = ADDRESS,
        "sound" sound: String = STRING,
        "section" section: Option<String> = Opt(STRING),
        "channel" channel: Option<u8> = Opt(CHANNEL),
        "channel_mode" channel_mode: Option<ChannelMode> = Opt(closed()),
        "note_range" note_range: [i64; 2] = Pair,
        "note" note: Option<u8> = Opt(U7),
        "velocity_usage" velocity_usage: Option<String> = Opt(STRING),
        "comments" comments: Option<String> = Opt(STRING),
    }
}

object! {
    /// An argument a received OSC message is matched on.
    MatchArg {
        "name" name: String = STRING,
        "type" ty: String = STRING,
    }
}

object! {
    /// A message the device answers or sends by itself, recognised.
    Reply {
        "match_frame" match_frame: Option<Vec<FrameToken>> = Opt(list(MATCH_TOKEN)),
        "match_osc" match_osc: Option<Address> = Opt(AddressCodec(AddressPlace::Recognised)),
        "match_args" match_args: Option<Vec<MatchArg>> = Opt(list(obj())),
        /// The address it is reported at; in 0.4 `{placeholders}` from the
        /// message and the arguments after the first space, in 0.5 segments
        /// with captures and the arguments in `emit_args`.
        "emit_osc" emit_osc: Address = AddressCodec(AddressPlace::Reported),
        /// 0.5: the OSC arguments it is reported with; absent, none.
        "emit_args" emit_args: Option<Vec<Scalar>> = Opt(list(ScalarCodec)),
        /// osc-bridge extension: a Lua script that reads the message.
        "script" script: Option<String> = Opt(STRING),
    }
}

/// Where a piece of data comes from: its kind of source, its credit and
/// licence, and the firmware it documents. A source may carry keys of its
/// own beside these.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    /// What kind of source: `vendor-doc`, `hardware-verified`,
    /// `community-re`, `knobkraft`…
    pub kind: String,
    pub url: Option<String>,
    /// The firmware the source documents.
    pub firmware: Option<String>,
    pub author: Option<String>,
    pub attribution: Option<String>,
    /// An SPDX id when it has one.
    pub license: Option<String>,
    pub commit: Option<String>,
    pub imported_at: Option<String>,
    pub notes: Option<String>,
    /// Its other keys, kept as they are.
    pub extra: BTreeMap<String, Value>,
}

impl Object for Source {
    const KEYS: &'static [&'static str] =
        &["type", "url", "firmware", "author", "attribution", "license", "commit", "imported_at", "notes"];

    fn read(v: &Value, at: &At) -> Result<Self, Fault> {
        let o = v.as_object().ok_or_else(|| at.fault(FaultKind::BadValue, "an object"))?;
        let text = |key| crate::json::field(o, key, at, Opt(STRING));
        Ok(Source {
            kind: crate::json::field(o, "type", at, STRING)?,
            url: text("url")?,
            firmware: text("firmware")?,
            author: text("author")?,
            attribution: text("attribution")?,
            license: text("license")?,
            commit: text("commit")?,
            imported_at: text("imported_at")?,
            notes: text("notes")?,
            extra: o.iter().filter(|(k, _)| !Self::KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect(),
        })
    }

    fn write(&self) -> Value {
        let mut o = serde_json::Map::new();
        o.insert("type".into(), Value::String(self.kind.clone()));
        let texts = [
            ("url", &self.url),
            ("firmware", &self.firmware),
            ("author", &self.author),
            ("attribution", &self.attribution),
            ("license", &self.license),
            ("commit", &self.commit),
            ("imported_at", &self.imported_at),
            ("notes", &self.notes),
        ];
        for (k, v) in texts {
            crate::json::put(&mut o, k, Opt(STRING), v);
        }
        o.extend(self.extra.iter().map(|(k, v)| (k.clone(), v.clone())));
        Value::Object(o)
    }
}

object! {
    /// A USB identity.
    UsbId {
        "vendor_id" vendor_id: u16 = int(0, 65535),
        "product_id" product_id: u16 = int(0, 65535),
    }
}

words! {
    /// How a device ends a note.
    NoteOff { NoteOff = "note_off", NoteOnVelocity0 = "note_on_velocity_0", }
}

object! {
    /// What the device sends by itself, unasked: habits that tell models apart.
    Sends {
        /// Sends Active Sensing (FE) about every 300 ms.
        "active_sensing" active_sensing: Option<bool> = Opt(Bool),
        /// Sends MIDI clock (F8) even when stopped.
        "clock" clock: Option<bool> = Opt(Bool),
        "note_off" note_off: Option<NoteOff> = Opt(closed()),
    }
}
