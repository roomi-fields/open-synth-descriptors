//! The trial of 0.7, proved without the SyntHub catalogue: the function the
//! catalogue test plays, on small catalogues written here.

use crate::common::trial::{judge, trial, Check, Failure, Trial};
use crate::common::{Folder, Schemas};
use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};

const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";
const P06: &str = "https://w3id.org/osd/0.6/profile.schema.json";

/// A catalogue of these files, by catalogue id, in a folder removed with the
/// guard returned.
fn catalogue_of(files: &[(&str, Value)]) -> (tempfile::TempDir, Folder) {
    let tmp = tempfile::tempdir().unwrap();
    for (id, doc) in files {
        let path = tmp.path().join(format!("{id}.json"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_string_pretty(doc).unwrap()).unwrap();
    }
    let dir = Folder(tmp.path().to_path_buf());
    (tmp, dir)
}

/// A driver with every route table, a control reached by two messages and
/// every text for people, its identity completed from a profile.
fn full_driver() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "Full", "osc_prefix": "/full" },
        "identity": { "profile": "acme", "family": [1, 0], "member": [2, 0] },
        "cc_params": { "entries": [
            {
                "osc": "/filter/cutoff", "cc": 74, "range": [0, 127],
                "also": [{ "nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "range": [0, 16383] }],
                "_usage": "how far the filter opens", "_name": "Cutoff",
                "_description": "the cutoff of the filter", "_notes": "measured once"
            },
            { "osc": "/filter/resonance", "cc": 71, "_name": "", "_notes": 4 }
        ] },
        "params": {
            "get_frame": [240, 66, { "arg": "pr" }, { "arg": "p" }, 247],
            "set_frame": [240, 67, { "arg": "pr" }, { "arg": "p" }, { "arg": "value" }, 247],
            "value_type": "u7",
            "entries": [
                { "osc": "/osc1/shape", "pr": 16, "p": 2, "range": [0, 100], "_name": "Shape", "_usage": "morph" },
                { "osc": "/osc1/pitch", "pr": 16, "p": 3, "_description": "the pitch", "_notes": "coarse" }
            ]
        },
        "commands": [
            { "osc": "/ping", "frame": [240, 66, 247], "_name": "Ping", "_usage": "any time", "_notes": "no reply" }
        ],
        "triggers": { "entries": [
            { "osc": "/kick", "sound": "kick", "note_range": [36, 36], "_name": "Kick", "_description": "the kick", "_usage": "hit" }
        ] }
    })
}

fn profile() -> Value {
    json!({ "$schema": P06, "identity": { "reply_text": "acme" } })
}

/// A driver another one shares the MIDI of, in the form of 0.5.
fn base_driver() -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.5/device.schema.json",
        "device": { "name": "Pair", "osc_prefix": "/pair" },
        "cc_params": { "entries": [{ "osc": "/volume", "cc": 7 }] }
    })
}

fn variant_driver() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "Pair", "osc_prefix": "/pair" },
        "midi_of": "acme/base"
    })
}

/// A driver of 0.4, the form without `$schema`.
fn old_driver() -> Value {
    json!({
        "device": { "name": "Old", "osc_prefix": "/old" },
        "cc_params": { "entries": [{ "osc": "/volume", "cc": 7, "_usage": "level" }] }
    })
}

fn passing() -> Vec<(&'static str, Value)> {
    vec![
        ("acme/full", full_driver()),
        ("acme/_profile", profile()),
        ("acme/base", base_driver()),
        ("acme/variant", variant_driver()),
        ("acme/old", old_driver()),
    ]
}

/// A driver both readings refuse: a field no version defines.
fn refused_by_both() -> Value {
    json!({ "$schema": D06, "device": { "name": "Bad", "osc_prefix": "/bad" }, "no_such_field": 1 })
}

/// A driver 0.6 reads, whose two sources write one `id`, which only 0.7 refuses
/// (`spec/0.7.md` 5.3.4).
fn twice_identified() -> Value {
    let source = json!({ "type": "vendor-doc", "id": "manual" });
    json!({
        "$schema": D06,
        "_sources": [source.clone(), source],
        "device": { "name": "Mute", "osc_prefix": "/mute" },
        "cc_params": { "entries": [{ "osc": "/a", "cc": 1 }] }
    })
}

