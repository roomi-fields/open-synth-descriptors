//! A control of 0.7 (a `cc_params` entry, a `params` entry) carries `readings` with the form of a
//! command's, without `args` and with a `range` on the reference scale of the control (proposal
//! 0006, `spec/0.7.md`): the form and the refusals, the scale of the range, the reading a firmware
//! holds, and the refusal before 0.7.

use osd::{Descriptor, FaultKind, Kind, ReadingState, Version};
use serde_json::{json, Value};

const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn cc_way() -> Value {
    json!([{"ctrlType": "cc", "ctrlIndex": [74]}])
}

fn cc_doc(entry: Value, extra: Value) -> Value {
    let mut d = json!({"$schema": D07, "device": {"name": "T", "osc_prefix": "/t"}, "cc_params": {"channel": 0, "entries": [entry]}});
    for (k, v) in extra.as_object().unwrap() {
        d[k] = v.clone();
    }
    d
}

fn params_doc(entry: Value) -> Value {
    json!({
        "$schema": D07,
        "device": {"name": "T", "osc_prefix": "/t"},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [entry]}
    })
}

fn with_readings(readings: Value) -> Value {
    cc_doc(json!({"osc": "/a", "ways": cc_way(), "readings": readings}), json!({}))
}

fn with_scale(scale: Value, readings: Value) -> Value {
    cc_doc(json!({"osc": "/a", "scale": scale, "ways": cc_way(), "readings": readings}), json!({}))
}

fn device(d: &Value) -> osd::Device {
    match osd::read(&d.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}\n{d}")) {
        Descriptor::Device(dev) => *dev,
        _ => panic!("a device"),
    }
}

fn cc_entry(d: &Value) -> osd::CcEntry {
    device(d).cc_params.unwrap().entries.remove(0)
}

fn refused(d: &Value) -> (FaultKind, String) {
    let f = osd::read(&d.to_string(), Kind::Device).expect_err(&format!("refused: {d}"));
    (f.kind, f.pointer)
}

fn r(state: &str, firmware: &str) -> Value {
    json!({"state": state, "firmware": firmware})
}

const AT: &str = "/cc_params/entries/0/readings";

// ---- the form of a reading ----

#[test]
fn a_reading_of_a_control_gives_its_state_its_firmware_and_leaves_the_rest_absent_when_not_written() {
    let list = cc_entry(&with_readings(json!([r("valid", "1.2")]))).readings.expect("readings");
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].state, list[0].firmware.as_str()), (ReadingState::Valid, "1.2"));
    assert_eq!((list[0].source.as_deref(), list[0].title.as_deref(), list[0].range), (None, None, None));
}

#[test]
fn a_params_entry_carries_readings_like_a_cc_entry() {
    let d = params_doc(json!({"osc": "/a", "ways": [{"ctrlType": "sysex", "id": 5}], "readings": [
        {"state": "to-confirm", "firmware": "2.0", "title": "Shape"}]}));
    let list = device(&d).params.unwrap().entries.remove(0).readings.expect("readings");
    assert_eq!((list[0].state, list[0].title.as_deref()), (ReadingState::ToConfirm, Some("Shape")));
}

#[test]
fn readings_of_a_control_keep_their_title_and_range_raw_in_the_order_written_and_are_written_back_key_for_key() {
    let d = cc_doc(
        json!({"osc": "/a", "title": "Filter cutoff", "scale": [0, 255], "ways": cc_way(), "readings": [
            {"state": "valid", "firmware": "1.2", "source": "manual", "title": "Filter cutoff", "range": [0, 127]},
            {"state": "to-confirm", "firmware": "1.3"},
            {"state": "refuted", "firmware": "1.4", "range": [0, 255]},
        ]}),
        json!({"sources": [{"type": "vendor-doc", "id": "manual"}]}),
    );
    let list = cc_entry(&d).readings.unwrap();
    assert_eq!((list[0].firmware.as_str(), list[0].source.as_deref(), list[0].range), ("1.2", Some("manual"), Some([0, 127])));
    // Raw: a reading without title or range does not take those of the control.
    assert_eq!((list[1].title.as_deref(), list[1].range), (None, None));
    assert_eq!(list[2].state, ReadingState::Refuted);
    let read = osd::read(&d.to_string(), Kind::Device).unwrap();
    let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
    assert_eq!(back, d);
}

#[test]
fn a_control_without_readings_has_none() {
    let e = cc_entry(&cc_doc(json!({"osc": "/a", "ways": cc_way()}), json!({})));
    assert!(e.readings.is_none());
    assert!(e.readings_for("1.2", false).is_empty());
    assert!(e.readings_for("1.2", true).is_empty());
}

