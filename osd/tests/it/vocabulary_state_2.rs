//! State 2 of the vocabulary of roles (`schemas/vocabulary/2/`, `spec/vocabulary/2.md`): state 1
//! and what describes drum machines and samplers (the track, the sample with its loop and slices,
//! the drum instruments, their settings) and the choices of the waveform and filter type selectors.

use crate::common;

use osd::{Kind, RoleMeaning};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const STATE_SCHEMA: &str = "https://w3id.org/osd/vocabulary/2/vocabulary.schema.json";

const INSTRUMENTS: [&str; 6] = ["kick", "snare", "clap", "hi-hat", "tom", "cymbal"];
const NEW_PARTS: [&str; 10] = ["track", "sample", "loop", "slice", "kick", "snare", "clap", "hi-hat", "tom", "cymbal"];
const NEW_SETTINGS: [&str; 5] = ["start", "end", "filter-type", "enabled", "effect-type"];
/// The effect and its kinds.
const EFFECTS: [&str; 5] = ["effect", "delay", "reverb", "chorus", "equalizer"];
const WAVEFORMS: [&str; 5] = ["sine", "triangle", "sawtooth", "square", "noise-wave"];
const FILTER_TYPES: [&str; 4] = ["low-pass", "high-pass", "band-pass", "band-reject"];

fn repo_file(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn terms_of(state: &str) -> BTreeMap<String, Value> {
    let doc: Value = serde_json::from_str(&repo_file(&format!("schemas/vocabulary/{state}/terms.json"))).unwrap();
    doc["terms"].as_array().unwrap().iter().map(|t| (t["name"].as_str().unwrap().to_string(), t.clone())).collect()
}

/// The one further setting state 2 adds: how far the own movement of an effect goes. Its name
/// is the one the state gives it.
fn movement_term() -> String {
    let (one, two) = (terms_of("1"), terms_of("2"));
    let found: Vec<&String> = two
        .iter()
        .filter(|(n, t)| t["type"] == "setting" && !one.contains_key(*n) && !NEW_SETTINGS.contains(&n.as_str()))
        .map(|(n, _)| n)
        .collect();
    assert_eq!(found.len(), 1, "state 2 adds one setting for the movement of an effect: {found:?}");
    found[0].clone()
}

/// The segment of that term in a `paramPath`.
fn movement_segment() -> String {
    movement_term().split('-').map(|w| format!("{}{}", w[..1].to_ascii_uppercase(), &w[1..])).collect()
}

fn new_names() -> Vec<String> {
    let listed = NEW_PARTS.iter().chain(&NEW_SETTINGS).chain(&WAVEFORMS).chain(&FILTER_TYPES).map(|n| n.to_string());
    listed.chain(std::iter::once(movement_term())).collect()
}

fn strings(v: &Value) -> BTreeSet<String> {
    v.as_array().map(|a| a.iter().map(|x| x.as_str().unwrap().to_string()).collect()).unwrap_or_default()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| n.to_string()).collect()
}

fn opens_a_path(t: &Value) -> bool {
    t["at_top"].as_bool().unwrap_or(t.get("stands_under").is_none())
}

// The files of the state.

#[test]
fn the_reader_holds_states_1_and_2_each_embedded_from_its_folder() {
    assert_eq!(osd::VOCABULARY_STATES, ["1", "2"]);
    let two = osd::vocabulary_state("2").expect("state 2");
    assert_eq!(two.state, "2");
    let source = repo_file("schemas/vocabulary/2/terms.json");
    assert_eq!(repo_file("osd/vocabulary-state-2.json"), source);
    assert_eq!(Some(two), osd::read_vocabulary(&source).ok().as_ref());
}