/// A driver of 0.6 with a control and a `params` entry that give no message, with
/// and without a `range`.
fn silent_controls() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "Mute", "osc_prefix": "/mute" },
        "cc_params": { "entries": [
            { "osc": "/a", "cc": 1 },
            { "osc": "/b" },
            { "osc": "/c", "range": [0, 3], "transmit": "absolute", "recognize": "none", "nrpn_lsb_range": [0, 7], "_name": "Bank" }
        ] },
        "params": {
            "get_frame": [240, { "arg": "p" }, 247],
            "set_frame": [240, { "arg": "p" }, { "arg": "value" }, 247],
            "value_type": "u7",
            "entries": [{ "osc": "/d" }, { "osc": "/e", "range": [0, 99], "recognize": "both" }]
        }
    })
}

#[test]
fn a_control_or_a_params_entry_without_a_message_is_neither_a_refusal_nor_a_loss() {
    let old = json!({
        "device": { "name": "Older", "osc_prefix": "/older" },
        "cc_params": { "entries": [{ "osc": "/bank_msb", "orientation": "0-based" }, { "osc": "/r", "range": [0, 5] }] }
    });
    let found = run(&[("acme/mute", silent_controls()), ("acme/older", old)]);
    assert_eq!(found.files, 2);
    assert!(found.failures.is_empty(), "{found}");
    assert!(found.refused_in_0_6.is_empty() && found.refused_in_0_7.is_empty(), "{found}");
}

#[test]
fn a_driver_whose_midi_of_target_holds_a_control_without_a_message_resolves() {
    let mut variant = variant_driver();
    variant["device"] = json!({ "name": "Mute", "osc_prefix": "/mute" });
    variant["midi_of"] = json!("acme/mute");
    let found = run(&[("acme/mute", silent_controls()), ("acme/variant", variant)]);
    assert!(found.failures.is_empty(), "{found}");
    assert!(found.refused_in_0_7.is_empty(), "{found}");
}

#[test]
fn a_field_an_entry_without_a_message_loses_in_0_7_is_a_loss_at_its_pointer() {
    let old = silent_controls();
    let mut new = old.clone();
    new["$schema"] = json!("https://w3id.org/osd/0.7/device.schema.json");
    new["cc_params"]["entries"][0] = json!({ "osc": "/a", "ways": [{ "ctrlType": "cc", "ctrlIndex": [1] }] });
    for n in [1, 2] {
        new["cc_params"]["entries"][n] = json!({ "osc": old["cc_params"]["entries"][n]["osc"], "ways": [] });
    }
    new["cc_params"]["entries"][2]["title"] = json!("Bank");
    let lost: Vec<String> = judge(&Schemas::load("0.7"), "acme/mute", Kind::Device, &old.to_string(), &new.to_string())
        .into_iter()
        .filter(|f| f.check == Check::Loss && f.pointer.starts_with("/cc_params/entries/2/"))
        .map(|f| f.pointer)
        .collect();
    for field in ["range", "transmit", "recognize", "nrpn_lsb_range"] {
        assert!(lost.contains(&format!("/cc_params/entries/2/{field}")), "{field}: {lost:?}");
    }
}

fn run(files: &[(&str, Value)]) -> Trial {
    let (_tmp, dir) = catalogue_of(files);
    trial(&dir)
}

/// The failures as `(check, file, pointer, kind)`.
fn named(found: &Trial) -> Vec<(Check, &str, &str, Option<FaultKind>)> {
    found.failures.iter().map(|f| (f.check, f.file.as_str(), f.pointer.as_str(), f.kind)).collect()
}

#[test]
fn a_catalogue_that_converts_without_loss_has_no_failure_and_no_refusal() {
    let found = run(&passing());
    assert_eq!(found.files, 5);
    assert!(found.failures.is_empty(), "{found}");
    assert!(found.refused_in_0_6.is_empty() && found.refused_in_0_7.is_empty(), "{found}");
}

#[test]
fn only_a_refusal_the_0_6_reading_does_not_have_fails_the_trial_named_by_file_pointer_and_kind() {
    let mut files = passing();
    files.push(("acme/bad", refused_by_both()));
    files.push(("acme/mute", twice_identified()));
    let found = run(&files);
    assert_eq!(found.files, 7);
    assert_eq!(found.refused_in_0_6, [(FaultKind::UnknownField, 1)].into(), "{found}");
    assert_eq!(found.refused_in_0_7, [(FaultKind::UnknownField, 1), (FaultKind::BadValue, 1)].into(), "{found}");
    // The file refused converts, so the trial resolves it too, and says so again.
    assert_eq!(
        named(&found),
        [
            (Check::Refusal, "acme/mute", "/sources/1", Some(FaultKind::BadValue)),
            (Check::Resolution, "acme/mute", "/sources/1", Some(FaultKind::BadValue)),
        ],
        "{found}"
    );
}

