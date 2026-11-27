//! What the trial of 0.7 compares of the credit of a maker profile: every field of every source of
//! its `_sources` that 0.7 reads, against the `sources` of the converted text, carried as a
//! device's (`spec/0.7.md` 5.3, 5.3.2, 6.8.4), the words of a profile given in 0.4 going first
//! through the tables of 6.6 and 6.7. A list of no entry that the conversion of any kind but the device leaves
//! out is no difference. Each case is the converted text of a small catalogue, left as it is or
//! altered, given to the trial.

use crate::catalogue_trial::run;
use crate::catalogue_trial_reference::{
    added, alter, assert_each_named, differences, new_text, removed, set, texts, Alter, JUDGED,
};
use crate::common::credit::{descriptor, made_on, KINDS};
use crate::common::trial::judge_from;
use crate::common::{reference_0_6, Schemas};
use osd::{Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const P05: &str = "https://w3id.org/osd/0.5/profile.schema.json";
const P06: &str = "https://w3id.org/osd/0.6/profile.schema.json";

/// A profile of the version of `schema` (none: 0.4, with no `$schema`) with these sources.
fn profile(schema: Option<&str>, sources: Value) -> Value {
    let mut doc = json!({ "identity": { "reply_text": "acme" }, "_sources": sources });
    if let Some(schema) = schema {
        doc["$schema"] = schema.into();
    }
    doc
}

/// A profile of 0.6 with every writing of a source the conversion to 0.7 changes, and a
/// `_midi_of_sources`, which stays a note whatever it holds.
fn rich_profile() -> Value {
    let mut doc = profile(
        Some(P06),
        json!([
            { "type": "electra-preset", "preset_name": "Preset A", "name": "Acme A", "revision": "3", "content_sha256": "aa",
              "preset_id": "abc", "adaptation": "Other", "url": "https://x.org/p/abc?v=1#top", "notes": "kept",
              "author": "someone", "extra/key": { "a": 1 } },
            { "type": "knobkraft", "name": "Only", "url": "https://x.org/a/Acme/", "adaptation": "Acme", "preset_id": "zzz" },
            { "type": "vendor-doc", "title": "T", "preset_name": "P", "name": "N", "version": "1", "revision": "2",
              "sha256": "bb", "content_sha256": "cc", "url": "https://x.org/p/ABC", "license": "NONE" },
            { "type": "midnam", "license": "NOASSERTION" },
            { "type": "pencilresearch", "provider": "acme-co" },
            { "type": "contributed", "provider": "Not Well Formed", "_provider": "taken" },
            { "type": "knobkraft", "provider": 7 },
            { "type": "community-re", "imported_at": "2026-01-01" },
            { "type": "community-re", "imported_at": "2026-02-02", "_imported_at": "taken" }
        ]),
    );
    doc["_midi_of_sources"] = json!([{ "type": "vendor", "preset_name": "stays", "imported_at": "x", "license": "none" }]);
    doc
}

/// A profile of 0.5 with sources the conversion changes.
fn profile_of_0_5() -> Value {
    profile(
        Some(P05),
        json!([
            { "type": "pencilresearch", "revision": "3", "imported_at": "2026-01-01" },
            { "type": "vendor-doc", "name": "Manual", "content_sha256": "ab", "url": "https://x.org/m/abc", "preset_id": "abc" }
        ]),
    )
}

/// A profile given in 0.4 with each word the tables of 0.4 to 0.5 rewrite, and words they keep.
fn profile_of_0_4() -> Value {
    profile(
        None,
        json!([
            { "type": "vendor", "license": "none", "name": "Manual", "notes": "vendor" },
            { "type": "agent-contributed", "license": "CC0-1.0" },
            { "type": "hardware-verified-partial", "license": "public-domain" },
            { "type": "electra-preset", "imported_at": "2026-01-01" },
            { "type": "community-re", "license": "NOASSERTION" }
        ]),
    )
}

#[track_caller]
fn assert_no_failure(files: &[(&str, Value)]) {
    let found = run(files);
    assert_eq!(found.files, files.len(), "{found}");
    assert!(found.refused_in_0_6.is_empty() && found.refused_in_0_7.is_empty(), "{found}");
    assert!(found.failures.is_empty(), "{found}");
}

/// The differences the trial names in `new`, the 0.7 text of the profile the catalogue holds as
/// `given` and whose 0.6 text is `old`.
fn named_from(given: &Value, old: &str, new: &Value) -> BTreeSet<String> {
    let schemas = Schemas::load("0.7");
    differences(&judge_from(&schemas, JUDGED, Kind::Profile, &given.to_string(), old, &new_text(new)))
}

fn pointers(of: &[&str]) -> BTreeSet<String> {
    of.iter().map(|p| p.to_string()).collect()
}

#[test]
fn a_profile_credited_in_0_6_converted_to_0_7_has_no_difference_with_its_0_6_text() {
    assert_no_failure(&[("acme/_profile", rich_profile())]);
}

#[test]
fn a_profile_credited_in_0_5_converted_to_0_7_has_no_difference_with_its_0_6_text() {
    assert_no_failure(&[("acme/_profile", profile_of_0_5())]);
}

#[test]
fn a_profile_with_an_empty_list_of_sources_has_no_difference_with_its_0_6_text() {
    assert_no_failure(&[
        ("acme/_profile", profile(Some(P06), json!([]))),
        ("acme/_profile.five", profile(Some(P05), json!([]))),
        ("acme/_profile.four", profile(None, json!([]))),
    ]);
}

/// The pointers of the differences between the 0.6 text `old` of a descriptor of `kind` and `new`.
fn compared(kind: Kind, old: &Value, new: &Value) -> BTreeSet<String> {
    reference_0_6::document(kind, old, old, new).into_iter().map(|(pointer, _)| pointer).collect()
}

/// A descriptor of `kind` in 0.6 with these `_sources`, and the 0.7 text the converter writes of it.
fn converted_from_0_6(kind: Kind, extra: Value) -> (Value, Value) {
    let old = descriptor("0.6", kind, extra);
    let new = osd::convert_to(&old.to_string(), kind, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {old}"));
    (old, serde_json::from_str(&new).unwrap())
}

#[test]
fn an_empty_list_of_sources_that_the_conversion_leaves_out_is_no_difference_in_each_kind_but_the_device() {
    let mut cases: Vec<(Kind, Value)> = KINDS.into_iter().map(|kind| (kind, json!({ "_sources": [] }))).collect();
    cases.push((Kind::Sound, json!({ "made_on": made_on(json!({ "_sources": [] })) })));
    cases.push((Kind::Sound, json!({ "_sources": [], "made_on": made_on(json!({ "_sources": [] })) })));
    for (kind, extra) in cases {
        let (old, new) = converted_from_0_6(kind, extra);
        assert!(new.get("sources").is_none() && new.pointer("/made_on/sources").is_none(), "{new}");
        assert_eq!(compared(kind, &old, &new), BTreeSet::new(), "{kind:?}: {old}");
    }
}

#[test]
fn an_empty_list_of_sources_that_gains_one_or_a_list_with_entries_that_goes_is_still_named() {
    let source = json!({ "type": "vendor-doc" });
    for kind in KINDS {
        let (empty, mut gained) = converted_from_0_6(kind, json!({ "_sources": [] }));
        gained["sources"] = json!([source]);
        let named = compared(kind, &empty, &gained);
        assert!(named.contains("/_sources"), "{kind:?}: {named:?}");

        let (full, kept) = converted_from_0_6(kind, json!({ "_sources": [source] }));
        for left in [None, Some(json!([]))] {
            let mut lost = kept.clone();
            let object = lost.as_object_mut().unwrap();
            object.remove("sources").expect("the source is carried");
            object.extend(left.map(|list| ("sources".to_string(), list)));
            let named = compared(kind, &full, &lost);
            assert!(named.contains("/_sources"), "{kind:?}: {named:?} in {lost}");
        }
    }
    let (empty, mut gained) = converted_from_0_6(Kind::Sound, json!({ "made_on": made_on(json!({ "_sources": [] })) }));
    gained["made_on"]["sources"] = json!([source]);
    let named = compared(Kind::Sound, &empty, &gained);
    assert!(named.contains("/made_on/_sources"), "{named:?}");
}

#[test]
fn the_empty_list_of_sources_of_a_device_removed_is_still_named() {
    let (old, mut new) = converted_from_0_6(Kind::Device, json!({ "_sources": [] }));
    assert_eq!(compared(Kind::Device, &old, &new), BTreeSet::new(), "the text left as it is");
    new.as_object_mut().unwrap().remove("sources").expect("a device keeps its list");
    assert_eq!(compared(Kind::Device, &old, &new), pointers(&["/_sources"]));
}

#[test]
fn a_profile_given_in_0_4_whose_source_words_follow_the_tables_has_no_difference() {
    assert_no_failure(&[("acme/_profile", profile_of_0_4())]);
}

#[test]
fn a_source_word_of_a_profile_given_in_0_4_that_the_tables_do_not_prescribe_is_named_at_its_0_6_key() {
    let given = profile_of_0_4();
    let (old, new) = texts(Kind::Profile, &given);
    assert_eq!(named_from(&given, &old, &new), BTreeSet::new(), "the text left as it is");
    for (alteration, pointer) in [
        // The word of 0.4 left as it is, then another term.
        (Alter::Set("/sources/0/type", json!("vendor")), "/_sources/0/type"),
        (Alter::Set("/sources/0/type", json!("community-re")), "/_sources/0/type"),
        (Alter::Set("/sources/1/type", json!("agent-contributed")), "/_sources/1/type"),
        (Alter::Set("/sources/2/type", json!("hardware-observed")), "/_sources/2/type"),
        (Alter::Set("/sources/0/license", json!("none")), "/_sources/0/license"),
        (Alter::Set("/sources/0/license", json!("NOASSERTION")), "/_sources/0/license"),
        (Alter::Set("/sources/1/license", json!("NONE")), "/_sources/1/license"),
        (Alter::Set("/sources/4/type", json!("contributed")), "/_sources/4/type"),
        // The tables rewrite `type` and `license` alone.
        (Alter::Set("/sources/0/notes", json!("vendor-doc")), "/_sources/0/notes"),
    ] {
        let mut altered = new.clone();
        alter(&mut altered, &alteration);
        assert_eq!(named_from(&given, &old, &altered), pointers(&[pointer]), "{alteration:?}");
    }
}

#[test]
fn a_source_word_of_0_4_outside_the_tables_written_as_a_term_is_named() {
    let given = profile(None, json!([{ "type": "rumour", "license": "None" }]));
    let mut old = given.clone();
    old["$schema"] = P06.into();
    let new = json!({
        "$schema": "https://w3id.org/osd/0.7/profile.schema.json",
        "identity": { "reply_text": "acme" },
        "sources": [{ "type": "contributed", "license": "NONE" }]
    });
    // 0.7 refuses the entry as the text writes it: it is no source to be carried.
    assert_eq!(named_from(&given, &old.to_string(), &new), pointers(&["/_sources"]));
}

#[test]
fn the_tables_of_0_4_are_not_applied_to_the_sources_of_a_profile_given_in_0_5_or_0_6() {
    for schema in [P05, P06] {
        let given = profile(
            Some(schema),
            json!([
                { "type": "vendor", "license": "none" },
                { "type": "agent-contributed" },
                { "type": "hardware-verified-partial" }
            ]),
        );
        let mut old = given.clone();
        old["$schema"] = P06.into();
        // What the tables of 0.4 would write: after 0.4 these words are no terms of 0.7, and
        // their entries are no sources to be carried.
        let new = json!({
            "$schema": "https://w3id.org/osd/0.7/profile.schema.json",
            "identity": { "reply_text": "acme" },
            "sources": [
                { "type": "vendor-doc", "license": "NONE" },
                { "type": "contributed" },
                { "type": "hardware-verified" }
            ]
        });
        assert_eq!(named_from(&given, &old.to_string(), &new), pointers(&["/_sources"]), "{schema}");
        let (old, converted) = texts(Kind::Profile, &given);
        assert_eq!(converted["_sources_2"], given["_sources"], "{schema}");
        assert_eq!(named_from(&given, &old, &converted), BTreeSet::new(), "{schema}");
    }
}

#[test]
fn a_source_field_of_a_profile_changed_or_removed_is_named_at_its_0_6_key() {
    use Alter::{Remove, Set};
    assert_each_named(
        Kind::Profile,
        &rich_profile(),
        [
            (Set("/sources/0/title", json!("x")), "/_sources/0/preset_name"),
            (Remove("/sources/0/title"), "/_sources/0/preset_name"),
            (Set("/sources/0/name", json!("x")), "/_sources/0/name"),
            (Remove("/sources/0/version"), "/_sources/0/revision"),
            (Set("/sources/0/sha256", json!("x")), "/_sources/0/content_sha256"),
            (Set("/sources/0/url", json!("https://x.org/p/abc")), "/_sources/0/url"),
            (Remove("/sources/0/author"), "/_sources/0/author"),
            (Set("/sources/0/extra~1key", json!({ "a": 2 })), "/_sources/0/extra~1key"),
            (Set("/sources/0/type", json!("vendor-doc")), "/_sources/0/type"),
            (Set("/sources/0/notes", json!("type in 0.6: electra-preset")), "/_sources/0/notes"),
            (Remove("/sources/1/preset_id"), "/_sources/1/preset_id"),
            (Remove("/sources/2/preset_name"), "/_sources/2/preset_name"),
            (Set("/sources/3/license", json!("NONE")), "/_sources/3/license"),
            (Set("/sources/4/provider", json!("pencilresearch")), "/_sources/4/provider"),
            (Remove("/sources/5/_provider"), "/_sources/5/_provider"),
            (Set("/sources/6/_provider", json!(8)), "/_sources/6/provider"),
            (Set("/sources/7/_imported_at", json!("2027-01-01")), "/_sources/7/imported_at"),
            (Remove("/sources/7/_imported_at"), "/_sources/7/imported_at"),
            (Remove("/sources/8/_imported_at_2"), "/_sources/8/imported_at"),
            (Remove("/sources/8/_imported_at"), "/_sources/8/_imported_at"),
        ],
    );
}

#[test]
fn a_key_of_a_source_of_a_profile_the_correspondence_does_not_give_is_named_at_its_0_7_pointer() {
    assert_each_named(
        Kind::Profile,
        &rich_profile(),
        [
            added("/sources/3/provider", json!("midnam")),
            // Dropped by the conversion: it ends the `url`.
            added("/sources/0/preset_id", json!("abc")),
            added("/sources/0/revision", json!("3")),
            added("/sources/3/_added", json!(1)),
            added("/sources/7/imported_at", json!("2026-01-01")),
        ],
    );
}

#[test]
fn a_source_or_the_list_of_sources_of_a_profile_removed_or_added_is_named_at_the_list() {
    let source = json!({ "type": "vendor-doc" });
    assert_each_named(
        Kind::Profile,
        &rich_profile(),
        [
            (Alter::Remove("/sources/8"), "/_sources"),
            (Alter::Add("/sources/-", source.clone()), "/_sources"),
            (Alter::Remove("/sources"), "/_sources"),
        ],
    );
    let bare = json!({ "$schema": P06, "identity": { "reply_text": "acme" } });
    assert_each_named(
        Kind::Profile,
        &bare,
        [
            (Alter::Add("/sources", json!([source])), "/_sources"),
            (Alter::Add("/sources", json!([])), "/_sources"),
        ],
    );
}

#[test]
fn the_midi_of_sources_of_a_profile_is_a_note_compared_as_it_is() {
    assert_each_named(
        Kind::Profile,
        &rich_profile(),
        [
            set("/_midi_of_sources/0/type", json!("vendor-doc")),
            removed("/_midi_of_sources/0/imported_at"),
            added("/_midi_of_sources/0/title", json!("stays")),
            removed("/_midi_of_sources"),
            added("/midi_of_sources", json!([{ "type": "vendor-doc" }])),
        ],
    );
}
