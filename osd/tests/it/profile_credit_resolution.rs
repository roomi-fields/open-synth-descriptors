//! A resolution whose result is 0.7 keeps the credit of the maker profiles it takes blocks from
//! (`spec/0.7.md`, section Resolution): their sources, each marked with its profile, go to
//! `sources` for a block the file writes, to `midi_of_sources` for a block of a `midi_of` target.

use crate::common::credit::{catalogue, descriptor, with};
use crate::common::Schemas;

use osd::{Descriptor, FaultKind, Kind, Resolved};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// A source of 0.7 titled `title`.
fn src(title: &str) -> Value {
    json!({"type": "vendor-doc", "title": title})
}

fn src_id(title: &str, id: &str) -> Value {
    with(src(title), json!({"id": id}))
}

/// `source` as a resolution gives it for the profile `profile`.
fn from(source: Value, profile: &str) -> Value {
    with(source, json!({"profile": profile}))
}

fn device(extra: Value) -> Value {
    descriptor("0.7", Kind::Device, extra)
}

fn routes() -> Value {
    json!({"entries": [{"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]})
}

/// A 0.7 profile that gives the blocks of `blocks`, with `sources`.
fn profile(blocks: Value, sources: Value) -> Value {
    with(json!({"$schema": "https://w3id.org/osd/0.7/profile.schema.json", "sources": sources}), blocks)
}

fn identity() -> Value {
    json!({"identity": {"reply_text": "acme"}})
}

fn programs() -> Value {
    json!({"programs": {"banks": [{"name": "A", "size": 128}]}})
}

fn both() -> Value {
    with(identity(), programs())
}

fn names(profile: &str) -> Value {
    json!({"profile": profile})
}

fn resolve(id: &str, files: &BTreeMap<String, String>) -> Resolved {
    osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"))
}

fn json_of(d: &Descriptor) -> Value {
    serde_json::from_str(&osd::write(d)).unwrap()
}

/// The resolved device as written.
fn written(r: &Resolved) -> Value {
    json_of(&Descriptor::Device(Box::new(r.device.clone())))
}

fn resolved(id: &str, files: &[(&str, Value)]) -> Value {
    written(&resolve(id, &catalogue(files)))
}

/// The two lists of credit of a device, an absent one as null.
fn credit(doc: &Value) -> (Value, Value) {
    (doc["sources"].clone(), doc["midi_of_sources"].clone())
}

// ---- a block the file writes ----

#[test]
fn the_sources_of_a_profile_follow_the_files_own_each_marked_with_its_profile() {
    let doc = resolved(
        "acme/synth",
        &[
            ("acme/synth", device(json!({"identity": names("acme"), "sources": [src("own")]}))),
            ("acme/_profile", profile(identity(), json!([src("P1"), src("P2")]))),
        ],
    );
    assert_eq!(credit(&doc), (json!([src("own"), from(src("P1"), "acme"), from(src("P2"), "acme")]), Value::Null));
}

#[test]
fn a_file_without_sources_gets_the_list_from_its_profile() {
    let source = json!({"type": "community-re", "url": "https://x.org", "author": "Dee", "license": "CC-BY-4.0", "provider": "acme-synths", "x-other": [1]});
    let doc = resolved(
        "acme/synth",
        &[("acme/synth", device(json!({"programs": names("acme")}))), ("acme/_profile", profile(programs(), json!([source.clone()])))],
    );
    assert_eq!(credit(&doc), (json!([from(source, "acme")]), Value::Null));
}

#[test]
fn the_profiles_are_credited_identity_first_the_nearest_first_each_profile_once() {
    let doc = resolved(
        "acme/synth",
        &[
            ("acme/synth", device(json!({"programs": names("acme/pads"), "identity": names("acme/keys"), "sources": [src("own")]}))),
            ("acme/_profile.keys", profile(json!({"identity": names("acme")}), json!([src("keys")]))),
            ("acme/_profile.pads", profile(json!({"programs": names("acme")}), json!([src("pads")]))),
            ("acme/_profile", profile(both(), json!([src("maker 1"), src("maker 2")]))),
        ],
    );
    let want = json!([
        src("own"),
        from(src("keys"), "acme/keys"),
        from(src("maker 1"), "acme"),
        from(src("maker 2"), "acme"),
        from(src("pads"), "acme/pads"),
    ]);
    assert_eq!(credit(&doc), (want, Value::Null));
}

#[test]
fn a_profile_without_sources_adds_nothing_and_the_chain_goes_on() {
    let silent = json!({"$schema": "https://w3id.org/osd/0.7/profile.schema.json", "identity": {"profile": "acme"}});
    let files = [
        ("acme/synth", device(json!({"identity": names("acme/keys")}))),
        ("acme/_profile.keys", silent.clone()),
        ("acme/_profile", profile(identity(), json!([src("maker")]))),
    ];
    assert_eq!(credit(&resolved("acme/synth", &files)), (json!([from(src("maker"), "acme")]), Value::Null));
    let none = resolved("acme/synth", &[("acme/synth", device(json!({"identity": names("acme")}))), ("acme/_profile", json!({"$schema": silent["$schema"], "identity": {"reply_text": "acme"}}))]);
    assert_eq!(credit(&none), (Value::Null, Value::Null));
}

#[test]
fn the_profile_a_resolution_writes_replaces_one_the_profiles_source_held() {
    let doc = resolved(
        "acme/synth",
        &[("acme/synth", device(json!({"identity": names("acme")}))), ("acme/_profile", profile(identity(), json!([from(src("P"), "elsewhere")])))],
    );
    assert_eq!(doc["sources"], json!([from(src("P"), "acme")]));
}

// ---- profiles of 0.6 ----

/// The `_sources` of a 0.6 profile and the sources a resolution of 0.7 takes from them.
fn old_credit() -> (Value, Value) {
    let before = json!([
        {"type": "knobkraft", "name": "K", "notes": "n"},
        {"url": "https://x.org/a"},
        "a text",
        {"type": "guess"},
    ]);
    let after = json!([
        from(json!({"type": "contributed", "provider": "knobkraft", "title": "K", "notes": "type in 0.6: knobkraft\nn"}), "acme"),
        from(json!({"type": "contributed", "url": "https://x.org/a", "notes": "no type in 0.6"}), "acme"),
    ]);
    (before, after)
}

#[test]
fn a_0_6_profile_joined_into_a_0_7_result_gives_the_sources_its_underscore_sources_converts_to() {
    let (before, after) = old_credit();
    let old = descriptor("0.6", Kind::Profile, json!({"_sources": before}));
    let doc = resolved("acme/synth", &[("acme/synth", device(json!({"identity": names("acme")}))), ("acme/_profile", old)]);
    assert_eq!(credit(&doc), (after, Value::Null));
    assert!(doc.as_object().unwrap().keys().all(|k| !k.starts_with("_sources")), "{doc}");
}

#[test]
fn a_0_4_and_a_0_5_profile_joined_into_a_0_7_result_are_credited_by_the_tables_of_their_version() {
    for (old, term) in [
        (json!({"identity": {"reply_text": "acme"}, "_sources": [{"type": "vendor", "license": "none"}]}), json!({"type": "vendor-doc", "license": "NONE"})),
        (
            descriptor("0.5", Kind::Profile, json!({"_sources": [{"type": "midnam", "revision": "2"}]})),
            json!({"type": "contributed", "version": "2", "notes": "type in 0.6: midnam"}),
        ),
    ] {
        let doc = resolved("acme/synth", &[("acme/synth", device(json!({"identity": names("acme")}))), ("acme/_profile", old)]);
        assert_eq!(credit(&doc), (json!([from(term, "acme")]), Value::Null));
    }
}

#[test]
fn a_0_6_device_over_a_0_7_profile_has_a_0_7_result_with_both_credits() {
    let file = descriptor("0.6", Kind::Device, json!({"identity": names("acme"), "_sources": [{"type": "vendor-doc", "name": "own"}]}));
    let doc = resolved("acme/synth", &[("acme/synth", file), ("acme/_profile", profile(identity(), json!([src("P")])))]);
    assert_eq!(doc["$schema"], "https://w3id.org/osd/0.7/device.schema.json");
    assert_eq!(credit(&doc), (json!([src("own"), from(src("P"), "acme")]), Value::Null));
}

#[test]
fn a_0_6_profile_beside_a_0_7_profile_is_credited_in_the_0_7_result() {
    let old = descriptor("0.6", Kind::Profile, json!({"programs": {"banks": [{"name": "A", "size": 128}]}, "_sources": [{"type": "vendor-doc", "name": "banks"}]}));
    let doc = resolved(
        "acme/synth",
        &[
            ("acme/synth", descriptor("0.6", Kind::Device, json!({"identity": names("acme"), "programs": names("acme/banks")}))),
            ("acme/_profile", profile(identity(), json!([src("maker")]))),
            ("acme/_profile.banks", old),
        ],
    );
    assert_eq!(doc["sources"], json!([from(src("maker"), "acme"), from(src("banks"), "acme/banks")]));
}

#[test]
fn a_resolution_with_no_text_of_0_7_adds_no_source_of_a_profile() {
    for version in ["0.5", "0.6"] {
        let file = descriptor(version, Kind::Device, json!({"identity": names("acme"), "_sources": [{"type": "vendor-doc"}]}));
        let old = descriptor(version, Kind::Profile, json!({"_sources": [{"type": "community-re"}]}));
        let doc = resolved("acme/synth", &[("acme/synth", file), ("acme/_profile", old.clone())]);
        assert_eq!(doc["_sources"], json!([{"type": "vendor-doc"}]), "{version}");
        assert!(doc.get("sources").is_none() && doc.get("_midi_of_sources").is_none() && doc.get("midi_of_sources").is_none(), "{doc}");
        let file = descriptor(version, Kind::Device, json!({"identity": names("acme")}));
        let doc = resolved("acme/synth", &[("acme/synth", file), ("acme/_profile", old)]);
        assert!(doc.as_object().unwrap().keys().all(|k| !k.contains("sources")), "{doc}");
    }
}

// ---- a block taken from a midi_of target ----

#[test]
fn the_sources_of_a_profile_of_an_inherited_block_follow_the_targets_in_midi_of_sources() {
    let files = [
        ("acme/a", device(json!({"midi_of": "acme/b", "sources": [src("A")]}))),
        ("acme/b", device(json!({"midi_of": "acme/c", "sources": [src("B")]}))),
        ("acme/c", device(json!({"identity": names("acme"), "cc_params": routes(), "sources": [src("C")]}))),
        ("acme/_profile", profile(identity(), json!([src("P")]))),
    ];
    assert_eq!(credit(&resolved("acme/a", &files)), (json!([src("A")]), json!([src("B"), src("C"), from(src("P"), "acme")])));
}

#[test]
fn an_inherited_block_creates_midi_of_sources_and_gives_the_file_no_sources() {
    let files = [
        ("acme/a", device(json!({"midi_of": "acme/b"}))),
        ("acme/b", device(json!({"programs": names("acme"), "cc_params": routes()}))),
        ("acme/_profile", profile(programs(), json!([src("P")]))),
    ];
    assert_eq!(credit(&resolved("acme/a", &files)), (Value::Null, json!([from(src("P"), "acme")])));
}

#[test]
fn the_profiles_come_after_the_midi_of_sources_a_target_carries() {
    let files = [
        ("acme/a", device(json!({"midi_of": "acme/b"}))),
        ("acme/b", device(json!({"identity": names("acme"), "cc_params": routes(), "sources": [src("B")], "midi_of_sources": [src("M")]}))),
        ("acme/_profile", profile(identity(), json!([src("P")]))),
    ];
    assert_eq!(resolved("acme/a", &files)["midi_of_sources"], json!([src("B"), src("M"), from(src("P"), "acme")]));
}

#[test]
fn an_own_block_credits_in_sources_and_an_inherited_one_in_midi_of_sources() {
    let files = [
        ("acme/a", device(json!({"midi_of": "acme/b", "programs": names("acme/mine")}))),
        ("acme/b", device(json!({"identity": names("acme/theirs"), "cc_params": routes(), "sources": [src("B")]}))),
        ("acme/_profile.mine", profile(programs(), json!([src("mine")]))),
        ("acme/_profile.theirs", profile(identity(), json!([src("theirs")]))),
    ];
    let want = (json!([from(src("mine"), "acme/mine")]), json!([src("B"), from(src("theirs"), "acme/theirs")]));
    assert_eq!(credit(&resolved("acme/a", &files)), want);
}

#[test]
fn a_profile_of_an_own_and_of_an_inherited_block_is_credited_once_in_sources() {
    for (own, inherited) in [("identity", "programs"), ("programs", "identity")] {
        let files = [
            ("acme/a", device(json!({"midi_of": "acme/b", own: names("acme")}))),
            ("acme/b", device(json!({inherited: names("acme"), "cc_params": routes()}))),
            ("acme/_profile", profile(both(), json!([src("P")]))),
        ];
        assert_eq!(credit(&resolved("acme/a", &files)), (json!([from(src("P"), "acme")]), Value::Null), "own {own}");
    }
}

// ---- the ids ----

#[test]
fn a_profile_source_whose_id_the_file_holds_is_given_without_it() {
    let doc = resolved(
        "acme/synth",
        &[
            ("acme/synth", device(json!({"identity": names("acme"), "sources": [src_id("own", "manual")]}))),
            ("acme/_profile", profile(identity(), json!([src_id("P", "manual"), src_id("Q", "other")]))),
        ],
    );
    assert_eq!(doc["sources"], json!([src_id("own", "manual"), from(src("P"), "acme"), from(src_id("Q", "other"), "acme")]));
}

#[test]
fn a_profile_source_whose_id_midi_of_sources_holds_is_given_without_it() {
    let files = [
        ("acme/a", device(json!({"midi_of": "acme/b", "identity": names("acme")}))),
        ("acme/b", device(json!({"cc_params": routes(), "sources": [src_id("B", "manual")]}))),
        ("acme/_profile", profile(identity(), json!([src_id("P", "manual")]))),
    ];
    assert_eq!(credit(&resolved("acme/a", &files)), (json!([from(src("P"), "acme")]), json!([src_id("B", "manual")])));
}

#[test]
fn the_second_profile_source_of_an_id_is_given_without_it() {
    let files = [
        ("acme/synth", device(json!({"identity": names("acme/keys")}))),
        ("acme/_profile.keys", profile(json!({"identity": names("acme")}), json!([src_id("keys", "manual")]))),
        ("acme/_profile", profile(identity(), json!([src_id("maker", "manual")]))),
    ];
    let want = json!([from(src_id("keys", "manual"), "acme/keys"), from(src("maker"), "acme")]);
    assert_eq!(resolved("acme/synth", &files)["sources"], want);
}

#[test]
fn a_reading_names_a_source_of_the_file_whose_id_a_profile_holds_too() {
    let file = device(json!({
        "identity": names("acme"), "sources": [src_id("own", "manual")],
        "commands": [{"osc": "/dump", "readings": [{"state": "valid", "firmware": "1", "source": "manual"}]}],
    }));
    let doc = resolved("acme/synth", &[("acme/synth", file), ("acme/_profile", profile(identity(), json!([src_id("P", "manual")])))]);
    assert_eq!(doc["commands"][0]["readings"][0]["source"], "manual");
    assert_eq!(doc["sources"], json!([src_id("own", "manual"), from(src("P"), "acme")]));
}

#[test]
fn a_reading_never_names_a_source_of_a_profile() {
    let file = device(json!({
        "midi_of": "acme/b", "identity": names("acme"),
        "commands": [{"osc": "/dump", "readings": [{"state": "valid", "firmware": "1", "source": "from-profile"}]}],
    }));
    let files = catalogue(&[
        ("acme/a", file),
        ("acme/b", device(json!({"cc_params": routes()}))),
        ("acme/_profile", profile(identity(), json!([src_id("P", "from-profile")]))),
    ]);
    let f = osd::resolve("acme/a", &files).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str(), f.file.as_deref()), (FaultKind::BadValue, "/commands/0/readings/0/source", Some("acme/a")));
}

// ---- what is already credited ----

#[test]
fn a_profile_source_the_file_already_holds_in_every_key_but_id_is_not_added_again() {
    let own = from(src_id("P", "mine"), "acme");
    let doc = resolved(
        "acme/synth",
        &[
            ("acme/synth", device(json!({"identity": names("acme"), "sources": [own.clone()]}))),
            ("acme/_profile", profile(identity(), json!([src_id("P", "theirs"), src("Q")]))),
        ],
    );
    assert_eq!(doc["sources"], json!([own, from(src("Q"), "acme")]));
}

#[test]
fn a_source_that_differs_in_any_other_key_hides_no_profile_source() {
    for held in [src("P"), from(src("P"), "acme/other"), from(with(src("P"), json!({"notes": "n"})), "acme"), from(src("p"), "acme")] {
        let doc = resolved(
            "acme/synth",
            &[
                ("acme/synth", device(json!({"identity": names("acme"), "sources": [held.clone()]}))),
                ("acme/_profile", profile(identity(), json!([src("P")]))),
            ],
        );
        assert_eq!(doc["sources"], json!([held, from(src("P"), "acme")]));
    }
}

#[test]
fn two_equal_sources_of_one_profile_are_both_credited() {
    let doc = resolved(
        "acme/synth",
        &[("acme/synth", device(json!({"identity": names("acme")}))), ("acme/_profile", profile(identity(), json!([src("P"), src("P")])))],
    );
    assert_eq!(doc["sources"], json!([from(src("P"), "acme"), from(src("P"), "acme")]));
}

#[test]
fn a_profile_source_midi_of_sources_already_holds_is_not_added_again() {
    let files = [
        ("acme/a", device(json!({"midi_of": "acme/b", "identity": names("acme")}))),
        ("acme/b", device(json!({"cc_params": routes(), "sources": [from(src("P"), "acme")]}))),
        ("acme/_profile", profile(identity(), json!([src("P")]))),
    ];
    assert_eq!(credit(&resolved("acme/a", &files)), (Value::Null, json!([from(src("P"), "acme")])));
}

/// A catalogue whose driver takes its identity from a profile.
fn stock() -> Vec<(&'static str, Value)> {
    vec![
        ("acme/synth", device(json!({"identity": names("acme"), "cc_params": routes(), "sources": [src("S")]}))),
        ("acme/_profile", profile(identity(), json!([src_id("P", "p")]))),
    ]
}

#[test]
fn resolving_a_resolved_device_again_adds_no_repeat() {
    let mut files = stock();
    let once = resolved("acme/synth", &files);
    assert_eq!(once["sources"], json!([src("S"), from(src_id("P", "p"), "acme")]));
    files[0].1 = once.clone();
    assert_eq!(resolved("acme/synth", &files), once);
}

#[test]
fn a_resolved_device_as_a_midi_of_target_adds_no_repeat() {
    let mut files = stock();
    let once = resolved("acme/synth", &files);
    files[0].1 = once.clone();
    files.push(("acme/variant", device(json!({"midi_of": "acme/synth"}))));
    assert_eq!(credit(&resolved("acme/variant", &files)), (Value::Null, once["sources"].clone()));
}

/// The catalogue of `stock` with an assembly of one module, whose MIDI is `module`.
fn assembly(module: Option<Value>) -> BTreeMap<String, String> {
    let target = |midi: &str| json!([{"device": {"descriptor": "acme/synth", "vendor": "Acme", "model": "Synth"}, "midi": {"descriptor": midi}}]);
    let firmware = |name: &str, kind: &str, midi: &str| descriptor("0.7", Kind::Firmware, json!({"name": name, "version": "1.0", "kind": kind, "targets": target(midi)}));
    let modules = if module.is_some() { json!([{"module": "acme/mod", "version": "1.0"}]) } else { json!([]) };
    let mut files = stock();
    files.push(("acme/build", descriptor("0.7", Kind::Assembly, json!({"modules": modules}))));
    files.push(("acme/stock", firmware("Stock", "firmware", "acme/synth")));
    if let Some(midi) = module {
        files.push(("acme/mod", firmware("Mod", "module", "acme/mod-midi")));
        files.push(("acme/mod-midi", midi));
    }
    catalogue(&files)
}

fn assembled(module: Option<Value>) -> Value {
    written(&osd::resolve_assembly("acme/build", None, &assembly(module)).unwrap_or_else(|f| panic!("{f}")))
}

#[test]
fn an_assembly_without_modules_credits_the_profile_of_its_stock_midi_once() {
    assert_eq!(credit(&assembled(None)), (json!([src("S"), from(src_id("P", "p"), "acme")]), Value::Null));
}

#[test]
fn a_module_applied_over_midi_already_credited_adds_no_repeat_of_a_profile_source() {
    let added = json!({"entries": [{"osc": "/y", "ways": [{"ctrlType": "cc", "ctrlIndex": [2]}]}]});
    for own in [json!({}), json!({"identity": names("acme")})] {
        let module = with(device(json!({"midi_of": "acme/synth", "cc_params": added})), own);
        let doc = assembled(Some(module));
        assert_eq!(credit(&doc), (Value::Null, json!([src("S"), from(src_id("P", "p"), "acme")])), "{doc}");
    }
}

/// The module file `acme/mod-midi`: one route of its own over the stock MIDI, and the keys of
/// `extra`.
fn module_midi(extra: Value) -> Value {
    let added = json!({"entries": [{"osc": "/y", "ways": [{"ctrlType": "cc", "ctrlIndex": [2]}]}]});
    with(device(json!({"midi_of": "acme/synth", "cc_params": added})), extra)
}

/// `extra` with a command that has a reading naming the source `id`.
fn reading_of(id: &str, extra: Value) -> Value {
    with(json!({"commands": [{"osc": "/dump", "readings": [{"state": "valid", "firmware": "1", "source": id}]}]}), extra)
}

#[test]
fn a_profile_source_whose_id_a_module_file_holds_is_given_without_it_and_the_assembly_resolves() {
    // The assembly keeps the identity of the stock MIDI, which credited P first.
    let applied = (json!([src_id("M", "p")]), json!([src("S"), from(src("P"), "acme")]));
    // The module file resolved alone over the stock MIDI gives the same credit, except when it
    // writes the block that names the profile: P is then among its own sources.
    let written_by_the_module = (json!([src_id("M", "p"), from(src("P"), "acme")]), json!([src("S")]));
    for (own, alone) in [(json!({}), applied.clone()), (json!({"identity": names("acme")}), written_by_the_module)] {
        let module = module_midi(with(json!({"sources": [src_id("M", "p")]}), own));
        let doc = assembled(Some(module.clone()));
        assert_eq!(credit(&doc), applied, "{doc}");
        let mut files = stock();
        files.push(("acme/mod-midi", module));
        assert_eq!(credit(&resolved("acme/mod-midi", &files)), alone);
    }
}

#[test]
fn a_reading_of_a_module_file_never_names_a_source_of_a_profile() {
    for own in [json!({}), json!({"identity": names("acme")}), json!({"sources": [src_id("M", "m")]})] {
        let files = assembly(Some(module_midi(reading_of("p", own.clone()))));
        for f in [osd::resolve_assembly("acme/build", None, &files).map(|_| ()), osd::resolve("acme/mod-midi", &files).map(|_| ())] {
            let f = f.expect_err(&format!("refused: {own}"));
            let got = (f.kind, f.pointer.as_str(), f.file.as_deref());
            assert_eq!(got, (FaultKind::BadValue, "/commands/0/readings/0/source", Some("acme/mod-midi")), "{own}");
        }
    }
}

#[test]
fn a_reading_of_a_module_file_names_a_source_of_the_stock_midi_file_or_its_own() {
    // A source of the stock file that holds `profile`, written by its author, is the file's.
    for (id, held) in [("s", src_id("S", "s")), ("a", from(src_id("A", "a"), "acme/other")), ("p", src_id("S", "p"))] {
        let mut files = assembly(Some(module_midi(reading_of(id, json!({})))));
        let synth = device(json!({"identity": names("acme"), "cc_params": routes(), "sources": [held]}));
        files.insert("acme/synth".into(), synth.to_string());
        let r = osd::resolve_assembly("acme/build", None, &files).unwrap_or_else(|f| panic!("{id}: {f}"));
        assert_eq!(written(&r)["commands"][0]["readings"][0]["source"], id);
    }
    let own = reading_of("m", json!({"sources": [src_id("M", "m")]}));
    assert_eq!(assembled(Some(module_midi(own)))["commands"][0]["readings"][0]["source"], "m");
}

#[test]
fn a_module_file_that_repeats_an_id_of_a_source_of_the_stock_midi_file_is_refused() {
    let mut files = assembly(Some(module_midi(json!({"sources": [src_id("M", "s")]}))));
    let synth = device(json!({"identity": names("acme"), "cc_params": routes(), "sources": [src_id("S", "s")]}));
    files.insert("acme/synth".into(), synth.to_string());
    let alone = osd::resolve("acme/mod-midi", &files).expect_err("refused alone");
    let applied = osd::resolve_assembly("acme/build", None, &files).expect_err("refused in the assembly");
    assert_eq!((applied.kind, applied.pointer), (alone.kind, alone.pointer));
}

// ---- the complete descriptor ----

#[test]
fn the_complete_descriptor_carries_the_profile_sources_reads_back_and_its_schema_accepts_them() {
    let files = catalogue(&[
        ("acme/a", device(json!({"midi_of": "acme/b", "programs": names("acme/mine"), "sources": [src_id("A", "a")]}))),
        ("acme/b", device(json!({"identity": names("acme/theirs"), "cc_params": routes()}))),
        ("acme/_profile.mine", profile(programs(), json!([src_id("mine", "a"), src_id("m2", "m2")]))),
        ("acme/_profile.theirs", profile(identity(), json!([src("theirs")]))),
    ]);
    let r = resolve("acme/a", &files);
    let full = json_of(&osd::complete_descriptor(&r));
    let want = (
        json!([src_id("A", "a"), from(src("mine"), "acme/mine"), from(src_id("m2", "m2"), "acme/mine")]),
        json!([from(src("theirs"), "acme/theirs")]),
    );
    assert_eq!(credit(&written(&r)), want);
    assert_eq!(credit(&full), want);
    assert!(full.get("midi_of").is_none() && full["programs"].get("profile").is_none(), "{full}");
    let back = osd::read(&full.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f} in {full}"));
    assert_eq!(json_of(&back), full);
    assert_eq!(Schemas::load("0.7").check(Kind::Device, &full), None);
}
