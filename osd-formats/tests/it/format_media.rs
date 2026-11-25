//! Each format says the media type and the extension of the file it writes.

use osd_formats::Format;

const EXPECTED: [(Format, &str, &str); 8] = [
    (Format::MidiCsv, "text/csv", "csv"),
    (Format::MidiTriggersCsv, "text/csv", "csv"),
    (Format::ElectraOne, "application/json", "epr"),
    (Format::Midnam, "application/xml", "midnam"),
    (Format::OscQuery, "application/json", "json"),
    (Format::Midi2Controllers, "application/json", "json"),
    (Format::Deluge, "application/xml", "xml"),
    (Format::Knobkraft, "text/x-python", "py"),
];

#[test]
fn each_format_gives_its_media_type_and_extension_from_the_table() {
    for (format, media_type, extension) in EXPECTED {
        assert_eq!(format.media_type(), media_type, "{}", format.name());
        assert_eq!(format.extension(), extension, "{}", format.name());
    }
}

#[test]
fn the_table_of_the_present_formats_is_in_order_the_head_of_the_list() {
    let listed: Vec<Format> = EXPECTED.iter().map(|(f, _, _)| *f).collect();
    assert_eq!(Format::ALL.get(..listed.len()), Some(listed.as_slice()));
}

#[test]
fn no_format_lacks_a_media_type_or_has_an_empty_or_dotted_extension() {
    for format in Format::ALL {
        let media_type = format.media_type();
        let (top, sub) = media_type.split_once('/').unwrap_or_else(|| panic!("{}: {media_type}", format.name()));
        assert!(!top.is_empty() && !sub.is_empty(), "{}: {media_type}", format.name());
        let extension = format.extension();
        assert!(!extension.is_empty(), "{}: empty extension", format.name());
        assert!(!extension.contains('.'), "{}: dotted extension {extension}", format.name());
    }
}

#[test]
fn the_interface_of_the_crate_names_media_type_and_extension() {
    let file = format!("{}/docs/INTERFACE.md", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
    for word in ["media_type", "extension"] {
        assert!(text.contains(word), "{file} does not name {word}");
    }
}
