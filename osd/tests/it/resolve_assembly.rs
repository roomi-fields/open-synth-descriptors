//! The assembly descriptor of 0.6: a stock firmware and the modules applied to it, each by catalogue
//! id and exact version (the lockfile of the firmware), written by the builder. Reading it strictly;
//! resolving its MIDI as the stock routes followed by each module's own routes, in the listed
//! order (`midi_of`, module after module).

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const ASSEMBLY: &str = "https://w3id.org/osd/0.6/assembly.schema.json";
const FIRMWARE: &str = "https://w3id.org/osd/0.6/firmware.schema.json";
const DEVICE: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn stock_pin() -> Value {
    json!({"firmware": "acme/stock", "version": "1.0"})
}

fn pins(ids: &[&str]) -> Value {
    Value::Array(ids.iter().map(|m| json!({"module": m, "version": "1.0"})).collect())
}

fn assembly(stock: Value, modules: Value) -> Value {
    json!({"$schema": ASSEMBLY, "stock": stock, "modules": modules})
}

fn read(doc: &Value) -> Result<Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Assembly)
}

/// The refusal of `doc` by the reader and by the schema (at the parent for a missing or unknown field).
fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let fault = read(doc).expect_err(&format!("{doc} is refused"));
    assert_eq!((fault.kind, fault.pointer.as_str()), (kind, pointer), "the reader refuses {doc}: {fault}");
    let errors = common::Schemas::load("0.6").errors(Kind::Assembly, doc);
    let want = if matches!(kind, FaultKind::Missing | FaultKind::UnknownField) { pointer.rsplit_once('/').map_or("", |(p, _)| p) } else { pointer };
    let got: Vec<&str> = errors.iter().map(|(at, _)| at.as_str()).collect();
    assert!(!got.is_empty() && got.iter().all(|at| *at == want), "the schema refuses {doc} at {want}, not {errors:?}");
}

#[test]
fn an_assembly_pins_a_stock_firmware_and_its_modules_each_with_its_version_in_order_and_is_read_as_written() {
    let doc = assembly(json!({"firmware": "acme/stock", "version": "2.1"}), json!([{"module": "acme/mod-b", "version": "1.3"}, {"module": "acme/mod-a", "version": "1.0"}]));
    let d = read(&doc).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(d.kind(), Kind::Assembly);
    assert_eq!(d.schema(), Some(Version::V0_6));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&d)).unwrap(), doc);
    assert_eq!(common::Schemas::load("0.6").check(Kind::Assembly, &doc), None);
    assert!(read(&assembly(stock_pin(), json!([]))).is_ok());
}

#[test]
fn an_assembly_without_a_stock_is_refused() {
    refused(&json!({"$schema": ASSEMBLY, "modules": []}), FaultKind::Missing, "/stock");
}

#[test]
fn a_stock_without_a_version_is_refused_at_the_version() {
    refused(&assembly(json!({"firmware": "acme/stock"}), json!([])), FaultKind::Missing, "/stock/version");
}

#[test]
fn a_stock_without_a_firmware_is_refused_at_the_firmware() {
    refused(&assembly(json!({"version": "1.0"}), json!([])), FaultKind::Missing, "/stock/firmware");
}

#[test]
fn a_stock_that_stays_a_text_is_refused() {
    refused(&assembly(json!("acme/stock"), json!([])), FaultKind::BadValue, "/stock");
}

#[test]
fn a_module_without_a_version_is_refused_at_the_version() {
    refused(&assembly(stock_pin(), json!([{"module": "acme/mod-a", "version": "1.0"}, {"module": "acme/mod-b"}])), FaultKind::Missing, "/modules/1/version");
}

#[test]
fn a_module_that_stays_a_text_is_refused() {
    refused(&assembly(stock_pin(), json!(["acme/mod-a"])), FaultKind::BadValue, "/modules/0");
}

#[test]
fn a_module_entry_without_a_module_or_with_an_unknown_key_or_a_version_that_is_not_text_is_refused() {
    refused(&assembly(stock_pin(), json!([{"version": "1.0"}])), FaultKind::Missing, "/modules/0/module");
    refused(&assembly(stock_pin(), json!([{"module": "acme/a", "version": "1.0", "colour": "red"}])), FaultKind::UnknownField, "/modules/0/colour");
    refused(&assembly(stock_pin(), json!([{"module": "acme/a", "version": 1}])), FaultKind::BadValue, "/modules/0/version");
    refused(&assembly(json!({"firmware": "acme/stock", "version": 1}), json!([])), FaultKind::BadValue, "/stock/version");
}

