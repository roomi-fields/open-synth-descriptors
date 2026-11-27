//! `osd`, the reference reader of Open Synth Descriptors: a descriptor read
//! strictly into the types of its version, and a device resolved with its
//! maker profiles and shared MIDI. It performs no I/O: the caller hands it
//! the text of each file.
//!
//! ```
//! let text = r#"{"device": {"name": "Synth", "osc_prefix": "/synth"}}"#;
//! let osd::Descriptor::Device(d) = osd::read(text, osd::Kind::Device).unwrap() else { unreachable!() };
//! assert_eq!(d.device.osc_prefix, "/synth");
//! ```

mod address;
mod convert;
mod device;
mod flat;
mod file_write;
mod fingerprint;
mod firmware;
mod frame;
mod json;
mod matching;
mod providers;
mod resolve;
mod resource;
mod assembly;
mod role;
mod role_vocabulary;
mod sound;
mod sound_transfer;
mod vocabulary;
mod warnings;

pub use address::{Address, Segment};
pub use device::{
    Arg, ArgType, Bank, CcEntry, CcParams, ChannelMode, Command, CustomCommand, Device, DeviceIdRange, DeviceInfo,
    DeviceKind, Dump, DumpKind, EndpointRule, Fidelity, FirmwareRule, Forward, Identity, IdentityTo, MatchArg, MidiIn, MidiOut,
    MidiTemplates, MidiType, NamedValue, NamedValues, NoteOff, Direction,
    ParamEntry, Params, Platform, Polarity, PortDirection, PortOperator, PortRule, Pre, Stack, Profile, Programs, Reading, ReadingState, ControlReading, Reply, Scalar, ScriptProfile, Sends, Source,
    Subscription, SubscriptionOn, Sysex, Transport, TransportKind, Trigger, Triggers, Unpack, UsbId, ValueType, Way,
};
pub use frame::{Algorithm, BinOp, Encoding, Expr, FramePart, FrameToken, IdentityPart};
pub use fingerprint::{Confirmed, Control, Fingerprint, MakerReply, Named, Ports, Program, Via};
pub use firmware::{
    compare_versions, version_in_target, BaseVersions, BasedOn, Builder, Community, CommunityKind, Firmware, FirmwareKind, FirmwareMidi, Fit,
    Install, Release, RunsOn, Target, VersionRange,
};
pub use assembly::{judge_needs, resolve_assembly, Assembly, NeedJudgement, PinnedFirmware, PinnedModule, Verdict};
pub use json::Remarks;
pub use matching::{match_identity, same_product_instance_id, MatchVerdict, RuleMatch};
pub use providers::PROVIDERS_VERSION;
pub use warnings::{warnings, Warning, WarningKind};
pub use resolve::{complete_descriptor, resolve, Files, Resolved};
pub use file_write::{encode as encode_chunk, Coding,FileItem, NumberAs, ReplyToken, SendToken, Step, StepKind};
pub use resource::{resource_path, CanSet, CarriedBy, FileWrite, PathSegment, PathVariable, Resource, ResourceVia};
pub use role::{role_from_param_path, roles, Role, RoleEntry, RoleMeaning, RolePart};
pub use role_vocabulary::{
    read_vocabulary, vocabulary_state, Controller, DocumentSource, MakerWord, SharedDevice, Term, TermKind, TermSource, Vocabulary,
    VOCABULARY_STATES,
};
pub use sound_transfer::{
    compare_settings, sound_identity, sound_identity_support, sound_settings, Carries, Comparison, Element, Elements, IdentitySupport,
    Packing, Setting, SettingComparison, SoundSetting, SoundTransfer,
};
pub use sound::{Attached, Author, Content, ExternalId, FileRef, ForDevice, Link, Origin, Preview, Sound, SoundKind};

use json::{At, Codec, Object};
use serde_json::Value;

/// A kind of descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Device,
    Profile,
    Sound,
    Fingerprint,
    Firmware,
    /// A stock firmware and the modules applied to it (from 0.6).
    Assembly,
}

/// A version of the format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Version {
    V0_4,
    V0_5,
    V0_6,
    /// A candidate build, not published.
    V0_7,
}

