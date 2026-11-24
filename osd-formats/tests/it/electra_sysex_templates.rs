//! The elements of a SysEx template of an Electra One preset (page "Preset format description",
//! section "SysEx template"): a checksum, a byte of the value, and the elements OSD has no form
//! for, each a loss that names its control; the revision and the hash of the preset in the
//! source.

use crate::common::electra::*;
use osd_formats::{import, Format, Loss};
use serde_json::{json, Value};

fn imported_with(p: &Value, s: &osd::Source) -> (Value, Vec<Loss>) {
    let t = import(&p.to_string(), Format::ElectraOne, s).expect("the preset imports");
    (json_of(&t.value), t.lost)
}

fn imported(p: &Value) -> (Value, Vec<Loss>) {
    imported_with(p, &source())
}

/// A preset of one control, "Send", whose message is a template.
fn one(template: Value) -> Value {
    preset(&[], vec![control(1, "Send", None, template, None)])
}

fn commands(d: &Value) -> Vec<Value> {
    d.get("commands").and_then(Value::as_array).cloned().unwrap_or_default()
}

const MESSAGE: &str = "/controls/0/values/0/message";

fn loss_at<'a>(lost: &'a [Loss], place: &str) -> &'a Loss {
    lost.iter().find(|l| l.place == place).unwrap_or_else(|| panic!("no loss at {place}: {lost:?}"))
}

// ---- checksum ---------------------------------------------------------------------------

#[test]
fn a_checksum_element_becomes_a_checksum_of_the_frame_that_sends_the_bytes_the_maker_s_rule_gives() {
    // F0 41 10 42 12 40 00 7F, then the checksum of 12 40 00 7F (indexes 4 to 7, F0 is 0):
    // 0x12 + 0x40 + 0x00 + 0x7F = 209; 209 mod 128 = 81; roland: 128 - 81 = 47.
    let data = json!([65, 16, 66, 18, 64, 0, 127, checksum_element(json!("roland"), json!(4), Some(4))]);
    let (d, lost) = imported(&one(sysex_template(1, data)));
    let c = commands(&d);
    assert_eq!(c.len(), 1, "{d}");
    assert_eq!(
        c[0]["frame"],
        json!([240, 65, 16, 66, 18, 64, 0, 127, {"checksum": "twos7", "from": 4, "length": 4}, 247])
    );
    assert_eq!(render_frame(&c[0]["frame"], 0), vec![240, 65, 16, 66, 18, 64, 0, 127, 47, 247]);
    assert!(lost.iter().all(|l| !l.place.starts_with(MESSAGE)), "{lost:?}");
}

#[test]
fn each_algorithm_of_the_maker_becomes_its_osd_algorithm_and_sends_its_own_byte() {
    // F0 41 10 = 240 + 65 + 16 = 321; 321 mod 128 = 65.
    // roland 128 - 65 = 63; waldorf 65; fractal 0xF0 ^ 0x41 ^ 0x10 = 0xA1, on 7 bits 0x21 = 33.
    for (algorithm, word, byte) in [("roland", "twos7", 63u8), ("waldorf", "sum7", 65), ("fractal", "xor7", 33)] {
        let data = json!([65, 16, checksum_element(json!(algorithm), json!(0), Some(3))]);
        let (d, _) = imported(&one(sysex_template(1, data)));
        let c = commands(&d);
        assert_eq!(c.len(), 1, "{algorithm}: {d}");
        assert_eq!(c[0]["frame"], json!([240, 65, 16, {"checksum": word, "from": 0, "length": 3}, 247]), "{algorithm}");
        assert_eq!(render_frame(&c[0]["frame"], 0), vec![240, 65, 16, byte, 247], "{algorithm}");
    }
}

#[test]
fn a_checksum_without_a_length_sums_every_byte_written_before_it() {
    let data = json!([65, 16, 66, 18, 64, 0, 127, checksum_element(json!("roland"), json!(4), None)]);
    let (d, _) = imported(&one(sysex_template(1, data)));
    let c = commands(&d);
    assert!(c[0]["frame"][8].get("length").is_none(), "{}", c[0]["frame"]);
    assert_eq!(render_frame(&c[0]["frame"], 0), vec![240, 65, 16, 66, 18, 64, 0, 127, 47, 247]);
}

#[test]
fn a_checksum_start_written_as_text_is_the_same_start_as_a_number() {
    let data = json!([65, 16, 66, 18, 64, 0, 127, checksum_element(json!("roland"), json!("4"), Some(4))]);
    let (d, lost) = imported(&one(sysex_template(1, data)));
    let c = commands(&d);
    assert_eq!(c.len(), 1, "{lost:?}");
    assert_eq!(c[0]["frame"][8], json!({"checksum": "twos7", "from": 4, "length": 4}));
}

