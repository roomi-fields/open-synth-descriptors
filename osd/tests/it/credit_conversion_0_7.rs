//! The converter 0.6 to 0.7 carries the `_sources` of a maker profile, a sound, a fingerprint, a
//! firmware and an assembly to `sources`, by the tables of a device's (`spec/0.7.md` 5.3 and
//! 6.8.4). Free in 0.6, the note is judged entry by entry and nothing is refused: what 0.7 cannot
//! hold stays under the note `_sources_2`, and the reader warns of it.

use crate::common::credit::{converted, descriptor, made_on, warned, with, KINDS};
use crate::common::{self, Schemas};

use osd::Kind;
use serde_json::{json, Value};

/// The 0.6 descriptor of `kind` with `note` under `_sources`, carried to 0.7.
fn carried(kind: Kind, note: Value, schemas: &Schemas) -> Value {
    converted(&descriptor("0.6", kind, json!({"_sources": note})), kind, schemas)
}

/// Sources that use every rule of the carrying of keys, and what each becomes.
fn tables() -> (Value, Value) {
    let before = json!([
        {"type": "vendor-doc", "preset_name": "P", "name": "N", "revision": "r7", "content_sha256": "ff00", "x-other": [1]},
        {"type": "community-re", "name": "Only", "url": "https://x.org/p/abc", "preset_id": "abc", "adaptation": "abc"},
        {"type": "knobkraft", "notes": "n"},
        {"type": "midnam"},
        {"type": "electra-preset", "provider": "Not A Provider"},
        {"type": "pencilresearch", "provider": "acme-synths"},
        {"type": "vendor-doc", "id": "manual", "imported_at": "2026-01-01", "license": "MIT"},
    ]);
    let after = json!([
        {"type": "vendor-doc", "title": "P", "name": "N", "version": "r7", "sha256": "ff00", "x-other": [1]},
        {"type": "community-re", "title": "Only", "url": "https://x.org/p/abc"},
        {"type": "contributed", "provider": "knobkraft", "notes": "type in 0.6: knobkraft\nn"},
        {"type": "contributed", "notes": "type in 0.6: midnam"},
        {"type": "contributed", "provider": "electra-one", "_provider": "Not A Provider", "notes": "type in 0.6: electra-preset"},
        {"type": "contributed", "provider": "acme-synths", "notes": "type in 0.6: pencilresearch"},
        {"type": "vendor-doc", "id": "manual", "_imported_at": "2026-01-01", "license": "MIT"},
    ]);
    (before, after)
}

#[test]
fn the_underscore_sources_of_each_kind_become_sources_with_the_carrying_of_keys_of_a_device() {
    let schemas = Schemas::load("0.7");
    let (before, after) = tables();
    for kind in KINDS {
        let out = carried(kind, before.clone(), &schemas);
        assert_eq!(out, descriptor("0.7", kind, json!({"sources": after})), "{kind:?}");
        // The one warning is that of the provider the table does not hold: every entry is credited.
        assert_eq!(warned(&out, kind), [("UnknownProvider".to_string(), "/sources/5/provider".to_string())], "{kind:?}");
    }
}

#[test]
fn a_0_4_and_a_0_5_text_of_each_kind_reach_sources_by_the_same_tables() {
    let schemas = Schemas::load("0.7");
    let (before, after) = tables();
    for (kind, path) in [
        (Kind::Sound, "examples/0.4/warm-pad.sound.json"),
        (Kind::Fingerprint, "examples/0.4/minilogue.fingerprint.json"),
        (Kind::Firmware, "examples/0.4/tribe-0.3-beta.firmware.json"),
        (Kind::Profile, "examples/0.5/acme.profile.json"),
        (Kind::Sound, "examples/0.5/warm-pad.sound.json"),
        (Kind::Fingerprint, "examples/0.5/minilogue.fingerprint.json"),
        (Kind::Firmware, "examples/0.5/tribe-0.3-beta.firmware.json"),
    ] {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(common::repo().join(path)).unwrap()).unwrap();
        let out = converted(&with(doc, json!({"_sources": before})), kind, &schemas);
        assert_eq!(out["sources"], after, "{path}");
        assert!(out.get("_sources").is_none() && out.get("_sources_2").is_none(), "{path}: {out}");
    }
}