impl Version {
    /// The version as the format names it (`0.4`).
    pub fn name(self) -> &'static str {
        match self {
            Version::V0_4 => "0.4",
            Version::V0_5 => "0.5",
            Version::V0_6 => "0.6",
            Version::V0_7 => "0.7",
        }
    }
}

/// The versions this crate reads.
pub const VERSIONS: &[Version] = &[Version::V0_4, Version::V0_5, Version::V0_6, Version::V0_7];

/// The latest published version: what [`convert`] writes and what the
/// `osd init` templates declare.
pub const PUBLISHED: Version = Version::V0_6;

/// Whether a descriptor's `$schema` names a version with the forms of 0.5
/// (structures, closed vocabularies): 0.5 and the versions after it.
pub(crate) fn is_05_form(v: Option<Version>) -> bool {
    v.is_some_and(|v| v >= Version::V0_5)
}

/// What a driver requires from its reader beyond the declarative fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Lua expressions and scripts (the osc-bridge extension fields of 0.4:
    /// `transform`, `transform_reverse`, `script`, `custom_commands`, and four
    /// of 0.5).
    Lua,
}

impl Capability {
    /// The word the format writes for it (`lua`).
    pub fn word(self) -> &'static str {
        <Self as json::Word>::word(self)
    }
}

impl json::Word for Capability {
    const WORDS: &'static [(&'static str, Self)] = &[("lua", Capability::Lua)];
}

/// A descriptor of any kind, typed.
#[derive(Clone, Debug, PartialEq)]
pub enum Descriptor {
    Device(Box<Device>),
    Profile(Box<Profile>),
    Sound(Box<Sound>),
    Fingerprint(Box<Fingerprint>),
    Firmware(Box<Firmware>),
    Assembly(Box<Assembly>),
}

impl Descriptor {
    pub fn kind(&self) -> Kind {
        match self {
            Descriptor::Device(_) => Kind::Device,
            Descriptor::Profile(_) => Kind::Profile,
            Descriptor::Sound(_) => Kind::Sound,
            Descriptor::Fingerprint(_) => Kind::Fingerprint,
            Descriptor::Firmware(_) => Kind::Firmware,
            Descriptor::Assembly(_) => Kind::Assembly,
        }
    }
}

/// A refusal: the rule broken, at a JSON pointer, in a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub kind: FaultKind,
    /// The catalogue id of the file, when the fault was met resolving; a
    /// file read alone with [`read`] has none.
    pub file: Option<String>,
    /// The JSON pointer (RFC 6901) of the value refused; empty for the
    /// whole file.
    pub pointer: String,
    /// The rule, in words.
    pub rule: String,
}

/// The kinds of refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaultKind {
    /// The text is not JSON.
    Syntax,
    /// A field the version does not define.
    UnknownField,
    /// A value outside its type or its closed list.
    BadValue,
    /// A required field is absent.
    Missing,
    /// A version the crate does not read.
    UnreadVersion,
    /// A shared-MIDI or profile link names no file.
    LinkMissing,
    /// The links form a cycle.
    LinkCycle,
    /// The links go deeper than 8.
    LinkTooDeep,
    /// A field the target version cannot hold.
    NotConvertible,
}

impl Fault {
    /// The fault, in `file` when it names none yet.
    pub(crate) fn in_file(mut self, file: &str) -> Self {
        self.file.get_or_insert_with(|| file.to_string());
        self
    }
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{file}: ")?;
        }
        let pointer = if self.pointer.is_empty() { "(the file)" } else { &self.pointer };
        write!(f, "{pointer}: {:?}: {}", self.kind, self.rule)
    }
}

impl std::error::Error for Fault {}

/// The descriptor of kind `kind` that `text` holds, every field checked
/// against its version's schema.
pub fn read(text: &str, kind: Kind) -> Result<Descriptor, Fault> {
    let v: Value = serde_json::from_str(text).map_err(|e| At::Root.fault(FaultKind::Syntax, e.to_string()))?;
    read_value(&v, kind)
}

