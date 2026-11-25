//! `import_into` adds entries that carry a `title`, a field defined from 0.7: whatever version
//! the base names (0.4, 0.5, 0.6), the descriptor it returns names 0.7, is read by the reader
//! of osd, and writes the credit as `sources`.

use crate::common::{csv, row, source, trigger_csv, trigger_row};
use osd_formats::{import, import_into, Format};
use osd::Version;
use serde_json::Value;

fn controllers() -> String {
    csv(&[row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74")])])
}

fn drums() -> String {
    trigger_csv(&[trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "36")])])
}

fn base_of(version: Version, text: &str, format: Format) -> osd::Device {
    // A base written by an older reader: its entries carry no 0.7 field, its credit is `_sources`.
    // A descriptor of 0.4 declares no `$schema`: the reader gives it no version.
    let mut base = import(text, format, &source()).unwrap().value;
    base.schema = (version != Version::V0_4).then_some(version);
    base.sources = None;
    base.cc_params = None;
    base.triggers = None;
    base
}

fn text_of(d: &osd::Device) -> Value {
    serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(d.clone())))).unwrap()
}

#[test]
fn a_controller_file_joined_to_a_base_before_0_7_gives_a_0_7_descriptor_the_reader_reads() {
    for version in [Version::V0_4, Version::V0_5, Version::V0_6] {
        let base = base_of(version, &drums(), Format::MidiTriggersCsv);
        let out = import_into(base, &controllers(), Format::MidiCsv, &source()).unwrap().value;
        assert_eq!(out.schema, Some(Version::V0_7), "{version:?}");
        let doc = text_of(&out);
        assert_eq!(doc["$schema"], "https://w3id.org/osd/0.7/device.schema.json", "{version:?}");
        assert!(doc.get("sources").is_some() && doc.get("_sources").is_none(), "{version:?}: {doc}");
        osd::read(&doc.to_string(), osd::Kind::Device).unwrap_or_else(|f| panic!("{version:?}: {f}"));
    }
}

#[test]
fn a_triggers_file_joined_to_a_base_before_0_7_gives_a_0_7_descriptor_the_reader_reads() {
    for version in [Version::V0_4, Version::V0_5, Version::V0_6] {
        let base = base_of(version, &controllers(), Format::MidiCsv);
        let out = import_into(base, &drums(), Format::MidiTriggersCsv, &source()).unwrap().value;
        assert_eq!(out.schema, Some(Version::V0_7), "{version:?}");
        let doc = text_of(&out);
        assert!(doc.get("sources").is_some() && doc.get("_sources").is_none(), "{version:?}: {doc}");
        osd::read(&doc.to_string(), osd::Kind::Device).unwrap_or_else(|f| panic!("{version:?}: {f}"));
    }
}

#[test]
fn a_file_joined_to_a_base_of_0_7_stays_0_7() {
    let base = import(&drums(), Format::MidiTriggersCsv, &source()).unwrap().value;
    let out = import_into(base, &controllers(), Format::MidiCsv, &source()).unwrap().value;
    assert_eq!(out.schema, Some(Version::V0_7));
}