// ---- the refusals ----

#[test]
fn a_reading_of_a_control_without_a_state_or_a_firmware_is_missing_at_its_place() {
    assert_eq!(refused(&with_readings(json!([{"firmware": "1"}]))), (FaultKind::Missing, format!("{AT}/0/state")));
    assert_eq!(refused(&with_readings(json!([r("valid", "1"), {"state": "valid"}]))), (FaultKind::Missing, format!("{AT}/1/firmware")));
}

#[test]
fn a_state_of_a_control_reading_outside_the_three_words_is_a_bad_value_at_the_state() {
    for word in ["probably", "Valid", "to_confirm", ""] {
        assert_eq!(refused(&with_readings(json!([r(word, "1")]))), (FaultKind::BadValue, format!("{AT}/0/state")), "{word}");
    }
}

#[test]
fn an_empty_readings_list_of_a_control_is_a_bad_value() {
    assert_eq!(refused(&with_readings(json!([]))), (FaultKind::BadValue, AT.to_string()));
}

#[test]
fn an_empty_title_of_a_control_reading_is_a_bad_value() {
    let d = with_readings(json!([{"state": "valid", "firmware": "1", "title": ""}]));
    assert_eq!(refused(&d), (FaultKind::BadValue, format!("{AT}/0/title")));
}

#[test]
fn a_reading_of_a_control_has_no_args_and_no_other_key_than_its_own() {
    let d = with_readings(json!([{"state": "valid", "firmware": "1", "args": [{"name": "v", "type": "int"}]}]));
    assert_eq!(refused(&d), (FaultKind::UnknownField, format!("{AT}/0/args")));
    let d = with_readings(json!([{"state": "valid", "firmware": "1", "confidence": 3}]));
    assert_eq!(refused(&d), (FaultKind::UnknownField, format!("{AT}/0/confidence")));
}

#[test]
fn a_range_that_is_not_a_pair_of_integers_is_refused_at_the_range() {
    for bad in [json!([0]), json!([0, 1, 2]), json!("0-127")] {
        let d = with_readings(json!([{"state": "valid", "firmware": "1", "range": bad}]));
        assert_eq!(refused(&d).1, format!("{AT}/0/range"), "{bad}");
    }
    // A non-integer element is refused at its own pointer, as `scale` is.
    {
        let d = with_readings(json!([{"state": "valid", "firmware": "1", "range": [0.5, 1]}]));
        assert_eq!(refused(&d).1, format!("{AT}/0/range/0"));
    }
}

#[test]
fn the_source_of_a_control_reading_names_an_id_of_the_sources_of_the_file() {
    let ok = cc_doc(
        json!({"osc": "/a", "ways": cc_way(), "readings": [{"state": "valid", "firmware": "1", "source": "manual"}]}),
        json!({"sources": [{"type": "vendor-doc", "id": "manual"}]}),
    );
    assert_eq!(cc_entry(&ok).readings.unwrap()[0].source.as_deref(), Some("manual"));
    let none = with_readings(json!([{"state": "valid", "firmware": "1", "source": "manual"}]));
    assert_eq!(refused(&none), (FaultKind::BadValue, format!("{AT}/0/source")));
    let ghost = cc_doc(
        json!({"osc": "/a", "ways": cc_way(), "readings": [r("valid", "1"), {"state": "valid", "firmware": "1", "source": "ghost"}]}),
        json!({"sources": [{"type": "vendor-doc", "id": "manual"}]}),
    );
    assert_eq!(refused(&ghost), (FaultKind::BadValue, format!("{AT}/1/source")));
}

#[test]
fn the_source_of_a_params_reading_names_an_id_of_the_sources_of_the_file() {
    let mut d = params_doc(json!({"osc": "/a", "ways": [{"ctrlType": "sysex", "id": 5}], "readings": [{"state": "valid", "firmware": "1", "source": "ghost"}]}));
    assert_eq!(refused(&d), (FaultKind::BadValue, "/params/entries/0/readings/0/source".into()));
    d["sources"] = json!([{"type": "vendor-doc", "id": "ghost"}]);
    assert!(osd::read(&d.to_string(), Kind::Device).is_ok());
}

