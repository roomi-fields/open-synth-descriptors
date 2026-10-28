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

mod device;
mod fingerprint;
mod firmware;
mod json;
mod resolve;
mod sound;

pub use device::{
    Arg, ArgType, Bank, CcEntry, CcParams, ChannelMode, Command, CustomCommand, Device, DeviceIdRange, DeviceInfo,
    DeviceKind, Dump, DumpKind, FirmwareRule, Forward, FrameToken, Identity, MatchArg, MidiIn, MidiOut, NoteOff,
    Orientation, ParamEntry, Params, Pre, Profile, Programs, Reply, ReplyToken, Scalar, ScriptProfile, Sends, Source,
    Subscription, SubscriptionOn, Sysex, Transport, TransportKind, Trigger, Triggers, Unpack, UsbId, ValueType,
};
pub use fingerprint::{Confirmed, Control, Fingerprint, MakerReply, Named, Ports, Via};
pub use firmware::{BasedOn, Community, CommunityKind, Firmware, FirmwareKind, FirmwareMidi, Install, Release};
pub use json::Remarks;
pub use resolve::{resolve, Files, Resolved};
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
}

impl Version {
    /// The version as the format names it (`0.4`).
    pub fn name(self) -> &'static str {
        match self {
            Version::V0_4 => "0.4",
        }
    }
}

/// The versions this crate reads.
pub const VERSIONS: &[Version] = &[Version::V0_4];

/// What a driver requires from its reader beyond the declarative fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Lua expressions and scripts (the osc-bridge extension fields of 0.4:
    /// `transform`, `transform_reverse`, `script`, `custom_commands`).
    Lua,
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
        Kind::Device => Descriptor::Device(Box::new(Device::read(v, &at)?)),
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

const SCHEMA_HOST: &str = "https://synthhub.roomi-fields.com/osd/";

impl Codec for SchemaUrl {
    type T = Version;
    fn read(&self, v: &Value, at: &At) -> Result<Version, Fault> {
        let s = v.as_str().unwrap_or_default();
        let tail = format!("/{}.schema.json", self.0);
        let version = s.strip_prefix(SCHEMA_HOST).and_then(|r| r.strip_suffix(tail.as_str()));
        match version {
            Some(name) => VERSIONS.iter().copied().find(|x| x.name() == name).ok_or_else(|| {
                at.fault(FaultKind::UnreadVersion, format!("version {name} is not read (only {})", names()))
            }),
            None => Err(at.fault(FaultKind::BadValue, format!("the URL of an OSD {} schema", self.0))),
        }
    }
    fn write(&self, t: &Version) -> Option<Value> {
        Some(Value::String(format!("{SCHEMA_HOST}{}/{}.schema.json", t.name(), self.0)))
    }
}

fn names() -> String {
    VERSIONS.iter().map(|v| v.name()).collect::<Vec<_>>().join(", ")
}
