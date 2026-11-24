//! How a file is written on a device by MIDI (0.6): `resources[].via.file_write.steps`, the
//! steps `open`, `write` (once per chunk) and `close`, each with the message it sends (`send`,
//! the frame elements of OSD, plus `text`, `file` and `captured`) and the reply awaited (`reply`,
//! the reply elements of OSD, plus `capture_json`) with `reply_timeout_ms`; `delay_ms` is the
//! pause before the next step. The `write` step carries `chunk_bytes` and `coding`.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";
const HEAD: [u8; 6] = [240, 0, 33, 123, 1, 4];
const REPLY_HEAD: [u8; 6] = [240, 0, 33, 123, 1, 5];

fn head() -> Vec<Value> {
    let mut v: Vec<Value> = HEAD.iter().map(|b| json!(b)).collect();
    v.push(json!(0));
    v
}

fn reply_head() -> Vec<Value> {
    let mut v: Vec<Value> = REPLY_HEAD.iter().map(|b| json!(b)).collect();
    v.push(json!("any"));
    v
}

fn cat(mut a: Vec<Value>, b: Vec<Value>) -> Vec<Value> {
    a.extend(b);
    a
}

fn steps() -> Value {
    json!([
        {
            "step": "open",
            "send": cat(head(), vec![json!({"text": "{\"open\":{\"path\":\""}), json!({"file": "path"}), json!({"text": "\",\"write\":1}}"}), json!(247)]),
            "reply": cat(reply_head(), vec![json!({"capture_json": "fid"})]),
            "reply_timeout_ms": 1000
        },
        {
            "step": "write",
            "chunk_bytes": 1024,
            "coding": "msb_first_7in8",
            "send": cat(head(), vec![
                json!({"text": "{\"write\":{\"fid\":"}), json!({"captured": "fid", "as": "decimal"}),
                json!({"text": ",\"addr\":"}), json!({"file": "offset", "as": "decimal"}),
                json!({"text": ",\"size\":"}), json!({"file": "chunk_size", "as": "decimal"}),
                json!({"text": "}}"}), json!(0), json!({"file": "chunk"}), json!(247)
            ]),
            "reply": reply_head(),
            "reply_timeout_ms": 1000
        },
        {
            "step": "close",
            "send": cat(head(), vec![json!({"text": "{\"close\":{\"fid\":"}), json!({"captured": "fid", "as": "decimal"}), json!({"text": "}}"}), json!(247)]),
            "reply": reply_head(),
            "reply_timeout_ms": 1000
        }
    ])
}

fn doc_with(steps: Value) -> Value {
    json!({
        "$schema": SCHEMA,
        "device": {"name": "Deluge", "vendor": "Synthstrom Audible", "osc_prefix": "/deluge"},
        "resources": [{
            "resource": "X-DeviceDefinition",
            "canSet": "full",
            "mediaTypes": ["application/xml"],
            "export": "deluge",
            "via": {"file_write": {
                "path": ["MIDI_DEVICES", "DEFINITION", {"variable": "vendor"}, {"variable": "device", "suffix": ".xml"}],
                "carried_by": "sysex",
                "steps": steps
            }}
        }]
    })
}

fn doc() -> Value {
    doc_with(steps())
}

/// The document with `f` applied to its steps.
fn changed(f: impl FnOnce(&mut Vec<Value>)) -> Value {
    let mut s = steps();
    f(s.as_array_mut().unwrap());
    doc_with(s)
}

fn read(d: &Value) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&d.to_string(), Kind::Device)
}

fn written(d: &osd::Descriptor) -> Value {
    serde_json::from_str(&osd::write(d)).unwrap()
}