#[test]
fn the_faults_of_a_params_reading_are_at_its_own_place() {
    let entry = |reading: Value| params_doc(json!({"osc": "/a", "ways": [{"ctrlType": "sysex", "id": 5}], "readings": [reading]}));
    assert_eq!(refused(&entry(json!({"state": "nope", "firmware": "1"}))), (FaultKind::BadValue, "/params/entries/0/readings/0/state".into()));
    assert_eq!(refused(&entry(json!({"state": "valid"}))), (FaultKind::Missing, "/params/entries/0/readings/0/firmware".into()));
    assert_eq!(
        refused(&entry(json!({"state": "valid", "firmware": "1", "args": []}))),
        (FaultKind::UnknownField, "/params/entries/0/readings/0/args".into())
    );
}

// ---- the range on the reference scale ----

#[test]
fn a_range_inside_the_scale_of_the_control_bounds_included_is_read() {
    let d = with_scale(json!([0, 255]), json!([
        {"state": "valid", "firmware": "1", "range": [0, 127]},
        {"state": "valid", "firmware": "2", "range": [0, 255]},
        {"state": "valid", "firmware": "3", "range": [10, 10]},
    ]));
    let list = cc_entry(&d).readings.unwrap();
    assert_eq!((list[0].range, list[1].range, list[2].range), (Some([0, 127]), Some([0, 255]), Some([10, 10])));
}

#[test]
fn a_range_outside_the_scale_of_the_control_is_a_bad_value_at_the_range_of_the_reading() {
    for range in [json!([0, 255]), json!([-1, 127]), json!([0, 128])] {
        let d = with_scale(json!([0, 127]), json!([r("valid", "0"), {"state": "valid", "firmware": "1", "range": range}]));
        assert_eq!(refused(&d), (FaultKind::BadValue, format!("{AT}/1/range")), "{range}");
    }
}

#[test]
fn a_range_whose_low_is_above_its_high_is_a_bad_value_at_the_range() {
    let d = with_scale(json!([0, 127]), json!([{"state": "valid", "firmware": "1", "range": [100, 10]}]));
    assert_eq!(refused(&d), (FaultKind::BadValue, format!("{AT}/0/range")));
}

#[test]
fn a_range_is_read_in_the_scale_of_the_control_not_in_the_range_of_a_way() {
    // The way carries 0 to 127, the control is written in [-64, 63].
    let d = cc_doc(
        json!({"osc": "/a", "scale": [-64, 63], "ways": [{"ctrlType": "cc", "ctrlIndex": [10], "range": [0, 127]}], "readings": [
            {"state": "valid", "firmware": "1", "range": [-64, 63]}]}),
        json!({}),
    );
    assert_eq!(cc_entry(&d).readings.unwrap()[0].range, Some([-64, 63]));
    let mut out = d;
    out["cc_params"]["entries"][0]["readings"][0]["range"] = json!([0, 127]);
    assert_eq!(refused(&out), (FaultKind::BadValue, format!("{AT}/0/range")));
}

#[test]
fn without_a_scale_the_range_is_read_in_the_scale_of_the_most_precise_way() {
    let way14 = |range: Value| {
        cc_doc(
            json!({"osc": "/a", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}, {"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14}],
                   "readings": [{"state": "valid", "firmware": "1", "range": range}]}),
            json!({}),
        )
    };
    assert_eq!(cc_entry(&way14(json!([0, 16383]))).readings.unwrap()[0].range, Some([0, 16383]));
    assert_eq!(refused(&way14(json!([0, 16384]))), (FaultKind::BadValue, format!("{AT}/0/range")));
    // A single 7-bit way: 0 to 127.
    assert_eq!(refused(&with_readings(json!([{"state": "valid", "firmware": "1", "range": [0, 128]}]))), (FaultKind::BadValue, format!("{AT}/0/range")));
}

#[test]
fn a_range_of_a_control_whose_scale_is_not_known_is_not_checked() {
    let d = params_doc(json!({"osc": "/a", "ways": [{"ctrlType": "sysex", "id": 5}], "readings": [
        {"state": "valid", "firmware": "1", "range": [0, 99999]}]}));
    let list = device(&d).params.unwrap().entries.remove(0).readings.unwrap();
    assert_eq!(list[0].range, Some([0, 99999]));
}

#[test]
fn a_range_of_a_params_entry_with_a_scale_is_read_in_it() {
    let entry = |range: Value| params_doc(json!({"osc": "/a", "scale": [0, 255], "ways": [{"ctrlType": "sysex", "id": 5}], "readings": [
        {"state": "valid", "firmware": "1", "range": range}]}));
    assert!(osd::read(&entry(json!([0, 255])).to_string(), Kind::Device).is_ok());
    assert_eq!(refused(&entry(json!([0, 256]))), (FaultKind::BadValue, "/params/entries/0/readings/0/range".into()));
}

