//! Frames and checksums as structures (0.5): at each of the eight frame
//! places an element is a byte or a one-key object (`arg`, `capture`,
//! `device_id`, `identity`, `checksum`, or the word `any`); a `pre`
//! expression is a tree of `add`, `sub`, `mul`, `gt`, `lt`, `eq`, `ne`, `if`.
//! `osd::convert` turns every 0.4 string form into its structure; a form it
//! cannot turn is a fault that names the element. The schema and the reader
//! refuse a 0.4 string, a two-key object, an unknown key and a value outside
//! the written lists, at the element's address.

mod common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.5/device.schema.json";

/// The eight frame places of a device, each with the JSON pointer of the
/// frame it holds and whether the place also accepts `any` and `capture`
/// (the places that recognise a message: `identity.reply`, `match_frame`).
#[derive(Clone, Copy, Debug)]
enum Place {
    CommandsFrame,
    ParamsGet,
    ParamsSet,
    RepliesMatch,
    IdentityRequest,
    IdentityReply,
    ProgramsBefore,
    DumpRequest,
}

const PLACES: [Place; 8] = [
    Place::CommandsFrame,
    Place::ParamsGet,
    Place::ParamsSet,
    Place::RepliesMatch,
    Place::IdentityRequest,
    Place::IdentityReply,
    Place::ProgramsBefore,
    Place::DumpRequest,
];

impl Place {
    fn pointer(self) -> &'static str {
        match self {
            Place::CommandsFrame => "/commands/0/frame",
            Place::ParamsGet => "/params/get_frame",
            Place::ParamsSet => "/params/set_frame",
            Place::RepliesMatch => "/replies/0/match_frame",
            Place::IdentityRequest => "/identity/request",
            Place::IdentityReply => "/identity/reply",
            Place::ProgramsBefore => "/programs/before/0",
            Place::DumpRequest => "/programs/dump/request",
        }
    }

    fn recognises(self) -> bool {
        matches!(self, Place::RepliesMatch | Place::IdentityReply)
    }

    /// A device whose frame at this place is `frame`; `$schema` of 0.5 when
    /// `v05`.
    fn device(self, frame: Value, v05: bool) -> Value {
        let base = json!({"device": {"name": "T", "osc_prefix": "/t"}});
        let mut doc = base;
        let part = match self {
            Place::CommandsFrame => json!({"commands": [{"osc": "/x", "frame": frame}]}),
            Place::ParamsGet => json!({"params": {"get_frame": frame, "set_frame": [240, 247], "value_type": "u7", "entries": []}}),
            Place::ParamsSet => json!({"params": {"get_frame": [240, 247], "set_frame": frame, "value_type": "u7", "entries": []}}),
            Place::RepliesMatch => json!({"replies": [{"match_frame": frame, "emit_osc": "/r"}]}),
            Place::IdentityRequest => json!({"identity": {"request": frame}}),
            Place::IdentityReply => json!({"identity": {"reply": frame}}),
            Place::ProgramsBefore => json!({"programs": {"before": [frame]}}),
            Place::DumpRequest => json!({"programs": {"dump": {"kind": "request", "request": frame}}}),
        };
        doc.as_object_mut().unwrap().extend(part.as_object().unwrap().clone());
        if v05 {
            let mut out = serde_json::Map::new();
            out.insert("$schema".into(), json!(SCHEMA));
            out.extend(doc.as_object().unwrap().clone());
            return Value::Object(out);
        }
        doc
    }
}

fn ptr(place: Place, index: usize) -> String {
    format!("{}/{index}", place.pointer())
}

fn convert(doc: &Value) -> Result<Value, osd::Fault> {
    osd::convert(&doc.to_string(), Kind::Device).map(|t| serde_json::from_str(&t).unwrap())
}

/// The 0.4 forms valid at every place, and the 0.5 structure of each.
fn forms() -> Vec<(Value, Value)> {
    vec![
        (json!(240), json!(240)),
        (json!("{dev}"), json!({"device_id": true})),
        (json!("{value}"), json!({"arg": "value"})),
        (json!("{v:u14_msb}"), json!({"arg": "v", "encoding": "u14_msb"})),
        (json!("{v:u14_lsb}"), json!({"arg": "v", "encoding": "u14_lsb"})),
        (json!("{s:s32}"), json!({"arg": "s", "encoding": "s32"})),
        (json!("{t:ascii}"), json!({"arg": "t", "encoding": "ascii"})),
        (json!("{b:bytes}"), json!({"arg": "b", "encoding": "bytes"})),
        (json!("{channel:u7+0x30}"), json!({"arg": "channel", "add": 48})),
        (json!("{channel:u7+32}"), json!({"arg": "channel", "add": 32})),
        (json!("{family}"), json!({"identity": "family"})),
        (json!("{member}"), json!({"identity": "member"})),
        (json!("{family:2}"), json!({"identity": "family", "index": 2})),
        (json!("{member:0}"), json!({"identity": "member", "index": 0})),
        (json!("{checksum:twos@1}"), json!({"checksum": "twos7", "from": 1})),
        (json!(247), json!(247)),
    ]
}

