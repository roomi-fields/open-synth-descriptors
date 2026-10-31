//! Closed vocabularies (0.5). Each closed list is a set of defined terms
//! (`identity.firmware.decode`, `expects`, `programs.selects` and
//! `banks[].selects`, `_sources[].type`); a value outside a list is valid only
//! as an extension `x-<registrant>-<term>` (`^x-[a-z0-9]+-[a-z0-9-]+$`), except
//! `decode`, which has none. `_sources[].license` is an SPDX id,
//! `public-domain` or `none`. `osd::convert` maps the makers' words of 0.4 to
//! the terms and refuses any other word as `NotConvertible`, at the pointer of
//! the value. The README writes the tables once.

mod common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const SCHEMA: &str = "https://w3id.org/osd/0.5/device.schema.json";

const DECODE: [&str; 6] = ["raw_bytes", "json", "korg_minor_major", "yamaha_tenths", "ascii", "lua"];
const EXPECTS: [&str; 6] = ["continuous", "switch", "momentary", "trigger", "clock", "discrete"];
const KINDS: [&str; 9] = ["sound", "kit", "sequence", "project", "bank", "clip", "performance", "scene", "setlist"];
const SOURCE_TYPES: [&str; 14] = [
    "hardware-verified", "hardware-observed", "software-verified", "vendor-doc", "vendor-osc-api",
    "vendor-self-described", "third-party-osc", "community-re", "electra-preset", "pencilresearch",
    "knobkraft", "midnam", "contributed", "test-fixture",
];
const CATALOGUE_EXTENSIONS: [&str; 6] = ["sampler element", "track", "canvas", "session", "synth slot", "drum slot"];

/// Where 0.4 / 0.5 hold a value of each vocabulary.
#[derive(Clone, Copy, Debug)]
enum Place {
    Decode,
    ProgramsSelects,
    BankSelects,
    CcExpects,
    ParamExpects,
    SourceType,
}

impl Place {
    fn pointer(self) -> &'static str {
        match self {
            Place::Decode => "/identity/firmware/decode",
            Place::ProgramsSelects => "/programs/selects",
            Place::BankSelects => "/programs/banks/0/selects",
            Place::CcExpects => "/cc_params/entries/0/expects",
            Place::ParamExpects => "/params/entries/0/expects",
            Place::SourceType => "/_sources/0/type",
        }
    }

    /// A device carrying `value` at this place, 0.5 when `v05`.
    fn device(self, value: Value, v05: bool) -> Value {
        let mut doc = serde_json::Map::new();
        if v05 {
            doc.insert("$schema".into(), json!(SCHEMA));
        }
        doc.insert("device".into(), json!({"name": "T", "osc_prefix": "/t"}));
        let (key, part) = match self {
            Place::Decode => {
                let mut rule = json!({"decode": value, "at": [5, 6]});
                match value.as_str() {
                    Some("json") => rule["field"] = json!("version"),
                    Some("lua") => rule["script"] = json!("return 1"),
                    _ => {}
                }
                ("identity", json!({"firmware": rule}))
            }
            Place::ProgramsSelects => ("programs", json!({"selects": value})),
            Place::BankSelects => ("programs", json!({"banks": [{"name": "A", "size": 8, "selects": value}]})),
            Place::CcExpects => ("cc_params", json!({"channel": 0, "entries": [{"osc": "/a", "cc": 1, "expects": value}]})),
            Place::ParamExpects => (
                "params",
                json!({"value_type": "u7", "get_frame": [240, 247], "set_frame": [240, 247], "entries": [{"osc": "/a", "expects": value}]}),
            ),
            Place::SourceType => ("_sources", json!([{"type": value}])),
        };
        doc.insert(key.into(), part);
        Value::Object(doc)
    }
}

fn convert(doc: &Value) -> Result<Value, osd::Fault> {
    osd::convert(&doc.to_string(), Kind::Device).map(|t| serde_json::from_str(&t).unwrap())
}

/// The value the converter leaves at `place` for the 0.4 `value`.
#[track_caller]
fn converted(place: Place, value: &str) -> Value {
    let out = convert(&place.device(json!(value), false)).unwrap_or_else(|f| panic!("{value:?} at {place:?}: {f}"));
    out.pointer(place.pointer()).unwrap_or_else(|| panic!("{out}")).clone()
}

