//! A device says what it can receive (0.6): `resources`, a list in the words of the ResourceList
//! of MIDI 2.0 Property Exchange (`resource`, `canSet`, `mediaTypes`), each with the `export`
//! that produces its content (a name of the closed list of formats) and, for a device that does
//! not speak Property Exchange, the `via` it arrives by: `file_write`, a path of segments (a fixed
//! text, or a `variable` of the identity of the device placed: `vendor`, `device`), and
//! `carried_by`. `resource_path` resolves the path from the target and the device to place.

use crate::common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn deluge() -> Value {
    json!({
        "resource": "X-DeviceDefinition",
        "canSet": "full",
        "mediaTypes": ["application/xml"],
        "export": "deluge",
        "via": {"file_write": {
            "path": ["MIDI_DEVICES", "DEFINITION", {"variable": "vendor"}, {"variable": "device", "suffix": ".xml"}],
            "carried_by": "sysex"
        }}
    })
}

fn doc(resources: Value) -> Value {
    json!({"$schema": SCHEMA, "device": {"name": "Deluge", "vendor": "Synthstrom Audible", "osc_prefix": "/deluge"}, "resources": resources})
}

fn read(doc: &Value) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn device(doc: &Value) -> osd::Device {
    match read(doc).unwrap_or_else(|f| panic!("{f}")) {
        osd::Descriptor::Device(d) => *d,
        _ => unreachable!(),
    }
}

fn written(d: &osd::Descriptor) -> Value {
    serde_json::from_str(&osd::write(d)).unwrap()
}

fn refused_at(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

fn schema_errors(doc: &Value) -> Vec<(String, String)> {
    common::Schemas::load("0.6").errors(Kind::Device, doc)
}

/// A device to place, by its identity.
fn source(vendor: Option<&str>, name: &str) -> osd::Device {
    let mut d = json!({"name": name, "osc_prefix": "/s"});
    if let Some(v) = vendor {
        d["vendor"] = json!(v);
    }
    device(&json!({"$schema": SCHEMA, "device": d}))
}

fn target() -> osd::Device {
    device(&doc(json!([deluge()])))
}

#[test]
fn a_device_declares_a_resource_in_the_words_of_property_exchange_and_reads_back_as_written() {
    let d = doc(json!([deluge()]));
    let r = read(&d).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(written(&r), d);
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
    let dev = device(&d);
    let list = dev.resources.as_ref().expect("resources");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].resource, "X-DeviceDefinition");
    assert_eq!(list[0].export, "deluge");
}

#[test]
fn a_device_declares_several_resources_and_each_can_set_word_of_property_exchange() {
    let d = doc(json!([
        {"resource": "X-A", "canSet": "none", "export": "midnam"},
        {"resource": "X-B", "canSet": "partial", "mediaTypes": ["application/json"], "export": "midi2-controllers"},
        deluge()
    ]));
    assert_eq!(written(&read(&d).unwrap()), d);
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
    assert_eq!(device(&d).resources.unwrap().len(), 3);
}

#[test]
fn a_resource_without_via_is_set_by_property_exchange_and_reads() {
    let d = doc(json!([{"resource": "X-Names", "canSet": "full", "export": "midnam"}]));
    assert_eq!(written(&read(&d).unwrap()), d);
    assert!(schema_errors(&d).is_empty());
}

#[test]
fn a_device_without_resources_reads_as_before() {
    let d = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}});
    let r = read(&d).unwrap();
    assert_eq!(written(&r), d);
    assert!(device(&d).resources.is_none());
    assert!(schema_errors(&d).is_empty());
}

#[test]
fn a_0_5_descriptor_refuses_resources_and_reads_without_them() {
    let with = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}, "resources": []});
    let f = osd::read(&with.to_string(), Kind::Device).expect_err("0.5 has no resources");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/resources"));
    let without = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}});
    assert!(osd::read(&without.to_string(), Kind::Device).is_ok());
}

#[test]
fn the_converter_from_0_5_adds_no_resources() {
    let text = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}}).to_string();
    let out = osd::convert_to(&text, Kind::Device, Version::V0_6).unwrap();
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("resources").is_none(), "{out}");
    assert_eq!(v["$schema"], SCHEMA);
}

