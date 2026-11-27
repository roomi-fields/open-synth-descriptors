//! A `midi_of` file of 0.7 gives a role to its own routes (23.9, 23.35, 23.39 of `spec/0.7.md`):
//! it may name `role_vocabulary` when its state is its target's, the one the target's chain gives,
//! or when the target names none. For the MIDI file of a module the target is the MIDI accumulated:
//! the stock's, then the earlier modules'.

use osd::{Descriptor, FaultKind, Resolved};
use serde_json::{json, Value};
use std::collections::BTreeMap;

type Catalogue = BTreeMap<String, String>;

fn device(version: &str, extra: Value) -> Value {
    let mut doc = json!({"$schema": format!("https://w3id.org/osd/{version}/device.schema.json"), "device": {"name": "Synth", "osc_prefix": "/synth"}});
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

/// A route of 0.7 on the CC `cc`, with the role of `setting`: the level of the output, or a setting of
/// the filter.
fn route(osc: &str, cc: i64, setting: &str) -> Value {
    let part = if setting == "level" { "output" } else { "filter" };
    json!({"osc": osc, "role": {"parts": [{"part": part}], "setting": setting}, "ways": [{"ctrlType": "cc", "ctrlIndex": [cc]}]})
}

/// A file of 0.7 with one route with a role, written with the vocabulary `state`.
fn roled(state: &str, osc: &str, cc: i64, setting: &str) -> Value {
    device("0.7", json!({"role_vocabulary": state, "cc_params": {"entries": [route(osc, cc, setting)]}}))
}

/// A file of 0.7 whose one route has no role.
fn unroled(osc: &str, cc: i64) -> Value {
    device("0.7", json!({"cc_params": {"entries": [{"osc": osc, "ways": [{"ctrlType": "cc", "ctrlIndex": [cc]}]}]}}))
}

fn follows(mut file: Value, target: &str) -> Value {
    file["midi_of"] = json!(target);
    file
}

fn catalogue(files: &[(&str, Value)]) -> Catalogue {
    files.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn doc_of(r: &Resolved) -> Value {
    serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device.clone())))).unwrap()
}

/// The state of the result and each route with a role, as (address, setting).
fn roles_of(r: &Resolved) -> (Value, Vec<(String, String)>) {
    let doc = doc_of(r);
    let roles = doc["cc_params"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e.get("role").is_some())
        .map(|e| (e["osc"].as_str().unwrap().to_string(), e["role"]["setting"].as_str().unwrap().to_string()))
        .collect();
    (doc["role_vocabulary"].clone(), roles)
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

fn resolved(id: &str, files: &Catalogue) -> Resolved {
    osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"))
}

fn refused(got: Result<Resolved, osd::Fault>, kind: FaultKind, file: &str) {
    let f = got.expect_err("refused");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (kind, Some(file), "/role_vocabulary"), "{f}");
}

#[test]
fn a_midi_of_file_of_0_7_gives_a_role_to_its_own_route_under_the_state_of_its_target() {
    for state in ["1", "2"] {
        let files = catalogue(&[
            ("acme/a", follows(roled(state, "/res", 71, "resonance"), "acme/b")),
            ("acme/b", roled(state, "/cutoff", 74, "cutoff")),
        ]);
        let r = resolved("acme/a", &files);
        assert_eq!(roles_of(&r), (json!(state), pairs(&[("/cutoff", "cutoff"), ("/res", "resonance")])), "state {state}");
    }
}

#[test]
fn a_midi_of_file_of_0_7_names_the_state_of_the_result_when_its_target_names_none() {
    for state in ["1", "2"] {
        let files = catalogue(&[("acme/a", follows(roled(state, "/res", 71, "resonance"), "acme/b")), ("acme/b", unroled("/x", 1))]);
        let r = resolved("acme/a", &files);
        assert_eq!(roles_of(&r), (json!(state), pairs(&[("/res", "resonance")])), "state {state}");
    }
}