#[test]
fn a_source_word_outside_the_tables_of_0_4_is_kept_under_the_note_in_each_kind_but_a_profile() {
    let schemas = Schemas::load("0.7");
    for (kind, path) in [
        (Kind::Sound, "examples/0.4/warm-pad.sound.json"),
        (Kind::Fingerprint, "examples/0.4/minilogue.fingerprint.json"),
        (Kind::Firmware, "examples/0.4/tribe-0.3-beta.firmware.json"),
    ] {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(common::repo().join(path)).unwrap()).unwrap();
        let out = converted(&with(doc, json!({"_sources": [{"type": "vendor", "license": "none"}, {"type": "guess"}]})), kind, &schemas);
        assert_eq!(out["sources"], json!([{"type": "vendor-doc", "license": "NONE"}]), "{path}");
        assert_eq!(out["_sources_2"], json!([{"type": "guess"}]), "{path}");
    }
}

#[test]
fn an_object_entry_without_a_type_becomes_contributed_with_no_type_in_0_6_as_first_line_of_notes() {
    let schemas = Schemas::load("0.7");
    for kind in KINDS {
        let out = carried(
            kind,
            json!([
                {"url": "https://x.org/a", "license": "CC-BY-4.0"},
                {"name": "N", "revision": "3", "notes": "two\nlines", "imported_at": "2026-01-01", "provider": "Bad One"},
                {},
                {"url": "https://x.org/b", "profile": 7},
            ]),
            &schemas,
        );
        let want = json!([
            {"type": "contributed", "url": "https://x.org/a", "license": "CC-BY-4.0", "notes": "no type in 0.6"},
            {"type": "contributed", "title": "N", "version": "3", "notes": "no type in 0.6\ntwo\nlines", "_imported_at": "2026-01-01", "_provider": "Bad One"},
            {"type": "contributed", "notes": "no type in 0.6"},
            {"type": "contributed", "url": "https://x.org/b", "_profile": 7, "notes": "no type in 0.6"},
        ]);
        assert_eq!(out["sources"], want, "{kind:?}");
        assert!(out.get("_sources_2").is_none(), "{out}");
    }
}

/// Entries 0.7 refuses even once their keys are carried.
fn refused_entries() -> Vec<Value> {
    vec![
        json!("a text"),
        json!(5),
        json!(null),
        json!(["a", "list"]),
        json!({"type": "guess", "url": "https://x.org"}),
        json!({"type": 5}),
        json!({"type": "vendor-doc", "license": "none", "name": "N"}),
        json!({"url": "https://x.org", "license": "two words", "name": "N"}),
        json!({"type": "vendor-doc", "url": 5}),
        json!({"url": "https://x.org", "notes": 5}),
        json!({"type": "knobkraft", "id": ""}),
        json!({"type": "vendor-doc", "id": 7, "revision": "2"}),
    ]
}

#[test]
fn an_entry_0_7_cannot_hold_is_kept_unchanged_under_a_note_without_taking_the_others_out_of_sources() {
    let schemas = Schemas::load("0.7");
    let mut note = vec![json!({"type": "vendor-doc", "name": "first"})];
    note.extend(refused_entries());
    note.push(json!({"type": "community-re", "name": "last"}));
    for kind in KINDS {
        let out = carried(kind, Value::Array(note.clone()), &schemas);
        assert_eq!(out["sources"], json!([{"type": "vendor-doc", "title": "first"}, {"type": "community-re", "title": "last"}]), "{kind:?}");
        assert_eq!(out["_sources_2"], Value::Array(refused_entries()), "{kind:?}");
        assert!(out.get("_sources").is_none(), "{out}");
    }
}

