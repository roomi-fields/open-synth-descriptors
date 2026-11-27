//! What the trial of 0.7 compares with the 0.6 text: every message and value of
//! the entries of `cc_params` and `params`, and every field of every source, as
//! `spec/0.7.md` says they are written (5.3, 5.3.3, 5.3.5, 6.8.4). Each case is
//! the converted text of a small catalogue, left as it is or altered, given to
//! `judge`.

use crate::catalogue_trial::run;
use crate::common::trial::{judge, judge_from, Check, Failure, Trial};
use crate::common::Schemas;
use osd::{Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const D05: &str = "https://w3id.org/osd/0.5/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";

/// A driver of 0.6 with every writing of a control and of a source the
/// conversion to 0.7 changes, but the date of import.
fn rich_driver() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "Rich", "osc_prefix": "/rich" },
        "capabilities": ["lua"],
        "cc_params": { "channel": 0, "entries": [
            // 0: three ways, a scale, named values, every text for people.
            {
                "osc": "/a", "cc": 74, "range": [0, 127], "transmit": "absolute", "recognize": "both",
                "default": 3, "values": { "Zed": 127, "Ann": 0 }, "step_count": 2, "orientation": "centered",
                "channel": 2, "priority": 1, "section": "filter", "expects": "continuous",
                "transform_reverse": "return value",
                "also": [
                    { "nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "range": [0, 16383], "_way": "fine" },
                    { "rpn_msb": 0, "rpn_lsb": 1, "value_bits": 7 }
                ],
                "_usage": "how far", "_name": "Cutoff", "_description": "the cutoff", "_notes": "measured", "_other": "kept"
            },
            // 1: a CC of 14 bits; no way takes the block.
            { "osc": "/b", "cc": 7, "cc_lsb": 39, "orientation": "0-based", "nrpn_lsb_range": [0, 3] },
            // 2: the one NRPN way of `also` takes the block.
            { "osc": "/c", "cc": 10, "recognize": "none", "nrpn_lsb_range": [0, 7],
              "also": [{ "nrpn_msb": 2, "nrpn_lsb": 0, "value_bits": 7 }] },
            // 3: the way of the flat NRPN takes the block; equal maxima, no scale.
            { "osc": "/d", "nrpn_msb": 3, "nrpn_lsb": 0, "value_bits": 7, "nrpn_lsb_range": [0, 9], "default": 100,
              "also": [{ "cc": 20 }] },
            // 4: two NRPN ways of `also`, two note keys taken.
            { "osc": "/e", "cc": 11, "nrpn_lsb_range": [1, 2], "_nrpn_lsb_range": "taken", "_nrpn_lsb_range_2": "too",
              "also": [{ "nrpn_msb": 4, "nrpn_lsb": 0, "value_bits": 7 }, { "nrpn_msb": 5, "nrpn_lsb": 0, "value_bits": 7 }] },
            // 5: no message.
            { "osc": "/f", "range": [0, 3], "transmit": "absolute", "recognize": "none", "nrpn_lsb_range": [0, 7],
              "default": 1, "values": { "Off": 0 }, "_name": "Bank" },
            // 6: the messages in `also` alone; a later way more precise, a scale.
            { "osc": "/g", "transmit": "relative", "default": 1,
              "also": [{ "cc": 21 }, { "cc": 22, "cc_lsb": 54 }] },
            // 7: a later way more precise and nothing measured, no scale.
            { "osc": "/h", "cc": 40, "also": [{ "nrpn_msb": 6, "nrpn_lsb": 0, "value_bits": 14 }] },
            // 8: a count of steps alone gives a scale; no range on the first way.
            { "osc": "/i", "cc": 41, "step_count": 4, "also": [{ "nrpn_msb": 7, "nrpn_lsb": 0, "value_bits": 14 }] },
            // 9: a text that does not move.
            { "osc": "/j", "cc": 42, "_name": "", "_notes": 4 }
        ] },
        "params": {
            "get_frame": [240, { "arg": "p" }, 247],
            "set_frame": [240, { "arg": "p" }, { "arg": "value" }, 247],
            "value_type": "u7",
            "entries": [
                { "osc": "/p0", "pr": 16, "p": 2, "range": [0, 100], "transmit": "absolute", "recognize": "none",
                  "default": 1, "values": { "Saw": 1, "Sine": 0 }, "step_count": 3, "section": "osc",
                  "transform_reverse": "return value", "_name": "Shape", "_description": "the shape", "_usage": "morph" },
                { "osc": "/p1", "p": 3 },
                { "osc": "/p2", "id": 200 },
                { "osc": "/p3", "pr": 1, "p": 2, "c": 3, "r": 4, "id": 5 },
                { "osc": "/p4", "range": [0, 99], "recognize": "both" }
            ]
        },
        "_sources": [
            { "type": "electra-preset", "preset_name": "Preset A", "name": "Acme A", "revision": "3", "content_sha256": "aa",
              "preset_id": "abc", "adaptation": "Other", "url": "https://x.org/p/abc?v=1#top", "notes": "kept",
              "author": "someone", "extra/key": { "a": 1 } },
            { "type": "knobkraft", "name": "Only", "url": "https://x.org/a/Acme/", "adaptation": "Acme", "preset_id": "zzz" },
            { "type": "vendor-doc", "title": "T", "preset_name": "P", "name": "N", "version": "1", "revision": "2",
              "sha256": "bb", "content_sha256": "cc", "preset_id": "abc", "url": "https://x.org/p/ABC" },
            { "type": "midnam", "license": "NOASSERTION" },
            { "type": "pencilresearch", "provider": "acme-co" },
            { "type": "contributed", "provider": "Not Well Formed", "_provider": "taken" },
            { "type": "knobkraft", "provider": 7 }
        ],
        "_midi_of_sources": [
            { "type": "electra-preset", "name": "Base", "revision": "9" },
            { "type": "community-re", "url": "https://x.org/base", "firmware": "1.0" }
        ]
    })
}