#[test]
fn a_checksum_without_an_algorithm_is_roland_and_without_a_start_begins_at_the_first_byte() {
    // The page: algorithm defaults to roland, start to 0. F0 41 10 = 321; 128 - 65 = 63.
    let data = json!([65, 16, checksum_element(Value::Null, Value::Null, Some(3))]);
    let (d, lost) = imported(&one(sysex_template(1, data)));
    let c = commands(&d);
    assert_eq!(c[0]["frame"][3], json!({"checksum": "twos7", "from": 0, "length": 3}), "{lost:?}");
    assert_eq!(render_frame(&c[0]["frame"], 0), vec![240, 65, 16, 63, 247]);
}

#[test]
fn the_framing_bytes_a_template_holds_are_dropped_before_the_checksum_counts_its_bytes() {
    // The page: F0 and F7 are dropped wherever they appear, so the indexes count the rendered bytes.
    let data = json!(["F0", 65, "F7", 16, 66, 18, 64, 0, 127, checksum_element(json!("roland"), json!(4), Some(4)), "F7"]);
    let (d, _) = imported(&one(sysex_template(1, data)));
    let c = commands(&d);
    assert_eq!(render_frame(&c[0]["frame"], 0), vec![240, 65, 16, 66, 18, 64, 0, 127, 47, 247]);
}

#[test]
fn a_checksum_algorithm_osd_does_not_know_is_a_loss_naming_it_and_no_command_is_written_by_halves() {
    for algorithm in ["yamaha", "crc16", "Roland"] {
        let data = json!([65, 16, checksum_element(json!(algorithm), json!(0), Some(2))]);
        let p = preset(&[], vec![control(1, "Kept", None, cc7(1), None), control(2, "Send", None, sysex_template(1, data), None)]);
        let (d, lost) = imported(&p);
        assert!(commands(&d).is_empty(), "{algorithm}: {d}");
        let l = loss_at(&lost, "/controls/1/values/0/message");
        assert!(l.reason.contains(algorithm), "{algorithm}: {}", l.reason);
        assert!(l.reason.contains("Send"), "the loss names the control: {}", l.reason);
    }
}

#[test]
fn a_checksum_start_that_is_no_whole_number_is_a_loss_and_no_command_is_written() {
    for start in [json!("abc"), json!(-1), json!(1.5)] {
        let data = json!([65, 16, checksum_element(json!("roland"), start.clone(), Some(2))]);
        let (d, lost) = imported(&one(sysex_template(1, data)));
        assert!(commands(&d).is_empty(), "{start}: {d}");
        assert!(lost.iter().any(|l| l.place == MESSAGE), "{start}: {lost:?}");
    }
}

// ---- a byte of the value ----------------------------------------------------------------

#[test]
fn a_value_byte_on_seven_bits_becomes_an_argument_and_the_frame_sends_the_value() {
    let data = json!([65, 16, {"type": "value"}, checksum_element(json!("roland"), json!(1), Some(3))]);
    let (d, lost) = imported(&one(sysex_template(1, data)));
    let c = commands(&d);
    assert_eq!(c.len(), 1, "{lost:?}");
    assert_eq!(c[0]["args"][0]["name"], "value");
    assert_eq!(c[0]["frame"][3], json!({"arg": "value"}));
    // 65 + 16 + 100 = 181; 181 mod 128 = 53; 128 - 53 = 75.
    assert_eq!(render_frame(&c[0]["frame"], 100), vec![240, 65, 16, 100, 75, 247]);
    // 65 + 16 + 0 = 81; 128 - 81 = 47.
    assert_eq!(render_frame(&c[0]["frame"], 0), vec![240, 65, 16, 0, 47, 247]);
}

#[test]
fn a_value_taken_with_its_default_rules_is_the_same_byte_as_one_with_the_rules_written() {
    for element in [value_element(0, 0, 7), json!({"type": "value"}), json!({"type": "value", "rules": [{}]})] {
        let (d, lost) = imported(&one(sysex_template(1, json!([67, element.clone()]))));
        let c = commands(&d);
        assert_eq!(c.len(), 1, "{element}: {lost:?}");
        assert_eq!(c[0]["frame"], json!([240, 67, {"arg": "value"}, 247]), "{element}");
    }
}

