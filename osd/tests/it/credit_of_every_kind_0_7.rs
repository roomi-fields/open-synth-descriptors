//! In 0.7 a maker profile, a sound, a fingerprint, a firmware and an assembly carry `sources` with
//! the schema and the rules of a device's (`spec/0.7.md` 5.3 and 6.8, proposal 0010): the reader
//! judges them, writes them back and warns of them, and `_sources` is refused. Before 0.7
//! `sources` is no field and `_sources` stays a note.

use crate::common::credit::{address, descriptor, made_on, read, refused, warned, with, word, written_back, KINDS};
use crate::common::{self, Schemas};

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};

/// A source that writes every key 0.7 knows, and two it does not.
fn full_source() -> Value {
    json!({
        "type": "vendor-doc", "id": "manual", "provider": "electra-one", "title": "Manual", "url": "https://x.org/m",
        "firmware": "2.1", "author": "Dee", "attribution": "Dee, 2026", "license": "CC-BY-4.0", "commit": "abc",
        "notes": "two\nlines", "profile": "no-such/profile", "x-other": [1, {"a": null}], "_why": "kept"
    })
}

/// Each shape 0.7 refuses in a list of sources: the value of `sources`, the refusal and its pointer.
fn refusals() -> Vec<(Value, FaultKind, &'static str)> {
    use FaultKind::{BadValue, Missing, UnknownField};
    let one = |s: Value| json!([s]);
    let ok = json!({"type": "vendor-doc"});
    let mut out = vec![
        (json!("a text"), BadValue, "/sources"),
        (json!({"type": "vendor-doc"}), BadValue, "/sources"),
        (one(json!("a text")), BadValue, "/sources/0"),
        (one(json!({"url": "https://x.org"})), Missing, "/sources/0/type"),
        (json!([ok.clone(), {"license": "MIT"}]), Missing, "/sources/1/type"),
        (one(json!({"type": 5})), BadValue, "/sources/0/type"),
        (one(json!({"type": "guess"})), BadValue, "/sources/0/type"),
        (one(json!({"type": "x-acme"})), BadValue, "/sources/0/type"),
        (json!([ok.clone(), {"type": "knobkraft"}]), BadValue, "/sources/1/type"),
        (one(json!({"type": "vendor-doc", "url": 5})), BadValue, "/sources/0/url"),
        (one(json!({"type": "vendor-doc", "imported_at": "2026-01-01"})), UnknownField, "/sources/0/imported_at"),
        (json!([ok, {"type": "vendor-doc", "imported_at": 5}]), UnknownField, "/sources/1/imported_at"),
    ];
    for word in ["electra-preset", "pencilresearch", "midnam"] {
        out.push((one(json!({"type": word})), BadValue, "/sources/0/type"));
    }
    for provider in [json!("Electra One"), json!(""), json!("a--b"), json!("a".repeat(65)), json!(5), json!(null)] {
        out.push((one(json!({"type": "vendor-doc", "provider": provider})), BadValue, "/sources/0/provider"));
    }
    for license in ["none", "Public-Domain", "two words"] {
        out.push((one(json!({"type": "vendor-doc", "license": license})), BadValue, "/sources/0/license"));
    }
    for id in [json!(""), json!(5), json!(null)] {
        out.push((one(json!({"type": "vendor-doc", "id": id})), BadValue, "/sources/0/id"));
    }
    for profile in [json!(5), json!(null), json!(["acme"]), json!({"id": "acme"})] {
        out.push((one(json!({"type": "vendor-doc", "profile": profile})), BadValue, "/sources/0/profile"));
    }
    out
}

/// Shapes 0.7 accepts: every term, an extension, the special licences, a provider no table holds.
fn accepted() -> Vec<Value> {
    let mut out: Vec<Value> = [
        "hardware-verified", "hardware-observed", "software-verified", "vendor-doc", "vendor-osc-api",
        "vendor-self-described", "third-party-osc", "community-re", "contributed", "test-fixture", "x-acme-scraped",
    ]
    .iter()
    .map(|t| json!([{"type": t}]))
    .collect();
    for license in ["MIT", "mit", "public-domain", "NOASSERTION", "NONE"] {
        out.push(json!([{"type": "contributed", "license": license}]));
    }
    out.push(json!([{"type": "contributed", "provider": "a-new-firmware"}]));
    out.push(json!([{"type": "contributed", "provider": "a".repeat(64)}]));
    out.push(json!([full_source(), {"type": "vendor-doc"}, {"type": "vendor-doc"}, {"type": "vendor-doc", "id": "other"}]));
    out.push(json!([]));
    out
}