#[test]
fn state_2_holds_its_terms_and_its_own_schema_which_accepts_them() {
    let doc: Value = serde_json::from_str(&repo_file("schemas/vocabulary/2/terms.json")).unwrap();
    assert_eq!(doc["$schema"], STATE_SCHEMA);
    assert_eq!(doc["state"], "2");
    let schema: Value = serde_json::from_str(&repo_file("schemas/vocabulary/2/vocabulary.schema.json")).unwrap();
    assert_eq!(schema["$id"], STATE_SCHEMA);
    let validator = jsonschema::options().build(&schema).unwrap();
    let errors: Vec<String> = validator.iter_errors(&doc).map(|e| format!("{}: {e}", e.instance_path())).collect();
    assert_eq!(errors, Vec::<String>::new());
    osd::read_vocabulary(&doc.to_string()).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn state_2_is_not_recorded_in_the_checksums_of_the_published_files() {
    assert!(common::repo().join("schemas/vocabulary/2/terms.json").is_file());
    assert!(!repo_file("schemas/SHA256SUMS").contains("vocabulary/2/"));
}

// Its terms.

#[test]
fn state_2_adds_the_terms_of_drum_machines_samplers_selectors_and_effects_and_no_other() {
    let (one, two) = (terms_of("1"), terms_of("2"));
    let added: BTreeSet<String> = two.keys().filter(|n| !one.contains_key(*n)).cloned().collect();
    assert_eq!(added, new_names().into_iter().collect());
    for (names, kind) in [(&NEW_PARTS[..], "part"), (&NEW_SETTINGS[..], "setting"), (&WAVEFORMS[..], "value"), (&FILTER_TYPES[..], "value")] {
        for n in names {
            assert_eq!(two[*n]["type"], kind, "{n}");
        }
    }
    assert_eq!(two[&movement_term()]["type"], "setting");
    for n in &new_names() {
        assert!(two[n].get("source").is_some() || two[n].get("shared_by").is_some(), "{n}");
    }
}

#[test]
fn state_2_keeps_every_term_of_state_1_and_every_element_it_had() {
    let (one, two) = (terms_of("1"), terms_of("2"));
    let mut failures = Vec::new();
    for (name, old) in &one {
        let Some(new) = two.get(name) else {
            failures.push(format!("{name}: gone"));
            continue;
        };
        for key in ["type", "definition", "source", "replaced_by"] {
            if old.get(key) != new.get(key) {
                failures.push(format!("{name}: {key} changed"));
            }
        }
        for key in ["words", "shared_by", "applies_to", "stands_under"] {
            let kept = new.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
            for element in old.get(key).and_then(Value::as_array).into_iter().flatten() {
                if !kept.contains(element) {
                    failures.push(format!("{name}: {key} loses {element}"));
                }
            }
        }
        if old["type"] == "part" && opens_a_path(old) && !opens_a_path(new) {
            failures.push(format!("{name}: no longer opens a path"));
        }
    }
    common::report("terms of state 1 changed", failures);
}

#[test]
fn each_part_of_state_2_stands_where_the_closed_list_says_and_nowhere_else() {
    let (one, two) = (terms_of("1"), terms_of("2"));
    let with_track: Vec<&str> = std::iter::once("track").chain(INSTRUMENTS).collect();
    let mut failures = Vec::new();
    for (name, t) in two.iter().filter(|(_, t)| t["type"] == "part") {
        let before = one.get(name);
        let mut under = before.map(|t| strings(&t["stands_under"])).unwrap_or_default();
        let mut opens = before.map(opens_a_path).unwrap_or(false);
        let added: Vec<&str> = match name.as_str() {
            "track" => {
                opens = true;
                vec![]
            }
            "sample" => {
                opens = true;
                with_track.clone()
            }
            "loop" | "slice" => vec!["sample"],
            n if INSTRUMENTS.contains(&n) => {
                opens = true;
                vec!["track"]
            }
            "filter" | "amplifier" | "envelope" => with_track.clone(),
            "lfo" => vec!["track"],
            _ => vec![],
        };
        under.extend(set(&added));
        if strings(&t["stands_under"]) != under {
            failures.push(format!("{name}: stands under {:?}, expected {under:?}", strings(&t["stands_under"])));
        }
        if opens_a_path(t) != opens {
            failures.push(format!("{name}: opens a path {}, expected {opens}", opens_a_path(t)));
        }
    }
    common::report("parts stand elsewhere", failures);
}

#[test]
fn each_setting_of_state_2_applies_to_the_parts_of_the_closed_list_and_no_other() {
    let (one, two) = (terms_of("1"), terms_of("2"));
    let tracks: Vec<&str> = ["track", "sample"].into_iter().chain(INSTRUMENTS).collect();
    let movement = movement_term();
    let mut failures = Vec::new();
    for (name, t) in two.iter().filter(|(_, t)| t["type"] == "setting") {
        let mut want = one.get(name).map(|t| strings(&t["applies_to"])).unwrap_or_default();
        let added: Vec<&str> = match name.as_str() {
            "level" | "pan" | "pitch" => tracks.clone(),
            "start" | "end" => vec!["sample", "loop", "slice"],
            "filter-type" => vec!["filter"],
            "enabled" => EFFECTS.into_iter().chain(["oscillator", "track"]).collect(),
            "effect-type" => EFFECTS.to_vec(),
            "depth" => vec!["delay", "equalizer"],
            n if n == movement => EFFECTS.to_vec(),
            _ => vec![],
        };
        want.extend(set(&added));
        if strings(&t["applies_to"]) != want {
            failures.push(format!("{name}: applies to {:?}, expected {want:?}", strings(&t["applies_to"])));
        }
    }
    common::report("settings apply elsewhere", failures);
}

/// The document sources of a term.
fn documents(t: &Value) -> Vec<Value> {
    t["source"].as_array().into_iter().flatten().filter(|s| s.get("document").is_some()).cloned().collect()
}

#[test]
fn the_waveforms_are_values_of_waveform_that_cite_the_sfz_waveforms() {
    let two = terms_of("2");
    for n in WAVEFORMS {
        let t = &two[n];
        assert_eq!(t["type"], "value", "{n}");
        assert_eq!(strings(&t["applies_to"]), set(&["waveform"]), "{n}");
        let sfz = documents(t).iter().any(|s| s["document"].as_str().unwrap().contains("SFZ"));
        assert!(sfz, "{n} cites no SFZ document and table: {}", t["source"]);
    }
    // The SFZ word that names exactly the sawtooth.
    let saw = two["sawtooth"]["words"].as_array().unwrap().iter().any(|w| {
        w["word"].as_str().unwrap().trim_start_matches('*').eq_ignore_ascii_case("saw") && w["origin"].as_str().unwrap().contains("SFZ")
    });
    assert!(saw, "sawtooth has no SFZ word saw: {}", two["sawtooth"]["words"]);
}

#[test]
fn the_filter_types_are_values_of_filter_type_that_cite_sfz_fil_type() {
    let two = terms_of("2");
    for n in FILTER_TYPES {
        let t = &two[n];
        assert_eq!(t["type"], "value", "{n}");
        assert_eq!(strings(&t["applies_to"]), set(&["filter-type"]), "{n}");
        let cited = documents(t).iter().any(|s| s["document"].as_str().unwrap().contains("SFZ") && s.to_string().contains("fil_type"));
        assert!(cited, "{n} cites no SFZ fil_type: {}", t["source"]);
        // A maker's word names exactly the function: not one filter of a given number of poles.
        for w in t["words"].as_array().unwrap() {
            let word = w["word"].as_str().unwrap().to_lowercase();
            assert!(!["_1p", "_2p", "_4p", "_6p"].iter().any(|p| word.contains(p)), "{n}: {word}");
        }
    }
}

#[test]
fn a_drum_instrument_cites_its_row_of_the_general_midi_percussion_key_map() {
    // The keys and names of the General MIDI Level 1 percussion key map, by instrument.
    let rows: [(&str, &[(u64, &str)]); 6] = [
        ("kick", &[(35, "acoustic bass drum"), (36, "bass drum 1")]),
        ("snare", &[(38, "acoustic snare"), (40, "electric snare")]),
        ("clap", &[(39, "hand clap")]),
        ("hi-hat", &[(42, "closed hi hat"), (44, "pedal hi hat"), (46, "open hi hat")]),
        (
            "tom",
            &[(41, "low floor tom"), (43, "high floor tom"), (45, "low tom"), (47, "low mid tom"), (48, "hi mid tom"), (50, "high tom")],
        ),
        (
            "cymbal",
            &[(49, "crash cymbal 1"), (51, "ride cymbal 1"), (52, "chinese cymbal"), (55, "splash cymbal"), (57, "crash cymbal 2"), (59, "ride cymbal 2")],
        ),
    ];
    let two = terms_of("2");
    for (instrument, keys) in rows {
        let t = &two[instrument];
        let cites = documents(t).iter().any(|s| {
            let document = s["document"].as_str().unwrap().to_lowercase();
            let name = s["name"].as_str().unwrap().to_lowercase().replace('-', " ");
            (document.contains("midi") || document.contains("gm"))
                && keys.iter().any(|(key, row)| name == *row && s.get("number").is_none_or(|n| n.as_u64() == Some(*key)))
        });
        assert!(cites, "{instrument} cites no row of the key map: {}", t["source"]);
    }
}

#[test]
fn enabled_cites_the_lv2_designation_and_carries_no_word_and_no_value() {
    let two = terms_of("2");
    let t = two.get("enabled").expect("state 2 has no term enabled");
    assert_eq!(t["type"], "setting");
    let cited = documents(t).iter().any(|s| s["document"].as_str().unwrap().contains("LV2") && s.to_string().contains("lv2:enabled"));
    assert!(cited, "enabled does not cite lv2:enabled of LV2: {}", t["source"]);
    carries_no_word_and_no_value(&two, "enabled");
}

#[test]
fn the_effect_type_cites_the_sfz_opcode_type_and_carries_no_word_and_no_value() {
    let two = terms_of("2");
    let t = two.get("effect-type").expect("state 2 has no term effect-type");
    assert_eq!(t["type"], "setting");
    let cited = documents(t).iter().any(|s| {
        s["document"].as_str().unwrap().contains("SFZ")
            && ["table", "name"].iter().any(|k| s[*k].as_str().is_some_and(|v| v == "type" || v == "Effect type"))
    });
    assert!(cited, "effect-type does not cite the SFZ opcode type: {}", t["source"]);
    carries_no_word_and_no_value(&two, "effect-type");
}

fn carries_no_word_and_no_value(two: &BTreeMap<String, Value>, setting: &str) {
    assert!(two[setting]["words"].as_array().is_some_and(Vec::is_empty), "{setting} has words: {}", two[setting]["words"]);
    let values: Vec<&String> =
        two.iter().filter(|(_, t)| t["type"] == "value" && strings(&t["applies_to"]).contains(setting)).map(|(n, _)| n).collect();
    assert!(values.is_empty(), "{setting} has values: {values:?}");
}

#[test]
fn the_send_and_the_movement_of_an_effect_apply_to_every_effect_and_keep_their_definitions() {
    let two = terms_of("2");
    let name = movement_term();
    assert_eq!(name, "modulation-depth");
    let send: BTreeSet<String> = EFFECTS.into_iter().chain(["vibrato"]).map(str::to_string).collect();
    assert_eq!(strings(&two["depth"]["applies_to"]), send, "depth");
    assert_eq!(strings(&two[&name]["applies_to"]), set(&EFFECTS), "{name}");
    assert_eq!(two["depth"]["definition"], "How much of an effect, or of the vibrato, is applied to the sound.");
    assert_eq!(
        two[&name]["definition"],
        "How far the own movement of an effect goes, such as the sweep a chorus makes, apart from how much of the effect is applied or heard."
    );
}

#[test]
fn the_movement_of_an_effect_cites_the_sfz_opcode_delay_moddepth_then_general_midi_2() {
    let two = terms_of("2");
    let t = &two[&movement_term()];
    let sources = t["source"].as_array().expect("sources");
    assert_eq!(sources.len(), 2, "two public norms: {}", t["source"]);
    assert_eq!(sources[0], json!({"document": "SFZ format (sfzformat.com)", "table": "delay_moddepth", "name": "delay_moddepth"}));
    let gm2 = &sources[1];
    let document = gm2["document"].as_str().unwrap_or_default();
    for said in ["General MIDI 2", "RP-024", "1.2a"] {
        assert!(document.contains(said), "the second source does not name {said}: {gm2}");
    }
    assert!(gm2["table"].as_str().is_some_and(|v| v.contains("4.5.3")), "no section 4.5.3: {gm2}");
    assert!(gm2["name"].as_str().is_some_and(|v| v.contains("Mod Depth")), "no parameter Mod Depth: {gm2}");
}

// Its roles.

/// The role of a path whose segments are terms and numbers: `/Track/2/Sample/Start`.
fn role_of(path: &str) -> Value {
    let kebab = |s: &str| {
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_ascii_uppercase() && i > 0 {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        }
        out
    };
    let mut segments: Vec<&str> = path.split('/').skip(1).collect();
    let setting = kebab(segments.pop().unwrap());
    let mut parts: Vec<Value> = Vec::new();
    for s in segments {
        match s.parse::<u64>() {
            Ok(i) => parts.last_mut().unwrap()["index"] = json!(i),
            Err(_) => parts.push(json!({"part": kebab(s)})),
        }
    }
    json!({"parts": parts, "setting": setting})
}

/// A device of this version whose one control has the role, written with the state.
fn device(version: &str, state: &str, role: &Value) -> String {
    let control = if version == "0.6" {
        json!({"osc": "/a", "cc": 1, "role": role})
    } else {
        json!({"osc": "/a", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}], "role": role})
    };
    json!({
        "$schema": format!("https://w3id.org/osd/{version}/device.schema.json"),
        "role_vocabulary": state,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"entries": [control]}
    })
    .to_string()
}

