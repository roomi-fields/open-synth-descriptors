//! `import_into` on a base before 0.7 converts the base by the converter chain of `osd`
//! (`write`, `convert_to` 0.7, `read`) before it adds the file: the blocks of the base come out as
//! the chain gives them, and a base the chain or the reader refuses is `NotConvertible`.

use crate::common::{csv, device_from, row, source, trigger_csv, trigger_row};
use osd_formats::{import_into, Fault, Format, Translated};
use osd::{Kind, Version};
use serde_json::{json, Value};

const SCHEMA_0_5: &str = "https://w3id.org/osd/0.5/device.schema.json";
const SCHEMA_0_6: &str = "https://w3id.org/osd/0.6/device.schema.json";
const SCHEMA_0_7: &str = "https://w3id.org/osd/0.7/device.schema.json";

/// A controller file of one control, CC 74.
fn controllers() -> String {
    csv(&[row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Resonance"), ("cc_msb", "74")])])
}

/// A triggers file of one trigger.
fn drums() -> String {
    trigger_csv(&[trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "36")])])
}

fn files() -> [(String, Format); 2] {
    [(controllers(), Format::MidiCsv), (drums(), Format::MidiTriggersCsv)]
}

fn written(d: &osd::Device) -> String {
    osd::write(&osd::Descriptor::Device(Box::new(d.clone())))
}

fn json_of(d: &osd::Device) -> Value {
    serde_json::from_str(&written(d)).unwrap()
}

/// The base as the chain of `osd` gives it in 0.7.
fn converted_by_osd(base: &osd::Device) -> Value {
    let text = osd::convert_to(&written(base), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("the chain refuses the fixture: {f}"));
    json_of(&device_from(&text))
}

fn joined(base: &osd::Device, text: &str, format: Format) -> Translated<osd::Device> {
    import_into(base.clone(), text, format, &source()).unwrap_or_else(|f| panic!("{format:?}: {f}"))
}

/// The joined descriptor without what the file added: the last entry of its table and the last
/// source. A table the file created is removed.
fn without_the_additions(mut doc: Value, format: Format, base: &Value) -> Value {
    let table = if format == Format::MidiCsv { "cc_params" } else { "triggers" };
    if base.get(table).is_none() {
        doc.as_object_mut().unwrap().remove(table);
    } else {
        assert!(doc[table]["entries"].as_array_mut().unwrap().pop().is_some());
    }
    assert!(doc["sources"].as_array_mut().unwrap().pop().is_some());
    if base.get("sources").is_none() {
        doc.as_object_mut().unwrap().remove("sources");
    }
    doc
}

/// Joins each of the two files to `base`: the result reads in 0.7, and is the base as the chain
/// gives it, plus what the file adds; nothing the chain removed is a loss of the file.
fn assert_joined_as_the_chain_converts(base: &osd::Device) {
    let expected = converted_by_osd(base);
    for (text, format) in files() {
        let out = joined(base, &text, format);
        assert_eq!(out.value.schema, Some(Version::V0_7), "{format:?}");
        let doc = json_of(&out.value);
        assert_eq!(doc["$schema"], SCHEMA_0_7, "{format:?}");
        osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{format:?}: the reader refuses the result: {f}\n{doc}"));
        assert_eq!(without_the_additions(doc, format, &expected), expected, "{format:?}");
        assert!(out.lost.is_empty(), "{format:?}: {:?}", out.lost);
    }
}

/// A base of 0.4 with the words 0.7 no longer takes, a block of each kind the chain changes, and a
/// `_midi_of_sources` that reads as sources.
fn base_0_4() -> osd::Device {
    device_from(
        &json!({
            "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler", "manufacturer_id": [66]},
            "_sources": [
                {"type": "vendor", "url": "u1", "license": "none"},
                {"type": "agent-contributed", "url": "u2", "license": "CC0-1.0"},
                {"type": "hardware-verified-partial", "url": "u3"},
                {"type": "knobkraft", "url": "u4"},
            ],
            "_midi_of_sources": [{"type": "pencilresearch", "url": "m1"}],
            "sysex": {"header": [240], "footer": [247]},
            "cc_params": {"entries": [{"osc": "/filter/cutoff", "cc": 20, "_name": "Cutoff", "transform_reverse": "return value"}]},
        })
        .to_string(),
    )
}

