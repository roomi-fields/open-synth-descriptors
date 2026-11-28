//! The device descriptor of OSD 0.4 (`schemas/0.4/device.schema.json`) and
//! the maker profile (`schemas/0.4/profile.schema.json`), whose blocks are
//! the device's.

use crate::json::{
    at_least, closed, int, list, list_of, obj, object, words, Any, At, Bool, Codec, Dict, Nullable, Object, Opt, Pair,
    Word, ADDRESS, INTEGER, STRING,
};
use crate::json::{matching, Text};

/// A title: a string that is not empty (0.7).
const TITLE: Text = matching(|s| !s.is_empty(), "that is not empty");
use crate::address::{
    check_address, convert_address, name_ok, named, split_template, Address, AddressCodec, AddressPlace,
};
use crate::frame::{
    check_expr, check_frame, convert_expr, convert_token, Expr, FrameToken, Place, EXPR, FRAME_TOKEN, MATCH_TOKEN,
    REPLY_TOKEN,
};
use crate::providers::is_provider;
use crate::role::{check_roles, Role};
use crate::resource::Resource;
use crate::sound_transfer::SoundTransfer;
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
        /// The state of the vocabulary of roles every role of the descriptor is
        /// written with (0.6).
        "role_vocabulary" role_vocabulary: Option<String> = Opt(STRING),
        /// The driver whose MIDI this firmware answers to unchanged
        /// (`maker/file`): this file then only says what differs.
        "midi_of" midi_of: Option<String> = Opt(STRING),
        /// The dimensions the device holds its controls in, each with its values: a mode, a part (0.7,
        /// proposal 0003).
        "contexts" contexts: Option<BTreeMap<String, ContextDimension>> = Opt(Dict(obj())),
        /// Devices (their address prefix without `/`) whose sounds this one
        /// loads as they are.
        "loads_sounds_of" loads_sounds_of: Option<Vec<String>> = Opt(list(STRING)),
        /// What in a message is the sound, per form of transfer (0.6).
        "sound_transfers" sound_transfers: Option<Vec<SoundTransfer>> = Opt(list(obj())),
        /// What the device can receive from another (0.6).
        "resources" resources: Option<Vec<Resource>> = Opt(list(obj())),
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
        /// firmware it documents. Written `_sources` before 0.7.
        "sources" sources: Option<Vec<Source>> = Opt(list(obj())),
        /// The credit of the MIDI a resolution took from the targets of `midi_of` (0.7).
        "midi_of_sources" midi_of_sources: Option<Vec<Source>> = Opt(list(obj())),
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
        /// Where the data comes from, with its credit, licence and the firmware it documents (0.7).
        "sources" sources: Option<Vec<Source>> = Opt(list(obj())),
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
    /// The three shape keys are required, except in a 0.6 file with `midi_of`,
    /// which takes them from its target (`check_params_shape`).
    Params {
        "get_frame" get_frame: Option<Vec<FrameToken>> = Opt(list(FRAME_TOKEN)),
        "set_frame" set_frame: Option<Vec<FrameToken>> = Opt(list(FRAME_TOKEN)),
        "value_type" value_type: Option<ValueType> = Opt(closed()),
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

impl MidiType {
    /// The OSC address the message is reported at, under the device's prefix
    /// (not included), with its OSC arguments after the space in this order:
    /// fixed by the specification.
    pub fn template(self) -> &'static str {
        match self {
            MidiType::NoteOn => "/note/on {note} {velocity} {channel}",
            MidiType::NoteOff => "/note/off {note} {velocity} {channel}",
            MidiType::Cc => "/cc/{num} {value} {channel}",
            MidiType::Pitchbend => "/pitchbend {value_u14} {channel}",
            MidiType::Aftertouch => "/aftertouch {value} {channel}",
            MidiType::PolyAftertouch => "/poly_aftertouch {note} {value} {channel}",
            MidiType::ProgramChange => "/program_change {program} {channel}",
        }
    }
}

/// What the device sends while played that is reported (an osc-bridge
/// extension).
#[derive(Clone, Debug, PartialEq)]
pub enum MidiIn {
    /// 0.4: the address each message is reported at.
    Templates(MidiTemplates),
    /// 0.5: the types of the messages reported, whose addresses the
    /// specification fixes ([`MidiType::template`]).
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
    if let Some(Some(r)) = &i.request {
        out.push((format!("{at}/request"), Place::Sent, r));
    }
    if let Some(Some(r)) = &i.reply {
        out.push((format!("{at}/reply"), Place::Reply, r));
    }
}