/// A 0.4 value the converter refuses, at the pointer of the value.
#[track_caller]
fn not_convertible(place: Place, value: &str) {
    let f = convert(&place.device(json!(value), false)).expect_err(&format!("{value:?} at {place:?} converted"));
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, place.pointer()), "{value:?}: {f}");
}

/// A 0.5 device the reader reads, writes back unchanged, and the schema accepts.
#[track_caller]
fn accepted(doc: &Value) {
    let read = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f} in {doc}"));
    assert_eq!(&serde_json::from_str::<Value>(&osd::write(&read)).unwrap(), doc);
    assert_eq!(common::Schemas::load("0.5").check(Kind::Device, doc), None, "{doc}");
}

/// A 0.5 device both the reader and the schema refuse, as a bad value at `at`.
#[track_caller]
fn refused_at(doc: &Value, at: &str) {
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err(&format!("the reader accepts {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, at), "{f}");
    let errors = common::Schemas::load("0.5").errors(Kind::Device, doc);
    assert!(errors.iter().any(|(p, _)| p.starts_with(at)), "schema: {errors:?} not under {at} for {doc}");
}

fn schema(file: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(common::repo().join("schemas/0.5").join(file)).unwrap()).unwrap()
}

fn words(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Every string of every `enum` under `v`.
fn enum_strings(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                if k == "enum" {
                    out.extend(x.as_array().unwrap().iter().filter_map(|s| s.as_str().map(String::from)));
                } else {
                    enum_strings(x, out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| enum_strings(x, out)),
        _ => {}
    }
}

const EXTENSIONS: [Place; 5] =
    [Place::ProgramsSelects, Place::BankSelects, Place::CcExpects, Place::ParamExpects, Place::SourceType];

// ---- criterion 1: identity.firmware.decode ----

#[test]
fn criterion_1_each_decode_term_is_accepted() {
    for term in DECODE {
        accepted(&Place::Decode.device(json!(term), true));
    }
}

#[test]
fn criterion_1_a_decode_outside_the_list_is_a_bad_value_and_has_no_extension() {
    for word in ["hex", "Json", "ASCII", "x-acme-decode", "x-osd-decode", "", "lua "] {
        refused_at(&Place::Decode.device(json!(word), true), "/identity/firmware/decode");
    }
}

#[test]
fn criterion_1_json_requires_field() {
    let doc = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"},
        "identity": {"firmware": {"decode": "json", "at": [5]}}});
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err("json without field");
    assert!(
        matches!(f.kind, FaultKind::Missing | FaultKind::BadValue) && f.pointer.starts_with("/identity/firmware"),
        "{f}"
    );
    let errors = common::Schemas::load("0.5").errors(Kind::Device, &doc);
    assert!(errors.iter().any(|(p, _)| p.starts_with("/identity/firmware")), "{errors:?}");
}

#[test]
fn criterion_1_lua_requires_script() {
    let doc = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"},
        "identity": {"firmware": {"decode": "lua", "at": [5]}}});
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err("lua without script");
    assert!(
        matches!(f.kind, FaultKind::Missing | FaultKind::BadValue) && f.pointer.starts_with("/identity/firmware"),
        "{f}"
    );
    let errors = common::Schemas::load("0.5").errors(Kind::Device, &doc);
    assert!(errors.iter().any(|(p, _)| p.starts_with("/identity/firmware")), "{errors:?}");
}

#[test]
fn criterion_1_a_script_is_a_string_and_the_other_decodes_need_neither_field_nor_script() {
    let mut doc = Place::Decode.device(json!("lua"), true);
    doc["identity"]["firmware"]["script"] = json!(3);
    refused_at(&doc, "/identity/firmware/script");
    for term in ["raw_bytes", "korg_minor_major", "yamaha_tenths", "ascii"] {
        accepted(&Place::Decode.device(json!(term), true));
    }
}

#[test]
fn criterion_1_a_0_4_decode_in_the_list_converts_unchanged_and_one_outside_is_not_convertible() {
    for term in ["raw_bytes", "korg_minor_major", "yamaha_tenths", "ascii"] {
        assert_eq!(converted(Place::Decode, term), json!(term));
    }
    for word in ["hex", "Json", "tenths"] {
        not_convertible(Place::Decode, word);
    }
}

// ---- criterion 2: expects ----

#[test]
fn criterion_2_each_expects_term_is_accepted_at_both_places() {
    for place in [Place::CcExpects, Place::ParamExpects] {
        for term in EXPECTS {
            accepted(&place.device(json!(term), true));
        }
    }
}