/// What is wrong with a path that state 2 reads as a known role and reads back.
fn unread(path: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let role = role_of(path);
    for version in ["0.6", "0.7"] {
        match osd::read(&device(version, "2", &role), Kind::Device) {
            Err(f) => failures.push(format!("{path} in {version}: {f}")),
            Ok(d) => {
                let roles = osd::roles(&d);
                if roles.len() != 1 || roles[0].meaning != RoleMeaning::Known || roles[0].param_path != path {
                    failures.push(format!("{path} in {version}: {roles:?}"));
                    continue;
                }
                match osd::role_from_param_path(path, "2") {
                    Ok(back) if back == roles[0].role => {}
                    other => failures.push(format!("{path}: read back as {other:?}")),
                }
            }
        }
    }
    failures
}

const CITED_PATHS: [&str; 13] = [
    "/Track/2/Level",
    "/Track/2/Filter/Cutoff",
    "/Track/2/Sample/Start",
    "/Sample/Start",
    "/Sample/End",
    "/Sample/Loop/Start",
    "/Sample/Loop/End",
    "/Sample/Slice/2/Start",
    "/Kick/Level",
    "/Kick/Filter/Cutoff",
    "/Clap/Pitch",
    "/Tom/Envelope/DecayTime",
    "/Track/1/Snare/Level",
];

