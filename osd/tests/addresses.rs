//! Addresses and reply templates as structures (0.5). An address that takes
//! captures (`commands[].osc`, `forward.path`, `replies[].match_osc`,
//! `replies[].emit_osc`) is a plain string without braces or an array of
//! segments, each a string or `{"capture": "name"}`; the arguments of
//! `forward.args` and `replies[].emit_args` are JSON literals or
//! `{"arg": "name"}` / `{"capture": "name"}`; `midi_in` is the list of the
//! MIDI messages received, whose OSC address is fixed by the specification.
//! `osd::convert` turns every 0.4 template into its structure; a template it
//! cannot turn is a fault that names the element. The schema and the reader
//! refuse a brace in a string address, a malformed array of segments, an
//! unknown argument object and an unknown `midi_in` type, at the address of
//! the fault.

mod common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.5/device.schema.json";

/// The four places of a 0.5 address that takes captures.
#[derive(Clone, Copy, Debug)]
enum Place {
    /// `commands[].osc`: an address that is recognised (first segment `/…`).
    CommandOsc,
    /// `commands[].forward.path`: an address that is emitted.
    ForwardPath,
    /// `replies[].match_osc`: an address that is recognised.
    MatchOsc,
    /// `replies[].emit_osc`: an address that is emitted.
    EmitOsc,
}

const PLACES: [Place; 4] = [Place::CommandOsc, Place::ForwardPath, Place::MatchOsc, Place::EmitOsc];

impl Place {
    fn pointer(self) -> &'static str {
        match self {
            Place::CommandOsc => "/commands/0/osc",
            Place::ForwardPath => "/commands/0/forward/path",
            Place::MatchOsc => "/replies/0/match_osc",
            Place::EmitOsc => "/replies/0/emit_osc",
        }
    }

    /// Where an address is recognised, two captures side by side are
    /// ambiguous and refused; where it is emitted they are not.
    fn recognises(self) -> bool {
        matches!(self, Place::CommandOsc | Place::MatchOsc)
    }

    /// A device whose address at this place is `address`, 0.5 when `v05`.
    fn device(self, address: Value, v05: bool) -> Value {
        let mut doc = serde_json::Map::new();
        if v05 {
            doc.insert("$schema".into(), json!(SCHEMA));
        }
        doc.insert("device".into(), json!({"name": "T", "osc_prefix": "/t"}));
        let (key, part) = match self {
            Place::CommandOsc => ("commands", json!([{"osc": address}])),
            Place::ForwardPath => ("commands", json!([{"osc": "/x", "forward": {"path": address}}])),
            Place::MatchOsc => ("replies", json!([{"match_osc": address, "emit_osc": "/r"}])),
            Place::EmitOsc => ("replies", json!([{"emit_osc": address}])),
        };
        doc.insert(key.into(), part);
        Value::Object(doc)
    }
}

fn convert(doc: &Value) -> Result<Value, osd::Fault> {
    osd::convert(&doc.to_string(), Kind::Device).map(|t| serde_json::from_str(&t).unwrap())
}

fn cap(name: &str) -> Value {
    json!({"capture": name})
}

const KINDS: [FaultKind; 2] = [FaultKind::BadValue, FaultKind::UnknownField];

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

/// A 0.5 device the reader reads, writes back unchanged, and the schema accepts.
#[track_caller]
fn accepted(doc: &Value) {
    let read = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f} in {doc}"));
    assert_eq!(&serde_json::from_str::<Value>(&osd::write(&read)).unwrap(), doc);
    assert_eq!(common::Schemas::load("0.5").check(Kind::Device, doc), None, "{doc}");
}