fn base_0_5() -> osd::Device {
    device_from(
        &json!({
            "$schema": SCHEMA_0_5,
            "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler", "manufacturer_id": [66]},
            "_sources": [{"type": "electra-preset", "url": "u1", "license": "CC0-1.0", "imported_at": "2026-01-01"}],
            "sysex": {"header": [240], "footer": [247]},
            "cc_params": {"entries": [{"osc": "/filter/cutoff", "cc": 20, "_name": "Cutoff"}, {"osc": "/filter/drive", "nrpn_msb": 1, "nrpn_lsb": 2}]},
        })
        .to_string(),
    )
}

fn base_0_6() -> osd::Device {
    device_from(
        &json!({
            "$schema": SCHEMA_0_6,
            "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler", "manufacturer_id": [66], "revision": "B"},
            "capabilities": ["lua"],
            "_sources": [{"type": "pencilresearch", "url": "u1", "license": "CC0-1.0"}],
            "_midi_of_sources": [{"type": "knobkraft", "url": "m1"}],
            "sysex": {"header": [240], "footer": [247]},
            "cc_params": {"entries": [{"osc": "/filter/cutoff", "cc": 20, "_name": "Cutoff", "transform_reverse": "return value"}]},
            "triggers": {"entries": [{"osc": "/pads/snare", "sound": "Snare", "note_range": [38, 38], "_name": "Snare"}]},
        })
        .to_string(),
    )
}

#[test]
fn a_base_of_0_4_comes_out_as_the_chain_converts_it_with_the_file_added() {
    assert_joined_as_the_chain_converts(&base_0_4());
}

#[test]
fn a_base_of_0_5_comes_out_as_the_chain_converts_it_with_the_file_added() {
    assert_joined_as_the_chain_converts(&base_0_5());
}

#[test]
fn a_base_of_0_6_comes_out_as_the_chain_converts_it_with_the_file_added() {
    assert_joined_as_the_chain_converts(&base_0_6());
}

#[test]
fn the_source_types_and_the_licence_of_a_base_of_0_4_are_translated_as_the_chain_translates_them() {
    for (text, format) in files() {
        let doc = json_of(&joined(&base_0_4(), &text, format).value);
        let sources = doc["sources"].as_array().unwrap();
        let types: Vec<&str> = sources[..3].iter().map(|s| s["type"].as_str().unwrap()).collect();
        assert_eq!(types, ["vendor-doc", "contributed", "hardware-verified"], "{format:?}");
        assert_eq!(sources[0]["license"], "NONE", "{format:?}");
        assert_eq!(sources[1]["license"], "CC0-1.0", "{format:?}");
        assert_eq!(sources.len(), 5, "{format:?}");
        assert!(doc.get("_sources").is_none() && doc.get("_midi_of_sources").is_none(), "{doc}");
    }
}

#[test]
fn the_fields_0_7_moves_or_removes_leave_the_base_without_a_loss() {
    let out = joined(&base_0_6(), &controllers(), Format::MidiCsv);
    let doc = json_of(&out.value);
    assert!(doc["identity"].get("manufacturerId").is_some(), "{doc}");
    assert!(doc["device"].get("manufacturer_id").is_none() && doc["device"].get("revision").is_none(), "{doc}");
    assert!(doc["sysex"].get("footer").is_none(), "{doc}");
    assert_eq!(doc["cc_params"]["entries"][0]["title"], "Cutoff");
    assert!(doc["cc_params"]["entries"][0].get("_name").is_none(), "{doc}");
    assert!(doc["cc_params"]["entries"][0].get("transform_reverse").is_none(), "{doc}");
    assert!(out.lost.is_empty(), "{:?}", out.lost);
}