#[test]
fn a_device_of_0_6_or_0_7_written_with_state_2_reads_the_roles_of_state_1() {
    let mut failures = Vec::new();
    for path in ["/Filter/Cutoff", "/Oscillator/2/Pitch", "/Amplifier/Envelope/AttackTime"] {
        failures.extend(unread(path));
    }
    common::report("roles fail", failures);
}

#[test]
fn the_roles_of_tracks_samples_and_drum_instruments_read_as_known_roles_with_state_2() {
    let failures: Vec<String> = CITED_PATHS.iter().flat_map(|p| unread(p)).collect();
    common::report("roles fail", failures);
}

#[test]
fn each_drum_instrument_reads_alone_and_on_a_track_with_its_settings_and_parts() {
    let mut failures = Vec::new();
    for instrument in ["Kick", "Snare", "Clap", "HiHat", "Tom", "Cymbal"] {
        for rest in [
            "Level",
            "Pan",
            "Pitch",
            "Filter/Cutoff",
            "Filter/FilterType",
            "Amplifier/Envelope/AttackTime",
            "Envelope/DecayTime",
            "Sample/Start",
            "Sample/Loop/End",
            "Sample/Slice/3/End",
        ] {
            failures.extend(unread(&format!("/{instrument}/{rest}")));
            failures.extend(unread(&format!("/Track/4/{instrument}/{rest}")));
        }
    }
    common::report("roles fail", failures);
}