/// A 0.4 template that cannot be turned: the fault names the element's
/// address in the 0.4 text.
#[track_caller]
fn not_convertible(doc: &Value, at: &str) -> osd::Fault {
    let f = convert(doc).expect_err(&format!("converted: {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, at), "{f}");
    f
}

// ---- criteria 1, 5, 6: the address ----

#[test]
fn criterion_1_a_plain_string_without_braces_is_an_address_at_each_place() {
    for place in PLACES {
        accepted(&place.device(json!("/a/b"), true));
    }
}

#[test]
fn criterion_1_and_6_an_array_of_segments_with_a_capture_is_an_address_at_each_place() {
    let forms = [
        json!(["/cc/", cap("num")]),
        json!(["/a/", cap("x"), "/b/", cap("y")]),
        json!(["/a/", cap("x"), "/b"]),
        json!(["/a/b/", cap("x_1")]),
        json!(["/", cap("_n"), "/"]),
    ];
    for place in PLACES {
        for form in &forms {
            accepted(&place.device(form.clone(), true));
        }
    }
}

#[test]
fn criterion_6_the_segments_are_concatenated_and_never_split_on_a_slash() {
    // a segment that holds several slashes is one segment; a capture may sit between two slashes
    for place in PLACES {
        accepted(&place.device(json!(["/a/b/c/", cap("x"), "/d/e"]), true));
        accepted(&place.device(json!(["/", cap("x"), "/"]), true));
    }
}

#[test]
fn criterion_5_an_address_with_a_brace_is_refused_as_a_string_and_read_as_segments() {
    for place in PLACES {
        let at = place.pointer();
        refused_at(&place.device(json!("/x/{n}"), true), at, &KINDS);
        refused_at(&place.device(json!("/x/{"), true), at, &KINDS);
        refused_at(&place.device(json!("/x/}"), true), at, &KINDS);
        refused_at(&place.device(json!("/x/{a}/{b}"), true), at, &KINDS);
        accepted(&place.device(json!(["/x/", cap("n")]), true));
    }
}

#[test]
fn criterion_6_an_empty_array_and_an_array_without_a_capture_are_refused() {
    for place in PLACES {
        let at = place.pointer();
        refused_at(&place.device(json!([]), true), at, &KINDS);
        // one string segment: an address without capture is written as a string
        refused_at(&place.device(json!(["/x"]), true), at, &KINDS);
        refused_at(&place.device(json!(["/x/", "y/"]), true), at, &KINDS);
    }
}

#[test]
fn criterion_6_a_segment_is_a_string_without_brace_or_a_capture_object_of_a_valid_name() {
    for place in PLACES {
        let at = place.pointer();
        for bad in [
            json!(["/x/{", cap("n")]),
            json!(["/x/}", cap("n")]),
            json!(["/x/", {"capture": "1a"}]),
            json!(["/x/", {"capture": "a-b"}]),
            json!(["/x/", {"capture": ""}]),
            json!(["/x/", {"capture": "a b"}]),
            json!(["/x/", {"capture": 3}]),
            json!(["/x/", {"capture": "a", "extra": 1}]),
            json!(["/x/", {"arg": "a"}]),
            json!(["/x/", {}]),
            json!(["/x/", cap("n"), 7]),
            json!(["/x/", cap("n"), null]),
            json!(["/x/", cap("n"), ["/y"]]),
        ] {
            refused_at(&place.device(bad, true), at, &KINDS);
        }
    }
}

#[test]
fn criterion_6_the_first_segment_of_a_command_address_is_a_string_that_starts_with_a_slash() {
    let at = Place::CommandOsc.pointer();
    refused_at(&Place::CommandOsc.device(json!([cap("n"), "/x"]), true), at, &KINDS);
    refused_at(&Place::CommandOsc.device(json!(["x/", cap("n")]), true), at, &KINDS);
    // the other places do not require it
    for place in [Place::ForwardPath, Place::MatchOsc, Place::EmitOsc] {
        accepted(&place.device(json!([cap("n"), "/x"]), true));
    }
}

#[test]
fn criterion_6_two_adjacent_captures_are_refused_where_an_address_is_recognised_and_read_where_it_is_emitted() {
    for place in PLACES {
        let doc = place.device(json!(["/x/", cap("a"), cap("b")]), true);
        if place.recognises() {
            refused_at(&doc, place.pointer(), &KINDS);
            // separated by a string segment they are fine
            accepted(&place.device(json!(["/x/", cap("a"), "-", cap("b")]), true));
        } else {
            accepted(&doc);
        }
    }
}

#[test]
fn criterion_1_a_subscription_forward_path_takes_segments_and_refuses_a_brace_string() {
    let doc = |path: Value| {
        json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"},
               "subscriptions": [{"on": "startup", "forward": {"path": path}}]})
    };
    accepted(&doc(json!(["/s/", cap("n")])));
    refused_at(&doc(json!("/s/{n}")), "/subscriptions/0/forward/path", &KINDS);
}

// ---- criteria 2, 8: the arguments ----