#[test]
fn an_assembly_with_an_unknown_field_is_refused() {
    let mut doc = assembly(stock_pin(), json!([]));
    doc["colour"] = json!("red");
    refused(&doc, FaultKind::UnknownField, "/colour");
}

#[test]
fn an_assembly_notes_for_people_are_kept_out_of_the_refusals() {
    let mut doc = assembly(stock_pin(), json!([]));
    doc["_about"] = json!("built by acme");
    assert!(read(&doc).is_ok());
}

#[test]
fn an_assembly_whose_modules_are_not_a_list_is_refused() {
    refused(&assembly(stock_pin(), json!("acme/mod-a")), FaultKind::BadValue, "/modules");
    refused(&assembly(json!(3), json!([])), FaultKind::BadValue, "/stock");
}

#[test]
fn an_assembly_converts_to_its_own_version_unchanged_and_is_never_carried_back() {
    let doc = assembly(stock_pin(), pins(&["acme/mod-a"]));
    let out = osd::convert_to(&doc.to_string(), Kind::Assembly, Version::V0_6).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), doc);
    assert!(osd::convert_to(&doc.to_string(), Kind::Assembly, Version::V0_5).is_err());
}

#[test]
fn the_kind_an_assembly_declares_is_the_assembly_kind() {
    assert_eq!(osd::declared_kind(&assembly(stock_pin(), json!([])).to_string()), Some(Kind::Assembly));
}

#[test]
fn a_firmware_of_kind_module_reads_in_0_6_and_a_0_5_firmware_refuses_it() {
    let target = json!([{"device": {"descriptor": "acme/synth", "vendor": "Acme", "model": "Synth"}}]);
    let m = json!({"$schema": FIRMWARE, "name": "M", "version": "1", "kind": "module", "targets": target});
    assert!(osd::read(&m.to_string(), Kind::Firmware).is_ok());
    assert_eq!(common::Schemas::load("0.6").check(Kind::Firmware, &m), None);
    let old = json!({"$schema": "https://w3id.org/osd/0.5/firmware.schema.json", "name": "M", "version": "1", "kind": "module", "device": {"descriptor": "acme/synth", "vendor": "Acme", "model": "Synth"}});
    assert!(osd::read(&old.to_string(), Kind::Firmware).is_err());
}

#[test]
fn a_firmware_a_device_and_a_sound_still_read_as_before() {
    let firmware = json!({"$schema": FIRMWARE, "name": "F", "version": "1", "targets": [{"device": {"descriptor": "acme/synth", "vendor": "Acme", "model": "Synth"}}]});
    assert!(osd::read(&firmware.to_string(), Kind::Firmware).is_ok());
    assert!(osd::read(&device(json!({})).to_string(), Kind::Device).is_ok());
    assert!(read(&firmware).is_err(), "a firmware is not an assembly");
}

// ---- resolution ----

type Catalogue = BTreeMap<String, String>;

