//! A command of 0.7 carries `readings`, each with its state and its firmware (proposal 0006,
//! `spec/0.7.md`): the form and the refusals, the `id` of a source and the `source` of a reading,
//! the reading a firmware holds, and the converter from 0.6.

use osd::{Descriptor, FaultKind, Kind, ReadingState, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn doc(schema: &str, command: Value, extra: Value) -> Value {
    let mut d = json!({"$schema": schema, "device": {"name": "T", "osc_prefix": "/t"}, "commands": [command]});
    for (k, v) in extra.as_object().unwrap() {
        d[k] = v.clone();
    }
    d
}

fn with_readings(readings: Value) -> Value {
    doc(D07, json!({"osc": "/dump", "readings": readings}), json!({}))
}

fn refused(d: &Value) -> (FaultKind, String) {
    let f = osd::read(&d.to_string(), Kind::Device).expect_err(&format!("refused: {d}"));
    (f.kind, f.pointer)
}

fn command(d: &Value) -> osd::Command {
    match osd::read(&d.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}\n{d}")) {
        Descriptor::Device(dev) => dev.commands.unwrap().remove(0),
        _ => panic!("a device"),
    }
}

fn r(state: &str, firmware: &str) -> Value {
    json!({"state": state, "firmware": firmware})
}

// ---- the form of a reading ----

#[test]
fn a_reading_gives_its_state_its_firmware_and_leaves_title_and_args_absent_when_not_written() {
    let c = command(&with_readings(json!([r("valid", "1.2")])));
    let list = c.readings.expect("readings");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].state, ReadingState::Valid);
    assert_eq!(list[0].firmware, "1.2");
    assert_eq!((list[0].source.as_deref(), list[0].title.as_deref(), list[0].args.as_ref()), (None, None, None));
}

#[test]
fn the_three_words_of_a_state_are_read() {
    let c = command(&with_readings(json!([r("valid", "1"), r("to-confirm", "1"), r("refuted", "1")])));
    let states: Vec<ReadingState> = c.readings.unwrap().iter().map(|x| x.state).collect();
    assert_eq!(states, [ReadingState::Valid, ReadingState::ToConfirm, ReadingState::Refuted]);
}

#[test]
fn readings_are_given_in_the_order_written_with_their_title_and_args_raw_and_written_back_key_for_key() {
    let d = doc(
        D07,
        json!({"osc": "/dump", "title": "Filter cutoff", "args": [{"name": "value", "type": "int", "range": [0, 1]}], "readings": [
            {"state": "valid", "firmware": "1.2", "source": "manual", "title": "Filter cutoff", "args": [{"name": "value", "type": "int", "range": [0, 127]}]},
            {"state": "to-confirm", "firmware": "1.3", "title": "Cutoff"},
        ]}),
        json!({"sources": [{"type": "vendor-doc", "id": "manual"}]}),
    );
    let c = command(&d);
    let list = c.readings.as_ref().unwrap();
    assert_eq!(list[0].firmware, "1.2");
    assert_eq!(list[0].source.as_deref(), Some("manual"));
    assert_eq!(list[0].args.as_ref().unwrap()[0].range, Some([0, 127]));
    // Raw: the second reading does not take the args of the command.
    assert_eq!((list[1].title.as_deref(), list[1].args.as_ref()), (Some("Cutoff"), None));
    let read = osd::read(&d.to_string(), Kind::Device).unwrap();
    let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
    assert_eq!(back, d);
}

#[test]
fn a_command_without_readings_has_none() {
    let c = command(&doc(D07, json!({"osc": "/dump"}), json!({})));
    assert!(c.readings.is_none());
    assert!(c.readings_for("1.2", false).is_empty());
    assert!(c.readings_for("1.2", true).is_empty());
}

// ---- the refusals ----

#[test]
fn a_reading_without_a_state_or_without_a_firmware_is_missing_at_its_place() {
    assert_eq!(refused(&with_readings(json!([{"firmware": "1.2"}]))), (FaultKind::Missing, "/commands/0/readings/0/state".into()));
    assert_eq!(refused(&with_readings(json!([r("valid", "1"), {"state": "valid"}]))), (FaultKind::Missing, "/commands/0/readings/1/firmware".into()));
}

