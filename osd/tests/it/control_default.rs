//! The default value of a control (0.6): an optional integer `default` on a CC entry and on a
//! param entry, the raw value of the control on an initialised sound, in the space of `range`.
//! Without `range` it is compared to nothing; with `range` its bounds are included. `osd`
//! reads it, writes it back and refuses a default outside the range, at the default's address.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

/// A 0.6 device whose one CC entry is `entry`.
fn with_cc(entry: Value) -> Value {
    json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [entry]}
    })
}

/// A 0.6 device whose one param entry is `entry`.
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

fn cc_default(doc: &Value) -> Option<i64> {
    device(doc).cc_params.unwrap().entries[0].default
}

fn param_default(doc: &Value) -> Option<i64> {
    device(doc).params.unwrap().entries[0].default
}

const CC: &str = "/cc_params/entries/0/default";
const PARAM: &str = "/params/entries/0/default";

#[test]
fn a_cc_entry_gives_its_default_and_the_reader_returns_it() {
    assert_eq!(cc_default(&with_cc(json!({"osc": "/a", "cc": 74, "range": [0, 127], "default": 64}))), Some(64));
}

#[test]
fn a_param_entry_gives_its_default_and_the_reader_returns_it() {
    assert_eq!(param_default(&with_param(json!({"osc": "/a", "id": 3, "range": [0, 127], "default": 10}))), Some(10));
}

#[test]
fn a_control_without_a_default_has_none_and_writes_none() {
    let doc = with_cc(json!({"osc": "/a", "cc": 74, "range": [0, 127]}));
    assert_eq!(cc_default(&doc), None);
    let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert!(out["cc_params"]["entries"][0].get("default").is_none());
    assert_eq!(param_default(&with_param(json!({"osc": "/a", "id": 3}))), None);
}

#[test]
fn a_default_is_written_back_where_it_was_read() {
    for doc in [
        with_cc(json!({"osc": "/a", "cc": 74, "range": [0, 127], "default": 64})),
        with_param(json!({"osc": "/a", "id": 3, "range": [0, 127], "default": 64})),
    ] {
        let out: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
        assert_eq!(out, doc);
    }
}

#[test]
fn a_default_without_a_range_is_compared_to_nothing() {
    for default in [0, -5, 100_000] {
        assert_eq!(cc_default(&with_cc(json!({"osc": "/a", "cc": 74, "default": default}))), Some(default));
        assert_eq!(param_default(&with_param(json!({"osc": "/a", "id": 3, "default": default}))), Some(default));
    }
}

#[test]
fn a_default_equal_to_a_bound_of_the_range_is_accepted() {
    for default in [10, 20] {
        assert_eq!(cc_default(&with_cc(json!({"osc": "/a", "cc": 74, "range": [10, 20], "default": default}))), Some(default));
        assert_eq!(param_default(&with_param(json!({"osc": "/a", "id": 3, "range": [10, 20], "default": default}))), Some(default));
    }
}

#[test]
fn a_default_outside_the_range_is_refused_at_the_default() {
    for default in [9, 21, -1, 128] {
        refused(&with_cc(json!({"osc": "/a", "cc": 74, "range": [10, 20], "default": default})), FaultKind::BadValue, CC);
        refused(&with_param(json!({"osc": "/a", "id": 3, "range": [10, 20], "default": default})), FaultKind::BadValue, PARAM);
    }
}

#[test]
fn the_default_of_a_later_entry_is_refused_at_its_own_address() {
    let doc = json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [
            {"osc": "/a", "cc": 1, "range": [0, 127], "default": 5},
            {"osc": "/b", "cc": 2, "range": [0, 127], "default": 200}
        ]}
    });
    refused(&doc, FaultKind::BadValue, "/cc_params/entries/1/default");
}

#[test]
fn a_14_bit_control_takes_its_default_in_the_14_bits() {
    let entry = |default: i64| json!({"osc": "/a", "cc": 1, "cc_lsb": 33, "range": [0, 16383], "default": default});
    assert_eq!(cc_default(&with_cc(entry(8192))), Some(8192));
    assert_eq!(cc_default(&with_cc(entry(16383))), Some(16383));
    refused(&with_cc(entry(16384)), FaultKind::BadValue, CC);
}