fn device(extra: Value) -> Value {
    let mut doc = json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"}});
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn synth() -> Value {
    device(json!({"params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [{"osc": "/p1"}, {"osc": "/p2"}]}}))
}

fn firmware(kind: &str, midi: Value) -> Value {
    json!({"$schema": FIRMWARE, "name": "F", "version": "1.0", "kind": kind, "targets": [{"device": {"descriptor": "acme/synth", "vendor": "Acme", "model": "Synth"}, "midi": midi}]})
}

/// A stock firmware and a driver for its MIDI, plus a module per `(name, midi driver extra)`:
/// the module's driver `acme/<name>-midi` follows the stock MIDI and adds `extra`.
fn catalogue(modules: &[(&str, Value)]) -> Catalogue {
    let mut files: Catalogue = BTreeMap::new();
    let mut put = |id: &str, v: Value| {
        files.insert(id.to_string(), v.to_string());
    };
    put("acme/synth", synth());
    put("acme/stock", firmware("firmware", json!({"descriptor": "acme/synth"})));
    for (name, extra) in modules {
        let mut own = extra.clone();
        own["midi_of"] = json!("acme/synth");
        put(&format!("acme/{name}-midi"), device(own));
        put(&format!("acme/{name}"), firmware("module", json!({"descriptor": format!("acme/{name}-midi")})));
    }
    files
}

fn add(files: &mut Catalogue, id: &str, v: Value) {
    files.insert(id.to_string(), v.to_string());
}

fn entries(extra: &[&str]) -> Value {
    json!({"params": {"entries": extra.iter().map(|o| json!({"osc": o})).collect::<Vec<_>>()}})
}

fn resolve(files: &Catalogue, modules: &[&str]) -> Result<osd::Resolved, osd::Fault> {
    let mut files = files.clone();
    let list: Vec<String> = modules.iter().map(|m| format!("acme/{m}")).collect();
    let list: Vec<&str> = list.iter().map(String::as_str).collect();
    add(&mut files, "acme/build", assembly(stock_pin(), pins(&list)));
    osd::resolve_assembly("acme/build", None, &files)
}

fn doc_of(r: &osd::Resolved) -> Value {
    serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device.clone())))).unwrap()
}

fn routes(r: &osd::Resolved) -> Vec<(String, Value)> {
    doc_of(r)["params"]["entries"].as_array().unwrap().iter().map(|e| (e["osc"].as_str().unwrap().to_string(), e.clone())).collect()
}

fn addresses(r: &osd::Resolved) -> Vec<String> {
    routes(r).into_iter().map(|(a, _)| a).collect()
}

#[test]
fn the_resolved_midi_of_an_assembly_is_the_stock_routes_then_each_modules_in_the_listed_order() {
    let files = catalogue(&[("mod-a", entries(&["/p3"])), ("mod-b", entries(&["/p4", "/p5"]))]);
    let r = resolve(&files, &["mod-a", "mod-b"]).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(addresses(&r), ["/p1", "/p2", "/p3", "/p4", "/p5"]);
    let p = &doc_of(&r)["params"];
    assert_eq!((&p["get_frame"], &p["set_frame"], &p["value_type"]), (&json!([240, 247]), &json!([240, 247]), &json!("u7")));
    assert_eq!(doc_of(&r)["device"]["name"], "Synth");
    // The order of the list is the order of the routes.
    let r = resolve(&files, &["mod-b", "mod-a"]).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(addresses(&r), ["/p1", "/p2", "/p4", "/p5", "/p3"]);
}

#[test]
fn a_module_adds_only_its_own_routes_never_the_stock_ones_it_followed() {
    let files = catalogue(&[("mod-a", entries(&["/p3"]))]);
    let r = resolve(&files, &["mod-a"]).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(addresses(&r), ["/p1", "/p2", "/p3"], "the stock routes are not added a second time");
    assert!(!doc_of(&r).as_object().unwrap().contains_key("midi_of"));
}

#[test]
fn the_files_an_assembly_reads_are_the_assembly_the_stock_and_each_module_by_its_id() {
    let files = catalogue(&[("mod-a", entries(&["/p3"]))]);
    let r = resolve(&files, &["mod-a"]).unwrap_or_else(|f| panic!("{f}"));
    for id in ["acme/build", "acme/stock", "acme/mod-a", "acme/mod-a-midi", "acme/synth"] {
        assert!(r.files.iter().any(|f| f == id), "{id} in {:?}", r.files);
    }
    assert_eq!(r.files[0], "acme/build");
}

#[test]
fn a_module_adds_routes_to_a_list_and_a_table_of_entries() {
    let own = json!({"commands": [{"osc": "/go"}], "cc_params": {"entries": [{"osc": "/cc1"}]}});
    let mut files = catalogue(&[("mod-a", own)]);
    let mut stock = synth();
    stock["commands"] = json!([{"osc": "/start"}]);
    stock["cc_params"] = json!({"entries": [{"osc": "/cc0"}]});
    add(&mut files, "acme/synth", stock);
    let r = resolve(&files, &["mod-a"]).unwrap_or_else(|f| panic!("{f}"));
    let doc = doc_of(&r);
    let osc = |k: &str| -> Vec<String> {
        let list = if k == "commands" { &doc[k] } else { &doc[k]["entries"] };
        list.as_array().unwrap().iter().map(|e| e["osc"].as_str().unwrap().to_string()).collect()
    };
    assert_eq!(osc("commands"), ["/start", "/go"]);
    assert_eq!(osc("cc_params"), ["/cc0", "/cc1"]);
}

