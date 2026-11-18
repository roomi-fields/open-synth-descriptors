//! What in a message is the sound (0.6): `sound_transfers[]`, per form a sound travels in
//! (alone, in a bank), the elements of its message (the data, the name, the transport parts)
//! and the identity of a sound, the SHA-256 of its data element alone.
//!
//! An element is found by positions (`{from, length}`, `F0` is 0) or by a Lua script. The
//! message of a transfer is as long as its `match_frame`, which holds one token per byte.

use crate::frame::{FramePart, FrameToken};
use crate::json::{at_least, closed, obj, object, words, At, Opt, STRING};
use crate::{Device, Fault, FaultKind};
use sha2::{Digest, Sha256};

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
    /// An element of a message: positions, or a Lua script.
    Element {
        /// The first byte in the message (`F0` is 0).
        "from" from: Option<i64> = Opt(at_least(0)),
        "length" length: Option<i64> = Opt(at_least(1)),
        /// A Lua script that gives the element from the message.
        "script" script: Option<String> = Opt(STRING),
        /// The packing of the data (the data element only).
        "packing" packing: Option<Packing> = Opt(closed()),
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
    }
    check check_transfer
}

fn fault(kind: FaultKind, pointer: String, rule: impl Into<String>) -> Fault {
    Fault { kind, file: None, pointer, rule: rule.into() }
}

fn check_element(e: &Element, at: &At) -> Result<(), Fault> {
    let bad = |rule: &str| fault(FaultKind::BadValue, at.pointer(), rule);
    match (e.from, e.length, &e.script) {
        (Some(_), Some(_), None) => Ok(()),
        (None, None, Some(_)) if e.packing.is_none() => Ok(()),
        (None, None, Some(_)) => Err(bad("a script gives the element itself: no packing")),
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

fn check_elements(e: &Elements, at: &At) -> Result<(), Fault> {
    for (name, el) in e.named().into_iter().skip(1) {
        if el.is_some_and(|el| el.packing.is_some()) {
            return Err(fault(
                FaultKind::UnknownField,
                format!("{}/{name}/packing", at.pointer()),
                "packing is a field of the data element only",
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
            let inside = d.0 <= nm.0 && nm.0 + nm.1 <= d.0 + d.1;
            if !inside {
                return Err(fault(FaultKind::BadValue, at_element("name"), "the name straddles the end of the data"));
            }
            if t.elements.data.packing == Some(Packing::MsbFirst7In8) {
                return Err(fault(FaultKind::BadValue, at_element("name"), "the name lies inside a packed data: it cannot be left out"));
            }
        }
    }
    if t.elements.data.packing == Some(Packing::MsbFirst7In8) && d_len(data) % 8 == 1 {
        return Err(fault(
            FaultKind::BadValue,
            format!("{}/length", at_element("data")),
            "a packed group of one byte holds no data",
        ));
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
        self.sound_transfers.iter().flatten().any(|t| t.elements.is_script())
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
/// header, checksum, bank and program stay out; a name inside the data is left out of it.
/// A message that the transfer's `match_frame` does not recognise, a transfer the device does
/// not have, an index past the bank, and a transfer whose identity needs Lua are refused.
pub fn sound_identity(device: &Device, transfer: &str, message: &[u8], index: usize) -> Result<String, Fault> {
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
    if let Some((nf, nl)) = t.elements.name.as_ref().and_then(span) {
        let (nf, nl) = (nf + shift, nl);
        if nf >= from && nf + nl <= from + length {
            data.drain(nf - from..nf - from + nl);
        }
    }
    if t.elements.data.packing == Some(Packing::MsbFirst7In8) {
        data = unpack_msb_first_7in8(&data);
    }
    Ok(Sha256::digest(&data).iter().map(|b| format!("{b:02x}")).collect())
}

fn unpack_msb_first_7in8(packed: &[u8]) -> Vec<u8> {
    packed
        .chunks(8)
        .flat_map(|group| group[1..].iter().enumerate().map(|(k, b)| (b & 0x7f) | ((group[0] >> k) & 1) << 7))
        .collect()
}

/// The sound transfers of a device before 0.6 are an unknown field.
pub(crate) fn check_version(d: &Device) -> Result<(), Fault> {
    if d.sound_transfers.is_some() && d.schema < Some(crate::Version::V0_6) {
        return Err(fault(FaultKind::UnknownField, "/sound_transfers".into(), "sound_transfers is not a field before 0.6"));
    }
    Ok(())
}
