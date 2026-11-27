//! What the trial of 0.7 expects of the `_sources` of a maker profile, a note of any value before
//! 0.7 that the conversion judges entry by entry (`spec/0.7.md`, "The credit of every kind"): an
//! object without `type` is carried as `contributed`, every entry 0.7 refuses is kept under
//! `_sources_2` as the text writes it, and the `profile` of a source that is no string, of a device
//! as of a profile, is under `_profile`. Each case is the converted text of a small catalogue, left
//! as it is or altered, given to the trial.

use crate::catalogue_trial::run;
use crate::catalogue_trial_reference::{alter, assert_each_named, differences, new_text, texts, Alter, JUDGED};
use crate::common::trial::judge_from;
use crate::common::Schemas;
use osd::Kind;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const P05: &str = "https://w3id.org/osd/0.5/profile.schema.json";
const P06: &str = "https://w3id.org/osd/0.6/profile.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";

/// A profile of the version of `schema` (none: 0.4, with no `$schema`) with this `_sources`.
fn profile(schema: Option<&str>, sources: Value) -> Value {
    let mut doc = json!({ "identity": { "reply_text": "acme" }, "_sources": sources });
    if let Some(schema) = schema {
        doc["$schema"] = schema.into();
    }
    doc
}

#[track_caller]
fn assert_no_failure(files: &[(&str, Value)]) {
    let found = run(files);
    assert_eq!(found.files, files.len(), "{found}");
    assert!(found.refused_in_0_6.is_empty() && found.refused_in_0_7.is_empty(), "{found}");
    assert!(found.failures.is_empty(), "{found}");
}

/// Entries without a `type`: one with an address alone, one with notes, a name to rename and a
/// licence, one empty, beside an entry that has a `type`.
fn untyped_entries() -> Value {
    json!([
        { "url": "u" },
        { "url": "v", "name": "Manual", "license": "CC0-1.0", "notes": "given\nsecond line", "revision": "3" },
        { "type": "vendor-doc", "url": "w" },
        {}
    ])
}

#[test]
fn a_source_of_a_profile_without_a_type_is_expected_contributed_in_its_place_with_no_difference() {
    assert_no_failure(&[
        ("acme/_profile", profile(Some(P06), untyped_entries())),
        ("acme/_profile.five", profile(Some(P05), untyped_entries())),
        ("acme/_profile.alone", profile(Some(P06), json!([{ "url": "u" }]))),
        ("acme/_profile.four", profile(None, json!([{ "url": "u", "license": "none" }]))),
    ]);
}

#[test]
fn the_type_and_the_notes_of_a_source_without_a_type_changed_or_removed_are_named() {
    use Alter::{Remove, Set};
    for schema in [P05, P06] {
        assert_each_named(
            Kind::Profile,
            &profile(Some(schema), untyped_entries()),
            [
                (Set("/sources/0/type", json!("vendor-doc")), "/sources/0/type"),
                (Remove("/sources/0/type"), "/sources/0/type"),
                (Set("/sources/0/notes", json!("no type")), "/sources/0/notes"),
                (Remove("/sources/0/notes"), "/sources/0/notes"),
                (Set("/sources/1/type", json!("community-re")), "/sources/1/type"),
                (Set("/sources/1/notes", json!("given\nsecond line")), "/_sources/1/notes"),
                (Set("/sources/1/notes", json!("given\nsecond line\nno type in 0.6")), "/_sources/1/notes"),
                (Remove("/sources/1/notes"), "/_sources/1/notes"),
                (Remove("/sources/1/title"), "/_sources/1/name"),
                (Set("/sources/2/type", json!("contributed")), "/_sources/2/type"),
                (Remove("/sources/3/type"), "/sources/3/type"),
                (Remove("/sources/3"), "/_sources"),
            ],
        );
    }
}

