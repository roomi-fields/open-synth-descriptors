//! The reading back every import ends on (docs/CADRE.md R15): a result is returned only when the
//! reader of `osd` reads it, else it is `InvalidFile` at the empty place, its rule carrying that
//! of `osd` and its pointer. No file an importer accepts is known to give a refused result, so the
//! results here are devices the reader refuses, given to the module itself.

use crate::importer::read_back;
use crate::{FaultKind, Loss, Translated};
use osd::{Descriptor, Device, Kind};
use serde_json::{json, Value};

/// A device of 0.7 that reads, as JSON: one control on the CC 74, one source.
fn readable() -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "device": { "name": "Trundler", "osc_prefix": "/trundler" },
        "cc_params": { "entries": [{ "osc": "/cutoff", "ways": [{ "ctrlType": "cc", "ctrlIndex": [74] }] }] },
        "sources": [{ "type": "community-re", "url": "u", "license": "CC0-1.0", "id": "a" }]
    })
}

fn device_of(json: &Value) -> Device {
    match osd::read(&json.to_string(), Kind::Device) {
        Ok(Descriptor::Device(d)) => *d,
        other => panic!("{other:?}"),
    }
}

fn result_of(device: Device) -> Translated<Device> {
    let lost = vec![Loss { place: "line 2".to_string(), reason: "left out".to_string() }];
    Translated { value: device, lost }
}

#[test]
fn a_result_the_reader_reads_is_returned_with_its_losses() {
    let given = result_of(device_of(&readable()));
    assert_eq!(read_back(result_of(device_of(&readable()))), Ok(given));
}

#[test]
fn a_result_the_reader_refuses_is_an_invalid_file_carrying_the_rule_and_the_pointer_of_osd() {
    // Two controls on one CC, a default outside its scale, one `id` for two sources.
    let mut claimed_twice = device_of(&readable());
    let entries = &mut claimed_twice.cc_params.as_mut().unwrap().entries;
    let mut second = entries[0].clone();
    second.osc = "/resonance".to_string();
    entries.push(second);

    let mut default_outside = device_of(&readable());
    default_outside.cc_params.as_mut().unwrap().entries[0].default = Some(200);

    let mut id_twice = device_of(&readable());
    let sources = id_twice.sources.as_mut().unwrap();
    sources.push(sources[0].clone());

    for (device, pointer) in [
        (claimed_twice, "/cc_params/entries/1/ways/0"),
        (default_outside, "/cc_params/entries/0/default"),
        (id_twice, "/sources/1"),
    ] {
        let text = osd::write(&Descriptor::Device(Box::new(device.clone())));
        let of_osd = osd::read(&text, Kind::Device).expect_err("the reader refuses this device");
        assert_eq!(of_osd.pointer, pointer);

        let fault = read_back(result_of(device)).expect_err(pointer);
        assert_eq!(fault.kind, FaultKind::InvalidFile, "{pointer}");
        assert_eq!(fault.place, "", "{pointer}");
        assert!(fault.rule.contains(&of_osd.rule), "{}", fault.rule);
        assert!(fault.rule.contains(pointer), "{}", fault.rule);
    }
}
