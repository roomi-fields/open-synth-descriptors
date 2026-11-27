//! The date of an import the caller gives (`Source.imported_at`): 0.7 has no field for it, so
//! every format that imports writes it on the source added as the note for people `_imported_at`
//! (`_imported_at_2`, then the next number, when the key is taken), by the rule of the converter of
//! `osd` (osd-formats/docs/CADRE.md R4, osd-formats/docs/CADRE.md R8,
//! osd-formats/docs/CADRE.md R15). Nothing is lost, and a source without a date gains no key.

use crate::common::{self, csv, device_from, row, trigger_csv, trigger_row};
use osd_formats::{import, import_into, Format, Translated};
use osd::{Kind, Version};
use serde_json::{json, Value};

const DATE: &str = "2026-01-02T03:04:05Z";

/// Values a caller can give as a date: the importer keeps the text, it does not judge it.
const DATES: [&str; 3] = [DATE, "", "  le 2 janvier à midi — 12:00  "];

/// The date the sources of a base before 0.7 hold.
const BASE_DATE: &str = "2025-06-07";

/// A controller file of one control, CC 74, and a row that is no control: one loss of the file.
fn controllers() -> String {
    csv(&[
        row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Resonance"), ("cc_msb", "74")]),
        row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Nothing")]),
    ])
}

/// A triggers file of two triggers, the second naming another maker: one loss of the file.
fn drums() -> String {
    trigger_csv(&[
        trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "36")]),
        trigger_row(&[("manufacturer", "Other"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Snare"), ("midi_channel", "10"), ("note_number_min_value", "38"), ("note_number_max_value", "38")]),
    ])
}

/// One file of each format that imports, each saying something the import cannot carry, with a
/// source the import accepts, without a date.
pub(crate) fn samples() -> Vec<(Format, String, osd::Source)> {
    let mut cutoff = common::oscquery::method("/synth/cutoff", "i", &[Some((0, 127))]);
    cutoff["CRITICAL"] = json!(true);
    let namespace = common::oscquery::namespace(vec![("cutoff", cutoff)]);
    let list = json!([{"ctrlType": "cc", "ctrlIndex": [74], "title": "Cut"}]);
    let preset = common::electra::preset(
        &[],
        vec![
            common::electra::control(1, "Cut", None, common::electra::cc7(74), None),
            common::electra::control(2, "Bend", None, common::electra::other("pitchbend"), None),
        ],
    );
    vec![
        (Format::MidiCsv, controllers(), common::source()),
        (Format::MidiTriggersCsv, drums(), common::source()),
        (Format::ElectraOne, preset.to_string(), common::electra::source()),
        (Format::Midnam, common::midnam::rich_text(), common::source()),
        (Format::OscQuery, namespace.to_string(), common::oscquery::source()),
        (Format::Midi2Controllers, list.to_string(), common::midi2::source()),
    ]
}

/// The two files that can be added to a base.
fn joinable() -> [(String, Format); 2] {
    [(controllers(), Format::MidiCsv), (drums(), Format::MidiTriggersCsv)]
}

/// `source` with every other key a caller can give.
fn with_every_key(mut source: osd::Source) -> osd::Source {
    source.id = Some("imported".to_string());
    source.provider = Some("pencilresearch".to_string());
    source.firmware = Some("1.2".to_string());
    source.attribution = Some("After the maker's table".to_string());
    source.commit = Some("215c64e8d3b3".to_string());
    source.notes = Some("Read by hand".to_string());
    source.extra.insert("revision".to_string(), json!("r42"));
    source
}

fn dated_with(mut source: osd::Source, date: &str) -> osd::Source {
    source.imported_at = Some(date.to_string());
    source
}

fn dated(source: osd::Source) -> osd::Source {
    dated_with(source, DATE)
}

/// `source` already holding these keys.
fn holding(mut source: osd::Source, held: &Value) -> osd::Source {
    for (key, value) in held.as_object().unwrap() {
        source.extra.insert(key.clone(), value.clone());
    }
    source
}

