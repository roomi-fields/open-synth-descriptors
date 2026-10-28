//! The fingerprint of OSD 0.4 (`schemas/0.4/fingerprint.schema.json`): what a
//! detection observed of one device plugged in, and the device the person
//! confirmed it is.

use crate::device::{Sends, UsbId};
use crate::json::{at_most, closed, int, list, list_of, matching, obj, object, words, At, Nullable, Opt, STRING};
use crate::{Fault, FaultKind, SchemaUrl, Version};

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
        /// Controls the person named while teaching it.
        "names" names: Option<Vec<Named>> = Opt(list_of(obj(), 0, 32)),
    }
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
    /// A control that arrived, without its value.
    Control {
        "cc" cc: Option<u8> = Opt(int(0, 127)),
        "nrpn_msb" nrpn_msb: Option<u8> = Opt(int(0, 127)),
        "nrpn_lsb" nrpn_lsb: Option<u8> = Opt(int(0, 127)),
        "channel" channel: Option<u8> = Opt(int(0, 15)),
    }
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