/// The id of the file judged.
pub(crate) const JUDGED: &str = "acme/judged";

/// The 0.6 and 0.7 texts of the stored descriptor `doc`.
pub(crate) fn texts(kind: Kind, doc: &Value) -> (String, Value) {
    let stored = doc.to_string();
    let in_version = |v| osd::convert_to(&stored, kind, v).unwrap_or_else(|f| panic!("{f}"));
    (in_version(Version::V0_6), serde_json::from_str(&in_version(Version::V0_7)).unwrap())
}

/// What the trial says of `new`, the 0.7 text of the file whose 0.6 text is `old`.
pub(crate) fn judge_text(kind: Kind, old: &str, new: &Value) -> Vec<Failure> {
    judge(&Schemas::load("0.7"), JUDGED, kind, old, &new_text(new))
}

/// `new` as the converter writes a text.
pub(crate) fn new_text(new: &Value) -> String {
    let mut text = serde_json::to_string_pretty(new).unwrap();
    text.push('\n');
    text
}

/// The pointers of the differences with the 0.6 text, each in the file judged
/// and named once.
pub(crate) fn differences(failures: &[Failure]) -> BTreeSet<String> {
    let mut named = BTreeSet::new();
    for f in failures.iter().filter(|f| f.check == Check::Loss) {
        assert_eq!(f.file, JUDGED, "{f}");
        assert!(named.insert(f.pointer.clone()), "named twice: {f}");
    }
    named
}