#[test]
fn the_printed_trial_gives_the_counts_by_kind_of_fault_and_names_each_failure() {
    let printed = run(&[("acme/bad", refused_by_both()), ("acme/mute", twice_identified())]).to_string();
    for part in ["refused in 0.6: 1", "refused in 0.7: 2", "BadValue", "UnknownField", "acme/mute", "/sources/1"] {
        assert!(printed.contains(part), "no {part:?} in:\n{printed}");
    }
}

#[test]
fn a_profile_0_6_reads_and_0_7_reads_too_is_tried_with_the_drivers() {
    let found = run(&[("acme/_profile", profile()), ("acme/_profile.other", json!({ "$schema": P06, "no_such_field": 1 }))]);
    assert_eq!(found.files, 2);
    assert_eq!(found.refused_in_0_6, [(FaultKind::UnknownField, 1)].into(), "{found}");
    assert!(found.failures.is_empty(), "{found}");
}

/// Two files that each read alone, joined by `midi_of`: their sources share an
/// `id`, which only 0.7 refuses (`spec/0.7.md` 5.3.4).
#[test]
fn a_resolution_only_the_0_7_junction_refuses_fails_the_trial_in_the_file_of_the_fault() {
    let source = json!([{ "type": "vendor-doc", "id": "manual" }]);
    let mut base = base_driver();
    base["_sources"] = source.clone();
    let mut variant = variant_driver();
    variant["_sources"] = source;
    let found = run(&[("acme/base", base), ("acme/variant", variant)]);
    assert!(found.refused_in_0_7.is_empty(), "{found}");
    assert_eq!(
        named(&found),
        [(Check::Resolution, "acme/variant", "/sources/0", Some(FaultKind::BadValue))],
        "{found}"
    );
}

#[test]
fn a_driver_whose_midi_of_target_0_7_refuses_fails_to_resolve_too() {
    let mut variant = variant_driver();
    variant["device"] = json!({ "name": "Mute", "osc_prefix": "/mute" });
    variant["midi_of"] = json!("acme/mute");
    let found = run(&[("acme/mute", twice_identified()), ("acme/variant", variant)]);
    // The fault of the variant's resolution is in the target's file.
    assert_eq!(
        named(&found),
        [
            (Check::Refusal, "acme/mute", "/sources/1", Some(FaultKind::BadValue)),
            (Check::Resolution, "acme/mute", "/sources/1", Some(FaultKind::BadValue)),
            (Check::Resolution, "acme/mute", "/sources/1", Some(FaultKind::BadValue)),
        ],
        "{found}"
    );
}

#[test]
fn a_driver_0_6_does_not_resolve_either_is_no_failure() {
    let found = run(&[("acme/variant", variant_driver())]);
    assert!(found.failures.is_empty(), "{found}");
}

/// The 0.6 text and the 0.7 text of the full driver.
fn texts() -> (String, String) {
    let stored = full_driver().to_string();
    let in_version = |v| osd::convert_to(&stored, Kind::Device, v).unwrap();
    (in_version(Version::V0_6), in_version(Version::V0_7))
}

/// What the trial says of the converted full driver once `tamper` has changed it.
fn judged(tamper: impl FnOnce(&mut Value)) -> Vec<Failure> {
    let (old, new) = texts();
    let mut doc: Value = serde_json::from_str(&new).unwrap();
    tamper(&mut doc);
    let mut text = serde_json::to_string_pretty(&doc).unwrap();
    text.push('\n');
    judge(&Schemas::load("0.7"), "acme/full", Kind::Device, &old, &text)
}

/// The tampered text is named: a failure of `check` in the file, at `pointer`.
#[track_caller]
fn assert_named(check: Check, pointer: &str, tamper: impl FnOnce(&mut Value)) {
    let failures = judged(tamper);
    assert!(
        failures.iter().any(|f| f.check == check && f.file == "acme/full" && f.pointer == pointer),
        "no {check:?} at {pointer} in {failures:#?}"
    );
}

fn remove(doc: &mut Value, object: &str, key: &str) {
    let removed = doc.pointer_mut(object).and_then(Value::as_object_mut).and_then(|o| o.remove(key));
    assert!(removed.is_some(), "no {object}/{key} to remove");
}

fn list<'a>(doc: &'a mut Value, at: &str) -> &'a mut Vec<Value> {
    doc.pointer_mut(at).and_then(Value::as_array_mut).unwrap()
}

