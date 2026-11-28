//! The skeleton of a KnobKraft Orm adaptation (KnobKraft Orm, by Christof Ruch,
//! https://github.com/christofmuc/KnobKraft-orm, "Adaptation Programming Guide"). An adaptation
//! is Python code that only the KnobKraft host runs: this module writes text and never reads,
//! parses or runs one. It writes the functions whose constants the descriptor gives and names
//! each other function of the interface lost. The fate of each function is in docs/knobkraft.md.

#[cfg(test)]
mod tests;

use crate::credit::source_text;
use crate::{Contract, Fault, FaultKind, Loss, Translated};
use osd::{Device, DumpKind, FrameToken, LicenseExpression, Resolved, Source};

pub(crate) const CONTRACT: Contract = Contract {
    name: "knobkraft",
    media_type: "text/x-python",
    extension: "py",
    import,
    export,
    import_into: None,
};

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

/// The licence of the skeleton: that of KnobKraft Orm and of its adaptations (docs/knobkraft.md).
const SKELETON_LICENCE: &str = "AGPL-3.0-only";

/// The SPDX identifiers (without `-only`, `-or-later`) of the licences allowed under the AGPL-3.0.
/// Each has its line and its piece in docs/knobkraft.md.
const AGPL_COMPATIBLE: [&str; 5] = ["AGPL-3.0", "GPL-3.0", "MIT", "Apache-2.0", "CC-BY-4.0"];

/// The identifiers known not to be allowed under the AGPL-3.0, with their versions.
const AGPL_NOT_ALLOWED: [&str; 12] = [
    "GPL-2.0",
    "GPL-1.0",
    "LGPL-2.0",
    "LGPL-2.1",
    "LGPL-3.0",
    "CC-BY-SA-3.0",
    "CC-BY-SA-4.0",
    "CC-BY-NC-3.0",
    "CC-BY-NC-4.0",
    "CC-BY-NC-SA-4.0",
    "CC-BY-ND-4.0",
    "EPL-1.0",
];

#[derive(Clone, Copy, PartialEq)]
enum Standing {
    Compatible,
    NotAllowed,
    NotRecognised,
}

impl Standing {
    /// `AND`: allowed when both are, not allowed when one is.
    fn and(self, other: Standing) -> Standing {
        match (self, other) {
            (Standing::Compatible, Standing::Compatible) => Standing::Compatible,
            (Standing::NotAllowed, _) | (_, Standing::NotAllowed) => Standing::NotAllowed,
            _ => Standing::NotRecognised,
        }
    }

    /// `OR`: allowed when one is, not allowed when both are.
    fn or(self, other: Standing) -> Standing {
        match (self, other) {
            (Standing::Compatible, _) | (_, Standing::Compatible) => Standing::Compatible,
            (Standing::NotAllowed, Standing::NotAllowed) => Standing::NotAllowed,
            _ => Standing::NotRecognised,
        }
    }
}

/// A licence as written: an SPDX expression is judged branch by branch, as `osd` reads it; any
/// other text is one id, its edge spaces ignored, which no list holds when it is not an id.
fn classify(license: &str) -> Standing {
    match LicenseExpression::parse(license) {
        Some(expression) => expression.judge(classify_with, Standing::and, Standing::or),
        None => classify_id(license.trim()),
    }
}

/// An id with its exception, if any: an exception may lift what the id forbids, so the pair is
/// allowed when the id alone is, and otherwise not recognised.
fn classify_with(id: &str, exception: Option<&str>) -> Standing {
    match (classify_id(id), exception) {
        (standing, None) => standing,
        (Standing::Compatible, Some(_)) => Standing::Compatible,
        (_, Some(_)) => Standing::NotRecognised,
    }
}

/// A licence by its SPDX identifier, case ignored. `X-only` is `X`; `X+` and `X-or-later` are
/// `X` or any later version of it: allowed when a later version is, else judged as `X`.
fn classify_id(id: &str) -> Standing {
    let id = id.to_ascii_lowercase();
    let or_later = id.strip_suffix('+').or_else(|| id.strip_suffix("-or-later"));
    let id = or_later.or_else(|| id.strip_suffix("-only")).unwrap_or(&id);
    let is = |list: &[&str]| list.iter().any(|k| k.eq_ignore_ascii_case(id));
    if is(&AGPL_COMPATIBLE) || or_later.is_some_and(has_compatible_later_version) {
        Standing::Compatible
    } else if is(&AGPL_NOT_ALLOWED) {
        Standing::NotAllowed
    } else {
        Standing::NotRecognised
    }
}