/// One alteration of a converted text, at a JSON pointer of it.
#[derive(Debug)]
pub(crate) enum Alter {
    /// The value at the pointer becomes this one.
    Set(&'static str, Value),
    /// The key or the element at the pointer is taken out.
    Remove(&'static str),
    /// A key the text does not hold is written.
    Add(&'static str, Value),
}

pub(crate) fn alter(doc: &mut Value, alteration: &Alter) {
    let split = |at: &str| {
        let (parent, key) = at.rsplit_once('/').unwrap();
        (parent.to_string(), key.replace("~1", "/").replace("~0", "~"))
    };
    match alteration {
        Alter::Set(at, value) => {
            let place = doc.pointer_mut(at).unwrap_or_else(|| panic!("nothing at {at}"));
            assert_ne!(place, value, "{at} already holds {value}");
            *place = value.clone();
        }
        Alter::Remove(at) => {
            let (parent, key) = split(at);
            match doc.pointer_mut(&parent) {
                Some(Value::Object(o)) => assert!(o.remove(&key).is_some(), "nothing at {at}"),
                Some(Value::Array(l)) => {
                    l.remove(key.parse().unwrap());
                }
                _ => panic!("nothing at {parent}"),
            }
        }
        Alter::Add(at, value) => {
            let (parent, key) = split(at);
            match doc.pointer_mut(&parent) {
                Some(Value::Object(o)) => assert!(o.insert(key, value.clone()).is_none(), "{at} is already there"),
                Some(Value::Array(l)) => l.push(value.clone()),
                _ => panic!("nothing at {parent}"),
            }
        }
    }
}

/// Each alteration alone of the converted text of `doc` is one difference with
/// the 0.6 text, named at its pointer, and no other.
#[track_caller]
pub(crate) fn assert_each_named(kind: Kind, doc: &Value, cases: impl IntoIterator<Item = (Alter, &'static str)>) {
    let (old, new) = texts(kind, doc);
    let as_it_is = differences(&judge_text(kind, &old, &new));
    assert!(as_it_is.is_empty(), "the text left as it is: {as_it_is:?}");
    let mut wrong = Vec::new();
    for (alteration, pointer) in cases {
        let mut altered = new.clone();
        alter(&mut altered, &alteration);
        let found = differences(&judge_text(kind, &old, &altered));
        if found != BTreeSet::from([pointer.to_string()]) {
            wrong.push(format!("{alteration:?}: named {found:?}, expected {pointer}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// An alteration of the 0.7 text named at the same pointer.
pub(crate) fn set(at: &'static str, value: Value) -> (Alter, &'static str) {
    (Alter::Set(at, value), at)
}

pub(crate) fn removed(at: &'static str) -> (Alter, &'static str) {
    (Alter::Remove(at), at)
}

pub(crate) fn added(at: &'static str, value: Value) -> (Alter, &'static str) {
    (Alter::Add(at, value), at)
}

#[test]
fn the_converted_text_of_every_writing_the_specification_changes_has_no_difference_with_its_0_6_text() {
    let (old, new) = texts(Kind::Device, &rich_driver());
    let failures = judge_text(Kind::Device, &old, &new);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn a_catalogue_of_older_versions_converted_has_no_difference_with_its_0_6_reading() {
    let older = json!({
        "device": { "name": "Older", "osc_prefix": "/older" },
        "_sources": [{ "type": "vendor", "license": "none" }],
        "cc_params": { "entries": [
            { "osc": "/v", "cc": 7, "_usage": "level", "orientation": "centered" },
            { "osc": "/n", "nrpn_msb": 1, "nrpn_lsb": 0, "nrpn_lsb_range": [0, 4] },
            // Messages not fully known, whose width the text does not give.
            { "osc": "/msb", "nrpn_msb": 2 },
            { "osc": "/lsb", "rpn_lsb": 3, "range": [0, 1000] },
            { "osc": "/low", "cc_lsb": 40 }
        ] }
    });
    let older_0_5 = json!({
        "$schema": D05,
        "device": { "name": "Five", "osc_prefix": "/five" },
        "cc_params": { "entries": [
            { "osc": "/msb", "nrpn_msb": 4, "nrpn_lsb_range": [0, 7] },
            { "osc": "/rpn", "rpn_msb": 0 }
        ] }
    });
    // The 0.6 conversion refuses a control that mixes kinds: nothing to compare with.
    let mixed = json!({
        "$schema": D05,
        "device": { "name": "Mixed", "osc_prefix": "/mixed" },
        "cc_params": { "entries": [{ "osc": "/m", "cc": 7, "nrpn_msb": 0, "nrpn_lsb": 7 }] }
    });
    let found = run(&[("acme/rich", rich_driver()), ("acme/older", older), ("acme/five", older_0_5), ("acme/mixed", mixed)]);
    assert!(found.failures.is_empty(), "{found}");
    assert!(found.refused_in_0_7.is_empty(), "{found}");
    assert_eq!(found.refused_in_0_6.values().sum::<usize>(), 1, "{found}");
}

#[test]
fn a_width_the_older_text_does_not_give_written_in_the_note_is_named() {
    let given = json!({
        "$schema": D05,
        "device": { "name": "Five", "osc_prefix": "/five" },
        "cc_params": { "entries": [{ "osc": "/msb", "nrpn_msb": 4 }] }
    });
    let (old, mut new) = texts(Kind::Device, &given);
    let given = given.to_string();
    let judged = |new: &Value| judge_from(&Schemas::load("0.7"), JUDGED, Kind::Device, &given, &old, &new_text(new));
    let failures = judged(&new);
    assert!(failures.is_empty(), "{failures:#?}");
    alter(&mut new, &Alter::Add("/cc_params/entries/0/_incomplete/0/value_bits", json!(14)));
    let named = BTreeSet::from(["/cc_params/entries/0/_incomplete".to_string()]);
    assert_eq!(differences(&judged(&new)), named);
}

#[test]
fn a_cc_number_the_conversion_changes_is_named() {
    assert_each_named(
        Kind::Device,
        &rich_driver(),
        [
            set("/cc_params/entries/0/ways/0/ctrlIndex", json!([75])),
            set("/cc_params/entries/1/ways/0/ctrlIndex", json!([7, 40])),
            set("/cc_params/entries/6/ways/1/ctrlIndex", json!([23, 54])),
        ],
    );
}

#[test]
fn a_message_value_changed_removed_or_added_is_named_at_the_field_of_its_way() {
    assert_each_named(
        Kind::Device,
        &rich_driver(),
        [
            // The kind.
            set("/cc_params/entries/0/ways/0/ctrlType", json!("pPress")),
            set("/cc_params/entries/0/ways/2/ctrlType", json!("nrpn")),
            removed("/cc_params/entries/0/ways/1/ctrlType"),
            set("/params/entries/0/ways/0/ctrlType", json!("cc")),
            // The numbers.
            set("/cc_params/entries/0/ways/1/ctrlIndex", json!([1, 9])),
            set("/cc_params/entries/0/ways/2/ctrlIndex", json!([1, 0])),
            removed("/cc_params/entries/3/ways/0/ctrlIndex"),
            set("/params/entries/0/ways/0/pr", json!(2)),
            set("/params/entries/3/ways/0/id", json!(6)),
            removed("/params/entries/2/ways/0/id"),
            removed("/params/entries/3/ways/0/pr"),
            added("/params/entries/1/ways/0/pr", json!(0)),
            added("/params/entries/0/ways/0/ctrlIndex", json!([16, 2])),
            // The width.
            set("/cc_params/entries/0/ways/1/value_bits", json!(7)),
            removed("/cc_params/entries/1/ways/0/value_bits"),
            removed("/cc_params/entries/6/ways/1/value_bits"),
            added("/cc_params/entries/0/ways/0/value_bits", json!(7)),
            added("/cc_params/entries/3/ways/1/value_bits", json!(7)),
            added("/params/entries/0/ways/0/value_bits", json!(7)),
            // The range of the message.
            set("/cc_params/entries/0/ways/0/range", json!([0, 126])),
            set("/cc_params/entries/0/ways/1/range", json!([1, 16383])),
            removed("/cc_params/entries/0/ways/0/range"),
            removed("/cc_params/entries/0/ways/1/range"),
            added("/cc_params/entries/0/ways/2/range", json!([0, 127])),
            set("/params/entries/0/ways/0/range", json!([0, 101])),
            removed("/params/entries/0/ways/0/range"),
            added("/params/entries/1/ways/0/range", json!([0, 127])),
            // The coding.
            set("/cc_params/entries/6/ways/0/transmit", json!("absolute")),
            set("/cc_params/entries/6/ways/1/transmit", json!("absolute")),
            removed("/cc_params/entries/0/ways/2/transmit"),
            set("/cc_params/entries/0/ways/0/recognize", json!("absolute")),
            removed("/cc_params/entries/2/ways/1/recognize"),
            added("/cc_params/entries/2/ways/0/transmit", json!("absolute")),
            added("/cc_params/entries/7/ways/1/recognize", json!("both")),
            set("/params/entries/0/ways/0/transmit", json!("relative")),
            removed("/params/entries/0/ways/0/recognize"),
            added("/params/entries/1/ways/0/transmit", json!("absolute")),
            // The block of an NRPN.
            set("/cc_params/entries/3/ways/0/nrpn_lsb_range", json!([0, 8])),
            removed("/cc_params/entries/2/ways/1/nrpn_lsb_range"),
            added("/cc_params/entries/2/ways/0/nrpn_lsb_range", json!([0, 7])),
            added("/cc_params/entries/4/ways/1/nrpn_lsb_range", json!([1, 2])),
            // The notes of a way, and a key the correspondence does not give.
            added("/params/entries/0/ways/0/_flat", json!({ "id": 5 })),
            removed("/cc_params/entries/0/ways/1/_way"),
            added("/cc_params/entries/0/ways/0/_way", json!("fine")),
        ],
    );
}

#[test]
fn ways_in_another_order_are_named_at_the_fields_that_differ() {
    let (old, mut new) = texts(Kind::Device, &rich_driver());
    new["cc_params"]["entries"][3]["ways"].as_array_mut().unwrap().swap(0, 1);
    let found = differences(&judge_text(Kind::Device, &old, &new));
    for way in [0, 1] {
        let at = format!("/cc_params/entries/3/ways/{way}/ctrlType");
        assert!(found.contains(&at), "no {at} in {found:?}");
    }
}

/// A 0.6 driver whose controls give a message not fully known: an NRPN or an RPN of one number, a
/// `cc_lsb` without its `cc`.
fn incomplete_driver() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "One", "osc_prefix": "/one" },
        "cc_params": { "entries": [
            // 0: nothing but it, with a block.
            { "osc": "/msb", "nrpn_msb": 5, "value_bits": 14, "range": [0, 9], "nrpn_lsb_range": [0, 3] },
            // 1: in `also`, beside a way.
            { "osc": "/lsb", "cc": 1, "also": [{ "rpn_lsb": 6, "value_bits": 7, "range": [0, 5], "_way": "w" }] },
            // 2: the first message of a measured control; the block of a flat NRPN not fully known.
            { "osc": "/first", "nrpn_lsb": 7, "value_bits": 14, "range": [0, 1000], "default": 900, "nrpn_lsb_range": [0, 7],
              "also": [{ "nrpn_msb": 2, "nrpn_lsb": 0, "value_bits": 7 }] },
            // 3: two of them, and a note key taken.
            { "osc": "/both", "cc_lsb": 42, "_incomplete": "mine", "also": [{ "cc": 8 }, { "cc_lsb": 44 }] }
        ] }
    })
}

#[test]
fn the_converted_text_of_a_message_not_fully_known_has_no_difference_with_its_0_6_text() {
    let (old, new) = texts(Kind::Device, &incomplete_driver());
    let failures = judge_text(Kind::Device, &old, &new);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn a_way_written_for_a_message_not_fully_known_is_named() {
    let doc = json!({
        "$schema": D06,
        "device": { "name": "One", "osc_prefix": "/one" },
        "cc_params": { "entries": [
            { "osc": "/msb", "nrpn_msb": 5, "value_bits": 14 },
            { "osc": "/lsb", "cc": 1, "also": [{ "rpn_lsb": 6, "value_bits": 7 }] },
            { "osc": "/low", "cc_lsb": 40 }
        ] }
    });
    let old = doc.to_string();
    // The 0.7 text that writes a 0 in place of the number not given, and a CC of the LSB number.
    let invented = json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "device": doc["device"],
        "cc_params": { "entries": [
            { "osc": "/msb", "ways": [{ "ctrlType": "nrpn", "ctrlIndex": [5, 0], "value_bits": 14 }] },
            { "osc": "/lsb", "ways": [
                { "ctrlType": "cc", "ctrlIndex": [1] },
                { "ctrlType": "rpn", "ctrlIndex": [0, 6], "value_bits": 7 }
            ] },
            { "osc": "/low", "ways": [{ "ctrlType": "cc", "ctrlIndex": [40] }] }
        ] }
    });
    let failures = judge_text(Kind::Device, &old, &invented);
    let expected = (0..3).flat_map(|n| ["ways", "_incomplete"].map(|key| format!("/cc_params/entries/{n}/{key}")));
    assert_eq!(differences(&failures), expected.collect(), "{failures:#?}");
}

#[test]
fn a_field_of_a_message_not_fully_known_changed_removed_or_added_is_named_at_its_note() {
    const MSB: &str = "/cc_params/entries/0/_incomplete";
    const ALSO: &str = "/cc_params/entries/1/_incomplete";
    const BOTH: &str = "/cc_params/entries/3/_incomplete_2";
    let named = |alteration: Alter, at: &'static str| (alteration, at);
    assert_each_named(
        Kind::Device,
        &incomplete_driver(),
        [
            removed(MSB),
            named(Alter::Add("/cc_params/entries/0/_incomplete/0/nrpn_lsb", json!(0)), MSB),
            named(Alter::Remove("/cc_params/entries/0/_incomplete/0/value_bits"), MSB),
            named(Alter::Remove("/cc_params/entries/1/_incomplete/0/range"), ALSO),
            named(Alter::Remove("/cc_params/entries/1/_incomplete/0/_way"), ALSO),
            named(Alter::Set("/cc_params/entries/1/_incomplete/0/rpn_lsb", json!(0)), ALSO),
            named(Alter::Remove("/cc_params/entries/3/_incomplete_2/1"), BOTH),
            removed(BOTH),
            set("/cc_params/entries/3/_incomplete", json!("other")),
            removed("/cc_params/entries/0/range"),
            removed("/cc_params/entries/0/nrpn_lsb_range"),
            removed("/cc_params/entries/2/_nrpn_lsb_range"),
            added("/cc_params/entries/2/ways/0/nrpn_lsb_range", json!([0, 7])),
            set("/cc_params/entries/2/scale", json!([0, 127])),
            removed("/cc_params/entries/2/scale"),
        ],
    );
}

#[test]
fn a_block_note_changed_removed_or_written_under_another_key_is_named() {
    assert_each_named(
        Kind::Device,
        &rich_driver(),
        [
            set("/cc_params/entries/1/_nrpn_lsb_range", json!([0, 4])),
            removed("/cc_params/entries/1/_nrpn_lsb_range"),
            set("/cc_params/entries/4/_nrpn_lsb_range_3", json!([1, 3])),
            removed("/cc_params/entries/4/_nrpn_lsb_range_3"),
            // The keys the 0.6 entry already took keep their own text.
            set("/cc_params/entries/4/_nrpn_lsb_range", json!([1, 2])),
            removed("/cc_params/entries/4/_nrpn_lsb_range_2"),
            // A way receives the block: no note is given.
            added("/cc_params/entries/3/_nrpn_lsb_range", json!([0, 9])),
            added("/cc_params/entries/2/_nrpn_lsb_range", json!([0, 7])),
            added("/cc_params/entries/0/_nrpn_lsb_range", json!([0, 7])),
            // An entry with no way keeps the block itself.
            added("/cc_params/entries/5/_nrpn_lsb_range", json!([0, 7])),
        ],
    );
}

#[test]
fn a_block_note_under_another_suffix_is_named_at_the_key_expected_and_at_the_key_written() {
    for (entry, expected, written) in [
        (1, "_nrpn_lsb_range", "_nrpn_lsb_range_2"),
        (4, "_nrpn_lsb_range_3", "_nrpn_lsb_range_4"),
    ] {
        let (old, mut new) = texts(Kind::Device, &rich_driver());
        let control = new["cc_params"]["entries"][entry].as_object_mut().unwrap();
        let block = control.remove(expected).unwrap();
        control.insert(written.into(), block);
        let found = differences(&judge_text(Kind::Device, &old, &new));
        let named = [expected, written].map(|k| format!("/cc_params/entries/{entry}/{k}"));
        assert_eq!(found, named.into());
    }
}

#[test]
fn a_value_of_an_entry_changed_or_removed_is_named_at_its_0_6_key() {
    let at = |a: Alter, key: &'static str| (a, key);
    assert_each_named(
        Kind::Device,
        &rich_driver(),
        [
            set("/cc_params/entries/0/default", json!(4)),
            removed("/cc_params/entries/0/default"),
            set("/cc_params/entries/0/step_count", json!(3)),
            removed("/cc_params/entries/8/step_count"),
            set("/cc_params/entries/0/channel", json!(3)),
            removed("/cc_params/entries/0/channel"),
            set("/cc_params/entries/0/priority", json!(2)),
            removed("/cc_params/entries/0/section"),
            set("/cc_params/entries/0/expects", json!("discrete")),
            set("/cc_params/entries/0/_other", json!("changed")),
            removed("/cc_params/entries/0/_other"),
            removed("/cc_params/entries/9/_notes"),
            // A named value, its title and its rank.
            at(Alter::Set("/cc_params/entries/0/values/0/value", json!(126)), "/cc_params/entries/0/values"),
            at(Alter::Set("/cc_params/entries/0/values/1/title", json!("Bob")), "/cc_params/entries/0/values"),
            at(Alter::Remove("/cc_params/entries/0/values/1"), "/cc_params/entries/0/values"),
            at(
                Alter::Set("/cc_params/entries/0/values", json!([{ "value": 0, "title": "Ann" }, { "value": 127, "title": "Zed" }])),
                "/cc_params/entries/0/values",
            ),
            removed("/cc_params/entries/5/values"),
            // The polarity, under the key of 0.6.
            at(Alter::Set("/cc_params/entries/0/polarity", json!("unipolar")), "/cc_params/entries/0/orientation"),
            at(Alter::Remove("/cc_params/entries/1/polarity"), "/cc_params/entries/1/orientation"),
            // The texts moved to their fields.
            at(Alter::Set("/cc_params/entries/0/title", json!("Other")), "/cc_params/entries/0/_name"),
            at(Alter::Remove("/cc_params/entries/0/value_description"), "/cc_params/entries/0/_usage"),
            // An entry with no way keeps its own fields.
            set("/cc_params/entries/5/range", json!([0, 4])),
            set("/cc_params/entries/5/default", json!(2)),
            removed("/cc_params/entries/5/transmit"),
            // The entries of `params`.
            set("/params/entries/0/default", json!(0)),
            removed("/params/entries/0/step_count"),
            set("/params/entries/0/section", json!("lfo")),
            at(Alter::Set("/params/entries/0/values/0/title", json!("Square")), "/params/entries/0/values"),
            at(Alter::Remove("/params/entries/0/values"), "/params/entries/0/values"),
            at(Alter::Set("/params/entries/0/description", json!("other")), "/params/entries/0/_description"),
            removed("/params/entries/0/_usage"),
            removed("/params/entries/4/range"),
            set("/params/entries/4/recognize", json!("none")),
        ],
    );
}

#[test]
fn a_key_of_an_entry_the_correspondence_does_not_give_is_named_at_its_0_7_pointer() {
    assert_each_named(
        Kind::Device,
        &rich_driver(),
        [
            // Equal maxima, nothing measured, no way: no scale is given.
            added("/cc_params/entries/3/scale", json!([0, 127])),
            added("/cc_params/entries/7/scale", json!([0, 127])),
            added("/cc_params/entries/5/scale", json!([0, 3])),
            added("/params/entries/0/scale", json!([0, 100])),
            // The scale prescribed is the range of the first way.
            set("/cc_params/entries/0/scale", json!([0, 16383])),
            removed("/cc_params/entries/0/scale"),
            removed("/cc_params/entries/6/scale"),
            removed("/cc_params/entries/8/scale"),
            added("/cc_params/entries/2/polarity", json!("bipolar")),
            added("/cc_params/entries/2/default", json!(0)),
            added("/cc_params/entries/2/title", json!("/c")),
            added("/cc_params/entries/2/_added", json!("new")),
            added("/params/entries/1/title", json!("/p1")),
            added("/params/entries/1/channel", json!(1)),
        ],
    );
}

/// The alterations of the sources of the rich driver: the 0.7 pointer altered
/// and the pointer named.
fn source_alterations() -> Vec<(Alter, &'static str)> {
    use Alter::{Remove, Set};
    vec![
        (Set("/sources/0/title", json!("x")), "/_sources/0/preset_name"),
        (Remove("/sources/0/title"), "/_sources/0/preset_name"),
        (Set("/sources/0/name", json!("x")), "/_sources/0/name"),
        (Remove("/sources/0/version"), "/_sources/0/revision"),
        (Set("/sources/0/sha256", json!("x")), "/_sources/0/content_sha256"),
        (Set("/sources/0/adaptation", json!("x")), "/_sources/0/adaptation"),
        (Set("/sources/0/url", json!("https://x.org/p/abc")), "/_sources/0/url"),
        (Remove("/sources/0/author"), "/_sources/0/author"),
        (Set("/sources/0/extra~1key", json!({ "a": 2 })), "/_sources/0/extra~1key"),
        (Remove("/sources/0/extra~1key"), "/_sources/0/extra~1key"),
        (Set("/sources/0/type", json!("vendor-doc")), "/_sources/0/type"),
        (Set("/sources/0/notes", json!("type in 0.6: electra-preset")), "/_sources/0/notes"),
        (Remove("/sources/0/provider"), "/sources/0/provider"),
        (Remove("/sources/1/title"), "/_sources/1/name"),
        (Remove("/sources/1/preset_id"), "/_sources/1/preset_id"),
        (Set("/sources/2/title", json!("x")), "/_sources/2/title"),
        (Remove("/sources/2/preset_name"), "/_sources/2/preset_name"),
        (Remove("/sources/2/revision"), "/_sources/2/revision"),
        (Remove("/sources/2/content_sha256"), "/_sources/2/content_sha256"),
        (Remove("/sources/2/preset_id"), "/_sources/2/preset_id"),
        (Set("/sources/3/license", json!("NONE")), "/_sources/3/license"),
        (Remove("/sources/3/notes"), "/sources/3/notes"),
        (Set("/sources/4/provider", json!("pencilresearch")), "/_sources/4/provider"),
        (Set("/sources/5/_provider_2", json!("x")), "/_sources/5/provider"),
        (Remove("/sources/5/_provider_2"), "/_sources/5/provider"),
        (Remove("/sources/5/_provider"), "/_sources/5/_provider"),
        (Set("/sources/6/_provider", json!(8)), "/_sources/6/provider"),
        (Set("/midi_of_sources/0/title", json!("x")), "/_midi_of_sources/0/name"),
        (Remove("/midi_of_sources/0/version"), "/_midi_of_sources/0/revision"),
        (Set("/midi_of_sources/1/url", json!("https://x.org/other")), "/_midi_of_sources/1/url"),
        (Remove("/midi_of_sources/1/firmware"), "/_midi_of_sources/1/firmware"),
    ]
}

#[test]
fn a_source_field_changed_or_removed_is_named_at_its_0_6_key_in_both_lists() {
    assert_each_named(Kind::Device, &rich_driver(), source_alterations());
}

#[test]
fn a_key_of_a_source_the_correspondence_does_not_give_is_named_at_its_0_7_pointer() {
    assert_each_named(
        Kind::Device,
        &rich_driver(),
        [
            added("/sources/3/provider", json!("midnam")),
            // Dropped by the conversion: they end the `url`.
            added("/sources/0/preset_id", json!("abc")),
            added("/sources/1/adaptation", json!("Acme")),
            added("/sources/1/name", json!("Only")),
            added("/sources/0/revision", json!("3")),
            added("/sources/3/_added", json!(1)),
            added("/midi_of_sources/1/author", json!("someone")),
        ],
    );
}

#[test]
fn a_source_or_a_list_of_sources_removed_or_added_is_named_at_the_list() {
    let source = json!({ "type": "vendor-doc" });
    assert_each_named(
        Kind::Device,
        &rich_driver(),
        [
            (Alter::Remove("/sources/6"), "/_sources"),
            (Alter::Add("/sources/-", source.clone()), "/_sources"),
            (Alter::Remove("/sources"), "/_sources"),
            (Alter::Remove("/midi_of_sources/1"), "/_midi_of_sources"),
            (Alter::Add("/midi_of_sources/-", source.clone()), "/_midi_of_sources"),
            (Alter::Remove("/midi_of_sources"), "/_midi_of_sources"),
        ],
    );
    assert_each_named(
        Kind::Device,
        &bare_driver(),
        [
            (Alter::Add("/sources", json!([source.clone()])), "/_sources"),
            (Alter::Add("/midi_of_sources", json!([source])), "/_midi_of_sources"),
        ],
    );
}

/// A driver with no source.
fn bare_driver() -> Value {
    json!({ "$schema": D06, "device": { "name": "Bare", "osc_prefix": "/bare" }, "cc_params": { "entries": [{ "osc": "/v", "cc": 7 }] } })
}

/// A driver whose sources have a date of import, one beside two notes that take
/// the first keys.
fn dated_driver() -> Value {
    let mut doc = bare_driver();
    doc["_sources"] = json!([
        { "type": "vendor-doc", "imported_at": "2026-01-01" },
        { "type": "vendor-doc", "imported_at": "2026-02-02", "_imported_at": "2025-12-01", "_imported_at_2": "2025-11-01" }
    ]);
    doc["_midi_of_sources"] = json!([{ "type": "community-re", "imported_at": "2026-03-03" }]);
    doc
}

/// The 0.6 text of the dated driver, and its 0.7 text as the correspondence
/// says it: each date kept under the first free note.
fn dated_texts() -> (String, Value) {
    let (old, mut new) = texts(Kind::Device, &dated_driver());
    new["sources"] = json!([
        { "type": "vendor-doc", "_imported_at": "2026-01-01" },
        { "type": "vendor-doc", "_imported_at": "2025-12-01", "_imported_at_2": "2025-11-01", "_imported_at_3": "2026-02-02" }
    ]);
    new["midi_of_sources"] = json!([{ "type": "community-re", "_imported_at": "2026-03-03" }]);
    (old, new)
}

#[test]
fn a_date_of_import_kept_under_the_first_free_note_is_no_difference() {
    let (old, new) = dated_texts();
    let failures = judge_text(Kind::Device, &old, &new);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn a_date_of_import_changed_removed_or_under_another_note_is_named() {
    use Alter::{Remove, Set};
    let (old, new) = dated_texts();
    for (alteration, named) in [
        (Set("/sources/0/_imported_at", json!("2026-01-02")), vec!["/_sources/0/imported_at"]),
        (Remove("/sources/0/_imported_at"), vec!["/_sources/0/imported_at"]),
        (Remove("/sources/1/_imported_at_3"), vec!["/_sources/1/imported_at"]),
        (Set("/sources/1/_imported_at", json!("2026-02-02")), vec!["/_sources/1/_imported_at"]),
        (Remove("/sources/1/_imported_at_2"), vec!["/_sources/1/_imported_at_2"]),
        (Remove("/midi_of_sources/0/_imported_at"), vec!["/_midi_of_sources/0/imported_at"]),
    ] {
        let mut altered = new.clone();
        alter(&mut altered, &alteration);
        let named: BTreeSet<String> = named.into_iter().map(String::from).collect();
        assert_eq!(differences(&judge_text(Kind::Device, &old, &altered)), named, "{alteration:?}");
    }
    let mut altered = new.clone();
    let source = altered["sources"][0].as_object_mut().unwrap();
    let date = source.remove("_imported_at").unwrap();
    source.insert("_imported_at_2".into(), date);
    let named = ["/_sources/0/imported_at", "/sources/0/_imported_at_2"].map(String::from);
    assert_eq!(differences(&judge_text(Kind::Device, &old, &altered)), named.into());
}

/// A driver of 0.6 whose controls give a `range`, no flat message and a
/// non-empty `also`.
fn unheld_driver() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "Unheld", "osc_prefix": "/unheld" },
        "cc_params": { "channel": 0, "entries": [
            // 0: nothing written in the range.
            { "osc": "/a", "range": [0, 127], "also": [{ "cc": 74 }] },
            // 1: a default the ways do not reach, and two note keys taken.
            { "osc": "/b", "range": [0, 500], "default": 300, "_range": "taken", "_range_2": "too", "also": [{ "cc": 75 }] },
            // 2: the range, not the scale of the first way.
            { "osc": "/c", "range": [0, 500], "values": { "Max": 500 }, "transmit": "absolute",
              "also": [{ "cc": 76, "range": [0, 9] }, { "nrpn_msb": 1, "nrpn_lsb": 0, "value_bits": 14 }] },
            // 3: the most precise way has that scale, the first has not: no scale.
            { "osc": "/d", "range": [0, 16383], "default": 8000,
              "also": [{ "cc": 77 }, { "nrpn_msb": 2, "nrpn_lsb": 0, "value_bits": 14 }] },
            // 4: a count of steps alone; no single NRPN way for the block.
            { "osc": "/e", "range": [0, 500], "step_count": 5, "nrpn_lsb_range": [0, 7], "also": [{ "cc": 78 }] }
        ] }
    })
}

#[test]
fn a_range_no_way_takes_kept_as_a_note_with_its_scale_is_no_difference() {
    let (old, new) = texts(Kind::Device, &unheld_driver());
    let failures = judge_text(Kind::Device, &old, &new);
    assert!(failures.is_empty(), "{failures:#?}");
    let found = run(&[("acme/unheld", unheld_driver())]);
    assert!(found.failures.is_empty() && found.refused_in_0_7.is_empty(), "{found}");
}

#[test]
fn a_converted_text_without_the_note_of_a_range_or_with_it_changed_is_named() {
    assert_each_named(
        Kind::Device,
        &unheld_driver(),
        [
            removed("/cc_params/entries/0/_range"),
            set("/cc_params/entries/0/_range", json!([0, 126])),
            removed("/cc_params/entries/1/_range_3"),
            set("/cc_params/entries/1/_range_3", json!([0, 499])),
            set("/cc_params/entries/1/_range", json!([0, 500])),
            removed("/cc_params/entries/1/_range_2"),
            removed("/cc_params/entries/3/_range"),
            // The range goes neither to the entry nor to a way.
            added("/cc_params/entries/0/range", json!([0, 127])),
            added("/cc_params/entries/0/ways/0/range", json!([0, 127])),
            added("/cc_params/entries/2/ways/1/range", json!([0, 500])),
        ],
    );
}

#[test]
fn the_scale_of_a_range_no_way_takes_removed_changed_or_added_is_named() {
    assert_each_named(
        Kind::Device,
        &unheld_driver(),
        [
            removed("/cc_params/entries/1/scale"),
            removed("/cc_params/entries/2/scale"),
            removed("/cc_params/entries/4/scale"),
            set("/cc_params/entries/2/scale", json!([0, 9])),
            // Nothing written in the range, or the scale of the most precise way: none.
            added("/cc_params/entries/0/scale", json!([0, 127])),
            added("/cc_params/entries/3/scale", json!([0, 16383])),
        ],
    );
}

#[test]
fn the_note_of_a_range_under_another_suffix_is_named_at_the_key_expected_and_at_the_key_written() {
    for (entry, expected, written) in [(0, "_range", "_range_2"), (1, "_range_3", "_range_4")] {
        let (old, mut new) = texts(Kind::Device, &unheld_driver());
        let control = new["cc_params"]["entries"][entry].as_object_mut().unwrap();
        let range = control.remove(expected).unwrap();
        control.insert(written.into(), range);
        let found = differences(&judge_text(Kind::Device, &old, &new));
        let named = [expected, written].map(|k| format!("/cc_params/entries/{entry}/{k}"));
        assert_eq!(found, named.into());
    }
}

#[test]
fn the_printed_trial_counts_zero_differences_with_the_0_6_text() {
    let found = run(&[("acme/rich", rich_driver())]);
    assert!(found.refused_in_0_6.is_empty() && found.refused_in_0_7.is_empty(), "{found}");
    let printed = found.to_string();
    assert!(printed.lines().any(|l| l == "differences with the 0.6 text: 0"), "{printed}");
}

#[test]
fn the_printed_trial_counts_the_differences_apart_from_the_other_failures_and_names_each() {
    let failure = |check, file: &str, pointer: &str| Failure {
        check,
        file: file.into(),
        pointer: pointer.into(),
        kind: None,
        why: "why".into(),
    };
    let found = Trial {
        files: 4,
        failures: vec![
            failure(Check::Loss, "acme/one", "/cc_params/entries/0/ways/0/ctrlIndex"),
            failure(Check::Refusal, "acme/two", "/device"),
            failure(Check::Schema, "acme/three", "/identity"),
            failure(Check::Unstable, "acme/three", "/programs"),
            failure(Check::Resolution, "acme/four", "/midi_of"),
            failure(Check::Loss, "acme/four", "/_sources/0/url"),
        ],
        ..Trial::default()
    };
    let printed = found.to_string();
    assert!(printed.lines().any(|l| l == "differences with the 0.6 text: 2"), "{printed}");
    for (file, pointer) in [("acme/one", "/cc_params/entries/0/ways/0/ctrlIndex"), ("acme/four", "/_sources/0/url")] {
        assert!(printed.lines().any(|l| l.contains(file) && l.contains(pointer)), "{pointer} in:\n{printed}");
    }
}

