//! A program that runs alike on several devices (0.6): a firmware descriptor of kind `application`
//! with one target per device, its MIDI written once and reused by `same_as`; the assembly whose stock
//! is that program resolves each device to the program's MIDI; a device with a USB identity and no MIDI
//! of its own is a host for programs.

use crate::common;

use osd::{Descriptor, FaultKind, Kind};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const FIRMWARE: &str = "https://w3id.org/osd/0.6/firmware.schema.json";
const FIRMWARE_05: &str = "https://w3id.org/osd/0.5/firmware.schema.json";
const ASSEMBLY: &str = "https://w3id.org/osd/0.6/assembly.schema.json";
const DEVICE: &str = "https://w3id.org/osd/0.6/device.schema.json";

type Catalogue = BTreeMap<String, String>;

fn program(kind: Option<&str>, second_midi: Value) -> Value {
    let mut doc = json!({"$schema": FIRMWARE, "name": "Grainer", "version": "1.2", "targets": [
        {"device": {"descriptor": "kestrel/pocket-one", "vendor": "Kestrel", "model": "Pocket One"}, "midi": {"descriptor": "lumen/grainer-midi"}, "runs_on": {"name": "Hostix", "versions": ["3.0"]}},
        {"device": {"descriptor": "marlin/slate-two", "vendor": "Marlin", "model": "Slate Two"}, "midi": second_midi, "runs_on": {"name": "Hostix", "versions": ["3.0"]}}
    ]});
    if let Some(k) = kind {
        doc["kind"] = json!(k);
    }
    doc
}

fn host(vendor_id: u32, name: &str) -> Value {
    json!({"$schema": DEVICE, "device": {"name": name, "osc_prefix": "/host"}, "identity": {"usb": [{"vendor_id": vendor_id, "product_id": 1}]}})
}

fn midi() -> Value {
    json!({"$schema": DEVICE, "device": {"name": "Grainer", "osc_prefix": "/grainer"},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [{"osc": "/grain/size"}, {"osc": "/grain/density"}]}})
}

fn catalogue(kind: Option<&str>, second_midi: Value) -> Catalogue {
    let mut files = Catalogue::new();
    let mut put = |id: &str, v: Value| {
        files.insert(id.to_string(), v.to_string());
    };
    put("lumen/grainer", program(kind, second_midi));
    put("lumen/grainer-midi", midi());
    put("lumen/build", json!({"$schema": ASSEMBLY, "stock": {"firmware": "lumen/grainer", "version": "1.2"}, "modules": []}));
    put("kestrel/pocket-one", host(1001, "Pocket One"));
    put("marlin/slate-two", host(2002, "Slate Two"));
    files
}

fn resolved(files: &Catalogue, device: &str) -> Result<Value, osd::Fault> {
    let r = osd::resolve_assembly("lumen/build", Some(device), files)?;
    Ok(serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap())
}

fn same_as() -> Value {
    json!({"same_as": "lumen/grainer-midi"})
}

// ---- the word `application` ----

#[test]
fn a_firmware_of_kind_application_reads_in_0_6_and_is_written_back_as_written() {
    let doc = program(Some("application"), same_as());
    let d = osd::read(&doc.to_string(), Kind::Firmware).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&d)).unwrap(), doc);
    assert_eq!(common::Schemas::load("0.6").check(Kind::Firmware, &doc), None);
}

#[test]
fn a_0_4_or_0_5_firmware_refuses_the_kind_application() {
    let device = json!({"descriptor": "acme/synth", "vendor": "Acme", "model": "Synth"});
    // 0.4 is written without `$schema` (its w3id address is not read by the 0.4 reader).
    for schema in [Some(FIRMWARE_05), None] {
        let mut old = json!({"name": "P", "version": "1", "kind": "application", "device": device});
        if let Some(s) = schema {
            old["$schema"] = json!(s);
        }
        let fault = osd::read(&old.to_string(), Kind::Firmware).expect_err("application is refused before 0.6");
        assert_eq!(fault.pointer, "/kind", "{schema:?}");
    }
}

