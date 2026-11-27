//! The Electra One export of a command whose frame holds fixed bytes, the argument `value` and
//! checksums: a SysEx template with its value and checksum elements, which sends by the rules of
//! the maker's page (section "SysEx template") the bytes of the command for every value of the
//! range of the argument, and comes back with that argument.

use crate::common::electra::*;
use crate::common::resolved;
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};

fn command(frame: &Value, arg: &Value) -> Value {
    json!({"osc": "/filter/dump", "title": "Dump", "section": "Filter", "args": [arg], "frame": frame})
}

fn device_of(command: Value) -> osd::Resolved {
    let doc = json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "community-re", "url": "https://example.org/d", "license": "CC0-1.0"}],
        "device": {"name": "Trundler", "osc_prefix": "/trundler"},
        "commands": [command],
    });
    resolved(crate::common::device_from(&doc.to_string()))
}

/// The preset written, its message, and the losses.
fn exported(frame: &Value, arg: &Value) -> (String, Value, Vec<Loss>) {
    let t = export(&device_of(command(frame, arg)), Format::ElectraOne).expect("exports");
    let p: Value = serde_json::from_str(&t.value).unwrap();
    assert_eq!(violations(&p), Vec::<String>::new());
    let message = p["controls"][0]["values"][0]["message"].clone();
    assert_eq!(message["type"], "sysex", "the command is written: {:?}", t.lost);
    (t.value, message, t.lost)
}

/// The bytes a template sends for a value, by the maker's rules: F0 and F7 dropped wherever they
/// appear and framing the message, a `value` element the bits of the value from
/// `parameterBitPosition`, a `checksum` over the bytes from `start` (F0 is 0) on `length` bytes.
fn sent(data: &Value, value: i64) -> Vec<u8> {
    let whole = |v: &Value| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()));
    let mut out = vec![0xF0u8];
    for e in data.as_array().unwrap_or_else(|| panic!("a template is a list: {data}")) {
        let byte = match e {
            Value::Number(n) => n.as_u64().unwrap() as u8,
            Value::String(s) => u8::from_str_radix(s, 16).unwrap(),
            o if o["type"] == "value" => {
                let rule = o["rules"][0].clone();
                let get = |k: &str, d: i64| rule.get(k).map_or(d, |v| v.as_i64().unwrap());
                let (from, at, width) = (get("parameterBitPosition", 0), get("byteBitPosition", 0), get("bitWidth", 7));
                (((value >> from) & ((1 << width) - 1)) << at) as u8
            }
            o if o["type"] == "checksum" => {
                let start = whole(&o["start"]).expect("the start is written") as usize;
                let length = whole(&o["length"]).expect("the length is written") as usize;
                let covered = &out[start..start + length];
                let sum: u32 = covered.iter().map(|b| u32::from(*b)).sum();
                match o["algorithm"].as_str().unwrap_or("roland") {
                    "roland" => ((128 - sum % 128) & 127) as u8,
                    "waldorf" => (sum & 127) as u8,
                    "fractal" => covered.iter().fold(0u8, |a, b| a ^ b) & 127,
                    other => panic!("algorithm {other}"),
                }
            }
            other => panic!("an element the page does not document: {other}"),
        };
        if e.is_object() || (byte != 0xF0 && byte != 0xF7) {
            out.push(byte);
        }
    }
    out.push(0xF7);
    out
}

fn u7(range: Option<[i64; 2]>) -> Value {
    with_range(json!({"name": "value", "type": "u7"}), range)
}

fn u14(range: Option<[i64; 2]>) -> Value {
    with_range(json!({"name": "value", "type": "u14"}), range)
}

fn with_range(mut arg: Value, range: Option<[i64; 2]>) -> Value {
    if let Some(r) = range {
        arg["range"] = json!(r);
    }
    arg
}

fn lsb() -> Value {
    json!({"arg": "value", "encoding": "u14_lsb"})
}