#[test]
fn a_value_split_over_two_bytes_becomes_a_14_bit_argument_in_msb_and_lsb_encodings() {
    // The page's example: LSB first, the low 7 bits then bits 7 to 13.
    let data = json!([67, 16, value_element(0, 0, 7), value_element(7, 0, 7)]);
    let (d, lost) = imported(&one(sysex_template(1, data)));
    let c = commands(&d);
    assert_eq!(c.len(), 1, "{lost:?}");
    assert_eq!(c[0]["args"][0]["type"], "u14");
    assert_eq!(
        c[0]["frame"],
        json!([240, 67, 16, {"arg": "value", "encoding": "u14_lsb"}, {"arg": "value", "encoding": "u14_msb"}, 247])
    );
    // 300 = 2 * 128 + 44.
    assert_eq!(render_frame(&c[0]["frame"], 300), vec![240, 67, 16, 44, 2, 247]);
    // 16383 = 127 * 128 + 127.
    assert_eq!(render_frame(&c[0]["frame"], 16383), vec![240, 67, 16, 127, 127, 247]);
}

#[test]
fn a_value_split_with_the_high_bits_first_puts_the_msb_encoding_first() {
    let data = json!([67, value_element(7, 0, 7), value_element(0, 0, 7)]);
    let (d, _) = imported(&one(sysex_template(1, data)));
    let c = commands(&d);
    assert_eq!(c[0]["frame"], json!([240, 67, {"arg": "value", "encoding": "u14_msb"}, {"arg": "value", "encoding": "u14_lsb"}, 247]));
    assert_eq!(render_frame(&c[0]["frame"], 300), vec![240, 67, 2, 44, 247]);
}

#[test]
fn a_value_byte_that_takes_other_bits_than_whole_seven_is_a_loss_naming_the_control() {
    // Packed bits (page, "Packing two parameters into one byte"), a byte position, another parameter.
    let packed = json!({"type": "value", "rules": [
        {"parameterNumber": 10, "parameterBitPosition": 0, "byteBitPosition": 0, "bitWidth": 4},
        {"parameterNumber": 11, "parameterBitPosition": 0, "byteBitPosition": 4, "bitWidth": 3}]});
    for element in [packed, value_element(0, 4, 3), value_element(14, 0, 2), json!({"type": "value", "rules": [{"parameterNumber": 12}]})] {
        let (d, lost) = imported(&one(sysex_template(1, json!([67, element.clone()]))));
        assert!(commands(&d).is_empty(), "{element}: {d}");
        assert!(loss_at(&lost, MESSAGE).reason.contains("Send"), "{element}: {lost:?}");
    }
}

// ---- elements OSD has no form for -------------------------------------------------------

#[test]
fn a_byte_built_from_the_parameter_number_is_a_loss_naming_the_control_and_no_command_is_written() {
    let (d, lost) = imported(&one(sysex_template(5, json!([67, {"type": "parameter"}]))));
    assert!(commands(&d).is_empty(), "{d}");
    let l = loss_at(&lost, MESSAGE);
    assert!(l.reason.contains("Send") && l.reason.contains("parameter"), "{}", l.reason);
}

#[test]
fn a_lua_function_element_is_a_loss_naming_the_control_and_the_function_and_is_never_read() {
    let preset = one(sysex_template(1, json!([67, {"type": "function", "name": "pickByte"}])));
    let mut p = preset;
    // Whatever Lua a document carries is not read: only the name of the function is.
    p["lua"] = json!("function pickByte() return 0x70 end");
    let (d, lost) = imported(&p);
    assert!(commands(&d).is_empty(), "{d}");
    let l = loss_at(&lost, MESSAGE);
    assert!(l.reason.contains("Send") && l.reason.contains("pickByte"), "{}", l.reason);
    assert!(!l.reason.contains("0x70"), "the function was read: {}", l.reason);
}

#[test]
fn a_lua_function_a_preset_calls_in_several_controls_is_named_in_each_loss() {
    let t = |n: &str| sysex_template(1, json!([67, {"type": "function", "name": n}]));
    let p = preset(&[], vec![control(1, "A", None, t("one"), None), control(2, "B", None, t("two"), None)]);
    let (_, lost) = imported(&p);
    assert!(loss_at(&lost, "/controls/0/values/0/message").reason.contains("one"));
    assert!(loss_at(&lost, "/controls/1/values/0/message").reason.contains("two"));
}

#[test]
fn any_other_element_of_a_template_is_a_loss_and_none_is_dropped_silently() {
    for element in [json!({"type": "any"}), json!({"type": "mystery"}), json!({"nothing": 1})] {
        let (d, lost) = imported(&one(sysex_template(1, json!([67, element.clone()]))));
        assert!(commands(&d).is_empty(), "{element}: {d}");
        assert!(loss_at(&lost, MESSAGE).reason.contains("Send"), "{element}: {lost:?}");
    }
}