#[test]
fn criterion_2_an_expects_outside_the_list_and_outside_the_extension_form_is_a_bad_value() {
    for place in [Place::CcExpects, Place::ParamExpects] {
        for word in ["slider", "Switch", "CONTINUOUS", "knob", "", "x-acme", "x--term", "X-acme-term", "x-Acme-term", "x-ac_me-term"] {
            refused_at(&place.device(json!(word), true), place.pointer());
        }
    }
}

#[test]
fn criterion_2_the_0_4_terms_convert_unchanged_and_other_words_are_not_convertible() {
    for place in [Place::CcExpects, Place::ParamExpects] {
        for term in EXPECTS {
            assert_eq!(converted(place, term), json!(term));
        }
        for word in ["slider", "Switch", "toggle"] {
            not_convertible(place, word);
        }
    }
}

// ---- the extension rule ----

#[test]
fn extension_a_declared_extension_is_accepted_at_each_open_place() {
    for place in EXTENSIONS {
        for word in ["x-acme-wheel", "x-acme-two-words", "x-a1-b2", "x-osd-track"] {
            accepted(&place.device(json!(word), true));
        }
    }
}

#[test]
fn extension_the_form_is_registrant_then_term_in_lower_case() {
    for place in EXTENSIONS {
        for word in ["x-acme", "x-acme-", "x--wheel", "x-acme_wheel-a", "X-acme-wheel", "x-Acme-wheel", "xacme-wheel", "x-acme-Wheel", "x-ac me-wheel"] {
            refused_at(&place.device(json!(word), true), place.pointer());
        }
    }
}

#[test]
fn extension_the_registry_is_listed_beside_the_schemas() {
    let text = std::fs::read_to_string(common::repo().join("schemas/extensions.md")).unwrap();
    for word in CATALOGUE_EXTENSIONS {
        let ext = format!("x-osd-{}", word.replace(' ', "-"));
        assert!(text.contains(&ext), "schemas/extensions.md does not list {ext}");
    }
}

// ---- criterion 3 and 7: selects ----

#[test]
fn criterion_3_each_sound_kind_is_accepted_by_programs_and_banks_selects() {
    for place in [Place::ProgramsSelects, Place::BankSelects] {
        for term in KINDS {
            accepted(&place.device(json!(term), true));
        }
    }
}

#[test]
fn criterion_3_a_maker_word_is_a_bad_value_in_0_5_selects() {
    for place in [Place::ProgramsSelects, Place::BankSelects] {
        for word in ["preset", "patch", "program", "voice", "tone", "instrument", "pattern", "song", "drum kit", "Sound", "snapshot", "tone preset"] {
            refused_at(&place.device(json!(word), true), place.pointer());
        }
    }
}

#[test]
fn criterion_3_the_converter_merges_the_synonyms_into_the_terms() {
    let table: [(&str, &str); 24] = [
        ("preset", "sound"), ("patch", "sound"), ("program", "sound"), ("voice", "sound"), ("tone", "sound"),
        ("instrument", "sound"),
        ("pattern", "sequence"), ("song", "sequence"),
        ("drum kit", "kit"), ("rhythm set", "kit"), ("drum voice", "kit"),
        ("studio set", "performance"), ("combination", "performance"), ("multi", "performance"),
        ("setup", "performance"), ("mixing voice", "performance"), ("performance", "performance"),
        ("snapshot", "scene"),
        ("playlist slot", "setlist"), ("set list slot", "setlist"), ("live set", "setlist"),
        ("sound", "sound"), ("clip", "clip"), ("setlist", "setlist"),
    ];
    for place in [Place::ProgramsSelects, Place::BankSelects] {
        for (maker, term) in table {
            assert_eq!(converted(place, maker), json!(term), "{maker:?} at {place:?}");
        }
    }
}

#[test]
fn criterion_3_the_catalogue_words_with_no_term_convert_to_an_osd_extension() {
    for place in [Place::ProgramsSelects, Place::BankSelects] {
        for word in CATALOGUE_EXTENSIONS {
            let ext = format!("x-osd-{}", word.replace(' ', "-"));
            assert_eq!(converted(place, word), json!(ext), "{word:?} at {place:?}");
            let doc = convert(&place.device(json!(word), false)).unwrap();
            accepted(&doc);
        }
    }
}

#[test]
fn criterion_3_a_converted_selects_is_a_term_every_one_of_the_nine_is_unchanged() {
    for term in KINDS {
        assert_eq!(converted(Place::ProgramsSelects, term), json!(term));
    }
}

