//! The device descriptor of OSD 0.4 (`schemas/0.4/device.schema.json`) and
//! the maker profile (`schemas/0.4/profile.schema.json`), whose blocks are
//! the device's.

use crate::json::{
    at_least, closed, int, list, obj, object, words, Any, At, Bool, Codec, Dict, Nullable, Object, Opt, Pair,
    ADDRESS, INTEGER, STRING,
};
use crate::{Fault, FaultKind, SchemaUrlFrom05, Version};
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
        "midi_in" midi_in: Option<MidiIn> = Opt(obj()),
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
}

object! {
    /// A maker profile: what several devices of a maker or of a family
    /// share, taken up by the device blocks that name it.
    Profile {
        "$schema" schema: Option<Version> = Opt(SchemaUrlFrom05("profile")),
        "identity" identity: Option<Identity> = Opt(obj()),
        "programs" programs: Option<Programs> = Opt(obj()),
    }
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

object! {
    /// What the device sends while played, and the address (an osc-bridge
    /// extension) each message is reported at.
    MidiIn {
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

/// One byte of a message, or a placeholder filled when sending: an argument
/// (`{value}`, `{value:u14_lsb}`), the device id (`{dev}`), a checksum
/// (`{checksum:roland}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameToken {
    Byte(u8),
    Placeholder(String),
}

/// A placeholder: `^\{.+\}$`, braces around at least one character that is
/// not a line break.
fn placeholder(s: &str) -> bool {
    let inner = s.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
    inner.is_some_and(|i| !i.is_empty() && !i.contains(['\n', '\r', '\u{2028}', '\u{2029}']))
}

pub(crate) struct FrameTokenCodec;
pub(crate) const FRAME_TOKEN: FrameTokenCodec = FrameTokenCodec;

impl Codec for FrameTokenCodec {
    type T = FrameToken;
    fn read(&self, v: &Value, at: &At) -> Result<FrameToken, Fault> {
        match v {
            Value::String(s) if placeholder(s) => Ok(FrameToken::Placeholder(s.clone())),
            Value::Number(_) => BYTE.read(v, at).map(FrameToken::Byte),
            _ => Err(at.fault(FaultKind::BadValue, "a byte (0 to 255) or a {placeholder}")),
        }
    }
    fn write(&self, t: &FrameToken) -> Option<Value> {
        Some(match t {
            FrameToken::Byte(b) => Value::from(*b),
            FrameToken::Placeholder(s) => Value::String(s.clone()),
        })
    }
}

/// One byte of an expected identity reply, or a placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyToken {
    Byte(u8),
    Placeholder(String),
}

pub(crate) struct ReplyTokenCodec;

impl Codec for ReplyTokenCodec {
    type T = ReplyToken;
    fn read(&self, v: &Value, at: &At) -> Result<ReplyToken, Fault> {
        match v {
            Value::String(s) => Ok(ReplyToken::Placeholder(s.clone())),
            Value::Number(_) => BYTE.read(v, at).map(ReplyToken::Byte),
            _ => Err(at.fault(FaultKind::BadValue, "a byte (0 to 255) or a placeholder")),
        }
    }
    fn write(&self, t: &ReplyToken) -> Option<Value> {
        Some(match t {
            ReplyToken::Byte(b) => Value::from(*b),
            ReplyToken::Placeholder(s) => Value::String(s.clone()),
        })
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
        "unpack" unpack: Option<Unpack> = Opt(closed()),
        /// The start of the reply, byte by byte; strings are placeholders.
        "reply" reply: Option<Vec<ReplyToken>> = Opt(list(ReplyTokenCodec)),
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
        /// How the bytes read as a version: `raw_bytes`, `yamaha_tenths`,
        /// `korg_minor_major`, `ascii`, `json`…
        "decode" decode: String = STRING,
        "at" at: Vec<i64> = list(INTEGER),
        /// With decode `json`: the key whose text is the version.
        "field" field: Option<String> = Opt(STRING),
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

/// A value an OSC message carries: a number (kept as written), a boolean or
/// a string.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Number(Number),
    Bool(bool),
    String(String),
}

pub(crate) struct ScalarCodec;

impl Codec for ScalarCodec {
    type T = Scalar;
    fn read(&self, v: &Value, at: &At) -> Result<Scalar, Fault> {
        match v {
            Value::Number(n) => Ok(Scalar::Number(n.clone())),
            Value::Bool(b) => Ok(Scalar::Bool(*b)),
            Value::String(s) => Ok(Scalar::String(s.clone())),
            _ => Err(at.fault(FaultKind::BadValue, "a number, true or false, or a string")),
        }
    }
    fn write(&self, t: &Scalar) -> Option<Value> {
        Some(match t {
            Scalar::Number(n) => Value::Number(n.clone()),
            Scalar::Bool(b) => Value::Bool(*b),
            Scalar::String(s) => Value::String(s.clone()),
        })
    }
}

object! {
    /// The OSC message sent to a software; `{name}` takes an argument.
    Forward {
        "path" path: String = STRING,
        "args" args: Option<Vec<Scalar>> = Opt(list(ScalarCodec)),
    }
}

object! {
    /// A value computed from the arguments before the message is built.
    Pre {
        "bind" bind: String = STRING,
        "expr" expr: String = STRING,
    }
}

object! {
    /// A named message: a SysEx utility, or an OSC message of a software.
    Command {
        "osc" osc: String = ADDRESS,
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
        "match_frame" match_frame: Option<Vec<FrameToken>> = Opt(list(FRAME_TOKEN)),
        "match_osc" match_osc: Option<String> = Opt(STRING),
        "match_args" match_args: Option<Vec<MatchArg>> = Opt(list(obj())),
        /// The address it is reported at, with `{placeholders}` from the message.
        "emit_osc" emit_osc: String = STRING,
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

#[cfg(test)]
mod tests {
    use super::placeholder;

    #[test]
    fn a_placeholder_is_braces_around_one_character_or_more() {
        assert!(placeholder("{value}") && placeholder("{}}") && placeholder("{x}"));
        assert!(!placeholder("{}") && !placeholder("value") && !placeholder("{a\nb}") && !placeholder("{value"));
    }
}