fn programs_frames<'a>(p: &'a Programs, at: &str, out: &mut Vec<(String, Place, &'a [FrameToken])>) {
    for (n, f) in p.before.iter().flatten().flatten().enumerate() {
        out.push((format!("{at}/before/{n}"), Place::Sent, f));
    }
    for (n, f) in p.after.iter().flatten().flatten().enumerate() {
        out.push((format!("{at}/after/{n}"), Place::Sent, f));
    }
    if let Some(r) = p.dump.iter().flatten().find_map(|d| d.request.as_ref()) {
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
        ("/programs/dump/script", p.is_some_and(|p| p.dump.iter().flatten().any(|d| d.script.is_some()))),
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

/// A direct key of `identity` or `programs` set to `null`: before 0.6 only `identity.firmware`
/// admits it, as the schema of the file's own version says (23.25 of `spec/0.6.md`).
fn check_null_keys(schema: Option<Version>, i: Option<&Identity>, p: Option<&Programs>) -> Result<(), Fault> {
    if schema >= Some(Version::V0_6) {
        return Ok(());
    }
    let nulls: [(&str, bool); 23] = [
        ("/identity/request", i.is_some_and(|i| matches!(i.request, Some(None)))),
        ("/identity/to", i.is_some_and(|i| matches!(i.to, Some(None)))),
        ("/identity/script", i.is_some_and(|i| matches!(i.script, Some(None)))),
        ("/identity/unpack", i.is_some_and(|i| matches!(i.unpack, Some(None)))),
        ("/identity/reply", i.is_some_and(|i| matches!(i.reply, Some(None)))),
        ("/identity/reply_text", i.is_some_and(|i| matches!(i.reply_text, Some(None)))),
        ("/identity/family", i.is_some_and(|i| matches!(i.family, Some(None)))),
        ("/identity/member", i.is_some_and(|i| matches!(i.member, Some(None)))),
        ("/identity/device_id", i.is_some_and(|i| matches!(i.device_id, Some(None)))),
        ("/identity/ports", i.is_some_and(|i| matches!(i.ports, Some(None)))),
        ("/identity/usb", i.is_some_and(|i| matches!(i.usb, Some(None)))),
        ("/identity/sends", i.is_some_and(|i| matches!(i.sends, Some(None)))),
        ("/programs/selects", p.is_some_and(|p| matches!(p.selects, Some(None)))),
        ("/programs/program_change", p.is_some_and(|p| matches!(p.program_change, Some(None)))),
        ("/programs/requires", p.is_some_and(|p| matches!(p.requires, Some(None)))),
        ("/programs/banks", p.is_some_and(|p| matches!(p.banks, Some(None)))),
        ("/programs/before", p.is_some_and(|p| matches!(p.before, Some(None)))),
        ("/programs/after", p.is_some_and(|p| matches!(p.after, Some(None)))),
        ("/programs/recall_script", p.is_some_and(|p| matches!(p.recall_script, Some(None)))),
        ("/programs/delay_ms", p.is_some_and(|p| matches!(p.delay_ms, Some(None)))),
        ("/programs/dump", p.is_some_and(|p| matches!(p.dump, Some(None)))),
        ("/programs/notes", p.is_some_and(|p| matches!(p.notes, Some(None)))),
        ("", false),
    ];
    match nulls.iter().find(|(_, here)| *here) {
        Some((pointer, _)) => Err(Fault {
            kind: FaultKind::BadValue,
            file: None,
            pointer: (*pointer).into(),
            rule: format!("{} does not take null before 0.6", pointer.rsplit('/').next().unwrap_or_default()),
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
            i.script.iter().flatten().next().is_some() || i.firmware.iter().flatten().any(|f| f.script.is_some() || f.decode == "lua")
        });
        let programs = d.programs.as_ref().is_some_and(|p| p.recall_script.iter().flatten().next().is_some() || p.dump.iter().flatten().any(|x| x.script.is_some()));
        let sounds = d.sound_transfers_use_lua();
        if cc || params || commands || replies || custom || identity || programs || sounds { vec![Capability::Lua] } else { Vec::new() }
    }
}

/// A declared `capabilities`, read as a set, equals the derived list. It
/// plays on the text of one file as written, never on a merged document.
pub(crate) fn check_capabilities(d: &Device) -> Result<(), Fault> {
    let derived = d.derived_capabilities();
    match &d.capabilities {
        Some(declared) if !(declared.iter().all(|c| derived.contains(c)) && derived.iter().all(|c| declared.contains(c))) => Err(Fault {
            kind: FaultKind::BadValue,
            file: None,
            pointer: "/capabilities".into(),
            rule: format!(
                "the list the fields present require is [{}]",
                d.derived_capabilities().iter().map(|c| c.word()).collect::<Vec<_>>().join(", ")
            ),
        }),
        None if crate::is_05_form(d.schema) && !d.derived_capabilities().is_empty() => Err(Fault {
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
    if let Some(Some(s)) = &p.selects {
        vocabulary::check_selects(s, "/programs/selects")?;
    }
    for (n, b) in p.banks.iter().flatten().flatten().enumerate() {
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
    check_source_words(d.schema, &credit_of(d))
}

/// A list of sources of a descriptor, under its key.
type Credit<'a> = (&'static str, &'a Option<Vec<Source>>);

/// The lists of sources of a device, in the order their `id` and warnings are read.
fn credit_of(d: &Device) -> [Credit<'_>; 2] {
    [("sources", &d.sources), ("midi_of_sources", &d.midi_of_sources)]
}

/// The sources of a file that is no device, each list under the pointer of the object that holds
/// it (empty for the file's own, `/made_on` for the assembly of a sound): their words, then their
/// `id`, written once in the whole file; the fault is at the pointer in that file.
pub(crate) fn check_credit(schema: Option<Version>, lists: &[(String, &Option<Vec<Source>>)]) -> Result<(), Fault> {
    let mut seen = std::collections::HashSet::new();
    for (holder, list) in lists {
        let credit = [("sources", *list)];
        check_source_words(schema, &credit)
            .and_then(|()| check_source_ids(&credit, &mut seen))
            .map_err(|mut f| {
                f.pointer.insert_str(0, holder);
                f
            })?;
    }
    Ok(())
}

/// The words of each source of `lists`: its `type` and `license`, and from 0.7 its `provider` and
/// its `profile`.
fn check_source_words(schema: Option<Version>, lists: &[Credit]) -> Result<(), Fault> {
    let from_0_7 = schema >= Some(Version::V0_7);
    for (key, list) in lists {
        for (n, s) in list.iter().flatten().enumerate() {
            let at = format!("/{key}/{n}");
            vocabulary::check_source(&s.kind, s.license.as_deref(), from_0_7, &at)?;
            if from_0_7 {
                check_provider(s, &at)?;
                check_profile_of_source(s, &at)?;
            }
        }
    }
    Ok(())
}

/// The `provider` of a source of 0.7 is a text of the form of an identifier of the table of
/// providers; whether the table holds it is no fault ([`crate::warnings`]).
fn check_provider(s: &Source, at: &str) -> Result<(), Fault> {
    if s.extra.contains_key("provider") || s.provider.as_deref().is_some_and(|p| !is_provider(p)) {
        let rule = "a provider is lowercase letters and digits in groups joined by single hyphens, at most 64 characters";
        return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("{at}/provider"), rule: rule.into() });
    }
    Ok(())
}

/// The `profile` of a source of 0.7, the maker profile it comes from, is a text; that the profile
/// exists is never checked.
fn check_profile_of_source(s: &Source, at: &str) -> Result<(), Fault> {
    if s.extra.get(PROFILE_OF_SOURCE).is_some_and(|p| !p.is_string()) {
        let rule = "the profile a source comes from is a text";
        return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("{at}/{PROFILE_OF_SOURCE}"), rule: rule.into() });
    }
    Ok(())
}

/// `banks[].writable` and `dump.reply_timeout_ms` are not fields before 0.6; from 0.6 a reply time
/// is never shorter than the pause `delay_ms` (a missing pause is zero).
fn check_banks_and_reply_time(d: &Device) -> Result<(), Fault> {
    let Some(p) = d.programs.as_ref() else { return Ok(()) };
    let v06 = d.schema >= Some(Version::V0_6);
    let unknown = |pointer: String, name: &str| Fault { kind: FaultKind::UnknownField, file: None, pointer, rule: format!("{name} is not a field before 0.6") };
    if !v06 {
        for (n, b) in p.banks.iter().flatten().flatten().enumerate() {
            if b.writable.is_some() {
                return Err(unknown(format!("/programs/banks/{n}/writable"), "writable"));
            }
        }
        if p.dump.iter().flatten().any(|k| k.reply_timeout_ms.is_some()) {
            return Err(unknown("/programs/dump/reply_timeout_ms".into(), "reply_timeout_ms"));
        }
        return Ok(());
    }
    if let Some(timeout) = p.dump.iter().flatten().find_map(|k| k.reply_timeout_ms) {
        let pause = p.delay_ms.flatten().unwrap_or(0);
        if timeout < pause {
            return Err(Fault {
                kind: FaultKind::BadValue,
                file: None,
                pointer: "/programs/dump/reply_timeout_ms".into(),
                rule: format!("the reply time {timeout} ms is shorter than the pause delay_ms {pause} ms"),
            });
        }
    }
    Ok(())
}

/// `range`, `transmit`, `recognize` and `nrpn_lsb_range` are fields of an entry without a way only
/// (`spec/0.7.md`): beside a way they are unknown fields.
fn check_fields_of_a_partial_control(d: &Device) -> Result<(), Fault> {
    let cc = d.cc_params.iter().flat_map(|t| &t.entries).enumerate().map(|(n, e)| {
        let given = [("range", e.range.is_some()), ("transmit", e.transmit.is_some()), ("recognize", e.recognize.is_some()), ("nrpn_lsb_range", e.nrpn_lsb_range.is_some())];
        ("cc_params", n, e.ways.is_empty(), given)
    });
    let params = d.params.iter().flat_map(|t| &t.entries).enumerate().map(|(n, e)| {
        let given = [("range", e.range.is_some()), ("transmit", e.transmit.is_some()), ("recognize", e.recognize.is_some()), ("nrpn_lsb_range", false)];
        ("params", n, e.ways.is_empty(), given)
    });
    for (table, n, partial, given) in cc.chain(params) {
        if let Some((key, _)) = given.iter().find(|(_, here)| !partial && *here) {
            let rule = format!("{key} is a field of an entry without a way: with ways, each way says it");
            return Err(Fault { kind: FaultKind::UnknownField, file: None, pointer: format!("/{table}/entries/{n}/{key}"), rule });
        }
    }
    Ok(())
}

/// The controls of a device of 0.7: the `default` and the named values lie in the
/// reference scale, and a message is claimed once by side (`spec/0.7.md`).
fn check_controls_0_7(d: &Device) -> Result<(), Fault> {
    check_fields_of_a_partial_control(d)?;
    let cc = d.cc_params.iter().flat_map(|t| &t.entries).enumerate().map(|(n, e)| (format!("/cc_params/entries/{n}"), &e.ways, e.known_scale(), e.default, e.values.as_ref(), e.readings.as_deref(), false));
    let params = d.params.iter().flat_map(|t| &t.entries).enumerate().map(|(n, e)| (format!("/params/entries/{n}"), &e.ways, e.known_scale(), e.default, e.values.as_ref(), e.readings.as_deref(), true));
    for (at, ways, scale, default, values, readings, sysex) in cc.chain(params) {
        for (m, w) in ways.iter().enumerate() {
            check_contexts_of_way(d, w, &format!("{at}/ways/{m}"), sysex)?;
        }
        let Some([lo, hi]) = scale else { continue };
        if let Some(v) = default.filter(|v| *v < lo || *v > hi) {
            return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("{at}/default"), rule: format!("the default {v} is outside the scale {lo}..{hi}") });
        }
        for (i, r) in readings.iter().flat_map(|l| l.iter()).enumerate() {
            let Some([a, b]) = r.range else { continue };
            if a > b || a < lo || b > hi {
                let rule = format!("the range {a}..{b} is not inside the scale {lo}..{hi} (low above high or outside)");
                return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("{at}/readings/{i}/range"), rule });
            }
        }
        for (i, item) in values.iter().flat_map(|v| v.iter()).enumerate() {
            if item.value < lo || item.value > hi {
                let rule = format!("the value {} is outside the scale {lo}..{hi}", item.value);
                return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("{at}/values/{i}/value"), rule });
            }
        }
    }
    check_claims(d)
}

/// `in` and `slice` are for a way of `cc_params` (0003 does not cover `params`); each dimension and
/// value of `in` is one the device declares. A file with `midi_of` and no `contexts` read alone does
/// not hold the dimensions of its target: the names are left to the reading that joins them.
fn check_contexts_of_way(d: &Device, w: &Way, at: &str, sysex: bool) -> Result<(), Fault> {
    let unknown = |key: &str| Fault { kind: FaultKind::UnknownField, file: None, pointer: format!("{at}/{key}"), rule: format!("{key} is a field of a way of cc_params only") };
    if sysex {
        return match (&w.held_in, &w.slice) {
            (Some(_), _) => Err(unknown("in")),
            (None, Some(_)) => Err(unknown("slice")),
            _ => Ok(()),
        };
    }
    if d.midi_of.is_some() && d.contexts.is_none() {
        return Ok(());
    }
    for (dimension, ids) in w.held_in.iter().flatten() {
        let bad = |pointer: String, rule: String| Fault { kind: FaultKind::BadValue, file: None, pointer, rule };
        let Some(dim) = d.contexts.as_ref().and_then(|c| c.get(dimension)) else {
            return Err(bad(format!("{at}/in/{dimension}"), format!("{dimension} is not a dimension of the contexts of the device")));
        };
        for (i, id) in ids.ids().iter().enumerate() {
            if !dim.values.iter().any(|v| v.id.as_deref() == Some(id.as_str())) {
                let pointer = if matches!(ids, ContextIds::One(_)) { format!("{at}/in/{dimension}") } else { format!("{at}/in/{dimension}/{i}") };
                return Err(bad(pointer, format!("{id} is not a value of the dimension {dimension}")));
            }
        }
    }
    Ok(())
}

/// What a claim holds in: per dimension the device knows, the ids of the way, and the interval of its
/// `slice`, the coordinate `value`.
struct Coordinates {
    dimensions: Vec<(String, Vec<String>)>,
    value: Option<[i64; 2]>,
}

impl Coordinates {
    fn of(d: &Device, w: &Way) -> Coordinates {
        let known = |dimension: &str| d.contexts.as_ref().is_some_and(|c| c.contains_key(dimension));
        let dimensions = w.held_in.iter().flatten().filter(|(k, _)| known(k)).map(|(k, ids)| (k.clone(), ids.ids().to_vec())).collect();
        Coordinates { dimensions, value: w.slice }
    }

    /// Two claims hold together unless a coordinate both name has two sets with nothing in common.
    fn hold_together(&self, other: &Coordinates) -> bool {
        let apart = self.dimensions.iter().any(|(k, ids)| other.dimensions.iter().any(|(o, theirs)| o == k && !ids.iter().any(|i| theirs.contains(i))));
        let apart_by_value = matches!((self.value, other.value), (Some(a), Some(b)) if a[1] < b[0] || b[1] < a[0]);
        !(apart || apart_by_value)
    }
}

/// A way that took a claim: the place of its control and of itself, and its coordinates.
type Taken = (usize, usize, Coordinates);

/// What a way claims on one side: its kind, its channel and one number it occupies.
type Claim = (u8, Option<u8>, u16);

/// The claims of a way (`spec/0.7.md`): a `cc` claims each of its numbers, an NRPN or an RPN its
/// two as one, a note or a polyphonic pressure its note, a channel pressure and a pitch bend their
/// channel; a way that works neither side claims nothing.
fn claims(w: &Way, channel: Option<u8>) -> ([bool; 2], Vec<Claim>) {
    let sides = [w.transmit != Some(Direction::None), w.recognize != Some(Direction::None)];
    let index = w.ctrl_index.as_deref().unwrap_or_default();
    let (kind, numbers): (u8, Vec<u16>) = match w.ctrl_type.as_str() {
        "cc" => (0, index.iter().map(|n| *n as u16).collect()),
        "nrpn" => (1, vec![((index[0] as u16) << 7) | index[1] as u16]),
        "rpn" => (2, vec![((index[0] as u16) << 7) | index[1] as u16]),
        "chPress" => (3, vec![0]),
        "pPress" => (4, vec![index[0] as u16]),
        "pBend" => (5, vec![0]),
        "note" => (6, vec![index[0] as u16]),
        _ => return ([false, false], Vec::new()),
    };
    (sides, numbers.into_iter().map(|n| (kind, channel, n)).collect())
}

