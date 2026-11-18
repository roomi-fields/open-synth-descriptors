//! A bank says whether it can be written (`writable`, absent = it can); a dump says how long
//! the device may take to answer its request (`reply_timeout_ms`), never less than the pause
//! `delay_ms`. `osd::convert` from 0.5 invents neither.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

fn device(programs: Value) -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "programs": programs,
    })
}

fn banks(bank: Value) -> Value {
    device(json!({"banks": [bank]}))
}

fn dump(delay_ms: Option<i64>, timeout: Value) -> Value {
    let mut p = json!({"dump": {"kind": "request", "request": [240, 1, 247], "reply_timeout_ms": timeout}});
    if let Some(d) = delay_ms {
        p["delay_ms"] = json!(d);
    }
    device(p)
}

/// The document read and written back, or the fault's kind and pointer.
fn read(doc: &Value) -> Result<Value, (FaultKind, String)> {
    let d = osd::read(&doc.to_string(), Kind::Device).map_err(|f| (f.kind, f.pointer))?;
    Ok(serde_json::from_str(&osd::write(&d)).unwrap())
}

fn schema_accepts(doc: &Value) -> bool {
    common::Schemas::load("0.6").check(Kind::Device, doc).is_none()
}

#[test]
fn a_bank_that_cannot_be_written_says_writable_false_and_reads_back_so() {
    let doc = banks(json!({"name": "Internal", "size": 64, "writable": false}));
    assert!(schema_accepts(&doc));
    assert_eq!(read(&doc).unwrap(), doc);
}

#[test]
fn a_bank_that_says_writable_true_reads_back_so() {
    let doc = banks(json!({"name": "Card", "size": 64, "writable": true}));
    assert!(schema_accepts(&doc));
    assert_eq!(read(&doc).unwrap(), doc);
}

#[test]
fn a_bank_that_says_nothing_of_writable_reads_as_before_without_the_field() {
    let doc = banks(json!({"name": "User", "size": 128}));
    let back = read(&doc).unwrap();
    assert_eq!(back, doc);
    assert!(back["programs"]["banks"][0].get("writable").is_none());
}

#[test]
fn writable_is_set_per_bank() {
    let doc = device(json!({"banks": [
        {"name": "Internal", "size": 64, "writable": false},
        {"name": "User", "size": 64},
    ]}));
    assert_eq!(read(&doc).unwrap(), doc);
}

#[test]
fn a_writable_that_is_not_a_boolean_is_refused_at_the_field() {
    for bad in [json!("no"), json!(0), json!(null)] {
        let doc = banks(json!({"name": "Internal", "size": 64, "writable": bad}));
        assert!(!schema_accepts(&doc), "{bad}");
        assert_eq!(read(&doc).unwrap_err(), (FaultKind::BadValue, "/programs/banks/0/writable".to_string()), "{bad}");
    }
}

#[test]
fn a_dump_states_the_reply_timeout_in_milliseconds_and_reads_back_so() {
    let doc = dump(Some(50), json!(500));
    assert!(schema_accepts(&doc));
    assert_eq!(read(&doc).unwrap(), doc);
}

#[test]
fn a_reply_timeout_without_a_pause_reads() {
    let doc = dump(None, json!(0));
    assert!(schema_accepts(&doc));
    assert_eq!(read(&doc).unwrap(), doc);
}

#[test]
fn a_reply_timeout_equal_to_the_pause_is_not_shorter_and_reads() {
    let doc = dump(Some(200), json!(200));
    assert!(schema_accepts(&doc));
    assert_eq!(read(&doc).unwrap(), doc);
}

#[test]
fn a_reply_timeout_shorter_than_the_pause_is_refused_at_the_timeout() {
    let doc = dump(Some(200), json!(199));
    assert!(schema_accepts(&doc), "the schema cannot compare two fields");
    assert_eq!(read(&doc).unwrap_err(), (FaultKind::BadValue, "/programs/dump/reply_timeout_ms".to_string()));
}

#[test]
fn a_reply_timeout_that_is_not_a_whole_number_of_milliseconds_from_zero_is_refused_at_the_field() {
    for bad in [json!(-1), json!(1.5), json!("100")] {
        let doc = dump(None, bad.clone());
        assert!(!schema_accepts(&doc), "{bad}");
        assert_eq!(read(&doc).unwrap_err(), (FaultKind::BadValue, "/programs/dump/reply_timeout_ms".to_string()), "{bad}");
    }
}

#[test]
fn before_0_6_writable_and_reply_timeout_ms_are_unknown_fields() {
    let cases = [
        (banks(json!({"name": "Internal", "size": 64, "writable": false})), "/programs/banks/0/writable"),
        (dump(Some(50), json!(500)), "/programs/dump/reply_timeout_ms"),
    ];
    for version in ["0.4", "0.5"] {
        for (doc, at) in &cases {
            let mut doc = doc.clone();
            let address = if version == "0.4" { "https://synthhub.roomi-fields.com/osd/0.4/" } else { "https://w3id.org/osd/0.5/" };
            if version == "0.4" {
                doc.as_object_mut().unwrap().remove("$schema");
            }
            let text = doc.to_string().replace("https://w3id.org/osd/0.6/", address);
            let f = osd::read(&text, Kind::Device).expect_err("unknown before 0.6");
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, *at), "{version}: {f}");
        }
    }
}

#[test]
fn converting_from_0_5_invents_neither_writable_nor_a_reply_timeout() {
    let src = json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "programs": {
            "banks": [{"name": "Internal", "size": 64}],
            "delay_ms": 50,
            "dump": {"kind": "request", "request": [240, 1, 247]},
        },
    });
    let out = osd::convert(&src.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&out).unwrap();
    assert!(out["programs"]["banks"][0].get("writable").is_none());
    assert!(out["programs"]["dump"].get("reply_timeout_ms").is_none());
}