#[test]
fn criterion_6_a_selects_outside_the_table_is_not_convertible_at_its_value() {
    for place in [Place::ProgramsSelects, Place::BankSelects] {
        for word in ["tone preset", "Preset", "PATTERN", "Drum Kit", "drum  kit", " preset", "preset ", "", "sounds", "bank slot"] {
            not_convertible(place, word);
        }
    }
}

#[test]
fn criterion_6_the_converter_neither_guesses_nor_normalises_the_case() {
    not_convertible(Place::ProgramsSelects, "Sound");
    not_convertible(Place::BankSelects, "KIT");
}

#[test]
fn criterion_7_the_selects_lists_and_the_sound_kind_list_are_the_same_nine_terms() {
    let kind = {
        let mut out = BTreeSet::new();
        enum_strings(&schema("sound.schema.json")["properties"]["kind"], &mut out);
        out
    };
    assert_eq!(kind, words(&KINDS));
    let device = schema("device.schema.json");
    for def in [&device["properties"]["programs"]["properties"]["selects"], &device["$defs"]["bank"]["properties"]["selects"]] {
        let mut out = BTreeSet::new();
        enum_strings(def, &mut out);
        assert_eq!(out, kind, "selects: {def}");
    }
}

#[test]
fn criterion_7_every_word_of_the_sound_kind_is_accepted_by_selects_and_nothing_else_but_extensions() {
    let schemas = common::Schemas::load("0.5");
    let kind = {
        let mut out = BTreeSet::new();
        enum_strings(&schema("sound.schema.json")["properties"]["kind"], &mut out);
        out
    };
    for place in [Place::ProgramsSelects, Place::BankSelects] {
        for word in &kind {
            assert_eq!(schemas.check(Kind::Device, &place.device(json!(word), true)), None, "{word}");
        }
        for word in CATALOGUE_EXTENSIONS {
            let ext = format!("x-osd-{}", word.replace(' ', "-"));
            assert_eq!(schemas.check(Kind::Device, &place.device(json!(ext), true)), None, "{ext}");
            assert!(!kind.contains(&ext));
        }
    }
}

// ---- criterion 4 and 8: _sources[].type, license ----

#[test]
fn criterion_4_each_of_the_14_source_types_is_accepted() {
    assert_eq!(words(&SOURCE_TYPES).len(), 14);
    for term in SOURCE_TYPES {
        accepted(&Place::SourceType.device(json!(term), true));
    }
}

#[test]
fn criterion_4_a_source_type_outside_the_list_is_a_bad_value() {
    for word in ["vendor", "agent-contributed", "hardware-verified-partial", "Vendor-Doc", "blog", "", "x-acme"] {
        refused_at(&Place::SourceType.device(json!(word), true), "/_sources/0/type");
    }
}

fn with_license(license: Value, v05: bool) -> Value {
    let mut doc = Place::SourceType.device(json!("vendor-doc"), v05);
    doc["_sources"][0]["license"] = license;
    doc
}

#[test]
fn criterion_4_a_source_license_is_an_spdx_id_public_domain_or_none() {
    for id in ["MIT", "Apache-2.0", "CC-BY-4.0", "CC0-1.0", "GPL-3.0-or-later", "BSD-3-Clause", "public-domain", "none"] {
        accepted(&with_license(json!(id), true));
    }
}

#[test]
fn criterion_4_a_source_license_that_is_no_spdx_id_is_a_bad_value() {
    for id in ["MIT License", "cc by 4", "free", "", "Public-Domain", "None", "CC BY"] {
        refused_at(&with_license(json!(id), true), "/_sources/0/license");
    }
}

#[test]
fn criterion_5_a_0_4_license_that_is_no_spdx_id_is_not_convertible_at_its_value() {
    for id in ["MIT", "CC-BY-4.0", "public-domain", "none"] {
        let out = convert(&with_license(json!(id), false)).unwrap();
        assert_eq!(out["_sources"][0]["license"], json!(id));
    }
    for id in ["MIT License", "free", "cc by 4"] {
        let f = convert(&with_license(json!(id), false)).expect_err(id);
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, "/_sources/0/license"), "{f}");
    }
}