/// A device whose first command forwards `args`.
fn forward_args(args: Value, v05: bool) -> Value {
    let mut doc = json!({"device": {"name": "T", "osc_prefix": "/t"},
                         "commands": [{"osc": "/x", "forward": {"path": "/p", "args": args}}]});
    if v05 {
        doc.as_object_mut().unwrap().insert("$schema".into(), json!(SCHEMA));
    }
    doc
}

/// A device whose first reply emits `args`.
fn emit_args(args: Value) -> Value {
    json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"},
           "replies": [{"match_osc": ["/m/", cap("a")], "emit_osc": "/r", "emit_args": args}]})
}

fn good_args() -> Vec<Value> {
    vec![json!(1), json!(-3), json!(1.5), json!(true), json!(false), json!("text"), json!({"arg": "value"}), cap("n")]
}

fn bad_args() -> Vec<Value> {
    vec![
        json!({"x": 1}),
        json!({}),
        json!({"arg": "a", "capture": "b"}),
        json!({"arg": "1a"}),
        json!({"arg": "a-b"}),
        json!({"arg": 3}),
        json!({"capture": "a b"}),
        json!({"arg": "a", "extra": 1}),
        json!([1]),
        json!([]),
        json!(null),
        json!("x{n}"),
        json!("{n}"),
        json!("{"),
        json!("}"),
    ]
}

#[test]
fn criterion_2_and_8_forward_args_hold_literals_arg_objects_and_capture_objects() {
    accepted(&forward_args(Value::Array(good_args()), true));
    for a in good_args() {
        accepted(&forward_args(json!([a]), true));
    }
    accepted(&forward_args(json!([]), true));
}

#[test]
fn criterion_8_a_literal_stays_as_it_is_and_any_other_object_or_array_is_refused() {
    for bad in bad_args() {
        refused_at(&forward_args(json!([1, bad]), true), "/commands/0/forward/args/1", &KINDS);
    }
}

#[test]
fn criterion_2_a_subscription_forward_takes_the_same_args() {
    let doc = |args: Value| {
        json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"},
               "subscriptions": [{"on": "startup", "forward": {"path": "/s", "args": args}}]})
    };
    accepted(&doc(Value::Array(good_args())));
    refused_at(&doc(json!([{"x": 1}])), "/subscriptions/0/forward/args/0", &KINDS);
}

#[test]
fn criterion_2_emit_args_hold_literals_arg_objects_and_capture_objects() {
    accepted(&emit_args(Value::Array(good_args())));
    accepted(&emit_args(json!([])));
    // absent means no argument
    accepted(&json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}, "replies": [{"emit_osc": "/r"}]}));
}

#[test]
fn criterion_2_an_emit_arg_of_another_form_is_refused_at_the_argument() {
    for bad in bad_args() {
        refused_at(&emit_args(json!([1, bad])), "/replies/0/emit_args/1", &KINDS);
    }
    refused_at(&emit_args(json!("/x")), "/replies/0/emit_args", &KINDS);
}

#[test]
fn criterion_2_emit_osc_is_the_address_alone_without_space() {
    let doc = |a: Value| json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}, "replies": [{"emit_osc": a}]});
    refused_at(&doc(json!("/r {a}")), "/replies/0/emit_osc", &KINDS);
    refused_at(&doc(json!("/r a")), "/replies/0/emit_osc", &KINDS);
    accepted(&doc(json!("/r")));
}

// ---- criterion 3, 10: midi_in as a list ----

const MIDI_IN: [&str; 7] = ["note_on", "note_off", "cc", "pitchbend", "aftertouch", "poly_aftertouch", "program_change"];

fn midi_doc(midi_in: Value) -> Value {
    json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}, "midi_in": midi_in})
}

#[test]
fn criterion_10_midi_in_is_a_list_of_the_types_received() {
    accepted(&midi_doc(json!(["note_on", "cc"])));
    accepted(&midi_doc(json!(MIDI_IN)));
    for t in MIDI_IN {
        accepted(&midi_doc(json!([t])));
    }
}

#[test]
fn criterion_10_an_empty_midi_in_is_valid_and_reports_nothing() {
    accepted(&midi_doc(json!([])));
    let read = osd::read(&midi_doc(json!([])).to_string(), Kind::Device).unwrap();
    let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
    assert_eq!(back["midi_in"], json!([]), "an empty list stays an empty list");
    let none = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}});
    let read = osd::read(&none.to_string(), Kind::Device).unwrap();
    assert!(serde_json::from_str::<Value>(&osd::write(&read)).unwrap().get("midi_in").is_none());
}