#[test]
fn a_state_outside_the_three_words_is_a_bad_value_at_the_state() {
    for word in ["probably", "Valid", "to_confirm", "confirmed", ""] {
        assert_eq!(refused(&with_readings(json!([r(word, "1.2")]))), (FaultKind::BadValue, "/commands/0/readings/0/state".into()), "{word}");
    }
    assert_eq!(refused(&with_readings(json!([{"state": 1, "firmware": "1"}]))).1, "/commands/0/readings/0/state");
}

#[test]
fn an_empty_readings_list_is_a_bad_value() {
    assert_eq!(refused(&with_readings(json!([]))), (FaultKind::BadValue, "/commands/0/readings".into()));
}

#[test]
fn an_empty_title_of_a_reading_is_a_bad_value() {
    let d = with_readings(json!([{"state": "valid", "firmware": "1", "title": ""}]));
    assert_eq!(refused(&d), (FaultKind::BadValue, "/commands/0/readings/0/title".into()));
}

#[test]
fn the_args_of_a_reading_follow_the_form_of_the_args_of_a_command() {
    let bad = with_readings(json!([{"state": "valid", "firmware": "1", "args": [{"name": "v", "type": "nonsense"}]}]));
    assert_eq!(refused(&bad), (FaultKind::BadValue, "/commands/0/readings/0/args/0/type".into()));
    let missing = with_readings(json!([{"state": "valid", "firmware": "1", "args": [{"type": "int"}]}]));
    assert_eq!(refused(&missing), (FaultKind::Missing, "/commands/0/readings/0/args/0/name".into()));
}

#[test]
fn an_unknown_key_of_a_reading_is_an_unknown_field() {
    let d = with_readings(json!([{"state": "valid", "firmware": "1", "confidence": 3}]));
    assert_eq!(refused(&d), (FaultKind::UnknownField, "/commands/0/readings/0/confidence".into()));
    // `range` is the reading of a control, not of a command.
    let d = with_readings(json!([{"state": "valid", "firmware": "1", "range": [0, 1]}]));
    assert_eq!(refused(&d), (FaultKind::UnknownField, "/commands/0/readings/0/range".into()));
}

#[test]
fn readings_are_an_unknown_field_of_a_command_before_0_7() {
    for version in ["0.5", "0.6"] {
        let d = doc(&format!("https://w3id.org/osd/{version}/device.schema.json"), json!({"osc": "/dump", "readings": [r("valid", "1")]}), json!({}));
        assert_eq!(refused(&d), (FaultKind::UnknownField, "/commands/0/readings".into()), "{version}");
    }
    let d = doc("", json!({"osc": "/dump", "readings": [r("valid", "1")]}), json!({}));
    let mut d = d;
    d.as_object_mut().unwrap().remove("$schema");
    assert_eq!(refused(&d), (FaultKind::UnknownField, "/commands/0/readings".into()));
}

// ---- the `id` of a source and the `source` of a reading ----

#[test]
fn a_source_of_0_7_accepts_an_id_and_an_empty_id_is_a_bad_value() {
    let d = doc(D07, json!({"osc": "/dump"}), json!({"sources": [{"type": "vendor-doc", "id": "manual"}, {"type": "vendor-doc"}]}));
    let Descriptor::Device(dev) = osd::read(&d.to_string(), Kind::Device).unwrap() else { panic!("a device") };
    let sources = dev.sources.unwrap();
    assert_eq!((sources[0].id.as_deref(), sources[1].id.as_deref()), (Some("manual"), None));
    let empty = doc(D07, json!({"osc": "/dump"}), json!({"sources": [{"type": "vendor-doc", "id": ""}]}));
    assert_eq!(refused(&empty), (FaultKind::BadValue, "/sources/0/id".into()));
}

#[test]
fn an_id_written_twice_in_the_sources_is_a_bad_value_at_the_second_entry() {
    let d = doc(D07, json!({"osc": "/dump"}), json!({"sources": [{"type": "vendor-doc", "id": "a"}, {"type": "vendor-doc", "id": "b"}, {"type": "vendor-doc", "id": "a"}]}));
    assert_eq!(refused(&d), (FaultKind::BadValue, "/sources/2".into()));
}

