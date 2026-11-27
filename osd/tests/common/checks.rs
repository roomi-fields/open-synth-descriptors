//! The checks the catalogue tests play, each a function over a catalogue folder, so that the
//! tests prove them without the SyntHub catalogue, on small catalogues they write. Each file is
//! judged in the version it declares, 0.4 when it declares none.

use super::trial::first_difference;
use super::{Folder, Schemas};
use osd::{Fault, FaultKind, Kind, Version, VERSIONS};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The drivers, then the maker profiles, of `dir`.
fn files(dir: &Folder) -> impl Iterator<Item = (Kind, String)> {
    let drivers = dir.drivers().into_iter().map(|id| (Kind::Device, id));
    drivers.chain(dir.profiles().into_iter().map(|id| (Kind::Profile, id)))
}

/// What `check` says of each of these files, each failure named by its file.
fn each(dir: &Folder, mut check: impl FnMut(Kind, &str, &str) -> Option<String>) -> Vec<String> {
    files(dir).filter_map(|(kind, id)| check(kind, &id, &dir.text(&id)).map(|why| format!("{id}: {why}"))).collect()
}

fn json(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))
}

/// The published schemas of `version`, loaded once.
fn schemas(version: Version) -> &'static Schemas {
    static LOADED: [OnceLock<Schemas>; 4] = [const { OnceLock::new() }; 4];
    let at = VERSIONS.iter().position(|v| *v == version).unwrap();
    LOADED[at].get_or_init(|| Schemas::load(version.name()))
}

/// Loads the published schemas of every version, so that no judgement after it loads any.
pub fn load_schemas() {
    for version in VERSIONS {
        schemas(*version);
    }
}

/// The version a document of `kind` declares: 0.4 without `$schema`, else the one whose
/// published schema its `$schema` identifies; none for any other `$schema`.
pub fn declared_version(kind: Kind, doc: &Value) -> Option<Version> {
    let Some(declared) = doc.get("$schema") else { return Some(Version::V0_4) };
    VERSIONS.iter().copied().find(|v| declared.as_str() == Some(schemas(*v).id(kind)))
}

/// The judgement of a written document: valid against the schemas of the version it declares,
/// which is returned; else what is wrong.
pub fn judge_written(kind: Kind, doc: &Value) -> Result<Version, String> {
    let version = declared_version(kind, doc)
        .ok_or_else(|| format!("/$schema: no published schemas for {}", doc["$schema"]))?;
    match schemas(version).check(kind, doc) {
        Some(why) => Err(format!("against the schemas of {}: {why}", version.name())),
        None => Ok(version),
    }
}

/// A file of a catalogue as `osd::convert` gives it.
#[derive(Debug)]
pub struct Converted {
    pub id: String,
    pub kind: Kind,
    pub doc: Value,
}

/// Each driver, then each maker profile, of `dir` that `osd::convert` accepts, converted; a file
/// it refuses is left out.
pub fn converted_catalogue(dir: &Folder) -> Vec<Converted> {
    files(dir)
        .filter_map(|(kind, id)| {
            let out = osd::convert(&dir.text(&id), kind).ok()?;
            let doc = json(&out).unwrap_or_else(|why| panic!("{id}: the converted text is {why}"));
            Some(Converted { id, kind, doc })
        })
        .collect()
}

/// The converted documents that are not right, each failure named by its file: a right one is
/// valid against the schemas of the version it declares, which is the published one.
pub fn judge_converted(documents: &[Converted]) -> Vec<String> {
    documents
        .iter()
        .filter_map(|Converted { id, kind, doc }| {
            let why = match judge_written(*kind, doc) {
                Ok(version) if version == osd::PUBLISHED => return None,
                Ok(version) => {
                    format!("declares {}, not the published version {}", version.name(), osd::PUBLISHED.name())
                }
                Err(why) => why,
            };
            Some(format!("{id}: {why}"))
        })
        .collect()
}

/// The files the reader does not read.
pub fn read_strictly(dir: &Folder) -> Vec<String> {
    each(dir, |kind, _, text| osd::read(text, kind).err().map(|f| f.to_string()))
}

/// The files that a read and a write change.
pub fn written_back(dir: &Folder) -> Vec<String> {
    each(dir, |kind, _, text| {
        let original = match json(text) { Ok(v) => v, Err(why) => return Some(why) };
        let read = match osd::read(text, kind) { Ok(d) => d, Err(f) => return Some(f.to_string()) };
        let back = match json(&osd::write(&read)) { Ok(v) => v, Err(why) => return Some(why) };
        (back != original).then(|| {
            let (at, how) = first_difference(&original, &back, "");
            format!("{at}: {how}")
        })
    })
}