/// The written text of a device, which the reader of osd reads back, in 0.7.
fn read_back(d: &osd::Device, context: &str) -> Value {
    let text = osd::write(&osd::Descriptor::Device(Box::new(d.clone())));
    osd::read(&text, Kind::Device).unwrap_or_else(|f| panic!("{context}: the written text is refused: {f}\n{text}"));
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["$schema"], "https://w3id.org/osd/0.7/device.schema.json", "{context}");
    doc
}

fn imported(format: Format, text: &str, source: &osd::Source) -> Translated<osd::Device> {
    import(text, format, source).unwrap_or_else(|f| panic!("{}: {f}", format.name()))
}

fn joined(base: &osd::Device, format: Format, text: &str, source: &osd::Source) -> Translated<osd::Device> {
    import_into(base.clone(), text, format, source).unwrap_or_else(|f| panic!("{}: {f}", format.name()))
}

/// The device an import without a date returns, with `date` under `key` on its last source.
fn with_note(plain: &osd::Device, key: &str, date: &str) -> osd::Device {
    let mut out = plain.clone();
    let added = out.sources.as_mut().and_then(|s| s.last_mut()).expect("an import adds a source");
    assert!(!added.extra.contains_key(key), "the key {key} is already taken: {added:?}");
    added.extra.insert(key.to_string(), json!(date));
    out
}

/// The keys of a written source that begin with `_imported_at`, with their values.
fn notes_of(source: &Value) -> serde_json::Map<String, Value> {
    source.as_object().unwrap().iter().filter(|(k, _)| k.starts_with("_imported_at")).map(|(k, v)| (k.clone(), v.clone())).collect()
}

/// What a source already holds, and the key the date then takes.
fn taken() -> Vec<(Value, &'static str)> {
    vec![
        (json!({}), "_imported_at"),
        (json!({"_imported_at": "2025-12-01"}), "_imported_at_2"),
        (json!({"_imported_at": DATE}), "_imported_at_2"),
        (json!({"_imported_at": "a", "_imported_at_2": "b"}), "_imported_at_3"),
        (json!({"_imported_at_2": "later"}), "_imported_at"),
        (json!({"_imported_at": "a", "_imported_at_3": "c"}), "_imported_at_2"),
        (json!({"_imported_at": null, "_imported_at_2": 7, "_imported_at_3": {"x": 1}}), "_imported_at_4"),
    ]
}

// ---- import -----------------------------------------------------------------------------

#[test]
fn the_samples_cover_every_format_that_imports() {
    let covered: Vec<Format> = samples().into_iter().map(|(f, _, _)| f).collect();
    for format in Format::ALL {
        if !covered.contains(&format) {
            let refused = import("{}", format, &common::source()).expect_err(format.name());
            assert_eq!(refused.kind, osd_formats::FaultKind::InvalidFile, "{}", format.name());
            assert!(["deluge", "knobkraft"].contains(&format.name()), "{} has no sample here", format.name());
        }
    }
}

#[test]
fn every_import_writes_the_date_of_the_caller_as_the_note_imported_at_of_the_source() {
    for (format, text, source) in samples() {
        for date in DATES {
            let context = format!("{} with {date:?}", format.name());
            let t = imported(format, &text, &dated_with(source.clone(), date));
            let held = &t.value.sources.as_ref().unwrap()[0];
            assert_eq!(held.imported_at, None, "{context}");
            assert_eq!(held.extra.get("_imported_at"), Some(&json!(date)), "{context}");
            let doc = read_back(&t.value, &context);
            let written = &doc["sources"][0];
            assert_eq!(notes_of(written), *json!({"_imported_at": date}).as_object().unwrap(), "{context}");
            assert!(written.get("imported_at").is_none(), "{context}: {written}");
        }
    }
}

#[test]
fn every_import_of_a_dated_source_returns_the_import_without_a_date_and_the_note() {
    for (format, text, source) in samples() {
        for source in [source.clone(), with_every_key(source)] {
            let plain = imported(format, &text, &source);
            assert!(!plain.lost.is_empty(), "{}: the file of the test loses nothing", format.name());
            let t = imported(format, &text, &dated(source));
            assert_eq!(t.lost, plain.lost, "{}", format.name());
            assert_eq!(t.value, with_note(&plain.value, "_imported_at", DATE), "{}", format.name());
        }
    }
}