#[test]
fn an_empty_module_list_resolves_to_the_stock_midi_as_it_is() {
    let files = catalogue(&[]);
    let r = resolve(&files, &[]).unwrap_or_else(|f| panic!("{f}"));
    let stock = osd::resolve("acme/synth", &files).unwrap();
    assert_eq!(doc_of(&r), doc_of(&stock));
    assert_eq!(addresses(&r), ["/p1", "/p2"]);
}

#[test]
fn a_module_whose_midi_is_the_stocks_adds_nothing() {
    let mut files = catalogue(&[("mod-b", entries(&["/p4"]))]);
    add(&mut files, "acme/same", firmware("module", json!({"same_as": "acme/synth"})));
    let r = resolve(&files, &["same", "mod-b", "same"]).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(addresses(&r), ["/p1", "/p2", "/p4"]);
    let alone = resolve(&files, &["same"]).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(doc_of(&alone), doc_of(&resolve(&files, &[]).unwrap()));
}

#[test]
fn a_module_whose_same_as_names_another_descriptor_is_refused_in_its_firmware_file() {
    let mut files = catalogue(&[]);
    add(&mut files, "acme/other-synth", device(json!({})));
    add(&mut files, "acme/same", firmware("module", json!({"same_as": "acme/other-synth"})));
    let f = resolve(&files, &["same"]).expect_err("refused");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/same"), "/targets/0/midi/same_as"), "{f}");
}

#[test]
fn a_module_whose_descriptor_is_not_of_the_kind_module_is_refused_in_its_firmware_file() {
    for kind in ["mod", "firmware", "script", "extension"] {
        let mut files = catalogue(&[("mod-a", entries(&["/p3"]))]);
        add(&mut files, "acme/mod-a", firmware(kind, json!({"descriptor": "acme/mod-a-midi"})));
        let f = resolve(&files, &["mod-a"]).expect_err(kind);
        assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/mod-a"), "/kind"), "{kind}: {f}");
    }
    // A firmware without `kind` is a firmware, not a module.
    let mut files = catalogue(&[("mod-a", entries(&["/p3"]))]);
    let mut bare = firmware("module", json!({"descriptor": "acme/mod-a-midi"}));
    bare.as_object_mut().unwrap().remove("kind");
    add(&mut files, "acme/mod-a", bare);
    let f = resolve(&files, &["mod-a"]).expect_err("no kind");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/kind"), "{f}");
}

#[test]
fn a_stock_whose_descriptor_is_not_of_the_kind_firmware_is_refused_in_its_firmware_file() {
    for kind in ["module", "mod"] {
        let mut files = catalogue(&[]);
        add(&mut files, "acme/stock", firmware(kind, json!({"descriptor": "acme/synth"})));
        let f = resolve(&files, &[]).expect_err(kind);
        assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/stock"), "/kind"), "{kind}: {f}");
    }
}

#[test]
fn two_modules_that_touch_the_same_address_are_neither_refused_nor_reported_and_the_later_one_wins() {
    let a = json!({"params": {"entries": [{"osc": "/p3", "p": 1}, {"osc": "/p2", "p": 1}]}});
    let b = json!({"params": {"entries": [{"osc": "/p3", "p": 2}]}});
    let files = catalogue(&[("mod-a", a), ("mod-b", b)]);
    let r = resolve(&files, &["mod-a", "mod-b"]).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(addresses(&r), ["/p1", "/p2", "/p3"]);
    let by = |osc: &str| routes(&r).into_iter().find(|(a, _)| a == osc).unwrap().1;
    assert_eq!(by("/p3")["p"], 2, "the module later in the list replaces the route of the one before");
    assert_eq!(by("/p2")["p"], 1, "a module's route replaces the stock's at its address");
    // The same module twice is no more refused.
    assert!(resolve(&files, &["mod-a", "mod-a"]).is_ok());
    // Reversed, the other wins.
    let r = resolve(&files, &["mod-b", "mod-a"]).unwrap();
    assert_eq!(routes(&r).into_iter().find(|(a, _)| a == "/p3").unwrap().1["p"], 1);
}

