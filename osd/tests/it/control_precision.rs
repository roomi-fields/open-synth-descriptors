//! A control at the precision of MIDI 2.0 (0.6): optional `priority` (1 to 5), `step_count`
//! (2 or more), `transmit` and `recognize` (absolute, relative, both, none) on a CC entry and
//! on a param entry, and the named `values` on a CC entry (as on a param entry), each label
//! inside the entry's own `range`. `osd` reads them, writes them back and refuses a fault at
//! the pointer of the faulty field.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn with_cc(entry: Value) -> Value {
    json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [entry]}
    })
}

fn with_param(entry: Value) -> Value {
    json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [entry]}
    })
}

fn read(doc: &Value) -> Result<Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn device(doc: &Value) -> osd::Device {
    match read(doc).unwrap_or_else(|f| panic!("{f}")) {
        Descriptor::Device(d) => *d,
        _ => panic!("a device"),
    }
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

fn word<T: std::fmt::Debug>(w: Option<T>) -> Option<String> {
    w.map(|w| format!("{w:?}").trim_matches('"').to_lowercase())
}

/// Both entry types, built from the same fields.
fn both(fields: Value) -> [(Value, &'static str); 2] {
    let mut cc = json!({"osc": "/a", "cc": 74});
    let mut param = json!({"osc": "/a", "id": 3});
    for (k, v) in fields.as_object().unwrap() {
        cc[k] = v.clone();
        param[k] = v.clone();
    }
    [(with_cc(cc), "/cc_params/entries/0"), (with_param(param), "/params/entries/0")]
}

#[test]
fn a_control_gives_its_priority_and_the_reader_returns_it() {
    for p in 1..=5 {
        let [(cc, _), (param, _)] = both(json!({"priority": p}));
        assert_eq!(device(&cc).cc_params.unwrap().entries[0].priority.map(i64::from), Some(p));
        assert_eq!(device(&param).params.unwrap().entries[0].priority.map(i64::from), Some(p));
    }
}

#[test]
fn a_control_gives_its_step_count_and_the_reader_returns_it() {
    for n in [2, 3, 128, 16384] {
        let [(cc, _), (param, _)] = both(json!({"step_count": n}));
        assert_eq!(device(&cc).cc_params.unwrap().entries[0].step_count, Some(n));
        assert_eq!(device(&param).params.unwrap().entries[0].step_count, Some(n));
    }
}

#[test]
fn a_control_gives_how_it_is_sent_and_recognised_and_the_reader_returns_it() {
    for w in ["absolute", "relative", "both", "none"] {
        let [(cc, _), (param, _)] = both(json!({"transmit": w, "recognize": w}));
        let way = device(&cc).cc_params.unwrap().entries[0].ways().remove(0);
        assert_eq!((word(way.transmit.as_ref()), word(way.recognize.as_ref())), (Some(w.into()), Some(w.into())));
        let way = device(&param).params.unwrap().entries[0].ways().remove(0);
        assert_eq!((word(way.transmit.as_ref()), word(way.recognize.as_ref())), (Some(w.into()), Some(w.into())));
    }
    let [(cc, _), _] = both(json!({"transmit": "relative"}));
    let way = device(&cc).cc_params.unwrap().entries[0].ways().remove(0);
    assert_eq!((word(way.transmit.as_ref()), word(way.recognize.as_ref())), (Some("relative".into()), None));
}

#[test]
fn a_cc_entry_gives_named_values_and_the_reader_returns_them() {
    let doc = with_cc(json!({"osc": "/a", "cc": 74, "range": [0, 3], "expects": "discrete", "values": {"off": 0, "low": 1, "high": 3}}));
    let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert_eq!(out["cc_params"]["entries"][0]["values"], json!({"off": 0, "low": 1, "high": 3}));
}

#[test]
fn the_added_fields_are_optional_and_written_back_where_they_were_read() {
    let bare = with_cc(json!({"osc": "/a", "cc": 74, "range": [0, 127]}));
    let out: Value = serde_json::from_str(&osd::write(&read(&bare).unwrap())).unwrap();
    for k in ["priority", "step_count", "transmit", "recognize", "values"] {
        assert!(out["cc_params"]["entries"][0].get(k).is_none(), "{k}");
    }
    let full = json!({"range": [0, 127], "priority": 2, "step_count": 8, "transmit": "both", "recognize": "none"});
    for (doc, _) in both(full) {
        let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
        assert_eq!(out, doc);
    }
    let named = with_cc(json!({"osc": "/a", "cc": 74, "range": [0, 3], "values": {"off": 0, "on": 3}}));
    let out: Value = serde_json::from_str(&osd::write(&read(&named).unwrap())).unwrap();
    assert_eq!(out, named);
}

#[test]
fn a_priority_outside_one_to_five_or_not_an_integer_is_refused_at_the_priority() {
    for bad in [json!(0), json!(6), json!(-1), json!(100), json!("2"), json!(2.5), json!(null), json!(true)] {
        for (doc, at) in both(json!({"priority": bad})) {
            refused(&doc, FaultKind::BadValue, &format!("{at}/priority"));
        }
    }
}

#[test]
fn a_step_count_below_two_or_not_an_integer_is_refused_at_the_step_count() {
    for bad in [json!(1), json!(0), json!(-3), json!(2.5), json!("8"), json!(null), json!(false)] {
        for (doc, at) in both(json!({"step_count": bad})) {
            refused(&doc, FaultKind::BadValue, &format!("{at}/step_count"));
        }
    }
}

#[test]
fn a_transmit_or_recognize_word_outside_the_four_is_refused_at_the_field() {
    for bad in [json!("relatif"), json!("Absolute"), json!(""), json!(1), json!(null), json!(["both"])] {
        for field in ["transmit", "recognize"] {
            for (doc, at) in both(json!({ field: bad })) {
                refused(&doc, FaultKind::BadValue, &format!("{at}/{field}"));
            }
        }
    }
}

#[test]
fn a_named_value_outside_the_range_is_refused_at_its_label_and_bounds_are_included() {
    for (doc, at) in [
        (with_cc(json!({"osc": "/a", "cc": 74, "range": [10, 20], "values": {"lo": 10, "hi": 20}})), "/cc_params/entries/0"),
        (with_param(json!({"osc": "/a", "id": 3, "range": [10, 20], "values": {"lo": 10, "hi": 20}})), "/params/entries/0"),
    ] {
        assert!(read(&doc).is_ok(), "{at}");
    }
    for out in [9, 21, -1, 128] {
        refused(
            &with_cc(json!({"osc": "/a", "cc": 74, "range": [10, 20], "values": {"fine": 15, "wild": out}})),
            FaultKind::BadValue,
            "/cc_params/entries/0/values/wild",
        );
        refused(
            &with_param(json!({"osc": "/a", "id": 3, "range": [10, 20], "values": {"wild": out, "fine": 15}})),
            FaultKind::BadValue,
            "/params/entries/0/values/wild",
        );
    }
}

#[test]
fn named_values_without_a_range_are_compared_to_nothing() {
    for v in [-5, 0, 100_000] {
        let values = device(&with_cc(json!({"osc": "/a", "cc": 74, "values": {"x": v}}))).cc_params.unwrap().entries[0].values.clone().unwrap();
        assert_eq!(values["x"], v);
        assert!(read(&with_param(json!({"osc": "/a", "id": 3, "values": {"x": v}}))).is_ok());
    }
}

#[test]
fn the_named_value_of_a_later_entry_is_refused_at_its_own_address() {
    let doc = json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [
            {"osc": "/a", "cc": 1, "range": [0, 127], "values": {"ok": 5}},
            {"osc": "/b", "cc": 2, "range": [0, 3], "values": {"ok": 1, "bad": 4}}
        ]}
    });
    refused(&doc, FaultKind::BadValue, "/cc_params/entries/1/values/bad");
}