// ---- reading and writing back ----

#[test]
fn each_kind_of_0_7_reads_its_sources_and_writes_them_back_as_read() {
    for kind in KINDS {
        for list in accepted() {
            let doc = descriptor("0.7", kind, json!({"sources": list}));
            assert_eq!(written_back(&doc, kind), doc, "{kind:?}");
        }
    }
}

#[test]
fn each_kind_of_0_7_judges_a_source_as_a_device_does() {
    for kind in KINDS {
        for (list, fault, pointer) in refusals() {
            let doc = descriptor("0.7", kind, json!({"sources": list}));
            assert_eq!(refused(&doc, kind), (fault, pointer.to_string()), "{kind:?}: {doc}");
        }
    }
}

#[test]
fn an_id_written_twice_in_the_sources_of_a_file_is_refused_at_the_later_source_in_each_kind() {
    for kind in KINDS {
        let list = json!([{"type": "vendor-doc", "id": "a"}, {"type": "vendor-doc", "id": "b"}, {"type": "community-re", "id": "a"}]);
        let doc = descriptor("0.7", kind, json!({"sources": list}));
        assert_eq!(refused(&doc, kind), (FaultKind::BadValue, "/sources/2".to_string()), "{kind:?}");
    }
}

#[test]
fn each_kind_of_0_7_refuses_underscore_sources_as_bad_value_whatever_its_value() {
    for kind in KINDS {
        for value in [json!([{"type": "vendor-doc"}]), json!("a text"), json!(null), json!([]), json!({"x": 1})] {
            let alone = descriptor("0.7", kind, json!({"_sources": value}));
            assert_eq!(refused(&alone, kind), (FaultKind::BadValue, "/_sources".to_string()), "{kind:?}: {alone}");
            let beside = with(alone, json!({"sources": [{"type": "vendor-doc"}]}));
            assert_eq!(refused(&beside, kind), (FaultKind::BadValue, "/_sources".to_string()), "{kind:?}: {beside}");
        }
    }
}

#[test]
fn another_note_of_a_0_7_descriptor_is_kept_beside_its_sources() {
    for kind in KINDS {
        let doc = descriptor("0.7", kind, json!({"_note": {"any": ["shape", 1]}, "_sources_note": "kept"}));
        assert_eq!(written_back(&doc, kind), doc, "{kind:?}");
    }
}

// ---- the schemas ----

#[test]
fn the_0_7_schema_of_each_kind_accepts_the_sources_the_reader_accepts() {
    let schemas = Schemas::load("0.7");
    for kind in KINDS.into_iter().chain([Kind::Device]) {
        for list in accepted() {
            let doc = descriptor("0.7", kind, json!({"sources": list}));
            assert_eq!(schemas.check(kind, &doc), None, "{kind:?}: {doc}");
        }
    }
}

#[test]
fn the_0_7_schema_of_each_kind_refuses_the_sources_the_reader_refuses() {
    let schemas = Schemas::load("0.7");
    for kind in KINDS {
        // The kind holds the field: a refusal below is of the shape, not of `sources` itself.
        let good = descriptor("0.7", kind, json!({"sources": [{"type": "vendor-doc"}]}));
        assert_eq!(schemas.check(kind, &good), None, "{kind:?}");
        for (list, _, _) in refusals() {
            let doc = descriptor("0.7", kind, json!({"sources": list}));
            assert!(schemas.check(kind, &doc).is_some(), "{kind:?} accepts {doc}");
        }
        let old = descriptor("0.7", kind, json!({"_sources": []}));
        assert!(schemas.check(kind, &old).is_some(), "{kind:?} accepts _sources");
    }
}