#[test]
fn criterion_10_an_unknown_type_is_refused_at_the_type() {
    for bad in ["note", "Note_On", "sysex", "clock", "", "note/on", "/note/on {note} {velocity} {channel}"] {
        refused_at(&midi_doc(json!(["cc", bad])), "/midi_in/1", &KINDS);
    }
}

#[test]
fn criterion_10_a_duplicate_a_non_string_and_an_object_are_refused() {
    refused_at(&midi_doc(json!(["cc", "cc"])), "/midi_in", &KINDS);
    refused_at(&midi_doc(json!(["note_on", "cc", "note_on"])), "/midi_in", &KINDS);
    refused_at(&midi_doc(json!(["cc", 7])), "/midi_in/1", &KINDS);
    refused_at(&midi_doc(json!(["cc", null])), "/midi_in/1", &KINDS);
    refused_at(&midi_doc(json!(["cc", {"cc": "/cc/{num}"}])), "/midi_in/1", &KINDS);
    refused_at(&midi_doc(json!({"note_on": "/note/on {note} {velocity} {channel}"})), "/midi_in", &KINDS);
    refused_at(&midi_doc(json!({})), "/midi_in", &KINDS);
    refused_at(&midi_doc(json!("cc")), "/midi_in", &KINDS);
}

// ---- criteria 1, 6, 7, 9, 11: the converter ----

/// The seven canonical 0.4 templates of `midi_in`, in the order of the schema.
fn canonical() -> [(&'static str, &'static str); 7] {
    [
        ("note_on", "/note/on {note} {velocity} {channel}"),
        ("note_off", "/note/off {note} {velocity} {channel}"),
        ("cc", "/cc/{num} {value} {channel}"),
        ("pitchbend", "/pitchbend {value_u14} {channel}"),
        ("aftertouch", "/aftertouch {value} {channel}"),
        ("poly_aftertouch", "/poly_aftertouch {note} {value} {channel}"),
        ("program_change", "/program_change {program} {channel}"),
    ]
}

fn midi_04(entries: &[(&str, &str)]) -> Value {
    let mut m = serde_json::Map::new();
    for (k, v) in entries {
        m.insert((*k).into(), json!(v));
    }
    json!({"device": {"name": "T", "osc_prefix": "/t"}, "midi_in": Value::Object(m)})
}

#[test]
fn criterion_7_a_0_4_template_with_names_is_split_into_segments_and_the_rest_stays() {
    let cases = [
        ("/cc/{num}", json!(["/cc/", cap("num")])),
        ("/a/{x}/b/{y}", json!(["/a/", cap("x"), "/b/", cap("y")])),
        ("/a/{x}/b", json!(["/a/", cap("x"), "/b"])),
        ("/a/{x_1}", json!(["/a/", cap("x_1")])),
        ("/a/{_n}", json!(["/a/", cap("_n")])),
        ("/a/b/c/{x}", json!(["/a/b/c/", cap("x")])),
    ];
    for place in [Place::CommandOsc, Place::MatchOsc] {
        for (old, new) in &cases {
            let out = convert(&place.device(json!(old), false)).unwrap_or_else(|f| panic!("{place:?} {old}: {f}"));
            assert_eq!(out.pointer(place.pointer()).unwrap(), new, "{place:?} {old}");
        }
    }
    // an address without brace is left as it is, at each place
    for place in PLACES {
        let out = convert(&place.device(json!("/a/b"), false)).unwrap();
        assert_eq!(out.pointer(place.pointer()).unwrap(), &json!("/a/b"), "{place:?}");
    }
}

#[test]
fn criterion_7_forward_path_and_emit_osc_of_0_4_are_split_too_and_may_hold_adjacent_captures() {
    for place in [Place::ForwardPath, Place::EmitOsc] {
        let out = convert(&place.device(json!("/a/{x}/b"), false)).unwrap();
        assert_eq!(out.pointer(place.pointer()).unwrap(), &json!(["/a/", cap("x"), "/b"]), "{place:?}");
        let out = convert(&place.device(json!("/a/{x}{y}"), false)).unwrap();
        assert_eq!(out.pointer(place.pointer()).unwrap(), &json!(["/a/", cap("x"), cap("y")]), "{place:?}");
        let out = convert(&place.device(json!("{x}/a"), false)).unwrap();
        assert_eq!(out.pointer(place.pointer()).unwrap(), &json!([cap("x"), "/a"]), "{place:?}");
    }
}

