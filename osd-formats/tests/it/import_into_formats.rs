//! Which formats add a file to a base: `Format::imports_into`, and the refusal of
//! `import_into` for the others (docs/INTERFACE.md).

use crate::common::{device_from, source};
use osd_formats::{import_into, FaultKind, Format};

fn base() -> osd::Device {
    device_from(
        r#"{"$schema": "https://w3id.org/osd/0.7/device.schema.json", "sources": [{"type": "community-re", "url": "u", "license": "CC0-1.0"}],
            "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}}"#,
    )
}

/// Whether `text` holds `name` as a whole word, a hyphen being part of a word.
fn names(text: &str, name: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).any(|w| w == name)
}

#[test]
fn of_the_present_formats_only_the_two_csv_add_a_file_to_a_base() {
    let adding: Vec<Format> = Format::ALL.into_iter().take(8).filter(|f| f.imports_into()).collect();
    assert_eq!(adding, [Format::MidiCsv, Format::MidiTriggersCsv]);
}

#[test]
fn a_format_imports_into_a_base_exactly_when_import_into_does_not_refuse_it_at_format() {
    for format in Format::ALL {
        let refused_for_its_format = import_into(base(), "", format, &source()).is_err_and(|f| f.place == "format");
        assert_eq!(format.imports_into(), !refused_for_its_format, "{}", format.name());
    }
}

#[test]
fn import_into_a_format_that_adds_nothing_is_an_invalid_file_naming_it_and_each_format_that_can() {
    let others: Vec<Format> = Format::ALL.into_iter().filter(|f| !f.imports_into()).collect();
    assert!(others.len() >= 6, "{others:?}");
    for format in others {
        let f = import_into(base(), "{}", format, &source()).expect_err("refused");
        assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "format"), "{}", format.name());
        for other in Format::ALL {
            assert_eq!(
                names(&f.rule, other.name()),
                other == format || other.imports_into(),
                "the rule for {} and the name {}: {:?}",
                format.name(),
                other.name(),
                f.rule
            );
        }
    }
}

#[test]
fn the_interface_of_the_crate_lists_imports_into() {
    let file = format!("{}/docs/INTERFACE.md", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
    assert!(text.contains("imports_into"), "{file} does not list imports_into");
}