/// An identifier as its licence and its version: `gpl-2.0` is `gpl`, version 2.0.
fn versioned(id: &str) -> Option<(&str, Vec<u32>)> {
    let (name, version) = id.rsplit_once('-')?;
    let numbers = version.split('.').map(|n| n.parse().ok()).collect::<Option<Vec<u32>>>()?;
    Some((name, numbers))
}

fn has_compatible_later_version(id: &str) -> bool {
    let Some((name, version)) = versioned(id) else { return false };
    AGPL_COMPATIBLE.iter().any(|k| versioned(k).is_some_and(|(n, v)| n.eq_ignore_ascii_case(name) && v > version))
}

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

/// This format is exported only: every file is refused.
fn import(_: &str, _: &Source) -> Result<Translated<Device>, Fault> {
    Err(Fault {
        kind: FaultKind::InvalidFile,
        place: "format".to_string(),
        rule: "KnobKraft does not import: an adaptation is code that only its host, KnobKraft Orm, runs, and osd-formats never reads nor runs it".to_string(),
    })
}

fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    let sources = d.sources.as_deref().unwrap_or_default();
    if sources.is_empty() {
        return Err(Fault { kind: FaultKind::NoSources, place: "/sources".into(), rule: "a device without sources is not written: its credit and licence cannot be".into() });
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
    text.push_str(&format!("# SPDX-License-Identifier: {SKELETON_LICENCE}\n"));
    for (i, s) in sources.iter().enumerate() {
        let line = source_text(s).replace(['\n', '\r'], " ");
        text.push_str(&format!("# Source: {line}\n"));
        let license = s.license.as_deref().unwrap_or("").trim().replace(['\n', '\r'], " ");
        let warning = match classify(s.license.as_deref().unwrap_or("")) {
            Standing::Compatible => continue,
            Standing::NotAllowed => format!("Licence not allowed under AGPL-3.0: {license}"),
            Standing::NotRecognised if license.is_empty() => "Licence not recognised, not checked against AGPL-3.0: none declared".to_string(),
            Standing::NotRecognised => format!("Licence not recognised, not checked against AGPL-3.0: {license}"),
        };
        text.push_str(&format!("# {warning}\n"));
        lose(&mut losses, &format!("/sources/{i}/license"), warning);
    }
    text.push('\n');
    text.push_str(&format!("def name():\n    return {}\n", python_string(&d.device.name)));

    let programs = d.programs.as_ref();
    let banks = programs.and_then(|p| p.banks.as_ref()).and_then(|b| b.as_deref()).filter(|b| !b.is_empty());
    match banks {
        Some(banks) => {
            text.push_str(&format!("\ndef numberOfBanks():\n    return {}\n", banks.len()));
            let size = banks[0].size;
            if banks.iter().all(|b| b.size == size) {
                text.push_str(&format!("\ndef numberOfPatchesPerBank():\n    return {size}\n"));
            } else {
                lose(&mut losses, "/programs/banks", "numberOfPatchesPerBank: the banks do not all have the same size: the function returns one number");
            }
        }
        None => {
            lose(&mut losses, "/sound_transfers", "numberOfBanks: the descriptor gives no banks");
            lose(&mut losses, "/sound_transfers", "numberOfPatchesPerBank: the descriptor gives no banks");
        }
    }

    let dump = programs.and_then(|p| p.dump.as_ref()).and_then(|x| x.as_ref());
    let request = dump.filter(|x| matches!(x.kind, DumpKind::Request | DumpKind::EditBuffer) && x.script.is_none()).and_then(|x| x.request.as_ref());
    let bytes: Option<Vec<u8>> = request.and_then(|r| r.iter().map(|t| if let FrameToken::Byte(b) = t { Some(*b) } else { None }).collect());
    match bytes {
        Some(bytes) if !bytes.is_empty() => {
            let list = bytes.iter().map(|b| format!("0x{b:02X}")).collect::<Vec<_>>().join(", ");
            text.push_str(&format!("\ndef createEditBufferRequest(channel):\n    return [{list}]\n"));
        }
        _ => lose(&mut losses, if dump.is_some() { "/programs/dump" } else { "/sound_transfers" }, "createEditBufferRequest: the descriptor gives no request of the current sound made of fixed bytes"),
    }

    for (function, reason) in LEFT_TO_HAND {
        lose(&mut losses, "/sound_transfers", format!("{function}: {reason}"));
    }
    Ok(Translated { value: text, lost: losses })
}
