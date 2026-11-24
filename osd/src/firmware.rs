//! The firmware descriptor of OSD 0.4 (`schemas/0.4/firmware.schema.json`): a
//! firmware, mod or script a device can run, and what it changes.

use crate::json::{closed, list, list_of, matching, obj, object, words, At, Bool, Codec, Object, Opt, Text, STRING};
use crate::sound::{Author, ForDevice};
use crate::{Fault, FaultKind, SchemaUrl, Version};
use serde_json::Value;
use std::cmp::Ordering;

words! {
    /// What a firmware descriptor describes.
    FirmwareKind { Firmware = "firmware", Mod = "mod", Module = "module", Application = "application", Script = "script", Extension = "extension", }
}

words! {
    /// Where a firmware's users meet.
    CommunityKind {
        Forum = "forum", Discord = "discord", Chat = "chat", Subreddit = "subreddit", Wiki = "wiki", Site = "site",
    }
}

object! {
    /// A firmware, mod or script a device can run: who makes it, where it
    /// lives, its licence, what it changes to the device's MIDI, how it is
    /// installed.
    Firmware {
        "$schema" schema: Option<Version> = Opt(SchemaUrl("firmware")),
        "name" name: String = STRING,
        "version" version: String = STRING,
        /// Firmware when absent.
        "kind" kind: Option<FirmwareKind> = Opt(closed()),
        /// The device it is made for (0.4 and 0.5; from 0.6 `targets`).
        "device" device: Option<ForDevice> = Opt(obj()),
        /// The devices it is made for, each with its base firmware versions (from 0.6).
        "targets" targets: Option<Vec<Target>> = Opt(list_of(obj(), 1, usize::MAX)),
        /// Released by the device's maker; false when absent.
        "official" official: Option<bool> = Opt(Bool),
        "author" author: Option<Vec<Author>> = Opt(list_of(obj(), 1, usize::MAX)),
        "repository" repository: Option<String> = Opt(STRING),
        "homepage" homepage: Option<String> = Opt(STRING),
        "release" release: Option<Release> = Opt(obj()),
        /// An SPDX licence id, `public-domain`, `NOASSERTION` or `NONE`.
        "license" license: Option<String> = Opt(STRING),
        "based_on" based_on: Option<BasedOn> = Opt(obj()),
        "midi" midi: Option<FirmwareMidi> = Opt(obj()),
        /// How the device shows which firmware it runs.
        "identify" identify: Option<String> = Opt(STRING),
        "install" install: Option<Install> = Opt(obj()),
        /// The tool that makes the assembly from the stock OS and modules (from 0.6).
        "builder" builder: Option<Builder> = Opt(obj()),
        "description" description: Option<String> = Opt(STRING),
        "communities" communities: Option<Vec<Community>> = Opt(list(obj())),
    }
    check licence_of_its_version
}

/// A 0.5 firmware holds a closed licence value; a 0.4 one any text. Up to 0.5 it names one
/// `device` and may name a `based_on` and a `midi`; from 0.6 it names `targets`, which carry the three.
fn licence_of_its_version(f: &Firmware, at: &At) -> Result<(), crate::Fault> {
    if crate::is_05_form(f.schema) {
        crate::vocabulary::check_license(f.license.as_deref(), "/license")?;
    }
    let unknown = |key| At::Key(at, key).fault(FaultKind::UnknownField, format!("{key} is not a field of a 0.6 firmware: it names targets"));
    if f.schema >= Some(Version::V0_6) {
        if f.device.is_some() {
            return Err(unknown("device"));
        }
        if f.based_on.is_some() {
            return Err(unknown("based_on"));
        }
        if f.midi.is_some() {
            return Err(unknown("midi"));
        }
        if f.targets.is_none() {
            return Err(At::Key(at, "targets").fault(FaultKind::Missing, "targets is required"));
        }
        let by_builder = f.install.as_ref().and_then(|i| i.method.as_deref()) == Some("builder");
        if by_builder && f.builder.is_none() {
            return Err(At::Key(at, "builder").fault(FaultKind::Missing, "install.method builder requires a builder"));
        }
    } else {
        if matches!(f.kind, Some(FirmwareKind::Module | FirmwareKind::Application)) {
            return Err(At::Key(at, "kind").fault(FaultKind::BadValue, "the kinds module and application exist from 0.6"));
        }
        if f.builder.is_some() {
            return Err(At::Key(at, "builder").fault(FaultKind::UnknownField, "builder is not a field before 0.6"));
        }
        if f.targets.is_some() {
            return Err(At::Key(at, "targets").fault(FaultKind::UnknownField, "targets is not a field before 0.6"));
        }
        if f.device.is_none() {
            return Err(At::Key(at, "device").fault(FaultKind::Missing, "device is required"));
        }
    }
    Ok(())
}