/// What the documentary fields of the drivers hold, counted in the types.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Documentary {
    pub sources: usize,
    pub coverage: usize,
    pub sections: usize,
    pub labels: usize,
    pub notes: usize,
}

/// The documentary fields of the drivers the reader reads.
pub fn documentary(dir: &Folder) -> Documentary {
    let mut out = Documentary::default();
    for id in dir.drivers() {
        let Ok(osd::Descriptor::Device(d)) = osd::read(&dir.text(&id), Kind::Device) else { continue };
        out.sources += d.sources.as_ref().map_or(0, Vec::len);
        out.coverage += usize::from(d.coverage.is_some());
        out.notes += d.remarks.len();
        let cc = d.cc_params.iter().flat_map(|t| &t.entries);
        out.sections += cc.filter(|e| e.section.is_some()).count();
        let params = d.params.iter().flat_map(|t| &t.entries);
        out.labels += params.filter_map(|e| e.values.as_ref()).map(|v| v.len()).sum::<usize>();
    }
    out
}

/// What a check of written documents found.
#[derive(Debug, Default)]
pub struct Judged {
    /// The version each file found right was judged in, by catalogue id.
    pub versions: BTreeMap<String, Version>,
    pub failures: Vec<String>,
}

impl Judged {
    fn note(&mut self, id: &str, judgement: Result<Version, String>) {
        match judgement {
            Ok(version) => {
                self.versions.insert(id.to_string(), version);
            }
            Err(why) => self.failures.push(format!("{id}: {why}")),
        }
    }
}

/// The written form of a descriptor, judged.
fn judge_descriptor(kind: Kind, d: &osd::Descriptor) -> Result<Version, String> {
    judge_written(kind, &json(&osd::write(d))?)
}

/// The model of each file: what the reader writes of it, judged.
pub fn models(dir: &Folder) -> Judged {
    let mut out = Judged::default();
    for (kind, id) in files(dir) {
        let read = osd::read(&dir.text(&id), kind).map_err(|f| f.to_string());
        out.note(&id, read.and_then(|d| judge_descriptor(kind, &d)));
    }
    out
}

/// Each driver resolved, from itself first, and the resolved document as written, judged.
pub fn resolutions(dir: &Folder) -> Judged {
    let mut out = Judged::default();
    for id in dir.drivers() {
        let judgement = osd::resolve(&id, dir).map_err(|f| f.to_string()).and_then(|r| {
            if r.files.first() != Some(&id) {
                return Err(format!("built from {:?}", r.files));
            }
            judge_descriptor(Kind::Device, &osd::Descriptor::Device(Box::new(r.device)))
        });
        out.note(&id, judgement);
    }
    out
}

/// A string frame element of 0.4 that names no algorithm or no function
/// (`{checksum}`, `{function}`): the converter cannot give it a structure.
/// With the mixed control, the closed list of expected faults: any other fault is a failure.
/// Both are removed by a SyntHub ticket:
/// - `{checksum}` without an algorithm: SH-cgi.7;
/// - `{function}`: SH-cgi.7.
fn is_expected_fault(s: &str) -> bool {
    s == "{checksum}" || s.starts_with("{function")
}

fn holds_expected_fault(v: &Value) -> bool {
    match v {
        Value::String(s) => is_expected_fault(s),
        Value::Array(a) => a.iter().any(holds_expected_fault),
        Value::Object(o) => o.values().any(holds_expected_fault),
        _ => false,
    }
}

/// A control whose flat fields mix a CC, an NRPN or an RPN: 0.4 and 0.5 say it, and no converter
/// to 0.6 chooses between them (`spec/0.6.md` 21.3).
fn mixes_messages(entry: &Value) -> bool {
    [["cc", "cc_lsb"], ["nrpn_msb", "nrpn_lsb"], ["rpn_msb", "rpn_lsb"]]
        .iter()
        .filter(|keys| keys.iter().any(|k| entry.get(k).is_some()))
        .count()
        > 1
}

fn holds_mixed_control(v: &Value) -> bool {
    v.pointer("/cc_params/entries").and_then(Value::as_array).is_some_and(|list| list.iter().any(mixes_messages))
}

/// Whether `pointer` is an entry of `cc_params` of `original` that mixes kinds of message.
pub fn at_mixed_control(original: &Value, pointer: &str) -> bool {
    let entry = pointer.strip_prefix("/cc_params/entries/").is_some_and(|n| !n.contains('/'));
    entry && original.pointer(pointer).is_some_and(mixes_messages)
}

