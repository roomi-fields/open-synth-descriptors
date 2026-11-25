//! The `provider` of a source (0.7) through the formats: an import writes the one the caller
//! gives and guesses none from the format; a base joined by `import_into` is carried to 0.7 by
//! the converter's rule; the credit a text export carries names the provider after the type.

use crate::common::{self, csv, device_from, row, trigger_csv, trigger_row};
use osd_formats::{credit_to_carry, export, import, import_into, Format};
use serde_json::{json, Value};

fn controllers() -> String {
    csv(&[row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74")])])
}

fn drums() -> String {
    trigger_csv(&[trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "36")])])
}

/// One file of each format that is imported, with a source the import accepts.
fn samples() -> Vec<(Format, String, osd::Source)> {
    let preset = common::electra::preset(&[(1, "G")], vec![common::electra::control(1, "Cut", Some(1), common::electra::cc7(74), Some((0, 127)))]);
    let namespace = common::oscquery::namespace(vec![("cutoff", common::oscquery::method("/synth/cutoff", "i", &[Some((0, 127))]))]);
    let list = json!([{"ctrlType": "cc", "ctrlIndex": [74], "title": "Cut"}]);
    vec![
        (Format::MidiCsv, controllers(), common::source()),
        (Format::MidiTriggersCsv, drums(), common::source()),
        (Format::ElectraOne, preset.to_string(), common::electra::source()),
        (Format::Midnam, common::midnam::rich_text(), common::source()),
        (Format::OscQuery, namespace.to_string(), common::oscquery::source()),
        (Format::Midi2Controllers, list.to_string(), common::midi2::source()),
    ]
}

fn json_of(d: &osd::Device) -> Value {
    let text = osd::write(&osd::Descriptor::Device(Box::new(d.clone())));
    osd::read(&text, osd::Kind::Device).unwrap_or_else(|f| panic!("{f}\n{text}"));
    serde_json::from_str(&text).unwrap()
}

#[test]
fn an_import_writes_no_provider_the_caller_did_not_give() {
    for (format, text, source) in samples() {
        let t = import(&text, format, &source).unwrap_or_else(|f| panic!("{}: {f}", format.name()));
        let doc = json_of(&t.value);
        assert!(doc["sources"][0].get("provider").is_none(), "{}: {}", format.name(), doc["sources"]);
    }
}

#[test]
fn every_import_keeps_the_provider_the_caller_gives() {
    for (format, text, mut source) in samples() {
        source.provider = Some("acme-synths".to_string());
        let t = import(&text, format, &source).unwrap_or_else(|f| panic!("{}: {f}", format.name()));
        let doc = json_of(&t.value);
        assert_eq!(doc["sources"][0]["provider"], "acme-synths", "{}: {}", format.name(), doc["sources"]);
    }
}

#[test]
fn a_joined_file_adds_its_provider_to_its_own_source_only() {
    for (base, base_format, added, added_format) in
        [(controllers(), Format::MidiCsv, drums(), Format::MidiTriggersCsv), (drums(), Format::MidiTriggersCsv, controllers(), Format::MidiCsv)]
    {
        let base = import(&base, base_format, &common::source()).unwrap().value;
        let mut source = common::source();
        source.provider = Some("pencilresearch".to_string());
        let doc = json_of(&import_into(base.clone(), &added, added_format, &source).unwrap().value);
        assert!(doc["sources"][0].get("provider").is_none(), "{}", doc["sources"]);
        assert_eq!(doc["sources"][1]["provider"], "pencilresearch", "{}", doc["sources"]);
        let plain = json_of(&import_into(base, &added, added_format, &common::source()).unwrap().value);
        assert!(plain["sources"].as_array().unwrap().iter().all(|s| s.get("provider").is_none()), "{}", plain["sources"]);
    }
}