#[test]
fn a_template_with_a_lost_element_loses_only_its_own_control_the_others_are_converted() {
    let p = preset(
        &[],
        vec![
            control(1, "Fixed", None, sysex_template(1, json!([67, 1])), None),
            control(2, "Lost", None, sysex_template(1, json!([67, {"type": "parameter"}])), None),
            control(3, "Summed", None, sysex_template(1, json!([67, 2, checksum_element(json!("roland"), json!(0), Some(3))])), None),
        ],
    );
    let (d, lost) = imported(&p);
    assert_eq!(commands(&d).len(), 2, "{d}");
    assert_eq!(lost.iter().filter(|l| l.place.ends_with("/message") && l.place.starts_with("/controls/")).count(), 1, "{lost:?}");
    assert!(lost.iter().any(|l| l.place == "/controls/1/values/0/message"));
}

// ---- the parameter number, the date, the revision and the hash --------------------------

fn source_of(d: &Value) -> Value {
    d["_sources"][0].clone()
}

#[test]
fn the_parameter_number_of_each_command_is_kept_in_the_source_under_the_address_of_the_command() {
    let p = preset(
        &[],
        vec![
            control(1, "First", None, sysex_template(5, json!([67, {"type": "value"}])), None),
            control(2, "Second", None, sysex_template(300, json!([67, 1])), None),
            control(3, "Cc", None, cc7(7), None),
        ],
    );
    let (d, _) = imported(&p);
    let c = commands(&d);
    let numbers = &source_of(&d)["parameter_numbers"];
    assert_eq!(c.len(), 2);
    assert_eq!(numbers[c[0]["osc"].as_str().unwrap()], 5, "{numbers}");
    assert_eq!(numbers[c[1]["osc"].as_str().unwrap()], 300, "{numbers}");
    assert_eq!(numbers.as_object().unwrap().len(), 2, "{numbers}");
}

#[test]
fn the_date_the_caller_gives_is_carried_and_none_is_invented_when_the_source_gives_none() {
    let p = one(sysex_template(1, json!([67, 1])));
    let mut dated = source();
    dated.imported_at = Some("2026-11-21".to_string());
    let (d, _) = imported_with(&p, &dated);
    assert_eq!(source_of(&d)["imported_at"], "2026-11-21");
    let (d, _) = imported_with(&p, &source());
    assert!(source_of(&d).get("imported_at").is_none(), "{}", source_of(&d));
}

#[test]
fn the_revision_the_caller_passes_in_the_source_is_kept_in_the_descriptor_s_sources() {
    let mut s = source();
    s.extra.insert("revision".to_string(), json!("r42"));
    let (d, _) = imported_with(&one(sysex_template(1, json!([67, 1]))), &s);
    assert_eq!(source_of(&d)["revision"], "r42");
    assert_eq!(source_of(&d)["license"], "CC-BY-4.0");
    assert_eq!(source_of(&d)["url"], "https://app.electra.one/preset/abc");
}

#[test]
fn the_sha256_of_the_preset_text_received_is_written_in_the_source_by_the_import() {
    let compact = r#"{"version":3,"name":"Hashed","controls":[]}"#;
    let spaced = r#"{ "version": 3, "name": "Hashed", "controls": [] }"#;
    let hash = |text: &str| {
        let t = import(text, Format::ElectraOne, &source()).unwrap();
        json_of(&t.value)["_sources"][0]["sha256"].clone()
    };
    assert_eq!(hash(compact), "d542ffc1c07f8c31261cacf3cd484d6e9883b6d4ac47a66ad8abb5eea73f2eca");
    // The text as received: another layout of the same preset is another hash.
    assert_eq!(hash(spaced), "77419632865ce8d04c82673b5f4da58c94a8e5f2b8bfeb0132a9d2d5997c53bf");
}

#[test]
fn the_revision_and_the_hash_sit_together_in_the_source_beside_its_credit() {
    let mut s = source();
    s.extra.insert("revision".to_string(), json!("r7"));
    let (d, _) = imported_with(&one(sysex_template(1, json!([67, 1]))), &s);
    let src = source_of(&d);
    assert_eq!(src["revision"], "r7");
    assert_eq!(src["sha256"].as_str().map(str::len), Some(64), "{src}");
    assert_eq!(src["author"], "A preset maker");
    assert_eq!(d["_sources"].as_array().unwrap().len(), 1);
}
