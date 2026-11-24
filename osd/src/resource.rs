//! What a device can receive from another (0.6): `resources`, a list in the words of the
//! ResourceList of MIDI 2.0 Property Exchange (`resource`, `canSet`, `mediaTypes`), each with the
//! `export` that produces its content and, for a device that does not speak Property Exchange,
//! the `via` it arrives by. `resource_path` resolves a file path from the device to place.

use crate::json::{closed, keys, field, list, list_of, matching, obj, object, put, words, At, Codec, Opt, STRING};
use crate::{Device, Fault, FaultKind, Version};
use serde_json::{Map, Value};

words! {
    /// Whether a resource can be set, with the words of Property Exchange.
    CanSet { None = "none", Partial = "partial", Full = "full", }
}

words! {
    /// What carries a file write.
    CarriedBy { Sysex = "sysex", }
}

words! {
    /// A part of the identity of the device placed that a path segment takes.
    PathVariable { Vendor = "vendor", Device = "device", }
}

object! {
    /// A resource a device can receive.
    Resource {
        /// Its name: a name of the list of Property Exchange, or a name of its own.
        "resource" resource: String = matching(|s| !s.is_empty(), "that is not empty"),
        "canSet" can_set: CanSet = closed(),
        "mediaTypes" media_types: Option<Vec<String>> = Opt(list(STRING)),
        /// The name of the format that produces its content; open, a name the reader does not know
        /// is read as it stands.
        "export" export: String = matching(|s| !s.is_empty(), "that is not empty"),
        /// Absent, Property Exchange sets the resource.
        "via" via: Option<ResourceVia> = Opt(obj()),
    }
}

object! {
    /// The way a resource arrives by.
    ResourceVia {
        "file_write" file_write: Option<FileWrite> = Opt(obj()),
    }
}

object! {
    /// A file written at a path.
    FileWrite {
        "path" path: Vec<PathSegment> = list_of(PathSegmentCodec, 1, usize::MAX),
        "carried_by" carried_by: Option<CarriedBy> = Opt(closed()),
        /// How the file is written by MIDI, step by step.
        "steps" steps: Option<Vec<crate::file_write::Step>> = Opt(crate::file_write::StepsCodec),
    }
}

/// A segment of a path: a fixed text, or a variable of the identity of the device placed.
#[derive(Clone, Debug, PartialEq)]
pub enum PathSegment {
    Text(String),
    /// The value of `variable`, then the fixed `suffix`.
    Variable { variable: PathVariable, suffix: Option<String> },
}

pub struct PathSegmentCodec;

impl Codec for PathSegmentCodec {
    type T = PathSegment;
    fn read(&self, v: &Value, at: &At) -> Result<PathSegment, Fault> {
        match v {
            Value::String(s) => Ok(PathSegment::Text(s.clone())),
            Value::Object(_) => {
                let o = keys(v, at, &["variable", "suffix"])?;
                if let Some(k) = o.keys().find(|k| k.starts_with('_')) {
                    return Err(At::Key(at, k).fault(FaultKind::UnknownField, format!("{k} is not a field here")));
                }
                Ok(PathSegment::Variable {
                    variable: field(o, "variable", at, closed())?,
                    suffix: field(o, "suffix", at, Opt(STRING))?,
                })
            }
            _ => Err(at.fault(FaultKind::BadValue, "a fixed text or an object with a variable")),
        }
    }
    fn write(&self, t: &PathSegment) -> Option<Value> {
        Some(match t {
            PathSegment::Text(s) => Value::String(s.clone()),
            PathSegment::Variable { variable, suffix } => {
                let mut o = Map::new();
                put(&mut o, "variable", closed(), variable);
                put(&mut o, "suffix", Opt(STRING), suffix);
                Value::Object(o)
            }
        })
    }
}

/// The resources of a descriptor before 0.6 are an unknown field.
pub(crate) fn check_version(d: &Device) -> Result<(), Fault> {
    if d.resources.is_some() && d.schema < Some(Version::V0_6) {
        return Err(Fault { kind: FaultKind::UnknownField, file: None, pointer: "/resources".into(), rule: "resources is not a field before 0.6".into() });
    }
    Ok(())
}

/// The path of the file that places `source` through the resource `resource` of `target`: its
/// segments, the variables filled from the `vendor` and the `name` of the identity of `source`,
/// joined by `/`. Refused (`BadValue`) when `target` has no such resource or no `file_write`
/// for it, and at the segment of the path whose value is empty or holds `/` or `\`.
pub fn resource_path(target: &Device, resource: &str, source: &Device) -> Result<String, Fault> {
    let (n, r) = target
        .resources
        .iter()
        .flatten()
        .enumerate()
        .find(|(_, r)| r.resource == resource)
        .ok_or_else(|| Fault { kind: FaultKind::BadValue, file: None, pointer: "/resources".into(), rule: format!("the device declares no resource {resource}") })?;
    let at = format!("/resources/{n}/via");
    let write = r
        .via
        .as_ref()
        .and_then(|v| v.file_write.as_ref())
        .ok_or_else(|| Fault { kind: FaultKind::BadValue, file: None, pointer: at.clone(), rule: format!("{resource} has no file_write to give a path") })?;
    let mut parts = Vec::new();
    for (i, segment) in write.path.iter().enumerate() {
        match segment {
            PathSegment::Text(s) => parts.push(s.clone()),
            PathSegment::Variable { variable, suffix } => {
                let (value, from) = match variable {
                    PathVariable::Vendor => (source.device.vendor.as_deref(), "/device/vendor"),
                    PathVariable::Device => (Some(source.device.name.as_str()), "/device/name"),
                };
                let pointer = format!("{at}/file_write/path/{i}");
                let value = value
                    .filter(|v| !v.is_empty() && !v.contains(['/', '\\']))
                    .ok_or_else(|| Fault {
                        kind: FaultKind::BadValue,
                        file: None,
                        pointer,
                        rule: format!("{from} of the device to place is absent, empty or holds / or \\, so it cannot be a part of a path"),
                    })?;
                parts.push(format!("{value}{}", suffix.as_deref().unwrap_or("")));
            }
        }
    }
    Ok(parts.join("/"))
}