object! {
    /// A device a firmware is made for: its own MIDI, the firmware it is built from, and what it runs on.
    Target {
        "device" device: ForDevice = obj(),
        /// What the firmware understands over MIDI on this device.
        "midi" midi: Option<FirmwareMidi> = Opt(obj()),
        /// The lineage: the firmware it was forked or built from, one version.
        "based_on" based_on: Option<BasedOn> = Opt(obj()),
        /// The base firmware versions it runs on.
        "runs_on" runs_on: Option<RunsOn> = Opt(obj()),
    }
}

object! {
    /// The base firmware a target runs on, and the versions of it.
    RunsOn {
        "name" name: String = STRING,
        "versions" versions: Option<BaseVersions> = Opt(BaseVersionsCodec),
    }
}

/// The base versions a target accepts: an explicit list, or a range.
#[derive(Clone, Debug, PartialEq)]
pub enum BaseVersions {
    List(Vec<String>),
    Range(VersionRange),
}

object! {
    /// A range of versions, bounds included; a missing bound leaves that side open.
    VersionRange {
        "min" min: Option<String> = Opt(VERSION),
        "max" max: Option<String> = Opt(VERSION),
    }
    check range_has_a_bound
}

fn range_has_a_bound(r: &VersionRange, at: &At) -> Result<(), Fault> {
    if r.min.is_none() && r.max.is_none() {
        return Err(at.fault(FaultKind::BadValue, "a range holds a min, a max or both"));
    }
    Ok(())
}

/// A compared version: components separated by one dot, each a number then optional letters.
pub(crate) const VERSION: Text = matching(is_version, "of components separated by one dot, each a number then optional letters");

pub(crate) fn is_version(s: &str) -> bool {
    s.split('.').all(|c| {
        let letters = c.trim_start_matches(|d: char| d.is_ascii_digit());
        letters.len() < c.len() && letters.chars().all(|l| l.is_ascii_alphabetic())
    })
}

struct BaseVersionsCodec;

impl Codec for BaseVersionsCodec {
    type T = BaseVersions;
    fn read(&self, v: &Value, at: &At) -> Result<BaseVersions, Fault> {
        match v {
            Value::Object(_) => Ok(BaseVersions::Range(VersionRange::read(v, at)?)),
            _ => Ok(BaseVersions::List(list_of(VERSION, 1, usize::MAX).read(v, at)?)),
        }
    }
    fn write(&self, t: &BaseVersions) -> Option<Value> {
        match t {
            BaseVersions::List(l) => list_of(VERSION, 1, usize::MAX).write(l),
            BaseVersions::Range(r) => Some(r.write()),
        }
    }
}

