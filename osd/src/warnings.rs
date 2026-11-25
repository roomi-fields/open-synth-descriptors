//! What the reader reports of a descriptor without refusing it: a control that is to be completed,
//! and a source whose provider the table of providers does not hold.

use crate::providers::{is_provider, known};
use crate::{Descriptor, Version};

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
}

/// The warnings of a device, in the order `cc_params`, `params`, `sources`, `midi_of_sources`: one
/// [`WarningKind::PartialControl`] for each entry with no way (0.4 to 0.7), then one
/// [`WarningKind::UnknownProvider`] for each well-formed `provider` of a device of 0.7 that the table
/// of providers does not hold (before 0.7 there is no `provider` field). A descriptor of another
/// kind gives none.
pub fn warnings(descriptor: &Descriptor) -> Vec<Warning> {
    let Descriptor::Device(d) = descriptor else { return Vec::new() };
    let cc = d.cc_params.iter().flat_map(|t| &t.entries).enumerate().filter(|(_, e)| e.ways.is_empty()).map(|(n, _)| format!("/cc_params/entries/{n}"));
    let params = d.params.iter().flat_map(|t| &t.entries).enumerate().filter(|(_, e)| e.ways.is_empty()).map(|(n, _)| format!("/params/entries/{n}"));
    let mut out: Vec<Warning> = cc.chain(params).map(|pointer| Warning { pointer, kind: WarningKind::PartialControl }).collect();
    if d.schema < Some(Version::V0_7) {
        return out;
    }
    for (key, list) in [("sources", &d.sources), ("midi_of_sources", &d.midi_of_sources)] {
        for (n, source) in list.iter().flatten().enumerate() {
            if let Some(id) = source.provider.as_deref().filter(|id| is_provider(id) && !known().contains(*id)) {
                out.push(Warning { pointer: format!("/{key}/{n}/provider"), kind: WarningKind::UnknownProvider(id.to_string()) });
            }
        }
    }
    out
}