/// The forms at one place: where a message is recognised (`identity.reply`,
/// `match_frame`) a bare name is a capture, elsewhere it is an `arg`.
fn forms_at(place: Place) -> Vec<(Value, Value)> {
    forms()
        .into_iter()
        .map(|(old, new)| {
            if place.recognises() && old == json!("{value}") {
                (old, json!({"capture": "value"}))
            } else {
                (old, new)
            }
        })
        .collect()
}

#[test]
fn criterion_1_a_bare_name_is_a_capture_where_a_message_is_recognised_and_an_arg_where_one_is_sent() {
    for place in PLACES {
        let out = convert(&place.device(json!([240, "{value}", 247]), false)).unwrap_or_else(|f| panic!("{place:?}: {f}"));
        let want = if place.recognises() { json!({"capture": "value"}) } else { json!({"arg": "value"}) };
        assert_eq!(frame_of(&out, place), &json!([240, want, 247]), "{place:?}");
    }
}

fn frame_of(doc: &Value, place: Place) -> &Value {
    doc.pointer(place.pointer()).unwrap_or_else(|| panic!("{place:?}: no frame in {doc}"))
}

#[test]
fn criterion_1_and_4_every_0_4_form_becomes_its_structure_at_each_of_the_eight_places() {
    for place in PLACES {
        let (before, after): (Vec<Value>, Vec<Value>) = forms_at(place).into_iter().unzip();
        let out = convert(&place.device(Value::Array(before.clone()), false)).unwrap_or_else(|f| panic!("{place:?}: {f}"));
        assert_eq!(frame_of(&out, place), &Value::Array(after.clone()), "{place:?}");
    }
}

#[test]
fn criterion_1_and_4_each_form_alone_converts_alone() {
    for place in PLACES {
        for (old, new) in forms_at(place) {
            let out = convert(&place.device(json!([old]), false)).unwrap_or_else(|f| panic!("{place:?} {old}: {f}"));
            assert_eq!(frame_of(&out, place), &json!([new]), "{place:?} {old}");
        }
    }
}

#[test]
fn criterion_1_and_4_the_places_that_recognise_turn_any_and_captures() {
    let old = json!([240, "{dev}", "{any}", "{id}", "{value:u14_msb}", "{any}", 247]);
    let new = json!([240, {"device_id": true}, "any", {"capture": "id"}, {"arg": "value", "encoding": "u14_msb"}, "any", 247]);
    for place in PLACES.into_iter().filter(|p| p.recognises()) {
        let out = convert(&place.device(old.clone(), false)).unwrap_or_else(|f| panic!("{place:?}: {f}"));
        assert_eq!(frame_of(&out, place), &new, "{place:?}");
    }
}