/// A message is claimed once on each side by claims that can hold together, across the ways of a
/// control as across the controls: the fault is at the later way and its rule names the earlier. The
/// reader keeps, by side and by claimed number, the places and coordinates of the ways that took it.
fn check_claims(d: &Device) -> Result<(), Fault> {
    let Some(c) = &d.cc_params else { return Ok(()) };
    let mut claimed: [std::collections::HashMap<Claim, Vec<Taken>>; 2] = Default::default();
    for (n, e) in c.entries.iter().enumerate() {
        let channel = e.channel.or(c.channel);
        for (m, w) in e.ways.iter().enumerate() {
            let (sides, numbers) = claims(w, channel);
            let coordinates = Coordinates::of(d, w);
            for (side, taken) in claimed.iter_mut().enumerate().filter(|(s, _)| sides[*s]) {
                for claim in &numbers {
                    let earlier = taken.get(claim).and_then(|l| l.iter().find(|(_, _, c)| c.hold_together(&coordinates)));
                    if let Some((en, wm, _)) = earlier {
                        let side = if side == 0 { "sends" } else { "receives" };
                        let rule = format!("the message is already claimed, as the one the control {side}, at /cc_params/entries/{en}/ways/{wm}: one control is reached by it");
                        return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("/cc_params/entries/{n}/ways/{m}"), rule });
                    }
                    taken.entry(*claim).or_default().push((n, m, Coordinates::of(d, w)));
                }
            }
        }
    }
    Ok(())
}

/// The three keys that give `params` its message shape, each present
/// (23.6, 23.10 of the section Resolution of `spec/0.6.md`): `Missing` at the first absent one.
pub(crate) fn check_params_shape(p: &Params) -> Result<(), Fault> {
    let absent = [("get_frame", p.get_frame.is_none()), ("set_frame", p.set_frame.is_none()), ("value_type", p.value_type.is_none())];
    match absent.iter().find(|(_, a)| *a) {
        Some((key, _)) => Err(Fault { kind: FaultKind::Missing, file: None, pointer: format!("/params/{key}"), rule: format!("{key} is required") }),
        None => Ok(()),
    }
}

/// Whether the frames and expressions of a descriptor are the forms of its
/// version: strings in 0.4 (no `$schema`), structures in 0.5.
fn check_device(d: &Device, _: &At) -> Result<(), Fault> {
    let v05 = crate::is_05_form(d.schema);
    if let Some(p) = &d.params {
        if !(d.schema >= Some(Version::V0_6) && d.midi_of.is_some()) {
            check_params_shape(p)?;
        }
    }
    check_05_fields(d.identity.as_ref(), d.programs.as_ref(), v05)?;
    check_null_keys(d.schema, d.identity.as_ref(), d.programs.as_ref())?;
    crate::sound_transfer::check_version(d)?;
    crate::resource::check_version(d)?;
    check_roles(d)?;
    check_stands_for(d)?;
    if d.schema >= Some(Version::V0_7) {
        check_controls_0_7(d)?;
        check_source_ids(&credit_of(d), &mut Default::default())?;
        if d.midi_of.is_none() {
            check_reading_sources(d)?;
        }
    }
    check_banks_and_reply_time(d)?;
    if !v05 && d.capabilities.is_some() {
        return Err(Fault { kind: FaultKind::UnknownField, file: None, pointer: "/capabilities".into(), rule: "capabilities is not a field of 0.4".into() });
    }
    if v05 {
        check_vocabularies(d)?;
    }
    let mut frames = Vec::new();
    if let Some(i) = &d.identity {
        identity_frames(i, "/identity", &mut frames);
    }
    if let Some(p) = &d.programs {
        programs_frames(p, "/programs", &mut frames);
    }
    if let Some(p) = &d.params {
        if let Some(f) = &p.get_frame {
            frames.push(("/params/get_frame".into(), Place::Sent, f));
        }
        if let Some(f) = &p.set_frame {
            frames.push(("/params/set_frame".into(), Place::Sent, f));
        }
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
    let v05 = crate::is_05_form(p.schema);
    check_05_fields(p.identity.as_ref(), p.programs.as_ref(), v05)?;
    check_null_keys(p.schema, p.identity.as_ref(), p.programs.as_ref())?;
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
    // `sources` is a field of a profile from 0.7 alone ([`read_profile`]).
    check_credit(p.schema, &[(String::new(), &p.sources)])
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
    if let Some(Some(r)) = &mut i.request {
        convert_frame(r, Place::Sent, &At::Key(at, "request"))?;
    }
    if let Some(Some(r)) = &mut i.reply {
        convert_frame(r, Place::Reply, &At::Key(at, "reply"))?;
    }
    Ok(())
}

fn convert_programs(p: &mut Programs, at: &At) -> Result<(), Fault> {
    if let Some(Some(s)) = &mut p.selects {
        *s = vocabulary::convert_selects(s, &At::Key(at, "selects").pointer())?;
    }
    let banks = At::Key(at, "banks");
    for (n, b) in p.banks.iter_mut().flatten().flatten().enumerate() {
        if let Some(s) = &mut b.selects {
            *s = vocabulary::convert_selects(s, &At::Key(&At::Index(&banks, n), "selects").pointer())?;
        }
    }
    let before = At::Key(at, "before");
    for (n, f) in p.before.iter_mut().flatten().flatten().enumerate() {
        convert_frame(f, Place::Sent, &At::Index(&before, n))?;
    }
    let after = At::Key(at, "after");
    for (n, f) in p.after.iter_mut().flatten().flatten().enumerate() {
        convert_frame(f, Place::Sent, &At::Index(&after, n))?;
    }
    if let Some(r) = p.dump.iter_mut().flatten().find_map(|d| d.request.as_mut()) {
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
                s.license = Some(vocabulary::convert_license(l, &At::Key(&at, "license").pointer())?);
            }
        }
        if let Some(p) = &mut self.params {
            let at = At::Key(&root, "params");
            if let Some(f) = &mut p.get_frame {
                convert_frame(f, Place::Sent, &At::Key(&at, "get_frame"))?;
            }
            if let Some(f) = &mut p.set_frame {
                convert_frame(f, Place::Sent, &At::Key(&at, "set_frame"))?;
            }
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
            let types = t.types(&At::Key(&root, "midi_in"))?;
            // 0.5 `midi_in` is a list, which holds no note: each note moves to
            // the device under `_midi_in` + its key, the first free one.
            for (key, note) in &t.remarks {
                let base = format!("_midi_in{key}");
                let mut dest = base.clone();
                let mut n = 2;
                while self.remarks.contains_key(&dest) {
                    dest = format!("{base}_{n}");
                    n += 1;
                }
                self.remarks.insert(dest, note.clone());
            }
            self.midi_in = Some(MidiIn::Types(types));
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
            let canonical = t.template();
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
        "device_id" device_id: Option<Vec<u8>> = Opt(list(U7)),
        /// Most messages a second the device takes; 0: no limit known.
        "rate_limit_hz" rate_limit_hz: Option<i64> = Opt(at_least(0)),
        "kind" kind: Option<DeviceKind> = Opt(closed()),
        /// The catalogue address `maker/device` of the device a virtual instrument stands for (0.6).
        "stands_for" stands_for: Option<String> = Opt(STRING),
        /// How true a virtual instrument is to the device it stands for (0.6).
        "fidelity" fidelity: Option<Fidelity> = Opt(closed()),
        "transport" transport: Option<Transport> = Opt(obj()),
    }
}

words! {
    /// How true a virtual instrument is to the device it stands for: `faithful` (the real firmware,
    /// emulated), `modelled` (the behaviour reproduced from study, without its firmware),
    /// `principle` (a design that follows the device's principle without claiming its behaviour).
    Fidelity { Faithful = "faithful", Modelled = "modelled", Principle = "principle", }
}

/// What a virtual instrument says of the device it stands for: from 0.6 only; a software device
/// only; `fidelity` only beside `stands_for`; `stands_for` of the form `maker/device`.
fn check_stands_for(d: &Device) -> Result<(), Fault> {
    let info = &d.device;
    let bad = |pointer: &str, kind: FaultKind, rule: &str| Err(Fault { kind, file: None, pointer: pointer.into(), rule: rule.into() });
    if d.schema < Some(Version::V0_6) {
        if info.stands_for.is_some() {
            return bad("/device/stands_for", FaultKind::UnknownField, "stands_for is not a field before 0.6");
        }
        if info.fidelity.is_some() {
            return bad("/device/fidelity", FaultKind::UnknownField, "fidelity is not a field before 0.6");
        }
        return Ok(());
    }
    if let Some(s) = &info.stands_for {
        let parts: Vec<&str> = s.split('/').collect();
        let word = |p: &&str| !p.is_empty() && !p.starts_with('.') && !p.chars().any(|c| c.is_whitespace() || c == '{' || c == '}');
        if parts.len() != 2 || !parts.iter().all(word) {
            return bad("/device/stands_for", FaultKind::BadValue, "not a catalogue address (maker/device)");
        }
        if info.kind != Some(DeviceKind::Software) {
            return bad("/device/stands_for", FaultKind::BadValue, "only a software device stands for another (kind: software)");
        }
    } else if info.fidelity.is_some() {
        return bad("/device/fidelity", FaultKind::BadValue, "a fidelity needs the device it stands for (stands_for)");
    }
    Ok(())
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
        /// The maker's SysEx id: one byte, or three starting with 0 (0.7); `null` drops the profile's.
        "manufacturerId" manufacturer_id: Option<Option<Vec<u8>>> = Opt(Nullable(list(U7))),
        /// A maker profile this extends (`maker` or `maker/family`).
        "profile" profile: Option<String> = Opt(STRING),
        /// The identify request, when not the Universal Identity Request.
        "request" request: Option<Option<Vec<FrameToken>>> = Opt(Nullable(list(FRAME_TOKEN))),
        /// Where the request goes (0.5).
        "to" to: Option<Option<IdentityTo>> = Opt(Nullable(closed())),
        /// A Lua script that says whether a reply is the device's (0.5).
        "script" script: Option<Option<String>> = Opt(Nullable(STRING)),
        "unpack" unpack: Option<Option<Unpack>> = Opt(Nullable(closed())),
        /// The start of the reply, recognised element by element.
        "reply" reply: Option<Option<Vec<FrameToken>>> = Opt(Nullable(list(REPLY_TOKEN))),
        "reply_text" reply_text: Option<Option<String>> = Opt(Nullable(STRING)),
        "family" family: Option<Option<Vec<u8>>> = Opt(Nullable(list(BYTE))),
        "member" member: Option<Option<Vec<u8>>> = Opt(Nullable(list(BYTE))),
        "device_id" device_id: Option<Option<DeviceIdRange>> = Opt(Nullable(obj())),
        /// Where the reply carries the firmware version. Every optional key
        /// but `profile` takes `null` (`Some(None)`), which removes the
        /// maker profile's key (23.25).
        "firmware" firmware: Option<Option<FirmwareRule>> = Opt(Nullable(obj())),
        /// The rules on its port names, for a device that does not answer. Before 0.7 each is a
        /// plain name, read as a `contains` rule with no direction, platform or stack.
        "ports" ports: Option<Option<Vec<PortRule>>> = Opt(Nullable(list(obj()))),
        /// The rule on the name the device gives its own MIDI 2.0 endpoint (0.7).
        "endpoint_name" endpoint_name: Option<Option<EndpointRule>> = Opt(Nullable(obj())),
        /// Its USB identities, one per model version or mode.
        "usb" usb: Option<Option<Vec<UsbId>>> = Opt(Nullable(list(obj()))),
        "sends" sends: Option<Option<Sends>> = Opt(Nullable(obj())),
    }
}