#[test]
fn a_track_and_a_sample_read_with_their_settings_and_the_filter_type_applies_to_a_filter() {
    let mut failures = Vec::new();
    for path in [
        "/Track/Pan",
        "/Track/Pitch",
        "/Track/1/Amplifier/Envelope/AttackTime",
        "/Track/1/Envelope/AttackTime",
        "/Track/1/Lfo/Rate",
        "/Track/1/Sample/Slice/1/End",
        "/Sample/Level",
        "/Sample/Pan",
        "/Sample/Pitch",
        "/Filter/FilterType",
        "/Filter/Cutoff",
        "/Lfo/Rate",
        "/Oscillator/Waveform",
    ] {
        failures.extend(unread(path));
    }
    common::report("roles fail", failures);
}

#[test]
fn the_roles_of_the_new_parts_are_refused_with_state_1() {
    for path in CITED_PATHS.iter().chain(&["/Filter/FilterType"]) {
        for version in ["0.6", "0.7"] {
            let doc = device(version, "1", &role_of(path));
            let f = osd::read(&doc, Kind::Device).err().unwrap_or_else(|| panic!("{path} accepted with state 1"));
            assert!(f.pointer.starts_with("/cc_params/entries/0/role"), "{path}: {f}");
        }
        assert!(osd::role_from_param_path(path, "1").is_err(), "{path} read with state 1");
    }
}

#[test]
fn a_part_or_a_setting_where_state_2_does_not_place_it_is_refused() {
    let mut failures = Vec::new();
    // The path, and where in the role it is refused.
    for (path, at) in [
        ("/Track/1/Amplifier/Level", ""),
        ("/Kick/Lfo/Rate", "/parts/1"),
        ("/Track/1/Oscillator/Pitch", "/parts/1"),
        ("/Loop/Start", "/parts/0"),
        ("/Slice/Start", "/parts/0"),
        ("/Track/1/Loop/Start", "/parts/1"),
        ("/Kick/Slice/Start", "/parts/1"),
        ("/Sample/Filter/Cutoff", "/parts/1"),
        ("/Sample/Envelope/AttackTime", "/parts/1"),
        ("/Track/1/Delay/DelayTime", "/parts/1"),
        ("/Sample/Track/Level", "/parts/1"),
        ("/Track/Kick/Snare/Level", "/parts/2"),
        ("/Track/Start", ""),
        ("/Kick/End", ""),
        ("/Amplifier/FilterType", ""),
        ("/Track/FilterType", ""),
    ] {
        let want = format!("/cc_params/entries/0/role{at}");
        match osd::read(&device("0.6", "2", &role_of(path)), Kind::Device) {
            Err(f) if f.kind == osd::FaultKind::BadValue && f.pointer == want => {}
            other => failures.push(format!("{path}: {:?}, expected a refusal at {want}", other.err().map(|f| f.to_string()))),
        }
    }
    common::report("roles accepted or refused elsewhere", failures);
}

/// What is wrong with paths that state 2 refuses at the role itself: the setting does not apply
/// to the last part.
fn not_refused_by_state_2(paths: &[String]) -> Vec<String> {
    let mut failures = Vec::new();
    for path in paths {
        for version in ["0.6", "0.7"] {
            match osd::read(&device(version, "2", &role_of(path)), Kind::Device) {
                Err(f) if f.kind == osd::FaultKind::BadValue && f.pointer == "/cc_params/entries/0/role" => {}
                other => failures.push(format!("{path} in {version}: {:?}", other.err().map(|f| f.to_string()))),
            }
        }
    }
    failures
}

/// What is wrong with paths that state 1 refuses.
fn not_refused_by_state_1(paths: &[String]) -> Vec<String> {
    let mut failures = Vec::new();
    for path in paths {
        for version in ["0.6", "0.7"] {
            match osd::read(&device(version, "1", &role_of(path)), Kind::Device) {
                Err(f) if f.pointer.starts_with("/cc_params/entries/0/role") => {}
                other => failures.push(format!("{path} in {version}: {:?}", other.err().map(|f| f.to_string()))),
            }
        }
        if osd::role_from_param_path(path, "1").is_ok() {
            failures.push(format!("{path}: read with state 1"));
        }
    }
    failures
}