#[test]
fn criterion_1_the_profile_holds_the_identity_and_program_places() {
    let old = json!({"identity": {"request": [240, "{family:1}", 247], "reply": [240, "{dev}", "{any}", "{x}", 247]},
                     "programs": {"before": [[240, "{value:u7+0x20}", 247]], "dump": {"kind": "request", "request": [240, "{dev}", 247]}}});
    let out: Value = serde_json::from_str(&osd::convert(&old.to_string(), Kind::Profile).unwrap()).unwrap();
    assert_eq!(out["identity"]["request"], json!([240, {"identity": "family", "index": 1}, 247]));
    assert_eq!(out["identity"]["reply"], json!([240, {"device_id": true}, "any", {"capture": "x"}, 247]));
    assert_eq!(out["programs"]["before"], json!([[240, {"arg": "value", "add": 32}, 247]]));
    assert_eq!(out["programs"]["dump"]["request"], json!([240, {"device_id": true}, 247]));
    let schemas = common::Schemas::load("0.5");
    assert_eq!(schemas.check(Kind::Profile, &out), None);
    osd::read_as(&out.to_string(), Kind::Profile, osd::Version::V0_5).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn criterion_7_a_converted_descriptor_validates_reads_writes_back_and_converts_to_itself() {
    let schemas = common::Schemas::load("0.5");
    for place in PLACES {
        let (before, _): (Vec<Value>, Vec<Value>) = forms_at(place).into_iter().unzip();
        let text = osd::convert(&place.device(Value::Array(before.clone()), false).to_string(), Kind::Device)
            .unwrap_or_else(|f| panic!("{place:?}: {f}"));
        let doc: Value = serde_json::from_str(&text).unwrap();
        assert!(frame_of(&doc, place).as_array().unwrap().iter().any(Value::is_object), "{place:?}: no structure");
        assert_eq!(schemas.check(Kind::Device, &doc), None, "{place:?}");
        let read = osd::read_as(&text, Kind::Device, osd::Version::V0_5).unwrap_or_else(|f| panic!("{place:?}: {f}"));
        assert_eq!(osd::write(&read), text, "{place:?}: written back");
        assert_eq!(osd::convert(&text, Kind::Device).unwrap(), text, "{place:?}: converted twice");
    }
}

#[test]
fn criterion_1_a_0_5_frame_of_every_form_reads_and_writes_back_key_for_key() {
    let schemas = common::Schemas::load("0.5");
    for place in PLACES {
        let mut frame: Vec<Value> = forms_at(place).into_iter().map(|(_, new)| new).collect();
        if place.recognises() {
            frame.push(json!("any"));
            frame.push(json!({"capture": "c"}));
        }
        let doc = place.device(Value::Array(frame), true);
        assert_eq!(schemas.check(Kind::Device, &doc), None, "{place:?}");
        let read = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{place:?}: {f}"));
        let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
        assert_eq!(back, doc, "{place:?}");
    }
}

#[test]
fn a_0_4_descriptor_still_reads_its_string_forms() {
    for place in PLACES {
        let doc = place.device(json!([240, "{dev}", "{value:u14_msb}", "{checksum:twos@1}", 247]), false);
        let read = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{place:?}: {f}"));
        assert_eq!(serde_json::from_str::<Value>(&osd::write(&read)).unwrap(), doc, "{place:?}");
    }
}

/// What the schema and the reader both say of a 0.5 device: refused, at
/// `at` (the element, or a value inside it).
#[track_caller]
fn refused_at(doc: &Value, at: &str, kinds: &[FaultKind]) {
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err(&format!("the reader accepts {doc}"));
    assert!(kinds.contains(&f.kind), "{f}: kind {:?} not in {kinds:?}", f.kind);
    assert!(f.pointer.starts_with(at), "reader: fault at {} not under {at} ({f})", f.pointer);
    let errors = common::Schemas::load("0.5").errors(Kind::Device, doc);
    assert!(!errors.is_empty(), "the schema accepts {doc}");
    assert!(errors.iter().any(|(p, _)| p.starts_with(at)), "schema: errors {errors:?} not under {at}");
}

const FRAME_FAULTS: [FaultKind; 2] = [FaultKind::BadValue, FaultKind::UnknownField];

/// The control of a refusal: the well-formed twin of the document is read by
/// the reader and valid against the schema, so a refusal is of the fault and
/// not of the whole form.
#[track_caller]
fn accepted(doc: &Value) {
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f} in {doc}"));
    assert_eq!(common::Schemas::load("0.5").check(Kind::Device, doc), None, "{doc}");
}

/// The twin of a frame at `place` whose element 1 is well formed.
#[track_caller]
fn accepted_at(place: Place) {
    accepted(&place.device(json!([240, {"arg": "v", "encoding": "u14_msb"}, 247]), true));
}

#[test]
fn criterion_5_a_0_4_string_form_is_refused_at_each_place_by_the_schema_and_the_reader() {
    for place in PLACES {
        for bad in ["{dev}", "{value}", "{v:u14_msb}", "{checksum:twos@1}", "{family}", "{any}"] {
            let doc = place.device(json!([240, bad, 247]), true);
            refused_at(&doc, &ptr(place, 1), &FRAME_FAULTS);
        }
    }
}

#[test]
fn criterion_5_the_word_any_is_the_only_string_and_only_where_a_message_is_recognised() {
    for place in PLACES.into_iter().filter(|p| p.recognises()) {
        let doc = place.device(json!([240, "any", 247]), true);
        assert!(osd::read(&doc.to_string(), Kind::Device).is_ok(), "{place:?}");
        assert_eq!(common::Schemas::load("0.5").check(Kind::Device, &doc), None, "{place:?}");
        refused_at(&place.device(json!([240, "anything", 247]), true), &ptr(place, 1), &FRAME_FAULTS);
    }
}