#[test]
fn criterion_7_a_subscription_forward_path_of_0_4_is_split() {
    let doc = json!({"device": {"name": "T", "osc_prefix": "/t"},
                     "subscriptions": [{"on": "startup", "forward": {"path": "/s/{n}", "args": ["{v}"]}}]});
    let out = convert(&doc).unwrap();
    assert_eq!(out["subscriptions"][0]["forward"], json!({"path": ["/s/", cap("n")], "args": [{"arg": "v"}]}));
}

#[test]
fn criterion_7_the_converted_address_reads_validates_and_converts_to_itself() {
    let schemas = common::Schemas::load("0.5");
    for place in PLACES {
        let out = convert(&place.device(json!("/a/{x}/b/{y}"), false)).unwrap();
        assert!(out.pointer(place.pointer()).unwrap().is_array(), "{place:?}: not segments");
        osd::read(&out.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{place:?}: {f}"));
        assert_eq!(schemas.check(Kind::Device, &out), None, "{place:?}");
        assert_eq!(convert(&out).unwrap(), out, "{place:?}");
    }
}

#[test]
fn criterion_7_a_brace_that_is_not_one_name_is_a_fault_that_names_the_address_and_the_pointer() {
    let bad = [
        "/x/{n", "/x/{", "/x/{}", "/x/{{n}}", "/x/{a{b}}", "/x/}", "/x/n}", "/x/}{n}", "/x/{a,b}", "/x/{a b}", "/x/{1a}",
        "/x/{a-b}", "/x/{a:u7}",
    ];
    for place in PLACES {
        // an emit_osc is cut at its first space: the arguments have their own test
        for old in bad.into_iter().filter(|b| !(matches!(place, Place::EmitOsc) && b.contains(' '))) {
            let f = not_convertible(&place.device(json!(old), false), place.pointer());
            assert!(f.to_string().contains(old), "{place:?} {old}: the fault does not name the address: {f}");
        }
    }
}

#[test]
fn criterion_6_and_7_two_adjacent_0_4_captures_are_a_fault_where_the_address_is_recognised() {
    for place in PLACES.into_iter().filter(|p| p.recognises()) {
        not_convertible(&place.device(json!("/x/{a}{b}"), false), place.pointer());
    }
}

#[test]
fn criterion_7_the_fault_of_an_address_in_a_later_element_names_that_element() {
    let doc = json!({"device": {"name": "T", "osc_prefix": "/t"},
                     "commands": [{"osc": "/ok/{a}"}, {"osc": "/x"}, {"osc": "/bad/{"}]});
    not_convertible(&doc, "/commands/2/osc");
    let doc = json!({"device": {"name": "T", "osc_prefix": "/t"},
                     "replies": [{"emit_osc": "/r"}, {"emit_osc": "/r/{}"}]});
    not_convertible(&doc, "/replies/1/emit_osc");
}

#[test]
fn criterion_9_a_0_4_forward_arg_that_is_a_name_is_an_arg_object_and_a_literal_stays() {
    let old = json!(["{value}", 5, 1.5, true, "text", "{n_2}", 0]);
    let out = convert(&forward_args(old, false)).unwrap();
    assert_eq!(
        out["commands"][0]["forward"]["args"],
        json!([{"arg": "value"}, 5, 1.5, true, "text", {"arg": "n_2"}, 0])
    );
    let out = convert(&forward_args(json!([]), false)).unwrap();
    assert_eq!(out["commands"][0]["forward"]["args"], json!([]));
}

#[test]
fn criterion_9_a_mixed_or_malformed_forward_arg_is_a_fault_at_the_argument() {
    for bad in ["x{n}", "{n}x", "{n}{m}", "{", "}", "{}", "{a,b}", "{a b}", "{n", "n}", "{1a}"] {
        let f = not_convertible(&forward_args(json!([1, bad]), false), "/commands/0/forward/args/1");
        assert!(f.to_string().contains(bad), "{bad}: {f}");
    }
}

#[test]
fn criterion_9_the_0_4_arguments_of_emit_osc_follow_the_first_space_and_become_capture_objects() {
    let reply = |emit: &str| json!({"device": {"name": "T", "osc_prefix": "/t"}, "replies": [{"match_osc": "/m", "emit_osc": emit}]});
    let out = convert(&reply("/r/{id} {value} {x}")).unwrap();
    assert_eq!(out["replies"][0]["emit_osc"], json!(["/r/", cap("id")]));
    assert_eq!(out["replies"][0]["emit_args"], json!([cap("value"), cap("x")]));
    let out = convert(&reply("/r {a}")).unwrap();
    assert_eq!(out["replies"][0]["emit_osc"], json!("/r"));
    assert_eq!(out["replies"][0]["emit_args"], json!([cap("a")]));
    let out = convert(&reply("/r/{id}")).unwrap();
    assert_eq!(out["replies"][0]["emit_osc"], json!(["/r/", cap("id")]));
    assert!(out["replies"][0].get("emit_args").is_none(), "no argument: no emit_args");
    let out = convert(&reply("/r")).unwrap();
    assert_eq!(out["replies"][0]["emit_osc"], json!("/r"));
    assert!(out["replies"][0].get("emit_args").is_none());
    assert_eq!(out["replies"][0]["match_osc"], json!("/m"));
}

#[test]
fn criterion_9_converted_replies_read_validate_and_convert_to_themselves() {
    let schemas = common::Schemas::load("0.5");
    let doc = json!({"device": {"name": "T", "osc_prefix": "/t"},
                     "replies": [{"match_osc": "/m/{a}", "emit_osc": "/r/{id} {value} {x}"}]});
    let out = convert(&doc).unwrap();
    assert!(out["replies"][0]["emit_osc"].is_array() && out["replies"][0]["match_osc"].is_array());
    assert!(out["replies"][0]["emit_args"].is_array());
    osd::read(&out.to_string(), Kind::Device).unwrap();
    assert_eq!(schemas.check(Kind::Device, &out), None);
    assert_eq!(convert(&out).unwrap(), out);
}

#[test]
fn criterion_9_an_emit_osc_argument_of_another_form_is_a_fault_at_the_reply() {
    let reply = |emit: &str| json!({"device": {"name": "T", "osc_prefix": "/t"}, "replies": [{"emit_osc": emit}]});
    for bad in ["/r {a} 5", "/r 5", "/r x{a}", "/r {a}x", "/r {a,b}", "/r {a", "/r {}", "/r text", "/r {a}{b}"] {
        let f = not_convertible(&reply(bad), "/replies/0/emit_osc");
        assert!(f.to_string().contains(bad), "{bad}: {f}");
    }
}

#[test]
fn criterion_11_each_canonical_0_4_midi_in_template_becomes_its_type() {
    for (t, template) in canonical() {
        let out = convert(&midi_04(&[(t, template)])).unwrap_or_else(|f| panic!("{t}: {f}"));
        assert_eq!(out["midi_in"], json!([t]));
    }
}

#[test]
fn criterion_11_the_types_come_out_in_the_order_of_the_schema() {
    let mut entries = canonical();
    entries.reverse();
    let text = {
        // written in the reverse order, as text, so no map reorders it
        let body: Vec<String> = entries.iter().map(|(k, v)| format!("{}: {}", json!(k), json!(v))).collect();
        format!("{{\"device\": {{\"name\": \"T\", \"osc_prefix\": \"/t\"}}, \"midi_in\": {{{}}}}}", body.join(", "))
    };
    let out: Value = serde_json::from_str(&osd::convert(&text, Kind::Device).unwrap()).unwrap();
    assert_eq!(out["midi_in"], json!(MIDI_IN));
    let out = convert(&midi_04(&[("cc", canonical()[2].1), ("note_on", canonical()[0].1)])).unwrap();
    assert_eq!(out["midi_in"], json!(["note_on", "cc"]));
}

#[test]
fn criterion_11_an_empty_0_4_midi_in_is_an_empty_list_and_an_absent_one_stays_absent() {
    let out = convert(&midi_04(&[])).unwrap();
    assert_eq!(out["midi_in"], json!([]));
    let out = convert(&json!({"device": {"name": "T", "osc_prefix": "/t"}})).unwrap();
    assert!(out.get("midi_in").is_none());
}

#[test]
fn criterion_11_the_converted_midi_in_reads_validates_and_converts_to_itself() {
    let schemas = common::Schemas::load("0.5");
    for doc in [midi_04(&canonical()), midi_04(&[]), midi_04(&[("cc", "/cc/{num} {value} {channel}")])] {
        let out = convert(&doc).unwrap();
        assert!(out["midi_in"].is_array(), "{doc}: midi_in is not a list");
        osd::read(&out.to_string(), Kind::Device).unwrap();
        assert_eq!(schemas.check(Kind::Device, &out), None, "{doc}");
        assert_eq!(convert(&out).unwrap(), out);
    }
}

#[test]
fn criterion_3_and_11_a_template_other_than_the_canonical_one_is_a_fault_that_names_the_type_and_the_template() {
    let bad = [
        ("note_on", "/note/on {velocity} {note} {channel}"),
        ("note_on", "/note/on {note} {velocity}"),
        ("note_on", "/note/on {note} {velocity} {channel} {extra}"),
        ("note_on", "/midi/note"),
        ("note_on", "/note/off {note} {velocity} {channel}"),
        ("note_off", "/note/on {note} {velocity} {channel}"),
        ("cc", "/cc/{number} {value} {channel}"),
        ("cc", "/cc/{num} {value}"),
        ("cc", "/cc {num} {value} {channel}"),
        ("pitchbend", "/pitchbend {value} {channel}"),
        ("aftertouch", "/aftertouch {value}"),
        ("poly_aftertouch", "/poly_aftertouch {value} {note} {channel}"),
        ("program_change", "/program_change {program}"),
        ("program_change", ""),
        ("program_change", "/program_change  {program}  {channel}"),
    ];
    for (t, template) in bad {
        let f = not_convertible(&midi_04(&[("cc", canonical()[2].1), (t, template)]), &format!("/midi_in/{t}"));
        let said = f.to_string();
        assert!(said.contains(t) && said.contains(template), "{t} {template:?}: {said}");
    }
}

#[test]
fn criterion_3_the_canonical_table_is_written_in_the_schema_and_in_the_interface() {
    let schema = std::fs::read_to_string(common::repo().join("schemas/0.5/device.schema.json")).unwrap();
    let interface = std::fs::read_to_string(common::repo().join("docs/INTERFACE.md")).unwrap();
    let doc: Value = serde_json::from_str(&schema).unwrap();
    let described = doc["properties"]["midi_in"]["description"].as_str().unwrap_or_default().to_string()
        + &doc["properties"]["midi_in"].to_string();
    for (t, template) in canonical() {
        assert!(described.contains(template), "schema: midi_in does not say {t} {template}");
        assert!(interface.contains(template), "docs/INTERFACE.md does not say {t} {template}");
    }
}

// ---- criterion 13: 0.4 is untouched ----

#[test]
fn criterion_13_a_0_4_descriptor_still_reads_its_string_templates_and_its_midi_in_object() {
    let doc = json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "midi_in": {"note_on": "/note/on {note} {velocity} {channel}", "cc": "/cc/{num} {value} {channel}"},
        "commands": [{"osc": "/x/{n}", "forward": {"path": "/p/{n}", "args": ["{value}", 1]}}],
        "replies": [{"match_osc": "/m/{a}", "emit_osc": "/r/{a} {b}"}]
    });
    let read = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&read)).unwrap(), doc);
    assert_eq!(common::Schemas::load("0.4").check(Kind::Device, &doc), None);
}

