//! What `osd` publishes of a control with no message: the fields of an entry without a way, and
//! `osd::warnings`, one `PartialControl` per control to be completed, beside the
//! `UnknownProvider` of a source of 0.7.

// @lit osd/providers.json

use osd::{Descriptor, Direction, Kind, Version, WarningKind};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VERSIONS: [Version; 4] = [Version::V0_4, Version::V0_5, Version::V0_6, Version::V0_7];

fn device(version: Version, mut body: Value) -> Value {
    body["device"] = json!({"name": "T", "osc_prefix": "/t"});
    if version != Version::V0_4 {
        body["$schema"] = json!(format!("https://w3id.org/osd/{}/device.schema.json", version.name()));
    }
    body
}

/// An entry with no message in the form of `version`.
fn silent(version: Version, osc: &str) -> Value {
    match version {
        Version::V0_7 => json!({"osc": osc, "ways": []}),
        _ => json!({"osc": osc}),
    }
}

/// An entry of `cc_params` with a message in the form of `version`.
fn heard(version: Version, osc: &str, cc: u8) -> Value {
    match version {
        Version::V0_7 => json!({"osc": osc, "ways": [{"ctrlType": "cc", "ctrlIndex": [cc]}]}),
        _ => json!({"osc": osc, "cc": cc}),
    }
}

/// An entry of `params` with a message in the form of `version`.
fn heard_param(version: Version, osc: &str, id: u8) -> Value {
    match version {
        Version::V0_7 => json!({"osc": osc, "ways": [{"ctrlType": "sysex", "ctrlIndex": [id]}]}),
        _ => json!({"osc": osc, "id": id}),
    }
}

fn params(version: Version, entries: Vec<Value>) -> Value {
    let (get, set) = match version {
        Version::V0_4 => (json!([240, "{p}", 247]), json!([240, "{p}", "{value}", 247])),
        _ => (json!([240, {"arg": "p"}, 247]), json!([240, {"arg": "p"}, {"arg": "value"}, 247])),
    };
    json!({"get_frame": get, "set_frame": set, "value_type": "u7", "entries": entries})
}

/// A device of `version` whose controls 1 and 3 of `cc_params` and whose entry 0 of `params` have
/// no message.
fn partial(version: Version) -> Value {
    device(version, json!({
        "cc_params": {"entries": [heard(version, "/a", 1), silent(version, "/b"), heard(version, "/c", 2), silent(version, "/d")]},
        "params": params(version, vec![silent(version, "/e"), heard_param(version, "/f", 3)]),
    }))
}

const PARTIAL: [&str; 3] = ["/cc_params/entries/1", "/cc_params/entries/3", "/params/entries/0"];

fn read(doc: &Value) -> Descriptor {
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}\n{doc}"))
}

fn entries(d: Descriptor) -> osd::Device {
    match d {
        Descriptor::Device(d) => *d,
        _ => panic!("a device"),
    }
}

/// The warnings of `d` as `(kind, provider, pointer)`.
fn warned(d: &Descriptor) -> Vec<(&'static str, Option<String>, String)> {
    osd::warnings(d)
        .into_iter()
        .map(|w| match w.kind {
            WarningKind::PartialControl => ("PartialControl", None, w.pointer),
            WarningKind::UnknownProvider(id) => ("UnknownProvider", Some(id), w.pointer),
            WarningKind::UncreditedSource => ("UncreditedSource", None, w.pointer),
        })
        .collect()
}

fn partials(pointers: &[&str]) -> Vec<(&'static str, Option<String>, String)> {
    pointers.iter().map(|p| ("PartialControl", None, p.to_string())).collect()
}

// ---- the fields of an entry without a way

/// The word of a direction, as the descriptor writes it.
fn word(d: &Option<Direction>) -> Option<String> {
    d.as_ref().map(|d| format!("{d:?}").trim_matches('"').to_lowercase())
}