#[test]
fn criterion_6_an_object_of_two_keys_or_of_an_unknown_key_is_refused_at_each_place() {
    let bad = [
        json!({"arg": "v", "capture": "c"}),
        json!({"device_id": true, "arg": "v"}),
        json!({"identity": "family", "checksum": "twos7"}),
        json!({"checksum": "twos7", "from": 1, "arg": "v"}),
        json!({"colour": "red"}),
        json!({"arg": "v", "colour": "red"}),
        json!({}),
    ];
    for place in PLACES {
        accepted_at(place);
        for b in &bad {
            refused_at(&place.device(json!([240, b, 247]), true), &ptr(place, 1), &FRAME_FAULTS);
        }
    }
}

#[test]
fn criterion_6_an_encoding_an_algorithm_an_index_or_an_add_outside_the_written_values_is_refused() {
    let bad = [
        json!({"arg": "v", "encoding": "u15"}),
        json!({"arg": "v", "encoding": "u7"}),
        json!({"arg": "v", "encoding": 14}),
        json!({"arg": "v", "add": "x"}),
        json!({"arg": "v", "add": 1.5}),
        json!({"arg": 7}),
        json!({"identity": "firmware"}),
        json!({"identity": "family", "index": -1}),
        json!({"identity": "family", "index": "a"}),
        json!({"checksum": "roland", "from": 1}),
        json!({"checksum": "twos7", "from": -1}),
        json!({"checksum": "twos7", "from": "a"}),
    ];
    for place in PLACES {
        accepted_at(place);
        for b in &bad {
            refused_at(&place.device(json!([240, b, 247]), true), &ptr(place, 1), &FRAME_FAULTS);
        }
    }
}

#[test]
fn criterion_6_a_byte_outside_0_to_255_is_still_refused() {
    for place in PLACES {
        accepted_at(place);
        refused_at(&place.device(json!([240, 256, 247]), true), &ptr(place, 1), &[FaultKind::BadValue]);
    }
}

#[test]
fn criterion_6_a_capture_is_refused_where_no_message_is_recognised() {
    for place in PLACES.into_iter().filter(|p| !p.recognises()) {
        accepted_at(place);
        refused_at(&place.device(json!([240, {"capture": "c"}, 247]), true), &ptr(place, 1), &FRAME_FAULTS);
    }
}

#[test]
fn criterion_3_and_8_a_checksum_without_algorithm_and_a_function_are_faults_that_name_the_element() {
    for place in PLACES {
        for bad in ["{checksum}", "{function}"] {
            let doc = place.device(json!([240, "{value}", bad, 247]), false);
            let f = convert(&doc).expect_err(&format!("{place:?} {bad} converted"));
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, ptr(place, 2).as_str()), "{place:?} {bad}: {f}");
        }
    }
}

#[test]
fn criterion_8_a_fault_leaves_no_partial_text_even_when_other_elements_convert() {
    let doc = json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "params": {"get_frame": [240, "{dev}", 247], "set_frame": [240, 247], "value_type": "u7", "entries": []},
        "commands": [{"osc": "/ok", "frame": [240, "{dev}", 247]}, {"osc": "/x", "frame": [240, "{checksum}", 247]}]
    });
    let f = osd::convert(&doc.to_string(), Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, "/commands/1/frame/1"), "{f}");
}

// ---- pre: expression trees -------------------------------------------------

fn with_pre(expr: Value, v05: bool) -> Value {
    let mut doc = json!({"device": {"name": "T", "osc_prefix": "/t"},
                         "commands": [{"osc": "/x", "pre": [{"bind": "r", "expr": expr}]}]});
    if v05 {
        doc.as_object_mut().unwrap().insert("$schema".into(), json!(SCHEMA));
    }
    doc
}

fn arg(n: &str) -> Value {
    json!({"arg": n})
}