pub(crate) fn read_value(v: &Value, kind: Kind) -> Result<Descriptor, Fault> {
    let at = At::Root;
    // A device and a profile judge the keys of their credit in their own reading.
    if !matches!(kind, Kind::Device | Kind::Profile) {
        check_credit_keys(v, kind, &at)?;
    }
    Ok(match kind {
        Kind::Device => {
            sound_transfer::check_controls(v, false)?;
            let mut d = device::read_device(v, &at)?;
            if is_05_form(d.schema) {
                device::check_capabilities(&d)?;
            }
            if d.schema >= Some(Version::V0_6) {
                // From 0.6 `capabilities` is a set: a repeated entry is refused.
                let list = d.capabilities.as_deref().unwrap_or_default();
                if list.iter().enumerate().any(|(i, c)| list[..i].contains(c)) {
                    return Err(At::Key(&at, "capabilities").fault(FaultKind::BadValue, "`capabilities` repeats an entry"));
                }
            }
            // `capabilities` reads as a set: a repeated entry is dropped, the
            // order of first occurrence kept.
            if let Some(c) = &mut d.capabilities {
                let mut seen = Vec::new();
                c.retain(|x| !seen.contains(x) && {
                    seen.push(*x);
                    true
                });
            }
            Descriptor::Device(Box::new(d))
        }
        Kind::Profile => Descriptor::Profile(Box::new(device::read_profile(v, &at)?)),
        Kind::Sound => Descriptor::Sound(Box::new(Sound::read(v, &at)?)),
        Kind::Fingerprint => Descriptor::Fingerprint(Box::new(fingerprint::read_fingerprint(v, &at)?)),
        Kind::Firmware => Descriptor::Firmware(Box::new(Firmware::read(v, &at)?)),
        Kind::Assembly => Descriptor::Assembly(Box::new(Assembly::read(v, &at)?)),
    })
}

/// The keys of the credit of a sound, a fingerprint, a firmware or an assembly, and of the assembly
/// a sound was made on, by the version its `$schema` names; a `$schema` that names none is the
/// reading's to refuse.
fn check_credit_keys(v: &Value, kind: Kind, at: &At) -> Result<(), Fault> {
    let Some(o) = v.as_object() else { return Ok(()) };
    let Ok(schema) = json::field(o, "$schema", at, json::Opt(SchemaUrl(kind_name(kind)))) else { return Ok(()) };
    device::check_credit_keys(v, at, schema)?;
    match o.get("made_on").filter(|_| kind == Kind::Sound) {
        Some(made_on) => device::check_credit_keys(made_on, &At::Key(at, "made_on"), schema),
        None => Ok(()),
    }
}

/// The text of a descriptor, as JSON: what [`read`] returns, written back
/// key for key.
pub fn write(d: &Descriptor) -> String {
    let v = match d {
        Descriptor::Device(x) => device::write_device(x),
        Descriptor::Profile(x) => device::write_profile(x),
        Descriptor::Sound(x) => x.write(),
        Descriptor::Fingerprint(x) => x.write(),
        Descriptor::Firmware(x) => x.write(),
        Descriptor::Assembly(x) => x.write(),
    };
    let mut out = serde_json::to_string_pretty(&v).unwrap_or_default();
    out.push('\n');
    out
}

/// The `$schema` of a descriptor kind that declares its version: the URL of
/// that version's schema of the kind.
pub(crate) struct SchemaUrl(pub &'static str);

/// Where the schemas of 0.4 are served.
const HOST_0_4: &str = "https://synthhub.roomi-fields.com/osd/";
/// Where the schemas of 0.5 are served (w3id).
const HOST_W3ID: &str = "https://w3id.org/osd/";

impl Version {
    /// The one table of `$schema` identifiers: the host that identifies
    /// each version read (0.4 its published `$id`, 0.5 the permanent address).
    fn host(self) -> &'static str {
        match self {
            Version::V0_4 => HOST_0_4,
            Version::V0_5 | Version::V0_6 | Version::V0_7 => HOST_W3ID,
        }
    }

    /// The `$schema` identifier of this version for the kind `kind`.
    fn address(self, kind: &str) -> String {
        format!("{}{}/{}.schema.json", self.host(), self.name(), kind)
    }
}

