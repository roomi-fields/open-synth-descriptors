//! What in a message is the sound (0.6): `sound_transfers[]`, per form a sound travels in
//! (alone, in a bank), the elements of its message (the data, the name, the transport parts)
//! and the identity of a sound, the SHA-256 of its data element alone, a name inside the data
//! blanked.
//!
//! An element is found by positions (`{from, length}`, `F0` is 0) or by a Lua script; a name
//! inside the data is `{in_data: {from, length}}`, counted in the data unpacked from 0. The
//! message of a transfer is as long as its `match_frame`, which holds one token per byte.

use crate::frame::{FramePart, FrameToken};
use crate::json::{at_least, closed, obj, object, words, At, Opt, STRING};
use crate::{Device, Fault, FaultKind};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

words! {
    /// What a transfer carries.
    Carries { Sound = "sound", Bank = "bank", }
}

words! {
    /// How the data of a sound is packed in 7 bits.
    Packing {
        /// The bytes of the message are the data.
        None = "none",
        /// A byte of the high bits (bit k is the bit 7 of byte k of the group), then up to 7
        /// bytes with their bits 6 to 0; the last group may be partial. KORG minilogue xd MIDI
        /// Implementation, note 1, "7 bit data format conversion".
        MsbFirst7In8 = "msb_first_7in8",
    }
}

object! {
    /// Where a name lies in the data of the sound, unpacked, the first byte being 0.
    InData {
        "from" from: i64 = at_least(0),
        "length" length: i64 = at_least(1),
    }
}

object! {
    /// An element of a message: positions, or a Lua script; a name may instead lie in the data.
    Element {
        /// The first byte in the message (`F0` is 0).
        "from" from: Option<i64> = Opt(at_least(0)),
        "length" length: Option<i64> = Opt(at_least(1)),
        /// A Lua script that gives the element from the message.
        "script" script: Option<String> = Opt(STRING),
        /// The coding of the data in 7 bits (the data element only).
        "coding" coding: Option<Packing> = Opt(closed()),
        /// The name only: its place in the data, unpacked (never with positions or a script).
        "in_data" in_data: Option<InData> = Opt(obj()),
    }
    check check_element
}

object! {
    /// The elements of a message that carries a sound.
    Elements {
        "data" data: Element = obj(),
        "name" name: Option<Element> = Opt(obj()),
        "channel" channel: Option<Element> = Opt(obj()),
        "device_number" device_number: Option<Element> = Opt(obj()),
        "header" header: Option<Element> = Opt(obj()),
        "checksum" checksum: Option<Element> = Opt(obj()),
        "bank_number" bank_number: Option<Element> = Opt(obj()),
        "program_number" program_number: Option<Element> = Opt(obj()),
    }
    check check_elements
}

object! {
    /// A control and where its value lies in the data of the sound (unpacked, the name in place,
    /// the first byte being 0): whole bytes `{from, length}`, the first byte the highest, or a Lua
    /// script. The reader reads whole bytes only.
    SoundSetting {
        /// The JSON pointer of the control: `/cc_params/entries/N` or `/params/entries/N`.
        "control" control: String = STRING,
        /// The first byte of the place in the data.
        "from" from: Option<i64> = Opt(at_least(0)),
        /// The number of bytes of the place.
        "length" length: Option<i64> = Opt(at_least(1)),
        /// A Lua script that gives the value from the data.
        "script" script: Option<String> = Opt(STRING),
    }
    check check_setting
}

object! {
    /// A form a sound travels in.
    SoundTransfer {
        "id" id: String = STRING,
        /// The same `sound_id` in two forms declares that their data is the same.
        "sound_id" sound_id: Option<String> = Opt(STRING),
        "carries" carries: Carries = closed(),
        "match_frame" match_frame: Vec<FrameToken> = crate::json::list(crate::frame::MATCH_TOKEN),
        "elements" elements: Elements = obj(),
        /// A bank: the number of sounds.
        "count" count: Option<i64> = Opt(at_least(1)),
        /// A bank: the sound i has its elements shifted by i times `stride`.
        "stride" stride: Option<i64> = Opt(at_least(1)),
        /// Where the value of each control lies in the data of a sound of this form.
        "places" places: Option<Vec<SoundSetting>> = Opt(crate::json::list(obj())),
    }
    check check_transfer
}

