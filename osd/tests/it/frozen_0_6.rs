//! What 0.6 freezes is exact and proven: every field it keeps has a written meaning and a
//! conformance case that shows it; what the norm names takes the norm's word; the forms that tied
//! the format to a tool or left it inconsistent are open or coherent, each refusal with a case.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

fn read(doc: &Value, kind: Kind) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), kind)
}

fn refused_at(doc: &Value, kind: Kind, fault: FaultKind, pointer: &str) {
    let f = read(doc, kind).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (fault, pointer), "{f}");
}

fn schema_errors(doc: &Value, kind: Kind) -> Vec<(String, String)> {
    common::Schemas::load("0.6").errors(kind, doc)
}

fn text(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn json_file(path: &str) -> Value {
    serde_json::from_str(&text(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

// ---- The hex text has one case, upper case, imposed by the schema and the reader.

const SOUND: &str = "https://w3id.org/osd/0.6/sound.schema.json";

fn sound(content: Value) -> Value {
    json!({"$schema": SOUND, "name": "S", "kind": "sound", "device": {"descriptor": "acme/synth", "vendor": "Acme", "model": "Synth"}, "content": content})
}

#[test]
fn a_hex_text_in_upper_case_is_read() {
    let doc = sound(json!({"sysex": "F0 7D 01 F7"}));
    assert!(read(&doc, Kind::Sound).is_ok());
    assert!(schema_errors(&doc, Kind::Sound).is_empty(), "{:?}", schema_errors(&doc, Kind::Sound));
}

#[test]
fn a_hex_text_with_a_lower_case_digit_is_refused_by_the_reader_and_the_schema() {
    for bad in ["f0 7d 01 f7", "F0 7d 01 F7", "F0 7D 0a F7"] {
        let doc = sound(json!({"sysex": bad}));
        refused_at(&doc, Kind::Sound, FaultKind::BadValue, "/content/sysex");
        assert!(!schema_errors(&doc, Kind::Sound).is_empty(), "{bad}");
    }
}

#[test]
fn a_fingerprint_hex_text_in_lower_case_is_refused_at_its_field() {
    let doc = json!({"$schema": "https://w3id.org/osd/0.6/fingerprint.schema.json", "observed": "2026-11-21", "device": {"descriptor": "korg/minilogue", "firmware": "2.01"},
        "identity_reply": "f0 7e 00 06 02 42 f7"});
    refused_at(&doc, Kind::Fingerprint, FaultKind::BadValue, "/identity_reply");
    assert!(!schema_errors(&doc, Kind::Fingerprint).is_empty());
}

// ---- A sound's content holds at least one of sysex, midi, settings.

#[test]
fn a_sound_content_with_none_of_sysex_midi_settings_is_refused_at_the_content_by_both() {
    let doc = sound(json!({}));
    refused_at(&doc, Kind::Sound, FaultKind::BadValue, "/content");
    let errors = schema_errors(&doc, Kind::Sound);
    assert_eq!(errors.first().map(|e| e.0.as_str()), Some("/content"), "{errors:?}");
}

#[test]
fn a_sound_content_with_one_of_them_is_read() {
    for content in [json!({"sysex": "F0 F7"}), json!({"midi": {"path": "a.mid"}}), json!({"settings": {"/amp/attack": [90]}})] {
        let doc = sound(content.clone());
        assert!(read(&doc, Kind::Sound).is_ok(), "{content}");
        assert!(schema_errors(&doc, Kind::Sound).is_empty(), "{content}");
    }
}

// ---- A control of a fingerprint knows the words of a device's control, and never mixes them.

const FINGERPRINT: &str = "https://w3id.org/osd/0.6/fingerprint.schema.json";

fn fingerprint(controls: Value) -> Value {
    json!({"$schema": FINGERPRINT, "observed": "2026-11-21", "device": {"descriptor": "korg/minilogue", "firmware": "2.01"}, "controls": controls})
}

#[test]
fn a_fingerprint_control_is_a_cc_a_cc_with_its_lsb_an_nrpn_or_an_rpn_and_reads_back_as_written() {
    let controls = json!([
        {"cc": 74},
        {"cc": 1, "cc_lsb": 33, "channel": 2},
        {"nrpn_msb": 1, "nrpn_lsb": 2},
        {"rpn_msb": 0, "rpn_lsb": 1}
    ]);
    let doc = fingerprint(controls.clone());
    let got = read(&doc, Kind::Fingerprint).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&got)).unwrap()["controls"], controls);
    assert!(schema_errors(&doc, Kind::Fingerprint).is_empty(), "{:?}", schema_errors(&doc, Kind::Fingerprint));
}

#[test]
fn a_fingerprint_control_mixing_two_kinds_of_message_is_refused_at_the_control() {
    for control in [
        json!({"cc": 74, "nrpn_msb": 1, "nrpn_lsb": 2}),
        json!({"cc": 74, "rpn_msb": 0}),
        json!({"cc_lsb": 33, "nrpn_lsb": 2}),
        json!({"nrpn_msb": 1, "rpn_msb": 0}),
    ] {
        let doc = fingerprint(json!([{"cc": 1}, control.clone()]));
        refused_at(&doc, Kind::Fingerprint, FaultKind::BadValue, "/controls/1");
        assert!(!schema_errors(&doc, Kind::Fingerprint).is_empty(), "{control}");
    }
}

#[test]
fn the_control_a_person_named_in_a_fingerprint_takes_the_same_words() {
    let doc = json!({"$schema": FINGERPRINT, "observed": "2026-11-21", "device": {"descriptor": "korg/minilogue", "firmware": "2.01"},
        "names": [{"control": {"rpn_msb": 0, "rpn_lsb": 2}, "name": "coarse tune"}, {"control": {"cc": 1, "cc_lsb": 33}, "name": "wheel"}]});
    assert!(read(&doc, Kind::Fingerprint).is_ok());
    let mixed = json!({"$schema": FINGERPRINT, "observed": "2026-11-21", "device": {"descriptor": "korg/minilogue", "firmware": "2.01"},
        "names": [{"control": {"cc": 1, "nrpn_msb": 3}, "name": "x"}]});
    refused_at(&mixed, Kind::Fingerprint, FaultKind::BadValue, "/names/0/control");
}

// ---- The flat fields of a control of a device never mix a CC and an NRPN.

const DEVICE: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn control(entry: Value) -> Value {
    json!({"$schema": DEVICE, "device": {"name": "S", "osc_prefix": "/s"}, "cc_params": {"channel": 0, "entries": [entry]}})
}

#[test]
fn the_flat_fields_of_a_control_mixing_a_cc_and_an_nrpn_or_an_rpn_are_refused_at_the_control() {
    for entry in [
        json!({"osc": "/a", "cc": 74, "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14}),
        json!({"osc": "/a", "cc": 74, "rpn_msb": 0, "value_bits": 7}),
        json!({"osc": "/a", "cc_lsb": 33, "nrpn_msb": 1, "value_bits": 7}),
        json!({"osc": "/a", "nrpn_msb": 1, "rpn_msb": 0, "value_bits": 7}),
    ] {
        let doc = control(entry.clone());
        refused_at(&doc, Kind::Device, FaultKind::BadValue, "/cc_params/entries/0");
        assert!(!schema_errors(&doc, Kind::Device).is_empty(), "{entry}");
    }
}

#[test]
fn a_control_reached_by_a_cc_and_by_an_nrpn_says_the_second_way_in_also() {
    let doc = control(json!({"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14}]}));
    assert!(read(&doc, Kind::Device).is_ok());
    assert!(schema_errors(&doc, Kind::Device).is_empty());
}

// ---- The place of each control in a sound has its own word, `places`; `settings` stays the
// ---- values of a sound by address of control.

fn transfer(extra: Value) -> Value {
    let mut frame = vec![json!(240), json!(66)];
    frame.extend((0..4).map(|_| json!("any")));
    frame.push(json!(247));
    let mut t = json!({"id": "single", "sound_id": "program", "carries": "sound", "match_frame": frame,
        "elements": {"data": {"from": 2, "length": 4}}});
    for (k, v) in extra.as_object().unwrap() {
        t[k] = v.clone();
    }
    json!({"$schema": DEVICE, "device": {"name": "S", "osc_prefix": "/s"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}}]},
        "role_vocabulary": "1", "sound_transfers": [t]})
}

#[test]
fn the_place_of_a_control_in_a_sound_is_places_and_settings_is_no_field_of_a_transfer() {
    let place = json!([{"control": "/cc_params/entries/0", "from": 0, "length": 1}]);
    let ok = transfer(json!({"places": place}));
    assert!(read(&ok, Kind::Device).is_ok(), "{:?}", read(&ok, Kind::Device).err());
    assert!(schema_errors(&ok, Kind::Device).is_empty(), "{:?}", schema_errors(&ok, Kind::Device));
    let old = transfer(json!({"settings": place}));
    refused_at(&old, Kind::Device, FaultKind::UnknownField, "/sound_transfers/0/settings");
    assert!(!schema_errors(&old, Kind::Device).is_empty());
}

#[test]
fn the_values_of_a_sound_by_address_of_control_stay_settings() {
    let doc = sound(json!({"settings": {"/amp/attack": [90]}}));
    assert!(read(&doc, Kind::Sound).is_ok());
}

// ---- The schema of 0.6 says what it freezes.

#[test]
fn the_schema_of_0_6_names_the_coding_of_the_data_and_of_the_step_with_the_same_word() {
    let schema = json_file("schemas/0.6/device.schema.json");
    let data = &schema["$defs"]["sound_data_element"]["properties"];
    assert!(data.is_object(), "sound_data_element has properties");
    let text = data.to_string();
    assert!(text.contains("\"coding\""), "{text}");
    assert!(!text.contains("\"packing\""), "{text}");
    let flat = schema.to_string();
    assert!(!flat.contains("\"packing\""), "no packing word is left in the schema");
    assert!(flat.contains("\"mcoded7\"") && flat.contains("\"msb_first_7in8\""));
    assert!(flat.contains("\"lsb_8to7\""), "identity.unpack keeps its 0.5 spelling");
}

// ---- The cases that show each field, each refusal and the coding of the bytes.

enum Outcome {
    /// The descriptor reads: each pointer exists in it (with that value, when given) and each
    /// fragment (the text without its white space) appears in it.
    Reads(&'static [(&'static str, Option<&'static str>)], &'static [&'static str]),
    Refused(&'static str, &'static str),
}
use Outcome::{Reads, Refused};

const CASES: &[(&str, &str, Outcome)] = &[
    // The fifteen fields of 0.5 that had no case.
    ("fingerprint-names-and-sends", "fingerprint", Reads(&[("/names/0/control", None), ("/names/0/name", None), ("/sends", None)], &[])),
    ("fingerprint-controls-cc-lsb-and-rpn", "fingerprint", Reads(&[("/controls/0/cc_lsb", None), ("/controls/1/rpn_msb", None), ("/controls/1/rpn_lsb", None), ("/controls/2/nrpn_msb", None)], &[])),
    ("identity-unpack-lsb-8to7", "device", Reads(&[("/identity/unpack", Some("lsb_8to7"))], &[])),
    ("control-orientation", "device", Reads(&[("/cc_params/entries/0/orientation", None)], &[])),
    ("loads-sounds-of", "device", Reads(&[("/loads_sounds_of/0", None)], &[])),
    ("trigger-channel-mode", "device", Reads(&[("/triggers/entries/0/channel_mode", None)], &[])),
    ("bank-first-pc-and-first-number", "device", Reads(&[("/programs/banks/0/first_pc", None), ("/programs/banks/0/first_number", None)], &[])),
    ("midi-out-note-offset", "device", Reads(&[("/midi_out/note_offset", None)], &[])),
    ("rate-limit-hz", "device", Reads(&[("/device/rate_limit_hz", None), ("/device/transport/rate_limit_hz", None)], &[])),
    ("custom-command-wrap-sysex-and-profile", "device", Reads(&[("/custom_commands/0/wrap_sysex", None), ("/custom_commands/0/profile", None)], &[])),
    ("sound-license-terms-and-derived-from", "sound", Reads(&[("/license_terms", None), ("/derived_from/url", None), ("/derived_from/name", None)], &[])),
    // The words with no written definition, each shown by a case.
    ("param-entry-words", "device", Reads(&[("/params/entries/0/pr", None), ("/params/entries/0/p", None), ("/params/entries/0/c", None), ("/params/entries/0/r", None), ("/params/entries/0/id", None)], &[])),
    ("dump-kind-edit-buffer", "device", Reads(&[("/programs/dump/kind", Some("edit_buffer"))], &[])),
    ("dump-kind-manual", "device", Reads(&[("/programs/dump/kind", Some("manual"))], &[])),
    ("dump-kind-none", "device", Reads(&[("/programs/dump/kind", Some("none"))], &[])),
    ("resource-file-size-and-offset-without-as", "device", Reads(&[], &["{\"file\":\"size\"}", "{\"file\":\"offset\"}"])),
    // The words of the norm beside OSD's.
    ("midi-in-every-word", "device", Reads(&[], &["\"aftertouch\"", "\"poly_aftertouch\"", "\"pitchbend\""])),
    ("file-write-coding-mcoded7", "device", Reads(&[], &["\"coding\":\"mcoded7\""])),
    // Each refusal of the coherent forms.
    ("hex-text-lowercase-refused", "sound", Refused("BadValue", "/content/sysex")),
    ("sound-content-empty-refused", "sound", Refused("BadValue", "/content")),
    ("fingerprint-control-mixing-cc-and-nrpn-refused", "fingerprint", Refused("BadValue", "/controls/0")),
    ("control-flat-fields-mixing-cc-and-nrpn-refused", "device", Refused("BadValue", "/cc_params/entries/0")),
    ("sound-transfer-coding-outside-list-refused", "device", Refused("BadValue", "/sound_transfers/0/elements/data/coding")),
];

fn pointer_value<'a>(doc: &'a Value, pointer: &str) -> Option<&'a Value> {
    doc.pointer(pointer)
}

#[test]
fn each_field_and_each_refusal_of_0_6_has_a_conformance_case_that_shows_it() {
    let mut failures = Vec::new();
    for (name, kind, outcome) in CASES {
        let dir = format!("conformance/0.6/read/{name}");
        let (case_path, descriptor_path) = (format!("{dir}/case.json"), format!("{dir}/descriptor.json"));
        let root = common::repo();
        if !root.join(&case_path).is_file() || !root.join(&descriptor_path).is_file() {
            failures.push(format!("{name}: the case is missing"));
            continue;
        }
        let case = json_file(&case_path);
        let descriptor = json_file(&descriptor_path);
        if case["read"] != json!(kind) {
            failures.push(format!("{name}: reads {}, expected {kind}", case["read"]));
        }
        match outcome {
            Reads(pointers, fragments) => {
                if case.get("result").is_none() || case.get("fault").is_some() {
                    failures.push(format!("{name}: states a result"));
                }
                for (pointer, want) in *pointers {
                    match (pointer_value(&descriptor, pointer), want) {
                        (None, _) => failures.push(format!("{name}: the descriptor shows nothing at {pointer}")),
                        (Some(v), Some(w)) if v != &json!(w) => failures.push(format!("{name}: {pointer} is {v}, expected {w}")),
                        _ => {}
                    }
                }
                let flat: String = text(&descriptor_path).split_whitespace().collect();
                for fragment in *fragments {
                    if !flat.contains(fragment) {
                        failures.push(format!("{name}: the descriptor lacks {fragment}"));
                    }
                }
            }
            Refused(fault, pointer) => {
                if case["fault"] != json!({"kind": fault, "pointer": pointer}) {
                    failures.push(format!("{name}: states {}", case["fault"]));
                }
            }
        }
    }
    common::report("0.6 cases missing or not showing their field", failures);
}

#[test]
fn the_fields_of_0_5_without_a_case_are_each_shown_by_a_case_that_reads() {
    let fields = [
        "maker_replies", "identity_reply", "names", "sends", "unpack", "orientation", "loads_sounds_of", "channel_mode",
        "first_pc", "first_number", "note_offset", "rate_limit_hz", "wrap_sysex", "license_terms", "derived_from",
    ];
    fn keys(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                for (k, c) in m {
                    out.push(k.clone());
                    keys(c, out);
                }
            }
            Value::Array(a) => a.iter().for_each(|c| keys(c, out)),
            _ => {}
        }
    }
    let mut seen = Vec::new();
    let root = common::repo().join("conformance/0.6/read");
    for case in std::fs::read_dir(&root).unwrap() {
        let case = case.unwrap().path();
        let c: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        if let Some(result) = c.get("result") {
            keys(result, &mut seen);
        }
    }
    let missing: Vec<&str> = fields.iter().copied().filter(|f| !seen.iter().any(|k| k == f)).collect();
    assert!(missing.is_empty(), "no result case of 0.6 shows {missing:?}");
}

#[test]
fn mcoded7_is_shown_by_a_case_that_reads_a_step_coded_with_it() {
    let mut found = false;
    let root = common::repo().join("conformance/0.6/read");
    for case in std::fs::read_dir(&root).unwrap() {
        let case = case.unwrap().path();
        let c: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        found |= c.get("result").is_some_and(|r| r.to_string().contains("\"coding\":\"mcoded7\""));
    }
    assert!(found, "no result case of 0.6 reads a step with the coding mcoded7");
}

#[test]
fn no_case_of_0_6_still_writes_the_old_names_of_the_place_of_a_control_or_of_the_coding() {
    fn old(v: &Value, here: &str, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                if m.contains_key("sound_transfers") {
                    for (i, t) in m["sound_transfers"].as_array().into_iter().flatten().enumerate() {
                        if t.get("settings").is_some() {
                            out.push(format!("{here}/sound_transfers/{i}/settings"));
                        }
                        if t.pointer("/elements/data/packing").is_some() {
                            out.push(format!("{here}/sound_transfers/{i}/elements/data/packing"));
                        }
                    }
                }
                m.iter().for_each(|(k, c)| old(c, &format!("{here}/{k}"), out));
            }
            Value::Array(a) => a.iter().enumerate().for_each(|(i, c)| old(c, &format!("{here}/{i}"), out)),
            _ => {}
        }
    }
    let mut left = Vec::new();
    let mut files = vec![common::repo().join("conformance/0.6")];
    while let Some(p) = files.pop() {
        if p.is_dir() {
            files.extend(std::fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
        } else if p.extension().is_some_and(|e| e == "json") {
            let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            old(&v, &p.display().to_string(), &mut left);
        }
    }
    assert!(left.is_empty(), "old names left: {left:?}");
    assert!(!common::repo().join("conformance/0.6/read/sound-transfer-packing-outside-list-refused").exists());
}