#[test]
fn a_module_midi_that_touches_more_than_routes_is_refused_in_its_own_file() {
    let files = catalogue(&[("mod-a", json!({"sysex": {"header": [240, 1]}}))]);
    let f = resolve(&files, &["mod-a"]).expect_err("refused");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::UnknownField, Some("acme/mod-a-midi"), "/sysex"), "{f}");
}

#[test]
fn a_module_midi_that_repeats_an_address_in_its_own_file_is_refused_in_that_file() {
    let files = catalogue(&[("mod-a", entries(&["/p3", "/p3"]))]);
    let f = resolve(&files, &["mod-a"]).expect_err("refused");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/mod-a-midi"), "/params/entries/1/osc"), "{f}");
}

#[test]
fn a_module_midi_of_another_device_or_another_maker_is_refused_in_its_own_file() {
    let mut files = catalogue(&[("mod-a", entries(&["/p3"]))]);
    let mut other = device(entries(&["/p3"]));
    other["device"]["name"] = json!("Other");
    other["midi_of"] = json!("acme/synth");
    add(&mut files, "acme/mod-a-midi", other);
    let f = resolve(&files, &["mod-a"]).expect_err("another device");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/mod-a-midi"), "/midi_of"), "{f}");

    let mut files = catalogue(&[]);
    let mut foreign = device(entries(&["/p3"]));
    foreign["midi_of"] = json!("acme/synth");
    add(&mut files, "rival/mod-x-midi", foreign);
    add(&mut files, "acme/mod-x", firmware("module", json!({"descriptor": "rival/mod-x-midi"})));
    let f = resolve(&files, &["mod-x"]).expect_err("another maker");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("rival/mod-x-midi"), "/midi_of"), "{f}");
}

#[test]
fn a_module_midi_that_does_not_follow_the_stock_midi_is_refused() {
    let mut files = catalogue(&[]);
    let mut stray = synth();
    stray["params"]["entries"] = json!([{"osc": "/z"}]);
    add(&mut files, "acme/stray", stray);
    add(&mut files, "acme/mod-x", firmware("module", json!({"descriptor": "acme/stray"})));
    assert!(resolve(&files, &["mod-x"]).is_err(), "a module's MIDI is a file that follows the stock's");
}

#[test]
fn an_assembly_that_names_a_missing_file_is_refused_at_its_link() {
    let files = catalogue(&[]);
    let f = resolve(&files, &["ghost"]).expect_err("refused");
    assert_eq!(f.kind, FaultKind::LinkMissing, "{f}");
    let mut files = files;
    add(&mut files, "acme/build", assembly(json!({"firmware": "acme/nostock", "version": "1.0"}), json!([])));
    let f = osd::resolve_assembly("acme/build", None, &files).expect_err("refused");
    assert_eq!(f.kind, FaultKind::LinkMissing, "{f}");
}

// ---- the device an assembly is resolved for ----

/// Two neighbouring devices of one maker, each with its own MIDI file; a stock firmware and a
/// module, each made for both with a target per device carrying its own MIDI.
fn two_devices() -> Catalogue {
    let target = |id: &str, model: &str, midi: Value| json!({"device": {"descriptor": id, "vendor": "Acme", "model": model}, "midi": midi});
    let fw = |kind: &str, targets: Value| json!({"$schema": FIRMWARE, "name": "F", "version": "1.0", "kind": kind, "targets": targets});
    let mut files: Catalogue = BTreeMap::new();
    add(&mut files, "acme/synth", synth());
    let mut keys = synth();
    keys["device"]["name"] = json!("Synth Keys");
    keys["params"]["entries"] = json!([{"osc": "/k1"}, {"osc": "/k2"}]);
    add(&mut files, "acme/keys", keys);
    let mut own_synth = entries(&["/p3"]);
    own_synth["midi_of"] = json!("acme/synth");
    add(&mut files, "acme/mod-synth-midi", device(own_synth));
    let mut own_keys = entries(&["/k3"]);
    own_keys["midi_of"] = json!("acme/keys");
    let mut own_keys = device(own_keys);
    own_keys["device"]["name"] = json!("Synth Keys");
    add(&mut files, "acme/mod-keys-midi", own_keys);
    add(&mut files, "acme/stock", fw("firmware", json!([
        target("acme/synth", "Synth", json!({"descriptor": "acme/synth"})),
        target("acme/keys", "Synth Keys", json!({"descriptor": "acme/keys"})),
    ])));
    add(&mut files, "acme/mod", fw("module", json!([
        target("acme/synth", "Synth", json!({"descriptor": "acme/mod-synth-midi"})),
        target("acme/keys", "Synth Keys", json!({"descriptor": "acme/mod-keys-midi"})),
    ])));
    add(&mut files, "acme/build", assembly(stock_pin(), pins(&["acme/mod"])));
    files
}