#[test]
fn the_0_6_schema_accepts_the_added_fields_and_refuses_each_fault_at_its_field() {
    let schemas = common::Schemas::load("0.6");
    let good = json!({"range": [0, 3], "priority": 1, "step_count": 4, "transmit": "absolute", "recognize": "both"});
    for (doc, _) in both(good) {
        assert!(schemas.errors(Kind::Device, &doc).is_empty(), "{doc}");
    }
    let cc_values = with_cc(json!({"osc": "/a", "cc": 74, "values": {"off": 0}}));
    assert!(schemas.errors(Kind::Device, &cc_values).is_empty());
    for (fields, field) in [
        (json!({"priority": 0}), "priority"),
        (json!({"priority": 6}), "priority"),
        (json!({"step_count": 1}), "step_count"),
        (json!({"transmit": "sometimes"}), "transmit"),
        (json!({"recognize": "sometimes"}), "recognize"),
    ] {
        for (doc, at) in both(fields) {
            let errors = schemas.errors(Kind::Device, &doc);
            assert!(errors.iter().any(|(p, _)| *p == format!("{at}/{field}")), "{doc}: {errors:?}");
        }
    }
    // A value outside the range is the reader's: the schema accepts it.
    let outside = with_cc(json!({"osc": "/a", "cc": 74, "range": [0, 3], "values": {"x": 9}}));
    assert!(schemas.errors(Kind::Device, &outside).is_empty());
}

