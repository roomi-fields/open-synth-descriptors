//! The vocabulary of roles: data under `schemas/vocabulary/<state>/` (its own
//! schema `vocabulary.schema.json`, its terms `terms.json`), a numbered state
//! never changed once published, each term of one name, one definition of one
//! sentence, a type (`part`, `setting` or `value`), the makers' words that mean
//! it each with its origin, and a `source`, the MIDI 1.0 controllers it comes
//! from. A `setting` lists the parts it `applies_to`, a `value` the settings. `osd::read_vocabulary` checks a
//! state, `osd::vocabulary_state` holds the published ones, and a state's
//! successor only adds (`Vocabulary::check_successor`).

use crate::common;

use osd::{Descriptor, FaultKind, Kind, RoleMeaning};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const STATE_SCHEMA: &str = "https://w3id.org/osd/vocabulary/1/vocabulary.schema.json";

fn repo_file(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn term(name: &str, kind: &str, controller: (u32, &str), words: &[(&str, &str)]) -> Value {
    json!({
        "name": name,
        "type": kind,
        "definition": format!("The {name} of a sound."),
        "words": words.iter().map(|(w, o)| json!({"word": w, "origin": o})).collect::<Vec<_>>(),
        "source": [{"controller": controller.0, "name": controller.1}]
    })
}

fn setting(name: &str, applies_to: &[&str], controller: (u32, &str), words: &[(&str, &str)]) -> Value {
    let mut t = term(name, "setting", controller, words);
    t["applies_to"] = json!(applies_to);
    t
}

/// A valid small state: filter and cutoff cite CC 74, output and pan cite CC 10.
fn terms() -> Vec<Value> {
    vec![
        term("filter", "part", (74, "Brightness"), &[]),
        setting("cutoff", &["filter"], (74, "Brightness"), &[("Brightness", "MIDI 1.0")]),
        term("output", "part", (10, "Pan"), &[]),
        setting("pan", &["output"], (10, "Pan"), &[("Pan", "MIDI 1.0")]),
    ]
}

fn state_of(terms: Vec<Value>) -> Value {
    json!({"$schema": STATE_SCHEMA, "state": "1", "terms": terms})
}

fn read(doc: &Value) -> Result<osd::Vocabulary, osd::Fault> {
    osd::read_vocabulary(&doc.to_string())
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).err().unwrap_or_else(|| panic!("accepted: {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

fn with_last(extra: Value) -> Value {
    let mut t = terms();
    t.push(extra);
    state_of(t)
}

#[test]
fn a_state_whose_terms_each_have_one_name_one_definition_and_a_source_is_read() {
    assert!(read(&state_of(terms())).is_ok());
}

#[test]
fn a_term_name_present_twice_is_refused_at_the_second_name() {
    let dup = term("filter", "part", (74, "Brightness"), &[]);
    refused(&with_last(dup), FaultKind::BadValue, "/terms/4/name");
    let mut t = terms();
    t.insert(1, term("output", "part", (10, "Pan"), &[]));
    refused(&state_of(t), FaultKind::BadValue, "/terms/3/name");
}

#[test]
fn a_makers_word_present_under_two_terms_is_refused_at_the_second_word() {
    let a = setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[("Timbre", "MIDI 1.0"), ("Brightness", "Acme")]);
    refused(&with_last(a), FaultKind::BadValue, "/terms/4/words/1/word");
    // The same word in another case is the same word.
    let b = setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[("brightness", "Acme")]);
    refused(&with_last(b), FaultKind::BadValue, "/terms/4/words/0/word");
}

#[test]
fn a_makers_word_that_is_the_name_of_another_term_is_refused() {
    let c = setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[("pan", "Acme")]);
    refused(&with_last(c), FaultKind::BadValue, "/terms/4/words/0/word");
}

#[test]
fn a_term_without_a_one_sentence_definition_is_refused_at_the_definition() {
    for (def, kind) in [
        (json!(""), FaultKind::BadValue),
        (json!("A cutoff. And another sentence."), FaultKind::BadValue),
        (json!(3), FaultKind::BadValue),
    ] {
        let mut t = setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[]);
        t["definition"] = def.clone();
        refused(&with_last(t), kind, "/terms/4/definition");
    }
    let mut t = setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[]);
    t.as_object_mut().unwrap().remove("definition");
    refused(&with_last(t), FaultKind::Missing, "/terms/4/definition");
}