// ---- the reading a firmware holds ----

fn reading(state: &str, firmware: &str, title: &str) -> Value {
    json!({"state": state, "firmware": firmware, "title": title})
}

fn picked(readings: Value, firmware: &str, with_refuted: bool) -> Vec<String> {
    let e = cc_entry(&with_readings(readings));
    e.readings_for(firmware, with_refuted).iter().map(|x| x.title.clone().unwrap()).collect()
}

#[test]
fn the_readings_of_a_control_for_the_firmware_asked_come_valid_then_to_confirm_then_refuted_if_asked() {
    let list = json!([
        reading("refuted", "1.2", "r"),
        reading("to-confirm", "1.2", "c1"),
        reading("valid", "1.2", "v1"),
        reading("valid", "1.3", "other"),
        reading("to-confirm", "1.2", "c2"),
        reading("valid", "1.2", "v2"),
    ]);
    assert_eq!(picked(list.clone(), "1.2", false), ["v1", "v2", "c1", "c2"]);
    assert_eq!(picked(list, "1.2", true), ["v1", "v2", "c1", "c2", "r"]);
}

#[test]
fn a_control_firmware_with_no_reading_gets_all_the_non_refuted_in_the_order_written_none_chosen() {
    let list = json!([reading("valid", "1.2", "a"), reading("refuted", "1.3", "r"), reading("to-confirm", "1.4", "c")]);
    assert_eq!(picked(list.clone(), "9", false), ["a", "c"]);
    assert_eq!(picked(list, "9", true), ["a", "r", "c"]);
}

#[test]
fn a_control_firmware_is_matched_as_the_same_text_not_as_a_version() {
    let list = json!([reading("valid", "1.2", "a"), reading("valid", "1.2.0", "b")]);
    assert_eq!(picked(list, "1.2.0", false), ["b"]);
}

#[test]
fn a_firmware_with_only_refuted_readings_gives_the_others_unless_refuted_is_asked() {
    let list = json!([reading("refuted", "1.2", "r"), reading("valid", "1.3", "v")]);
    assert_eq!(picked(list.clone(), "1.2", false), ["v"]);
    assert_eq!(picked(list, "1.2", true), ["r"]);
}

#[test]
fn a_params_entry_gives_the_readings_of_a_firmware_by_the_same_rule() {
    let d = params_doc(json!({"osc": "/a", "ways": [{"ctrlType": "sysex", "id": 5}], "readings": [
        reading("to-confirm", "1", "c"), reading("valid", "1", "v"), reading("valid", "2", "w")]}));
    let e = device(&d).params.unwrap().entries.remove(0);
    let titles = |fw: &str| e.readings_for(fw, false).iter().map(|x| x.title.clone().unwrap()).collect::<Vec<_>>();
    assert_eq!(titles("1"), ["v", "c"]);
    assert_eq!(titles("2"), ["w"]);
}

// ---- before 0.7, the converter ----

#[test]
fn readings_are_an_unknown_field_of_a_control_before_0_7() {
    for version in ["0.4", "0.5", "0.6"] {
        let schema = format!("https://w3id.org/osd/{version}/device.schema.json");
        let cc = json!({"$schema": schema, "device": {"name": "T", "osc_prefix": "/t"},
            "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 74, "readings": [r("valid", "1")]}]}});
        assert_eq!(refused(&cc), (FaultKind::UnknownField, AT.into()), "{version} cc");
        let params = json!({"$schema": schema, "device": {"name": "T", "osc_prefix": "/t"},
            "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [{"osc": "/a", "id": 3, "readings": [r("valid", "1")]}]}});
        assert_eq!(refused(&params), (FaultKind::UnknownField, "/params/entries/0/readings".into()), "{version} params");
    }
}

#[test]
fn the_converter_from_0_6_writes_no_readings_on_a_control() {
    let d06 = json!({
        "$schema": D06,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 74, "range": [0, 127], "default": 64}]},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [{"osc": "/b", "id": 3}]},
    });
    let out = osd::convert_to(&d06.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&out).unwrap();
    assert!(out["cc_params"]["entries"][0].get("readings").is_none());
    assert!(out["params"]["entries"][0].get("readings").is_none());
    let back = osd::read_as(&out.to_string(), Kind::Device, Version::V0_7).unwrap();
    let Descriptor::Device(dev) = back else { panic!("a device") };
    assert!(dev.cc_params.unwrap().entries[0].readings.is_none());
    assert!(dev.params.unwrap().entries[0].readings.is_none());
}
