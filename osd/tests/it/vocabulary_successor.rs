//! `Vocabulary::check_successor`: a role valid in a state stays valid in the next. A term stays
//! with its type, loses no element of its `applies_to` or `stands_under`, and a part that opened a
//! path still does; a correction may change a definition, a source, a word or a `shared_by`.

use osd::{FaultKind, Vocabulary};
use serde_json::{json, Value};

fn term(name: &str, kind: &str) -> Value {
    json!({
        "name": name,
        "type": kind,
        "definition": format!("The {name} of a sound."),
        "words": [{"word": format!("{name} word"), "origin": "Acme"}],
        "shared_by": [{"maker": "Moog", "device": "Minimoog"}, {"maker": "Roland", "device": "Juno-106"}]
    })
}

fn with(mut t: Value, key: &str, value: Value) -> Value {
    t[key] = value;
    t
}

const FILTER: usize = 0;
const AMPLIFIER: usize = 1;
const ENVELOPE: usize = 2;
const LFO: usize = 3;
const LEVEL: usize = 4;
const SHAPE: usize = 5;
const CURVE: usize = 6;
const FLAT: usize = 7;

fn terms() -> Vec<Value> {
    vec![
        term("filter", "part"),
        term("amplifier", "part"),
        with(term("envelope", "part"), "stands_under", json!(["filter", "amplifier"])),
        with(with(term("lfo", "part"), "stands_under", json!(["filter"])), "at_top", json!(true)),
        with(term("level", "setting"), "applies_to", json!(["filter", "amplifier"])),
        with(term("shape", "setting"), "applies_to", json!(["lfo"])),
        with(term("curve", "setting"), "applies_to", json!(["envelope"])),
        with(term("flat", "value"), "applies_to", json!(["shape", "curve"])),
    ]
}

fn read(terms: Vec<Value>) -> Vocabulary {
    let doc = json!({"$schema": "https://w3id.org/osd/vocabulary/2/vocabulary.schema.json", "state": "2", "terms": terms});
    osd::read_vocabulary(&doc.to_string()).unwrap_or_else(|f| panic!("{f}: {doc}"))
}

fn changed(change: impl FnOnce(&mut Vec<Value>)) -> Result<(), osd::Fault> {
    let mut next = terms();
    change(&mut next);
    read(terms()).check_successor(&read(next))
}

fn refused(what: &str, change: impl FnOnce(&mut Vec<Value>)) {
    let f = changed(change).err().unwrap_or_else(|| panic!("accepted: {what}"));
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/terms"), "{what}: {f}");
}

fn accepted(what: &str, change: impl FnOnce(&mut Vec<Value>)) {
    if let Err(f) = changed(change) {
        panic!("{what}: {f}");
    }
}

fn remove(t: &mut Value, key: &str) {
    t.as_object_mut().unwrap().remove(key);
}

#[test]
fn a_setting_that_loses_a_part_it_applies_to_is_refused() {
    refused("level loses amplifier", |t| t[LEVEL]["applies_to"] = json!(["filter"]));
    refused("level loses filter", |t| t[LEVEL]["applies_to"] = json!(["amplifier"]));
    refused("level swaps a part for another", |t| t[LEVEL]["applies_to"] = json!(["filter", "lfo"]));
}

#[test]
fn a_value_that_loses_a_setting_it_applies_to_is_refused() {
    refused("flat loses curve", |t| t[FLAT]["applies_to"] = json!(["shape"]));
    refused("flat swaps a setting for another", |t| t[FLAT]["applies_to"] = json!(["shape", "level"]));
}

#[test]
fn a_part_that_loses_a_part_it_stands_under_is_refused() {
    refused("envelope no longer under amplifier", |t| t[ENVELOPE]["stands_under"] = json!(["filter"]));
    refused("lfo swaps filter for amplifier", |t| t[LFO]["stands_under"] = json!(["amplifier"]));
    refused("lfo loses every stands_under", |t| {
        remove(&mut t[LFO], "stands_under");
    });
}

#[test]
fn a_part_that_no_longer_opens_a_path_is_refused() {
    // A part without stands_under opens a path: gaining one without writing at_top closes it.
    refused("amplifier gains stands_under", |t| t[AMPLIFIER]["stands_under"] = json!(["filter"]));
    refused("amplifier writes at_top false", |t| t[AMPLIFIER]["at_top"] = json!(false));
    refused("lfo writes at_top false", |t| t[LFO]["at_top"] = json!(false));
    refused("lfo drops its at_top", |t| {
        remove(&mut t[LFO], "at_top");
    });
}

#[test]
fn a_next_state_may_add_elements_terms_and_openings() {
    accepted("the same state", |_| {});
    accepted("level also on lfo", |t| t[LEVEL]["applies_to"] = json!(["filter", "amplifier", "lfo"]));
    accepted("flat also of level", |t| t[FLAT]["applies_to"] = json!(["shape", "curve", "level"]));
    accepted("envelope also under lfo", |t| t[ENVELOPE]["stands_under"] = json!(["filter", "amplifier", "lfo"]));
    accepted("amplifier stands under filter and still opens a path", |t| {
        t[AMPLIFIER]["stands_under"] = json!(["filter"]);
        t[AMPLIFIER]["at_top"] = json!(true);
    });
    accepted("envelope opens a path", |t| t[ENVELOPE]["at_top"] = json!(true));
    accepted("a new value", |t| t.push(with(term("steep", "value"), "applies_to", json!(["curve"]))));
    accepted("the same elements in another order", |t| {
        t[LEVEL]["applies_to"] = json!(["amplifier", "filter"]);
        t[ENVELOPE]["stands_under"] = json!(["amplifier", "filter"]);
    });
    accepted("at_top written where it was implied", |t| {
        t[FILTER]["at_top"] = json!(true);
        t[ENVELOPE]["at_top"] = json!(false);
    });
}

#[test]
fn a_next_state_may_correct_a_definition_a_source_a_word_or_a_shared_by() {
    accepted("a definition", |t| t[FLAT]["definition"] = json!("Another sentence."));
    accepted("a word changed", |t| t[LEVEL]["words"] = json!([{"word": "Volume", "origin": "Acme"}]));
    accepted("a word withdrawn", |t| t[LEVEL]["words"] = json!([]));
    accepted("a source in place of shared_by", |t| {
        remove(&mut t[FLAT], "shared_by");
        t[FLAT]["source"] = json!([{"document": "SFZ", "table": "fil_type", "name": "flat"}]);
    });
    accepted("another shared_by", |t| {
        t[SHAPE]["shared_by"] = json!([{"maker": "Korg", "device": "MS-20"}, {"maker": "Yamaha", "device": "DX7"}])
    });
}

#[test]
fn a_value_that_becomes_a_setting_or_goes_is_refused() {
    refused("flat becomes a setting", |t| {
        t[FLAT]["type"] = json!("setting");
        t[FLAT]["applies_to"] = json!(["filter"]);
    });
    refused("curve goes", |t| {
        t[FLAT]["applies_to"] = json!(["shape"]);
        t.remove(CURVE);
    });
}

#[test]
fn state_2_is_a_successor_of_state_1() {
    let one = osd::vocabulary_state("1").expect("state 1");
    let two = osd::vocabulary_state("2").expect("state 2");
    if let Err(f) = one.check_successor(two) {
        panic!("{f}");
    }
    // The other way round, the new terms go.
    assert!(two.check_successor(one).is_err());
}