/// Entries that do and do not read as sources of 0.7, in turn. Those carried are at 0, 3 and 7;
/// those kept at 1, 2, 4, 5, 6 and 8.
fn mixed_entries() -> Value {
    json!([
        { "type": "vendor-doc", "id": "a" },
        "a text",
        { "type": "rumour", "url": "r", "name": "kept as written", "imported_at": "2026-01-01" },
        { "url": "u" },
        7,
        { "type": "community-re", "id": "a", "preset_name": "later of the two" },
        { "url": "no", "license": "none of the list" },
        { "type": "community-re", "id": "b" },
        null
    ])
}

#[test]
fn the_entries_of_the_sources_of_a_profile_that_0_7_refuses_are_expected_under_the_note_with_no_difference() {
    let mut taken = profile(Some(P06), mixed_entries());
    taken["_sources_2"] = json!("taken");
    taken["_sources_3"] = json!(["taken too"]);
    let none_carried = json!(["a", { "type": "rumour" }, []]);
    // The first of one `id` is refused for another reason: the second is the one source of that `id`.
    let refused_then_read = json!([{ "type": "rumour", "id": "a" }, { "type": "vendor-doc", "id": "a" }]);
    assert_no_failure(&[
        ("acme/_profile", profile(Some(P06), mixed_entries())),
        ("acme/_profile.five", profile(Some(P05), mixed_entries())),
        ("acme/_profile.taken", taken),
        ("acme/_profile.none", profile(Some(P06), none_carried)),
        ("acme/_profile.then", profile(Some(P06), refused_then_read)),
        ("acme/_profile.object", profile(Some(P06), json!({ "url": "u" }))),
        ("acme/_profile.text", profile(Some(P05), json!("see the manual"))),
        ("acme/_profile.number", profile(Some(P06), json!(3))),
        ("acme/_profile.null", profile(Some(P06), json!(null))),
    ]);
}

#[test]
fn an_entry_of_the_note_changed_lost_or_carried_into_the_sources_is_named() {
    use Alter::{Add, Remove, Set};
    for schema in [P05, P06] {
        assert_each_named(
            Kind::Profile,
            &profile(Some(schema), mixed_entries()),
            [
                (Set("/_sources_2/0", json!("another text")), "/_sources/1"),
                (Set("/_sources_2/1/type", json!("contributed")), "/_sources/2"),
                (Remove("/_sources_2/1/imported_at"), "/_sources/2"),
                (Add("/_sources_2/1/title", json!("x")), "/_sources/2"),
                (Set("/_sources_2/2", json!(8)), "/_sources/4"),
                (Set("/_sources_2/3/id", json!("c")), "/_sources/5"),
                (Set("/_sources_2/5", json!({})), "/_sources/8"),
                (Remove("/_sources_2/0"), "/_sources"),
                (Remove("/_sources_2"), "/_sources"),
                (Set("/_sources_2", json!("a text")), "/_sources"),
                (Add("/_sources_2/-", json!("more")), "/_sources"),
                (Add("/sources/-", json!({ "type": "community-re", "id": "c" })), "/_sources"),
                (Add("/sources/-", json!("a text")), "/_sources"),
                (Remove("/sources/2"), "/_sources"),
                (Add("/_sources_3", json!(["a text"])), "/_sources_3"),
            ],
        );
    }
}

#[test]
fn the_note_of_a_profile_with_no_entry_to_carry_changed_or_given_sources_is_named() {
    use Alter::{Add, Remove, Set};
    for note in [json!(["a", { "type": "rumour" }]), json!({ "type": "vendor-doc" }), json!("see the manual")] {
        assert_each_named(
            Kind::Profile,
            &profile(Some(P06), note),
            [
                (Set("/_sources_2", json!(false)), "/_sources"),
                (Remove("/_sources_2"), "/_sources"),
                (Add("/sources", json!([])), "/_sources"),
                (Add("/sources", json!([{ "type": "vendor-doc" }])), "/_sources"),
            ],
        );
    }
}