/// The coding cases (`conformance/0.6/coding/<case>/case.json`): `coding`, `bytes` and `coded`, each in
/// upper-case hexadecimal, `_about` citing the norm's example.
const CODING_CASES: &[(&str, &str)] = &[
    ("mcoded7-full-group", "mcoded7"),
    ("mcoded7-short-group", "mcoded7"),
    ("mcoded7-full-group-and-short-group", "mcoded7"),
    ("mcoded7-empty", "mcoded7"),
    ("msb-first-7in8-full-group", "msb_first_7in8"),
    ("msb-first-7in8-short-group", "msb_first_7in8"),
];

#[test]
fn the_codings_of_the_bytes_have_a_case_built_on_the_examples_of_the_norm() {
    let mut failures = Vec::new();
    for (name, coding) in CODING_CASES {
        let path = format!("conformance/0.6/coding/{name}/case.json");
        if !common::repo().join(&path).is_file() {
            failures.push(format!("{name}: the case is missing"));
            continue;
        }
        let c = json_file(&path);
        if c["coding"] != json!(coding) {
            failures.push(format!("{name}: coding {}", c["coding"]));
        }
        for field in ["bytes", "coded"] {
            let ok = c[field].as_str().is_some_and(|s| s.split(' ').all(|b| b.is_empty() || (b.len() == 2 && b.chars().all(|c| matches!(c, '0'..='9' | 'A'..='F')))));
            if !ok {
                failures.push(format!("{name}: {field} is no upper-case hexadecimal text: {}", c[field]));
            }
        }
        if *coding == "mcoded7" && !c["_about"].as_str().unwrap_or("").contains("M2-103-UM") {
            failures.push(format!("{name}: _about does not cite M2-103-UM"));
        }
    }
    common::report("coding cases missing or malformed", failures);
}

