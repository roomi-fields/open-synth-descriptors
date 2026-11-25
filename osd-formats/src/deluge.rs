//! The Deluge device definition: an XML `midiDevice` with a `ccLabels` tag, one attribute per
//! CC from 0 to 119 (DelugeFirmware, src/deluge/model/instrument/midi_instrument.cpp,
//! `readCCLabelsFromFile`). The Deluge reads a definition and writes none back: there is no
//! importer. The fate of each tag and attribute is in docs/deluge.md.

use crate::credit::source_text;
use crate::xml::escape;
use crate::{Contract, Fault, FaultKind, Loss, Translated};
use osd::{Descriptor, Device, Resolved, Source};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) const CONTRACT: Contract = Contract {
    name: "deluge",
    media_type: "application/xml",
    extension: "xml",
    import,
    export,
    import_into: None,
};

/// The CC numbers the firmware reads (kNumRealCCNumbers, 120 of them).
const CC_COUNT: i64 = 120;

fn loss(losses: &mut Vec<Loss>, place: impl Into<String>, reason: impl Into<String>) {
    losses.push(Loss { place: place.into(), reason: reason.into() });
}

fn segment(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// A text safe inside an XML comment: no `--`, and a space after a final `-`.
fn comment_safe(text: &str) -> String {
    let mut out = text.to_string();
    while out.contains("--") {
        out = out.replace("--", "- -");
    }
    if out.ends_with('-') {
        out.push(' ');
    }
    out
}

/// This format is exported only: every file is refused.
fn import(_: &str, _: &Source) -> Result<Translated<Device>, Fault> {
    Err(Fault {
        kind: FaultKind::InvalidFile,
        place: "format".to_string(),
        rule: "the Deluge does not import: its device definitions are written by hand and read by the firmware, so there is nothing to read back".to_string(),
    })
}

fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    let sources = d.sources.as_deref().unwrap_or_default();
    if sources.is_empty() {
        return Err(Fault { kind: FaultKind::NoSources, place: "/sources".into(), rule: "a device without sources is not written: its credit and licence cannot be".into() });
    }
    let descriptor = Descriptor::Device(Box::new(d.clone()));
    let doc: Value = serde_json::from_str(&osd::write(&descriptor)).expect("a descriptor is JSON");
    let role_paths: Vec<(String, String)> = osd::roles(&descriptor).into_iter().map(|r| (r.pointer, r.param_path)).collect();
    let mut losses = Vec::new();

    if let Some(info) = doc.get("device").and_then(Value::as_object) {
        for key in info.keys() {
            loss(&mut losses, format!("/device/{}", segment(key)), "the device definition says nothing of the device itself");
        }
    }
    if let Some(top) = doc.as_object() {
        for key in top.keys().filter(|k| !["$schema", "device", "role_vocabulary", "cc_params", "params", "sources", "_sources"].contains(&k.as_str())) {
            loss(&mut losses, format!("/{}", segment(key)), "the device definition holds CC labels only: nothing of this");
        }
    }
    loss(&mut losses, "/sources", "the device definition has no field for a credit or a licence: it goes in a comment of the file");

    let mut labels: BTreeMap<i64, String> = BTreeMap::new();
    if let Some(b) = doc.get("cc_params").and_then(Value::as_object) {
        for key in b.keys().filter(|k| *k != "entries") {
            loss(&mut losses, format!("/cc_params/{}", segment(key)), "the device definition has no field for it");
        }
        for (n, e) in b.get("entries").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let Some(e) = e.as_object() else { continue };
            let ptr = format!("/cc_params/entries/{n}");
            let ways = e.get("ways").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
            // The first way that is a Control Change is the one written; each other way is named lost.
            let first_cc = ways.iter().position(|w| w.get("ctrlType").and_then(Value::as_str) == Some("cc"));
            let Some(at) = first_cc else {
                loss(&mut losses, &ptr, "the device definition names CC numbers only: a command with no CC is not written");
                continue;
            };
            let Some(cc) = ways[at].get("ctrlIndex").and_then(Value::as_array).and_then(|i| i.first()).and_then(Value::as_i64) else {
                loss(&mut losses, &ptr, "the device definition names CC numbers only: a command with no CC is not written");
                continue;
            };
            if !(0..CC_COUNT).contains(&cc) {
                loss(&mut losses, &ptr, format!("the Deluge reads CC 0 to 119: CC {cc} is not written"));
                continue;
            }
            if labels.contains_key(&cc) {
                loss(&mut losses, &ptr, format!("CC {cc} already has the name of an earlier command: one name per CC, it is not written"));
                continue;
            }
            let osc = e.get("osc").and_then(Value::as_str).unwrap_or("");
            let section = e.get("section").and_then(Value::as_str);
            let slash_section = section.filter(|s| s.starts_with('/'));
            let role = role_paths.iter().find(|(p, _)| *p == format!("{ptr}/role")).map(|(_, r)| r.as_str());
            let name = role.or(slash_section).unwrap_or(osc);
            if section.is_some() {
                if role.is_some() {
                    loss(&mut losses, format!("{ptr}/section"), "the control has a role: its path is the name, the section has no field");
                } else if slash_section.is_none() {
                    loss(&mut losses, format!("{ptr}/section"), "a free section is not a path: it is never a name");
                }
            }
            for key in e.keys().filter(|k| !["ways", "osc", "section", "role"].contains(&k.as_str())) {
                loss(&mut losses, format!("{ptr}/{}", segment(key)), "the device definition has no field for it: only the name of the CC is written");
            }
            for (m, w) in ways.iter().enumerate() {
                if m != at {
                    loss(&mut losses, format!("{ptr}/ways/{m}"), "the device definition names one CC per command: only the first way that is a CC is written");
                } else if let Some(w) = w.as_object() {
                    for key in w.keys().filter(|k| !["ctrlType", "ctrlIndex"].contains(&k.as_str())) {
                        loss(&mut losses, format!("{ptr}/ways/{m}/{}", segment(key)), "the device definition has no field for it: only the name of the CC is written");
                    }
                }
            }
            labels.insert(cc, name.to_string());
        }
    }
    if let Some(b) = doc.get("params").and_then(Value::as_object) {
        for (n, _) in b.get("entries").and_then(Value::as_array).into_iter().flatten().enumerate() {
            loss(&mut losses, format!("/params/entries/{n}"), "the device definition names CC numbers only: a SysEx param is not written");
        }
    }

    let mut text = String::from("<?xml version=\"1.0\"?>\n");
    for s in sources {
        text.push_str(&format!("<!-- {} -->\n", comment_safe(&source_text(s))));
    }
    text.push_str("<midiDevice>\n\t<ccLabels\n");
    for (cc, name) in &labels {
        text.push_str(&format!("\t\t{cc}=\"{}\"\n", escape(name)));
    }
    text.push_str("\t/>\n</midiDevice>\n");
    Ok(Translated { value: text, lost: losses })
}
