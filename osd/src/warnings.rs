//! What the reader reports of a descriptor without refusing it: a control that is to be completed,
//! a source whose provider the table of providers does not hold, and a source not carried as credit.

use crate::device::LEGACY_SOURCES;
use crate::json::Remarks;
use crate::providers::{is_provider, known};
use crate::{Descriptor, Source, Version};
use serde_json::Value;

/// Something a descriptor says that is no fault: the descriptor is read as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    /// The JSON pointer of the value the warning is about.
    pub pointer: String,
    pub kind: WarningKind,
}

/// The kinds of warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WarningKind {
    /// A control (an entry of `cc_params` or `params`) with no way: its message is to be completed.
    /// The pointer is the entry's.
    PartialControl,
    /// A well-formed `provider` the reader's table does not hold, as the source writes it. The
    /// pointer is the `provider` field's.
    UnknownProvider(String),
    /// A source not carried as credit: an entry of a note `_sources_<n>` (n from 2). The pointer is
    /// the entry's, or the note's when it is no list.
    UncreditedSource,
}

/// The warnings of a descriptor. A device gives one [`WarningKind::PartialControl`] for each entry
/// of `cc_params` then `params` with no way (0.4 to 0.7), then one
/// [`WarningKind::UnknownProvider`] for each well-formed `provider` of `sources` then
/// `midi_of_sources` of a device of 0.7 that the table of providers does not hold (before 0.7 there
/// is no `provider` field). A maker profile, a sound, a fingerprint, a firmware or an assembly of
/// 0.7 gives those of its `sources`, then one [`WarningKind::UncreditedSource`] for each source its
/// notes `_sources_<n>` keep; a sound gives the same for the assembly it was made on, after its
/// own. Before 0.7 these five kinds give none.
pub fn warnings(descriptor: &Descriptor) -> Vec<Warning> {
    if let Descriptor::Device(d) = descriptor {
        let cc = d.cc_params.iter().flat_map(|t| &t.entries).enumerate().filter(|(_, e)| e.ways.is_empty()).map(|(n, _)| format!("/cc_params/entries/{n}"));
        let params = d.params.iter().flat_map(|t| &t.entries).enumerate().filter(|(_, e)| e.ways.is_empty()).map(|(n, _)| format!("/params/entries/{n}"));
        let mut out: Vec<Warning> = cc.chain(params).map(|pointer| Warning { pointer, kind: WarningKind::PartialControl }).collect();
        if d.schema >= Some(Version::V0_7) {
            out.extend(unknown_providers("", &[("sources", &d.sources), ("midi_of_sources", &d.midi_of_sources)]));
        }
        return out;
    }
    if descriptor.schema() < Some(Version::V0_7) {
        return Vec::new();
    }
    match descriptor {
        Descriptor::Device(_) => Vec::new(),
        Descriptor::Profile(p) => credit_warnings("", &p.sources, &p.remarks),
        Descriptor::Sound(s) => {
            let mut out = credit_warnings("", &s.sources, &s.remarks);
            out.extend(s.made_on.iter().flat_map(|a| credit_warnings("/made_on", &a.sources, &a.remarks)));
            out
        }
        Descriptor::Fingerprint(f) => credit_warnings("", &f.sources, &f.remarks),
        Descriptor::Firmware(f) => credit_warnings("", &f.sources, &f.remarks),
        Descriptor::Assembly(a) => credit_warnings("", &a.sources, &a.remarks),
    }
}

/// The warnings on the credit of the object at `holder`, which is no device: its `sources`, then its
/// notes of sources not carried, by their number.
fn credit_warnings(holder: &str, sources: &Option<Vec<Source>>, notes: &Remarks) -> Vec<Warning> {
    let mut out = unknown_providers(holder, &[("sources", sources)]);
    let mut kept: Vec<(u32, &String, &Value)> = notes.iter().filter_map(|(k, v)| note_number(k).map(|n| (n, k, v))).collect();
    kept.sort_by_key(|(n, ..)| *n);
    for (_, key, value) in kept {
        let uncredited = |pointer| Warning { pointer, kind: WarningKind::UncreditedSource };
        match value.as_array() {
            Some(list) => out.extend((0..list.len()).map(|i| uncredited(format!("{holder}/{key}/{i}")))),
            None => out.push(uncredited(format!("{holder}/{key}"))),
        }
    }
    out
}

/// The number of a note of sources not carried, `_sources_<n>` with `n` from 2.
fn note_number(key: &str) -> Option<u32> {
    let digits = key.strip_prefix(LEGACY_SOURCES)?.strip_prefix('_')?;
    digits.parse().ok().filter(|n| *n >= 2 && digits.bytes().all(|b| b.is_ascii_digit()) && !digits.starts_with('0'))
}

/// One [`WarningKind::UnknownProvider`] for each well-formed `provider` of `lists`, those of the
/// object at `holder`, that the table of providers does not hold, in order.
fn unknown_providers(holder: &str, lists: &[(&str, &Option<Vec<Source>>)]) -> Vec<Warning> {
    let mut out = Vec::new();
    for (key, list) in lists {
        for (n, source) in list.iter().flatten().enumerate() {
            if let Some(id) = source.provider.as_deref().filter(|id| is_provider(id) && !known().contains(*id)) {
                out.push(Warning { pointer: format!("{holder}/{key}/{n}/provider"), kind: WarningKind::UnknownProvider(id.to_string()) });
            }
        }
    }
    out
}
