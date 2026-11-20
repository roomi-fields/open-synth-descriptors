//! The skeleton of a KnobKraft Orm adaptation (KnobKraft Orm, by Christof Ruch,
//! https://github.com/christofmuc/KnobKraft-orm, "Adaptation Programming Guide"). An adaptation
//! is Python code that only the KnobKraft host runs: this module writes text and never reads,
//! parses or runs one. It writes the functions whose constants the descriptor gives and names
//! each other function of the interface lost. The fate of each function is in docs/knobkraft.md.

use crate::credit::source_text;
use crate::{Fault, FaultKind, Loss, Translated};
use osd::{DumpKind, FrameToken, Resolved};

/// The functions of the interface the export cannot write, each with its reason.
const LEFT_TO_HAND: [(&str, &str); 20] = [
    ("createDeviceDetectMessage", "the detection message is code of the adaptation: the descriptor's identity is not turned into it"),
    ("needsChannelSpecificDetection", "it follows the detection message, which is not written"),
    ("channelIfValidDeviceResponse", "it reads a reply: code the descriptor does not give"),
    ("deviceDetectWaitMilliseconds", "it follows the detection message, which is not written"),
    ("isEditBufferDump", "recognising a dump is code: the descriptor's match frames are not turned into it"),
    ("convertToEditBuffer", "turning a dump into an edit buffer message is code the descriptor does not give"),
    ("nameFromDump", "reading the name out of a dump is code: the descriptor's name element is not turned into it"),
    ("renamePatch", "writing a name into a dump is code the descriptor does not give"),
    ("isDefaultName", "the descriptor does not say which names are defaults"),
    ("calculateFingerprint", "which bytes of a dump make the sound is code: the descriptor's elements are not turned into it"),
    ("createProgramDumpRequest", "the descriptor gives one request of the current sound, not one per program"),
    ("isSingleProgramDump", "recognising a dump is code: the descriptor's match frames are not turned into it"),
    ("convertToProgramDump", "turning a dump into a program dump message is code the descriptor does not give"),
    ("numberFromDump", "reading the program number out of a dump is code the descriptor does not give"),
    ("createBankDumpRequest", "the descriptor does not give a request of a whole bank"),
    ("isPartOfBankDump", "recognising the part of a bank dump is code the descriptor does not give"),
    ("isBankDumpFinished", "recognising the end of a bank dump is code the descriptor does not give"),
    ("extractPatchesFromBank", "cutting a bank dump into sounds is code the descriptor does not give"),
    ("convertPatchesToBankDump", "building a bank dump from sounds is code the descriptor does not give"),
    ("friendlyBankName", "the descriptor's bank names are not turned into a function: left to write"),
];

fn lose(losses: &mut Vec<Loss>, place: &str, reason: impl Into<String>) {
    losses.push(Loss { place: place.to_string(), reason: reason.into() });
}

/// A Python string literal: quotes and backslashes escaped, line breaks written as escapes.
fn python_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn import_refusal() -> Fault {
    Fault {
        kind: FaultKind::InvalidFile,
        place: "format".to_string(),
        rule: "KnobKraft does not import: an adaptation is code that only its host, KnobKraft Orm, runs, and osd-formats never reads nor runs it".to_string(),
    }
}

pub(crate) fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    let sources = d.sources.as_deref().unwrap_or_default();
    if sources.is_empty() {
        return Err(Fault { kind: FaultKind::NoSources, place: "/_sources".into(), rule: "a device without sources is not written: its credit and licence cannot be".into() });
    }
    if d.sound_transfers.as_ref().is_none_or(|t| t.is_empty()) {
        return Err(Fault {
            kind: FaultKind::NoSoundTransfers,
            place: "/sound_transfers".into(),
            rule: "a device without sound transfers is not written: an adaptation is about the sounds that travel".into(),
        });
    }
    let mut losses = Vec::new();
    let mut text = String::new();
    text.push_str("# Skeleton of a KnobKraft Orm adaptation, written from an Open Synth Descriptors device.\n");
    text.push_str("# KnobKraft Orm, by Christof Ruch, https://github.com/christofmuc/KnobKraft-orm, under the AGPL-3.0 licence;\n");
    text.push_str("# the Adaptation Programming Guide of KnobKraft Orm names the functions below.\n");
    for s in sources {
        let line = source_text(s).replace(['\n', '\r'], " ");
        text.push_str(&format!("# Source: {line}\n"));
    }
    text.push('\n');
    text.push_str(&format!("def name():\n    return {}\n", python_string(&d.device.name)));

    let programs = d.programs.as_ref();
    let banks = programs.and_then(|p| p.banks.as_deref()).filter(|b| !b.is_empty());
    match banks {
        Some(banks) => {
            text.push_str(&format!("\ndef numberOfBanks():\n    return {}\n", banks.len()));
            let size = banks[0].size;
            if banks.iter().all(|b| b.size == size) {
                text.push_str(&format!("\ndef numberOfPatchesPerBank():\n    return {size}\n"));
            } else {
                lose(&mut losses, "numberOfPatchesPerBank", "the banks do not all have the same size: the function returns one number");
            }
        }
        None => {
            lose(&mut losses, "numberOfBanks", "the descriptor gives no banks");
            lose(&mut losses, "numberOfPatchesPerBank", "the descriptor gives no banks");
        }
    }

    let dump = programs.and_then(|p| p.dump.as_ref());
    let request = dump.filter(|x| matches!(x.kind, DumpKind::Request | DumpKind::EditBuffer) && x.script.is_none()).and_then(|x| x.request.as_ref());
    let bytes: Option<Vec<u8>> = request.and_then(|r| r.iter().map(|t| if let FrameToken::Byte(b) = t { Some(*b) } else { None }).collect());
    match bytes {
        Some(bytes) if !bytes.is_empty() => {
            let list = bytes.iter().map(|b| format!("0x{b:02X}")).collect::<Vec<_>>().join(", ");
            text.push_str(&format!("\ndef createEditBufferRequest(channel):\n    return [{list}]\n"));
        }
        _ => lose(&mut losses, "createEditBufferRequest", "the descriptor gives no request of the current sound made of fixed bytes"),
    }

    for (function, reason) in LEFT_TO_HAND {
        lose(&mut losses, function, reason);
    }
    Ok(Translated { value: text, lost: losses })
}