fn check_setting(s: &SoundSetting, at: &At) -> Result<(), Fault> {
    let bad = |rule: &str| fault(FaultKind::BadValue, at.pointer(), rule);
    if s.script.is_some() {
        return if s.from.is_some() || s.length.is_some() {
            Err(bad("a place or a script, never both"))
        } else {
            Ok(())
        };
    }
    match (s.from, s.length) {
        (Some(_), Some(_)) => Ok(()),
        (Some(_), None) => Err(fault(FaultKind::Missing, format!("{}/length", at.pointer()), "length is required with from")),
        (None, Some(_)) => Err(fault(FaultKind::Missing, format!("{}/from", at.pointer()), "from is required with length")),
        (None, None) => Err(bad("a place {from, length} or a {script}")),
    }
}

/// The length of the data of a sound once unpacked (the name included), when the data is
/// found by positions.
fn data_length(t: &SoundTransfer) -> Option<usize> {
    let (_, length) = span(&t.elements.data)?;
    let mut n = length;
    if t.elements.data.coding == Some(Packing::MsbFirst7In8) {
        n -=n.div_ceil(8);
    }
    Some(n)
}

fn fault(kind: FaultKind, pointer: String, rule: impl Into<String>) -> Fault {
    Fault { kind, file: None, pointer, rule: rule.into() }
}

fn check_element(e: &Element, at: &At) -> Result<(), Fault> {
    let bad = |rule: &str| fault(FaultKind::BadValue, at.pointer(), rule);
    if e.in_data.is_some() {
        return if e.from.is_some() || e.length.is_some() || e.script.is_some() || e.coding.is_some() {
            Err(bad("in_data places the name in the data: no positions of the message, no script, no coding"))
        } else {
            Ok(())
        };
    }
    match (e.from, e.length, &e.script) {
        (Some(_), Some(_), None) => Ok(()),
        (None, None, Some(_)) if e.coding.is_none() => Ok(()),
        (None, None, Some(_)) => Err(bad("a script gives the element itself: no coding")),
        (_, _, Some(_)) => Err(bad("positions or a script, never both")),
        (Some(_), None, None) => Err(fault(FaultKind::Missing, format!("{}/length", at.pointer()), "length is required with from")),
        (None, Some(_), None) => Err(fault(FaultKind::Missing, format!("{}/from", at.pointer()), "from is required with length")),
        (None, None, None) => Err(bad("positions {from, length} or a {script}")),
    }
}

impl Elements {
    /// The elements by name, in the order the faults are looked for.
    fn named(&self) -> [(&'static str, Option<&Element>); 8] {
        [
            ("data", Some(&self.data)),
            ("name", self.name.as_ref()),
            ("channel", self.channel.as_ref()),
            ("device_number", self.device_number.as_ref()),
            ("header", self.header.as_ref()),
            ("checksum", self.checksum.as_ref()),
            ("bank_number", self.bank_number.as_ref()),
            ("program_number", self.program_number.as_ref()),
        ]
    }

    fn is_script(&self) -> bool {
        self.named().iter().any(|(_, e)| e.is_some_and(|e| e.script.is_some()))
    }
}

impl SoundTransfer {
    fn is_script(&self) -> bool {
        self.elements.is_script() || self.places.iter().flatten().any(|s| s.script.is_some())
    }

