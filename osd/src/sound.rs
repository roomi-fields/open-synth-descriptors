//! The sound descriptor of OSD 0.4 (`schemas/0.4/sound.schema.json`): what a
//! shared sound is and who made it, beside its data.

use crate::json::{at_least, at_most, closed, list, matching, obj, object, words, Any, At, Codec, Dict, Opt, Num, Text, STRING};
use crate::{Fault, FaultKind, SchemaUrl, Version};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

words! {
    /// What a sound descriptor holds.
    SoundKind {
        Sound = "sound", Kit = "kit", Sequence = "sequence", Project = "project", Bank = "bank", Clip = "clip",
        Performance = "performance", Scene = "scene", Setlist = "setlist",
    }
}

object! {
    /// A shared sound, kit, pattern or sequence, project, bank of the
    /// author's sounds, or MIDI clip.
    Sound {
        "$schema" schema: Option<Version> = Opt(SchemaUrl("sound")),
        "name" name: String = at_most(80),
        "kind" kind: SoundKind = closed(),
        /// The device it is made for; required unless it is a clip.
        "device" device: Option<ForDevice> = Opt(obj()),
        /// The firmware it was made on, when it matters.
        "firmware" firmware: Option<String> = Opt(STRING),
        "content" content: Content = obj(),
        "author" author: Option<Author> = Opt(obj()),
        /// An SPDX licence id, `public-domain`, `NOASSERTION` or `NONE`.
        "license" license: Option<String> = Opt(STRING),
        /// The author's own terms, when they are not a known licence.
        "license_terms" license_terms: Option<String> = Opt(STRING),
        "origin" origin: Option<Origin> = Opt(obj()),
        /// The sound it is a version of.
        "derived_from" derived_from: Option<Link> = Opt(obj()),
        "description" description: Option<String> = Opt(at_most(500)),
        "category" category: Option<String> = Opt(STRING),
        "genres" genres: Option<Vec<String>> = Opt(list(STRING)),
        "tags" tags: Option<Vec<String>> = Opt(list(STRING)),
        /// Beats per minute, for a sequence or clip.
        "tempo" tempo: Option<Number> = Opt(Num { above: Some(0.0) }),
        "key" key: Option<String> = Opt(STRING),
        /// How to play it: mod wheel, aftertouch, register.
        "tips" tips: Option<String> = Opt(at_most(500)),
        "preview" preview: Option<Preview> = Opt(obj()),
        /// The tool that took it from the device, and its version.
        "captured_with" captured_with: Option<String> = Opt(STRING),
        "version" version: Option<i64> = Opt(at_least(1)),
        "published" published: Option<String> = Opt(STRING),
        "updated" updated: Option<String> = Opt(STRING),
        "links" links: Option<Vec<Link>> = Opt(list(obj())),
    }
    check device_unless_clip
}

fn device_unless_clip(s: &Sound, at: &At) -> Result<(), Fault> {
    if s.schema == Some(Version::V0_5) {
        crate::vocabulary::check_license(s.license.as_deref(), "/license")?;
    }
    let newer = matches!(s.kind, SoundKind::Performance | SoundKind::Scene | SoundKind::Setlist);
    if newer && s.schema != Some(Version::V0_5) {
        return Err(At::Key(at, "kind").fault(FaultKind::BadValue, "this kind is not a sound kind of 0.4"));
    }
    if s.kind != SoundKind::Clip && s.device.is_none() {
        return Err(At::Key(at, "device").fault(FaultKind::Missing, "device is required unless the kind is clip"));
    }
    Ok(())
}

object! {
    /// The device a sound or a firmware is made for.
    ForDevice {
        /// The OSD device descriptor it is for (`maker/file`).
        "descriptor" descriptor: Option<String> = Opt(STRING),
        "vendor" vendor: String = STRING,
        "model" model: String = STRING,
    }
}

object! {
    /// The sound itself: its SysEx, a MIDI file, or settings by control address.
    Content {
        /// The device's own data, as it sends and takes it.
        "sysex" sysex: Option<Attached> = Opt(Attachment { text: matching(hex, "of hex digits and spaces") }),
        /// A standard MIDI file.
        "midi" midi: Option<Attached> = Opt(Attachment { text: STRING }),
        /// Values by control address of the device descriptor
        /// (`/filter/cutoff: [80]`).
        "settings" settings: Option<BTreeMap<String, Vec<Value>>> = Opt(Dict(list(Any))),
    }
}

/// `^[0-9A-Fa-f ]+$`
fn hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit() || b == b' ')
}

/// `^[0-9a-f]{64}$`
fn sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

object! {
    /// A file, beside the descriptor or online.
    FileRef {
        /// Relative to this descriptor.
        "path" path: Option<String> = Opt(STRING),
        "url" url: Option<String> = Opt(STRING),
        "sha256" sha256: Option<String> = Opt(matching(sha256, "of 64 lower-case hex digits")),
    }
}

/// Data given as a file, or written in the descriptor itself.
#[derive(Clone, Debug, PartialEq)]
pub enum Attached {
    File(FileRef),
    Inline(String),
}

/// An attachment whose inline form is the string `text` reads.
pub(crate) struct Attachment {
    text: Text,
}

impl Codec for Attachment {
    type T = Attached;
    fn read(&self, v: &Value, at: &At) -> Result<Attached, Fault> {
        match v {
            Value::Object(_) => obj::<FileRef>().read(v, at).map(Attached::File),
            Value::String(_) => self.text.read(v, at).map(Attached::Inline),
            _ => Err(at.fault(FaultKind::BadValue, "a file (an object) or a string")),
        }
    }
    fn write(&self, t: &Attached) -> Option<Value> {
        match t {
            Attached::File(f) => obj::<FileRef>().write(f),
            Attached::Inline(s) => Some(Value::String(s.clone())),
        }
    }
}

object! {
    /// A person or group, as they sign.
    Author {
        "name" name: String = STRING,
        "url" url: Option<String> = Opt(STRING),
    }
}

object! {
    /// Where a sound was first published, when not here.
    Origin {
        "source" source: Option<String> = Opt(STRING),
        "url" url: String = STRING,
        "author" author: Option<String> = Opt(STRING),
        "author_url" author_url: Option<String> = Opt(STRING),
    }
}

object! {
    /// A named link.
    Link {
        "name" name: Option<String> = Opt(STRING),
        "url" url: String = STRING,
    }
}

object! {
    /// What a sound sounds and looks like: an audio file, an image.
    Preview {
        "audio" audio: Option<Attached> = Opt(Attachment { text: STRING }),
        "image" image: Option<Attached> = Opt(Attachment { text: STRING }),
    }
}