#[test]
fn one_module_resolves_on_each_of_two_devices_with_the_routes_of_its_own_target() {
    let files = two_devices();
    let on_synth = osd::resolve_assembly("acme/build", Some("acme/synth"), &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(addresses(&on_synth), ["/p1", "/p2", "/p3"]);
    assert_eq!(doc_of(&on_synth)["device"]["name"], "Synth");
    let on_keys = osd::resolve_assembly("acme/build", Some("acme/keys"), &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(addresses(&on_keys), ["/k1", "/k2", "/k3"]);
    assert_eq!(doc_of(&on_keys)["device"]["name"], "Synth Keys");
}

#[test]
fn a_stock_with_several_targets_needs_the_device_to_resolve() {
    let f = osd::resolve_assembly("acme/build", None, &two_devices()).expect_err("which device?");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/stock"), "/targets"), "{f}");
}

#[test]
fn a_device_the_stock_does_not_run_on_is_refused_in_the_stock_file() {
    let mut files = two_devices();
    add(&mut files, "acme/other", device(json!({})));
    let f = osd::resolve_assembly("acme/build", Some("acme/other"), &files).expect_err("no target for it");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/stock"), "/targets"), "{f}");
}

#[test]
fn a_module_without_a_target_for_the_device_is_refused_in_its_own_file() {
    let mut files = two_devices();
    let mut only_synth: Value = serde_json::from_str(&files["acme/mod"]).unwrap();
    only_synth["targets"].as_array_mut().unwrap().truncate(1);
    add(&mut files, "acme/mod", only_synth);
    assert!(osd::resolve_assembly("acme/build", Some("acme/synth"), &files).is_ok());
    let f = osd::resolve_assembly("acme/build", Some("acme/keys"), &files).expect_err("the module is not for the keys");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/mod"), "/targets"), "{f}");
}

#[test]
fn a_firmware_whose_two_targets_name_the_same_device_is_refused() {
    let mut files = two_devices();
    let mut twice: Value = serde_json::from_str(&files["acme/mod"]).unwrap();
    twice["targets"][1]["device"]["descriptor"] = json!("acme/synth");
    add(&mut files, "acme/mod", twice);
    let f = osd::resolve_assembly("acme/build", Some("acme/synth"), &files).expect_err("two targets for one device");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/mod"), "/targets"), "{f}");
}

#[test]
fn the_device_may_be_given_when_the_stock_has_one_target_and_must_then_be_its_device() {
    let files = catalogue(&[("mod-a", entries(&["/p3"]))]);
    let mut files = files;
    add(&mut files, "acme/build", assembly(stock_pin(), pins(&["acme/mod-a"])));
    let given = osd::resolve_assembly("acme/build", Some("acme/synth"), &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(addresses(&given), ["/p1", "/p2", "/p3"]);
    let f = osd::resolve_assembly("acme/build", Some("acme/elsewhere"), &files).expect_err("not its device");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/targets"), "{f}");
}

#[test]
fn a_target_of_the_chosen_device_without_midi_is_refused_where_the_stock_must_name_it() {
    let mut files = two_devices();
    let mut stock: Value = serde_json::from_str(&files["acme/stock"]).unwrap();
    stock["targets"][1].as_object_mut().unwrap().remove("midi");
    add(&mut files, "acme/stock", stock);
    assert!(osd::resolve_assembly("acme/build", Some("acme/synth"), &files).is_ok(), "the other target is not read");
    let f = osd::resolve_assembly("acme/build", Some("acme/keys"), &files).expect_err("no midi");
    assert_eq!((f.kind, f.file.as_deref()), (FaultKind::BadValue, Some("acme/stock")), "{f}");
    assert!(f.pointer.starts_with("/targets"), "{f}");
}