#[test]
fn every_import_writes_the_other_keys_of_a_dated_source() {
    for (format, text, source) in samples() {
        let source = with_every_key(source);
        let t = imported(format, &text, &dated(source.clone()));
        let doc = read_back(&t.value, format.name());
        let written = &doc["sources"][0];
        for (key, value) in [
            ("type", source.kind.as_str()),
            ("id", "imported"),
            ("provider", "pencilresearch"),
            ("url", source.url.as_deref().unwrap()),
            ("firmware", "1.2"),
            ("author", source.author.as_deref().unwrap()),
            ("attribution", "After the maker's table"),
            ("license", source.license.as_deref().unwrap()),
            ("commit", "215c64e8d3b3"),
            ("notes", "Read by hand"),
            ("revision", "r42"),
            ("_imported_at", DATE),
        ] {
            assert_eq!(written[key], value, "{}: {key} in {written}", format.name());
        }
    }
}

#[test]
fn a_note_the_source_already_holds_stays_and_the_date_takes_the_first_free_key() {
    for (format, text, source) in samples() {
        for (held, key) in taken() {
            let context = format!("{} holding {held}", format.name());
            let source = holding(source.clone(), &held);
            let plain = imported(format, &text, &source);
            let t = imported(format, &text, &dated(source));
            assert_eq!(t.lost, plain.lost, "{context}");
            assert_eq!(t.value, with_note(&plain.value, key, DATE), "{context}");
            let mut want = held.as_object().unwrap().clone();
            want.insert(key.to_string(), json!(DATE));
            let doc = read_back(&t.value, &context);
            assert_eq!(notes_of(&doc["sources"][0]), want, "{context}");
        }
    }
}

/// The notes `osd::convert_to` 0.7 writes from a 0.6 descriptor holding this source with its date.
fn notes_of_the_converter(held: &Value, date: &str) -> serde_json::Map<String, Value> {
    let mut source = json!({"type": "community-re", "url": "u", "license": "CC0-1.0", "imported_at": date});
    source.as_object_mut().unwrap().extend(held.as_object().unwrap().clone());
    let doc = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "_sources": [source],
    });
    let text = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{doc}"));
    let out: Value = serde_json::from_str(&text).unwrap();
    notes_of(&out["sources"][0])
}

#[test]
fn an_import_writes_the_notes_the_converter_of_osd_writes_for_the_same_source() {
    for (format, text, source) in samples() {
        for (held, _) in taken() {
            for date in DATES {
                let context = format!("{} holding {held} with {date:?}", format.name());
                let t = imported(format, &text, &dated_with(holding(source.clone(), &held), date));
                let doc = read_back(&t.value, &context);
                assert_eq!(notes_of(&doc["sources"][0]), notes_of_the_converter(&held, date), "{context}");
            }
        }
    }
}

#[test]
fn an_import_of_a_source_without_a_date_adds_no_note_and_no_loss() {
    for (format, text, source) in samples() {
        for (held, _) in taken() {
            let context = format!("{} holding {held}", format.name());
            let t = imported(format, &text, &holding(source.clone(), &held));
            assert!(t.lost.iter().all(|l| !l.place.contains("imported_at")), "{context}: {:?}", t.lost);
            let doc = read_back(&t.value, &context);
            assert_eq!(notes_of(&doc["sources"][0]), *held.as_object().unwrap(), "{context}");
            assert!(doc["sources"][0].get("imported_at").is_none(), "{context}: {}", doc["sources"]);
        }
    }
}

// ---- import_into ------------------------------------------------------------------------