#[test]
fn criterion_2_and_4_every_0_4_expression_becomes_a_tree() {
    let cases = [
        ("a + 1", json!({"add": [arg("a"), 1]})),
        ("a - b", json!({"sub": [arg("a"), arg("b")]})),
        ("a * 2", json!({"mul": [arg("a"), 2]})),
        ("a > 3", json!({"gt": [arg("a"), 3]})),
        ("a < 3", json!({"lt": [arg("a"), 3]})),
        ("a == 3", json!({"eq": [arg("a"), 3]})),
        ("a != 3", json!({"ne": [arg("a"), 3]})),
        ("a > 63 ? 127 : 0", json!({"if": [{"gt": [arg("a"), 63]}, 127, 0]})),
        ("a + b * 2", json!({"add": [arg("a"), {"mul": [arg("b"), 2]}]})),
        ("(a + b) * 2", json!({"mul": [{"add": [arg("a"), arg("b")]}, 2]})),
        ("a - b - c", json!({"sub": [{"sub": [arg("a"), arg("b")]}, arg("c")]})),
        ("a", arg("a")),
        ("64", json!(64)),
    ];
    for (text, tree) in cases {
        let out = convert(&with_pre(json!(text), false)).unwrap_or_else(|f| panic!("{text}: {f}"));
        assert_eq!(out["commands"][0]["pre"][0]["expr"], tree, "{text}");
        assert_eq!(out["commands"][0]["pre"][0]["bind"], "r", "{text}");
    }
}

#[test]
fn criterion_2_and_7_a_converted_expression_validates_reads_and_converts_to_itself() {
    let schemas = common::Schemas::load("0.5");
    for text in ["a + 1", "a > 63 ? 127 : 0", "(a + b) * 2", "a != b ? a - b : b - a"] {
        let out = osd::convert(&with_pre(json!(text), false).to_string(), Kind::Device).unwrap_or_else(|f| panic!("{text}: {f}"));
        let doc: Value = serde_json::from_str(&out).unwrap();
        assert!(doc["commands"][0]["pre"][0]["expr"].is_object(), "{text}: not a tree");
        assert_eq!(schemas.check(Kind::Device, &doc), None, "{text}");
        let read = osd::read_as(&out, Kind::Device, osd::Version::V0_5).unwrap_or_else(|f| panic!("{text}: {f}"));
        assert_eq!(osd::write(&read), out, "{text}");
        assert_eq!(osd::convert(&out, Kind::Device).unwrap(), out, "{text}");
    }
}

#[test]
fn criterion_9_literals_from_0_to_127_are_read_and_the_bounds_are_inclusive() {
    for lit in [0, 1, 127] {
        let doc = with_pre(json!({"add": [arg("a"), lit]}), true);
        assert!(osd::read(&doc.to_string(), Kind::Device).is_ok(), "{lit}");
        assert_eq!(common::Schemas::load("0.5").check(Kind::Device, &doc), None, "{lit}");
    }
}

#[test]
fn criterion_9_a_literal_outside_0_to_127_is_refused_by_the_reader_with_its_address() {
    accepted(&with_pre(json!({"if": [{"gt": [arg("a"), 1]}, {"mul": [arg("a"), 127]}, 0]}), true));
    for lit in [json!(128), json!(-1), json!(1000)] {
        let doc = with_pre(json!({"if": [{"gt": [arg("a"), 1]}, {"mul": [arg("a"), lit]}, 0]}), true);
        let f = osd::read(&doc.to_string(), Kind::Device).expect_err("refused");
        assert_eq!(f.kind, FaultKind::BadValue, "{f}");
        assert!(f.pointer.starts_with("/commands/0/pre/0/expr"), "{f}");
    }
    let f = osd::read(&with_pre(json!(200), true).to_string(), Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/commands/0/pre/0/expr"), "{f}");
}

#[test]
fn criterion_9_a_literal_outside_0_to_127_is_refused_by_the_schema() {
    accepted(&with_pre(json!({"add": [arg("a"), 127]}), true));
    for lit in [json!(128), json!(-1)] {
        let doc = with_pre(json!({"add": [arg("a"), lit]}), true);
        assert!(!common::Schemas::load("0.5").errors(Kind::Device, &doc).is_empty());
    }
}

#[test]
fn criterion_5_a_pre_expression_written_as_a_string_is_refused() {
    let doc = with_pre(json!("a + 1"), true);
    refused_at(&doc, "/commands/0/pre/0/expr", &[FaultKind::BadValue]);
}

#[test]
fn criterion_6_a_malformed_tree_is_refused_with_its_address() {
    let bad = [
        json!({"add": [arg("a")]}),
        json!({"add": [arg("a"), 1, 2]}),
        json!({"if": [1, 2]}),
        json!({"div": [arg("a"), 2]}),
        json!({"add": [arg("a"), 1], "mul": [arg("a"), 2]}),
        json!({"add": [arg("a"), "b"]}),
        json!({"add": [arg("a"), 1.5]}),
        json!({"arg": "a", "add": 1}),
        json!({}),
    ];
    accepted(&with_pre(json!({"if": [{"ne": [arg("a"), 1]}, arg("a"), 0]}), true));
    for expr in bad {
        refused_at(&with_pre(expr, true), "/commands/0/pre/0/expr", &FRAME_FAULTS);
    }
}

#[test]
fn criterion_9_the_bounds_are_written_in_the_specification() {
    let text = std::fs::read_to_string(common::repo().join("docs/INTERFACE.md")).unwrap();
    assert!(text.contains("127"), "docs/INTERFACE.md: no 0..127 bound");
    let lower = text.to_lowercase();
    assert!(lower.contains("clamp") || lower.contains("bounded"), "docs/INTERFACE.md: the bound is not stated");
    assert!(lower.contains("json logic"), "docs/INTERFACE.md: the departure from JSON Logic has no reason");
}

// ---- criteria 10 and 14 ----------------------------------------------------

#[test]
fn criterion_10_an_encoding_the_engine_does_not_know_is_a_fault_naming_the_field_at_each_place() {
    for place in PLACES {
        for bad in ["{position:lsb}", "{position:msb}"] {
            let doc = place.device(json!([242, bad, "{position:msb}"]), false);
            let f = convert(&doc).expect_err(&format!("{place:?} {bad} converted"));
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, ptr(place, 1).as_str()), "{place:?} {bad}: {f}");
            assert!(osd::convert(&doc.to_string(), Kind::Device).is_err(), "no partial text");
        }
    }
}