#[test]
fn the_0_7_schema_of_a_device_refuses_a_profile_of_a_source_that_is_not_a_string() {
    let schemas = Schemas::load("0.7");
    for key in ["sources", "midi_of_sources"] {
        for profile in [json!(5), json!(null), json!([])] {
            let doc = descriptor("0.7", Kind::Device, json!({key: [{"type": "vendor-doc", "profile": profile}]}));
            assert!(schemas.check(Kind::Device, &doc).is_some(), "{doc}");
        }
    }
}

// ---- the profile a source comes from ----

#[test]
fn a_0_7_device_refuses_a_profile_of_a_source_that_is_not_a_string() {
    for key in ["sources", "midi_of_sources"] {
        for profile in [json!(5), json!(null), json!(["acme"])] {
            let doc = descriptor("0.7", Kind::Device, json!({key: [{"type": "vendor-doc"}, {"type": "vendor-doc", "profile": profile}]}));
            assert_eq!(refused(&doc, Kind::Device), (FaultKind::BadValue, format!("/{key}/1/profile")), "{doc}");
        }
    }
}

#[test]
fn a_profile_an_author_writes_in_a_source_is_kept_and_its_existence_never_checked() {
    for profile in ["acme", "acme/logue", "nobody/nothing", "not a profile id", ""] {
        let doc = descriptor("0.7", Kind::Device, json!({"sources": [{"type": "vendor-doc", "profile": profile}]}));
        assert_eq!(written_back(&doc, Kind::Device), doc);
        osd::resolve("acme/a", &common::credit::catalogue(&[("acme/a", doc)])).unwrap_or_else(|f| panic!("{f}"));
    }
}

#[test]
fn before_0_7_a_profile_in_a_source_of_a_device_is_a_key_kept_as_it_is() {
    for profile in [json!(5), json!(null), json!("acme"), json!({"x": 1})] {
        let doc = descriptor("0.6", Kind::Device, json!({"_sources": [{"type": "vendor-doc", "profile": profile}]}));
        assert_eq!(written_back(&doc, Kind::Device), doc);
    }
}

// ---- the warnings ----

#[test]
fn each_kind_of_0_7_warns_of_a_provider_the_table_does_not_hold_at_its_field() {
    for kind in KINDS {
        let list = json!([
            {"type": "vendor-doc", "provider": "electra-one"},
            {"type": "vendor-doc", "provider": "a-new-firmware"},
            {"type": "vendor-doc"},
            {"type": "contributed", "provider": "another-one"},
        ]);
        let doc = descriptor("0.7", kind, json!({"sources": list}));
        let want: Vec<(String, String)> = [1, 3].iter().map(|n| ("UnknownProvider".to_string(), format!("/sources/{n}/provider"))).collect();
        assert_eq!(warned(&doc, kind), want, "{}", word(kind));
    }
}

#[test]
fn the_warning_of_an_unknown_provider_names_it_in_a_sound_as_in_a_device() {
    let doc = descriptor("0.7", Kind::Sound, json!({"sources": [{"type": "vendor-doc", "provider": "a-new-firmware"}]}));
    let got = osd::warnings(&read(&doc, Kind::Sound));
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].kind, osd::WarningKind::UnknownProvider("a-new-firmware".into()));
}

#[test]
fn a_known_provider_and_a_descriptor_without_sources_give_no_warning() {
    for kind in KINDS {
        let known = descriptor("0.7", kind, json!({"sources": [{"type": "vendor-doc", "provider": "electra-one"}]}));
        assert_eq!(warned(&known, kind), [], "{kind:?}");
        assert_eq!(warned(&descriptor("0.7", kind, json!({})), kind), [], "{kind:?}");
    }
}

#[test]
fn each_kind_of_0_7_warns_of_each_entry_of_a_note_of_sources_not_carried_as_credit() {
    for kind in KINDS {
        let doc = descriptor("0.7", kind, json!({"sources": [{"type": "vendor-doc"}], "_sources_2": ["a text", {"type": "guess"}]}));
        let want = [("UncreditedSource".to_string(), "/_sources_2/0".to_string()), ("UncreditedSource".to_string(), "/_sources_2/1".to_string())];
        assert_eq!(warned(&doc, kind), want, "{kind:?}");
        assert_eq!(written_back(&doc, kind), doc, "{kind:?}: the note is kept");
    }
}