#[test]
fn the_later_of_two_entries_of_one_id_is_kept_under_the_note() {
    let schemas = Schemas::load("0.7");
    for kind in KINDS {
        let out = carried(
            kind,
            json!([
                {"type": "vendor-doc", "id": "a", "name": "first"},
                {"type": "vendor-doc", "id": "b"},
                {"id": "a", "name": "second"},
                {"type": "knobkraft", "id": "b"},
            ]),
            &schemas,
        );
        assert_eq!(out["sources"], json!([{"type": "vendor-doc", "id": "a", "title": "first"}, {"type": "vendor-doc", "id": "b"}]), "{kind:?}");
        assert_eq!(out["_sources_2"], json!([{"id": "a", "name": "second"}, {"type": "knobkraft", "id": "b"}]), "{kind:?}");
    }
}

#[test]
fn with_no_entry_for_sources_the_converter_writes_no_sources() {
    let schemas = Schemas::load("0.7");
    for kind in KINDS {
        let out = carried(kind, json!(["a text", {"type": "guess"}]), &schemas);
        assert!(out.get("sources").is_none() && out.get("_sources").is_none(), "{kind:?}: {out}");
        assert_eq!(out["_sources_2"], json!(["a text", {"type": "guess"}]), "{kind:?}");
        let empty = carried(kind, json!([]), &schemas);
        assert_eq!(empty, descriptor("0.7", kind, json!({})), "{kind:?}");
    }
}

#[test]
fn underscore_sources_that_is_not_a_list_is_kept_whole_under_the_note() {
    let schemas = Schemas::load("0.7");
    for kind in KINDS {
        for value in [json!("see the manual"), json!({"type": "vendor-doc"}), json!(5), json!(null), json!(true)] {
            let out = carried(kind, value.clone(), &schemas);
            assert_eq!(out, descriptor("0.7", kind, json!({"_sources_2": value})), "{kind:?}");
        }
    }
}

#[test]
fn the_note_takes_the_next_free_number_when_its_key_is_taken() {
    let schemas = Schemas::load("0.7");
    for kind in KINDS {
        let doc = descriptor("0.6", kind, json!({"_sources": ["a text"], "_sources_2": "mine", "_sources_3": [1]}));
        let out = converted(&doc, kind, &schemas);
        assert_eq!(out, descriptor("0.7", kind, json!({"_sources_2": "mine", "_sources_3": [1], "_sources_4": ["a text"]})), "{kind:?}");
    }
}

#[test]
fn each_entry_the_converter_keeps_under_the_note_is_warned_of_at_its_pointer() {
    let schemas = Schemas::load("0.7");
    for kind in KINDS {
        let list = carried(kind, json!([{"type": "vendor-doc"}, "a text", {"type": "guess"}]), &schemas);
        let want: Vec<(String, String)> = [0, 1].iter().map(|n| ("UncreditedSource".to_string(), format!("/_sources_2/{n}"))).collect();
        assert_eq!(warned(&list, kind), want, "{kind:?}");
        let whole = carried(kind, json!("see the manual"), &schemas);
        assert_eq!(warned(&whole, kind), [("UncreditedSource".to_string(), "/_sources_2".to_string())], "{kind:?}");
    }
}

#[test]
fn the_converter_keeps_a_profile_that_is_not_a_string_under_a_note_in_each_kind_and_a_string_as_it_is() {
    let schemas = Schemas::load("0.7");
    let note = json!([
        {"type": "vendor-doc", "profile": "nobody/nothing"},
        {"type": "vendor-doc", "profile": 5},
        {"type": "vendor-doc", "profile": null, "_profile": "mine"},
        {"type": "vendor-doc", "profile": [1], "_profile": "mine", "_profile_2": "also"},
    ]);
    let want = json!([
        {"type": "vendor-doc", "profile": "nobody/nothing"},
        {"type": "vendor-doc", "_profile": 5},
        {"type": "vendor-doc", "_profile_2": null, "_profile": "mine"},
        {"type": "vendor-doc", "_profile_3": [1], "_profile": "mine", "_profile_2": "also"},
    ]);
    for kind in KINDS {
        assert_eq!(carried(kind, note.clone(), &schemas)["sources"], want, "{kind:?}");
    }
}