#[test]
fn criterion_10_the_circuit_command_names_its_field_and_no_text_comes_back() {
    let doc = json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "commands": [{"osc": "/ok", "frame": [240, "{p:u14_lsb}", 247]},
                     {"osc": "/song/position", "frame": [242, "{position:lsb}", "{position:msb}"]}]
    });
    let f = osd::convert(&doc.to_string(), Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, "/commands/1/frame/1"), "{f}");
}

#[test]
fn criterion_9_a_pre_literal_above_127_is_a_conversion_fault_naming_the_field() {
    let doc = with_pre(json!("128"), false);
    let f = osd::convert(&doc.to_string(), Kind::Device);
    let f = f.expect_err("a literal above 127 is refused, not clamped");
    assert_eq!(f.kind, FaultKind::NotConvertible, "{f}");
    assert!(f.pointer.starts_with("/commands/0/pre/0"), "{f}");
}

#[test]
fn criterion_14_the_schema_holds_eight_frame_sites_and_each_has_a_place_in_the_cases() {
    let text = std::fs::read_to_string(common::repo().join("schemas/0.5/device.schema.json")).unwrap();
    let plain = text.matches("\"$ref\": \"#/$defs/frame_token\"").count();
    let recognised = text.matches("\"$ref\": \"#/$defs/recognised_frame_token\"").count();
    assert_eq!((plain, recognised), (6, 2), "the sites of the schema changed: add a place for each");
    let profile = std::fs::read_to_string(common::repo().join("schemas/0.5/profile.schema.json")).unwrap();
    assert!(!profile.contains("frame_token"), "the profile schema holds a frame site");
    let pointers: std::collections::BTreeSet<&str> = PLACES.iter().map(|p| p.pointer()).collect();
    assert_eq!(pointers.len(), plain + recognised);
    assert_eq!(PLACES.iter().filter(|p| p.recognises()).count(), recognised);
}

#[test]
fn criterion_1_the_word_any_of_a_0_4_document_is_read_only_in_identity_reply() {
    let witness = Place::IdentityReply.device(json!([240, "any", 247]), false);
    osd::read(&witness.to_string(), Kind::Device).unwrap_or_else(|f| panic!("identity.reply 0.4: {f}"));
    for place in PLACES.into_iter().filter(|p| !matches!(p, Place::IdentityReply)) {
        let doc = place.device(json!([240, "any", 247]), false);
        let f = osd::read(&doc.to_string(), Kind::Device).expect_err(&format!("{place:?}: 0.4 reads any"));
        assert_eq!(f.kind, FaultKind::BadValue, "{place:?}: {f}");
        assert!(f.pointer.starts_with(&ptr(place, 1)), "{place:?}: fault at {} not under {}", f.pointer, ptr(place, 1));
    }
}
