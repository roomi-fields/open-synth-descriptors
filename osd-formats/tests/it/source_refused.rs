//! A source the reader of 0.7 refuses in the descriptor written is refused by every import and by
//! `import_into`, as `SourceRefused` at `source/<field>`, its rule carrying the reader's; no
//! descriptor is returned (osd-formats/docs/CADRE.md R8). A source the reader accepts is carried
//! as given.

use crate::common::{self, csv, device_from, row, trigger_csv, trigger_row};
use crate::imported_at::samples;
use osd_formats::{import, import_into, Fault, FaultKind, Format, Translated};
use serde_json::json;

/// A change of a source.
type Change = Box<dyn Fn(&mut osd::Source)>;

/// The changes the reader of 0.7 refuses, each with the field at fault.
fn spoils() -> Vec<(&'static str, Change)> {
    let mut out: Vec<(&'static str, Change)> = Vec::new();
    let kinds = ["pencilresearch", "knobkraft", "electra-preset", "midnam", "blog", "", "x-acme", "X-acme-forum", "Community-Re"];
    for kind in kinds {
        out.push(("type", Box::new(move |s| s.kind = kind.to_string())));
    }
    for license in ["cc by", "None", "Public-Domain", "noassertion", "CC BY 4.0"] {
        out.push(("license", Box::new(move |s| s.license = Some(license.to_string()))));
    }
    for provider in ["Not Well Formed".to_string(), "-acme".to_string(), "acme--co".to_string(), String::new(), "a".repeat(65)] {
        out.push(("provider", Box::new(move |s| s.provider = Some(provider.clone()))));
    }
    out.push(("provider", Box::new(|s| drop(s.extra.insert("provider".to_string(), json!(7))))));
    out.push(("id", Box::new(|s| s.id = Some(String::new()))));
    out.push(("id", Box::new(|s| drop(s.extra.insert("id".to_string(), json!(5))))));
    out
}

/// A device of 0.7 with these sources and these sources of its target.
fn base(sources: &str, midi_of_sources: &str) -> osd::Device {
    device_from(&format!(
        r#"{{"$schema": "https://w3id.org/osd/0.7/device.schema.json", "sources": [{sources}], "midi_of_sources": [{midi_of_sources}],
        "device": {{"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}}}}"#
    ))
}

fn plain_base() -> osd::Device {
    base(r#"{"type": "vendor-doc", "url": "u", "license": "CC0-1.0"}"#, "")
}

/// A base of 0.4, which `import_into` first converts.
fn old_base() -> osd::Device {
    device_from(r#"{"device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}, "_sources": [{"type": "vendor", "url": "u", "license": "CC0-1.0"}]}"#)
}

fn joinable() -> [(Format, String); 2] {
    [
        (Format::MidiCsv, csv(&[row(&[("parameter_name", "Cutoff"), ("cc_msb", "74")])])),
        (Format::MidiTriggersCsv, trigger_csv(&[trigger_row(&[("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "36")])])),
    ]
}

/// The reader's rule for `device` with `source` added after its sources: what a descriptor written
/// with it would meet. A descriptor written carries the date of the import as the note
/// `_imported_at`, 0.7 having no field for it.
fn readers_rule(mut device: osd::Device, source: &osd::Source) -> String {
    let mut written = source.clone();
    if let Some(date) = written.imported_at.take() {
        written.extra.insert("_imported_at".to_string(), json!(date));
    }
    device.sources.get_or_insert_with(Vec::new).push(written);
    let text = osd::write(&osd::Descriptor::Device(Box::new(device)));
    match osd::read(&text, osd::Kind::Device) {
        Err(f) => f.rule,
        Ok(_) => panic!("the reader accepts this source: {source:?}"),
    }
}

fn assert_refused(result: Result<Translated<osd::Device>, Fault>, field: &str, rule: &str, context: &str) {
    let f = match result {
        Ok(t) => panic!("{context}: a descriptor is returned, with the sources {:?}", t.value.sources),
        Err(f) => f,
    };
    assert_eq!(format!("{:?}", f.kind), "SourceRefused", "{context}: {f}");
    assert_eq!(f.place, format!("source/{field}"), "{context}: {f}");
    assert!(f.rule.contains(rule), "{context}: the rule {:?} does not carry the reader's, {rule:?}", f.rule);
}

#[test]
fn every_import_refuses_a_source_the_reader_refuses_at_its_field_with_the_readers_rule() {
    for (format, text, source) in samples() {
        for (n, (field, spoil)) in spoils().iter().enumerate() {
            let mut bad = source.clone();
            spoil(&mut bad);
            let rule = readers_rule(plain_base(), &bad);
            assert_refused(import(&text, format, &bad), field, &rule, &format!("{} spoil {n}", format.name()));
        }
    }
}

#[test]
fn import_into_refuses_a_source_the_reader_refuses_whatever_the_version_of_the_base() {
    for (format, text) in joinable() {
        for (n, (field, spoil)) in spoils().iter().enumerate() {
            let mut bad = common::source();
            spoil(&mut bad);
            let rule = readers_rule(plain_base(), &bad);
            for (version, base) in [("0.7", plain_base()), ("0.4", old_base())] {
                assert_refused(import_into(base, &text, format, &bad), field, &rule, &format!("{} into {version}, spoil {n}", format.name()));
            }
        }
    }
}

#[test]
fn a_refused_source_that_carries_a_date_of_import_is_refused_the_same() {
    let mut bad = common::source();
    bad.kind = "pencilresearch".to_string();
    bad.imported_at = Some("2026-01-02".to_string());
    let (format, text) = &joinable()[0];
    let rule = readers_rule(plain_base(), &bad);
    assert_refused(import(text, *format, &bad), "type", &rule, "import");
    assert_refused(import_into(plain_base(), text, *format, &bad), "type", &rule, "import_into");
}

#[test]
fn import_into_refuses_a_source_whose_id_a_source_of_the_base_already_has() {
    let held = || {
        base(
            r#"{"type": "vendor-doc", "id": "maker", "url": "u", "license": "CC0-1.0"}, {"type": "community-re", "id": "second", "url": "v"}"#,
            r#"{"type": "community-re", "id": "target", "url": "w"}"#,
        )
    };
    for (format, text) in joinable() {
        for id in ["maker", "second", "target"] {
            let mut source = common::source();
            source.id = Some(id.to_string());
            let rule = readers_rule(held(), &source);
            assert_refused(import_into(held(), &text, format, &source), "id", &rule, &format!("{} with the id {id}", format.name()));
        }
    }
}

#[test]
fn import_into_takes_a_source_whose_id_no_source_of_the_base_has() {
    for (format, text) in joinable() {
        for id in [Some("partner"), None] {
            let held = base(r#"{"type": "vendor-doc", "id": "maker", "url": "u", "license": "CC0-1.0"}"#, "");
            let mut source = common::source();
            source.id = id.map(str::to_string);
            let t = import_into(held, &text, format, &source).unwrap_or_else(|f| panic!("{}: {f}", format.name()));
            assert_eq!(t.value.sources.as_ref().and_then(|s| s.last()), Some(&source), "{}", format.name());
        }
    }
}

/// Changes of a source that the reader of 0.7 accepts.
fn accepted() -> Vec<Change> {
    let mut out: Vec<Change> = Vec::new();
    let kinds = ["hardware-verified", "hardware-observed", "software-verified", "vendor-doc", "vendor-osc-api", "vendor-self-described", "third-party-osc", "community-re", "contributed", "test-fixture", "x-acme-forum"];
    for kind in kinds {
        out.push(Box::new(move |s| s.kind = kind.to_string()));
    }
    for license in ["public-domain", "NOASSERTION", "NONE", "MIT", "GPL-2.0+"] {
        out.push(Box::new(move |s| s.license = Some(license.to_string())));
    }
    for provider in ["pencilresearch".to_string(), "unknown-partner".to_string(), "a".repeat(64)] {
        out.push(Box::new(move |s| s.provider = Some(provider.clone())));
    }
    out.push(Box::new(|s| s.id = Some("partner".to_string())));
    out
}

#[test]
fn every_import_carries_a_source_the_reader_accepts_as_given() {
    for (format, text, source) in samples() {
        // What a format writes of its own on the source (a preset's checksum) is on each import.
        let plain = import(&text, format, &source).unwrap_or_else(|f| panic!("{}: {f}", format.name()));
        let written = plain.value.sources.and_then(|s| s.into_iter().next()).expect("an import adds a source");
        for (n, change) in accepted().iter().enumerate() {
            let (mut good, mut expected) = (source.clone(), written.clone());
            change(&mut good);
            change(&mut expected);
            let context = format!("{} change {n}", format.name());
            let t = import(&text, format, &good).unwrap_or_else(|f| panic!("{context}: {f}"));
            assert_eq!(t.value.sources, Some(vec![expected]), "{context}");
        }
    }
}

#[test]
fn every_import_and_import_into_carry_a_license_id_in_lower_case_as_written() {
    for license in ["curl", "mit", "free", "zlib-acknowledgement", "0BSD", "mit OR isc"] {
        for (format, text, mut source) in samples() {
            source.license = Some(license.to_string());
            let context = format!("{} {license}", format.name());
            let t = import(&text, format, &source).unwrap_or_else(|f| panic!("{context}: {f}"));
            assert_carried(t.value, license, &context);
        }
        for (format, text) in joinable() {
            let mut source = common::source();
            source.license = Some(license.to_string());
            for (version, base) in [("0.7", plain_base()), ("0.4", old_base())] {
                let context = format!("{} {license} into {version}", format.name());
                let t = import_into(base, &text, format, &source).unwrap_or_else(|f| panic!("{context}: {f}"));
                assert_carried(t.value, license, &context);
            }
        }
    }
}

/// The last source of `device` has `license`, and the descriptor written is read.
fn assert_carried(device: osd::Device, license: &str, context: &str) {
    let last = device.sources.as_ref().and_then(|s| s.last()).expect("an import adds a source");
    assert_eq!(last.license.as_deref(), Some(license), "{context}");
    let written = osd::write(&osd::Descriptor::Device(Box::new(device)));
    osd::read(&written, osd::Kind::Device).unwrap_or_else(|f| panic!("{context}: {f}"));
}

#[test]
fn a_source_without_credit_or_without_license_stays_incomplete_even_when_the_reader_would_refuse_it() {
    for (format, text, source) in samples() {
        let mut no_license = source.clone();
        no_license.kind = "pencilresearch".to_string();
        no_license.license = None;
        let mut no_credit = source.clone();
        no_credit.license = Some("cc by".to_string());
        (no_credit.url, no_credit.author, no_credit.attribution) = (None, None, None);
        for bad in [no_license, no_credit] {
            let f = import(&text, format, &bad).expect_err(format.name());
            assert_eq!(f.kind, FaultKind::SourceIncomplete, "{}: {f}", format.name());
        }
    }
}
