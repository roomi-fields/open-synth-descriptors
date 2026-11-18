//! What the vocabulary of roles says so that a role is a path: a part says under which
//! parts it may stand (`stands_under`) and whether it may open a path (`at_top`, true
//! when `stands_under` is absent, false when it is present, unless the part writes it);
//! a term says where it is shared (`shared_by`, a list of `{maker, device}`); a source
//! may cite a document and its table (M2-104-UM table 22, M2-117-UM table 5) beside the
//! controller of MIDI 1.0. State 1 holds the names of table 22 that set a sound and the
//! parts and settings the standard leaves to each maker.

use crate::common;

use osd::{FaultKind, Kind, RoleMeaning};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const STATE_SCHEMA: &str = "https://w3id.org/osd/vocabulary/1/vocabulary.schema.json";

fn repo_file(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn shared() -> Value {
    json!([{"maker": "Moog", "device": "Minimoog"}, {"maker": "Roland", "device": "Juno-106"}])
}

fn midi_source(controller: u32, name: &str) -> Value {
    json!([{"controller": controller, "name": name}])
}

fn base_term(name: &str, kind: &str) -> Value {
    json!({"name": name, "type": kind, "definition": format!("The {name} of a sound."), "words": [], "shared_by": shared()})
}

fn part(name: &str) -> Value {
    base_term(name, "part")
}

fn setting(name: &str, applies_to: &[&str]) -> Value {
    let mut t = base_term(name, "setting");
    t["applies_to"] = json!(applies_to);
    t
}

/// A valid small state (indices 0 to 3): filter and cutoff cite CC 74, output and pan cite CC 10.
fn terms() -> Vec<Value> {
    vec![
        json!({"name": "filter", "type": "part", "definition": "The filter of a sound.", "words": [], "source": midi_source(74, "Brightness")}),
        json!({"name": "cutoff", "type": "setting", "definition": "The cutoff of a sound.", "applies_to": ["filter"], "words": [{"word": "Brightness", "origin": "MIDI 1.0"}], "source": midi_source(74, "Brightness")}),
        json!({"name": "output", "type": "part", "definition": "The output of a sound.", "words": [], "source": midi_source(10, "Pan")}),
        json!({"name": "pan", "type": "setting", "definition": "The pan of a sound.", "applies_to": ["output"], "words": [], "source": midi_source(10, "Pan")}),
    ]
}

fn state_of(terms: Vec<Value>) -> Value {
    json!({"$schema": STATE_SCHEMA, "state": "1", "terms": terms})
}

fn with(extra: Vec<Value>) -> Value {
    let mut t = terms();
    t.extend(extra);
    state_of(t)
}

fn read(doc: &Value) -> Result<osd::Vocabulary, osd::Fault> {
    osd::read_vocabulary(&doc.to_string())
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).err().unwrap_or_else(|| panic!("accepted: {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

fn validator() -> jsonschema::Validator {
    jsonschema::options().build(&serde_json::from_str::<Value>(&repo_file("schemas/vocabulary/1/vocabulary.schema.json")).unwrap()).unwrap()
}

fn schema_paths(doc: &Value) -> Vec<String> {
    validator().iter_errors(doc).map(|e| e.instance_path().to_string()).collect()
}

// Where a part stands.

#[test]
fn a_part_that_says_where_it_stands_is_read_with_or_without_at_top() {
    let mut envelope = part("envelope");
    envelope["stands_under"] = json!(["filter"]);
    assert!(read(&with(vec![envelope.clone()])).is_ok());
    envelope["at_top"] = json!(true);
    assert!(read(&with(vec![envelope.clone()])).is_ok(), "a part that stands under others and opens a path too");
    envelope["at_top"] = json!(false);
    assert!(read(&with(vec![envelope])).is_ok());
    // A part may name a part declared after it.
    let mut early = part("envelope");
    early["stands_under"] = json!(["late"]);
    let state = with(vec![early, part("late")]);
    assert!(read(&state).is_ok());
}

#[test]
fn stands_under_names_parts_of_the_state_and_only_those() {
    let mut t = part("envelope");
    t["stands_under"] = json!(["filter", "ghost"]);
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/stands_under/1");
    let mut t = part("envelope");
    t["stands_under"] = json!(["cutoff"]);
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/stands_under/0");
    let mut t = part("envelope");
    t["stands_under"] = json!([]);
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/stands_under");
}

#[test]
fn a_setting_neither_stands_under_a_part_nor_opens_a_path() {
    let mut t = setting("attack-time", &["filter"]);
    t["stands_under"] = json!(["filter"]);
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/stands_under");
    let mut t = setting("attack-time", &["filter"]);
    t["at_top"] = json!(true);
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/at_top");
    let mut t = part("envelope");
    t["at_top"] = json!("yes");
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/at_top");
}

#[test]
fn the_schema_of_the_vocabulary_states_where_a_part_stands() {
    let mut ok = part("envelope");
    ok["stands_under"] = json!(["filter"]);
    ok["at_top"] = json!(false);
    assert!(schema_paths(&with(vec![ok.clone()])).is_empty());
    let at = |t: Value| schema_paths(&with(vec![t]));
    let mut t = ok.clone();
    t["stands_under"] = json!([]);
    assert!(at(t).contains(&"/terms/4/stands_under".to_string()));
    let mut t = ok.clone();
    t["stands_under"] = json!(["filter", "filter"]);
    assert!(at(t).contains(&"/terms/4/stands_under".to_string()));
    let mut t = ok.clone();
    t["at_top"] = json!("yes");
    assert!(at(t).contains(&"/terms/4/at_top".to_string()));
    let mut t = setting("attack-time", &["filter"]);
    t["stands_under"] = json!(["filter"]);
    assert!(!at(t).is_empty(), "a setting with stands_under");
    let mut t = setting("attack-time", &["filter"]);
    t["at_top"] = json!(true);
    assert!(!at(t).is_empty(), "a setting with at_top");
}

// Where a term is shared.

#[test]
fn a_term_without_a_source_is_read_when_it_names_two_devices_of_two_makers() {
    assert!(read(&with(vec![part("mixer")])).is_ok());
}

#[test]
fn a_term_needs_a_source_or_a_shared_by() {
    let mut t = part("mixer");
    t.as_object_mut().unwrap().remove("shared_by");
    refused(&with(vec![t]), FaultKind::Missing, "/terms/4/source");
}

#[test]
fn a_shared_by_names_at_least_two_devices_of_two_distinct_makers() {
    let cases = [
        json!([{"maker": "Moog", "device": "Minimoog"}]),
        json!([{"maker": "Moog", "device": "Minimoog"}, {"maker": "Moog", "device": "Sub 37"}]),
        json!([{"maker": "Moog", "device": "Minimoog"}, {"maker": "Moog", "device": "Minimoog"}]),
        json!([]),
    ];
    for shared_by in cases {
        let mut t = part("mixer");
        t["shared_by"] = shared_by.clone();
        refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/shared_by");
    }
    // A term with a source and a shared_by holds both rules: a lone device is refused too.
    let mut t = setting("resonance", &["filter"]);
    t["source"] = midi_source(71, "Sound Controller 2 (Timbre/Harmonic Intensity)");
    t["shared_by"] = json!([{"maker": "Moog", "device": "Minimoog"}]);
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/shared_by");
}

#[test]
fn a_shared_by_entry_is_an_object_of_a_maker_and_a_device_and_never_a_text() {
    let at = |shared_by: Value, kind: FaultKind, pointer: &str| {
        let mut t = part("mixer");
        t["shared_by"] = shared_by;
        refused(&with(vec![t]), kind, pointer);
    };
    at(json!(["Moog Minimoog", "Roland Juno-106"]), FaultKind::BadValue, "/terms/4/shared_by/0");
    at(json!([{"maker": "Moog"}, {"maker": "Roland", "device": "Juno-106"}]), FaultKind::Missing, "/terms/4/shared_by/0/device");
    at(json!([{"maker": "Moog", "device": "Minimoog"}, {"device": "Juno-106"}]), FaultKind::Missing, "/terms/4/shared_by/1/maker");
    at(json!([{"maker": "", "device": "Minimoog"}, {"maker": "Roland", "device": "Juno-106"}]), FaultKind::BadValue, "/terms/4/shared_by/0/maker");
    at(json!([{"maker": "Moog", "device": ""}, {"maker": "Roland", "device": "Juno-106"}]), FaultKind::BadValue, "/terms/4/shared_by/0/device");
    at(
        json!([{"maker": "Moog", "device": "Minimoog", "year": 1970}, {"maker": "Roland", "device": "Juno-106"}]),
        FaultKind::UnknownField,
        "/terms/4/shared_by/0/year",
    );
    let mut t = part("mixer");
    t["shared_by"] = json!("Moog Minimoog and Roland Juno-106");
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/shared_by");
}

#[test]
fn the_schema_of_the_vocabulary_states_shared_by_and_the_need_of_a_source_or_a_shared_by() {
    assert!(schema_paths(&with(vec![part("mixer")])).is_empty());
    let at = |t: Value| schema_paths(&with(vec![t]));
    let mut t = part("mixer");
    t["shared_by"] = json!([{"maker": "Moog", "device": "Minimoog"}]);
    assert!(at(t).contains(&"/terms/4/shared_by".to_string()));
    let mut t = part("mixer");
    t["shared_by"] = json!(["Moog Minimoog", "Roland Juno-106"]);
    assert!(at(t).iter().any(|p| p.starts_with("/terms/4/shared_by")));
    let mut t = part("mixer");
    t.as_object_mut().unwrap().remove("shared_by");
    assert!(at(t).contains(&"/terms/4".to_string()), "neither a source nor a shared_by");
}

// A source by document.

fn doc_source(document: &str, table: &str, number: u64, name: &str) -> Value {
    json!({"document": document, "table": table, "number": number, "name": name})
}

#[test]
fn a_source_may_cite_a_document_and_its_table_beside_a_midi_1_0_controller() {
    let mut envelope = part("envelope");
    envelope.as_object_mut().unwrap().remove("shared_by");
    envelope["source"] = json!([doc_source("M2-104-UM v1.1.2", "22", 73, "Sound Controller 4 (Attack Time)")]);
    let mut attack = setting("attack-time", &["envelope"]);
    attack.as_object_mut().unwrap().remove("shared_by");
    attack["source"] = json!([doc_source("M2-104-UM v1.1.2", "22", 73, "Sound Controller 4 (Attack Time)"), json!({"document": "M2-117-UM v1.0.1", "table": "5", "name": "paramPath"})]);
    assert!(read(&with(vec![envelope.clone(), attack.clone()])).is_ok());
    assert!(schema_paths(&with(vec![envelope.clone(), attack.clone()])).is_empty());
    // Both forms side by side.
    let mut both = attack;
    both["source"] = json!([{"controller": 73, "name": "Sound Controller 4 (Attack Time)"}, doc_source("M2-104-UM v1.1.2", "22", 73, "Sound Controller 4 (Attack Time)")]);
    assert!(read(&with(vec![envelope, both])).is_ok());
}

#[test]
fn a_source_by_document_that_is_malformed_is_refused_at_its_place() {
    let make = |source: Value| {
        let mut t = part("envelope");
        t.as_object_mut().unwrap().remove("shared_by");
        t["source"] = source;
        with(vec![t])
    };
    let good = || doc_source("M2-104-UM v1.1.2", "22", 73, "Sound Controller 4 (Attack Time)");
    let without = |key: &str| {
        let mut s = good();
        s.as_object_mut().unwrap().remove(key);
        s
    };
    refused(&make(json!([without("table")])), FaultKind::Missing, "/terms/4/source/0/table");
    refused(&make(json!([without("name")])), FaultKind::Missing, "/terms/4/source/0/name");
    // The number of a row is written when the table numbers its rows: it may be absent.
    assert!(read(&make(json!([without("number")]))).is_ok());
    let mut s = good();
    s["table"] = json!("");
    refused(&make(json!([s])), FaultKind::BadValue, "/terms/4/source/0/table");
    let mut s = good();
    s["document"] = json!("");
    refused(&make(json!([s])), FaultKind::BadValue, "/terms/4/source/0/document");
    let mut s = good();
    s["number"] = json!(200);
    refused(&make(json!([s])), FaultKind::BadValue, "/terms/4/source/0/number");
    let mut s = good();
    s["colour"] = json!("red");
    refused(&make(json!([s])), FaultKind::UnknownField, "/terms/4/source/0/colour");
    // The schema refuses the same sources at the source.
    for bad in [without("table"), without("name")] {
        assert!(schema_paths(&make(json!([bad]))).iter().any(|p| p.starts_with("/terms/4/source/0")));
    }
}

// What the two rules of entry say on paths.

fn envelope_state(extra: Vec<Value>) -> Value {
    let mut envelope = part("envelope");
    envelope["stands_under"] = json!(["filter"]);
    let mut t = vec![envelope, setting("attack-time", &["envelope"])];
    t.extend(extra);
    with(t)
}

#[test]
fn a_term_that_says_a_path_the_state_already_holds_is_refused_at_the_term() {
    assert!(read(&envelope_state(vec![])).is_ok());
    // The names joined of a path of two parts and a setting.
    refused(&envelope_state(vec![setting("filter-envelope-attack-time", &["envelope"])]), FaultKind::BadValue, "/terms/6");
    // A maker's word that says the path.
    let mut snap = setting("snap", &["envelope"]);
    snap["words"] = json!([{"word": "Filter Envelope Attack Time", "origin": "Acme"}]);
    refused(&envelope_state(vec![snap]), FaultKind::BadValue, "/terms/6");
    // The composition of a part and a setting still holds.
    refused(&envelope_state(vec![setting("envelope-attack-time", &["envelope"])]), FaultKind::BadValue, "/terms/6");
}

#[test]
fn two_terms_whose_segments_are_the_same_in_a_path_are_refused_at_the_second_name() {
    // `x1` and `x-1` are both the segment X1.
    let state = with(vec![part("x1"), part("x-1")]);
    refused(&state, FaultKind::BadValue, "/terms/5/name");
    // Distinct segments stay accepted.
    assert!(read(&with(vec![part("x1"), part("x2")])).is_ok());
    assert!(read(&with(vec![part("ab"), part("a-b")])).is_ok(), "Ab and AB are two segments");
}

// State 1.

fn state_1() -> Value {
    serde_json::from_str(&repo_file("schemas/vocabulary/1/terms.json")).unwrap()
}

fn terms_1() -> Vec<Value> {
    state_1()["terms"].as_array().unwrap().clone()
}

fn named(name: &str) -> Value {
    terms_1().into_iter().find(|t| t["name"] == name).unwrap_or_else(|| panic!("no term {name}"))
}

fn strings(v: &Value) -> BTreeSet<String> {
    v.as_array().into_iter().flatten().map(|s| s.as_str().unwrap().to_string()).collect()
}

/// The terms of state 1 before this work: they keep their source and are not held to `shared_by`.
const BEFORE: [&str; 19] = [
    "voice", "sound", "filter", "envelope", "output", "effect", "portamento-time", "portamento", "portamento-start", "variation", "resonance", "cutoff",
    "attack-time", "release-time", "level", "balance", "pan", "expression", "depth",
];

const PARTS: [&str; 18] = [
    "oscillator", "noise", "mixer", "filter", "amplifier", "envelope", "lfo", "effect", "delay", "reverb", "chorus", "equalizer", "arpeggiator", "sequencer",
    "voice", "output", "performance", "vibrato",
];

const SETTINGS: [&str; 23] = [
    "waveform", "pitch", "detune", "level", "cutoff", "resonance", "attack-time", "decay-time", "sustain-level", "release-time", "rate", "depth", "delay-time",
    "feedback", "mix", "amount", "pan", "balance", "expression", "modulation", "breath", "variation", "glide-time",
];

#[test]
fn the_first_state_holds_the_parts_and_settings_the_standard_leaves_to_each_maker() {
    for p in PARTS {
        assert_eq!(named(p)["type"], json!("part"), "{p}");
    }
    // `glide-time` may stay out when `portamento-time` says it already.
    for s in SETTINGS.iter().filter(|s| **s != "glide-time") {
        assert_eq!(named(s)["type"], json!("setting"), "{s}");
    }
}

#[test]
fn each_term_of_the_first_state_has_a_source_or_names_where_it_is_shared() {
    let mut failures = Vec::new();
    for t in terms_1() {
        let name = t["name"].as_str().unwrap();
        let has_source = t.get("source").and_then(Value::as_array).is_some_and(|s| !s.is_empty());
        let shared = t.get("shared_by").and_then(Value::as_array);
        if !has_source && shared.is_none() {
            failures.push(format!("{name}: no source and no shared_by"));
        }
        if !BEFORE.contains(&name) && !name.starts_with("x-") && shared.is_none() {
            failures.push(format!("{name}: an added term cites where it is shared"));
        }
        if let Some(shared) = shared {
            let makers: BTreeSet<&str> = shared.iter().filter_map(|d| d["maker"].as_str()).collect();
            let devices: BTreeSet<(&str, &str)> = shared.iter().filter_map(|d| Some((d["maker"].as_str()?, d["device"].as_str()?))).collect();
            if shared.iter().any(|d| d["maker"].as_str().is_none_or(str::is_empty) || d["device"].as_str().is_none_or(str::is_empty)) {
                failures.push(format!("{name}: a device without its maker or its name"));
            }
            if makers.len() < 2 || devices.len() < 2 {
                failures.push(format!("{name}: {} devices of {} makers", devices.len(), makers.len()));
            }
        }
    }
    common::report("terms fail", failures);
}

#[test]
fn the_names_of_table_22_that_set_a_sound_are_held_each_with_its_document_and_table() {
    // (row of M2-104-UM table 22, the setting that holds it, a part it applies to)
    let rows = [
        (1, "modulation", "performance"),
        (2, "breath", "performance"),
        (3, "pitch", "voice"),
        (7, "level", "output"),
        (8, "balance", "output"),
        (10, "pan", "output"),
        (11, "expression", "performance"),
        (70, "variation", "sound"),
        (71, "resonance", "filter"),
        (72, "release-time", "envelope"),
        (73, "attack-time", "envelope"),
        (74, "cutoff", "filter"),
        (75, "decay-time", "envelope"),
        (76, "rate", "vibrato"),
        (77, "depth", "vibrato"),
        (78, "delay-time", "vibrato"),
        (91, "depth", "reverb"),
        (92, "depth", "effect"),
        (93, "depth", "chorus"),
        (94, "depth", "effect"),
        (95, "depth", "effect"),
    ];
    let mut failures = Vec::new();
    for (number, term, part) in rows {
        let t = named(term);
        if t["type"] != "setting" || !applies(&t, part) {
            failures.push(format!("{number}: {term} is not a setting that applies to {part}"));
        }
        if !cites_row(&t, number) {
            failures.push(format!("{number}: {term} does not cite M2-104-UM table 22 row {number}"));
        }
    }
    // Sound Controller 10 is undefined: no term.
    if terms_1().iter().any(|t| cites_row(t, 79)) {
        failures.push("79: Sound Controller 10 is undefined and has no term".into());
    }
    common::report("rows fail", failures);
}

fn applies(t: &Value, part: &str) -> bool {
    t["applies_to"].as_array().is_some_and(|a| a.contains(&json!(part)))
}

fn cites_row(t: &Value, number: u64) -> bool {
    t.get("source").and_then(Value::as_array).is_some_and(|a| {
        a.iter().any(|s| s["document"].as_str().is_some_and(|d| d.starts_with("M2-104-UM")) && s["table"] == json!("22") && s["number"] == json!(number))
    })
}

#[test]
fn the_makers_words_of_table_22_are_words_of_the_terms_that_hold_them() {
    let words = |t: &str| -> BTreeSet<String> { named(t)["words"].as_array().unwrap().iter().map(|w| w["word"].as_str().unwrap().to_string()).collect() };
    assert!(words("pitch").contains("Pitch 7.25"));
    assert!(words("level").contains("Volume") || words("level").contains("Channel Volume"));
    assert!(words("depth").contains("Reverb Send Level") && words("depth").contains("Chorus Send Level"));
    let effect = words("effect");
    assert!(effect.contains("Tremolo") && effect.contains("Phaser") && (effect.contains("Celeste") || effect.contains("Detune")), "{effect:?}");
    // A word names the document or the maker it comes from.
    for t in terms_1() {
        for w in t["words"].as_array().unwrap() {
            if ["Pitch 7.25", "Reverb Send Level", "Chorus Send Level", "Tremolo", "Phaser"].contains(&w["word"].as_str().unwrap()) {
                assert!(w["origin"].as_str().is_some_and(|o| !o.is_empty()), "{w}");
            }
        }
    }
}

#[test]
fn balance_and_expression_are_terms_of_their_own_with_what_a_player_moves_apart() {
    assert!(applies(&named("balance"), "output"));
    assert!(applies(&named("expression"), "performance"));
    for s in ["modulation", "breath"] {
        assert!(applies(&named(s), "performance"), "{s}");
    }
    assert!(terms_1().iter().all(|t| t["name"] != "volume"), "Volume is a maker's word of level");
    assert!(terms_1().iter().all(|t| t["name"] != "brightness"));
}

#[test]
fn the_vibrato_holds_its_rate_its_depth_and_its_delay_time() {
    for s in ["rate", "depth", "delay-time"] {
        assert!(applies(&named(s), "vibrato"), "{s}");
    }
    assert!(applies(&named("rate"), "lfo"), "the rate of an lfo");
}

#[test]
fn the_kinds_of_effect_are_parts_of_their_own_that_say_they_are_an_effect() {
    let kinds = ["delay", "reverb", "chorus", "equalizer"];
    for k in kinds {
        let t = named(k);
        assert!(t["definition"].as_str().unwrap().to_lowercase().contains("effect"), "{k}: {}", t["definition"]);
        let opens = t["at_top"] == json!(true) || (t.get("stands_under").is_none() && t["at_top"] != json!(false));
        assert!(opens, "{k} opens a path");
    }
    for s in ["mix", "feedback", "delay-time", "level"] {
        let t = named(s);
        for p in std::iter::once("effect").chain(kinds) {
            assert!(applies(&t, p), "{s} applies to {p}");
        }
    }
}

#[test]
fn the_envelope_and_the_lfo_say_where_they_stand() {
    let envelope = named("envelope");
    assert_eq!(strings(&envelope["stands_under"]), ["filter", "amplifier", "oscillator", "voice"].map(String::from).into_iter().collect());
    assert_ne!(envelope["at_top"], json!(true), "an envelope does not open a path");
    let lfo = named("lfo");
    assert_eq!(lfo["at_top"], json!(true));
    assert_eq!(strings(&lfo["stands_under"]), ["oscillator", "filter", "amplifier"].map(String::from).into_iter().collect());
    // The parts that stand under none open a path.
    for p in ["oscillator", "filter", "amplifier", "output", "voice", "sound", "effect"] {
        let t = named(p);
        assert!(t.get("stands_under").is_none() && t["at_top"] != json!(false), "{p}");
    }
}

#[test]
fn a_role_of_the_table_22_names_is_known_in_the_first_state() {
    let known = |parts: &[&str], setting: &str| -> bool {
        let parts: Vec<Value> = parts.iter().map(|p| json!({"part": p})).collect();
        let doc = json!({
            "$schema": "https://w3id.org/osd/0.6/device.schema.json",
            "role_vocabulary": "1",
            "device": {"name": "T", "osc_prefix": "/t"},
            "cc_params": {"entries": [{"osc": "/a", "cc": 1, "role": {"parts": parts, "setting": setting}}]}
        });
        osd::read(&doc.to_string(), Kind::Device).is_ok_and(|d| osd::roles(&d).iter().all(|r| r.meaning == RoleMeaning::Known) && osd::roles(&d).len() == 1)
    };
    for (parts, setting) in [
        (vec!["performance"], "modulation"),
        (vec!["performance"], "breath"),
        (vec!["performance"], "expression"),
        (vec!["voice"], "pitch"),
        (vec!["output"], "level"),
        (vec!["output"], "balance"),
        (vec!["output"], "pan"),
        (vec!["sound"], "variation"),
        (vec!["filter"], "resonance"),
        (vec!["filter"], "cutoff"),
        (vec!["amplifier", "envelope"], "attack-time"),
        (vec!["amplifier", "envelope"], "decay-time"),
        (vec!["amplifier", "envelope"], "release-time"),
        (vec!["filter", "envelope"], "attack-time"),
        (vec!["reverb"], "depth"),
        (vec!["chorus"], "depth"),
        (vec!["effect"], "depth"),
    ] {
        assert!(known(&parts, setting), "{parts:?} {setting}");
    }
}