/// A base of `version` read by osd, that already has `count` sources.
fn base_of(version: Version, count: usize) -> osd::Device {
    let sources: Vec<Value> = (0..count)
        .map(|n| {
            let mut s = json!({"type": "community-re", "url": format!("u{n}"), "license": "CC0-1.0"});
            if version < Version::V0_7 {
                // The conversion of a base before 0.7 keeps this date as the note `_imported_at`.
                s["imported_at"] = json!(BASE_DATE);
            }
            s
        })
        .collect();
    let mut doc = json!({"device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}});
    let credit = if version < Version::V0_7 { "_sources" } else { "sources" };
    doc[credit] = Value::Array(sources);
    match version {
        Version::V0_4 => {}
        Version::V0_5 => doc["$schema"] = json!("https://w3id.org/osd/0.5/device.schema.json"),
        Version::V0_6 => doc["$schema"] = json!("https://w3id.org/osd/0.6/device.schema.json"),
        _ => doc["$schema"] = json!("https://w3id.org/osd/0.7/device.schema.json"),
    }
    device_from(&doc.to_string())
}

/// The sources the base carries once said in 0.7 by the converter of osd.
fn sources_in_0_7(base: &osd::Device) -> Vec<Value> {
    let text = osd::write(&osd::Descriptor::Device(Box::new(base.clone())));
    let text = if base.schema >= Some(Version::V0_7) {
        text
    } else {
        osd::convert_to(&text, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("the chain refuses the base: {f}"))
    };
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["sources"].as_array().expect("the base has sources").clone()
}

fn bases() -> Vec<(Version, usize)> {
    [Version::V0_4, Version::V0_5, Version::V0_6, Version::V0_7].into_iter().flat_map(|v| [(v, 1), (v, 3)]).collect()
}

#[test]
fn a_dated_source_joined_to_a_base_is_added_at_its_rank_with_the_note_imported_at() {
    for (version, count) in bases() {
        for (text, format) in joinable() {
            for date in DATES {
                let context = format!("{} on {count} sources of {version:?} with {date:?}", format.name());
                let base = base_of(version, count);
                let source = with_every_key(common::source());
                let t = joined(&base, format, &text, &dated_with(source.clone(), date));
                let sources = t.value.sources.as_ref().unwrap();
                assert_eq!(sources.len(), count + 1, "{context}: {sources:?}");
                assert_eq!(sources[count], holding(source, &json!({"_imported_at": date})), "{context}");
                let doc = read_back(&t.value, &context);
                let written = &doc["sources"][count];
                assert_eq!(written["_imported_at"], date, "{context}: {written}");
                assert!(written.get("imported_at").is_none(), "{context}: {written}");
                for (key, value) in [("id", "imported"), ("license", "CC-BY-SA-4.0"), ("notes", "Read by hand"), ("revision", "r42")] {
                    assert_eq!(written[key], value, "{context}: {key} in {written}");
                }
            }
        }
    }
}

#[test]
fn a_date_joined_to_a_base_changes_neither_the_losses_nor_the_sources_of_the_base() {
    for (version, count) in bases() {
        for (text, format) in joinable() {
            let context = format!("{} on {count} sources of {version:?}", format.name());
            let base = base_of(version, count);
            let plain = joined(&base, format, &text, &common::source());
            assert!(!plain.lost.is_empty(), "{context}: the file of the test loses nothing");
            let t = joined(&base, format, &text, &dated(common::source()));
            assert_eq!(t.lost, plain.lost, "{context}");
            assert_eq!(t.value, with_note(&plain.value, "_imported_at", DATE), "{context}");
            let doc = read_back(&t.value, &context);
            assert_eq!(doc["sources"].as_array().unwrap()[..count], sources_in_0_7(&base)[..], "{context}");
        }
    }
}

#[test]
fn a_note_of_a_source_joined_to_a_base_stays_and_the_date_takes_the_first_free_key() {
    for (version, count) in bases() {
        for (text, format) in joinable() {
            for (held, key) in taken() {
                let context = format!("{} holding {held} on {count} sources of {version:?}", format.name());
                let base = base_of(version, count);
                let source = holding(common::source(), &held);
                let plain = joined(&base, format, &text, &source);
                let t = joined(&base, format, &text, &dated(source));
                assert_eq!(t.lost, plain.lost, "{context}");
                assert_eq!(t.value, with_note(&plain.value, key, DATE), "{context}");
                let doc = read_back(&t.value, &context);
                assert_eq!(notes_of(&doc["sources"][count]), notes_of_the_converter(&held, DATE), "{context}");
            }
        }
    }
}

#[test]
fn a_source_without_a_date_joined_to_a_base_adds_no_note_and_no_loss() {
    for (version, count) in bases() {
        for (text, format) in joinable() {
            let context = format!("{} on {count} sources of {version:?}", format.name());
            let t = joined(&base_of(version, count), format, &text, &common::source());
            assert!(t.lost.iter().all(|l| !l.place.contains("imported_at")), "{context}: {:?}", t.lost);
            let doc = read_back(&t.value, &context);
            let added = &doc["sources"][count];
            assert!(added.get("imported_at").is_none() && notes_of(added).is_empty(), "{context}: {added}");
        }
    }
}

#[test]
fn the_date_of_a_base_before_0_7_comes_out_as_a_note_of_its_source() {
    for (version, count) in bases().into_iter().filter(|(v, _)| *v < Version::V0_7) {
        for (text, format) in joinable() {
            for source in [common::source(), dated(common::source())] {
                let context = format!("{} on {count} sources of {version:?}", format.name());
                let t = joined(&base_of(version, count), format, &text, &source);
                let doc = read_back(&t.value, &context);
                for n in 0..count {
                    let want = json!({"type": "community-re", "url": format!("u{n}"), "license": "CC0-1.0", "_imported_at": BASE_DATE});
                    assert_eq!(doc["sources"][n], want, "{context}");
                    let held = &t.value.sources.as_ref().unwrap()[n];
                    assert_eq!(held.imported_at, None, "{context}");
                    assert_eq!(held.extra.get("_imported_at"), Some(&json!(BASE_DATE)), "{context}");
                }
            }
        }
    }
}

#[test]
fn a_base_that_already_holds_the_note_keeps_it_and_its_date_takes_the_next_key() {
    for schema in [None, Some("https://w3id.org/osd/0.5/device.schema.json"), Some("https://w3id.org/osd/0.6/device.schema.json")] {
        let mut doc = json!({
            "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
            "_sources": [{"type": "community-re", "url": "u", "license": "CC0-1.0", "imported_at": BASE_DATE, "_imported_at": "2024-01-01"}],
        });
        if let Some(s) = schema {
            doc["$schema"] = json!(s);
        }
        let base = device_from(&doc.to_string());
        for (text, format) in joinable() {
            let context = format!("{} on {schema:?}", format.name());
            let t = joined(&base, format, &text, &common::source());
            let written = read_back(&t.value, &context);
            let want = json!({"type": "community-re", "url": "u", "license": "CC0-1.0", "_imported_at": "2024-01-01", "_imported_at_2": BASE_DATE});
            assert_eq!(written["sources"][0], want, "{context}");
        }
    }
}

// ---- the pages --------------------------------------------------------------------------

fn page(name: &str) -> String {
    let path = format!("{}/docs/{name}.md", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn the_page_of_every_format_that_imports_says_the_note_and_no_loss_at_the_date() {
    for (format, _, _) in samples() {
        let text = page(format.name());
        assert!(text.contains("`_imported_at`"), "{}.md does not say `_imported_at`", format.name());
        assert!(!text.contains("/imported_at`"), "{}.md still names a loss at the date", format.name());
    }
}

#[test]
fn the_interface_says_the_note_for_import_for_import_into_and_for_the_pages_of_the_formats() {
    let text = page("INTERFACE");
    for start in ["| `import` |", "| `import_into` |", "The fate of each column"] {
        let line = text.lines().find(|l| l.starts_with(start)).unwrap_or_else(|| panic!("no line {start:?}"));
        assert!(line.contains("`_imported_at`"), "{start}: {line}");
        assert!(!line.contains("/imported_at`"), "{start}: {line}");
    }
}