fn msb() -> Value {
    json!({"arg": "value", "encoding": "u14_msb"})
}

/// The commands a preset holds: their frame, their argument and the scale of the argument.
fn carried() -> Vec<(Value, Value, [i64; 2])> {
    vec![
        (json!([240, 67, {"arg": "value"}, 247]), u7(None), [0, 127]),
        (json!([240, 67, {"arg": "value"}, 247]), u7(Some([0, 100])), [0, 100]),
        (json!([240, 65, 16, {"arg": "value"}, {"checksum": "twos7", "from": 1}, 247]), u7(Some([3, 90])), [3, 90]),
        (json!([240, 65, 16, {"arg": "value"}, {"checksum": "twos7", "from": 0}, 247]), u7(None), [0, 127]),
        (json!([240, 65, 16, 1, {"arg": "value"}, {"checksum": "xor7", "from": 2, "length": 2}, 247]), u7(Some([0, 127])), [0, 127]),
        (json!([240, 67, lsb(), msb(), {"checksum": "sum7", "from": 1, "length": 3}, 247]), u14(None), [0, 16383]),
        (json!([240, 67, msb(), lsb(), 5, {"checksum": "xor7", "from": 1}, 247]), u14(Some([100, 9000])), [100, 9000]),
        (json!([240, 67, lsb(), {"checksum": "twos7", "from": 1}, msb(), {"checksum": "sum7", "from": 3, "length": 2}, 247]), u14(None), [0, 16383]),
    ]
}

#[test]
fn the_preset_sends_the_bytes_of_the_command_for_every_value_of_the_range_of_its_argument() {
    for (frame, arg, [low, high]) in carried() {
        let (_, message, _) = exported(&frame, &arg);
        for value in low..=high {
            assert_eq!(sent(&message["data"], value), render_frame(&frame, value), "{frame}, value {value}: {message}");
        }
    }
}

#[test]
fn the_range_of_the_argument_is_the_range_of_the_message_and_of_the_value_only_when_the_descriptor_gives_one() {
    for (frame, arg, [low, high]) in carried() {
        let t = export(&device_of(command(&frame, &arg)), Format::ElectraOne).unwrap();
        let p: Value = serde_json::from_str(&t.value).unwrap();
        let v = &p["controls"][0]["values"][0];
        assert_eq!((v["message"]["min"].clone(), v["message"]["max"].clone()), (json!(low), json!(high)), "{frame}: {v}");
        let given = arg.get("range").map(|_| (json!(low), json!(high))).unwrap_or((Value::Null, Value::Null));
        assert_eq!((v["min"].clone(), v["max"].clone()), given, "{frame}: {v}");
    }
}

#[test]
fn a_checksum_is_written_with_its_start_and_its_length_always() {
    let checksums = |data: &Value| -> Vec<(Value, Value)> {
        data.as_array().unwrap().iter().filter(|e| e["type"] == "checksum").map(|e| (e["start"].clone(), e["length"].clone())).collect()
    };
    for (frame, arg, expected) in [
        // Without a length, every byte from its start up to the checksum, F0 being byte 0.
        (json!([240, 65, 16, {"arg": "value"}, {"checksum": "twos7", "from": 1}, 247]), u7(None), vec![(1, 3)]),
        (json!([240, 65, 16, {"arg": "value"}, {"checksum": "twos7", "from": 0}, 247]), u7(None), vec![(0, 4)]),
        (json!([240, 65, 16, {"arg": "value"}, {"checksum": "sum7", "from": 2, "length": 2}, 247]), u7(None), vec![(2, 2)]),
    ] {
        let (_, message, _) = exported(&frame, &arg);
        let expected: Vec<(Value, Value)> = expected.iter().map(|(s, l)| (json!(s), json!(l))).collect();
        assert_eq!(checksums(&message["data"]), expected, "{message}");
    }
}