fn owned(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|p| p.to_string()).collect()
}

#[test]
fn an_effect_an_oscillator_and_a_track_read_whether_they_are_enabled_with_state_2_only() {
    let paths = owned(&[
        "/Effect/Enabled",
        "/Effect/2/Enabled",
        "/Delay/Enabled",
        "/Reverb/Enabled",
        "/Chorus/Enabled",
        "/Equalizer/Enabled",
        "/Oscillator/Enabled",
        "/Oscillator/1/Enabled",
        "/Track/Enabled",
        "/Track/1/Enabled",
    ]);
    let mut failures: Vec<String> = paths.iter().flat_map(|p| unread(p)).collect();
    failures.extend(not_refused_by_state_1(&paths));
    common::report("roles fail", failures);
}

#[test]
fn enabled_is_refused_on_a_part_the_state_does_not_give_it() {
    let paths = owned(&[
        "/Performance/Enabled",
        "/Sound/Enabled",
        "/Voice/Enabled",
        "/Output/Enabled",
        "/Noise/Enabled",
        "/Filter/Enabled",
        "/Lfo/2/Enabled",
        "/Track/1/Lfo/Enabled",
        "/Vibrato/Enabled",
        "/Arpeggiator/Enabled",
        "/Sequencer/Enabled",
        "/Sample/Enabled",
        "/Kick/Enabled",
    ]);
    common::report("roles not refused", not_refused_by_state_2(&paths));
}

#[test]
fn an_effect_reads_its_type_with_state_2_only() {
    let paths = owned(&[
        "/Effect/EffectType",
        "/Effect/1/EffectType",
        "/Delay/EffectType",
        "/Reverb/EffectType",
        "/Chorus/EffectType",
        "/Equalizer/EffectType",
    ]);
    let mut failures: Vec<String> = paths.iter().flat_map(|p| unread(p)).collect();
    failures.extend(not_refused_by_state_1(&paths));
    common::report("roles fail", failures);
}

#[test]
fn the_effect_type_is_refused_on_a_part_that_is_no_effect() {
    let paths = owned(&["/Filter/EffectType", "/Oscillator/EffectType", "/Track/1/EffectType", "/Vibrato/EffectType"]);
    common::report("roles not refused", not_refused_by_state_2(&paths));
}

#[test]
fn a_chorus_reads_the_movement_of_its_own_beside_its_send_its_mix_and_its_level_with_state_2() {
    let movement = movement_segment();
    let own = vec![format!("/Chorus/{movement}"), format!("/Effect/1/{movement}")];
    let beside = owned(&["/Chorus/Depth", "/Chorus/Mix", "/Chorus/Level"]);
    let mut failures: Vec<String> = own.iter().chain(&beside).flat_map(|p| unread(p)).collect();
    failures.extend(not_refused_by_state_1(&own));
    failures.extend(not_refused_by_state_2(&[format!("/Vibrato/{movement}"), format!("/Filter/{movement}")]));
    common::report("roles fail", failures);
}

#[test]
fn every_effect_reads_its_send_and_the_movement_of_its_own_with_state_2() {
    let mut paths = owned(&["/Effect/1/Depth", "/Effect/1/ModulationDepth", "/Vibrato/Depth"]);
    for effect in ["Effect", "Delay", "Reverb", "Chorus", "Equalizer"] {
        paths.push(format!("/{effect}/Depth"));
        paths.push(format!("/{effect}/ModulationDepth"));
    }
    common::report("roles fail", paths.iter().flat_map(|p| unread(p)).collect());
}

#[test]
fn the_send_and_the_movement_of_an_effect_are_refused_on_a_part_that_is_no_effect() {
    let paths = owned(&["/Vibrato/ModulationDepth", "/Filter/Depth", "/Filter/ModulationDepth"]);
    common::report("roles not refused", not_refused_by_state_2(&paths));
}

#[test]
fn state_1_refuses_the_send_to_a_delay_or_an_equalizer_and_every_movement_of_an_effect() {
    let mut movements = owned(&["/Effect/1/ModulationDepth", "/Vibrato/ModulationDepth"]);
    for effect in ["Effect", "Delay", "Reverb", "Chorus", "Equalizer"] {
        movements.push(format!("/{effect}/ModulationDepth"));
    }
    // State 1 holds no term for the movement: neither a role nor a `paramPath` says it.
    let mut failures = not_refused_by_state_1(&movements);
    // It holds `depth`, `delay` and `equalizer`, so only the role of a device is refused.
    for path in ["/Delay/Depth", "/Equalizer/Depth"] {
        for version in ["0.6", "0.7"] {
            match osd::read(&device(version, "1", &role_of(path)), Kind::Device) {
                Err(f) if f.kind == osd::FaultKind::BadValue && f.pointer == "/cc_params/entries/0/role" => {}
                other => failures.push(format!("{path} in {version}: {:?}", other.err().map(|f| f.to_string()))),
            }
        }
    }
    common::report("roles not refused", failures);
}