#[test]
fn before_0_6_the_added_fields_are_unknown() {
    for version in ["0.4", "0.5"] {
        for (field, value) in [("priority", json!(2)), ("step_count", json!(4)), ("transmit", json!("both")), ("recognize", json!("both"))] {
            let [(cc, cc_at), (param, param_at)] = both(json!({ field: value }));
            for (doc, at) in [(cc, cc_at), (param, param_at)] {
                let address = if version == "0.4" { "https://synthhub.roomi-fields.com/osd/0.4/" } else { "https://w3id.org/osd/0.5/" };
                let mut doc = doc;
                if version == "0.4" {
                    doc.as_object_mut().unwrap().remove("$schema");
                }
                let text = doc.to_string().replace("https://w3id.org/osd/0.6/", address);
                let f = osd::read(&text, Kind::Device).expect_err("unknown before 0.6");
                assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, format!("{at}/{field}").as_str()), "{version}: {f}");
            }
        }
    }
}

#[test]
fn a_0_5_param_entry_keeps_its_values_without_a_range_check() {
    let doc = with_param(json!({"osc": "/a", "id": 3, "range": [0, 3], "values": {"wild": 99}}));
    let text = doc.to_string().replace("osd/0.6/", "osd/0.5/");
    let d = osd::read(&text, Kind::Device).expect("0.5 reads it");
    let out: Value = serde_json::from_str(&osd::write(&d)).unwrap();
    assert_eq!(out["params"]["entries"][0]["values"]["wild"], 99);
}

#[test]
fn converting_a_0_5_descriptor_invents_none_of_the_added_fields() {
    let text = std::fs::read_to_string(common::repo().join("examples/0.5/synth.device.json")).unwrap();
    let outs = [osd::convert(&text, Kind::Device).unwrap(), osd::convert_to(&text, Kind::Device, Version::V0_6).unwrap()];
    for out in outs {
        let v: Value = serde_json::from_str(&out).unwrap();
        let mut seen = 0;
        for block in ["cc_params", "params"] {
            for e in v[block]["entries"].as_array().into_iter().flatten() {
                seen += 1;
                for k in ["priority", "step_count", "transmit", "recognize"] {
                    assert!(e.get(k).is_none(), "{k} in {e}");
                }
                if block == "cc_params" {
                    assert!(e.get("values").is_none(), "{e}");
                }
            }
        }
        assert!(seen > 0, "the example has no control");
    }
}

/// The conformance cases of the added fields: `(case, field, the fault's kind (empty: accepted), reader only)`.
const CASES: [(&str, &str, &str, bool); 8] = [
    ("priority-given", "priority", "", false),
    ("step-count-given", "step_count", "", false),
    ("transmit-recognize-given", "transmit", "", false),
    ("values-given", "values", "", false),
    ("priority-out-of-range-refused", "priority", "BadValue", false),
    ("step-count-below-two-refused", "step_count", "BadValue", false),
    ("transmit-word-refused", "transmit", "BadValue", false),
    ("value-out-of-range-refused", "values/wild", "BadValue", true),
];

#[test]
fn the_conformance_cases_show_each_added_field_accepted_and_each_refusal() {
    let root = common::repo().join("conformance/0.6/read");
    let mut failures = Vec::new();
    for (name, field, kind, reader_only) in CASES {
        let dir = root.join(name);
        let read_file = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap_or_else(|e| panic!("{name}/{f}: {e}"));
        let case: Value = serde_json::from_str(&read_file("case.json")).unwrap();
        let doc: Value = serde_json::from_str(&read_file("descriptor.json")).unwrap();
        let entry = &doc["cc_params"]["entries"][0];
        let top = field.split('/').next().unwrap();
        if entry.get(top).is_none() {
            failures.push(format!("{name}: the descriptor does not show {top}"));
        }
        if (case.get("schema_accepts") == Some(&json!(true))) != reader_only {
            failures.push(format!("{name}: schema_accepts is not {reader_only}"));
        }
        if kind.is_empty() {
            if case.get("fault").is_some() || case.get("result").is_none() {
                failures.push(format!("{name}: states no result"));
            }
        } else if case["fault"] != json!({"kind": kind, "pointer": format!("/cc_params/entries/0/{field}")}) {
            failures.push(format!("{name}: states {}", case["fault"]));
        }
    }
    common::report("added-field cases fail", failures);
}