#[test]
fn a_resource_name_of_its_own_starts_with_x_dash() {
    let mut r = deluge();
    r["resource"] = json!("DeviceDefinition");
    let d = doc(json!([r]));
    refused_at(&d, FaultKind::BadValue, "/resources/0/resource");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn can_set_is_none_partial_or_full_and_another_word_is_refused_at_the_field() {
    for word in ["write", "Full", "true", ""] {
        let mut r = deluge();
        r["canSet"] = json!(word);
        let d = doc(json!([r]));
        refused_at(&d, FaultKind::BadValue, "/resources/0/canSet");
        assert!(!schema_errors(&d).is_empty(), "{word}");
    }
}

#[test]
fn an_export_outside_the_closed_list_is_refused_at_the_field_in_the_reader_and_the_schema() {
    let mut r = deluge();
    r["export"] = json!("midi");
    let d = doc(json!([r]));
    refused_at(&d, FaultKind::BadValue, "/resources/0/export");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_resource_without_export_is_refused_as_missing() {
    let mut r = deluge();
    r.as_object_mut().unwrap().remove("export");
    let d = doc(json!([r]));
    refused_at(&d, FaultKind::Missing, "/resources/0/export");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn an_unknown_field_of_a_resource_is_refused_at_the_field() {
    let mut r = deluge();
    r["overwrite"] = json!(true);
    let d = doc(json!([r]));
    refused_at(&d, FaultKind::UnknownField, "/resources/0/overwrite");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_way_other_than_file_write_is_refused_at_its_key() {
    let mut r = deluge();
    r["via"] = json!({"ftp": {"path": ["a"]}});
    let d = doc(json!([r]));
    refused_at(&d, FaultKind::UnknownField, "/resources/0/via/ftp");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_file_write_path_has_one_segment_at_least() {
    let mut r = deluge();
    r["via"]["file_write"]["path"] = json!([]);
    let d = doc(json!([r]));
    refused_at(&d, FaultKind::BadValue, "/resources/0/via/file_write/path");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_path_variable_other_than_vendor_or_device_is_refused_at_the_variable() {
    let mut r = deluge();
    r["via"]["file_write"]["path"] = json!(["A", {"variable": "revision"}]);
    let d = doc(json!([r]));
    refused_at(&d, FaultKind::BadValue, "/resources/0/via/file_write/path/1/variable");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_path_given_as_one_template_string_is_refused() {
    let mut r = deluge();
    r["via"]["file_write"]["path"] = json!("MIDI_DEVICES/DEFINITION/{vendor}/{device}.xml");
    let d = doc(json!([r]));
    assert!(read(&d).is_err());
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_path_segment_with_an_unknown_key_is_refused() {
    let mut r = deluge();
    r["via"]["file_write"]["path"] = json!([{"variable": "vendor", "prefix": "x"}]);
    let d = doc(json!([r]));
    assert!(read(&d).is_err());
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn carried_by_is_sysex_and_another_word_is_refused_at_the_field() {
    let mut r = deluge();
    r["via"]["file_write"]["carried_by"] = json!("usb");
    let d = doc(json!([r]));
    refused_at(&d, FaultKind::BadValue, "/resources/0/via/file_write/carried_by");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn the_error_of_a_second_resource_names_its_own_place() {
    let mut bad = deluge();
    bad["canSet"] = json!("nope");
    refused_at(&doc(json!([deluge(), bad])), FaultKind::BadValue, "/resources/1/canSet");
}

#[test]
fn the_deluge_path_resolves_from_the_vendor_and_the_name_of_the_device_to_place() {
    let got = osd::resource_path(&target(), "X-DeviceDefinition", &source(Some("Korg"), "minilogue")).unwrap();
    assert_eq!(got, "MIDI_DEVICES/DEFINITION/Korg/minilogue.xml");
}

#[test]
fn the_resolved_path_keeps_the_spaces_of_the_identity() {
    let got = osd::resource_path(&target(), "X-DeviceDefinition", &source(Some("Native Instruments"), "Prophet 6 Mk2")).unwrap();
    assert_eq!(got, "MIDI_DEVICES/DEFINITION/Native Instruments/Prophet 6 Mk2.xml");
}

#[test]
fn a_path_without_variable_resolves_as_it_is() {
    let mut r = deluge();
    r["via"]["file_write"]["path"] = json!(["a", "b.xml"]);
    let t = device(&doc(json!([r])));
    assert_eq!(osd::resource_path(&t, "X-DeviceDefinition", &source(None, "x")).unwrap(), "a/b.xml");
}

fn refused_path(vendor: Option<&str>, name: &str, pointer: &str, field: &str) {
    let f = osd::resource_path(&target(), "X-DeviceDefinition", &source(vendor, name)).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{f}");
    assert!(f.to_string().contains(field), "{f} does not name {field}");
}

#[test]
fn a_vendor_or_name_that_is_empty_or_holds_a_slash_or_a_backslash_is_refused_with_its_place() {
    let vendor = "/resources/0/via/file_write/path/2";
    let name = "/resources/0/via/file_write/path/3";
    refused_path(Some(""), "minilogue", vendor, "/device/vendor");
    refused_path(None, "minilogue", vendor, "/device/vendor");
    refused_path(Some("Ko/rg"), "minilogue", vendor, "/device/vendor");
    refused_path(Some("Ko\\rg"), "minilogue", vendor, "/device/vendor");
    refused_path(Some("Korg"), "", name, "/device/name");
    refused_path(Some("Korg"), "mini/logue", name, "/device/name");
    refused_path(Some("Korg"), "mini\\logue", name, "/device/name");
}

#[test]
fn a_resource_that_the_device_does_not_declare_or_that_has_no_file_write_has_no_path() {
    let f = osd::resource_path(&target(), "X-Other", &source(Some("Korg"), "m")).expect_err("no such resource");
    assert_eq!(f.kind, FaultKind::BadValue);
    let t = device(&doc(json!([{"resource": "X-Names", "canSet": "full", "export": "midnam"}])));
    let f = osd::resource_path(&t, "X-Names", &source(Some("Korg"), "m")).expect_err("no via");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/resources/0/via"), "{f}");
}

fn schema_export_names() -> Vec<String> {
    fn find(v: &Value, out: &mut Option<Vec<String>>) {
        match v {
            Value::Object(m) => {
                if let Some(p) = m.get("properties").and_then(|p| p.get("export")) {
                    if let Some(e) = p.get("enum").and_then(Value::as_array) {
                        *out = Some(e.iter().map(|s| s.as_str().unwrap().to_string()).collect());
                    }
                }
                m.values().for_each(|c| find(c, out));
            }
            Value::Array(a) => a.iter().for_each(|c| find(c, out)),
            _ => {}
        }
    }
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(common::repo().join("schemas/0.6/device.schema.json")).unwrap()).unwrap();
    let mut out = None;
    find(&schema, &mut out);
    out.expect("the 0.6 device schema lists the export names in an `export` property")
}

#[test]
fn the_closed_list_of_exports_is_the_same_in_the_schema_and_in_the_reader() {
    let mut schema = schema_export_names();
    let mut reader: Vec<String> = osd::RESOURCE_EXPORTS.iter().map(|s| s.to_string()).collect();
    schema.sort();
    reader.sort();
    assert_eq!(schema, reader);
    assert!(reader.contains(&"deluge".to_string()));
    for name in &reader {
        let mut r = deluge();
        r["export"] = json!(name);
        assert!(read(&doc(json!([r]))).is_ok(), "{name}");
    }
}

fn case(name: &str, file: &str) -> Value {
    let path = common::repo().join("conformance/0.6/read").join(name).join(file);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

#[test]
fn a_conformance_case_describes_the_deluge_with_its_path_its_source_and_its_licence() {
    let name = "resource-deluge-definition";
    let c = case(name, "case.json");
    let d = case(name, "descriptor.json");
    assert_eq!(c["read"], "device");
    assert_eq!(c["result"], d, "the result is the descriptor read back");
    let path = &d["resources"][0]["via"]["file_write"]["path"];
    assert_eq!(path, &deluge()["via"]["file_write"]["path"]);
    assert_eq!(d["resources"][0]["via"]["file_write"]["carried_by"], "sysex");
    let source = &d["_sources"][0];
    assert!(source["url"].as_str().unwrap().contains("DelugeFirmware"), "{source}");
    assert!(source["license"].as_str().is_some_and(|l| !l.is_empty() && l != "NOASSERTION"), "{source}");
    let text = c.to_string();
    assert!(text.contains("SysEx") && text.contains("MIDI_DEVICES/DEFINITION"), "the case notes the SysEx write and the folder: {text}");
}

#[test]
fn the_conformance_cases_hold_each_resource_form_and_each_refusal() {
    let cases = [
        ("resource-two-forms", None),
        ("resource-name-without-prefix-refused", Some(("BadValue", "/resources/0/resource"))),
        ("resource-can-set-outside-list-refused", Some(("BadValue", "/resources/0/canSet"))),
        ("resource-export-unknown-refused", Some(("BadValue", "/resources/0/export"))),
        ("resource-export-missing-refused", Some(("Missing", "/resources/0/export"))),
        ("resource-via-way-unknown-refused", Some(("UnknownField", "/resources/0/via/ftp"))),
        ("resource-path-empty-refused", Some(("BadValue", "/resources/0/via/file_write/path"))),
        ("resource-carried-by-outside-list-refused", Some(("BadValue", "/resources/0/via/file_write/carried_by"))),
    ];
    for (name, fault) in cases {
        let c = case(name, "case.json");
        match fault {
            None => assert!(c.get("result").is_some(), "{name}"),
            Some((kind, pointer)) => assert_eq!(c["fault"], json!({"kind": kind, "pointer": pointer}), "{name}"),
        }
    }
}