#[test]
fn criterion_13_the_0_5_schema_refuses_the_0_4_forms() {
    let schemas = common::Schemas::load("0.5");
    let old = |mut d: Value| {
        d.as_object_mut().unwrap().insert("$schema".into(), json!(SCHEMA));
        d
    };
    let base = json!({"device": {"name": "T", "osc_prefix": "/t"}});
    let with = |k: &str, v: Value| {
        let mut d = base.clone();
        d.as_object_mut().unwrap().insert(k.into(), v);
        old(d)
    };
    for doc in [
        with("midi_in", json!({"cc": "/cc/{num} {value} {channel}"})),
        with("commands", json!([{"osc": "/x/{n}"}])),
        with("commands", json!([{"osc": "/x", "forward": {"path": "/p/{n}"}}])),
        with("commands", json!([{"osc": "/x", "forward": {"path": "/p", "args": ["{n}"]}}])),
        with("replies", json!([{"match_osc": "/m/{a}", "emit_osc": "/r"}])),
        with("replies", json!([{"emit_osc": "/r/{a} {b}"}])),
    ] {
        assert!(schemas.check(Kind::Device, &doc).is_some(), "the 0.5 schema accepts {doc}");
        assert!(osd::read(&doc.to_string(), Kind::Device).is_err(), "the reader accepts {doc}");
    }
}