#[test]
fn the_0_6_schema_lists_application_among_the_kinds_of_a_firmware() {
    let (s5, s6) = (common::Schemas::load("0.5"), common::Schemas::load("0.6"));
    let doc = program(Some("application"), same_as());
    assert_eq!(s6.check(Kind::Firmware, &doc), None);
    let mut old = json!({"$schema": FIRMWARE_05, "name": "P", "version": "1", "kind": "application", "device": {"vendor": "Acme", "model": "Synth"}});
    assert!(s5.check(Kind::Firmware, &old).is_some(), "0.5 does not know application");
    old["kind"] = json!("module");
    assert!(s5.check(Kind::Firmware, &old).is_some(), "0.5 does not know module either");
}

// ---- the assembly whose stock is the program ----

#[test]
fn an_assembly_whose_stock_is_a_program_resolves_each_device_to_the_programs_midi_whether_named_or_same_as() {
    for kind in [Some("application"), None] {
        let files = catalogue(kind, same_as());
        let first = resolved(&files, "kestrel/pocket-one").unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        let second = resolved(&files, "marlin/slate-two").unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        assert_eq!(first, midi(), "{kind:?}");
        assert_eq!(second, first, "{kind:?}: the two devices share the MIDI of the program");
    }
}

#[test]
fn a_stock_that_names_no_midi_at_all_is_still_refused() {
    let mut files = catalogue(Some("application"), same_as());
    let mut p = program(Some("application"), json!({}));
    p["targets"][1].as_object_mut().unwrap().remove("midi");
    files.insert("lumen/grainer".into(), p.to_string());
    let fault = resolved(&files, "marlin/slate-two").expect_err("no MIDI named");
    assert_eq!((fault.kind, fault.pointer.as_str()), (FaultKind::BadValue, "/targets/1/midi/descriptor"));
}

#[test]
fn a_stock_that_is_a_script_or_a_module_is_still_refused() {
    for kind in ["script", "module", "mod", "extension"] {
        let files = catalogue(Some(kind), same_as());
        let fault = resolved(&files, "kestrel/pocket-one").expect_err(kind);
        assert_eq!((fault.kind, fault.pointer.as_str()), (FaultKind::BadValue, "/kind"), "{kind}");
    }
}

#[test]
fn a_module_of_an_assembly_cannot_be_an_application() {
    let mut files = catalogue(Some("application"), same_as());
    files.insert("lumen/add".into(), program(Some("application"), same_as()).to_string());
    files.insert(
        "lumen/build".into(),
        json!({"$schema": ASSEMBLY, "stock": {"firmware": "lumen/grainer", "version": "1.2"}, "modules": [{"module": "lumen/add", "version": "1.2"}]}).to_string(),
    );
    let fault = resolved(&files, "kestrel/pocket-one").expect_err("an application is no module");
    assert_eq!((fault.kind, fault.pointer.as_str()), (FaultKind::BadValue, "/kind"));
}

// ---- a device with an identity and no MIDI of its own ----

#[test]
fn a_device_with_a_usb_identity_and_no_midi_surface_is_read_by_the_reader_and_the_schema() {
    for doc in [host(1001, "Pocket One"), host(2002, "Slate Two")] {
        let d = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
        assert_eq!(serde_json::from_str::<Value>(&osd::write(&d)).unwrap(), doc);
        assert_eq!(common::Schemas::load("0.6").check(Kind::Device, &doc), None);
    }
}

#[test]
fn a_device_of_kind_software_still_reads_without_any_firmware_target() {
    let doc = json!({"$schema": DEVICE, "device": {"name": "Tribe Emulation", "osc_prefix": "/tribe", "kind": "software"}});
    assert!(osd::read(&doc.to_string(), Kind::Device).is_ok());
    assert_eq!(common::Schemas::load("0.6").check(Kind::Device, &doc), None);
}

// ---- the conformance cases ----