#[test]
fn a_control_without_a_message_reads_with_no_way_and_its_fields_under_the_names_of_0_7() {
    for version in VERSIONS {
        let directions = version >= Version::V0_6;
        let mut entry = silent(version, "/a");
        entry["range"] = json!([-1, 5]);
        entry["nrpn_lsb_range"] = json!([0, 15]);
        if directions {
            entry["transmit"] = json!("relative");
            entry["recognize"] = json!("none");
        }
        let d = entries(read(&device(version, json!({"cc_params": {"entries": [entry, silent(version, "/bare")]}}))));
        let entries = d.cc_params.unwrap().entries;
        assert!(entries[0].ways.is_empty() && entries[1].ways.is_empty(), "{}", version.name());
        assert_eq!(entries[0].range, Some([-1, 5]), "{}", version.name());
        assert_eq!(entries[0].nrpn_lsb_range, Some([0, 15]), "{}", version.name());
        assert_eq!(word(&entries[0].transmit).as_deref(), directions.then_some("relative"), "{}", version.name());
        assert_eq!(word(&entries[0].recognize).as_deref(), directions.then_some("none"), "{}", version.name());
        let bare = &entries[1];
        assert!(bare.range.is_none() && bare.nrpn_lsb_range.is_none() && bare.transmit.is_none() && bare.recognize.is_none(), "{bare:?}");
    }
}

#[test]
fn a_params_entry_with_a_range_and_no_number_reads_and_keeps_its_fields() {
    for version in VERSIONS {
        let directions = version >= Version::V0_6;
        let mut entry = silent(version, "/a");
        entry["range"] = json!([0, 99]);
        if directions {
            entry["transmit"] = json!("both");
            entry["recognize"] = json!("absolute");
        }
        let d = entries(read(&device(version, json!({"params": params(version, vec![entry])}))));
        let table = d.params.unwrap();
        let e = &table.entries[0];
        assert!(e.ways.is_empty(), "{}", version.name());
        assert_eq!(e.range, Some([0, 99]), "{}", version.name());
        assert_eq!(word(&e.transmit).as_deref(), directions.then_some("both"), "{}", version.name());
        assert_eq!(word(&e.recognize).as_deref(), directions.then_some("absolute"), "{}", version.name());
    }
}

#[test]
fn a_control_with_a_way_holds_no_range_or_direction_on_the_entry() {
    for version in VERSIONS {
        let mut entry = heard(version, "/a", 1);
        if version < Version::V0_7 {
            entry["range"] = json!([0, 5]);
        } else {
            entry["ways"][0]["range"] = json!([0, 5]);
        }
        let d = entries(read(&device(version, json!({"cc_params": {"entries": [entry]}}))));
        let table = d.cc_params.unwrap();
        let e = &table.entries[0];
        assert_eq!(e.ways[0].range, Some([0, 5]), "{}", version.name());
        assert!(e.range.is_none() && e.nrpn_lsb_range.is_none() && e.transmit.is_none() && e.recognize.is_none(), "{}: {e:?}", version.name());
    }
}

#[test]
fn the_reference_scale_of_an_entry_without_a_way_is_its_scale_else_its_range() {
    let d = entries(read(&device(Version::V0_7, json!({
        "cc_params": {"entries": [
            {"osc": "/range", "ways": [], "range": [-64, 63]},
            {"osc": "/both", "ways": [], "range": [0, 10], "scale": [0, 1000]},
            {"osc": "/scale", "ways": [], "scale": [1, 8]},
        ]},
        "params": params(Version::V0_7, vec![json!({"osc": "/range", "ways": [], "range": [0, 99]})]),
    }))));
    let cc: Vec<[i64; 2]> = d.cc_params.unwrap().entries.iter().map(|e| e.reference_scale()).collect();
    assert_eq!(cc, [[-64, 63], [0, 1000], [1, 8]]);
    assert_eq!(d.params.unwrap().entries[0].reference_scale(), [0, 99]);
}

// ---- the warnings

#[test]
fn each_control_without_a_way_of_0_4_to_0_7_is_warned_at_the_pointer_of_its_entry() {
    for version in VERSIONS {
        assert_eq!(warned(&read(&partial(version))), partials(&PARTIAL), "{}", version.name());
    }
}