#[test]
fn a_midi_of_file_of_0_7_may_name_the_state_of_its_target_without_a_role_of_its_own() {
    let mut own = follows(unroled("/x", 1), "acme/b");
    own["role_vocabulary"] = json!("1");
    let files = catalogue(&[("acme/a", own), ("acme/b", roled("1", "/cutoff", 74, "cutoff"))]);
    assert_eq!(roles_of(&resolved("acme/a", &files)), (json!("1"), pairs(&[("/cutoff", "cutoff")])));
}

#[test]
fn the_route_of_a_midi_of_file_replaces_the_target_one_at_its_address_with_its_own_role() {
    let files = catalogue(&[
        ("acme/a", follows(roled("1", "/cutoff", 75, "resonance"), "acme/b")),
        ("acme/b", roled("1", "/cutoff", 74, "cutoff")),
    ]);
    assert_eq!(roles_of(&resolved("acme/a", &files)), (json!("1"), pairs(&[("/cutoff", "resonance")])));
}

#[test]
fn the_state_of_a_target_is_the_one_its_chain_gives() {
    let files = |state: &str| {
        catalogue(&[
            ("acme/a", follows(roled(state, "/res", 71, "resonance"), "acme/b")),
            ("acme/b", follows(unroled("/x", 1), "acme/c")),
            ("acme/c", roled("1", "/cutoff", 74, "cutoff")),
        ])
    };
    assert_eq!(roles_of(&resolved("acme/a", &files("1"))), (json!("1"), pairs(&[("/cutoff", "cutoff"), ("/res", "resonance")])));
    refused(osd::resolve("acme/a", &files("2")), FaultKind::BadValue, "acme/a");
}

#[test]
fn a_state_named_in_the_middle_of_a_chain_holds_for_the_files_above() {
    let files = |state: &str| {
        catalogue(&[
            ("acme/a", follows(roled(state, "/res", 71, "resonance"), "acme/b")),
            ("acme/b", follows(roled("2", "/level", 72, "level"), "acme/c")),
            ("acme/c", unroled("/x", 1)),
        ])
    };
    assert_eq!(roles_of(&resolved("acme/a", &files("2"))), (json!("2"), pairs(&[("/level", "level"), ("/res", "resonance")])));
    refused(osd::resolve("acme/a", &files("1")), FaultKind::BadValue, "acme/a");
}