impl Codec for SchemaUrl {
    type T = Version;
    fn read(&self, v: &Value, at: &At) -> Result<Version, Fault> {
        let s = v.as_str().unwrap_or_default();
        let tail = format!("/{}.schema.json", self.0);
        let found = [HOST_0_4, HOST_W3ID].iter().find_map(|host| {
            let name = s.strip_prefix(host)?.strip_suffix(tail.as_str())?;
            Some(name)
        });
        match found {
            Some(name) => match VERSIONS.iter().copied().find(|x| x.name() == name) {
                Some(version) if version.address(self.0) == s => Ok(version),
                Some(version) => Err(at.fault(
                    FaultKind::UnreadVersion,
                    format!(
                        "`{s}` is not an address of {name}; accepted: `{}`",
                        version.address(self.0)
                    ),
                )),
                None => Err(at.fault(
                    FaultKind::UnreadVersion,
                    format!("`{s}` names a version that is not read; versions read: {}", names()),
                )),
            },
            None => Err(at.fault(FaultKind::BadValue, format!("the URL of an OSD {} schema", self.0))),
        }
    }
    fn write(&self, t: &Version) -> Option<Value> {
        Some(Value::String(t.address(self.0)))
    }
}

/// The `$schema` of a kind whose 0.4 schema defines no such field: only the
/// URL of version 0.5 is read; one of 0.4 is an unknown field.
pub(crate) struct SchemaUrlFrom05(pub &'static str);

impl Codec for SchemaUrlFrom05 {
    type T = Version;
    fn read(&self, v: &Value, at: &At) -> Result<Version, Fault> {
        match SchemaUrl(self.0).read(v, at)? {
            Version::V0_4 => Err(at.fault(FaultKind::UnknownField, "$schema is not a field of 0.4 here".to_string())),
            x => Ok(x),
        }
    }
    fn write(&self, t: &Version) -> Option<Value> {
        SchemaUrl(self.0).write(t)
    }
}

fn names() -> String {
    VERSIONS.iter().map(|v| v.name()).collect::<Vec<_>>().join(", ")
}

impl Descriptor {
    /// The version its `$schema` names, when it carries one.
    pub fn schema(&self) -> Option<Version> {
        match self {
            Descriptor::Device(x) => x.schema,
            Descriptor::Profile(x) => x.schema,
            Descriptor::Sound(x) => x.schema,
            Descriptor::Fingerprint(x) => x.schema,
            Descriptor::Firmware(x) => x.schema,
            Descriptor::Assembly(x) => x.schema,
        }
    }
}

/// The kind that the `$schema` of `text` declares: `None` when the text is
/// not JSON, has no `$schema`, or one that is not an accepted address of a
/// version read.
pub fn declared_kind(text: &str) -> Option<Kind> {
    let v: Value = serde_json::from_str(text).ok()?;
    let s = v.get("$schema")?.as_str()?;
    const KINDS: [(&str, Kind); 6] = [
        ("assembly", Kind::Assembly),
        ("device", Kind::Device),
        ("profile", Kind::Profile),
        ("sound", Kind::Sound),
        ("fingerprint", Kind::Fingerprint),
        ("firmware", Kind::Firmware),
    ];
    VERSIONS
        .iter()
        .find_map(|ver| KINDS.iter().find(|(name, _)| ver.address(name) == s).map(|(_, k)| *k))
}

/// The descriptor of kind `kind` that `text` holds, read in `version`: a
/// 0.5 descriptor must carry its `$schema`; one that names another version
/// is refused.
pub fn read_as(text: &str, kind: Kind, version: Version) -> Result<Descriptor, Fault> {
    let d = read(text, kind)?;
    let at = At::Key(&At::Root, "$schema");
    match (d.schema(), version) {
        (Some(s), v) if s == v => Ok(d),
        (None, Version::V0_4) => Ok(d),
        (None, v) => Err(at.fault(FaultKind::Missing, format!("a {} descriptor carries `$schema`", v.name()))),
        (Some(s), v) => Err(at.fault(
            FaultKind::BadValue,
            format!("`$schema` names version {}, read as {}", s.name(), v.name()),
        )),
    }
}

/// The text of a descriptor in the latest published version ([`PUBLISHED`]):
/// [`convert_to`] with that target.
pub fn convert(text: &str, kind: Kind) -> Result<String, Fault> {
    convert_to(text, kind, PUBLISHED)
}

/// The text of a descriptor in `target`: its `$schema` (the first key) names
/// the target, every other value is kept, a text that changes version is laid
/// out as [`write`] does. Conversion goes forward only: a text already in
/// the target comes back as it is, except that a repeated `capabilities`
/// entry is written once; a target older than the text, or 0.4, is refused
/// at `/$schema`.
pub fn convert_to(text: &str, kind: Kind, target: Version) -> Result<String, Fault> {
    let d = read(text, kind).map_err(|f| if target >= Version::V0_7 { convert::orientation_not_convertible(f) } else { f })?;
    let from = d.schema().unwrap_or(Version::V0_4);
    if let (Descriptor::Fingerprint(f), true) = (&d, target >= Version::V0_6 && from < Version::V0_6) {
        fingerprint::check_subject(f, &At::Root)?;
    }
    let at = At::Key(&At::Root, "$schema");
    if target == Version::V0_4 || target < from {
        return Err(at.fault(
            FaultKind::BadValue,
            format!("a descriptor of {} is not converted to {}: conversion goes forward only", from.name(), target.name()),
        ));
    }
    if from == target {
        return convert_same(text, &d);
    }
    convert::carry(text, kind, from, target)
}

/// The flat fields that give each kind of message of a control: a CC, an NRPN, an RPN.
const MESSAGE_FIELDS: [[&str; 2]; 3] = [["cc", "cc_lsb"], ["nrpn_msb", "nrpn_lsb"], ["rpn_msb", "rpn_lsb"]];

/// Whether the flat fields of a control name more than one kind of message.
fn mixes(control: &serde_json::Map<String, Value>) -> bool {
    MESSAGE_FIELDS.iter().filter(|keys| keys.iter().any(|k| control.contains_key(*k))).count() > 1
}

/// The three writings 0.6 tightens, from a 0.5 text: a SysEx text in lower case is written in upper
/// case (the same bytes); a control whose flat fields mix a CC, an NRPN or an RPN, and a sound
/// content with none of `sysex`, `midi`, `settings`, are refused as `NotConvertible` at the control
/// and at the content: the converter does not choose which to keep and invents no content.
fn tighten_writings(v: &mut serde_json::Map<String, Value>) -> Result<(), Fault> {
    let refuse = |pointer: String, rule: &str| Fault { kind: FaultKind::NotConvertible, file: None, pointer, rule: rule.into() };
    const RULE: &str = "the flat fields of a control mix a CC, an NRPN or an RPN";
    for table in ["cc_params", "params"] {
        let Some(Value::Array(entries)) = v.get(table).and_then(|c| c.get("entries")) else { continue };
        for (n, entry) in entries.iter().enumerate().filter_map(|(n, e)| e.as_object().map(|e| (n, e))) {
            if mixes(entry) {
                return Err(refuse(format!("/{table}/entries/{n}"), RULE));
            }
        }
    }
    // A fingerprint's controls, and the control of each name, are controls too.
    if let Some(Value::Array(controls)) = v.get("controls") {
        for (n, control) in controls.iter().enumerate().filter_map(|(n, e)| e.as_object().map(|e| (n, e))) {
            if mixes(control) {
                return Err(refuse(format!("/controls/{n}"), RULE));
            }
        }
    }
    if let Some(Value::Array(names)) = v.get("names") {
        for (n, control) in names.iter().enumerate().filter_map(|(n, e)| e.get("control").and_then(Value::as_object).map(|c| (n, c))) {
            if mixes(control) {
                return Err(refuse(format!("/names/{n}/control"), RULE));
            }
        }
    }
    if let Some(Value::Object(content)) = v.get_mut("content") {
        if ["sysex", "midi", "settings"].iter().all(|k| !content.contains_key(*k)) {
            return Err(refuse("/content".into(), "a content holds at least one of sysex, midi, settings"));
        }
        if let Some(Value::String(text)) = content.get_mut("sysex") {
            *text = text.to_ascii_uppercase();
        }
    }
    Ok(())
}

/// The value of an NRPN or an RPN of 0.5 is the 14 bits of the data MSB and LSB, the one coding
/// 0.5 knew: a control of `cc_params` that gives an NRPN or an RPN gets `value_bits: 14`, which 0.6
/// requires.
fn full_value_width(v: &mut serde_json::Map<String, Value>) {
    let Some(Value::Array(entries)) = v.get_mut("cc_params").and_then(|c| c.get_mut("entries")) else { return };
    for entry in entries.iter_mut().filter_map(Value::as_object_mut) {
        if ["nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb"].iter().any(|k| entry.contains_key(*k)) {
            entry.insert("value_bits".into(), Value::from(14));
        }
    }
}

/// A 0.5 firmware's `device`, `midi` and `based_on` as the one `targets` entry of 0.6, each as it
/// is: the lineage stays a lineage and no `runs_on` is written.
fn firmware_targets_of(v: &mut serde_json::Map<String, Value>) {
    let mut target = serde_json::Map::new();
    if let Some(device) = v.remove("device") {
        target.insert("device".into(), device);
    }
    if let Some(midi) = v.remove("midi") {
        target.insert("midi".into(), midi);
    }
    if let Some(base) = v.remove("based_on") {
        target.insert("based_on".into(), base);
    }
    v.insert("targets".into(), Value::Array(vec![Value::Object(target)]));
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Device => "device",
        Kind::Profile => "profile",
        Kind::Sound => "sound",
        Kind::Fingerprint => "fingerprint",
        Kind::Firmware => "firmware",
        Kind::Assembly => "assembly",
    }
}

