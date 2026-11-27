//! The fingerprint of OSD 0.4 (`schemas/0.4/fingerprint.schema.json`): what a
//! detection observed of one device plugged in, and the device the person
//! confirmed it is.

use crate::device::{check_credit, check_stack, refuse_paths, Platform, Sends, Stack, UsbId};
use crate::firmware::FirmwareKind;
use crate::json::{at_most, closed, field, int, list, list_of, matching, obj, object, words, At, Nullable, Object, Opt, STRING};
use crate::{Fault, FaultKind, SchemaUrl, Source, Version};
use serde_json::Value;

words! {
    /// Where a detection ran: a web browser (no USB identity) or a program
    /// on the machine.
    Via { WebMidi = "web-midi", Native = "native", }
}

object! {
    /// What a detection observed of one device, and what the person
    /// confirmed it is.
    Fingerprint {
        "$schema" schema: Option<Version> = Opt(SchemaUrl("fingerprint")),
        /// The day, no time.
        "observed" observed: String = STRING,
        "via" via: Option<Via> = Opt(closed()),
        /// The platform the observation was taken on; unknown when absent (0.7).
        "platform" platform: Option<Platform> = Opt(closed()),
        /// The naming stack, when the tool knows it (0.7).
        "stack" stack: Option<Stack> = Opt(closed()),
        /// The name the device gives its own MIDI 2.0 endpoint, lower case (0.7).
        "endpoint_name" endpoint_name: Option<String> = Opt(STRING),
        /// The MIDI 2.0 product instance id of the unit observed, as it gives it (0.7).
        "product_instance_id" product_instance_id: Option<String> = Opt(STRING),
        "ports" ports: Option<Ports> = Opt(obj()),
        "usb" usb: Option<UsbId> = Opt(obj()),
        /// Its answer to the Universal Identity Request, hex.
        "identity_reply" identity_reply: Option<String> = Opt(matching(identity_reply, "of upper-case hex bytes from F0 to F7")),
        /// Read-only maker requests sent and what came back.
        "maker_replies" maker_replies: Option<Vec<MakerReply>> = Opt(list(obj())),
        "sends" sends: Option<Sends> = Opt(obj()),
        /// The controls that arrived while the person turned knobs, in order.
        "controls" controls: Option<Vec<Control>> = Opt(list_of(obj(), 0, 32)),
        "device" device: Option<Confirmed> = Opt(obj()),
        /// A firmware or application the fingerprint was taken on, in the words of its card (from 0.6).
        "program" program: Option<Program> = Opt(obj()),
        /// Controls the person named while teaching it.
        "names" names: Option<Vec<Named>> = Opt(list_of(obj(), 0, 32)),
        /// Where the data comes from, with its credit, licence and the firmware it documents (0.7).
        "sources" sources: Option<Vec<Source>> = Opt(list(obj())),
    }
    check check_controls
}

/// The keys of a fingerprint defined only from 0.7.
const FROM_0_7: &[&[&str]] = &[&["platform"], &["stack"], &["endpoint_name"], &["product_instance_id"]];

/// The fingerprint of `v`: before 0.7 the system and the signs of MIDI 2.0 are unknown fields.
pub(crate) fn read_fingerprint(v: &Value, at: &At) -> Result<Fingerprint, Fault> {
    if let Some(o) = v.as_object() {
        if field(o, "$schema", at, Opt(SchemaUrl("fingerprint"))).is_ok_and(|s| s < Some(Version::V0_7)) {
            refuse_paths(v, at, FROM_0_7, "is not a field before 0.7")?;
        }
    }
    Fingerprint::read(v, at)
}

/// `^F0( [0-9A-F]{2})* F7$`
fn identity_reply(s: &str) -> bool {
    let Some(mid) = s.strip_prefix("F0").and_then(|s| s.strip_suffix(" F7")) else { return false };
    mid.len() % 3 == 0
        && mid.as_bytes().chunks(3).all(|c| c[0] == b' ' && c[1..].iter().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(b)))
}

object! {
    /// The device's MIDI port names, lower case, words only.
    Ports {
        "inputs" inputs: Option<Vec<String>> = Opt(list(STRING)),
        "outputs" outputs: Option<Vec<String>> = Opt(list(STRING)),
    }
}

object! {
    /// A maker request sent, and its answer.
    MakerReply {
        "request" request: String = STRING,
        /// `Some(None)`, written `null`: no answer.
        "reply" reply: Option<Option<String>> = Opt(Nullable(STRING)),
    }
}

