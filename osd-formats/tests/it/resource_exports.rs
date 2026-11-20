//! The closed list of export names that a device resource may carry (`osd::RESOURCE_EXPORTS`, the
//! `export` of a `resources` entry) is the list of the names of the formats: osd cannot read
//! `Format`, so this guard fails when the two differ.

use osd_formats::Format;

#[test]
fn the_export_names_a_device_resource_accepts_are_the_names_of_the_formats() {
    let mut accepted: Vec<&str> = osd::RESOURCE_EXPORTS.to_vec();
    let mut formats: Vec<&str> = Format::ALL.iter().map(|f| f.name()).collect();
    accepted.sort();
    formats.sort();
    assert_eq!(accepted, formats);
}

#[test]
fn every_export_name_a_device_resource_accepts_is_a_format_that_osd_formats_finds_by_name() {
    for name in osd::RESOURCE_EXPORTS {
        assert!(Format::from_name(name).is_some(), "{name} is no format");
    }
}

#[test]
fn the_interface_of_the_crate_describes_the_guard_of_the_export_names() {
    let file = format!("{}/docs/INTERFACE.md", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
    assert!(text.contains("RESOURCE_EXPORTS"), "{file} does not describe the guard of the export names");
}