#[test]
fn the_command_imported_back_sends_the_same_bytes_and_keeps_the_type_and_range_of_its_argument() {
    for (frame, arg, [low, high]) in carried() {
        let (text, _, lost) = exported(&frame, &arg);
        assert!(lost.iter().all(|l| !l.place.starts_with("/commands")), "{frame}: {lost:?}");
        let back = import(&text, Format::ElectraOne, &source()).expect("imports");
        let d = json_of(&back.value);
        let got = &d["commands"][0];
        assert_eq!(got["args"], json!([arg]), "{frame}: {got}");
        for value in low..=high {
            assert_eq!(render_frame(&got["frame"], value), render_frame(&frame, value), "{frame}, value {value}: {got}");
        }
    }
}

#[test]
fn the_named_values_and_the_default_of_a_written_argument_are_losses_at_their_pointers() {
    let mut arg = u7(None);
    arg["values"] = json!({"on": 1});
    arg["default"] = json!(3);
    let (_, _, lost) = exported(&json!([240, 67, {"arg": "value"}, 247]), &arg);
    for field in ["values", "default"] {
        let place = format!("/commands/0/args/0/{field}");
        assert!(lost.iter().any(|l| l.place == place && !l.reason.is_empty()), "{place}: {lost:?}");
    }
    assert!(lost.iter().all(|l| l.place != "/commands/0/args" && l.place != "/commands/0/args/0/range"), "{lost:?}");
}

#[test]
fn any_other_frame_is_a_loss_at_the_frame_and_each_field_of_the_argument_is_named_at_its_pointer() {
    let named = |name: &str, ty: &str| json!({"name": name, "type": ty, "range": [0, 10]});
    for (frame, arg) in [
        // An argument that is not `value`.
        (json!([240, 67, {"arg": "v"}, 247]), named("v", "u7")),
        // A constant added to the value.
        (json!([240, 67, {"arg": "value", "add": 1}, 247]), named("value", "u7")),
        // A byte that is the device's.
        (json!([240, 67, {"device_id": true}, {"arg": "value"}, 247]), named("value", "u7")),
        // A type the preset has no value element for.
        (json!([240, 67, {"arg": "value", "encoding": "s32"}, 247]), named("value", "s32")),
        // One byte alone of a 14-bit value.
        (json!([240, 67, lsb(), 247]), named("value", "u14")),
    ] {
        let t = export(&device_of(command(&frame, &arg)), Format::ElectraOne).expect("exports");
        let p: Value = serde_json::from_str(&t.value).unwrap();
        assert!(p["controls"].as_array().unwrap().is_empty(), "{frame}: {p}");
        for place in ["/commands/0/frame", "/commands/0/args/0/name", "/commands/0/args/0/type", "/commands/0/args/0/range"] {
            assert!(t.lost.iter().any(|l| l.place == place && !l.reason.is_empty()), "{frame}: {place} not in {:?}", t.lost);
        }
    }
}

#[test]
fn a_command_whose_parameter_number_the_source_keeps_is_named_as_written_without_it() {
    let numbered = sysex_template(300, json!([67, 1]));
    let unnumbered = sysex(&[67, 2]);
    let p = preset(&[], vec![control(1, "Plain", None, unnumbered, None), control(2, "Numbered", None, numbered, None)]);
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("imports");
    let d = json_of(&t.value);
    assert_eq!(d["commands"].as_array().map(Vec::len), Some(2), "{d}");
    let out = export(&resolved(t.value), Format::ElectraOne).expect("exports");
    let written: Value = serde_json::from_str(&out.value).unwrap();
    assert!(written["controls"][1]["values"][0]["message"].get("parameterNumber").is_none(), "{written}");
    let named: Vec<&str> = out.lost.iter().filter(|l| l.reason.contains("parameter_numbers")).map(|l| l.place.as_str()).collect();
    assert_eq!(named, ["/commands/1"], "{:?}", out.lost);
}