#[test]
fn the_note_is_expected_at_the_first_free_number_beside_the_notes_already_there() {
    let mut given = profile(Some(P06), json!(["refused"]));
    given["_sources_2"] = json!("taken");
    assert_each_named(
        Kind::Profile,
        &given,
        [
            (Alter::Set("/_sources_2", json!(["refused"])), "/_sources_2"),
            (Alter::Remove("/_sources_3"), "/_sources"),
        ],
    );
}

#[test]
fn an_entry_of_a_profile_given_in_0_4_that_0_7_refuses_is_expected_under_the_note_with_the_words_of_0_4() {
    let given = profile(
        None,
        json!([
            { "type": "vendor", "id": "a", "license": "none" },
            { "type": "vendor", "id": "a", "license": "none" },
            { "type": "agent-contributed", "id": "" },
            "vendor"
        ]),
    );
    assert_no_failure(&[("acme/_profile", given.clone())]);
    let (old, new) = texts(Kind::Profile, &given);
    assert_eq!(new["sources"], json!([{ "type": "vendor-doc", "id": "a", "license": "NONE" }]));
    assert_eq!(new["_sources_2"], json!([given["_sources"][1], given["_sources"][2], given["_sources"][3]]));
    let schemas = Schemas::load("0.7");
    for (at, word, named) in [
        ("/_sources_2/0/type", "vendor-doc", "/_sources/1"),
        ("/_sources_2/0/license", "NONE", "/_sources/1"),
        ("/_sources_2/1/type", "contributed", "/_sources/2"),
    ] {
        let mut altered = new.clone();
        alter(&mut altered, &Alter::Set(at, json!(word)));
        let found = judge_from(&schemas, JUDGED, Kind::Profile, &given.to_string(), &old, &new_text(&altered));
        assert_eq!(differences(&found), BTreeSet::from([named.to_string()]), "{at}");
    }
}

/// Sources whose `profile` is no string, one beside notes that take the key, and one whose
/// `profile` is a string.
fn sources_with_profiles() -> Value {
    json!([
        { "type": "vendor-doc", "profile": 7 },
        { "type": "vendor-doc", "profile": { "a": 1 }, "_profile": "taken", "_profile_2": "too" },
        { "type": "vendor-doc", "profile": "acme" },
        { "type": "vendor-doc", "profile": null }
    ])
}

fn driver_with_profiles() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "Credited", "osc_prefix": "/credited" },
        "_sources": sources_with_profiles(),
        "_midi_of_sources": [{ "type": "community-re", "profile": ["acme"] }]
    })
}

#[test]
fn the_profile_of_a_source_that_is_no_string_is_expected_under_its_note_with_no_difference() {
    assert_no_failure(&[
        ("acme/credited", driver_with_profiles()),
        ("acme/_profile", profile(Some(P06), sources_with_profiles())),
        ("acme/_profile.five", profile(Some(P05), sources_with_profiles())),
    ]);
}

#[test]
fn the_profile_of_a_source_that_is_no_string_changed_or_lost_is_named_at_its_0_6_key() {
    use Alter::{Remove, Set};
    let cases = || {
        [
            (Set("/sources/0/_profile", json!(8)), "/_sources/0/profile"),
            (Remove("/sources/0/_profile"), "/_sources/0/profile"),
            (Set("/sources/1/_profile_3", json!({ "a": 2 })), "/_sources/1/profile"),
            (Remove("/sources/1/_profile_3"), "/_sources/1/profile"),
            (Remove("/sources/1/_profile"), "/_sources/1/_profile"),
            (Set("/sources/2/profile", json!("other")), "/_sources/2/profile"),
            (Remove("/sources/3/_profile"), "/_sources/3/profile"),
        ]
    };
    assert_each_named(Kind::Profile, &profile(Some(P06), sources_with_profiles()), cases());
    let of_the_target = (Set("/midi_of_sources/0/_profile", json!("acme")), "/_midi_of_sources/0/profile");
    assert_each_named(Kind::Device, &driver_with_profiles(), cases().into_iter().chain([of_the_target]));
}
