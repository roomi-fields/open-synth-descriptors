//! The list of formats and the contract each one fulfils (docs/CADRE.md R18): a format that
//! exists here only is added by its module, `tests/test_only.rs`, and its one line.

use crate::Format as Listed;
use osd::{Resolved, Source};

formats! {
    TestOnly => test_only: "A format that exists in this test only.",
}

fn source() -> Source {
    Source {
        kind: "community-re".to_string(),
        url: Some("https://example.org/test-only".to_string()),
        license: Some("CC0-1.0".to_string()),
        ..Default::default()
    }
}

fn resolved(device: osd::Device) -> Resolved {
    Resolved { device, files: vec![], capabilities: vec![], inherited: vec![] }
}

#[test]
fn a_format_added_by_its_one_line_is_in_the_list_and_found_by_its_name() {
    assert_eq!(Format::ALL, [Format::TestOnly]);
    assert_eq!(Format::from_name("test-only"), Some(Format::TestOnly));
    assert_eq!(Format::from_name("test_only"), None);
    assert_eq!(Format::from_name("TestOnly"), None);
}

#[test]
fn a_format_added_by_its_one_line_says_what_its_module_declares() {
    let format = Format::TestOnly;
    assert_eq!(format.name(), "test-only");
    assert_eq!(format.media_type(), "application/x-test-only");
    assert_eq!(format.extension(), "tst");
    assert!(format.imports_into());
}

#[test]
fn a_format_added_by_its_one_line_imports_exports_and_adds_to_a_base_through_its_contract() {
    let contract = Format::TestOnly.contract();
    let imported = (contract.import)("Trundler", &source()).expect("imports");
    assert_eq!(imported.value.device.name, "Trundler");
    assert_eq!(imported.value.sources, Some(vec![source()]));
    let exported = (contract.export)(&resolved(imported.value.clone())).expect("exports");
    assert_eq!(exported.value, "Trundler");
    let add = contract.import_into.expect("adds a file to a base");
    let added = add(imported.value, "one more", &source()).expect("adds");
    assert_eq!(added.value.sources, Some(vec![source(), source()]));
    assert_eq!(added.lost.len(), 1);
    assert_eq!(added.lost[0].reason, "one more");
}

#[test]
fn each_listed_format_says_what_its_contract_says() {
    for format in Listed::ALL {
        let contract = format.contract();
        assert_eq!(format.name(), contract.name);
        assert_eq!(format.media_type(), contract.media_type, "{}", contract.name);
        assert_eq!(format.extension(), contract.extension, "{}", contract.name);
        assert_eq!(format.imports_into(), contract.import_into.is_some(), "{}", contract.name);
    }
}

#[test]
fn the_public_operations_give_what_the_contract_of_the_format_gives() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.7/device.schema.json", "device": {"name": "Trundler", "osc_prefix": "/trundler"},
        "sources": [{"type": "community-re", "url": "u", "license": "CC0-1.0"}]}"#;
    let Ok(osd::Descriptor::Device(device)) = osd::read(text, osd::Kind::Device) else { panic!("{text}") };
    for format in Listed::ALL {
        let contract = format.contract();
        let name = format.name();
        assert_eq!(crate::import("{", format, &source()), (contract.import)("{", &source()), "{name}");
        let device = resolved((*device).clone());
        assert_eq!(crate::export(&device, format), (contract.export)(&device), "{name}");
        if let Some(add) = contract.import_into {
            let added = add(device.device.clone(), "", &source());
            assert_eq!(crate::import_into(device.device, "", format, &source()), added, "{name}");
        }
    }
}
