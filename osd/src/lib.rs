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
mod device;
mod fingerprint;
mod firmware;
mod frame;
mod json;
mod resolve;
mod role;
mod role_vocabulary;
mod sound;
mod vocabulary;

pub use address::{Address, Segment};
pub use device::{
    Arg, ArgType, Bank, CcEntry, CcParams, ChannelMode, Command, CustomCommand, Device, DeviceIdRange, DeviceInfo,
    DeviceKind, Dump, DumpKind, FirmwareRule, Forward, Identity, IdentityTo, MatchArg, MidiIn, MidiOut, MidiTemplates, MidiType,
    NoteOff,
    Orientation, ParamEntry, Params, Pre, Profile, Programs, Reply, Scalar, ScriptProfile, Sends, Source,
    Subscription, SubscriptionOn, Sysex, Transport, TransportKind, Trigger, Triggers, Unpack, UsbId, ValueType,
};
pub use frame::{Algorithm, BinOp, Encoding, Expr, FramePart, FrameToken, IdentityPart};
pub use fingerprint::{Confirmed, Control, Fingerprint, MakerReply, Named, Ports, Via};
pub use firmware::{BasedOn, Community, CommunityKind, Firmware, FirmwareKind, FirmwareMidi, Install, Release};
pub use json::Remarks;
pub use resolve::{resolve, Files, Resolved};
pub use role::{role_from_param_path, roles, Role, RoleEntry, RoleMeaning, RolePart};
pub use role_vocabulary::{
    read_vocabulary, vocabulary_state, Controller, DocumentSource, MakerWord, SharedDevice, Term, TermKind, TermSource, Vocabulary,
    VOCABULARY_STATES,
};
pub use sound::{Attached, Author, Content, FileRef, ForDevice, Link, Origin, Preview, Sound, SoundKind};

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
}

/// A version of the format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Version {
    V0_4,
    V0_5,
    /// A draft: it may change until it is published.
    V0_6,
}

impl Version {
    /// The version as the format names it (`0.4`).
    pub fn name(self) -> &'static str {
        match self {
            Version::V0_4 => "0.4",
            Version::V0_5 => "0.5",
            Version::V0_6 => "0.6",
        }
    }
}

/// The versions this crate reads, the draft included.
pub const VERSIONS: &[Version] = &[Version::V0_4, Version::V0_5, Version::V0_6];

/// The latest published version: what [`convert`] writes and what the
/// `osd init` templates declare. A draft read after it is not published.
pub const PUBLISHED: Version = Version::V0_5;

/// Whether a descriptor's `$schema` names a version with the forms of 0.5
/// (structures, closed vocabularies): 0.5 and the drafts after it.
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
}

impl Descriptor {
    pub fn kind(&self) -> Kind {
        match self {
            Descriptor::Device(_) => Kind::Device,
            Descriptor::Profile(_) => Kind::Profile,
            Descriptor::Sound(_) => Kind::Sound,
            Descriptor::Fingerprint(_) => Kind::Fingerprint,
            Descriptor::Firmware(_) => Kind::Firmware,
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
    Ok(match kind {
        Kind::Device => {
            let mut d = Device::read(v, &at)?;
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
        Kind::Profile => Descriptor::Profile(Box::new(Profile::read(v, &at)?)),
        Kind::Sound => Descriptor::Sound(Box::new(Sound::read(v, &at)?)),
        Kind::Fingerprint => Descriptor::Fingerprint(Box::new(Fingerprint::read(v, &at)?)),
        Kind::Firmware => Descriptor::Firmware(Box::new(Firmware::read(v, &at)?)),
    })
}

/// The text of a descriptor, as JSON: what [`read`] returns, written back
/// key for key.
pub fn write(d: &Descriptor) -> String {
    let v = match d {
        Descriptor::Device(x) => x.write(),
        Descriptor::Profile(x) => x.write(),
        Descriptor::Sound(x) => x.write(),
        Descriptor::Fingerprint(x) => x.write(),
        Descriptor::Firmware(x) => x.write(),
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
            Version::V0_5 | Version::V0_6 => HOST_W3ID,
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
    fn schema(&self) -> Option<Version> {
        match self {
            Descriptor::Device(x) => x.schema,
            Descriptor::Profile(x) => x.schema,
            Descriptor::Sound(x) => x.schema,
            Descriptor::Fingerprint(x) => x.schema,
            Descriptor::Firmware(x) => x.schema,
        }
    }
}

/// The kind that the `$schema` of `text` declares: `None` when the text is
/// not JSON, has no `$schema`, or one that is not an accepted address of a
/// version read.
pub fn declared_kind(text: &str) -> Option<Kind> {
    let v: Value = serde_json::from_str(text).ok()?;
    let s = v.get("$schema")?.as_str()?;
    const KINDS: [(&str, Kind); 5] = [
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
    let d = read(text, kind)?;
    let from = d.schema().unwrap_or(Version::V0_4);
    let at = At::Key(&At::Root, "$schema");
    if target == Version::V0_4 || target < from {
        return Err(at.fault(
            FaultKind::BadValue,
            format!("a descriptor of {} is not converted to {}: conversion goes forward only", from.name(), target.name()),
        ));
    }
    if from == target || from == Version::V0_5 {
        let kept = convert_same(text, &d)?;
        if from == target {
            return Ok(kept);
        }
        // 0.5 to the draft: the address changes, the values stay.
        let Ok(Value::Object(mut v)) = serde_json::from_str::<Value>(&kept) else { unreachable!("a descriptor is a JSON object") };
        v.insert("$schema".into(), SchemaUrl(kind_name(kind)).write(&target).unwrap_or_default());
        let mut out = serde_json::to_string_pretty(&Value::Object(v)).unwrap_or_default();
        out.push('\n');
        return Ok(out);
    }
    let via_05 = convert_from_04(d, kind)?;
    if target == Version::V0_5 {
        return Ok(via_05);
    }
    convert_to(&via_05, kind, target)
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Device => "device",
        Kind::Profile => "profile",
        Kind::Sound => "sound",
        Kind::Fingerprint => "fingerprint",
        Kind::Firmware => "firmware",
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