#[test]
fn every_role_of_a_setting_on_a_part_of_state_1_reads_in_state_2_with_the_same_param_path() {
    let one = terms_of("1");
    // Every chain of parts of state 1 from a part that opens a path down to `part`.
    fn chains(one: &BTreeMap<String, Value>, part: &str, depth: usize) -> Vec<Vec<String>> {
        let t = &one[part];
        let mut out = Vec::new();
        if opens_a_path(t) {
            out.push(vec![part.to_string()]);
        }
        if depth < 3 {
            for above in strings(&t["stands_under"]) {
                for mut c in chains(one, &above, depth + 1) {
                    c.push(part.to_string());
                    out.push(c);
                }
            }
        }
        out
    }
    let mut failures = Vec::new();
    let mut checked = 0;
    for (name, s) in one.iter().filter(|(_, t)| t["type"] == "setting") {
        for part in strings(&s["applies_to"]) {
            for chain in chains(&one, &part, 0) {
                let parts: Vec<Value> = chain.iter().map(|p| json!({"part": p})).collect();
                let role = json!({"parts": parts, "setting": name});
                let path_with = |state: &str| {
                    osd::read(&device("0.6", state, &role), Kind::Device).map(|d| osd::roles(&d)[0].param_path.clone()).map_err(|f| f.to_string())
                };
                let (a, b) = (path_with("1"), path_with("2"));
                if a.is_err() || a != b {
                    failures.push(format!("{chain:?} {name}: {a:?} in state 1, {b:?} in state 2"));
                }
                checked += 1;
            }
        }
    }
    assert!(checked >= 50, "{checked} roles");
    common::report("roles of state 1 differ", failures);
}

// Its text.

const TEXT: &str = "spec/vocabulary/2.md";

#[test]
fn the_text_of_state_2_names_each_new_term_and_the_type_value() {
    let text = repo_file(TEXT);
    let names = new_names().into_iter().chain(["value".to_string()]);
    let missing: Vec<String> = names.filter(|n| !text.contains(&format!("`{n}`"))).collect();
    assert!(missing.is_empty(), "{TEXT} does not name {missing:?}");
}

#[test]
fn every_role_the_text_of_state_2_writes_reads_against_state_2() {
    // The quotations of the text are lines opening with `>`.
    let text: String = repo_file(TEXT).lines().map(|l| l.trim_start_matches(['>', ' '])).collect::<Vec<_>>().join("\n");
    // The roles as paths: inline code that opens with a segment of a term or of an extension.
    // The one path left aside is the example M2-117-UM gives, which is no role of OSD.
    let paths: BTreeSet<&str> = text
        .split('`')
        .filter(|c| c.starts_with('/') && !c.contains(char::is_whitespace) && c[1..].starts_with(|f: char| f.is_ascii_uppercase() || f == 'x'))
        .filter(|c| *c != "/Oscillator/A/Envelope/Attack")
        .collect();
    for cited in ["/Filter/Cutoff", "/Track/2/Filter/Cutoff"] {
        assert!(paths.contains(cited), "{TEXT} does not write {cited}: {paths:?}");
    }
    let mut roles: Vec<(String, Value)> = Vec::new();
    for path in paths {
        let role = osd::role_from_param_path(path, "2").unwrap_or_else(|f| panic!("{path}: {f}"));
        let parts: Vec<Value> = role.parts.iter().map(|p| match p.index {
            Some(i) => json!({"part": p.part, "index": i}),
            None => json!({"part": p.part}),
        }).collect();
        roles.push((path.to_string(), json!({"parts": parts, "setting": role.setting})));
    }
    // The roles as JSON objects.
    for (_, after) in text.match_indices("\"role\":").map(|(i, m)| (i, &text[i + m.len()..])) {
        let role = serde_json::Deserializer::from_str(after).into_iter::<Value>().next();
        let role = role.and_then(Result::ok).unwrap_or_else(|| panic!("a role that is not JSON: {}", after.lines().next().unwrap_or("")));
        roles.push((role.to_string(), role));
    }
    let failures: Vec<String> = roles
        .iter()
        .filter_map(|(what, role)| osd::read(&device("0.6", "2", role), Kind::Device).err().map(|f| format!("{what}: {f}")))
        .collect();
    common::report("roles of the text fail", failures);
}

#[test]
fn the_text_of_state_2_cites_the_sources_of_enabled_and_of_the_effect_type() {
    let text = repo_file(TEXT);
    assert!(text.contains("lv2:enabled"), "{TEXT} does not cite lv2:enabled");
    let opcode = text.lines().any(|l| ["`effect-type`", "SFZ", "`type`"].iter().all(|said| l.contains(said)));
    assert!(opcode, "{TEXT} has no line that gives `effect-type` the SFZ opcode `type`");
}