/// How two base versions compare: the text splits on `.`; each component is a number then
/// optional letters; numbers compare as numbers, then the letters without case, no letters
/// before any letters; a missing component is 0 with no letters. So 1.40 < 1.40B < 1.40C < 1.41,
/// 1.9 < 1.10 and 1.40 = 1.40.0. A text outside that grammar (`0.3-beta`, `1..2`) has no order: the
/// answer is `None`, never an invented order.
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    if !is_version(a) || !is_version(b) {
        return None;
    }
    /// A component: its number without leading zeros, then its letters in lower case.
    fn component(s: &str) -> (&str, String) {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        (s[..end].trim_start_matches('0'), s[end..].to_ascii_lowercase())
    }
    let (a, b): (Vec<&str>, Vec<&str>) = (a.split('.').collect(), b.split('.').collect());
    for i in 0..a.len().max(b.len()) {
        let (xn, xl) = component(a.get(i).copied().unwrap_or("0"));
        let (yn, yl) = component(b.get(i).copied().unwrap_or("0"));
        // Numbers compare as numbers: by length, then digit by digit (no overflow).
        let order = xn.len().cmp(&yn.len()).then_with(|| xn.cmp(yn)).then_with(|| xl.cmp(&yl));
        if order != Ordering::Equal {
            return Some(order);
        }
    }
    Some(Ordering::Equal)
}

/// Whether `version` fits `target`: it equals one of the listed versions (by [`compare_versions`]),
/// or lies in the range, bounds included; a `min` alone is open upward, a `max` alone downward.
/// A target without `runs_on` versions answers [`Fit::Unknown`]: neither yes nor no; the caller decides.
pub fn version_in_target(version: &str, target: &Target) -> Fit {
    let yes = |b: bool| if b { Fit::Yes } else { Fit::No };
    if !is_version(version) {
        return Fit::Unknown;
    }
    match target.runs_on.as_ref().and_then(|b| b.versions.as_ref()) {
        Some(BaseVersions::List(l)) => yes(l.iter().any(|v| compare_versions(version, v) == Some(Ordering::Equal))),
        Some(BaseVersions::Range(r)) => yes(
            r.min.as_deref().is_none_or(|m| compare_versions(version, m) != Some(Ordering::Less))
                && r.max.as_deref().is_none_or(|m| compare_versions(version, m) != Some(Ordering::Greater)),
        ),
        None => Fit::Unknown,
    }
}

/// Whether a version fits a target: yes, no, or unknown when the target states no versions it runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Yes,
    No,
    Unknown,
}

object! {
    /// Where a version is downloaded, from its author.
    Release {
        "url" url: Option<String> = Opt(STRING),
        "date" date: Option<String> = Opt(STRING),
    }
}

object! {
    /// The firmware it was forked or built from (its lineage, one version).
    BasedOn {
        "name" name: String = STRING,
        "version" version: Option<String> = Opt(STRING),
        "url" url: Option<String> = Opt(STRING),
    }
}

object! {
    /// What a firmware understands over MIDI.
    FirmwareMidi {
        /// The OSD device descriptor of its MIDI.
        "descriptor" descriptor: Option<String> = Opt(STRING),
        /// When it answers to another firmware's MIDI unchanged, that descriptor.
        "same_as" same_as: Option<String> = Opt(STRING),
        /// What it adds or changes, in words.
        "changes" changes: Option<String> = Opt(STRING),
    }
}

object! {
    /// How a firmware is installed, and how to go back to the maker's.
    Install {
        "method" method: Option<String> = Opt(STRING),
        "steps" steps: Option<String> = Opt(STRING),
        "revert" revert: Option<String> = Opt(STRING),
    }
}

object! {
    /// The builder of a module (Elekloader, Modwerk): the tool that makes an assembly from the
    /// stock OS and modules, which the person then flashes. It stays the reference for building,
    /// installing and updating; OSD never rehosts.
    Builder {
        "name" name: String = STRING,
        "url" url: String = STRING,
    }
}

object! {
    /// Where a firmware's users meet.
    Community {
        "name" name: String = STRING,
        "url" url: String = STRING,
        "kind" kind: CommunityKind = closed(),
    }
}