words! {
    /// How a rule compares its value with a name, case ignored.
    PortOperator { Equals = "equals", Contains = "contains", StartsWith = "starts_with", EndsWith = "ends_with", }
}

words! {
    /// The direction of the port a rule names.
    PortDirection { Input = "input", Output = "output", }
}

words! {
    /// The system a port name was taken on.
    Platform { Windows = "windows", Linux = "linux", Macos = "macos", }
}

words! {
    /// The naming stack of a platform ([`Stack::platform`]).
    Stack {
        Winmm = "winmm",
        WindowsMidiServices = "windows-midi-services",
        Winrt = "winrt",
        AlsaSeq = "alsa-seq",
        AlsaRawmidi = "alsa-rawmidi",
        Jack = "jack",
        Coremidi = "coremidi",
    }
}

impl Stack {
    /// The platform the stack belongs to.
    pub fn platform(self) -> Platform {
        match self {
            Stack::Winmm | Stack::WindowsMidiServices | Stack::Winrt => Platform::Windows,
            Stack::AlsaSeq | Stack::AlsaRawmidi | Stack::Jack => Platform::Linux,
            Stack::Coremidi => Platform::Macos,
        }
    }
}

/// A `stack` requires a `platform` and belongs to it, on a rule as on a fingerprint; the refusal is
/// at `stack`.
pub(crate) fn check_stack(platform: Option<Platform>, stack: Option<Stack>, at: &At) -> Result<(), Fault> {
    match stack {
        Some(s) if platform != Some(s.platform()) => Err(At::Key(at, "stack").fault(
            FaultKind::BadValue,
            format!("the stack {} needs the platform {}", s.word(), s.platform().word()),
        )),
        _ => Ok(()),
    }
}

object! {
    /// A rule on a port name (0.7). A rule read from a text before 0.7 may hold an empty value,
    /// which matches nothing.
    PortRule {
        "operator" operator: PortOperator = closed(),
        /// The text compared, not empty.
        "value" value: String = TITLE,
        /// The ports it names; both directions when absent.
        "direction" direction: Option<PortDirection> = Opt(closed()),
        /// The platform it holds on; every one when absent.
        "platform" platform: Option<Platform> = Opt(closed()),
        /// The naming stack it holds on; every stack of its platform when absent.
        "stack" stack: Option<Stack> = Opt(closed()),
    }
    check check_port_rule
}

fn check_port_rule(r: &PortRule, at: &At) -> Result<(), Fault> {
    check_stack(r.platform, r.stack, at)
}

object! {
    /// A rule on the endpoint name (0.7): one endpoint serves both directions, and the device
    /// gives its name itself, so it names no direction and no system.
    EndpointRule {
        "operator" operator: PortOperator = closed(),
        /// The text compared, not empty.
        "value" value: String = TITLE,
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
        /// Whether the bank can be written; absent, it can (0.6).
        "writable" writable: Option<bool> = Opt(Bool),
    }
}

object! {
    /// Its programs and banks: how to select one, how they are numbered, how
    /// a sound is taken from it.
    Programs {
        "profile" profile: Option<String> = Opt(STRING),
        /// What a program change selects (sound, pattern, kit…).
        "selects" selects: Option<Option<String>> = Opt(Nullable(STRING)),
        "program_change" program_change: Option<Option<bool>> = Opt(Nullable(Bool)),
        /// Settings on the device needed first, as its manual names them.
        "requires" requires: Option<Option<Vec<String>>> = Opt(Nullable(list(STRING))),
        "banks" banks: Option<Option<Vec<Bank>>> = Opt(Nullable(list(obj()))),
        /// Messages sent before a recall.
        "before" before: Option<Option<Vec<Vec<FrameToken>>>> = Opt(Nullable(list(list(FRAME_TOKEN)))),
        /// Messages sent after a recall (0.5).
        "after" after: Option<Option<Vec<Vec<FrameToken>>>> = Opt(Nullable(list(list(FRAME_TOKEN)))),
        /// A Lua script that writes the recall itself (0.5).
        "recall_script" recall_script: Option<Option<String>> = Opt(Nullable(STRING)),
        "delay_ms" delay_ms: Option<Option<i64>> = Opt(Nullable(at_least(0))),
        /// Every optional key but `profile` takes `null` (`Some(None)`),
        /// which removes the maker profile's key (23.25).
        "dump" dump: Option<Option<Dump>> = Opt(Nullable(obj())),
        "notes" notes: Option<Option<String>> = Opt(Nullable(STRING)),
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
        /// How long the device may take to answer the request, in milliseconds (0.6).
        "reply_timeout_ms" reply_timeout_ms: Option<i64> = Opt(at_least(0)),
    }
}

words! {
    /// Where a control's values start: `unipolar` at its lowest, `bipolar` around a centre (0.7).
    Polarity { Unipolar = "unipolar", Bipolar = "bipolar", }
}

words! {
    /// Whether a control is sent or recognised as absolute values, relative values, both or neither.
    Direction { Absolute = "absolute", Relative = "relative", Both = "both", None = "none", }
}

object! {
    /// A control set by Control Change, a 14-bit CC pair, NRPN or RPN.
    ///
    /// Its `Default` is the base of a struct update (`..Default::default()`), not a valid
    /// descriptor: `osc` is empty, and the reader does not validate a descriptor on its default alone.
    #[derive(Default)]
    CcEntry {
        /// The control's address, after the device's prefix (`/filter/cutoff`).
        "osc" osc: String = ADDRESS,
        /// The name a person reads (0.7); never empty.
        "title" title: Option<String> = Opt(TITLE),
        /// A description for a person, in CommonMark (0.7).
        "description" description: Option<String> = Opt(STRING),
        /// What the values of the control mean, in CommonMark (0.7).
        "value_description" value_description: Option<String> = Opt(STRING),
        /// A note of reserve or history (0.7).
        "notes" notes: Option<String> = Opt(STRING),
        /// The range the `default`, the `values`, the `step_count` and the readings are written in
        /// (0.7); without it, the range of the most precise way.
        "scale" scale: Option<[i64; 2]> = Opt(Pair),
        /// Lowest and highest value, on a control without a way only: with ways, each way says its own.
        "range" range: Option<[i64; 2]> = Opt(Pair),
        /// Whether the control is sent, and how, on a control without a way only.
        "transmit" transmit: Option<Direction> = Opt(closed()),
        /// Whether the control is recognised, and how, on a control without a way only.
        "recognize" recognize: Option<Direction> = Opt(closed()),
        /// The parameter LSB numbers of the block the control stands for, on a control without a way only.
        "nrpn_lsb_range" nrpn_lsb_range: Option<[i64; 2]> = Opt(Pair),
        /// The value of the control on an initialised sound, in the reference scale (0.6).
        "default" default: Option<i64> = Opt(INTEGER),
        /// How important the control is, 1 (essential) to 5 (hidden), as in MIDI-CI Property Exchange (0.6).
        "priority" priority: Option<u8> = Opt(int(1, 5)),
        /// The number of distinct steps of the control, 2 or more (0.6).
        "step_count" step_count: Option<i64> = Opt(at_least(2)),
        /// Named values: each with its value in the reference scale and its title (0.7).
        "values" values: Option<NamedValues> = Opt(NamedValuesCodec),
        "polarity" polarity: Option<Polarity> = Opt(closed()),
        "channel" channel: Option<u8> = Opt(CHANNEL),
        "section" section: Option<String> = Opt(STRING),
        /// The accounts of this control, each with its state and the firmware it holds for (0.7);
        /// never empty.
        "readings" readings: Option<Vec<ControlReading>> = Opt(list_of(obj(), 1, usize::MAX)),
        /// What the control expects: continuous (default), switch,
        /// momentary, trigger, clock or discrete; other words are allowed.
        "expects" expects: Option<String> = Opt(STRING),
        /// osc-bridge extension: a Lua expression applied to the value.
        "transform" transform: Option<String> = Opt(STRING),
        /// osc-bridge extension: its reverse.
        "transform_reverse" transform_reverse: Option<String> = Opt(STRING),
        /// What the control does, in the words every device shares (0.6).
        "role" role: Option<Role> = Opt(obj()),
        /// The messages that reach the control, in the order of preference (0.7); empty when the
        /// message is not known yet: the control is partial, to be completed.
        "ways" ways: Vec<Way> = list(WayOf { sysex: false }),
    }
    kept {
        /// The flat message fields a text before 0.7 gave on the control, held where its ways alone
        /// would not write them back as given. No key of any version: only the writing of an older
        /// version reads it.
        #[doc(hidden)]
        flat: Option<Box<crate::flat::FlatFields>>
    }
}

object! {
    /// A value of a dimension of `contexts`: its `id`, and the channel its messages arrive on when
    /// the descriptor knows it; without `channel` the channel is the user's setting (0.7).
    ContextValue {
        "id" id: Option<String> = Opt(STRING),
        "channel" channel: Option<u8> = Opt(CHANNEL),
    }
    check check_context_value
}