#[test]
fn a_makers_word_without_its_origin_is_refused_at_the_word() {
    let mut t = setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[("Timbre", "MIDI 1.0")]);
    t["words"] = json!([{"word": "Timbre", "origin": "MIDI 1.0"}, {"word": "Resonance"}]);
    refused(&with_last(t.clone()), FaultKind::Missing, "/terms/4/words/1/origin");
    t["words"] = json!([{"word": "Resonance", "origin": ""}]);
    refused(&with_last(t), FaultKind::BadValue, "/terms/4/words/0/origin");
}

#[test]
fn a_term_without_a_source_is_refused_at_the_source() {
    let mut t = setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[]);
    t["source"] = json!([]);
    refused(&with_last(t.clone()), FaultKind::BadValue, "/terms/4/source");
    t.as_object_mut().unwrap().remove("source");
    refused(&with_last(t.clone()), FaultKind::Missing, "/terms/4/source");
    t["source"] = json!([{"controller": 71}]);
    refused(&with_last(t.clone()), FaultKind::Missing, "/terms/4/source/0/name");
    t["source"] = json!([{"controller": 200, "name": "Nothing"}]);
    refused(&with_last(t), FaultKind::BadValue, "/terms/4/source/0/controller");
}

#[test]
fn a_term_of_a_type_outside_part_setting_and_value_is_refused_at_its_type() {
    let t = term("resonance", "knob", (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[]);
    refused(&with_last(t), FaultKind::BadValue, "/terms/4/type");
}

#[test]
fn a_setting_lists_the_parts_it_applies_to_and_only_parts_of_the_same_state() {
    let ctl = (71, "Sound Controller 2 (Timbre/Harmonic Intensity)");
    let mut t = setting("resonance", &["filter"], ctl, &[]);
    t.as_object_mut().unwrap().remove("applies_to");
    refused(&with_last(t), FaultKind::Missing, "/terms/4/applies_to");
    // A name the state lacks.
    refused(&with_last(setting("resonance", &["filter", "ghost"], ctl, &[])), FaultKind::BadValue, "/terms/4/applies_to/1");
    // A term that is not a part.
    refused(&with_last(setting("resonance", &["cutoff"], ctl, &[])), FaultKind::BadValue, "/terms/4/applies_to/0");
    // A part may name a part declared after the setting: order does not matter.
    let mut t = terms();
    t.insert(0, setting("resonance", &["filter"], ctl, &[]));
    assert!(read(&state_of(t)).is_ok());
}

#[test]
fn a_term_that_composes_a_part_and_a_setting_it_already_holds_is_refused_at_the_term() {
    // `brightness` (CC 74) next to filter + cutoff, each citing CC 74.
    let brightness = setting("brightness", &["filter"], (74, "Brightness"), &[]);
    refused(&with_last(brightness), FaultKind::BadValue, "/terms/4");
    // By name: the joined names of a part and a setting.
    let joined = setting("filter-cutoff", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[]);
    refused(&with_last(joined), FaultKind::BadValue, "/terms/4");
    // By a maker's word: a part and a setting written together.
    let word = setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[("Filter Cutoff", "Acme")]);
    refused(&with_last(word), FaultKind::BadValue, "/terms/4");
    // Another composition of the same state.
    let pan = setting("stereo-position", &["output"], (10, "Pan"), &[]);
    refused(&with_last(pan), FaultKind::BadValue, "/terms/4");
}

#[test]
fn a_replaced_term_keeps_its_place_and_names_the_term_that_replaces_it() {
    let mut old = setting("sweep", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[]);
    old["replaced_by"] = json!("cutoff");
    assert!(read(&with_last(old.clone())).is_ok());
    old["replaced_by"] = json!("ghost");
    refused(&with_last(old), FaultKind::BadValue, "/terms/4/replaced_by");
}