#[test]
fn only_what_the_file_could_not_say_is_a_loss_on_a_base_before_0_7() {
    // CC 20 is claimed by the control the base holds: the way of the file is the one loss.
    let file = csv(&[row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Again"), ("cc_msb", "20")])]);
    for base in [base_0_4(), base_0_5(), base_0_6()] {
        let out = joined(&base, &file, Format::MidiCsv);
        let places: Vec<&str> = out.lost.iter().map(|l| l.place.as_str()).collect();
        assert_eq!(places, ["line 2, column cc_msb"], "{:?}", base.schema);
    }
}

#[test]
fn a_base_of_0_7_comes_out_as_it_was_with_the_file_added() {
    let base = device_from(&converted_by_osd(&base_0_6()).to_string());
    let expected = json_of(&base);
    for (text, format) in files() {
        let out = joined(&base, &text, format);
        assert_eq!(without_the_additions(json_of(&out.value), format, &expected), expected, "{format:?}");
        assert!(out.lost.is_empty(), "{format:?}: {:?}", out.lost);
    }
}

#[test]
fn a_control_without_a_message_of_a_base_before_0_7_is_kept_with_an_empty_ways_and_its_fields() {
    for (schema, fields) in [(SCHEMA_0_5, json!({"range": [0, 3]})), (SCHEMA_0_6, json!({"range": [0, 3], "transmit": "absolute", "recognize": "none"}))] {
        let mut entry = json!({"osc": "/a"});
        entry.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
        let base = device_from(
            &json!({
                "$schema": schema,
                "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
                "cc_params": {"entries": [entry.clone()]},
            })
            .to_string(),
        );
        for (file, format) in files() {
            let out = joined(&base, &file, format);
            let mut kept = entry.clone();
            kept["ways"] = json!([]);
            assert_eq!(json_of(&out.value)["cc_params"]["entries"][0], kept, "{schema} {format:?}");
            assert!(out.lost.is_empty(), "{schema} {format:?}: {:?}", out.lost);
        }
    }
}

// ---- a base that does not convert -------------------------------------------------------

/// The refusal of both files joined to `base`, at `place`, repeating the rule `osd` gives for it.
fn assert_not_convertible(base: &osd::Device, place: &str) {
    let text = written(base);
    let given = match osd::convert_to(&text, Kind::Device, Version::V0_7) {
        Err(f) => f,
        Ok(converted) => osd::read(&converted, Kind::Device).expect_err("osd accepts the fixture"),
    };
    assert_eq!(given.pointer, place, "the fixture is refused elsewhere: {given}");
    for (file, format) in files() {
        let f: Fault = match import_into(base.clone(), &file, format, &source()) {
            Ok(t) => panic!("{format:?}: a descriptor is returned:\n{}", written(&t.value)),
            Err(f) => f,
        };
        assert_eq!(format!("{:?}", f.kind), "NotConvertible", "{format:?}: {f}");
        assert_eq!(f.place, place, "{format:?}: {f}");
        assert!(f.rule.contains(&given.rule), "{format:?}: the rule of osd ({}) is not in {:?}", given.rule, f.rule);
        let own = f.rule.replace(&given.rule, "").to_lowercase();
        for word in ["conver", "base", "version"] {
            assert!(own.contains(word), "{format:?}: the rule says the fault comes from converting the base, at a pointer that may name a key of the version reached ({word}): {:?}", f.rule);
        }
    }
}

fn base_0_4_with_sources(sources: Value, midi_of_sources: Option<Value>) -> osd::Device {
    let mut doc = json!({"device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}, "_sources": sources});
    if let Some(m) = midi_of_sources {
        doc["_midi_of_sources"] = m;
    }
    device_from(&doc.to_string())
}

#[test]
fn a_base_of_0_4_with_a_licence_the_chain_refuses_is_not_convertible_at_that_licence() {
    let first = base_0_4_with_sources(json!([{"type": "vendor", "url": "u", "license": "proprietary"}]), None);
    assert_not_convertible(&first, "/_sources/0/license");
    let second = base_0_4_with_sources(
        json!([{"type": "vendor", "url": "u", "license": "none"}, {"type": "community-re", "url": "v", "license": "proprietary"}]),
        None,
    );
    assert_not_convertible(&second, "/_sources/1/license");
}

#[test]
fn a_midi_of_sources_note_the_reader_of_0_7_refuses_makes_the_base_not_convertible() {
    let note = device_from(
        &json!({
            "$schema": SCHEMA_0_6,
            "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
            "_midi_of_sources": "taken from the maker's manual",
        })
        .to_string(),
    );
    assert_not_convertible(&note, "/midi_of_sources");
    // The converter from 0.4 translates the types of `_sources`, not those of this note.
    let word_of_0_4 = base_0_4_with_sources(json!([{"type": "community-re", "url": "u"}]), Some(json!([{"type": "vendor"}])));
    assert_not_convertible(&word_of_0_4, "/midi_of_sources/0/type");
}