fn check_context_value(v: &ContextValue, at: &At) -> Result<(), Fault> {
    match v.id.as_deref() {
        None | Some("") => Err(At::Key(at, "id").bad("an id is a text that is not empty")),
        Some(_) => Ok(()),
    }
}

object! {
    /// A dimension of `contexts`: the values the device takes in it (0.7).
    ContextDimension {
        "values" values: Vec<ContextValue> = list(obj()),
    }
    check check_context_dimension
}

/// Two values of one dimension never pin the same channel: the fault is at the second. A value with
/// no `channel` is the user's setting and is never compared.
fn check_context_dimension(d: &ContextDimension, at: &At) -> Result<(), Fault> {
    let mut seen: Vec<u8> = Vec::new();
    for (n, v) in d.values.iter().enumerate() {
        let Some(c) = v.channel else { continue };
        if seen.contains(&c) {
            let values = At::Key(at, "values");
            let value = At::Index(&values, n);
            return Err(At::Key(&value, "channel").bad(format!("the channel {c} is already the channel of an earlier value of this dimension")));
        }
        seen.push(c);
    }
    Ok(())
}

/// What `in` says of one dimension: one id or a list of ids, kept as written.
#[derive(Clone, Debug, PartialEq)]
pub enum ContextIds {
    One(String),
    Many(Vec<String>),
}

impl ContextIds {
    pub fn ids(&self) -> &[String] {
        match self {
            ContextIds::One(s) => std::slice::from_ref(s),
            ContextIds::Many(l) => l,
        }
    }
}

pub(crate) struct ContextIdsCodec;

impl Codec for ContextIdsCodec {
    type T = ContextIds;
    fn read(&self, v: &Value, at: &At) -> Result<ContextIds, Fault> {
        match v {
            Value::String(s) => Ok(ContextIds::One(s.clone())),
            Value::Array(_) => list_of(STRING, 1, usize::MAX).read(v, at).map(ContextIds::Many),
            _ => Err(at.bad("an id or a list of ids")),
        }
    }
    fn write(&self, t: &ContextIds) -> Option<Value> {
        match t {
            ContextIds::One(s) => Some(Value::String(s.clone())),
            ContextIds::Many(l) => list_of(STRING, 1, usize::MAX).write(l),
        }
    }
}

/// A slice: a closed interval `[lowest, highest]` of two integers, lowest at most highest.
pub(crate) struct SliceCodec;

impl Codec for SliceCodec {
    type T = [i64; 2];
    fn read(&self, v: &Value, at: &At) -> Result<[i64; 2], Fault> {
        let bounds = v.as_array().filter(|a| a.len() == 2).and_then(|a| Some([a[0].as_i64()?, a[1].as_i64()?]));
        bounds.filter(|b| b[0] <= b[1]).ok_or_else(|| at.bad("a closed interval [lowest, highest] of two integers, lowest at most highest"))
    }
    fn write(&self, t: &[i64; 2]) -> Option<Value> {
        Some(Value::from(t.to_vec()))
    }
}

/// The message kinds of a way, the words of `ctrlType` (0.7).
const KINDS: [&str; 8] = ["cc", "nrpn", "rpn", "chPress", "pPress", "pBend", "note", "sysex"];

object! {
    /// One message that reaches a control, with its numbers, its width, its range and its direction
    /// (0.7). `in` names the contexts the message holds in and `slice` the closed interval of its
    /// values it holds for (proposal 0003); both are read on a way of `cc_params`, and each is a
    /// coordinate of the claim the way makes (`check_claims`).
    #[derive(Default)]
    Way {
        /// The kind of the message: `cc`, `nrpn`, `rpn`, `chPress`, `pPress`, `pBend`, `note`, or
        /// `sysex` for a `params` entry.
        "ctrlType" ctrl_type: String = matching(|s| KINDS.contains(&s), "one of cc, nrpn, rpn, chPress, pPress, pBend, note, sysex"),
        /// The numbers of the message, by kind: absent for `chPress`, `pBend` and `sysex`.
        "ctrlIndex" ctrl_index: Option<Vec<i64>> = Opt(list(INTEGER)),
        /// The `pr` byte of the maker's frame, on a `sysex` way only, as `p`, `c`, `r` and `id`.
        "pr" pr: Option<i64> = Opt(FRAME_BYTE),
        /// The `p` byte of the maker's frame.
        "p" p: Option<i64> = Opt(FRAME_BYTE),
        /// The `c` byte of the maker's frame.
        "c" c: Option<i64> = Opt(FRAME_BYTE),
        /// The `r` byte of the maker's frame.
        "r" r: Option<i64> = Opt(FRAME_BYTE),
        /// The parameter's own number in the maker's frame, any integer.
        "id" id: Option<i64> = Opt(INTEGER),
        /// The width of the value: 7 or 14.
        "value_bits" value_bits: Option<i64> = Opt(INTEGER),
        /// Lowest and highest value of the message.
        "range" range: Option<[i64; 2]> = Opt(Pair),
        /// Whether the way is sent as absolute, relative or both values, or not sent.
        "transmit" transmit: Option<Direction> = Opt(closed()),
        /// Whether the way is recognised as absolute, relative or both values, or not recognised.
        "recognize" recognize: Option<Direction> = Opt(closed()),
        /// For an `nrpn` way: the parameter LSB numbers of the block the way stands for.
        "nrpn_lsb_range" nrpn_lsb_range: Option<[i64; 2]> = Opt(Pair),
        /// The contexts the way holds in: per dimension of the device, one id or a list of ids (0.7).
        "in" held_in: Option<BTreeMap<String, ContextIds>> = Opt(Dict(ContextIdsCodec)),
        /// The closed interval `[lowest, highest]` of the values of the message the way holds for (0.7).
        "slice" slice: Option<[i64; 2]> = Opt(SliceCodec),
    }
    check check_way
}

/// A byte of the maker's frame.
const FRAME_BYTE: crate::json::Int<i64> = int(0, 255);

/// The fields that give the numbers of a `sysex` way.
pub(crate) const SYSEX_NUMBERS: [&str; 5] = ["pr", "p", "c", "r", "id"];

/// A way of an entry of `params` (`sysex`) or of `cc_params`. Its unknown keys are refused first,
/// then a kind its table does not take, then a field its kind does not hold, and last its values
/// (`spec/0.7.md` 5.3.3, the order of the faults of a way).
struct WayOf {
    sysex: bool,
}

impl Codec for WayOf {
    type T = Way;
    fn read(&self, v: &Value, at: &At) -> Result<Way, Fault> {
        let o = crate::json::keys(v, at, Way::KEYS)?;
        if let Some(kind) = o.get("ctrlType").and_then(Value::as_str).filter(|k| KINDS.contains(k)) {
            if (kind == "sysex") != self.sysex {
                let rule = if self.sysex { "a params entry is reached by sysex ways" } else { "a cc_params entry is reached by the kinds of MIDI: sysex is for a params entry" };
                return Err(At::Key(at, "ctrlType").bad(rule));
            }
            let foreign: &[&str] = if self.sysex { &["ctrlIndex"] } else { &SYSEX_NUMBERS };
            if let Some(key) = foreign.iter().find(|k| o.contains_key(**k)) {
                return Err(At::Key(at, key).fault(FaultKind::UnknownField, format!("{key} is not a field of a {kind} way")));
            }
        }
        Way::read(v, at)
    }
    fn write(&self, t: &Way) -> Option<Value> {
        Some(t.write())
    }
}

/// The numbers of a way follow its kind, and its width follows its kind and its numbers
/// (`spec/0.7.md`, `ctrlIndex` and `value_bits`). A `sysex` way gives at least one of its five.
fn check_way(w: &Way, at: &At) -> Result<(), Fault> {
    if w.ctrl_type == "sysex" && [w.pr, w.p, w.c, w.r, w.id].iter().all(Option::is_none) {
        return Err(at.bad("a sysex way gives at least one of pr, p, c, r, id"));
    }
    let index_at = At::Key(at, "ctrlIndex");
    let (count, top): (&[usize], i64) = match w.ctrl_type.as_str() {
        "cc" => (&[1, 2], 127),
        "nrpn" | "rpn" => (&[2], 127),
        "pPress" | "note" => (&[1], 127),
        _ => (&[], 0),
    };
    match (&w.ctrl_index, count.is_empty()) {
        (Some(_), true) => return Err(index_at.bad("absent: this kind has no numbers")),
        (None, false) => return Err(index_at.fault(FaultKind::Missing, "ctrlIndex is required: the numbers of the message")),
        (Some(i), false) if !count.contains(&i.len()) || i.iter().any(|n| *n < 0 || *n > top) => {
            let rule = format!("{} integers from 0 to {top}", count.iter().map(usize::to_string).collect::<Vec<_>>().join(" or "));
            return Err(index_at.bad(rule));
        }
        _ => {}
    }
    let bits_at = At::Key(at, "value_bits");
    let long = w.ctrl_index.as_ref().is_some_and(|i| i.len() == 2);
    match (w.ctrl_type.as_str(), w.value_bits) {
        ("nrpn" | "rpn", None) => return Err(bits_at.fault(FaultKind::Missing, "value_bits is required with an nrpn or an rpn: 7 or 14")),
        ("nrpn" | "rpn", Some(7 | 14)) => {}
        ("nrpn" | "rpn", Some(_)) => return Err(bits_at.bad("7 (the data MSB alone) or 14 (the data MSB and LSB)")),
        ("cc", None) if long => return Err(bits_at.fault(FaultKind::Missing, "value_bits: 14 is required with a cc of two numbers")),
        ("cc", Some(14)) if long => {}
        ("cc", Some(_)) if long => return Err(bits_at.bad("14: a cc of two numbers is 14 bits")),
        ("cc", None | Some(7)) => {}
        ("cc", Some(_)) => return Err(bits_at.bad("7: a cc of one number is 7 bits")),
        ("chPress" | "pPress" | "note", Some(_)) => return Err(bits_at.bad("none: the width of this kind is implied")),
        ("pBend", Some(n)) if n != 14 => return Err(bits_at.bad("14: a pitch bend is 14 bits")),
        ("sysex", Some(n)) if n != 7 && n != 14 => return Err(bits_at.bad("7 or 14")),
        _ => {}
    }
    if w.nrpn_lsb_range.is_some() && w.ctrl_type != "nrpn" {
        return Err(At::Key(at, "nrpn_lsb_range").bad("nrpn_lsb_range is for an nrpn way"));
    }
    Ok(())
}