fn refused_at(d: &Value, kind: FaultKind, pointer: &str) {
    let f = read(d).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

fn schema_errors(d: &Value) -> Vec<(String, String)> {
    common::Schemas::load("0.6").errors(Kind::Device, d)
}

const P: &str = "/resources/0/via/file_write/steps";

#[test]
fn a_file_write_with_steps_reads_as_typed_steps_and_reads_back_as_written() {
    let d = doc();
    let r = read(&d).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(written(&r), d);
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
    let osd::Descriptor::Device(dev) = r else { unreachable!() };
    let fw = dev.resources.as_ref().unwrap()[0].via.as_ref().unwrap().file_write.as_ref().unwrap();
    let steps = fw.steps.as_ref().expect("steps");
    assert_eq!(steps.len(), 3);
    assert_eq!(format!("{:?}", steps[0].step), "Open");
    assert_eq!(format!("{:?}", steps[1].step), "Write");
    assert_eq!(format!("{:?}", steps[2].step), "Close");
    assert_eq!(steps[1].chunk_bytes, Some(1024));
    assert_eq!(steps[0].reply_timeout_ms, Some(1000));
    assert_eq!(steps[0].send.len(), 11);
}

#[test]
fn a_step_pauses_before_the_next_step_by_delay_ms_and_the_wait_for_the_reply_may_equal_it() {
    let d = changed(|s| {
        s[0]["delay_ms"] = json!(50);
        s[0]["reply_timeout_ms"] = json!(50);
        s[1]["delay_ms"] = json!(10);
    });
    assert_eq!(written(&read(&d).unwrap_or_else(|f| panic!("{f}"))), d);
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
    let osd::Descriptor::Device(dev) = read(&d).unwrap() else { unreachable!() };
    let steps = dev.resources.unwrap()[0].via.clone().unwrap().file_write.unwrap().steps.unwrap();
    assert_eq!(steps[0].delay_ms, Some(50));
    assert_eq!(steps[1].delay_ms, Some(10));
}

#[test]
fn a_step_that_expects_no_reply_has_no_reply_and_no_timeout() {
    let d = changed(|s| {
        let c = s[2].as_object_mut().unwrap();
        c.remove("reply");
        c.remove("reply_timeout_ms");
    });
    assert_eq!(written(&read(&d).unwrap_or_else(|f| panic!("{f}"))), d);
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
}

#[test]
fn a_file_write_that_says_only_its_path_a_0_6_device_without_steps_and_a_0_5_device_read_as_before() {
    let mut d = doc();
    d["resources"][0]["via"]["file_write"].as_object_mut().unwrap().remove("steps");
    assert_eq!(written(&read(&d).unwrap()), d);
    assert!(schema_errors(&d).is_empty());
    let osd::Descriptor::Device(dev) = read(&d).unwrap() else { unreachable!() };
    assert!(dev.resources.unwrap()[0].via.clone().unwrap().file_write.unwrap().steps.is_none());
    let plain = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}});
    assert_eq!(written(&read(&plain).unwrap()), plain);
    let v05 = json!({"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}});
    assert!(read(&v05).is_ok());
}

#[test]
fn steps_are_unknown_in_a_0_5_descriptor() {
    let mut d = doc();
    d["$schema"] = json!("https://w3id.org/osd/0.5/device.schema.json");
    let f = read(&d).expect_err("0.5 has no resources");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/resources"));
}

#[test]
fn a_step_outside_open_write_and_close_is_refused_at_the_step() {
    for word in ["flush", "Open", "read", ""] {
        let d = changed(|s| s[2]["step"] = json!(word));
        refused_at(&d, FaultKind::BadValue, &format!("{P}/2/step"));
        assert!(!schema_errors(&d).is_empty(), "{word}");
    }
}

#[test]
fn a_chunk_of_zero_bytes_is_refused_at_chunk_bytes_and_one_byte_reads() {
    for bad in [json!(0), json!(-1), json!(1.5), json!("1024")] {
        let d = changed(|s| s[1]["chunk_bytes"] = bad.clone());
        refused_at(&d, FaultKind::BadValue, &format!("{P}/1/chunk_bytes"));
        assert!(!schema_errors(&d).is_empty(), "{bad}");
    }
    let d = changed(|s| s[1]["chunk_bytes"] = json!(1));
    assert!(read(&d).is_ok());
    assert!(schema_errors(&d).is_empty());
}

#[test]
fn the_write_step_says_its_chunk_bytes_and_its_coding() {
    let d = changed(|s| {
        s[1].as_object_mut().unwrap().remove("chunk_bytes");
    });
    refused_at(&d, FaultKind::Missing, &format!("{P}/1/chunk_bytes"));
    let d = changed(|s| {
        s[1].as_object_mut().unwrap().remove("coding");
    });
    refused_at(&d, FaultKind::Missing, &format!("{P}/1/coding"));
}

#[test]
fn chunk_bytes_and_coding_belong_to_the_write_step_only() {
    for (i, key, value) in [(0, "chunk_bytes", json!(64)), (2, "coding", json!("mcoded7"))] {
        let d = changed(|s| s[i][key] = value.clone());
        refused_at(&d, FaultKind::BadValue, &format!("{P}/{i}/{key}"));
    }
}

#[test]
fn a_coding_is_mcoded7_of_the_norm_or_msb_first_7in8_and_another_word_is_refused() {
    for word in ["mcoded7", "msb_first_7in8"] {
        let d = changed(|s| s[1]["coding"] = json!(word));
        assert!(read(&d).is_ok(), "{word}");
        assert!(schema_errors(&d).is_empty(), "{word}: {:?}", schema_errors(&d));
        assert_eq!(written(&read(&d).unwrap()), d);
    }
    for word in ["deluge", "7in8", "Mcoded7", "none", ""] {
        let d = changed(|s| s[1]["coding"] = json!(word));
        refused_at(&d, FaultKind::BadValue, &format!("{P}/1/coding"));
        assert!(!schema_errors(&d).is_empty(), "{word}");
    }
}

#[test]
fn a_reply_without_reply_timeout_ms_is_refused_at_the_step() {
    for i in 0..3 {
        let d = changed(|s| {
            s[i].as_object_mut().unwrap().remove("reply_timeout_ms");
        });
        refused_at(&d, FaultKind::Missing, &format!("{P}/{i}/reply_timeout_ms"));
    }
}

#[test]
fn a_reply_timeout_ms_shorter_than_the_delay_ms_of_the_step_is_refused_at_reply_timeout_ms() {
    let d = changed(|s| {
        s[1]["delay_ms"] = json!(2000);
    });
    refused_at(&d, FaultKind::BadValue, &format!("{P}/1/reply_timeout_ms"));
    let d = changed(|s| {
        s[0]["delay_ms"] = json!(1001);
    });
    refused_at(&d, FaultKind::BadValue, &format!("{P}/0/reply_timeout_ms"));
}

#[test]
fn a_negative_or_non_integer_delay_or_timeout_is_refused_at_its_place() {
    for (key, bad) in [("delay_ms", json!(-1)), ("reply_timeout_ms", json!(-5)), ("reply_timeout_ms", json!(1.5)), ("delay_ms", json!("10"))] {
        let d = changed(|s| s[0][key] = bad.clone());
        refused_at(&d, FaultKind::BadValue, &format!("{P}/0/{key}"));
        assert!(!schema_errors(&d).is_empty(), "{key} {bad}");
    }
}

#[test]
fn a_list_of_steps_without_open_or_without_write_is_refused_at_the_list() {
    let d = changed(|s| {
        s.remove(0);
    });
    refused_at(&d, FaultKind::BadValue, P);
    let d = changed(|s| {
        s.remove(1);
    });
    refused_at(&d, FaultKind::BadValue, P);
    refused_at(&doc_with(json!([])), FaultKind::BadValue, P);
}

#[test]
fn a_list_of_steps_without_close_is_refused_at_the_list() {
    let d = changed(|s| {
        s.remove(2);
    });
    refused_at(&d, FaultKind::BadValue, P);
}

#[test]
fn a_second_open_write_or_close_is_refused_at_the_later_step() {
    for (from, at) in [(0usize, 3usize), (1, 3), (2, 3)] {
        let d = changed(|s| {
            let copy = s[from].clone();
            s.push(copy);
        });
        refused_at(&d, FaultKind::BadValue, &format!("{P}/{at}"));
    }
    let d = changed(|s| {
        let copy = s[1].clone();
        s.insert(2, copy);
    });
    refused_at(&d, FaultKind::BadValue, &format!("{P}/2"));
}

#[test]
fn an_unknown_key_of_a_step_is_refused_at_the_key_and_a_note_is_kept() {
    let d = changed(|s| s[0]["retries"] = json!(3));
    refused_at(&d, FaultKind::UnknownField, &format!("{P}/0/retries"));
    assert!(!schema_errors(&d).is_empty());
    let d = changed(|s| s[0]["_note"] = json!("a note for people"));
    assert_eq!(written(&read(&d).unwrap_or_else(|f| panic!("{f}"))), d);
}

#[test]
fn a_step_without_send_is_refused_as_missing_and_a_step_without_step_too() {
    let d = changed(|s| {
        s[0].as_object_mut().unwrap().remove("send");
    });
    refused_at(&d, FaultKind::Missing, &format!("{P}/0/send"));
    let d = changed(|s| {
        s[0].as_object_mut().unwrap().remove("step");
    });
    refused_at(&d, FaultKind::Missing, &format!("{P}/0/step"));
}

#[test]
fn the_variable_parts_of_a_message_are_objects_the_schema_checks_and_never_a_text_with_patterns() {
    // path, size, offset, chunk_size, chunk: the names of the elements `file`.
    let d = changed(|s| {
        s[0]["send"] = json!([240, {"file": "size", "as": "decimal"}, {"file": "path"}, 247]);
        s[1]["send"] = json!([240, {"file": "chunk"}, {"file": "offset", "as": "decimal"}, {"file": "chunk_size", "as": "decimal"}, 247]);
    });
    assert_eq!(written(&read(&d).unwrap_or_else(|f| panic!("{f}"))), d);
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
    // a template in a string is not an element
    for bad in [json!("{path}"), json!("path"), json!("{\"open\":1}")] {
        let d = changed(|s| s[0]["send"] = json!([240, bad.clone(), 247]));
        refused_at(&d, FaultKind::BadValue, &format!("{P}/0/send/1"));
        assert!(!schema_errors(&d).is_empty(), "{bad}");
    }
}

#[test]
fn a_file_element_outside_the_list_or_with_another_key_is_refused_at_its_place() {
    let d = changed(|s| s[0]["send"] = json!([240, {"file": "date"}, 247]));
    refused_at(&d, FaultKind::BadValue, &format!("{P}/0/send/1/file"));
    assert!(!schema_errors(&d).is_empty());
    let d = changed(|s| s[0]["send"] = json!([240, {"file": "path", "pad": 1}, 247]));
    refused_at(&d, FaultKind::UnknownField, &format!("{P}/0/send/1/pad"));
    assert!(!schema_errors(&d).is_empty());
    let d = changed(|s| s[0]["send"] = json!([240, {"file": "offset", "as": "hex"}, 247]));
    refused_at(&d, FaultKind::BadValue, &format!("{P}/0/send/1/as"));
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_text_element_is_a_non_empty_printable_ascii_string() {
    for bad in [json!(""), json!("é"), json!("a\nb"), json!(5)] {
        let d = changed(|s| s[0]["send"] = json!([240, {"text": bad.clone()}, 247]));
        refused_at(&d, FaultKind::BadValue, &format!("{P}/0/send/1/text"));
        assert!(!schema_errors(&d).is_empty(), "{bad}");
    }
}

#[test]
fn a_captured_element_names_a_value_read_in_the_reply_of_the_open_step() {
    let d = changed(|s| s[1]["send"] = json!([240, {"captured": "fid", "as": "decimal"}, 247]));
    assert!(read(&d).is_ok());
    let d = changed(|s| s[1]["send"] = json!([240, {"captured": "", "as": "decimal"}, 247]));
    refused_at(&d, FaultKind::BadValue, &format!("{P}/1/send/1/captured"));
    // a name that no capture_json of the reply of the open step gives is refused at the element
    let d = changed(|s| s[1]["send"] = json!([240, {"captured": "handle", "as": "decimal"}, 247]));
    refused_at(&d, FaultKind::BadValue, &format!("{P}/1/send/1/captured"));
}

#[test]
fn an_element_of_a_message_is_a_byte_or_the_elements_of_osd_and_a_byte_over_255_is_refused() {
    let d = changed(|s| s[0]["send"] = json!([240, 256, 247]));
    refused_at(&d, FaultKind::BadValue, &format!("{P}/0/send/1"));
    let d = changed(|s| s[0]["send"] = json!([240, {"device_id": true}, {"arg": "x", "encoding": "ascii"}, 247]));
    assert!(read(&d).is_ok(), "the elements OSD already has stay readable");
    assert!(schema_errors(&d).is_empty());
}

#[test]
fn a_reply_takes_the_elements_of_osd_and_a_capture_on_a_json_key() {
    let d = changed(|s| s[0]["reply"] = json!([240, "any", {"capture": "n"}, {"capture_json": "fid"}]));
    assert_eq!(written(&read(&d).unwrap_or_else(|f| panic!("{f}"))), d);
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
    for bad in [json!({"capture_json": ""}), json!({"capture_json": 3}), json!({"capture_json": "fid", "x": 1}), json!({"capture_json": "$.fid"})] {
        let d = changed(|s| s[0]["reply"] = json!([240, bad.clone()]));
        let f = read(&d).expect_err("refused");
        assert!(f.pointer.starts_with(&format!("{P}/0/reply/1")), "{f}");
        assert!(!schema_errors(&d).is_empty(), "{bad}");
    }
}

#[test]
fn a_capture_on_a_json_key_is_a_reply_element_only_and_never_part_of_a_message_sent() {
    let d = changed(|s| s[0]["send"] = json!([240, {"capture_json": "fid"}, 247]));
    let f = read(&d).expect_err("refused");
    assert!(f.pointer.starts_with(&format!("{P}/0/send/1")), "{f}");
    assert!(!schema_errors(&d).is_empty());
}

#[test]
fn a_file_write_with_steps_needs_no_script_and_derives_no_capability() {
    let osd::Descriptor::Device(dev) = read(&doc()).unwrap() else { unreachable!() };
    assert!(dev.derived_capabilities().is_empty());
}

#[test]
fn a_second_resource_with_steps_names_its_own_place_in_a_refusal() {
    let mut d = doc();
    let mut second = d["resources"][0].clone();
    second["resource"] = json!("X-Other");
    second["via"]["file_write"]["steps"][1]["chunk_bytes"] = json!(0);
    d["resources"].as_array_mut().unwrap().push(second);
    refused_at(&d, FaultKind::BadValue, "/resources/1/via/file_write/steps/1/chunk_bytes");
}

#[test]
fn the_path_of_a_file_write_with_steps_still_resolves() {
    let osd::Descriptor::Device(target) = read(&doc()).unwrap() else { unreachable!() };
    let source = json!({"$schema": SCHEMA, "device": {"name": "minilogue", "vendor": "Korg", "osc_prefix": "/s"}});
    let osd::Descriptor::Device(source) = read(&source).unwrap() else { unreachable!() };
    assert_eq!(osd::resource_path(&target, "X-DeviceDefinition", &source).unwrap(), "MIDI_DEVICES/DEFINITION/Korg/minilogue.xml");
}

fn case(name: &str, file: &str) -> Value {
    let path = common::repo().join("conformance/0.6/read").join(name).join(file);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

#[test]
fn a_conformance_case_writes_a_file_on_the_deluge_from_the_code_with_its_source_and_its_licence() {
    let name = "resource-deluge-file-write";
    let c = case(name, "case.json");
    let d = case(name, "descriptor.json");
    assert_eq!(c["read"], "device");
    assert_eq!(c["result"], d, "the result is the descriptor read back");
    let steps = &d["resources"][0]["via"]["file_write"]["steps"];
    let words: Vec<&str> = steps.as_array().unwrap().iter().map(|s| s["step"].as_str().unwrap()).collect();
    assert_eq!(words, ["open", "write", "close"]);
    assert_eq!(steps[1]["chunk_bytes"], 1024);
    assert_eq!(steps[1]["coding"], "msb_first_7in8");
    for s in steps.as_array().unwrap() {
        assert_eq!(s["reply_timeout_ms"], 1000, "{s}");
        assert_eq!(&s["send"].as_array().unwrap()[..6], &HEAD.iter().map(|b| json!(b)).collect::<Vec<_>>()[..], "the identity and the command Json of the Deluge");
        assert_eq!(s["send"].as_array().unwrap().last().unwrap(), &json!(247));
    }
    let source = &d["_sources"][0];
    assert!(source["url"].as_str().unwrap().contains("DelugeFirmware"), "{source}");
    assert_eq!(source["commit"], "680b93c");
    assert!(source["license"].as_str().is_some_and(|l| !l.is_empty() && l != "NOASSERTION"), "{source}");
    let text = format!("{c} {source}");
    assert!(text.contains("not observed on a device"), "the case says it is built from the code: {text}");
    assert!(text.contains("choice of OSD"), "the wait for a reply is a choice of OSD: {text}");
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
}

#[test]
fn the_conformance_cases_hold_each_refusal_of_a_file_write_with_its_place() {
    let cases = [
        ("resource-file-write-step-unknown-refused", ("BadValue", format!("{P}/2/step"))),
        ("resource-file-write-chunk-bytes-zero-refused", ("BadValue", format!("{P}/1/chunk_bytes"))),
        ("resource-file-write-reply-without-timeout-refused", ("Missing", format!("{P}/0/reply_timeout_ms"))),
        ("resource-file-write-timeout-shorter-than-pause-refused", ("BadValue", format!("{P}/0/reply_timeout_ms"))),
        ("resource-file-write-without-open-refused", ("BadValue", P.to_string())),
        ("resource-file-write-two-writes-refused", ("BadValue", format!("{P}/2"))),
        ("resource-file-write-coding-unknown-refused", ("BadValue", format!("{P}/1/coding"))),
    ];
    for (name, (kind, pointer)) in cases {
        let c = case(name, "case.json");
        assert_eq!(c["fault"], json!({"kind": kind, "pointer": pointer}), "{name}");
        assert_eq!(c["read"], "device", "{name}");
        let d = case(name, "descriptor.json");
        let f = read(&d).expect_err(name);
        assert_eq!((format!("{:?}", f.kind).as_str(), f.pointer.as_str()), (kind, pointer.as_str()), "{name}: {f}");
    }
}

#[test]
fn the_documentation_shows_the_deluge_file_write_from_the_resource_to_its_steps() {
    let text = std::fs::read_to_string(common::repo().join("spec/0.6.md")).unwrap();
    for needle in ["\"steps\"", "chunk_bytes", "coding", "reply_timeout_ms", "capture_json", "msb_first_7in8", "X-DeviceDefinition", "\"open\"", "\"close\""] {
        assert!(text.contains(needle), "spec/0.6.md does not show {needle}");
    }
}