object! {
    /// A control that arrived, without its value: a CC (with `cc_lsb` for 14 bits), an NRPN or an
    /// RPN, in the words of a device's control, one message kind only (0.6).
    Control {
        "cc" cc: Option<u8> = Opt(int(0, 127)),
        "cc_lsb" cc_lsb: Option<u8> = Opt(int(0, 127)),
        "nrpn_msb" nrpn_msb: Option<u8> = Opt(int(0, 127)),
        "nrpn_lsb" nrpn_lsb: Option<u8> = Opt(int(0, 127)),
        "rpn_msb" rpn_msb: Option<u8> = Opt(int(0, 127)),
        "rpn_lsb" rpn_lsb: Option<u8> = Opt(int(0, 127)),
        "channel" channel: Option<u8> = Opt(int(0, 15)),
    }
}

/// The controls of a fingerprint from 0.6: `cc_lsb`, `rpn_msb` and `rpn_lsb` are fields of 0.6 (before
/// it, `UnknownField`), and a control carries one message kind only.
fn check_controls(f: &Fingerprint, at: &At) -> Result<(), Fault> {
    check_stack(f.platform, f.stack, at)?;
    check_credit(f.schema, &[(at.pointer(), &f.sources)])?;
    let v06 = f.schema >= Some(Version::V0_6);
    if v06 {
        check_subject(f, at)?;
    } else if f.program.is_some() {
        return Err(At::Key(at, "program").fault(FaultKind::UnknownField, "program is not a field before 0.6"));
    }
    let controls = f.controls.iter().flatten().enumerate().map(|(i, c)| (format!("/controls/{i}"), c));
    let names = f.names.iter().flatten().enumerate().map(|(i, n)| (format!("/names/{i}/control"), &n.control));
    for (pointer, c) in controls.chain(names) {
        if !v06 {
            for (key, given) in [("cc_lsb", c.cc_lsb), ("rpn_msb", c.rpn_msb), ("rpn_lsb", c.rpn_lsb)] {
                if given.is_some() {
                    return Err(Fault { kind: FaultKind::UnknownField, file: None, pointer: format!("{pointer}/{key}"), rule: format!("{key} is not a field before 0.6") });
                }
            }
            continue;
        }
        let kinds = [c.cc.is_some() || c.cc_lsb.is_some(), c.nrpn_msb.is_some() || c.nrpn_lsb.is_some(), c.rpn_msb.is_some() || c.rpn_lsb.is_some()];
        if kinds.iter().filter(|k| **k).count() > 1 {
            return Err(Fault { kind: FaultKind::BadValue, file: None, pointer, rule: "a control carries one message kind only: a cc, an nrpn or an rpn".into() });
        }
    }
    Ok(())
}

object! {
    /// What the person confirmed a device is: a descriptor, or its maker and
    /// model as they name it.
    Confirmed {
        "descriptor" descriptor: Option<String> = Opt(STRING),
        "firmware" firmware: Option<String> = Opt(STRING),
        "vendor" vendor: Option<String> = Opt(at_most(60)),
        "model" model: Option<String> = Opt(at_most(60)),
    }
    check descriptor_or_model
}

fn descriptor_or_model(c: &Confirmed, at: &At) -> Result<(), Fault> {
    if c.descriptor.is_none() && (c.vendor.is_none() || c.model.is_none()) {
        return Err(At::Key(at, "descriptor").fault(FaultKind::Missing, "descriptor, or vendor and model, is required"));
    }
    Ok(())
}

/// A 0.6 fingerprint names what it was taken on: exactly one of a device (with its firmware) and a program.
pub(crate) fn check_subject(f: &Fingerprint, at: &At) -> Result<(), Fault> {
    match (&f.device, &f.program) {
        (None, None) => Err(At::Key(at, "device").fault(FaultKind::Missing, "a fingerprint names a device or a program")),
        (Some(_), Some(_)) => Err(At::Key(at, "program").fault(FaultKind::BadValue, "a fingerprint names a device or a program, not both")),
        (Some(d), None) if d.firmware.is_none() => {
            Err(At::Key(&At::Key(at, "device"), "firmware").fault(FaultKind::Missing, "the firmware of the device is required"))
        }
        _ => Ok(()),
    }
}

object! {
    /// A firmware or an application the fingerprint was taken on, as its card names it.
    Program {
        "name" name: String = STRING,
        "version" version: String = STRING,
        /// Firmware when absent.
        "kind" kind: Option<FirmwareKind> = Opt(closed()),
    }
}

object! {
    /// A control the person named.
    Named {
        "control" control: Control = obj(),
        "name" name: String = at_most(60),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_identity_reply_is_hex_bytes_from_f0_to_f7() {
        assert!(super::identity_reply("F0 F7") && super::identity_reply("F0 7E 00 F7"));
        assert!(!super::identity_reply("F0 7e F7") && !super::identity_reply("F0 7E0 F7") && !super::identity_reply("F07E F7"));
    }
}