#[test]
fn the_converted_text_left_as_it_is_loses_nothing() {
    let failures = judged(|_| {});
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn a_converted_text_without_one_of_its_ways_is_named() {
    assert_named(Check::Loss, "/cc_params/entries/0/ways", |d| {
        list(d, "/cc_params/entries/0/ways").pop().unwrap();
    });
}

#[test]
fn a_converted_text_with_a_way_the_0_6_entry_does_not_give_is_named() {
    assert_named(Check::Loss, "/params/entries/0/ways", |d| {
        list(d, "/params/entries/0/ways").push(json!({ "ctrlType": "sysex", "ctrlIndex": [9] }));
    });
}

#[test]
fn a_converted_text_without_an_entry_is_named_in_each_table_of_routes() {
    for table in ["/cc_params/entries", "/params/entries", "/commands", "/triggers/entries"] {
        assert_named(Check::Loss, table, |d| {
            list(d, table).pop().unwrap();
        });
    }
}

#[test]
fn a_converted_text_with_entries_in_another_order_is_named_at_their_address() {
    for table in ["/cc_params/entries", "/params/entries"] {
        assert_named(Check::Loss, &format!("{table}/0/osc"), |d| list(d, table).swap(0, 1));
    }
}

#[test]
fn a_converted_text_with_another_address_is_named_in_each_table_of_routes() {
    for entry in ["/cc_params/entries/0", "/params/entries/0", "/commands/0", "/triggers/entries/0"] {
        let at = format!("{entry}/osc");
        assert_named(Check::Loss, &at, |d| *d.pointer_mut(&at).unwrap() = json!("/elsewhere"));
    }
}

/// Where each text of the full driver is after the conversion
/// (`spec/0.7.md` 5.3.1), and the key it had in 0.6.
const TEXTS: [(&str, &str, &str); 16] = [
    ("/cc_params/entries/0", "value_description", "_usage"),
    ("/cc_params/entries/0", "title", "_name"),
    ("/cc_params/entries/0", "description", "_description"),
    ("/cc_params/entries/0", "notes", "_notes"),
    ("/cc_params/entries/1", "_name", "_name"),
    ("/cc_params/entries/1", "_notes", "_notes"),
    ("/params/entries/0", "title", "_name"),
    ("/params/entries/0", "_usage", "_usage"),
    ("/params/entries/1", "description", "_description"),
    ("/params/entries/1", "_notes", "_notes"),
    ("/commands/0", "_name", "_name"),
    ("/commands/0", "_usage", "_usage"),
    ("/commands/0", "_notes", "_notes"),
    ("/triggers/entries/0", "title", "_name"),
    ("/triggers/entries/0", "description", "_description"),
    ("/triggers/entries/0", "_usage", "_usage"),
];

#[test]
fn a_converted_text_without_one_of_its_texts_is_named_at_the_0_6_key() {
    for (entry, field, key) in TEXTS {
        assert_named(Check::Loss, &format!("{entry}/{key}"), |d| remove(d, entry, field));
    }
}

#[test]
fn a_converted_text_whose_text_is_not_identical_is_named() {
    for (entry, field, key) in TEXTS {
        assert_named(Check::Loss, &format!("{entry}/{key}"), |d| {
            *d.pointer_mut(&format!("{entry}/{field}")).unwrap() = json!("another text");
        });
    }
}

#[test]
fn a_text_moved_to_a_field_that_is_not_its_own_is_named() {
    assert_named(Check::Loss, "/cc_params/entries/0/_usage", |d| {
        remove(d, "/cc_params/entries/0", "description");
        let text = d.pointer_mut("/cc_params/entries/0").unwrap().as_object_mut().unwrap().remove("value_description").unwrap();
        d["cc_params"]["entries"][0]["description"] = text;
    });
}

#[test]
fn a_converted_text_the_reader_refuses_is_named_at_its_pointer() {
    assert_named(Check::Refusal, "/device/revision", |d| d["device"]["revision"] = json!("1"));
}

#[test]
fn a_converted_text_the_schema_of_0_7_refuses_is_named_with_its_file() {
    let failures = judged(|d| d["cc_params"]["entries"][0]["ways"][0]["ctrlType"] = json!("no-such-kind"));
    assert!(
        failures.iter().any(|f| f.check == Check::Schema && f.file == "acme/full" && f.pointer.starts_with("/cc_params/entries/0")),
        "{failures:#?}"
    );
}