#[test]
fn the_text_of_state_2_tells_the_send_to_an_effect_from_its_level_and_from_its_own_movement() {
    let text = repo_file(TEXT);
    let movement = movement_term();
    for said in ["`depth`", "`mix`", "`level`", "`/Reverb/Depth`", "`/Reverb/Level`", &format!("`{movement}`")] {
        assert!(text.contains(said), "{TEXT} does not write {said}");
    }
}

/// The text from the heading that opens with `heading` to the next heading of any level.
fn section(text: &str, heading: &str) -> String {
    let mut lines = text.lines().skip_while(|l| !l.starts_with(heading));
    let first = lines.next().unwrap_or_else(|| panic!("{TEXT} has no heading {heading}"));
    std::iter::once(first).chain(lines.take_while(|l| !l.starts_with('#'))).collect::<Vec<_>>().join("\n")
}

#[test]
fn the_table_of_what_settings_apply_to_adds_the_send_to_delay_and_equalizer_and_the_movement_to_every_effect() {
    let table = section(&repo_file(TEXT), "## 6 ");
    let added = |setting: &str| -> BTreeSet<String> {
        table
            .lines()
            .filter_map(|l| {
                let cells: Vec<&str> = l.strip_prefix('|')?.split('|').collect();
                cells.first()?.contains(&format!("`{setting}`")).then(|| cells.get(1).copied().unwrap_or(""))
            })
            .flat_map(|cell| cell.split('`').skip(1).step_by(2).map(str::to_string))
            .collect()
    };
    assert_eq!(added("depth"), set(&["delay", "equalizer"]), "depth in the table of section 6");
    assert_eq!(added(&movement_term()), set(&EFFECTS), "modulation-depth in the table of section 6");
}

#[test]
fn the_text_of_state_2_cites_sfz_and_general_midi_2_for_the_movement_of_an_effect_and_no_longer_the_novation_peak() {
    let text = repo_file(TEXT);
    let row = section(&text, "### 4.2 ");
    let row = row.lines().find(|l| l.starts_with("| `modulation-depth`")).expect("the row of modulation-depth in 4.2");
    for said in ["SFZ", "delay_moddepth", "General MIDI 2"] {
        assert!(row.contains(said), "the row of 4.2 does not cite {said}: {row}");
    }
    let note = section(&text, "### 6.1 ");
    for said in ["delay_moddepth", "General MIDI 2"] {
        assert!(note.contains(said), "section 6.1 does not cite {said}");
    }
    let sources = section(&text, "## 9 ");
    for said in ["delay_moddepth", "sfzformat.com/opcodes/delay_moddepth", "General MIDI 2", "RP-024", "4.5.3"] {
        assert!(sources.contains(said), "section 9 does not cite {said}");
    }
    for gone in ["Novation Peak", "userguides.novationmusic.com"] {
        assert!(!text.contains(gone), "{TEXT} still cites {gone}");
    }
}

#[test]
fn the_note_of_limits_adds_the_speed_of_an_effect_s_movement_and_the_time_of_a_reverb_without_counting_the_limits() {
    let part = section(&repo_file(TEXT), "### 6.1 ");
    let notes: Vec<&str> = part.lines().filter(|l| l.starts_with('>')).collect();
    let notes = notes.join("\n");
    for said in ["`/Reverb/Depth`", "track", "mute", "`rate`", "`decay-time`", "`envelope`"] {
        assert!(notes.contains(said), "the notes of 6.1 do not say {said}");
    }
    let lower = notes.to_lowercase();
    for count in ["two", "three", "four", "five", "2", "3", "4"] {
        assert!(!lower.contains(&format!("{count} limits")), "the notes of 6.1 count the limits: {count}");
    }
}

#[test]
fn the_text_of_state_2_says_the_modulation_matrix_and_destination_are_the_route_and_destination_of_draft_0021() {
    let text = repo_file(TEXT);
    for said in ["0021", "`route`", "`destination`"] {
        assert!(text.contains(said), "{TEXT} does not write {said}");
    }
}

#[test]
fn draft_0021_writes_depth_for_a_send_alone_and_the_movement_of_a_chorus_under_its_own_term() {
    const DRAFT: &str = "spec/proposals/0021-a-source-or-destination-of-modulation-carries-its-meaning.md";
    let draft = repo_file(DRAFT);
    let movement = movement_segment();
    let row = draft.lines().find(|l| l.starts_with("| how much goes to the chorus")).unwrap_or_else(|| panic!("{DRAFT}: no row of the chorus"));
    assert!(!row.contains("send or a depth"), "`/Chorus/Depth` still names two functions: {row}");
    assert!(row.contains("`/Chorus/Depth`"), "the send is not written `/Chorus/Depth`: {row}");
    assert!(draft.contains(&format!("`/Chorus/{movement}`")), "{DRAFT} does not write `/Chorus/{movement}`");
}