// ---- The documents say it.

/// The line of a table of `spec/0.6.md` that begins with the word between backticks.
fn row(doc: &str, word: &str) -> Option<String> {
    let start = format!("| `{word}` |");
    doc.lines().find(|l| l.starts_with(&start)).map(str::to_string)
}

#[test]
fn every_word_that_had_no_definition_has_a_row_that_defines_it_in_the_interface() {
    let doc = text("spec/0.6.md");
    let words = [
        "param_entry.pr", "param_entry.p", "param_entry.c", "param_entry.r", "param_entry.id", "dump.kind", "channel_mode",
        "orientation", "custom_commands.profile", "file.size", "file.offset", "capabilities.lua",
    ];
    let mut failures = Vec::new();
    for w in words {
        match row(&doc, w) {
            None => failures.push(format!("no row `| `{w}` |` in spec/0.6.md")),
            Some(r) if r.len() < 60 => failures.push(format!("the row of {w} defines nothing: {r}")),
            _ => {}
        }
    }
    common::report("words without a written definition", failures);
}

#[test]
fn the_definitions_say_what_the_lua_capability_the_file_size_and_the_dump_kinds_mean() {
    let doc = text("spec/0.6.md");
    let lua = row(&doc, "capabilities.lua").unwrap_or_default();
    for needle in ["version", "inputs", "return"] {
        assert!(lua.contains(needle), "the Lua capability row lacks {needle}: {lua}");
    }
    let file = format!("{}{}", row(&doc, "file.size").unwrap_or_default(), row(&doc, "file.offset").unwrap_or_default());
    assert!(file.contains("as"), "the rows of file.size and file.offset say what they give without `as`: {file}");
    let dump = row(&doc, "dump.kind").unwrap_or_default();
    for kind in ["none", "request", "edit_buffer", "manual"] {
        assert!(dump.contains(kind), "the dump.kind row lacks {kind}: {dump}");
    }
}