#[test]
fn a_note_of_sources_that_is_not_a_list_is_warned_of_at_the_note() {
    for kind in KINDS {
        for value in [json!("see the manual"), json!({"url": "https://x.org"})] {
            let doc = descriptor("0.7", kind, json!({"_sources_2": value}));
            assert_eq!(warned(&doc, kind), [("UncreditedSource".to_string(), "/_sources_2".to_string())], "{kind:?}");
        }
    }
}

#[test]
fn a_note_of_sources_before_0_7_gives_no_warning() {
    for kind in KINDS {
        let doc = descriptor("0.6", kind, json!({"_sources": ["a text"], "_sources_2": ["a text"]}));
        assert_eq!(warned(&doc, kind), [], "{kind:?}");
    }
}

// ---- the assembly a sound was made on ----

fn sound_made_on(extra: Value) -> Value {
    descriptor("0.7", Kind::Sound, json!({"made_on": made_on(extra)}))
}

#[test]
fn the_assembly_a_0_7_sound_was_made_on_carries_sources_written_back_as_read() {
    let doc = sound_made_on(json!({"sources": [full_source()]}));
    assert_eq!(written_back(&doc, Kind::Sound), doc);
    assert_eq!(Schemas::load("0.7").check(Kind::Sound, &doc), None);
}

#[test]
fn the_sources_of_the_assembly_a_sound_was_made_on_are_judged_as_an_assembly_judges_them() {
    let schemas = Schemas::load("0.7");
    assert_eq!(schemas.check(Kind::Sound, &sound_made_on(json!({"sources": [{"type": "vendor-doc"}]}))), None);
    for (list, fault, pointer) in refusals() {
        let doc = sound_made_on(json!({"sources": list}));
        assert_eq!(refused(&doc, Kind::Sound), (fault, format!("/made_on{pointer}")), "{doc}");
        assert!(schemas.check(Kind::Sound, &doc).is_some(), "{doc}");
    }
    let twice = sound_made_on(json!({"sources": [{"type": "vendor-doc", "id": "a"}, {"type": "vendor-doc", "id": "a"}]}));
    assert_eq!(refused(&twice, Kind::Sound), (FaultKind::BadValue, "/made_on/sources/1".to_string()));
}

#[test]
fn an_id_of_a_sound_written_again_in_the_assembly_it_was_made_on_is_refused_at_the_later_source() {
    let source = |id: &str| json!({"type": "vendor-doc", "id": id});
    let apart = with(sound_made_on(json!({"sources": [source("b")]})), json!({"sources": [source("a")]}));
    read(&apart, Kind::Sound);
    let twice = with(sound_made_on(json!({"sources": [source("b"), source("a")]})), json!({"sources": [source("a")]}));
    assert_eq!(refused(&twice, Kind::Sound), (FaultKind::BadValue, "/made_on/sources/1".to_string()));
}

#[test]
fn underscore_sources_in_the_assembly_a_0_7_sound_was_made_on_is_refused_at_its_key() {
    let schemas = Schemas::load("0.7");
    for value in [json!([{"type": "vendor-doc"}]), json!("a text"), json!(null)] {
        let doc = sound_made_on(json!({"_sources": value}));
        assert_eq!(refused(&doc, Kind::Sound), (FaultKind::BadValue, "/made_on/_sources".to_string()), "{doc}");
        assert!(schemas.check(Kind::Sound, &doc).is_some(), "{doc}");
    }
}

#[test]
fn the_assembly_a_sound_was_made_on_is_warned_of_like_an_assembly() {
    let provider = sound_made_on(json!({"sources": [{"type": "vendor-doc", "provider": "a-new-firmware"}]}));
    assert_eq!(warned(&provider, Kind::Sound), [("UnknownProvider".to_string(), "/made_on/sources/0/provider".to_string())]);
    let note = sound_made_on(json!({"_sources_2": ["a text"]}));
    assert_eq!(warned(&note, Kind::Sound), [("UncreditedSource".to_string(), "/made_on/_sources_2/0".to_string())]);
}