impl Way {
    /// The range the way's values lie in: its `range`, else the one its kind and width imply (0 to
    /// 127 for 7 bits, 0 to 16383 for 14).
    pub fn scale(&self) -> [i64; 2] {
        self.range.unwrap_or_else(|| {
            let wide = match self.ctrl_type.as_str() {
                "pBend" => true,
                "chPress" | "pPress" | "note" => false,
                "cc" => self.ctrl_index.as_ref().is_some_and(|i| i.len() == 2),
                _ => self.value_bits == Some(14),
            };
            if wide { [0, 16383] } else { [0, 127] }
        })
    }
}

object! {
    /// A named value of a control: a value of the reference scale and its title (0.7).
    NamedValue {
        "value" value: i64 = INTEGER,
        "title" title: String = STRING,
    }
}

/// The named values of a control, in the order written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NamedValues(pub Vec<NamedValue>);

impl NamedValues {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> std::slice::Iter<'_, NamedValue> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a NamedValues {
    type Item = &'a NamedValue;
    type IntoIter = std::slice::Iter<'a, NamedValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// The value of the first named value with this title.
impl std::ops::Index<&str> for NamedValues {
    type Output = i64;
    fn index(&self, title: &str) -> &i64 {
        &self.0.iter().find(|v| v.title == title).unwrap_or_else(|| panic!("no value titled {title:?}")).value
    }
}

pub(crate) struct NamedValuesCodec;

impl Codec for NamedValuesCodec {
    type T = NamedValues;
    fn read(&self, v: &Value, at: &At) -> Result<NamedValues, Fault> {
        list(obj::<NamedValue>()).read(v, at).map(NamedValues)
    }
    fn write(&self, t: &NamedValues) -> Option<Value> {
        list(obj::<NamedValue>()).write(&t.0)
    }
}

/// The scale a way of a control is read in, and the conversion of a value between the scales of the
/// control and of a way: the affine scaling of one range on the other, rounded to the nearest, the
/// half upward (`spec/0.7.md`, the scale of a control).
fn convert_between(v: i64, from: [i64; 2], to: [i64; 2]) -> i64 {
    let (num, den) = (i128::from(v - from[0]) * i128::from(to[1] - to[0]), i128::from(from[1] - from[0]));
    if den == 0 {
        return to[0];
    }
    let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
    to[0] + (2 * num + den).div_euclid(2 * den) as i64
}

/// The reference scale of the ways of a control: the scale it gives, else the one of its most precise
/// way (the highest maximum, the first on a tie); `None` when neither says it.
pub(crate) fn known_scale(scale: Option<[i64; 2]>, ways: &[Way]) -> Option<[i64; 2]> {
    scale.or_else(|| {
        let mut best: Option<(&Way, [i64; 2])> = None;
        for w in ways {
            let s = w.scale();
            if best.is_none_or(|(_, b)| s[1] > b[1]) {
                best = Some((w, s));
            }
        }
        best.filter(|(w, _)| w.ctrl_type != "sysex" || w.range.is_some() || w.value_bits.is_some()).map(|(_, s)| s)
    })
}

macro_rules! control_scale {
    ($entry:ident) => {
        impl $entry {
            /// The ways of reaching the control in the order written.
            pub fn ways(&self) -> Vec<Way> {
                self.ways.clone()
            }
            /// The reference scale: the range `default`, `values` and `step_count` are written in.
            pub fn reference_scale(&self) -> [i64; 2] {
                self.known_scale().unwrap_or([0, 127])
            }
            fn known_scale(&self) -> Option<[i64; 2]> {
                // A `range` is on the entry only when it has no way (`check_fields_of_a_partial_control`).
                known_scale(self.scale.or(self.range), &self.ways)
            }
            /// A value of the reference scale as the way carries it.
            pub fn in_way(&self, value: i64, way: &Way) -> i64 {
                convert_between(value, self.reference_scale(), way.scale())
            }
            /// A value received on a way, in the reference scale.
            pub fn from_way(&self, value: i64, way: &Way) -> i64 {
                convert_between(value, way.scale(), self.reference_scale())
            }
        }
    };
}
control_scale!(CcEntry);
control_scale!(ParamEntry);

object! {
    /// A parameter read and written by SysEx.
    ///
    /// Its `Default` is the base of a struct update (`..Default::default()`), not a valid
    /// descriptor: `osc` is empty, and the reader does not validate a descriptor on its default alone.
    #[derive(Default)]
    ParamEntry {
        "osc" osc: String = ADDRESS,
        /// The name a person reads (0.7); never empty.
        "title" title: Option<String> = Opt(TITLE),
        /// A description for a person, in CommonMark (0.7).
        "description" description: Option<String> = Opt(STRING),
        /// The range the `default`, the `values`, the `step_count` and the readings are written in
        /// (0.7); without it, the range of the most precise way.
        "scale" scale: Option<[i64; 2]> = Opt(Pair),
        /// Lowest and highest value, on an entry without a way only: with ways, each way says its own.
        "range" range: Option<[i64; 2]> = Opt(Pair),
        /// Whether the parameter is sent, and how, on an entry without a way only.
        "transmit" transmit: Option<Direction> = Opt(closed()),
        /// Whether the parameter is recognised, and how, on an entry without a way only.
        "recognize" recognize: Option<Direction> = Opt(closed()),
        /// The value of the control on an initialised sound, in the reference scale (0.6).
        "default" default: Option<i64> = Opt(INTEGER),
        /// How important the control is, 1 (essential) to 5 (hidden), as in MIDI-CI Property Exchange (0.6).
        "priority" priority: Option<u8> = Opt(int(1, 5)),
        /// The number of distinct steps of the control, 2 or more (0.6).
        "step_count" step_count: Option<i64> = Opt(at_least(2)),
        "section" section: Option<String> = Opt(STRING),
        /// The accounts of this control, each with its state and the firmware it holds for (0.7);
        /// never empty.
        "readings" readings: Option<Vec<ControlReading>> = Opt(list_of(obj(), 1, usize::MAX)),
        "expects" expects: Option<String> = Opt(STRING),
        /// Named values: each with its value in the reference scale and its title (0.7).
        "values" values: Option<NamedValues> = Opt(NamedValuesCodec),
        "transform" transform: Option<String> = Opt(STRING),
        "transform_reverse" transform_reverse: Option<String> = Opt(STRING),
        /// What the control does, in the words every device shares (0.6).
        "role" role: Option<Role> = Opt(obj()),
        /// The messages that address the parameter, `sysex` ways (0.7); empty when the message is
        /// not known yet: the entry is partial, to be completed.
        "ways" ways: Vec<Way> = list(WayOf { sysex: true }),
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
    ///
    /// Its `Default` is the base of a struct update (`..Default::default()`), not a valid
    /// descriptor: `osc` is empty, and the reader does not validate a descriptor on its default alone.
    #[derive(Default)]
    Command {
        "osc" osc: Address = AddressCodec(AddressPlace::Command),
        /// The number a controller's preset gives the command (0.7); not unique.
        "id" id: Option<i64> = Opt(INTEGER),
        /// The name a person reads (0.7); never empty.
        "title" title: Option<String> = Opt(TITLE),
        /// A description for a person, in CommonMark (0.7).
        "description" description: Option<String> = Opt(STRING),
        "section" section: Option<String> = Opt(STRING),
        "notes" notes: Option<String> = Opt(STRING),
        "args" args: Option<Vec<Arg>> = Opt(list(obj())),
        "pre" pre: Option<Vec<Pre>> = Opt(list(obj())),
        "frame" frame: Option<Vec<FrameToken>> = Opt(list(FRAME_TOKEN)),
        "forward" forward: Option<Forward> = Opt(obj()),
        /// osc-bridge extension: a Lua script that changes the message
        /// before it is sent.
        "script" script: Option<String> = Opt(STRING),
        /// The accounts of this command, each with its state and the firmware it holds for (0.7);
        /// never empty. The command itself is its own reading when this is absent.
        "readings" readings: Option<Vec<Reading>> = Opt(list_of(obj(), 1, usize::MAX)),
    }
}

words! {
    /// How far a reading is held (0.7), the ranks of Wikidata: `valid` is preferred, `to-confirm`
    /// normal, `refuted` deprecated.
    ReadingState { Valid = "valid", ToConfirm = "to-confirm", Refuted = "refuted", }
}

object! {
    /// One account of a command for a firmware (0.7), kept raw: `title` and `args` replace those of
    /// the command for whoever shows the reading, and are never merged with them.
    Reading {
        "state" state: ReadingState = closed(),
        "firmware" firmware: String = STRING,
        /// The `id` of a source of the descriptor that holds this account.
        "source" source: Option<String> = Opt(STRING),
        /// The name a person reads; never empty.
        "title" title: Option<String> = Opt(TITLE),
        "args" args: Option<Vec<Arg>> = Opt(list(obj())),
    }
}

object! {
    /// One account of a control for a firmware (0.7), kept raw: `title` and `range` are those
    /// written, never merged with the control's. The `range` is on the reference scale of the control.
    ControlReading {
        "state" state: ReadingState = closed(),
        "firmware" firmware: String = STRING,
        /// The `id` of a source of the descriptor that holds this account.
        "source" source: Option<String> = Opt(STRING),
        /// The name a person reads; never empty.
        "title" title: Option<String> = Opt(TITLE),
        /// The range of the control in this reading, `[lowest, highest]`, inside the reference scale.
        "range" range: Option<[i64; 2]> = Opt(Pair),
    }
}

/// The readings that hold for `firmware`: those written for that very text, `valid` then
/// `to-confirm` then (if `with_refuted`) `refuted`, each state in the order written; when there
/// are none, every reading that is not `refuted` (all if `with_refuted`) in the order written.
fn pick_readings<'a, R>(all: &'a [R], firmware: &str, with_refuted: bool, state: impl Fn(&R) -> ReadingState, fw: impl Fn(&R) -> &str) -> Vec<&'a R> {
    let wanted = |r: &&R| with_refuted || state(r) != ReadingState::Refuted;
    let mut held: Vec<&R> = all.iter().filter(|r| fw(r) == firmware).filter(wanted).collect();
    if held.is_empty() {
        return all.iter().filter(wanted).collect();
    }
    held.sort_by_key(|r| match state(r) {
        ReadingState::Valid => 0,
        ReadingState::ToConfirm => 1,
        ReadingState::Refuted => 2,
    });
    held
}