    /// The controls this transfer places twice or more at places that differ (`from`, `length` or
    /// `script`): the places contradict each other and none is taken as sure. Identical places do
    /// not, and the places of another transfer are other facts.
    fn contradicted_controls(&self) -> HashSet<&str> {
        let mut first = HashMap::new();
        let mut out = HashSet::new();
        for s in self.places.iter().flatten() {
            let place = (s.from, s.length, s.script.as_deref());
            if *first.entry(s.control.as_str()).or_insert(place) != place {
                out.insert(s.control.as_str());
            }
        }
        out
    }
}

/// The JSON pointer of the `control` of each place whose control its transfer places at places that
/// differ, in the order of the transfers, then of their places.
pub(crate) fn unconfirmed_places(device: &Device) -> Vec<String> {
    let mut out = Vec::new();
    for (n, t) in device.sound_transfers.iter().flatten().enumerate() {
        let contradicted = t.contradicted_controls();
        for (m, s) in t.places.iter().flatten().enumerate() {
            if contradicted.contains(s.control.as_str()) {
                out.push(format!("/sound_transfers/{n}/places/{m}/control"));
            }
        }
    }
    out
}

fn check_elements(e: &Elements, at: &At) -> Result<(), Fault> {
    for (name, el) in e.named().into_iter().skip(1) {
        if name != "name" && el.is_some_and(|el| el.in_data.is_some()) {
            return Err(fault(
                FaultKind::UnknownField,
                format!("{}/{name}/in_data", at.pointer()),
                "in_data is a field of the name only",
            ));
        }
        if el.is_some_and(|el| el.coding.is_some()) {
            return Err(fault(
                FaultKind::UnknownField,
                format!("{}/{name}/coding", at.pointer()),
                "coding is a field of the data element only",
            ));
        }
    }
    Ok(())
}

/// The bytes `[from, from + length)` of an element found by positions.
fn span(e: &Element) -> Option<(usize, usize)> {
    Some((usize::try_from(e.from?).ok()?, usize::try_from(e.length?).ok()?))
}

fn overlap(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 < b.0 + b.1 && b.0 < a.0 + a.1
}

fn check_transfer(t: &SoundTransfer, at: &At) -> Result<(), Fault> {
    let base = at.pointer();
    for (i, token) in t.match_frame.iter().enumerate() {
        if matches!(token, FrameToken::Part(FramePart::Arg { .. } | FramePart::Identity { index: None, .. })) {
            return Err(fault(
                FaultKind::BadValue,
                format!("{base}/match_frame/{i}"),
                "a message of a sound transfer is one token per byte: this token is not one byte",
            ));
        }
    }
    let (n, stride) = match (t.carries, t.count, t.stride) {
        (Carries::Sound, None, None) => (1, 0),
        (Carries::Sound, _, _) => {
            let key = if t.count.is_some() { "count" } else { "stride" };
            return Err(fault(FaultKind::BadValue, format!("{base}/{key}"), format!("{key} belongs to a bank")));
        }
        (Carries::Bank, Some(c), Some(s)) => (c as usize, s as usize),
        (Carries::Bank, None, _) => return Err(fault(FaultKind::Missing, format!("{base}/count"), "a bank states its count")),
        (Carries::Bank, _, None) => return Err(fault(FaultKind::Missing, format!("{base}/stride"), "a bank states its stride")),
    };
    let len = t.match_frame.len();
    let named = t.elements.named();
    let at_element = |name: &str| format!("{base}/elements/{name}");
    for (name, el) in &named {
        if let Some((from, length)) = el.and_then(span) {
            if from + (n - 1) * stride + length > len {
                return Err(fault(
                    FaultKind::BadValue,
                    at_element(name),
                    format!("the element reaches past the message of {len} bytes"),
                ));
            }
        }
    }
    let data = span(&t.elements.data);
    if let Some(d) = data {
        for (name, el) in named.iter().skip(2) {
            if el.and_then(span).is_some_and(|p| overlap(d, p)) {
                return Err(fault(FaultKind::BadValue, at_element("data"), format!("the data overlaps the {name}")));
            }
        }
    }
    for (i, (name, el)) in named.iter().enumerate().skip(2) {
        let Some(p) = el.and_then(span) else { continue };
        if named[2..i].iter().any(|(_, e)| e.and_then(span).is_some_and(|q| overlap(p, q))) {
            return Err(fault(FaultKind::BadValue, at_element(name), "the part overlaps another transport part"));
        }
    }
    if let (Some(d), Some(nm)) = (data, t.elements.name.as_ref().and_then(span)) {
        if overlap(d, nm) {
            return Err(fault(
                FaultKind::BadValue,
                at_element("name"),
                "the name overlaps the data: a name that lies in the data is given by in_data",
            ));
        }
    }
    if let Some(inner) = t.elements.name.as_ref().and_then(|n| n.in_data.as_ref()) {
        let Some(dlen) = data_length(t) else {
            return Err(fault(FaultKind::BadValue, at_element("name"), "in_data needs a data given by positions: a script may leave the name aside"));
        };
        if (inner.from + inner.length) as usize > dlen {
            return Err(fault(
                FaultKind::BadValue,
                at_element("name"),
                format!("the name reaches past the data of {dlen} byte(s) (unpacked)"),
            ));
        }
    }
    if t.elements.data.coding == Some(Packing::MsbFirst7In8) && d_len(data) % 8 == 1 {
        return Err(fault(
            FaultKind::BadValue,
            format!("{}/length", at_element("data")),
            "a packed group of one byte holds no data",
        ));
    }
    if let Some(dlen) = data_length(t) {
        for (m, s) in t.places.iter().flatten().enumerate() {
            if let (Some(f), Some(l)) = (s.from, s.length) {
                if (f + l) as usize > dlen {
                    return Err(fault(
                        FaultKind::BadValue,
                        format!("{base}/places/{m}"),
                        format!("the place reaches past the data of {dlen} byte(s) (unpacked, the name included)"),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn d_len(data: Option<(usize, usize)>) -> usize {
    data.map_or(0, |d| d.1)
}

/// For one transfer, whether the identity of its sounds is computed without Lua.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentitySupport {
    /// The JSON pointer of the transfer (`/sound_transfers/0`).
    pub pointer: String,
    /// `false` when the data is given by a script. A name given by a script has no position:
    /// it is taken as outside the data, and a device whose name lies inside a data that
    /// positions cannot cut gives the data by a script that leaves the name out.
    pub without_lua: bool,
}

impl SoundTransfer {
    fn without_lua(&self) -> bool {
        self.elements.data.script.is_none()
    }
}

impl Device {
    /// Whether any element of any sound transfer is a script.
    pub(crate) fn sound_transfers_use_lua(&self) -> bool {
        self.sound_transfers.iter().flatten().any(SoundTransfer::is_script)
    }
}

/// For each sound transfer of the device, whether the reader computes the identity of its
/// sounds without Lua; `sound_transfers` absent gives none.
pub fn sound_identity_support(device: &Device) -> Vec<IdentitySupport> {
    device
        .sound_transfers
        .iter()
        .flatten()
        .enumerate()
        .map(|(i, t)| IdentitySupport { pointer: format!("/sound_transfers/{i}"), without_lua: t.without_lua() })
        .collect()
}

/// The identity of the sound `index` of a message of the transfer `transfer`: the SHA-256 of
/// its data element alone, in 64 lowercase hexadecimal digits. The name, device number, channel,
/// header, checksum, bank and program stay out; a name inside the data (`in_data`) is blanked in it: its bytes, unpacked, set to zero.
/// A message that the transfer's `match_frame` does not recognise, a transfer the device does
/// not have, an index past the bank, and a transfer whose identity needs Lua are refused.
pub fn sound_identity(device: &Device, transfer: &str, message: &[u8], index: usize) -> Result<String, Fault> {
    let (t, mut data) = sound_data(device, transfer, message, index)?;
    if let Some(inner) = t.elements.name.as_ref().and_then(|n| n.in_data.as_ref()) {
        let (from, length) = (inner.from as usize, inner.length as usize);
        data[from..from + length].fill(0);
    }
    Ok(Sha256::digest(&data).iter().map(|b| format!("{b:02x}")).collect())
}

/// The transfer `transfer` and the data of the sound `index` of `message`: unpacked, the name in place.
fn sound_data<'d>(device: &'d Device, transfer: &str, message: &[u8], index: usize) -> Result<(&'d SoundTransfer, Vec<u8>), Fault> {
    let none = || fault(FaultKind::BadValue, String::new(), format!("the device has no sound transfer `{transfer}`"));
    let (i, t) = device.sound_transfers.iter().flatten().enumerate().find(|(_, t)| t.id == transfer).ok_or_else(none)?;
    let base = format!("/sound_transfers/{i}");
    if !t.without_lua() {
        return Err(fault(FaultKind::BadValue, base, "the identity of this transfer needs Lua, which the reader does not run"));
    }
    // A byte a transport part covers (channel, device number, header, checksum, bank, program)
    // varies from one message to the next: recognition leaves it out.
    let count = t.count.map_or(1, |c| c as usize);
    let stride = t.stride.map_or(0, |s| s as usize);
    let masked = |at: usize| {
        t.elements.named().iter().skip(2).filter_map(|(_, e)| e.and_then(span)).any(|(from, length)| {
            (0..count).any(|k| (from + k * stride..from + k * stride + length).contains(&at))
        })
    };
    let recognised = message.len() == t.match_frame.len()
        && t.match_frame.iter().zip(message).enumerate().all(|(at, (token, b))| {
            masked(at) || !matches!(token, FrameToken::Byte(x) if x != b)
        });
    if !recognised {
        return Err(fault(FaultKind::BadValue, format!("{base}/match_frame"), "the message is not the one this transfer recognises"));
    }
    if index >= count {
        return Err(fault(FaultKind::BadValue, base, format!("the transfer carries {count} sound(s): no index {index}")));
    }
    let shift = index * stride;
    let (from, length) = span(&t.elements.data).expect("a data without script has positions");
    let (from, length) = (from + shift, length);
    let mut data: Vec<u8> = message[from..from + length].to_vec();
    if t.elements.data.coding == Some(Packing::MsbFirst7In8) {
        data =unpack_msb_first_7in8(&data);
    }
    Ok((t, data))
}

/// A setting of a sound: a control found in its data, with its role and its raw value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setting {
    /// The JSON pointer of the control (`/cc_params/entries/0`).
    pub control: String,
    /// The role of the control as a `paramPath` (`/Filter/Cutoff`).
    pub param_path: String,
    /// The unsigned integer of the bytes of the place, read as they stand in the data.
    pub value: u64,
}

/// The settings of the sound `index` of a message of the transfer `transfer`, in the order of the
/// controls (`cc_params`, then `params`). A control with no role, no place, places of the
/// transfer that differ, a place by bits or 7-bit words, a script or a place the data does not
/// hold is absent: nothing is guessed. The refusals are those of `sound_identity`.
pub fn sound_settings(device: &Device, transfer: &str, message: &[u8], index: usize) -> Result<Vec<Setting>, Fault> {
    let (t, data) = sound_data(device, transfer, message, index)?;
    let mut out = Vec::new();
    let contradicted = t.contradicted_controls();
    for (pointer, path) in crate::role::control_param_paths(device) {
        if contradicted.contains(pointer.as_str()) {
            continue;
        }
        let Some(s) = t.places.iter().flatten().find(|s| s.control == pointer) else { continue };
        let (Some(from), Some(length), None) = (s.from, s.length, &s.script) else { continue };
        let (from, length) = (from as usize, length as usize);
        // A place of more than 8 bytes holds no 64-bit integer: the value is not read.
        let Some(bytes) = data.get(from..from + length).filter(|b| b.len() <= 8) else { continue };
        let value = bytes.iter().fold(0u64, |v, b| v << 8 | u64::from(*b));
        out.push(Setting { control: pointer, param_path: path, value });
    }
    Ok(out)
}

/// What two sounds say of a setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparison {
    Equal,
    Differ,
    /// One of the two sounds has no such setting: never equal.
    Absent,
}

/// A setting of two sounds side by side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingComparison {
    pub control: String,
    pub param_path: String,
    pub a: Option<u64>,
    pub b: Option<u64>,
    pub outcome: Comparison,
}

/// The place of a control in the order of the controls: `cc_params` before `params`, then its number.
fn control_rank(pointer: &str) -> (bool, usize) {
    let mut parts = pointer.split('/').skip(1);
    let params = parts.next() == Some("params");
    let n = parts.nth(1).and_then(|n| n.parse().ok()).unwrap_or(usize::MAX);
    (params, n)
}

/// Two lists of settings of one device side by side, in the order of the controls: each setting
/// that one of them has, `Equal` or `Differ` when both have it, `Absent` when one lacks it.
pub fn compare_settings(a: &[Setting], b: &[Setting]) -> Vec<SettingComparison> {
    let mut all: Vec<(&str, &str)> = a.iter().chain(b).map(|s| (s.control.as_str(), s.param_path.as_str())).collect();
    all.sort_by_key(|(c, _)| control_rank(c));
    all.dedup_by_key(|(c, _)| *c);
    all.into_iter()
        .map(|(control, path)| {
            let find = |l: &[Setting]| l.iter().find(|s| s.control == control).map(|s| s.value);
            let (va, vb) = (find(a), find(b));
            let outcome = match (va, vb) {
                (Some(x), Some(y)) if x == y => Comparison::Equal,
                (Some(_), Some(_)) => Comparison::Differ,
                _ => Comparison::Absent,
            };
            SettingComparison { control: control.to_string(), param_path: path.to_string(), a: va, b: vb, outcome }
        })
        .collect()
}

fn unpack_msb_first_7in8(packed: &[u8]) -> Vec<u8> {
    packed
        .chunks(8)
        .flat_map(|group| group[1..].iter().enumerate().map(|(k, b)| (b & 0x7f) | ((group[0] >> k) & 1) << 7))
        .collect()
}

/// The `control` of each setting of each sound transfer of a device text, judged before any
/// other field is read, so that it comes first among the faults of a setting: it matches the
/// pattern of the schema (`^/(cc_params|params)/entries/[0-9]+$`), then names an entry of the
/// description. `merged` says the document is the result of the merge of a resolution, whose
/// tables are complete: existence is judged, whether or not it keeps `midi_of`. The text of a
/// `midi_of` file read alone (`merged` false) takes its tables from its target: only the
/// pattern is judged. What is not shaped as a setting is left to the reader.
pub(crate) fn check_controls(v: &serde_json::Value, merged: bool) -> Result<(), Fault> {
    // Before 0.6 the field is unknown, which the reader says itself.
    if !v.get("$schema").and_then(|s| s.as_str()).is_some_and(|s| s.contains("/0.6/") || s.contains("/0.7/")) {
        return Ok(());
    }
    let Some(transfers) = v.get("sound_transfers").and_then(|t| t.as_array()) else { return Ok(()) };
    let delta = !merged && v.get("midi_of").is_some();
    let count = |table: &str| v.get(table).and_then(|t| t.get("entries")).and_then(|e| e.as_array()).map_or(0, Vec::len);
    for (n, t) in transfers.iter().enumerate() {
        let Some(settings) = t.get("places").and_then(|s| s.as_array()) else { continue };
        for (m, s) in settings.iter().enumerate() {
            let Some(control) = s.get("control").and_then(|c| c.as_str()) else { continue };
            let at = format!("/sound_transfers/{n}/places/{m}/control");
            let entry = control
                .strip_prefix("/cc_params/entries/")
                .map(|i| ("cc_params", i))
                .or_else(|| control.strip_prefix("/params/entries/").map(|i| ("params", i)))
                .filter(|(_, i)| !i.is_empty() && i.bytes().all(|b| b.is_ascii_digit()));
            let Some((table, i)) = entry else {
                return Err(fault(FaultKind::BadValue, at, "a pointer `/cc_params/entries/N` or `/params/entries/N`"));
            };
            if !delta && i.parse::<usize>().map_or(true, |i| i >= count(table)) {
                return Err(fault(FaultKind::BadValue, at, format!("the description has no entry {control}")));
            }
        }
    }
    Ok(())
}

/// The sound transfers of a device before 0.6 are an unknown field.
pub(crate) fn check_version(d: &Device) -> Result<(), Fault> {
    if d.sound_transfers.is_some() && d.schema < Some(crate::Version::V0_6) {
        return Err(fault(FaultKind::UnknownField, "/sound_transfers".into(), "sound_transfers is not a field before 0.6"));
    }
    Ok(())
}
