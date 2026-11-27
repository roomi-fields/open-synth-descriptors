//! What the importers share: the credit a source must give, the device of a first import, the
//! addresses and claimed messages of its entries, the identity of a base a file is added to and
//! the reading back of what is returned.

use crate::{Fault, FaultKind, Translated};
use osd::{Device, DeviceInfo, Source, Version, Way};
use std::collections::BTreeSet;

/// The source an import takes: it gives its credit (a way to find it and its licence), and the
/// reader of 0.7 accepts it in the descriptor written.
pub(crate) fn check_source(source: &Source) -> Result<(), Fault> {
    let named = |o: &Option<String>| o.as_deref().is_some_and(|s| !s.trim().is_empty());
    let mut missing = Vec::new();
    if !(named(&source.url) || named(&source.author) || named(&source.attribution)) {
        missing.push("url, author or attribution");
    }
    if !named(&source.license) {
        missing.push("license");
    }
    if missing.is_empty() {
        return read_with(std::slice::from_ref(source));
    }
    let place = format!("source: {}", missing.join(" and "));
    Err(Fault {
        kind: FaultKind::SourceIncomplete,
        place,
        rule: "an import carries the source's credit and licence: the source names one of url, author or attribution, and a license".to_string(),
    })
}

/// Refuses a source whose `id` a source of `base`, a base of 0.7, already has: the descriptor
/// written would hold the id twice.
pub(crate) fn check_new_id(base: &Device, source: &Source) -> Result<(), Fault> {
    let held = [&base.sources, &base.midi_of_sources].into_iter().flatten().flatten().any(|s| s.id.is_some() && s.id == source.id);
    if held {
        return read_with(&[source.clone(), source.clone()]);
    }
    Ok(())
}

/// The judgement of the reader of `osd` on a descriptor of 0.7 that holds `sources`, one source
/// or the same source twice: its fault is the refusal of the source, at the field it names, or at
/// `id` when it refuses the second entry as a whole.
fn read_with(sources: &[Source]) -> Result<(), Fault> {
    let mut probe = blank_device("probe".to_string(), None, &sources[0]);
    probe.sources = Some(sources.to_vec());
    let text = osd::write(&osd::Descriptor::Device(Box::new(probe)));
    let Err(fault) = osd::read(&text, osd::Kind::Device) else { return Ok(()) };
    let mut path = fault.pointer.trim_start_matches('/').split('/').skip(2);
    let field = path.next().unwrap_or("id");
    Err(Fault {
        kind: FaultKind::SourceRefused,
        place: format!("source/{field}"),
        rule: format!("the 0.7 reader refuses this source: {}", fault.rule),
    })
}

/// A message a control claims (`spec/0.7.md`): its kind (CC, NRPN, RPN), its channel and its
/// numbers. A CC of two numbers claims each, one message each; an NRPN or an RPN its two as one.
pub(crate) type Claim = (u8, Option<u8>, Option<u8>, Option<u8>);

/// The messages a way claims on `channel`.
pub(crate) fn way_claims(w: &Way, channel: Option<u8>) -> Vec<Claim> {
    let number = |n: usize| w.ctrl_index.as_ref().and_then(|i| i.get(n)).map(|n| *n as u8);
    match w.ctrl_type.as_str() {
        "cc" => [number(0), number(1)].into_iter().flatten().map(|v| (0, channel, Some(v), None)).collect(),
        "nrpn" => vec![(1, channel, number(0), number(1))],
        "rpn" => vec![(2, channel, number(0), number(1))],
        _ => Vec::new(),
    }
}

/// The result of an import as the reader of `osd` judges it (docs/CADRE.md R15): the descriptor is returned
/// only when its text reads; otherwise the import is refused, with the reader's rule and pointer.
pub(crate) fn read_back(imported: Translated<Device>) -> Result<Translated<Device>, Fault> {
    let text = osd::write(&osd::Descriptor::Device(Box::new(imported.value.clone())));
    match osd::read(&text, osd::Kind::Device) {
        Ok(_) => Ok(imported),
        Err(f) => Err(Fault {
            kind: FaultKind::InvalidFile,
            place: String::new(),
            rule: format!("the descriptor the import would write does not read in 0.7: {} at {}", f.rule, f.pointer),
        }),
    }
}

/// The source an importer reads: the date of the import (`imported_at`), for which 0.7 has no
/// field, is the note `_imported_at`, its value unchanged; when that key is taken, the first free
/// of `_imported_at_2`, `_imported_at_3`… (the rule of the converter of `osd`).
pub(crate) fn note_date(source: &Source) -> Source {
    let mut noted = source.clone();
    if let Some(date) = noted.imported_at.take() {
        let mut key = "_imported_at".to_string();
        let mut n = 2;
        while noted.extra.contains_key(&key) {
            key = format!("_imported_at_{n}");
            n += 1;
        }
        noted.extra.insert(key, date.into());
    }
    noted
}

/// The address of a control: its section and name as path segments.
pub(crate) fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// An address not yet used: `base`, else `base-2`, `base-3`…
pub(crate) fn unique(base: String, used: &mut BTreeSet<String>) -> String {
    let mut address = base.clone();
    let mut n = 2;
    while !used.insert(address.clone()) {
        address = format!("{base}-{n}");
        n += 1;
    }
    address
}

/// The device of a first import: its identity and its one source, no block.
pub(crate) fn blank_device(name: String, vendor: Option<String>, source: &Source) -> Device {
    let prefix = format!("/{}", slug(&name));
    Device {
        schema: Some(Version::V0_7),
        device: DeviceInfo {
            name,
            vendor,
            revision: None,
            osc_prefix: prefix,
            device_id: None,
            rate_limit_hz: None,
            kind: None,
            transport: None,
            stands_for: None,
            fidelity: None,
            remarks: Default::default(),
        },
        capabilities: None,
        midi_of: None,
        contexts: None,
        loads_sounds_of: None,
        sound_transfers: None,
        resources: None,
        identity: None,
        programs: None,
        sysex: None,
        cc_params: None,
        params: None,
        commands: None,
        triggers: None,
        midi_out: None,
        midi_in: None,
        replies: None,
        subscriptions: None,
        custom_commands: None,
        sources: Some(vec![source.clone()]),
        midi_of_sources: None,
        coverage: None,
        role_vocabulary: None,
        remarks: Default::default(),
    }
}

/// The addresses a device already uses, controls and triggers.
pub(crate) fn used_addresses(base: &Device) -> BTreeSet<String> {
    let cc = base.cc_params.iter().flat_map(|c| c.entries.iter().map(|e| e.osc.clone()));
    let tr = base.triggers.iter().flat_map(|t| t.entries.iter().map(|e| e.osc.clone()));
    cc.chain(tr).collect()
}

/// A base that names no vendor or no device takes the first the file gave.
pub(crate) fn adopt_identity(base: &mut Device, identity: (String, String)) {
    if base.device.vendor.is_none() {
        base.device.vendor = Some(identity.0).filter(|v| !v.is_empty());
    }
    if base.device.name.is_empty() {
        base.device.name = identity.1;
    }
}