#[test]
fn a_centered_control_compares_its_default_to_the_range_as_it_is() {
    let entry = |default: i64| json!({"osc": "/a", "cc": 1, "range": [0, 127], "orientation": "centered", "default": default});
    assert_eq!(cc_default(&with_cc(entry(64))), Some(64));
    assert_eq!(cc_default(&with_cc(entry(0))), Some(0));
    refused(&with_cc(entry(-64)), FaultKind::BadValue, CC);
    let signed = json!({"osc": "/a", "cc": 1, "range": [-64, 63], "orientation": "centered", "default": -64});
    assert_eq!(cc_default(&with_cc(signed)), Some(-64));
}

#[test]
fn a_default_that_is_not_an_integer_is_refused_at_the_default() {
    for bad in [json!("64"), json!(64.5), json!(null), json!(true), json!([64]), json!({"v": 64})] {
        refused(&with_cc(json!({"osc": "/a", "cc": 74, "default": bad})), FaultKind::BadValue, CC);
        refused(&with_param(json!({"osc": "/a", "id": 3, "default": bad})), FaultKind::BadValue, PARAM);
    }
}

#[test]
fn the_0_6_schema_accepts_a_default_on_both_entries_and_refuses_one_that_is_not_an_integer() {
    let schemas = common::Schemas::load("0.6");
    let good = [
        with_cc(json!({"osc": "/a", "cc": 74, "range": [0, 127], "default": 64})),
        with_param(json!({"osc": "/a", "id": 3, "default": 64})),
    ];
    for doc in good {
        assert!(schemas.errors(Kind::Device, &doc).is_empty(), "{doc}");
    }
    let bad = with_cc(json!({"osc": "/a", "cc": 74, "default": "64"}));
    let errors = schemas.errors(Kind::Device, &bad);
    assert!(errors.iter().any(|(at, _)| at == CC), "{errors:?}");
}

#[test]
fn a_0_5_descriptor_does_not_know_the_default() {
    let doc = with_cc(json!({"osc": "/a", "cc": 74, "default": 64}));
    let text = doc.to_string().replace("osd/0.6/", "osd/0.5/");
    let f = osd::read(&text, Kind::Device).expect_err("0.5 has no default");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, CC), "{f}");
}

#[test]
fn converting_a_0_5_descriptor_invents_no_default() {
    let text = std::fs::read_to_string(common::repo().join("examples/0.5/synth.device.json")).unwrap();
    let outs = [
        osd::convert(&text, Kind::Device).unwrap(),
        osd::convert_to(&text, Kind::Device, Version::V0_6).unwrap(),
    ];
    for out in outs {
        let v: Value = serde_json::from_str(&out).unwrap();
        let mut seen = 0;
        for (block, key) in [("cc_params", "entries"), ("params", "entries")] {
            for e in v[block][key].as_array().into_iter().flatten() {
                seen += 1;
                assert!(e.get("default").is_none(), "{e}");
            }
        }
        assert!(seen > 0, "the example has no control");
    }
}

/// The conformance cases of the default: `(case, the fault's kind, whether only the reader refuses it)`.
const CASES: [(&str, &str, bool); 3] = [
    ("default-given", "", false),
    ("default-none", "", false),
    ("default-out-of-range-refused", "BadValue", true),
];

#[test]
fn the_conformance_cases_show_a_default_none_and_one_out_of_range_refused() {
    let root = common::repo().join("conformance/0.6/read");
    let mut failures = Vec::new();
    for (name, kind, reader_only) in CASES {
        let dir = root.join(name);
        let read_file = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap_or_else(|e| panic!("{name}/{f}: {e}"));
        let case: Value = serde_json::from_str(&read_file("case.json")).unwrap();
        let doc: Value = serde_json::from_str(&read_file("descriptor.json")).unwrap();
        if (case.get("schema_accepts") == Some(&json!(true))) != reader_only {
            failures.push(format!("{name}: schema_accepts is not {reader_only}"));
        }
        let entry = &doc["cc_params"]["entries"][0];
        let shows = match name {
            "default-given" => entry["default"].is_i64() && entry["range"].is_array(),
            "default-none" => entry.get("default").is_none(),
            _ => entry["default"].as_i64().is_some_and(|d| d < entry["range"][0].as_i64().unwrap() || d > entry["range"][1].as_i64().unwrap()),
        };
        if !shows {
            failures.push(format!("{name}: the descriptor does not show its case"));
        }
        if kind.is_empty() {
            if case.get("fault").is_some() || case.get("result").is_none() {
                failures.push(format!("{name}: states no result"));
            }
        } else if case["fault"] != json!({"kind": kind, "pointer": CC}) {
            failures.push(format!("{name}: states {}", case["fault"]));
        }
    }
    common::report("default cases fail", failures);
}
