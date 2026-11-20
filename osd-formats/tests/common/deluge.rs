//! What the Deluge tests share: a reader that keeps what the Deluge firmware keeps of a device
//! definition (DelugeFirmware 680b93c, src/deluge/model/instrument/midi_instrument.cpp:605-625:
//! `readCCLabelsFromFile` takes each attribute of `ccLabels` whose name is a number from 0 to
//! 119, and keeps its value as it is), a definition the firmware repository holds, and the
//! devices and exports the tests build.

#![allow(dead_code)]

use super::xml;
use serde_json::{json, Value};

/// The first lines of contrib/sd_card/MIDI_DEVICES/DEFINITION/ASM/Hydrasynth.XML of
/// DelugeFirmware at 680b93c: the form the firmware reads.
pub const FIRMWARE_DEFINITION: &str = "<?xml version=\"\"?>\n<midiDevice>\n\t<ccLabels\n\t\t0=\"Select MSB\"\n\t\t1=\"ModWheel\"\n\t\t2=\"\"\n\t\t3=\"Noise-Vol\"\n\t\t7=\"Master Volume\"\n\t/>\n</midiDevice>\n";

/// What the firmware keeps of a definition: the labels by CC number (0 to 119), in the order
/// of the file. Panics when the text is not well formed or has no `midiDevice` root.
pub fn labels_read(text: &str) -> Vec<(u32, String)> {
    assert_well_formed(text);
    let root = xml::parse(text).unwrap().root;
    assert_eq!(root.name, "midiDevice", "{text}");
    let mut out = Vec::new();
    for labels in root.kids("ccLabels") {
        for (k, v) in &labels.attrs {
            if let Ok(n) = k.parse::<i64>() {
                if (0..120).contains(&n) {
                    out.push((n as u32, v.clone()));
                }
            }
        }
    }
    out
}

/// The text of each comment of a document.
pub fn comments(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("<!--") {
        let after = &rest[at + 4..];
        let end = after.find("-->").unwrap_or_else(|| panic!("an unterminated comment in {text}"));
        out.push(after[..end].to_string());
        rest = &after[end + 3..];
    }
    out
}

/// XML 1.0 (section 2.5): a comment holds no `--` and does not end with `-`; the document is
/// read whole by the reader of the tests.
pub fn assert_well_formed(text: &str) {
    if let Err(e) = xml::parse(text) {
        panic!("not well formed: {e}\n{text}");
    }
    for c in comments(text) {
        assert!(!c.contains("--"), "a comment holds `--`: {c:?}");
        assert!(!c.ends_with('-'), "a comment ends with `-`: {c:?}");
    }
}

/// A source with a credit and a licence.
pub fn source_json() -> Value {
    json!({"type": "community-re", "url": "https://example.org/deluge/device", "author": "A maker", "license": "CC-BY-4.0"})
}

/// The device the tests export: its blocks are the argument (the role vocabulary is declared).
pub fn device(extra: Value) -> osd::Resolved {
    let mut doc = json!({"role_vocabulary": "1"});
    for (k, v) in extra.as_object().expect("an object of blocks") {
        doc[k] = v.clone();
    }
    super::midnam::device_with(doc)
}

pub fn cc_device(entries: Value) -> osd::Resolved {
    device(json!({"cc_params": {"entries": entries}}))
}

/// What an export wrote: the document and the losses.
pub fn exported(d: &osd::Resolved) -> (String, Vec<osd_formats::Loss>) {
    let t = osd_formats::export(d, osd_formats::Format::Deluge).unwrap_or_else(|f| panic!("{f}"));
    (t.value, t.lost)
}

/// The places of the losses.
pub fn places(lost: &[osd_formats::Loss]) -> Vec<&str> {
    lost.iter().map(|l| l.place.as_str()).collect()
}