/// Whether the conversion of `original` to `target` is expected to be refused: a string frame
/// element with no structure for every target, a mixed control from 0.6.
fn expects_refusal(original: &Value, target: Version) -> bool {
    holds_expected_fault(original) || (target >= Version::V0_6 && holds_mixed_control(original))
}

/// What is wrong with `result`, the conversion of `original` to `target`, if anything. A file
/// that holds an expected fault is refused as `NotConvertible` there; every other file converts.
fn conversion_fault(original: &Value, target: Version, result: &Result<String, Fault>) -> Option<String> {
    if !expects_refusal(original, target) {
        return result.as_ref().err().map(|f| format!("unexpected fault (any other fault goes up to the supervisor): {f}"));
    }
    let f = match result {
        Err(f) => f,
        Ok(_) => return Some("converted, but holds an expected fault".into()),
    };
    if f.kind != FaultKind::NotConvertible {
        return Some(format!("fault of kind {:?}: {f}", f.kind));
    }
    match original.pointer(&f.pointer) {
        Some(Value::String(s)) if is_expected_fault(s) => None,
        _ if target >= Version::V0_6 && at_mixed_control(original, &f.pointer) => None,
        other => Some(format!("fault at {}, which holds {other:?}", f.pointer)),
    }
}

/// What is wrong with the conversion of a catalogue file to the published version, if anything.
pub fn published_conversion_fault(text: &str, kind: Kind) -> Option<String> {
    let original = match json(text) { Ok(v) => v, Err(why) => return Some(why) };
    conversion_fault(&original, osd::PUBLISHED, &osd::convert(text, kind))
}

/// What a conversion of a catalogue found.
#[derive(Debug)]
pub struct Conversions {
    pub target: Version,
    /// The files converted as expected.
    pub converted: usize,
    /// The files refused for an expected fault.
    pub expected: usize,
    /// The files of a version newer than the target, which no conversion reaches.
    pub newer: usize,
    pub failures: Vec<String>,
}

impl std::fmt::Display for Conversions {
    /// The counts.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target = self.target.name();
        write!(
            f,
            "conversion to {target}: {} converted, {} refused for an expected fault, {} newer than {target}, not converted",
            self.converted, self.expected, self.newer
        )
    }
}

/// Each file a forward conversion to `target` reaches, converted by `convert`; `after` judges the
/// text of a conversion, given the file's own.
fn conversions(
    dir: &Folder,
    target: Version,
    convert: impl Fn(&str, Kind) -> Result<String, Fault>,
    after: impl Fn(Kind, &str, &str) -> Option<String>,
) -> Conversions {
    let mut out = Conversions { target, converted: 0, expected: 0, newer: 0, failures: Vec::new() };
    out.failures = each(dir, |kind, _, text| {
        let original = match json(text) { Ok(v) => v, Err(why) => return Some(why) };
        if declared_version(kind, &original).is_some_and(|v| v > target) {
            out.newer += 1;
            return None;
        }
        let result = convert(text, kind);
        let wrong = conversion_fault(&original, target, &result).or_else(|| result.as_deref().ok().and_then(|t| after(kind, text, t)));
        if wrong.is_none() {
            *(if result.is_ok() { &mut out.converted } else { &mut out.expected }) += 1;
        }
        wrong
    });
    out
}

/// Whether the reader writes `out`, a 0.5 text read as `read`, as it is: the same text when the
/// conversion wrote it, the same JSON when it gave back the file, which keeps its own layout.
fn written_unchanged(read: &osd::Descriptor, file: &str, out: &str) -> bool {
    let written = osd::write(read);
    if out == file { json(&written) == json(out) } else { written == out }
}

/// The conversion to 0.5: the text reads in 0.5, is written back unchanged, converts to itself,
/// and validates.
pub fn conversions_to_0_5(dir: &Folder) -> Conversions {
    let convert = |text: &str, kind| osd::convert_to(text, kind, Version::V0_5);
    conversions(dir, Version::V0_5, convert, |kind, file, out| {
        let read = match osd::read_as(out, kind, Version::V0_5) { Ok(d) => d, Err(f) => return Some(f.to_string()) };
        if !written_unchanged(&read, file, out) {
            return Some("the 0.5 text is written back differently".into());
        }
        if convert(out, kind).as_deref() != Ok(out) {
            return Some("converting the 0.5 text changes it".into());
        }
        json(out).and_then(|doc| judge_written(kind, &doc)).err()
    })
}

/// The conversion to the published version.
pub fn published_conversions(dir: &Folder) -> Conversions {
    conversions(dir, osd::PUBLISHED, osd::convert, |_, _, _| None)
}