/// A text already in the form of its version: as it is, except that a
/// repeated `capabilities` entry is written once.
fn convert_same(text: &str, d: &Descriptor) -> Result<String, Fault> {
    if let Descriptor::Device(x) = d {
        let mut v: Value = serde_json::from_str(text).map_err(|e| At::Root.fault(FaultKind::Syntax, e.to_string()))?;
        let kept = x.capabilities.as_ref().map(Vec::len);
        let given = v.get("capabilities").and_then(Value::as_array).map(Vec::len);
        if kept != given {
            let list = x.capabilities.iter().flatten().map(|c| Value::String(c.word().into())).collect();
            v["capabilities"] = Value::Array(list);
            let mut out = serde_json::to_string_pretty(&v).unwrap_or_default();
            out.push('\n');
            return Ok(out);
        }
    }
    Ok(text.to_string())
}

/// The 0.5 text of a 0.4 descriptor: structures for the strings, closed
/// licences, `$schema` first.
fn convert_from_04(d: Descriptor, kind: Kind) -> Result<String, Fault> {
    let d = match d {
        Descriptor::Device(x) => Descriptor::Device(Box::new(x.into_structures()?)),
        Descriptor::Profile(x) => Descriptor::Profile(Box::new(x.into_structures()?)),
        Descriptor::Sound(mut x) => {
            if let Some(l) = &x.license {
                x.license = Some(vocabulary::convert_license(l, "/license")?);
            }
            Descriptor::Sound(x)
        }
        Descriptor::Firmware(mut x) => {
            if let Some(l) = &x.license {
                x.license = Some(vocabulary::convert_license(l, "/license")?);
            }
            Descriptor::Firmware(x)
        }
        other => other,
    };
    let Ok(Value::Object(written)) = serde_json::from_str::<Value>(&write(&d)) else {
        unreachable!("write gives a JSON object")
    };
    let mut out = serde_json::Map::new();
    out.insert("$schema".into(), SchemaUrl(kind_name(kind)).write(&Version::V0_5).unwrap_or_default());
    out.extend(written.into_iter().filter(|(k, _)| k != "$schema"));
    let mut text = serde_json::to_string_pretty(&Value::Object(out)).unwrap_or_default();
    text.push('\n');
    Ok(text)
}
