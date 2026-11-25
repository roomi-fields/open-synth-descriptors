//! A format that exists in the tests of the crate only: its module and its line in the list of
//! `tests.rs` are all that adds it. Each of its operations returns what shows it ran.

use crate::{Contract, Fault, Loss, Translated};
use osd::{Descriptor, Device, Resolved, Source};

pub(crate) const CONTRACT: Contract = Contract {
    name: "test-only",
    media_type: "application/x-test-only",
    extension: "tst",
    import,
    export,
    import_into: Some(import_into),
};

/// A device named `text`, carrying `source`.
fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    let descriptor = format!(
        r#"{{"$schema": "https://w3id.org/osd/0.7/device.schema.json", "device": {{"name": "{text}", "osc_prefix": "/test-only"}}}}"#
    );
    let Ok(Descriptor::Device(mut device)) = osd::read(&descriptor, osd::Kind::Device) else { panic!("{descriptor}") };
    device.sources = Some(vec![source.clone()]);
    Ok(Translated { value: *device, lost: Vec::new() })
}

/// The name of the device.
fn export(device: &Resolved) -> Result<Translated<String>, Fault> {
    Ok(Translated { value: device.device.device.name.clone(), lost: Vec::new() })
}

/// `base` with `source` after its sources, and `text` listed as a loss.
fn import_into(mut base: Device, text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    base.sources.get_or_insert_with(Vec::new).push(source.clone());
    Ok(Translated { value: base, lost: vec![Loss { place: "line 1".to_string(), reason: text.to_string() }] })
}