fn case(root: &str, name: &str) -> Value {
    let path = common::repo().join("conformance/0.6").join(root).join(name).join("case.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("no conformance case {root}/{name}"));
    serde_json::from_str(&text).unwrap()
}

fn files_of(name: &str) -> BTreeMap<String, Value> {
    let dir = common::repo().join("conformance/0.6/resolve").join(name).join("files");
    let mut out = BTreeMap::new();
    for maker in std::fs::read_dir(&dir).unwrap_or_else(|_| panic!("no files for {name}")) {
        let maker = maker.unwrap().path();
        for f in std::fs::read_dir(&maker).unwrap() {
            let f = f.unwrap().path();
            let id = format!("{}/{}", maker.file_name().unwrap().to_str().unwrap(), f.file_stem().unwrap().to_str().unwrap());
            out.insert(id, serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap());
        }
    }
    out
}

#[test]
fn the_program_cases_resolve_one_assembly_for_two_devices_of_two_makers_to_the_same_midi() {
    let (first, second) = (case("resolve", "assembly-program-on-first-device"), case("resolve", "assembly-program-on-second-device"));
    assert_eq!(first["resolve_assembly"], second["resolve_assembly"], "one assembly");
    let (a, b) = (first["device"].as_str().unwrap(), second["device"].as_str().unwrap());
    assert_ne!(a, b);
    assert_ne!(a.split('/').next(), b.split('/').next(), "two makers");
    assert!(first.get("result").is_some() && first["result"] == second["result"], "the same MIDI for both");
    let files = files_of("assembly-program-on-first-device");
    let asm = &files[first["resolve_assembly"].as_str().unwrap()];
    assert_eq!(asm["modules"], json!([]), "an assembly without module");
    let stock = &files[asm["stock"]["firmware"].as_str().unwrap()];
    assert_eq!(stock["kind"], "application");
    let targets = stock["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0]["device"]["descriptor"], a);
    assert_eq!(targets[1]["device"]["descriptor"], b);
    let named = targets[0]["midi"]["descriptor"].as_str().expect("the first target names the MIDI");
    assert_eq!(targets[1]["midi"]["same_as"].as_str(), Some(named), "the second reuses it by same_as");
    assert_eq!(targets[0]["runs_on"]["name"], targets[1]["runs_on"]["name"], "the same host system");
    for dev in [a, b] {
        let d = &files[dev];
        assert!(d["identity"]["usb"].as_array().is_some_and(|u| !u.is_empty()), "{dev} carries a USB identity");
        assert!(d.get("params").is_none() && d.get("controls").is_none() && d.get("commands").is_none(), "{dev} has no MIDI of its own");
    }
}

#[test]
fn the_program_second_case_shares_the_files_of_the_first() {
    assert_eq!(files_of("assembly-program-on-first-device"), files_of("assembly-program-on-second-device"));
}

#[test]
fn the_program_cases_are_named_by_the_application_firmware_reading_case() {
    let read = case("read", "firmware-application-on-two-devices");
    assert_eq!(read["read"], "firmware");
    assert_eq!(read["result"]["kind"], "application");
    assert_eq!(read["result"]["targets"].as_array().map(Vec::len), Some(2));
    assert!(read["result"]["targets"][1]["midi"]["same_as"].is_string());
}

#[test]
fn two_reading_cases_show_a_device_with_a_usb_identity_and_no_midi_from_two_makers() {
    let (a, b) = (case("read", "device-identity-without-midi-first"), case("read", "device-identity-without-midi-second"));
    let makers: Vec<&str> = [&a, &b].iter().map(|c| c["result"]["device"]["vendor"].as_str().expect("a maker")).collect();
    assert_ne!(makers[0], makers[1]);
    for c in [&a, &b] {
        assert_eq!(c["read"], "device");
        assert!(c["result"]["identity"]["usb"].as_array().is_some_and(|u| !u.is_empty()));
        for key in ["params", "controls", "commands"] {
            assert!(c["result"].get(key).is_none(), "{key}");
        }
    }
}

#[test]
fn a_reading_case_shows_a_program_whose_host_machine_is_indifferent_as_device_kind_software() {
    let c = case("read", "device-kind-software-host-indifferent");
    assert_eq!(c["read"], "device");
    assert_eq!(c["result"]["device"]["kind"], "software");
}

// ---- nothing published changes ----

#[test]
fn the_published_firmware_schemas_still_list_only_the_kinds_they_listed() {
    for v in ["0.4", "0.5"] {
        let text = std::fs::read_to_string(common::repo().join(format!("schemas/{v}/firmware.schema.json"))).unwrap();
        let schema: Value = serde_json::from_str(&text).unwrap();
        let list = schema["properties"]["kind"]["enum"].as_array().unwrap();
        assert!(!list.iter().any(|k| k == "application" || k == "module"), "{v}");
    }
}