// ---- before 0.7 ----

/// A descriptor of each kind in each version before 0.7 that has it, the examples for 0.4.
fn older() -> Vec<(Kind, &'static str, Value)> {
    let example = |path: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(common::repo().join(path)).unwrap()).unwrap() };
    let mut out = vec![
        (Kind::Sound, "0.4", example("examples/0.4/warm-pad.sound.json")),
        (Kind::Fingerprint, "0.4", example("examples/0.4/minilogue.fingerprint.json")),
        (Kind::Firmware, "0.4", example("examples/0.4/tribe-0.3-beta.firmware.json")),
        // A 0.4 profile names no `$schema`.
        (Kind::Profile, "0.4", json!({"identity": {"reply_text": "acme"}})),
        (Kind::Profile, "0.5", example("examples/0.5/acme.profile.json")),
        (Kind::Sound, "0.5", example("examples/0.5/warm-pad.sound.json")),
        (Kind::Fingerprint, "0.5", example("examples/0.5/minilogue.fingerprint.json")),
        (Kind::Firmware, "0.5", example("examples/0.5/tribe-0.3-beta.firmware.json")),
        (Kind::Profile, "0.6", descriptor("0.6", Kind::Profile, json!({}))),
    ];
    for (kind, path) in [
        (Kind::Sound, "examples/0.6/warm-pad.sound.json"),
        (Kind::Fingerprint, "examples/0.6/minilogue.fingerprint.json"),
        (Kind::Firmware, "examples/0.6/tribe-0.3-beta.firmware.json"),
        (Kind::Assembly, "examples/0.6/tribe-on-fm1.assembly.json"),
    ] {
        out.push((kind, "0.6", example(path)));
    }
    out
}

#[test]
fn before_0_7_sources_is_an_unknown_field_of_each_kind() {
    for (kind, version, doc) in older() {
        read(&doc, kind);
        let doc = with(doc, json!({"sources": [{"type": "vendor-doc"}]}));
        assert_eq!(refused(&doc, kind), (FaultKind::UnknownField, "/sources".to_string()), "{kind:?} {version}");
    }
    let mut sound = descriptor("0.6", Kind::Sound, json!({}));
    sound["made_on"] = made_on(json!({"sources": [{"type": "vendor-doc"}]}));
    assert_eq!(refused(&sound, Kind::Sound), (FaultKind::UnknownField, "/made_on/sources".to_string()));
}

#[test]
fn before_0_7_underscore_sources_is_a_note_of_any_value_read_written_back_and_converted_as_it_is() {
    let values = [
        json!("a text"), json!(null), json!([]), json!({"x": 1}), json!([{"url": "https://x.org"}]),
        json!([{"type": "knobkraft", "name": "N", "profile": 5}, "a text"]),
    ];
    for (kind, version, doc) in older() {
        for value in &values {
            let doc = with(doc.clone(), json!({"_sources": value}));
            assert_eq!(written_back(&doc, kind), doc, "{kind:?} {version}");
            for (to, target) in [("0.5", Version::V0_5), ("0.6", Version::V0_6)] {
                if to < version || (kind == Kind::Assembly && to == "0.5") {
                    continue;
                }
                let out = osd::convert_to(&doc.to_string(), kind, target).unwrap_or_else(|f| panic!("{f} in {doc}"));
                let out: Value = serde_json::from_str(&out).unwrap();
                assert_eq!(out["_sources"], *value, "{kind:?} {version} to {to}");
                assert!(out.get("sources").is_none(), "{out}");
            }
        }
    }
}

#[test]
fn the_older_schemas_still_accept_any_value_of_underscore_sources() {
    let schemas = Schemas::load("0.6");
    for kind in KINDS {
        let doc = descriptor("0.6", kind, json!({"_sources": "a text"}));
        assert_eq!(schemas.check(kind, &doc), None, "{}", address("0.6", kind));
    }
}