#[test]
fn the_interface_cites_the_words_of_the_norm_beside_the_words_osd_keeps() {
    let doc = text("spec/0.6.md");
    for needle in [
        "Channel Pressure", "Polyphonic Key Pressure", "Pitch Bend Change", "`aftertouch`", "`poly_aftertouch`", "`pitchbend`",
        "Mcoded7", "M2-103-UM", "6.1.7", "`lsb_8to7`", "`coding`", "`places`",
    ] {
        assert!(doc.contains(needle), "spec/0.6.md does not cite {needle}");
    }
}

#[test]
fn the_interface_says_midi_in_is_what_the_device_sends_seen_from_the_host() {
    let doc = text("spec/0.6.md");
    assert!(doc.contains("Channel Pressure"), "the norm's words of the three types are not given");
    let line = doc.lines().find(|l| l.contains("`midi_in`") && l.contains("Its point of view")).unwrap_or("");
    assert!(line.contains("from the host") && line.contains("sends"), "midi_in lacks its point of view: {line}");
}

#[test]
fn the_interface_says_the_hex_text_is_upper_case_and_a_content_holds_one_of_three() {
    let doc = text("spec/0.6.md");
    assert!(doc.contains("upper-case hexadecimal"), "the case of the hex text is not written");
    assert!(doc.contains("at least one of `sysex`, `midi`, `settings`"), "the content rule is not written");
}

#[test]
fn the_interface_names_no_closed_list_of_exports_and_no_prefix_of_resources() {
    let doc = text("spec/0.6.md");
    assert!(!doc.contains("a closed list, the names of the formats"));
    assert!(!doc.contains("(a name of its own starts with `X-`)"));
    for file in ["osd/docs/INTERFACE.md", "osd-formats/docs/INTERFACE.md"] {
        assert!(!text(file).contains("RESOURCE_EXPORTS"), "{file} still names the closed list");
    }
}

#[test]
fn the_readme_of_0_6_says_what_was_taken_out_and_what_is_left_for_a_later_version() {
    let spec = text("spec/0.6.md");
    let start = spec.find("What 0.6 changes").expect("the section of 0.6");
    let section = &spec[start..];
    let section = &section[..section.find("\n## ").unwrap_or(section.len())];
    for needle in ["Taken out of 0.6", "Left for a later version"] {
        assert!(section.contains(needle), "the section of 0.6 lacks {needle:?}");
    }
    let lower = section.to_lowercase();
    for item in ["mpe", "groups", "multitimbral", "per-note", "get/set", "checksum", "variable length"] {
        assert!(lower.contains(item), "the section of 0.6 does not leave {item} for a later version");
    }
}