#[test]
fn a_device_whose_controls_all_have_a_way_gives_no_warning() {
    for version in VERSIONS {
        let doc = device(version, json!({
            "cc_params": {"entries": [heard(version, "/a", 1)]},
            "params": params(version, vec![heard_param(version, "/f", 3)]),
        }));
        assert!(warned(&read(&doc)).is_empty(), "{}", version.name());
        assert!(warned(&read(&device(version, json!({})))).is_empty(), "{}", version.name());
    }
}

#[test]
fn a_control_that_gives_only_an_incomplete_message_is_partial() {
    let doc = device(Version::V0_6, json!({
        "cc_params": {"entries": [
            {"osc": "/lsb", "cc_lsb": 33},
            {"osc": "/nrpn", "nrpn_lsb": 1, "value_bits": 14},
            {"osc": "/also", "also": [{"cc": 3}]},
            {"osc": "/beside", "rpn_msb": 1, "value_bits": 14, "also": [{"cc": 4}]},
            {"osc": "/both", "also": [{"cc_lsb": 40}, {"nrpn_msb": 2, "value_bits": 7}]},
        ]},
    }));
    assert_eq!(warned(&read(&doc)), partials(&["/cc_params/entries/0", "/cc_params/entries/1", "/cc_params/entries/4"]));
}

#[test]
fn a_params_entry_that_gives_one_of_its_numbers_is_not_partial() {
    let doc = device(Version::V0_6, json!({
        "params": params(Version::V0_6, vec![json!({"osc": "/r", "r": 1}), json!({"osc": "/c", "c": 1})]),
    }));
    assert!(warned(&read(&doc)).is_empty());
}

#[test]
fn the_warnings_come_in_the_order_cc_params_params_sources_midi_of_sources() {
    let source = |provider: &str| json!({"type": "contributed", "provider": provider});
    let mut doc = partial(Version::V0_7);
    doc["midi_of_sources"] = json!([source("zz-target")]);
    doc["sources"] = json!([source("electra-one"), source("zz-one"), source("zz-two")]);
    let mut want = partials(&PARTIAL);
    for (id, pointer) in [("zz-one", "/sources/1/provider"), ("zz-two", "/sources/2/provider"), ("zz-target", "/midi_of_sources/0/provider")] {
        want.push(("UnknownProvider", Some(id.to_string()), pointer.to_string()));
    }
    assert_eq!(warned(&read(&doc)), want);
}

#[test]
fn a_partial_control_before_0_7_is_warned_and_its_provider_is_not() {
    let doc = device(Version::V0_6, json!({
        "_sources": [{"type": "knobkraft", "provider": "zz-one"}],
        "cc_params": {"entries": [{"osc": "/a"}]},
    }));
    assert_eq!(warned(&read(&doc)), partials(&["/cc_params/entries/0"]));
}

#[test]
fn a_converted_and_a_resolved_device_warn_of_their_partial_controls() {
    for version in [Version::V0_4, Version::V0_5, Version::V0_6] {
        for target in [Version::V0_6, Version::V0_7].into_iter().filter(|t| *t >= version) {
            let out = osd::convert_to(&partial(version).to_string(), Kind::Device, target).unwrap_or_else(|f| panic!("{f}"));
            let d = osd::read(&out, Kind::Device).unwrap();
            assert_eq!(warned(&d), partials(&PARTIAL), "{} to {}", version.name(), target.name());
        }
    }
    let target = partial(Version::V0_5);
    let variant = device(Version::V0_7, json!({"midi_of": "acme/b"}));
    let files: BTreeMap<String, String> = [("acme/a", variant), ("acme/b", target)].into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect();
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(warned(&Descriptor::Device(Box::new(r.device))), partials(&PARTIAL));
}

#[test]
fn a_descriptor_that_is_not_a_device_gives_no_warning() {
    let profile = json!({"$schema": "https://w3id.org/osd/0.7/profile.schema.json"});
    let d = osd::read(&profile.to_string(), Kind::Profile).unwrap();
    assert!(osd::warnings(&d).is_empty());
}