#[test]
fn a_midi_of_file_of_0_7_gives_a_role_over_a_target_of_0_6_of_the_same_state() {
    let target = device("0.6", json!({"role_vocabulary": "1", "cc_params": {"entries": [
        {"osc": "/cutoff", "cc": 74, "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}}
    ]}}));
    let files = catalogue(&[("acme/a", follows(roled("1", "/res", 71, "resonance"), "acme/b")), ("acme/b", target)]);
    assert_eq!(roles_of(&resolved("acme/a", &files)), (json!("1"), pairs(&[("/cutoff", "cutoff"), ("/res", "resonance")])));
}

#[test]
fn a_midi_of_file_of_0_7_that_names_another_state_than_its_targets_is_refused() {
    for (own, target) in [("2", "1"), ("1", "2")] {
        let files = catalogue(&[
            ("acme/a", follows(roled(own, "/res", 71, "resonance"), "acme/b")),
            ("acme/b", roled(target, "/cutoff", 74, "cutoff")),
        ]);
        refused(osd::resolve("acme/a", &files), FaultKind::BadValue, "acme/a");
    }
}

#[test]
fn a_midi_of_file_of_0_6_still_takes_its_state_from_its_target_even_under_a_target_of_0_7() {
    let mut own = device("0.6", json!({"midi_of": "acme/b", "role_vocabulary": "1"}));
    for target in [roled("1", "/cutoff", 74, "cutoff"), unroled("/x", 1)] {
        let files = catalogue(&[("acme/a", own.clone()), ("acme/b", target)]);
        refused(osd::resolve("acme/a", &files), FaultKind::UnknownField, "acme/a");
    }
    // The same in the middle of a chain whose top is of 0.7.
    own["midi_of"] = json!("acme/c");
    let files = catalogue(&[
        ("acme/a", follows(roled("1", "/res", 71, "resonance"), "acme/b")),
        ("acme/b", own),
        ("acme/c", roled("1", "/cutoff", 74, "cutoff")),
    ]);
    refused(osd::resolve("acme/a", &files), FaultKind::UnknownField, "acme/b");
}

// ---- the MIDI file of a module: its target is the MIDI accumulated ----

/// An assembly `acme/build` of the stock MIDI `stock` and one module per `(name, MIDI file)`.
fn assembly(stock: Value, modules: &[(&str, Value)]) -> Catalogue {
    let firmware = |kind: &str, midi: &str| {
        json!({"$schema": "https://w3id.org/osd/0.7/firmware.schema.json", "name": "F", "version": "1.0", "kind": kind,
            "targets": [{"device": {"descriptor": "acme/synth", "vendor": "Acme", "model": "Synth"}, "midi": {"descriptor": midi}}]})
    };
    let mut files = catalogue(&[("acme/synth", stock), ("acme/stock", firmware("firmware", "acme/synth"))]);
    let mut pins = Vec::new();
    for (name, own) in modules {
        let midi = format!("acme/{name}-midi");
        files.insert(midi.clone(), follows(own.clone(), "acme/synth").to_string());
        files.insert(format!("acme/{name}"), firmware("module", &midi).to_string());
        pins.push(json!({"module": format!("acme/{name}"), "version": "1.0"}));
    }
    let build = json!({"$schema": "https://w3id.org/osd/0.7/assembly.schema.json", "stock": {"firmware": "acme/stock", "version": "1.0"}, "modules": pins});
    files.insert("acme/build".into(), build.to_string());
    files
}

fn built(files: &Catalogue) -> Result<Resolved, osd::Fault> {
    osd::resolve_assembly("acme/build", None, files)
}

#[test]
fn a_module_gives_a_role_to_its_own_route_over_a_stock_of_the_same_state_or_of_none() {
    for (stock, want) in [
        (roled("1", "/cutoff", 74, "cutoff"), pairs(&[("/cutoff", "cutoff"), ("/res", "resonance")])),
        (unroled("/x", 1), pairs(&[("/res", "resonance")])),
    ] {
        let files = assembly(stock, &[("a", roled("1", "/res", 71, "resonance"))]);
        let r = built(&files).unwrap_or_else(|f| panic!("{f}"));
        assert_eq!(roles_of(&r), (json!("1"), want));
    }
}

#[test]
fn two_modules_that_name_one_state_resolve_and_the_result_names_it() {
    let files = assembly(unroled("/x", 1), &[("a", roled("2", "/res", 71, "resonance")), ("b", roled("2", "/level", 72, "level"))]);
    let r = built(&files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(roles_of(&r), (json!("2"), pairs(&[("/res", "resonance"), ("/level", "level")])));
}

#[test]
fn the_state_a_module_names_holds_after_a_later_module_that_names_none() {
    let files = assembly(unroled("/x", 1), &[("a", roled("1", "/res", 71, "resonance")), ("b", unroled("/y", 2))]);
    let r = built(&files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(roles_of(&r), (json!("1"), pairs(&[("/res", "resonance")])));
}

#[test]
fn a_module_that_names_another_state_than_an_earlier_one_is_refused_in_its_midi_file() {
    let one = ("one", roled("1", "/res", 71, "resonance"));
    let two = ("two", roled("2", "/level", 72, "level"));
    let files = assembly(unroled("/x", 1), &[one.clone(), two.clone()]);
    refused(built(&files), FaultKind::BadValue, "acme/two-midi");
    let files = assembly(unroled("/x", 1), &[two, one]);
    refused(built(&files), FaultKind::BadValue, "acme/one-midi");
}

#[test]
fn a_module_that_names_another_state_than_the_stocks_is_refused_in_its_midi_file() {
    let files = assembly(roled("1", "/cutoff", 74, "cutoff"), &[("a", roled("2", "/res", 71, "resonance"))]);
    refused(built(&files), FaultKind::BadValue, "acme/a-midi");
}