#[test]
fn an_id_written_twice_in_midi_of_sources_is_a_bad_value_at_the_second_entry() {
    let d = doc(D07, json!({"osc": "/dump"}), json!({"midi_of_sources": [{"type": "vendor-doc", "id": "a"}, {"type": "vendor-doc", "id": "a"}]}));
    assert_eq!(refused(&d), (FaultKind::BadValue, "/midi_of_sources/1".into()));
}

#[test]
fn before_0_7_an_id_of_a_source_is_a_kept_key_never_checked() {
    for version in ["0.5", "0.6"] {
        let d = json!({
            "$schema": format!("https://w3id.org/osd/{version}/device.schema.json"),
            "device": {"name": "T", "osc_prefix": "/t"},
            "_sources": [{"type": "vendor-doc", "id": "a"}, {"type": "vendor-doc", "id": "a"}, {"type": "vendor-doc", "id": ""}],
        });
        let read = osd::read(&d.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{version}: {f}"));
        let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
        assert_eq!(back["_sources"], d["_sources"], "{version}");
    }
}

#[test]
fn the_source_of_a_reading_names_an_id_of_the_sources_of_the_file() {
    let d = doc(
        D07,
        json!({"osc": "/dump", "readings": [{"state": "valid", "firmware": "1", "source": "manual"}]}),
        json!({"sources": [{"type": "vendor-doc", "id": "manual"}]}),
    );
    assert_eq!(command(&d).readings.unwrap()[0].source.as_deref(), Some("manual"));
}

#[test]
fn a_source_that_names_no_id_is_a_bad_value_at_source() {
    let no_sources = with_readings(json!([{"state": "valid", "firmware": "1", "source": "manual"}]));
    assert_eq!(refused(&no_sources), (FaultKind::BadValue, "/commands/0/readings/0/source".into()));
    let other = doc(
        D07,
        json!({"osc": "/dump", "readings": [r("valid", "1"), {"state": "valid", "firmware": "1", "source": "ghost"}]}),
        json!({"sources": [{"type": "vendor-doc", "id": "manual"}]}),
    );
    assert_eq!(refused(&other), (FaultKind::BadValue, "/commands/0/readings/1/source".into()));
    let empty = with_readings(json!([{"state": "valid", "firmware": "1", "source": ""}]));
    assert_eq!(refused(&empty).1, "/commands/0/readings/0/source");
}

// ---- a file with midi_of ----

fn files(list: &[(&str, Value)]) -> BTreeMap<String, String> {
    list.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn device(extra: Value) -> Value {
    let mut d = json!({"$schema": D07, "device": {"name": "T", "osc_prefix": "/t"}});
    for (k, v) in extra.as_object().unwrap() {
        d[k] = v.clone();
    }
    d
}

#[test]
fn an_id_of_a_file_with_midi_of_that_its_target_has_is_a_bad_value_in_the_file_of_the_duplicate() {
    let f = files(&[
        ("acme/a", device(json!({"midi_of": "acme/b", "sources": [{"type": "vendor-doc", "id": "x"}]}))),
        ("acme/b", device(json!({"sources": [{"type": "vendor-doc", "id": "x"}]}))),
    ]);
    let e = osd::resolve("acme/a", &f).expect_err("refused");
    assert_eq!((e.kind, e.file.as_deref(), e.pointer.as_str()), (FaultKind::BadValue, Some("acme/a"), "/sources/0"));
}

#[test]
fn the_source_of_a_reading_of_a_file_with_midi_of_is_resolved_in_the_union_with_its_target() {
    let a = device(json!({"midi_of": "acme/b", "commands": [{"osc": "/dump", "readings": [{"state": "valid", "firmware": "1", "source": "theirs"}, {"state": "valid", "firmware": "2", "source": "mine"}]}], "sources": [{"type": "vendor-doc", "id": "mine"}]}));
    let b = device(json!({"sources": [{"type": "vendor-doc", "id": "theirs"}]}));
    let r = osd::resolve("acme/a", &files(&[("acme/a", a.clone()), ("acme/b", b.clone())])).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.device.commands.unwrap()[0].readings.as_ref().unwrap().len(), 2);
    // A source that is in neither is refused at `source`, in the file of the reading.
    let mut bad = a;
    bad["commands"][0]["readings"][0]["source"] = json!("ghost");
    let e = osd::resolve("acme/a", &files(&[("acme/a", bad), ("acme/b", b)])).expect_err("refused");
    assert_eq!((e.kind, e.file.as_deref(), e.pointer.as_str()), (FaultKind::BadValue, Some("acme/a"), "/commands/0/readings/0/source"));
}

// ---- the reading a firmware holds ----

fn reading(state: &str, firmware: &str, title: &str) -> Value {
    json!({"state": state, "firmware": firmware, "title": title})
}

fn picked(readings: Value, firmware: &str, with_refuted: bool) -> Vec<String> {
    let c = command(&with_readings(readings));
    c.readings_for(firmware, with_refuted).iter().map(|x| x.title.clone().unwrap()).collect()
}

#[test]
fn the_readings_of_the_firmware_asked_come_valid_then_to_confirm_then_refuted_if_asked() {
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
fn two_valid_readings_on_one_firmware_are_both_given_in_the_order_written() {
    let list = json!([reading("valid", "1.2", "first"), reading("valid", "1.2", "second")]);
    assert_eq!(picked(list, "1.2", false), ["first", "second"]);
}

#[test]
fn the_figure_of_the_proposal_picks_by_firmware() {
    let list = json!([reading("valid", "1.2", "Filter cutoff"), reading("to-confirm", "1.3", "Cutoff")]);
    assert_eq!(picked(list.clone(), "1.2", false), ["Filter cutoff"]);
    assert_eq!(picked(list.clone(), "1.3", false), ["Cutoff"]);
    // A firmware with no reading: all of them, in the order written, none chosen.
    assert_eq!(picked(list.clone(), "1.4", false), ["Filter cutoff", "Cutoff"]);
    // `unknown` is a firmware text like another.
    assert_eq!(picked(list, "unknown", false), ["Filter cutoff", "Cutoff"]);
    let unknown = json!([reading("valid", "1.2", "a"), reading("to-confirm", "unknown", "b")]);
    assert_eq!(picked(unknown, "unknown", false), ["b"]);
}

#[test]
fn a_firmware_is_matched_as_the_same_text_not_as_a_version() {
    let list = json!([reading("valid", "1.2", "a"), reading("valid", "1.2.0", "b")]);
    assert_eq!(picked(list, "1.2.0", false), ["b"]);
}

#[test]
fn a_firmware_with_only_refuted_readings_gives_the_other_readings_of_the_command_unless_refuted_is_asked() {
    let list = json!([reading("refuted", "1.2", "r"), reading("valid", "1.3", "v"), reading("refuted", "1.4", "r2"), reading("to-confirm", "1.5", "c")]);
    assert_eq!(picked(list.clone(), "1.2", false), ["v", "c"]);
    assert_eq!(picked(list, "1.2", true), ["r"]);
}

#[test]
fn a_command_whose_only_readings_are_refuted_gives_none_unless_asked() {
    let list = json!([reading("refuted", "1.2", "r")]);
    assert!(picked(list.clone(), "1.2", false).is_empty());
    assert!(picked(list.clone(), "1.3", false).is_empty());
    assert_eq!(picked(list.clone(), "1.2", true), ["r"]);
    // No reading of that firmware and `with_refuted`: all the readings, refuted included.
    assert_eq!(picked(list, "1.3", true), ["r"]);
}

// ---- the converter ----

#[test]
fn the_converter_from_0_6_writes_no_readings_and_leaves_a_command_and_the_id_of_a_source_as_they_are() {
    let d06 = json!({
        "$schema": D06,
        "device": {"name": "T", "osc_prefix": "/t"},
        "commands": [{"osc": "/dump", "section": "Utility", "args": [{"name": "value", "type": "int", "range": [0, 127]}]}],
        "_sources": [{"type": "vendor-doc", "id": "kept", "firmware": "1.2"}],
    });
    let out = osd::convert_to(&d06.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(out["commands"], d06["commands"]);
    assert!(out["commands"][0].get("readings").is_none());
    assert_eq!(out["sources"], d06["_sources"]);
    let back = osd::read_as(&out.to_string(), Kind::Device, Version::V0_7).unwrap();
    let Descriptor::Device(dev) = back else { panic!("a device") };
    assert!(dev.commands.unwrap()[0].readings.is_none());
}