macro_rules! control_readings {
    ($entry:ident) => {
        impl $entry {
            /// The readings that hold for `firmware`, by the rule of a command's
            /// (`Command::readings_for`). A control without `readings` gives none.
            pub fn readings_for(&self, firmware: &str, with_refuted: bool) -> Vec<&ControlReading> {
                pick_readings(self.readings.as_deref().unwrap_or_default(), firmware, with_refuted, |r| r.state, |r| r.firmware.as_str())
            }
        }
    };
}
control_readings!(CcEntry);
control_readings!(ParamEntry);

impl Command {
    /// The readings that hold for `firmware`, best first: those written for that very text,
    /// `valid` then `to-confirm` then (if `with_refuted`) `refuted`, each state in the order
    /// written; when there are none, every reading of the command that is not `refuted` (all of them
    /// if `with_refuted`) in the order written, none chosen. A command without `readings` gives
    /// none: its own reading is the command itself.
    pub fn readings_for(&self, firmware: &str, with_refuted: bool) -> Vec<&Reading> {
        pick_readings(self.readings.as_deref().unwrap_or_default(), firmware, with_refuted, |r| r.state, |r| r.firmware.as_str())
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
        /// The name a person reads (0.7); never empty.
        "title" title: Option<String> = Opt(TITLE),
        /// A description for a person, in CommonMark (0.7).
        "description" description: Option<String> = Opt(STRING),
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
///
/// Its `Default` is the base of a struct update (`..Default::default()`), not a valid
/// descriptor: `kind` is empty, and the reader does not validate a descriptor on its default alone.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Source {
    /// What kind of source: `vendor-doc`, `hardware-verified`,
    /// `community-re`, `knobkraft`…
    pub kind: String,
    /// What a reading names it by (0.7): never empty, unique in the sources of a file. Before 0.7 an
    /// `id` is a key kept as it is, never checked.
    pub id: Option<String>,
    /// Who supplies the source (0.7): an identifier of the table of providers. Before 0.7 a
    /// `provider` is a key kept as it is, never checked.
    pub provider: Option<String>,
    pub url: Option<String>,
    /// The firmware the source documents.
    pub firmware: Option<String>,
    pub author: Option<String>,
    pub attribution: Option<String>,
    /// From 0.7 an SPDX licence expression ([`crate::LicenseExpression`]), before it an SPDX id;
    /// or `public-domain`, `NOASSERTION` or `NONE`.
    pub license: Option<String>,
    pub commit: Option<String>,
    pub imported_at: Option<String>,
    pub notes: Option<String>,
    /// Its other keys, kept as they are.
    pub extra: BTreeMap<String, Value>,
}

impl Object for Source {
    const KEYS: &'static [&'static str] =
        &["type", "id", "provider", "url", "firmware", "author", "attribution", "license", "commit", "imported_at", "notes"];

    fn read(v: &Value, at: &At) -> Result<Self, Fault> {
        let o = v.as_object().ok_or_else(|| at.fault(FaultKind::BadValue, "an object"))?;
        let text = |key| crate::json::field(o, key, at, Opt(STRING));
        Ok(Source {
            kind: crate::json::field(o, "type", at, STRING)?,
            id: o.get("id").and_then(Value::as_str).map(str::to_string),
            provider: o.get("provider").and_then(Value::as_str).map(str::to_string),
            url: text("url")?,
            firmware: text("firmware")?,
            author: text("author")?,
            attribution: text("attribution")?,
            license: text("license")?,
            commit: text("commit")?,
            imported_at: text("imported_at")?,
            notes: text("notes")?,
            extra: o.iter().filter(|(k, v)| !Self::KEYS.contains(&k.as_str()) || (matches!(k.as_str(), "id" | "provider") && !v.is_string())).map(|(k, v)| (k.clone(), v.clone())).collect(),
        })
    }

    fn write(&self) -> Value {
        let mut o = serde_json::Map::new();
        o.insert("type".into(), Value::String(self.kind.clone()));
        let texts = [
            ("id", &self.id),
            ("provider", &self.provider),
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

/// The key the credit is written under before 0.7.
pub(crate) const LEGACY_SOURCES: &str = "_sources";

/// The key of a source that names the maker profile it comes from (0.7).
pub(crate) const PROFILE_OF_SOURCE: &str = "profile";

/// Whether `source` reads as a source of 0.7, taken alone; its `id`, if it has one.
pub(crate) fn source_of_0_7(source: &Value) -> Option<Option<String>> {
    let list = Some(vec![Source::read(source, &At::Root).ok()?]);
    let dated = source.get("imported_at").is_some();
    check_credit(Some(Version::V0_7), &[(String::new(), &list)]).ok().filter(|()| !dated)?;
    Some(list.into_iter().flatten().next().and_then(|s| s.id))
}

/// The keys of the credit of a maker profile, a sound, a fingerprint, a firmware or an assembly
/// `v` of version `schema`, judged before it is read: from 0.7 `_sources` is refused, and the date of
/// a source of `sources`, which is no field; before 0.7 `sources` is no field.
pub(crate) fn check_credit_keys(v: &Value, at: &At, schema: Option<Version>) -> Result<(), Fault> {
    let Some(o) = v.as_object() else { return Ok(()) };
    if schema >= Some(Version::V0_7) {
        refuse_legacy_sources(o, at)?;
        return refuse_paths(v, at, &[&["sources", "*", "imported_at"]], "is not a field from 0.7");
    }
    match o.contains_key("sources") {
        true => Err(At::Key(at, "sources").fault(FaultKind::UnknownField, "sources is not a field before 0.7")),
        false => Ok(()),
    }
}

/// `o` with the key `from` named `to`, at the same place.
pub(crate) fn rename_key(o: &serde_json::Map<String, Value>, from: &str, to: &str) -> serde_json::Map<String, Value> {
    o.iter().map(|(k, v)| (if k == from { to.to_string() } else { k.clone() }, v.clone())).collect()
}

/// The keys 0.7 removes from the objects the format closes, as paths from the root of a device or
/// a profile (`*` is every entry of a list). A reader of 0.7 refuses them as unknown fields,
/// the date of a source of `sources` and of `midi_of_sources` too, though a source is an open
/// object. The converter 0.6 to 0.7 drops them, but for that date, which it keeps as a note
/// ([`drop_removed_in_0_7`]).
const REMOVED_IN_0_7: &[&[&str]] = &[
    &["cc_params", "entries", "*", "transform_reverse"],
    &["params", "entries", "*", "transform_reverse"],
    &["device", "revision"],
    &["device", "transport", "rate_limit_hz"],
    &["sysex", "footer"],
    &["midi_out", "note_offset"],
    &["identity", "sends"],
    &["sources", "*", "imported_at"],
    &["midi_of_sources", "*", "imported_at"],
];

/// The keys defined only from 0.7, as paths from the root of a device (`*` is every entry of a
/// list). A reader of a version before 0.7 refuses them as unknown fields; a field new in 0.7 is
/// one line more.
const FROM_0_7: &[&[&str]] = &[
    &["contexts"],
    &["cc_params", "entries", "*", "title"],
    &["cc_params", "entries", "*", "description"],
    &["cc_params", "entries", "*", "value_description"],
    &["cc_params", "entries", "*", "notes"],
    &["cc_params", "entries", "*", "readings"],
    &["params", "entries", "*", "readings"],
    &["params", "entries", "*", "title"],
    &["params", "entries", "*", "description"],
    &["cc_params", "entries", "*", "polarity"],
    &["identity", "manufacturerId"],
    &["identity", "endpoint_name"],
    &["commands", "*", "id"],
    &["commands", "*", "title"],
    &["commands", "*", "description"],
    &["commands", "*", "readings"],
    &["triggers", "entries", "*", "title"],
    &["triggers", "entries", "*", "description"],
];

/// The first key of `paths` that `v` carries, refused as an unknown field at its pointer.
pub(crate) fn refuse_paths(v: &Value, at: &At, paths: &[&[&str]], reason: &str) -> Result<(), Fault> {
    fn walk(v: &Value, path: &[&str], at: &At, reason: &str) -> Result<(), Fault> {
        let (key, rest) = path.split_first().expect("a path is not empty");
        match (*key, v) {
            ("*", Value::Array(items)) => items.iter().enumerate().try_for_each(|(i, x)| walk(x, rest, &At::Index(at, i), reason)),
            (_, Value::Object(o)) if rest.is_empty() => match o.contains_key(*key) {
                true => Err(At::Key(at, key).fault(FaultKind::UnknownField, format!("{key} {reason}"))),
                false => Ok(()),
            },
            (_, Value::Object(o)) => o.get(*key).map_or(Ok(()), |x| walk(x, rest, &At::Key(at, key), reason)),
            _ => Ok(()),
        }
    }
    paths.iter().try_for_each(|path| walk(v, path, at, reason))
}

/// The first key defined only from 0.7 that `v` carries, refused as an unknown field at its pointer.
pub(crate) fn refuse_from_0_7(v: &Value, at: &At) -> Result<(), Fault> {
    refuse_paths(v, at, FROM_0_7, "is not a field before 0.7")
}

/// The first removed key `v` carries, refused as an unknown field at its pointer.
pub(crate) fn refuse_removed_in_0_7(v: &Value, at: &At) -> Result<(), Fault> {
    refuse_paths(v, at, REMOVED_IN_0_7, "is not a field from 0.7")
}

/// The `id` of the sources of a file (those of `lists`, in order): each is a text that is not empty,
/// and written once, `seen` holding those of the file met before; the later entry is refused.
fn check_source_ids<'a>(lists: &[Credit<'a>], seen: &mut std::collections::HashSet<&'a str>) -> Result<(), Fault> {
    for (key, list) in lists {
        for (n, s) in list.iter().flatten().enumerate() {
            let at = format!("/{key}/{n}");
            if s.extra.contains_key("id") || s.id.as_deref() == Some("") {
                return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("{at}/id"), rule: "an id is a text that is not empty".into() });
            }
            if let Some(id) = &s.id {
                if !seen.insert(id.as_str()) {
                    return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: at, rule: format!("the id {id} is written twice in the sources") });
                }
            }
        }
    }
    Ok(())
}