#[test]
fn the_converter_keeps_a_profile_that_is_not_a_string_under_a_note_in_both_lists_of_a_device() {
    let schemas = Schemas::load("0.7");
    let list = json!([{"type": "vendor-doc", "profile": {"x": 1}}, {"type": "knobkraft", "profile": 3}]);
    let want = json!([
        {"type": "vendor-doc", "_profile": {"x": 1}},
        {"type": "contributed", "provider": "knobkraft", "_profile": 3, "notes": "type in 0.6: knobkraft"},
    ]);
    let doc = descriptor("0.6", Kind::Device, json!({"midi_of": "acme/b", "_sources": list, "_midi_of_sources": list}));
    let out = converted(&doc, Kind::Device, &schemas);
    assert_eq!(out["sources"], want);
    assert_eq!(out["midi_of_sources"], want);
}

#[test]
fn the_underscore_sources_of_the_assembly_a_sound_was_made_on_are_carried_the_same_way() {
    let schemas = Schemas::load("0.7");
    let (before, after) = tables();
    let mut list = before.as_array().unwrap().clone();
    list.push(json!("a text"));
    list.push(json!({"url": "https://x.org"}));
    let mut want = after.as_array().unwrap().clone();
    want.push(json!({"type": "contributed", "url": "https://x.org", "notes": "no type in 0.6"}));
    let doc = descriptor("0.6", Kind::Sound, json!({"made_on": made_on(json!({"_sources": list}))}));
    let out = converted(&doc, Kind::Sound, &schemas);
    assert_eq!(out["made_on"], made_on(json!({"sources": want, "_sources_2": ["a text"]})));
    assert!(out.get("sources").is_none(), "{out}");
    let want = [("UnknownProvider", "/made_on/sources/5/provider"), ("UncreditedSource", "/made_on/_sources_2/0")];
    assert_eq!(warned(&out, Kind::Sound), want.map(|(k, p)| (k.to_string(), p.to_string())));
}

#[test]
fn an_id_of_a_sound_written_again_in_the_assembly_it_was_made_on_is_kept_under_the_note_there() {
    let schemas = Schemas::load("0.7");
    let doc = descriptor(
        "0.6",
        Kind::Sound,
        json!({
            "_sources": [{"type": "vendor-doc", "id": "a", "name": "sound"}],
            "made_on": made_on(json!({"_sources": [{"type": "vendor-doc", "id": "a", "name": "builder"}, {"type": "vendor-doc", "id": "b"}]})),
        }),
    );
    let out = converted(&doc, Kind::Sound, &schemas);
    assert_eq!(out["sources"], json!([{"type": "vendor-doc", "id": "a", "title": "sound"}]));
    assert_eq!(
        out["made_on"],
        made_on(json!({"sources": [{"type": "vendor-doc", "id": "b"}], "_sources_2": [{"type": "vendor-doc", "id": "a", "name": "builder"}]}))
    );
}

#[test]
fn a_sound_and_the_assembly_it_was_made_on_each_keep_their_own_credit() {
    let schemas = Schemas::load("0.7");
    let doc = descriptor("0.6", Kind::Sound, json!({"_sources": [{"type": "vendor-doc", "name": "sound"}], "made_on": made_on(json!({"_sources": "see the builder"}))}));
    let out = converted(&doc, Kind::Sound, &schemas);
    assert_eq!(out["sources"], json!([{"type": "vendor-doc", "title": "sound"}]));
    assert_eq!(out["made_on"], made_on(json!({"_sources_2": "see the builder"})));
}