/// A device of `schema` (none for 0.4) whose credit is `sources` under the old key.
fn base_before_0_7(schema: Option<&str>, sources: Value) -> osd::Device {
    let mut doc = json!({"_sources": sources, "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}});
    if let Some(s) = schema {
        doc["$schema"] = json!(s);
    }
    device_from(&doc.to_string())
}

const BEFORE_0_7: [Option<&str>; 3] = [None, Some("https://w3id.org/osd/0.5/device.schema.json"), Some("https://w3id.org/osd/0.6/device.schema.json")];

#[test]
fn a_base_before_0_7_has_its_product_sources_said_by_the_converter_s_rule() {
    for schema in BEFORE_0_7 {
        let base = base_before_0_7(
            schema,
            json!([
                {"type": "electra-preset", "url": "u1", "license": "CC0-1.0", "notes": "kept\nlines"},
                {"type": "pencilresearch", "url": "u2"},
                {"type": "knobkraft", "url": "u3"},
                {"type": "midnam", "url": "u4"},
                {"type": "community-re", "url": "u5", "notes": "n"},
            ]),
        );
        for (text, format) in [(controllers(), Format::MidiCsv), (drums(), Format::MidiTriggersCsv)] {
            let mut source = common::source();
            source.provider = Some("acme-synths".to_string());
            let out = import_into(base.clone(), &text, format, &source).unwrap_or_else(|f| panic!("{f}")).value;
            assert_eq!(out.schema, Some(osd::Version::V0_7), "{schema:?}");
            let doc = json_of(&out);
            let list = doc["sources"].as_array().unwrap();
            assert_eq!(list.len(), 6, "{list:?}");
            assert_eq!(
                list[..5],
                [
                    json!({"type": "contributed", "provider": "electra-one", "url": "u1", "license": "CC0-1.0", "notes": "type in 0.6: electra-preset\nkept\nlines"}),
                    json!({"type": "contributed", "provider": "pencilresearch", "url": "u2", "notes": "type in 0.6: pencilresearch"}),
                    json!({"type": "contributed", "provider": "knobkraft", "url": "u3", "notes": "type in 0.6: knobkraft"}),
                    json!({"type": "contributed", "url": "u4", "notes": "type in 0.6: midnam"}),
                    json!({"type": "community-re", "url": "u5", "notes": "n"}),
                ],
                "{schema:?} {format:?}"
            );
            assert_eq!(list[5]["provider"], "acme-synths");
        }
    }
}

#[test]
fn a_base_before_0_7_keeps_a_well_formed_provider_and_sets_aside_another_value() {
    let base = base_before_0_7(
        BEFORE_0_7[2],
        json!([
            {"type": "knobkraft", "provider": "acme-synths"},
            {"type": "knobkraft", "provider": "Acme Synths"},
            {"type": "community-re", "provider": 7},
        ]),
    );
    let out = import_into(base, &controllers(), Format::MidiCsv, &common::source()).unwrap().value;
    let doc = json_of(&out);
    assert_eq!(
        doc["sources"].as_array().unwrap()[..3],
        [
            json!({"type": "contributed", "provider": "acme-synths", "notes": "type in 0.6: knobkraft"}),
            json!({"type": "contributed", "provider": "knobkraft", "_provider": "Acme Synths", "notes": "type in 0.6: knobkraft"}),
            json!({"type": "community-re", "_provider": 7}),
        ]
    );
}

#[test]
fn the_midi_of_sources_of_a_base_before_0_7_follow_the_same_rule() {
    let text = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [{"type": "community-re"}],
        "_midi_of_sources": [{"type": "pencilresearch"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
    });
    let out = import_into(device_from(&text.to_string()), &controllers(), Format::MidiCsv, &common::source()).unwrap().value;
    assert_eq!(json_of(&out)["midi_of_sources"], json!([{"type": "contributed", "provider": "pencilresearch", "notes": "type in 0.6: pencilresearch"}]));
}

// ---- the credit a file carries ----

/// A device whose two sources are alike but for the provider of the first.
fn credited() -> osd::Resolved {
    common::knobkraft::device(json!({
        "sources": [
            {"type": "contributed", "provider": "acme-synths", "url": "https://example.org/one", "author": "Dee", "license": "CC-BY-4.0"},
            {"type": "vendor-doc", "url": "https://example.org/two", "author": "Eve", "license": "MIT"},
        ],
        "cc_params": {"entries": [{"osc": "/filter/cutoff", "title": "Cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}]}]}
    }))
}

/// The provider follows the type of its source, and the source without one is credited as before.
fn assert_credit(what: &str, text: &str) {
    let kind = text.find("contributed").unwrap_or_else(|| panic!("{what}: no type in {text}"));
    let provider = text.find("acme-synths").unwrap_or_else(|| panic!("{what}: no provider in {text}"));
    assert!(provider > kind, "{what}: the provider comes after the type: {text}");
    assert!(provider < text.find("https://example.org/two").unwrap_or_else(|| panic!("{what}: no second source in {text}")), "{what}: {text}");
    assert!(text.contains("Eve, https://example.org/two, MIT (vendor-doc)"), "{what}: {text}");
    assert_eq!(text.matches("acme-synths").count(), 1, "{what}: {text}");
}

#[test]
fn the_credit_to_carry_names_the_provider_of_a_source_after_its_type() {
    let d = credited();
    let lost = export(&d, Format::MidiCsv).unwrap().lost;
    let lines = credit_to_carry(&d, &lost);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_credit("credit_to_carry", &lines.join("\n"));
    assert_eq!(lines[1], "Eve, https://example.org/two, MIT (vendor-doc)");
}

#[test]
fn the_text_of_the_deluge_knobkraft_midnam_and_oscquery_exports_names_the_provider_after_the_type() {
    let d = credited();
    for format in [Format::Deluge, Format::Knobkraft, Format::Midnam, Format::OscQuery] {
        let text = export(&d, format).unwrap_or_else(|f| panic!("{}: {f}", format.name())).value;
        assert_credit(format.name(), &text);
    }
}