/// The `source` of a reading names an `id` of the sources of the descriptor (its `sources` and
/// `midi_of_sources`: for a file with `midi_of` that is the union with its target, so the reader
/// of a file alone leaves this to `resolve`).
pub(crate) fn check_reading_sources(d: &Device) -> Result<(), Fault> {
    let ids: std::collections::HashSet<&str> = [&d.sources, &d.midi_of_sources].into_iter().flatten().flatten().filter_map(|s| s.id.as_deref()).collect();
    for (n, c) in d.commands.iter().flatten().enumerate() {
        for (m, r) in c.readings.iter().flatten().enumerate() {
            if let Some(s) = r.source.as_deref().filter(|s| !ids.contains(s)) {
                let rule = format!("the source {s} names no id of the sources");
                return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("/commands/{n}/readings/{m}/source"), rule });
            }
        }
    }
    let controls = d.cc_params.iter().flat_map(|t| &t.entries).enumerate().map(|(n, e)| (format!("/cc_params/entries/{n}"), e.readings.as_deref()));
    let params = d.params.iter().flat_map(|t| &t.entries).enumerate().map(|(n, e)| (format!("/params/entries/{n}"), e.readings.as_deref()));
    for (at, readings) in controls.chain(params) {
        for (m, r) in readings.iter().flat_map(|l| l.iter()).enumerate() {
            if let Some(s) = r.source.as_deref().filter(|s| !ids.contains(s)) {
                let rule = format!("the source {s} names no id of the sources");
                return Err(Fault { kind: FaultKind::BadValue, file: None, pointer: format!("{at}/readings/{m}/source"), rule });
            }
        }
    }
    Ok(())
}

/// `v` without the keys 0.7 removes from the objects it closes. The keys of a source are not
/// dropped here: the converter carries each source ([`crate::convert::carry_source`]), its date
/// included.
pub(crate) fn drop_removed_in_0_7(v: &mut Value) {
    fn walk(v: &mut Value, path: &[&str]) {
        let Some((key, rest)) = path.split_first() else { return };
        match (*key, v) {
            ("*", Value::Array(items)) => items.iter_mut().for_each(|x| walk(x, rest)),
            (_, Value::Object(o)) if rest.is_empty() => {
                o.remove(*key);
            }
            (_, Value::Object(o)) => {
                if let Some(x) = o.get_mut(*key) {
                    walk(x, rest);
                }
            }
            _ => {}
        }
    }
    let of_a_source = |path: &[&str]| matches!(path.first(), Some(&("sources" | "midi_of_sources")));
    REMOVED_IN_0_7.iter().filter(|path| !of_a_source(path)).for_each(|path| walk(v, path));
}

/// A descriptor of 0.7 that names its credit `sources` carries no `_sources`, whatever its value.
fn refuse_legacy_sources(o: &serde_json::Map<String, Value>, at: &At) -> Result<(), Fault> {
    match o.contains_key(LEGACY_SOURCES) {
        true => Err(At::Key(at, LEGACY_SOURCES).fault(FaultKind::BadValue, "`_sources` is not read from 0.7: the credit is `sources`")),
        false => Ok(()),
    }
}

/// The device of `v`. The credit is `sources` from 0.7, where `_sources` is refused (its value is
/// not read); before 0.7 it is `_sources`, and `sources` is no field.
pub(crate) fn read_device(v: &Value, at: &At) -> Result<Device, Fault> {
    let Some(o) = v.as_object() else { return Device::read(v, at) };
    let schema = crate::json::field(o, "$schema", at, Opt(SchemaUrlFrom05("device"))).ok().flatten();
    if schema >= Some(Version::V0_7) {
        refuse_removed_in_0_7(v, at)?;
        refuse_legacy_sources(o, at)?;
        return Device::read(v, at);
    }
    refuse_paths(v, at, FROM_0_7, "is not a field before 0.7")?;
    for key in ["sources", "midi_of_sources"] {
        if o.contains_key(key) {
            return Err(At::Key(at, key).fault(FaultKind::UnknownField, format!("{key} is not a field before 0.7")));
        }
    }
    // The reader models the form of 0.7: the controls of an older text are read in their own form,
    // with its checks, and lifted to ways.
    let (mut lifted, kept) = crate::flat::lift(o, schema, at)?;
    move_manufacturer_id(&mut lifted);
    let rules = take_port_names(&mut lifted, at)?;
    let renamed = Value::Object(rename_key(&lifted, LEGACY_SOURCES, "sources"));
    let mut device = Device::read(&renamed, at).map_err(|mut f| {
        if f.pointer == "/sources" || f.pointer.starts_with("/sources/") {
            f.pointer = format!("/_{}", &f.pointer[1..]);
        }
        f
    })?;
    put_port_rules(device.identity.as_mut(), rules);
    for (entry, flat) in device.cc_params.iter_mut().flat_map(|t| &mut t.entries).zip(kept) {
        entry.flat = flat;
    }
    // `provider` is a field from 0.7: before, it is a key of the source like another.
    for source in device.sources.iter_mut().flatten() {
        if let Some(p) = source.provider.take() {
            source.extra.insert("provider".into(), Value::String(p));
        }
    }
    Ok(device)
}

/// The profile of `v`, read in the form of 0.7 as [`read_device`] reads a device. The credit is
/// `sources` from 0.7, where `_sources` is refused; before 0.7 `sources` is no field and `_sources`
/// is a note.
pub(crate) fn read_profile(v: &Value, at: &At) -> Result<Profile, Fault> {
    let Some(o) = v.as_object() else { return Profile::read(v, at) };
    let schema = crate::json::field(o, "$schema", at, Opt(SchemaUrlFrom05("profile"))).ok().flatten();
    if schema >= Some(Version::V0_7) {
        refuse_removed_in_0_7(v, at)?;
        check_credit_keys(v, at, schema)?;
        return Profile::read(v, at);
    }
    refuse_from_0_7(v, at)?;
    check_credit_keys(v, at, schema)?;
    let mut older = o.clone();
    let rules = take_port_names(&mut older, at)?;
    let mut profile = Profile::read(&Value::Object(older), at)?;
    put_port_rules(profile.identity.as_mut(), rules);
    Ok(profile)
}

/// The port names of a text before 0.7, taken out of `identity.ports` and read as the rules 0.7
/// says them: each name a `contains` rule with no direction, platform or stack, an empty one too.
/// An entry that is no text is refused. A `ports` that is no list stays, for the reader to judge.
fn take_port_names(v: &mut serde_json::Map<String, Value>, at: &At) -> Result<Option<Vec<PortRule>>, Fault> {
    let Some(Value::Object(identity)) = v.get_mut("identity") else { return Ok(None) };
    let Some(Value::Array(names)) = identity.get("ports") else { return Ok(None) };
    let (block, ports) = (At::Key(at, "identity"), "ports");
    let list = At::Key(&block, ports);
    let rules = names
        .iter()
        .enumerate()
        .map(|(n, name)| {
            let value = STRING.read(name, &At::Index(&list, n))?;
            Ok(PortRule {
                operator: PortOperator::Contains,
                value,
                direction: None,
                platform: None,
                stack: None,
                remarks: Default::default(),
            })
        })
        .collect::<Result<Vec<_>, Fault>>()?;
    identity.remove(ports);
    Ok(Some(rules))
}

fn put_port_rules(identity: Option<&mut Identity>, rules: Option<Vec<PortRule>>) {
    if let (Some(identity), Some(rules)) = (identity, rules) {
        identity.ports = Some(Some(rules));
    }
}

/// The other way: each rule of a text before 0.7 written as its plain name.
fn port_rules_to_names(v: &mut Value) {
    if let Some(Value::Array(ports)) = v.get_mut("identity").and_then(|i| i.get_mut("ports")) {
        for rule in ports {
            if let Some(name) = rule.get_mut("value") {
                *rule = name.take();
            }
        }
    }
}

/// The text of a profile as JSON: its port names as plain texts before 0.7.
pub(crate) fn write_profile(p: &Profile) -> Value {
    let mut v = p.write();
    if p.schema < Some(Version::V0_7) {
        port_rules_to_names(&mut v);
    }
    v
}

/// `device.manufacturer_id` of a device of a version before 0.7 as 0.7 says it: `identity.manufacturerId`,
/// the same bytes, the block made when the file has none. A file with `midi_of` and no `identity` block
/// gets none (the block would replace its target's, `manufacturerId` included): the value stays at
/// `device._manufacturer_id`, a note. `device_id` stays in `device`.
pub(crate) fn move_manufacturer_id(v: &mut serde_json::Map<String, Value>) {
    let Some(Value::Object(device)) = v.get_mut("device") else { return };
    if !device.contains_key("manufacturer_id") {
        return;
    }
    let linked = v.contains_key("midi_of");
    let has_identity = v.get("identity").is_some_and(Value::is_object);
    let Some(Value::Object(device)) = v.get_mut("device") else { return };
    if linked && !has_identity {
        if !device.contains_key("_manufacturer_id") {
            *device = rename_key(device, "manufacturer_id", "_manufacturer_id");
        }
        return;
    }
    let Some(bytes) = device.remove("manufacturer_id") else { return };
    let identity = v.entry("identity").or_insert_with(|| Value::Object(Default::default()));
    if let Value::Object(i) = identity {
        i.insert("manufacturerId".into(), bytes);
    }
}

/// The other way, for a device of a version before 0.7 written as its text had it.
fn manufacturer_id_to_device(v: &mut Value) {
    let Some(o) = v.as_object_mut() else { return };
    let bytes = match o.get_mut("identity").and_then(Value::as_object_mut) {
        Some(i) => i.remove("manufacturerId"),
        None => None,
    };
    if o.get("identity").and_then(Value::as_object).is_some_and(|i| i.is_empty()) {
        o.remove("identity");
    }
    let (linked, has_identity) = (o.contains_key("midi_of"), o.contains_key("identity"));
    let Some(Value::Object(device)) = o.get_mut("device") else { return };
    match bytes {
        Some(b) => {
            device.insert("manufacturer_id".into(), b);
        }
        None if linked && !has_identity && device.contains_key("_manufacturer_id") && !device.contains_key("manufacturer_id") => {
            *device = rename_key(device, "_manufacturer_id", "manufacturer_id");
        }
        None => {}
    }
}

/// The text of a device as JSON: before 0.7, its credit under `_sources` and its port names as
/// plain texts.
pub(crate) fn write_device(d: &Device) -> Value {
    match d.write() {
        Value::Object(o) if d.schema < Some(Version::V0_7) => {
            let mut v = Value::Object(rename_key(&o, "sources", LEGACY_SOURCES));
            let kept = d.cc_params.iter().flat_map(|t| &t.entries).map(|e| e.flat.as_deref()).collect::<Vec<_>>();
            crate::flat::flatten(&mut v, d.schema, &kept);
            manufacturer_id_to_device(&mut v);
            port_rules_to_names(&mut v);
            v
        }
        other => other,
    }
}