#[test]
fn a_state_only_adds_to_the_one_before() {
    let before = read(&state_of(terms())).unwrap();
    let mut grown = terms();
    grown.push(setting("resonance", &["filter"], (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"), &[]));
    assert!(before.check_successor(&read(&state_of(grown)).unwrap()).is_ok());
    assert!(before.check_successor(&before).is_ok());
    // A term gone, or renamed, is a refusal.
    let mut removed = terms();
    removed.remove(3);
    let f = before.check_successor(&read(&state_of(removed)).unwrap()).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/terms"), "{f}");
    let mut renamed = terms();
    renamed[3]["name"] = json!("balance");
    assert!(before.check_successor(&read(&state_of(renamed)).unwrap()).is_err());
    // A term replaced stays, with `replaced_by`.
    let mut replaced = terms();
    replaced[3]["replaced_by"] = json!("cutoff");
    assert!(before.check_successor(&read(&state_of(replaced)).unwrap()).is_ok());
}

#[test]
fn a_vocabulary_that_is_not_json_or_lacks_its_terms_is_refused() {
    let f = osd::read_vocabulary("{\"terms\": ").unwrap_err();
    assert_eq!(f.kind, FaultKind::Syntax);
    refused(&json!({"$schema": STATE_SCHEMA, "state": "1"}), FaultKind::Missing, "/terms");
    refused(&json!({"$schema": STATE_SCHEMA, "terms": []}), FaultKind::Missing, "/state");
    refused(&json!({"$schema": STATE_SCHEMA, "state": "1", "terms": [], "colour": 1}), FaultKind::UnknownField, "/colour");
    let mut t = terms();
    t[0]["colour"] = json!("red");
    refused(&state_of(t), FaultKind::UnknownField, "/terms/0/colour");
}

// The schema of the vocabulary states what a schema can: shapes and lists.

/// The validator of the schema of a state, from its folder.
fn validator_of(state: &str) -> jsonschema::Validator {
    let schema: Value = serde_json::from_str(&repo_file(&format!("schemas/vocabulary/{state}/vocabulary.schema.json"))).unwrap();
    assert_eq!(schema["$id"], format!("https://w3id.org/osd/vocabulary/{state}/vocabulary.schema.json"));
    jsonschema::options().build(&schema).unwrap()
}

/// The validator of the schema of the state a document says it is.
fn schema_of(doc: &Value) -> jsonschema::Validator {
    validator_of(doc["state"].as_str().unwrap_or_else(|| panic!("no state: {doc}")))
}

fn state_validator() -> jsonschema::Validator {
    validator_of("1")
}

fn schema_paths(doc: &Value) -> Vec<String> {
    state_validator().iter_errors(doc).map(|e| e.instance_path().to_string()).collect()
}

#[test]
fn the_schema_of_the_vocabulary_accepts_a_valid_state_and_refuses_each_malformed_term_at_it() {
    assert!(schema_paths(&state_of(terms())).is_empty());
    let ctl = (71, "Sound Controller 2 (Timbre/Harmonic Intensity)");
    let base = || setting("resonance", &["filter"], ctl, &[("Timbre", "MIDI 1.0")]);
    let at = |t: Value| schema_paths(&with_last(t));
    let mut t = base();
    t["source"] = json!([]);
    assert!(at(t).contains(&"/terms/4/source".to_string()));
    let mut t = base();
    t.as_object_mut().unwrap().remove("source");
    assert!(at(t).contains(&"/terms/4".to_string()));
    let mut t = base();
    t["definition"] = json!("");
    assert!(at(t).contains(&"/terms/4/definition".to_string()));
    let mut t = base();
    t["definition"] = json!("One sentence. Two sentences.");
    assert!(at(t).contains(&"/terms/4/definition".to_string()));
    let mut t = base();
    t["words"] = json!([{"word": "Timbre"}]);
    assert!(at(t).contains(&"/terms/4/words/0".to_string()));
    let mut t = base();
    t["type"] = json!("knob");
    assert!(at(t).contains(&"/terms/4/type".to_string()));
    let mut t = base();
    t.as_object_mut().unwrap().remove("applies_to");
    assert!(at(t).contains(&"/terms/4".to_string()), "a setting without `applies_to`");
    let mut t = base();
    t["colour"] = json!("red");
    assert!(at(t).contains(&"/terms/4".to_string()));
}

// The cases of conformance/0.6/vocabulary/<case>/ (vocabulary.json and case.json).

/// (case, fault kind, pointer, whether only the reader refuses it). Each
/// refused case holds a state of four valid terms (a part, its setting, a
/// part, its setting) followed by the term at index 4 that the case refuses;
/// `valid-state` holds a state read as it is, with `result` in its case.json
/// and `fault` in each refused case's, `schema_accepts: true` when only the
/// reader refuses it.
const VOCABULARY_CASES: [(&str, &str, &str, bool); 12] = [
    ("duplicate-name-refused", "BadValue", "/terms/4/name", true),
    ("duplicate-word-refused", "BadValue", "/terms/4/words/0/word", true),
    ("word-without-origin-refused", "Missing", "/terms/4/words/0/origin", false),
    ("composition-refused", "BadValue", "/terms/4", true),
    ("applies-to-absent-refused", "BadValue", "/terms/4/applies_to/0", true),
    ("definition-two-sentences-refused", "BadValue", "/terms/4/definition", false),
    ("value-applies-to-missing-refused", "Missing", "/terms/4/applies_to", false),
    ("value-applies-to-empty-refused", "BadValue", "/terms/4/applies_to", false),
    ("value-applies-to-part-refused", "BadValue", "/terms/4/applies_to/0", true),
    ("value-stands-under-refused", "BadValue", "/terms/4/stands_under", false),
    ("value-at-top-refused", "BadValue", "/terms/4/at_top", false),
    ("setting-applies-to-value-refused", "BadValue", "/terms/4/applies_to/0", true),
];

/// The cases that hold a valid state.
const VALID_VOCABULARY_CASES: [&str; 2] = ["valid-state", "value-state-valid"];

fn parent(pointer: &str) -> &str {
    pointer.rsplit_once('/').map_or("", |(p, _)| p)
}

#[test]
fn the_vocabulary_conformance_cases_hold_a_valid_state_and_each_refusal_at_its_pointer() {
    let root = common::repo().join("conformance/0.6/vocabulary");
    let mut failures = Vec::new();
    let read_case = |name: &str| -> (String, Value) {
        let dir = format!("conformance/0.6/vocabulary/{name}");
        (repo_file(&format!("{dir}/vocabulary.json")), serde_json::from_str(&repo_file(&format!("{dir}/case.json"))).unwrap())
    };
    // Each valid state reads, with the terms its case states, and validates.
    for name in VALID_VOCABULARY_CASES {
        let (text, case) = read_case(name);
        assert!(case.get("result").is_some() && case.get("fault").is_none(), "{case}");
        match osd::read_vocabulary(&text) {
            Ok(v) => {
                let terms: Vec<&str> = v.terms.iter().map(|t| t.name.as_str()).collect();
                if case["result"]["terms"] != json!(terms) {
                    failures.push(format!("{name}: reads {terms:?}"));
                }
            }
            Err(f) => failures.push(format!("{name}: {f}")),
        }
        let doc: Value = serde_json::from_str(&text).unwrap();
        if !schema_of(&doc).is_valid(&doc) {
            failures.push(format!("{name}: the schema refuses it"));
        }
    }
    for (name, kind, pointer, reader_only) in VOCABULARY_CASES {
        let (text, case) = read_case(name);
        if case["fault"] != json!({"kind": kind, "pointer": pointer}) {
            failures.push(format!("{name}: case.json states {}", case["fault"]));
        }
        if (case.get("schema_accepts") == Some(&json!(true))) != reader_only {
            failures.push(format!("{name}: schema_accepts is not {reader_only}"));
        }
        match osd::read_vocabulary(&text) {
            Ok(_) => failures.push(format!("{name}: read, but should be refused")),
            Err(f) if (format!("{:?}", f.kind), f.pointer.as_str()) != (kind.to_string(), pointer) => {
                failures.push(format!("{name}: refused with {f}"))
            }
            Err(_) => {}
        }
        let doc: Value = serde_json::from_str(&text).unwrap();
        let paths: Vec<String> = schema_of(&doc).iter_errors(&doc).map(|e| e.instance_path().to_string()).collect();
        let ok = if reader_only { paths.is_empty() } else { paths.iter().any(|p| p == pointer || p == parent(pointer)) };
        if !ok {
            failures.push(format!("{name}: schema errors {paths:?}"));
        }
    }
    let names: BTreeSet<String> = std::fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    let want: BTreeSet<String> = VOCABULARY_CASES.iter().map(|c| c.0).chain(VALID_VOCABULARY_CASES).map(str::to_string).collect();
    assert_eq!(names, want);
    common::report("vocabulary cases failed", failures);
}

// The published state 1: the data.

fn state_1() -> Value {
    serde_json::from_str(&repo_file("schemas/vocabulary/1/terms.json")).unwrap()
}

fn terms_1() -> Vec<Value> {
    state_1()["terms"].as_array().unwrap().clone()
}

fn named(name: &str) -> Value {
    terms_1().into_iter().find(|t| t["name"] == name).unwrap_or_else(|| panic!("no term {name}"))
}

fn cites(t: &Value, number: u64, name: &str) -> bool {
    t["source"].as_array().unwrap().iter().any(|s| s["controller"] == json!(number) && s["name"] == json!(name))
}

#[test]
fn the_vocabulary_states_osd_holds_are_the_folders_under_schemas_vocabulary() {
    let mut folders: Vec<String> = std::fs::read_dir(common::repo().join("schemas/vocabulary"))
        .unwrap()
        .filter_map(|e| e.ok().filter(|e| e.path().is_dir()).map(|e| e.file_name().into_string().unwrap()))
        .collect();
    folders.sort();
    let mut held: Vec<String> = osd::VOCABULARY_STATES.iter().map(|s| s.to_string()).collect();
    held.sort();
    assert_eq!(held, folders);
    assert!(folders.contains(&"1".to_string()));
    for s in &folders {
        assert!(osd::vocabulary_state(s).is_some(), "{s}");
        let text = repo_file(&format!("schemas/vocabulary/{s}/terms.json"));
        assert!(osd::read_vocabulary(&text).is_ok(), "{s}");
        let doc: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(doc["state"], json!(s));
        assert!(validator_of(s).is_valid(&doc), "{s}: the schema refuses it");
    }
    assert!(osd::vocabulary_state("999").is_none());
    assert!(osd::vocabulary_state("").is_none());
}

#[test]
fn each_state_keeps_every_term_of_the_state_before_it() {
    let mut states: Vec<String> = osd::VOCABULARY_STATES.iter().map(|s| s.to_string()).collect();
    states.sort_by_key(|s| s.parse::<u64>().unwrap());
    for pair in states.windows(2) {
        let (a, b) = (osd::vocabulary_state(&pair[0]).unwrap(), osd::vocabulary_state(&pair[1]).unwrap());
        assert!(a.check_successor(b).is_ok(), "{} to {}", pair[0], pair[1]);
    }
}

#[test]
fn a_published_state_that_is_recorded_in_the_checksums_is_unchanged() {
    use sha2::{Digest, Sha256};
    let sums = repo_file("schemas/SHA256SUMS");
    for line in sums.lines().filter(|l| l.contains("vocabulary/")) {
        let (hash, path) = line.split_once("  ").unwrap();
        let bytes = std::fs::read(common::repo().join("schemas").join(path)).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), hash, "{path}");
    }
}

#[test]
fn the_first_state_names_its_parts_and_settings_from_the_defined_controllers() {
    for (name, kind) in [("filter", "part"), ("cutoff", "setting"), ("voice", "part"), ("portamento-time", "setting"), ("output", "part"), ("pan", "setting")] {
        assert_eq!(named(name)["type"], json!(kind), "{name}");
    }
    assert!(named("cutoff")["applies_to"].as_array().unwrap().contains(&json!("filter")));
    assert!(named("portamento-time")["applies_to"].as_array().unwrap().contains(&json!("voice")));
    assert!(named("pan")["applies_to"].as_array().unwrap().contains(&json!("output")));
    // Each function enters once: CC 74 is filter + cutoff, not a term of its own.
    assert!(terms_1().iter().all(|t| t["name"] != "brightness"));
    assert!(named("cutoff")["words"].as_array().unwrap().iter().any(|w| w["word"] == "Brightness"));
    // Each controller of a composition is cited by each term the composition uses.
    for (t, n, w) in [("filter", 74, "Brightness"), ("cutoff", 74, "Brightness"), ("voice", 5, "Portamento Time"), ("portamento-time", 5, "Portamento Time"), ("output", 10, "Pan"), ("pan", 10, "Pan")] {
        assert!(cites(&named(t), n, w), "{t} cites CC {n} {w}");
    }
}

#[test]
fn each_term_of_the_first_state_cites_a_controller_of_the_midi_1_0_table_and_each_word_its_origin() {
    let mut failures = Vec::new();
    for t in terms_1() {
        let name = t["name"].as_str().unwrap().to_string();
        let source = t.get("source").and_then(Value::as_array).cloned().unwrap_or_default();
        if source.is_empty() && t.get("shared_by").is_none() {
            failures.push(format!("{name}: no source and no shared_by"));
        }
        // A source by controller is a row of the MIDI 1.0 table; one by document cites its document and table.
        for s in source.iter().filter(|s| s.get("document").is_none()) {
            if !common::midi1::is_defined(s["controller"].as_u64().unwrap(), s["name"].as_str().unwrap()) {
                failures.push(format!("{name}: {s} is not a row of the MIDI 1.0 table"));
            }
        }
        for s in source.iter().filter(|s| s.get("document").is_some()) {
            let ok = ["document", "table", "name"].iter().all(|k| s[*k].as_str().is_some_and(|v| !v.is_empty()));
            if !ok {
                failures.push(format!("{name}: {s} is not a document and a table with its row"));
            }
        }
        for w in t["words"].as_array().unwrap() {
            if w["origin"].as_str().is_none_or(|o| o.is_empty()) {
                failures.push(format!("{name}: word {w} has no origin"));
            }
        }
    }
    common::report("terms fail", failures);
}

#[test]
fn the_first_state_covers_the_defined_controllers_that_set_a_sound() {
    // Portamento time, volume, balance, pan, expression, portamento on/off,
    // the sound controllers 1 to 5, portamento control and the effects depths.
    let sets_a_sound = [5, 7, 8, 10, 11, 65, 70, 71, 72, 73, 74, 84, 91, 92, 93, 94, 95];
    let cited: BTreeSet<u64> = terms_1()
        .iter()
        .flat_map(|t| t.get("source").and_then(Value::as_array).cloned().unwrap_or_default())
        .filter(|s| s.get("document").is_none())
        .map(|s| s["controller"].as_u64().unwrap())
        .collect();
    let missing: Vec<u64> = sets_a_sound.into_iter().filter(|n| !cited.contains(n)).collect();
    assert!(missing.is_empty(), "controllers no term cites: {missing:?}");
}

#[test]
fn a_role_of_each_setting_on_each_part_it_applies_to_resolves_to_a_known_role_from_the_embedded_state() {
    let mut failures = Vec::new();
    let mut checked = 0;
    let all = terms_1();
    // The parts from a part that opens a path down to `part`, following `stands_under`.
    let chain = |part: &Value| -> Vec<Value> {
        let mut parts = vec![part.clone()];
        loop {
            let t = all.iter().find(|t| t["name"] == *parts.last().unwrap()).unwrap();
            match t.get("stands_under").and_then(Value::as_array).and_then(|u| u.first()) {
                Some(above) if parts.len() < 8 => parts.push(above.clone()),
                _ => break,
            }
        }
        parts.reverse();
        parts
    };
    for t in all.iter().filter(|t| t["type"] == "setting") {
        for part in t["applies_to"].as_array().unwrap() {
            let parts: Vec<Value> = chain(part).into_iter().map(|p| json!({"part": p})).collect();
            let doc = json!({
                "$schema": "https://w3id.org/osd/0.6/device.schema.json",
                "role_vocabulary": "1",
                "device": {"name": "T", "osc_prefix": "/t"},
                "cc_params": {"entries": [{"osc": "/a", "cc": 1, "role": {"parts": parts, "setting": t["name"]}}]}
            });
            match osd::read(&doc.to_string(), Kind::Device) {
                Ok(d) if osd::roles(&d).iter().map(|r| (r.pointer.as_str(), r.meaning)).collect::<Vec<_>>() == vec![("/cc_params/entries/0/role", RoleMeaning::Known)] => {
                    checked += 1
                }
                other => failures.push(format!("{part} {}: {:?}", t["name"], other.map(|d: Descriptor| osd::roles(&d).len()))),
            }
        }
    }
    assert!(checked >= 3, "{checked} roles");
    common::report("roles fail", failures);
}
