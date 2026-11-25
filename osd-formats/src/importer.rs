//! What the importers share: the credit a source must give, the device of a first import, the
//! addresses of its entries and the identity of a base a file is added to.

use crate::{Fault, FaultKind};
use osd::{Device, DeviceInfo, Source, Version};
use std::collections::BTreeSet;

/// The credit an import needs: a way to find the source and its licence.
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
        return Ok(());
    }
    let place = format!("source: {}", missing.join(" and "));
    Err(Fault {
        kind: FaultKind::SourceIncomplete,
        place,
        rule: "an import carries the source's credit and licence: the source names one of url, author or attribution, and a license".to_string(),
    })
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