#[test]
fn criterion_4_and_8_the_converter_maps_the_makers_source_types() {
    for term in SOURCE_TYPES {
        assert_eq!(converted(Place::SourceType, term), json!(term));
    }
    assert_eq!(converted(Place::SourceType, "vendor"), json!("vendor-doc"));
    assert_eq!(converted(Place::SourceType, "agent-contributed"), json!("contributed"));
    assert_eq!(converted(Place::SourceType, "hardware-verified-partial"), json!("hardware-verified"));
}

#[test]
fn criterion_8_a_0_4_source_type_outside_the_list_and_the_table_is_not_convertible() {
    for word in ["blog", "Vendor", "hardware", "hardware-verified-full", ""] {
        not_convertible(Place::SourceType, word);
    }
}

#[test]
fn criterion_8_a_converted_source_keeps_its_other_keys_and_reads_as_0_5() {
    let mut doc = Place::SourceType.device(json!("hardware-verified-partial"), false);
    doc["_sources"][0]["notes"] = json!("only the filter was tested");
    doc["_sources"][0]["url"] = json!("https://example.com");
    let out = convert(&doc).unwrap();
    assert_eq!(out["_sources"][0]["notes"], json!("only the filter was tested"));
    assert_eq!(out["_sources"][0]["url"], json!("https://example.com"));
    osd::read(&out.to_string(), Kind::Device).unwrap();
    assert_eq!(common::Schemas::load("0.5").check(Kind::Device, &out), None);
}

// ---- criterion 5: the converter's result reads ----

#[test]
fn criterion_5_a_converted_descriptor_with_every_vocabulary_reads_and_validates() {
    let mut doc = json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "identity": {"firmware": {"decode": "ascii", "at": [5]}},
        "programs": {"selects": "patch", "banks": [{"name": "A", "size": 8, "selects": "pattern"}]},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "expects": "switch"}]},
        "_sources": [{"type": "vendor", "license": "MIT"}],
    });
    let out = convert(&doc).unwrap();
    assert_eq!(out["programs"]["selects"], json!("sound"));
    assert_eq!(out["programs"]["banks"][0]["selects"], json!("sequence"));
    assert_eq!(out["_sources"][0]["type"], json!("vendor-doc"));
    osd::read(&out.to_string(), Kind::Device).unwrap();
    assert_eq!(common::Schemas::load("0.5").check(Kind::Device, &out), None);
    // converting what is already converted changes nothing
    assert_eq!(convert(&out).unwrap(), out);
    doc["programs"]["selects"] = json!("tone preset");
    not_convertible_doc(&doc, "/programs/selects");
}

#[track_caller]
fn not_convertible_doc(doc: &Value, at: &str) {
    let f = convert(doc).expect_err("converted");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, at), "{f}");
}

// ---- criterion 9: one document ----

fn readme() -> String {
    std::fs::read_to_string(common::repo().join("README.md")).unwrap()
}

#[test]
fn criterion_9_the_readme_writes_every_list_term_and_the_converter_tables() {
    let text = readme();
    let all = DECODE.iter().chain(&EXPECTS).chain(&KINDS).chain(&SOURCE_TYPES);
    for term in all {
        assert!(text.contains(&format!("`{term}`")), "README.md does not write `{term}`");
    }
    let maker_words = [
        "preset", "patch", "program", "voice", "tone", "instrument", "pattern", "song", "drum kit", "rhythm set",
        "drum voice", "studio set", "combination", "multi", "setup", "mixing voice", "snapshot", "playlist slot",
        "set list slot", "live set", "vendor", "agent-contributed", "hardware-verified-partial",
    ];
    for word in maker_words {
        assert!(text.contains(&format!("`{word}`")), "README.md does not write the maker word `{word}`");
    }
    for word in CATALOGUE_EXTENSIONS {
        assert!(text.contains(&format!("x-osd-{}", word.replace(' ', "-"))), "README.md does not write x-osd-{word}");
    }
    assert!(text.contains("^x-[a-z0-9]+-[a-z0-9-]+$"), "README.md does not write the extension rule");
}

#[test]
fn criterion_9_the_interface_of_the_reader_points_to_the_readme_without_copying_the_tables() {
    let text = std::fs::read_to_string(common::repo().join("osd/docs/INTERFACE.md")).unwrap();
    assert!(text.contains("README.md"), "osd/docs/INTERFACE.md does not point to README.md");
    for word in ["hardware-verified-partial", "agent-contributed", "rhythm set", "mixing voice"] {
        assert!(!text.contains(word), "osd/docs/INTERFACE.md copies the table (`{word}`)");
    }
}
