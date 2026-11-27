//! The table of the OSD 0.7 device in docs/midi-csv.md and docs/electra-one.md: one row per
//! property of the device schema in every object the format enters, its fate `carried`, `lost` or
//! `conditional` with its reason or condition, each row checked on devices that fill its field
//! (osd-formats/docs/CADRE.md R8, docs/CADRE.md R16).

use crate::common::fields::{base, differences, device_rows, held, named, row_of, table, trip, under};
use crate::common::knobkraft;
use osd_formats::Format;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const HEADER: &str = "OSD field";
const E: &str = "/cc_params/entries/0";
const W: &str = "/cc_params/entries/0/ways";
const C: &str = "/commands/0";
const A: &str = "/commands/0/args/0";

/// One device that fills a field: what changes in the base device, the pointer of the field in it,
/// and whether an export then an import gives the field back.
struct Case {
    edits: Vec<(String, Value)>,
    tweak: Option<fn(&mut osd::Device)>,
    pointer: String,
    kept: bool,
}

fn case(kept: bool, pointer: impl Into<String>, edits: &[(&str, Value)]) -> Case {
    Case { edits: edits.iter().map(|(p, v)| (p.to_string(), v.clone())).collect(), tweak: None, pointer: pointer.into(), kept }
}

fn keep(pointer: impl Into<String>, edits: &[(&str, Value)]) -> Case {
    case(true, pointer, edits)
}

fn lose(pointer: impl Into<String>, edits: &[(&str, Value)]) -> Case {
    case(false, pointer, edits)
}

/// A field of the control, kept or lost, set to this value.
fn entry(kept: bool, field: &str, value: Value) -> Case {
    let p = format!("{E}/{field}");
    case(kept, p.clone(), &[(&p, value)])
}

/// A field of the command, kept or lost, set to this value.
fn command(kept: bool, field: &str, value: Value) -> Case {
    let p = format!("{C}/{field}");
    case(kept, p.clone(), &[(&p, value)])
}

/// The control's ways replaced, the field at `pointer` under them kept or lost.
fn ways(kept: bool, pointer: &str, list: Value) -> Case {
    case(kept, format!("{W}/{pointer}"), &[(W, list)])
}

/// A control without a way, which holds a field of an entry.
fn wayless(field: &str, value: Value) -> Case {
    let p = format!("{E}/{field}");
    lose(p.clone(), &[(W, json!([])), (&p, value)])
}

/// A field of the device's source, lost, filled on the device read.
fn source_field(field: &str, tweak: fn(&mut osd::Device)) -> Case {
    Case { edits: vec![], tweak: Some(tweak), pointer: format!("/sources/0/{field}"), kept: false }
}

fn first_source(d: &mut osd::Device) -> &mut osd::Source {
    &mut d.sources.as_mut().unwrap()[0]
}

/// The command given this frame and this argument, the field of the argument kept or lost.
fn args(kept: bool, field: &str, frame: Value, arg: Value) -> Case {
    case(kept, format!("{A}/{field}"), &[("/commands/0/frame", frame), ("/commands/0/args", json!([arg]))])
}

fn cc(n: u64) -> Value {
    json!({"ctrlType": "cc", "ctrlIndex": [n]})
}

fn cc7(n: u64) -> Value {
    json!({"ctrlType": "cc", "ctrlIndex": [n], "value_bits": 7})
}

fn cc14() -> Value {
    json!({"ctrlType": "cc", "ctrlIndex": [5, 37], "value_bits": 14})
}

fn nrpn(bits: u64) -> Value {
    json!({"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": bits})
}

fn rpn(bits: u64) -> Value {
    json!({"ctrlType": "rpn", "ctrlIndex": [0, 1], "value_bits": bits})
}

fn with(mut way: Value, key: &str, value: Value) -> Value {
    way[key] = value;
    way
}

fn value_frame() -> Value {
    json!([240, 1, {"arg": "value"}, 247])
}

/// A frame with an added constant, which no template of the Electra One holds.
fn added_frame() -> Value {
    json!([240, 1, {"arg": "value", "add": 1}, 247])
}

fn u7() -> Value {
    json!({"name": "value", "type": "u7"})
}

/// The cases of the fields both formats treat alike.
fn shared(path: &str) -> Option<Vec<Case>> {
    let block = |key: &str, value: Value| {
        let p = format!("/{key}");
        vec![lose(p.clone(), &[(&p, value)])]
    };
    Some(match path {
        "/$schema" => vec![keep("/$schema", &[])],
        // A device with a field that holds Lua declares it.
        "/capabilities" => vec![lose("/capabilities", &[("/capabilities", json!(["lua"])), ("/custom_commands", json!([{"osc": "/c", "script": "x"}]))])],
        "/role_vocabulary" => block("role_vocabulary", json!("1")),
        "/midi_of" => block("midi_of", json!("acme/other")),
        "/contexts" => block("contexts", json!({"part": {"values": [{"id": "a"}]}})),
        "/loads_sounds_of" => block("loads_sounds_of", json!(["acme/other"])),
        "/sound_transfers" => block("sound_transfers", knobkraft::base()["sound_transfers"].clone()),
        "/resources" => block("resources", json!([{"resource": "patch", "canSet": "none", "export": "sysex"}])),
        "/identity" => block("identity", json!({"manufacturerId": [0, 32, 41]})),
        "/programs" => block("programs", json!({"banks": [{"name": "A", "size": 64}]})),
        "/sysex" => block("sysex", json!({"header": [240]})),
        "/params" => block("params", json!({"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7",
            "entries": [{"osc": "/p", "ways": [{"ctrlType": "sysex", "id": 5}]}]})),
        "/triggers" => block("triggers", json!({"entries": [{"osc": "/kick", "sound": "Kick", "note_range": [36, 36]}]})),
        "/midi_out" => block("midi_out", json!({"default_channel": 1})),
        "/midi_in" => block("midi_in", json!(["cc"])),
        "/replies" => block("replies", json!([{"match_osc": "/y", "emit_osc": "/x"}])),
        "/subscriptions" => block("subscriptions", json!([{"forward": {"path": "/x"}}])),
        "/custom_commands" => vec![lose("/custom_commands", &[
            ("/capabilities", json!(["lua"])), ("/custom_commands", json!([{"osc": "/c", "script": "x"}]))])],
        "/midi_of_sources" => block("midi_of_sources", json!([{"type": "vendor-doc", "url": "https://example.org/m", "license": "MIT"}])),
        "/_coverage" => block("_coverage", json!({"controls": 1})),
        "/_*" => block("_note", json!("a note")),
        "/sources" => vec![lose("/sources", &[])],
        "/device" => vec![keep("/device", &[]), lose("/device", &[("/device/kind", json!("hardware"))])],
        "/device/osc_prefix" => vec![keep("/device/osc_prefix", &[]), lose("/device/osc_prefix", &[("/device/osc_prefix", json!("/other"))])],
        "/device/device_id" => vec![lose("/device/device_id", &[("/device/device_id", json!([16]))])],
        "/device/rate_limit_hz" => vec![lose("/device/rate_limit_hz", &[("/device/rate_limit_hz", json!(10))])],
        "/device/kind" => vec![lose("/device/kind", &[("/device/kind", json!("hardware"))])],
        "/device/stands_for" => vec![lose("/device/stands_for", &[("/device/kind", json!("software")), ("/device/stands_for", json!("acme/box"))])],
        "/device/fidelity" => vec![lose("/device/fidelity", &[
            ("/device/kind", json!("software")), ("/device/stands_for", json!("acme/box")), ("/device/fidelity", json!("modelled"))])],
        "/device/transport" => vec![lose("/device/transport", &[("/device/transport", json!({"kind": "midi"}))])],
        // A key that a JSON pointer escapes.
        "/device/_*" => vec![lose("/device/_a~1b", &[("/device/_a~1b", json!("a note"))])],
        "/cc_params" => vec![keep("/cc_params", &[]), lose("/cc_params", &[("/cc_params/channel", json!(3))])],
        "/cc_params/channel" => vec![lose("/cc_params/channel", &[("/cc_params/channel", json!(3))])],
        "/cc_params/entries" => vec![
            keep("/cc_params/entries", &[]),
            lose("/cc_params/entries", &[("/cc_params/entries/1", json!({"osc": "/x", "ways": []}))]),
        ],
        "/cc_params/_*" => vec![lose("/cc_params/_note", &[("/cc_params/_note", json!("a note"))])],
        "/cc_params/entries/N/osc" => vec![entry(true, "osc", json!("/filter/cutoff")), entry(false, "osc", json!("/other"))],
        "/cc_params/entries/N/range" => vec![wayless("range", json!([0, 3]))],
        "/cc_params/entries/N/transmit" => vec![wayless("transmit", json!("absolute"))],
        "/cc_params/entries/N/recognize" => vec![wayless("recognize", json!("both"))],
        "/cc_params/entries/N/nrpn_lsb_range" => vec![wayless("nrpn_lsb_range", json!([0, 15]))],
        "/cc_params/entries/N/priority" => vec![entry(false, "priority", json!(1))],
        "/cc_params/entries/N/step_count" => vec![entry(false, "step_count", json!(8))],
        "/cc_params/entries/N/channel" => vec![entry(false, "channel", json!(3))],
        "/cc_params/entries/N/transform" => vec![case(false, format!("{E}/transform"), &[
            ("/capabilities", json!(["lua"])), ("/cc_params/entries/0/transform", json!("x * 2"))])],
        "/cc_params/entries/N/role" => vec![case(false, format!("{E}/role"), &[
            ("/role_vocabulary", json!("1")), ("/cc_params/entries/0/role", json!({"parts": [{"part": "filter"}], "setting": "cutoff"}))])],
        "/cc_params/entries/N/readings" => vec![entry(false, "readings", json!([{"state": "valid", "firmware": "1.0"}]))],
        "/cc_params/entries/N/_*" => vec![entry(false, "_note", json!("a note"))],
        "/cc_params/entries/N/ways/M/nrpn_lsb_range" => vec![ways(false, "0/nrpn_lsb_range", json!([with(nrpn(14), "nrpn_lsb_range", json!([0, 10]))]))],
        "/cc_params/entries/N/ways/M/in" => vec![case(false, format!("{W}/0/in"), &[
            ("/contexts", json!({"part": {"values": [{"id": "a"}]}})), (W, json!([with(cc7(74), "in", json!({"part": "a"}))]))])],
        "/cc_params/entries/N/ways/M/slice" => vec![ways(false, "0/slice", json!([with(cc7(74), "slice", json!([0, 63]))]))],
        "/cc_params/entries/N/ways/M/_*" => vec![ways(false, "0/_note", json!([with(cc7(74), "_note", json!("a note"))]))],
        "/sources/N/id" => vec![source_field("id", |d| first_source(d).id = Some("s1".into()))],
        "/sources/N/type" => vec![source_field("type", |d| first_source(d).kind = "vendor-doc".into())],
        "/sources/N/provider" => vec![source_field("provider", |d| first_source(d).provider = Some("pencilresearch".into()))],
        "/sources/N/url" => vec![source_field("url", |d| first_source(d).url = Some("https://example.org/d".into()))],
        "/sources/N/firmware" => vec![source_field("firmware", |d| first_source(d).firmware = Some("1.0".into()))],
        "/sources/N/author" => vec![source_field("author", |d| first_source(d).author = Some("Dee".into()))],
        "/sources/N/attribution" => vec![source_field("attribution", |d| first_source(d).attribution = Some("Thanks".into()))],
        "/sources/N/license" => vec![source_field("license", |d| first_source(d).license = Some("CC0-1.0".into()))],
        "/sources/N/commit" => vec![source_field("commit", |d| first_source(d).commit = Some("abc".into()))],
        "/sources/N/profile" => vec![source_field("profile", |d| {
            first_source(d).extra.insert("profile".into(), json!("p"));
        })],
        "/sources/N/imported_at" => vec![source_field("imported_at", |d| first_source(d).imported_at = Some("2026-01-02".into()))],
        "/sources/N/notes" => vec![source_field("notes", |d| first_source(d).notes = Some("n".into()))],
        "/sources/N/_*" => vec![source_field("_note", |d| {
            first_source(d).extra.insert("_note".into(), json!("a note"));
        })],
        _ => return None,
    })
}

fn midi_cases(path: &str) -> Vec<Case> {
    // Two ways of different scales and a default: the import writes the scale back.
    let scaled = || vec![(W, json!([cc(74), nrpn(14)])), ("/cc_params/entries/0/default", json!(64)), ("/cc_params/entries/0/scale", json!([0, 127]))];
    match path {
        "/commands" => vec![lose("/commands", &[("/commands", json!([{"osc": "/dump", "frame": [240, 247]}]))])],
        "/device/name" => vec![keep("/device/name", &[]), lose("/device/name", &[("/cc_params", Value::Null)])],
        "/device/vendor" => vec![keep("/device/vendor", &[]), lose("/device/vendor", &[("/cc_params", Value::Null)])],
        "/cc_params/entries/N/title" => vec![keep(format!("{E}/title"), &[]), lose(format!("{E}/title"), &[("/cc_params/entries/0/title", Value::Null)])],
        "/cc_params/entries/N/description" => vec![entry(true, "description", json!("Opens the filter"))],
        "/cc_params/entries/N/value_description" => vec![entry(true, "value_description", json!("0 closed"))],
        "/cc_params/entries/N/notes" => vec![entry(true, "notes", json!("a note"))],
        "/cc_params/entries/N/scale" => vec![keep(format!("{E}/scale"), &scaled()), entry(false, "scale", json!([0, 127]))],
        "/cc_params/entries/N/default" => vec![
            entry(true, "default", json!(64)),
            case(false, format!("{E}/default"), &[(W, json!([rpn(14), cc(74)])), ("/cc_params/entries/0/default", json!(5))]),
        ],
        "/cc_params/entries/N/values" => vec![entry(false, "values", json!([{"value": 0, "title": "Off"}]))],
        "/cc_params/entries/N/polarity" => vec![entry(true, "polarity", json!("bipolar"))],
        "/cc_params/entries/N/section" => vec![keep(format!("{E}/section"), &[])],
        "/cc_params/entries/N/expects" => vec![entry(false, "expects", json!("continuous"))],
        "/cc_params/entries/N/ways" => vec![keep(W, &[]), lose(W, &[(W, json!([cc(74), {"ctrlType": "pBend"}]))])],
        "/cc_params/entries/N/ways/M/ctrlType" => vec![
            ways(true, "0/ctrlType", json!([cc(74)])),
            ways(false, "1/ctrlType", json!([cc(74), {"ctrlType": "pBend"}])),
        ],
        "/cc_params/entries/N/ways/M/ctrlIndex" => vec![
            ways(true, "0/ctrlIndex", json!([cc(74)])),
            ways(false, "1/ctrlIndex", json!([cc(74), {"ctrlType": "pPress", "ctrlIndex": [60]}])),
        ],
        "/cc_params/entries/N/ways/M/value_bits" => vec![
            ways(true, "0/value_bits", json!([cc14()])),
            ways(true, "0/value_bits", json!([nrpn(14)])),
            ways(false, "0/value_bits", json!([nrpn(7)])),
            ways(false, "0/value_bits", json!([cc7(74)])),
        ],
        "/cc_params/entries/N/ways/M/range" => vec![
            ways(true, "0/range", json!([with(cc(74), "range", json!([10, 100]))])),
            ways(true, "0/range", json!([with(nrpn(14), "range", json!([0, 1000]))])),
            // The row writes the scale of the way without a range: it comes back with one.
            ways(false, "1/range", json!([with(cc(74), "range", json!([0, 100])), nrpn(14)])),
            ways(false, "1/range", json!([cc(74), with(rpn(14), "range", json!([0, 10]))])),
        ],
        "/cc_params/entries/N/ways/M/transmit" => vec![ways(false, "0/transmit", json!([with(cc(74), "transmit", json!("relative"))]))],
        "/cc_params/entries/N/ways/M/recognize" => vec![ways(false, "0/recognize", json!([with(cc(74), "recognize", json!("relative"))]))],
        other => shared(other).unwrap_or_else(|| panic!("no case for the row `{other}`: a property of the schema needs its case")),
    }
}

fn electra_cases(path: &str) -> Vec<Case> {
    let long = "A name far too long for the controller to show whole";
    let relative = || with(with(cc7(74), "transmit", json!("relative")), "recognize", json!("relative"));
    let named_values = |a: &str, b: &str| json!([{"value": 0, "title": a}, {"value": 1, "title": b}]);
    match path {
        "/device/name" => vec![keep("/device/name", &[]), lose("/device/name", &[("/device/name", json!(long))])],
        "/device/vendor" => vec![lose("/device/vendor", &[("/device/vendor", json!("Lofty"))])],
        "/cc_params/entries/N/title" => vec![
            keep(format!("{E}/title"), &[]),
            lose(format!("{E}/title"), &[("/cc_params/entries/0/title", Value::Null)]),
            entry(false, "title", json!(long)),
        ],
        "/cc_params/entries/N/description" => vec![entry(false, "description", json!("Opens the filter"))],
        "/cc_params/entries/N/value_description" => vec![entry(false, "value_description", json!("0 closed"))],
        "/cc_params/entries/N/notes" => vec![entry(false, "notes", json!("a note"))],
        "/cc_params/entries/N/scale" => vec![entry(false, "scale", json!([0, 127]))],
        "/cc_params/entries/N/default" => vec![entry(false, "default", json!(64))],
        "/cc_params/entries/N/values" => vec![entry(true, "values", named_values("Off", "On")), entry(false, "values", named_values("Off", long))],
        "/cc_params/entries/N/polarity" => vec![entry(false, "polarity", json!("bipolar"))],
        "/cc_params/entries/N/section" => vec![keep(format!("{E}/section"), &[]), entry(false, "section", json!(long))],
        "/cc_params/entries/N/expects" => vec![entry(false, "expects", json!("switch")), entry(false, "expects", json!("continuous"))],
        "/cc_params/entries/N/ways" => vec![keep(W, &[]), lose(W, &[(W, json!([cc7(74), {"ctrlType": "pBend"}]))])],
        "/cc_params/entries/N/ways/M/ctrlType" => vec![
            ways(true, "0/ctrlType", json!([cc7(74)])),
            ways(false, "1/ctrlType", json!([cc7(74), {"ctrlType": "pBend"}])),
        ],
        "/cc_params/entries/N/ways/M/ctrlIndex" => vec![
            ways(true, "0/ctrlIndex", json!([cc14()])),
            ways(false, "0/ctrlIndex", json!([{"ctrlType": "cc", "ctrlIndex": [40, 50], "value_bits": 14}])),
        ],
        "/cc_params/entries/N/ways/M/value_bits" => vec![
            ways(true, "0/value_bits", json!([cc7(74)])),
            ways(true, "0/value_bits", json!([cc14()])),
            ways(true, "0/value_bits", json!([nrpn(14)])),
            ways(true, "0/value_bits", json!([rpn(14)])),
            ways(false, "0/value_bits", json!([nrpn(7)])),
            ways(false, "0/value_bits", json!([rpn(7)])),
            // The import writes the width of a CC: it is added.
            ways(false, "0/value_bits", json!([cc(74)])),
        ],
        "/cc_params/entries/N/ways/M/range" => vec![
            ways(true, "0/range", json!([with(cc7(74), "range", json!([10, 100]))])),
            ways(true, "0/range", json!([with(cc7(74), "range", json!([0, 127]))])),
            ways(true, "0/range", json!([with(nrpn(14), "range", json!([0, 1000]))])),
            ways(false, "1/range", json!([cc7(74), with(cc7(75), "range", json!([0, 10]))])),
        ],
        "/cc_params/entries/N/ways/M/transmit" => vec![
            ways(true, "0/transmit", json!([relative()])),
            ways(false, "0/transmit", json!([with(cc7(74), "transmit", json!("both"))])),
        ],
        "/cc_params/entries/N/ways/M/recognize" => vec![
            ways(true, "0/recognize", json!([relative()])),
            ways(false, "0/recognize", json!([with(cc7(74), "recognize", json!("none"))])),
        ],
        "/commands" => vec![keep("/commands", &[]), lose("/commands", &[("/commands/1", json!({"osc": "/osc-only"}))])],
        "/commands/N/osc" => vec![command(true, "osc", json!("/filter/dump")), command(false, "osc", json!("/other"))],
        "/commands/N/id" => vec![command(false, "id", json!(3))],
        "/commands/N/title" => vec![
            keep(format!("{C}/title"), &[]),
            lose(format!("{C}/title"), &[("/commands/0/title", Value::Null)]),
            command(false, "title", json!(long)),
        ],
        "/commands/N/description" => vec![command(false, "description", json!("Asks for a dump"))],
        "/commands/N/section" => vec![keep(format!("{C}/section"), &[]), command(false, "section", json!(long))],
        "/commands/N/notes" => vec![command(false, "notes", json!("a note"))],
        "/commands/N/args" => vec![
            case(true, format!("{C}/args"), &[("/commands/0/frame", value_frame()), ("/commands/0/args", json!([u7()]))]),
            case(false, format!("{C}/args"), &[
                ("/commands/0/frame", value_frame()), ("/commands/0/args", json!([u7(), {"name": "other", "type": "u7"}]))]),
        ],
        "/commands/N/readings" => vec![command(false, "readings", json!([{"state": "valid", "firmware": "1.0"}]))],
        "/commands/N/pre" => vec![command(false, "pre", json!([{"bind": "x", "expr": 1}]))],
        "/commands/N/frame" => vec![
            keep(format!("{C}/frame"), &[]),
            case(false, format!("{C}/frame"), &[("/commands/0/frame", added_frame()), ("/commands/0/args", json!([u7()]))]),
        ],
        "/commands/N/forward" => vec![command(false, "forward", json!({"path": "/x"}))],
        "/commands/N/script" => vec![case(false, format!("{C}/script"), &[("/capabilities", json!(["lua"])), ("/commands/0/script", json!("x"))])],
        "/commands/N/_*" => vec![command(false, "_note", json!("a note"))],
        "/commands/N/args/M/name" => vec![
            args(true, "name", value_frame(), u7()),
            args(false, "name", json!([240, 1, {"arg": "v"}, 247]), json!({"name": "v", "type": "u7"})),
        ],
        "/commands/N/args/M/type" => vec![
            args(true, "type", value_frame(), u7()),
            args(true, "type", json!([240, {"arg": "value", "encoding": "u14_lsb"}, {"arg": "value", "encoding": "u14_msb"}, 247]),
                json!({"name": "value", "type": "u14"})),
            args(false, "type", value_frame(), json!({"name": "value", "type": "int"})),
        ],
        "/commands/N/args/M/range" => vec![
            args(true, "range", value_frame(), with(u7(), "range", json!([1, 10]))),
            args(false, "range", added_frame(), with(u7(), "range", json!([1, 10]))),
        ],
        "/commands/N/args/M/values" => vec![args(false, "values", value_frame(), with(u7(), "values", json!({"on": 1})))],
        "/commands/N/args/M/default" => vec![args(false, "default", value_frame(), with(u7(), "default", json!(3)))],
        "/commands/N/args/M/_*" => vec![args(false, "_note", value_frame(), with(u7(), "_note", json!("a note")))],
        other => shared(other).unwrap_or_else(|| panic!("no case for the row `{other}`: a property of the schema needs its case")),
    }
}

fn cases_of(format: Format, path: &str) -> Vec<Case> {
    if format == Format::ElectraOne {
        electra_cases(path)
    } else {
        midi_cases(path)
    }
}

fn device_of(format: Format, c: &Case) -> osd::Device {
    let edits: Vec<(&str, Value)> = c.edits.iter().map(|(p, v)| (p.as_str(), v.clone())).collect();
    let mut device = crate::common::fields::changed(format, &edits);
    if let Some(tweak) = c.tweak {
        tweak(&mut device);
    }
    device
}

/// The row of `path` in the table of the page says what the code does on each of its cases:
/// `carried` comes back equal and is not in the losses, `lost` is in the losses, `conditional`
/// shows both.
fn check(format: Format, page: &str, path: &str) {
    let cases = cases_of(format, path);
    let mut broken = Vec::new();
    for (n, c) in cases.iter().enumerate() {
        let pointer = c.pointer.as_str();
        let t = trip(format, device_of(format, c));
        let same = t.given.pointer(pointer) == t.back.pointer(pointer);
        let in_losses = named(&t.lost, pointer);
        if c.kept {
            if !same || in_losses {
                broken.push(format!(
                    "case {n}: {pointer} is to come back equal and not be named: given {:?}, back {:?}, losses {:?}",
                    t.given.pointer(pointer), t.back.pointer(pointer), t.lost
                ));
            }
            continue;
        }
        if same {
            broken.push(format!("case {n}: {pointer} comes back equal, where the format cannot give it back"));
            continue;
        }
        let silent: Vec<String> = differences(&t.given, &t.back)
            .into_iter()
            .filter(|d| (under(d, pointer) || under(pointer, d)) && !held(&t.lost, d))
            .collect();
        if !in_losses || !silent.is_empty() {
            broken.push(format!("case {n}: {pointer} is to be in the losses: silent {silent:?}, losses {:?}", t.lost));
        }
    }
    assert!(broken.is_empty(), "{}: {path}:\n{}", format.name(), broken.join("\n"));
    let fate = match (cases.iter().any(|c| c.kept), cases.iter().any(|c| !c.kept)) {
        (true, false) => "carried",
        (false, true) => "lost",
        _ => "conditional",
    };
    let row = row_of(page, HEADER, path);
    assert_eq!(row.get(1).map(String::as_str), Some(fate), "docs/{page}.md, `{path}`: its fate");
    if fate != "carried" {
        assert!(row.get(2).is_some_and(|r| !r.is_empty()), "docs/{page}.md, `{path}`: no reason or condition");
    }
}

/// The table of the page has one row per property of the schema and no other, and each row has its
/// test.
fn check_rows(format: Format, page: &str, commands: bool, paths: &[&str]) {
    let expected = device_rows(commands);
    let tested: BTreeSet<String> = paths.iter().map(|s| s.to_string()).collect();
    assert_eq!(tested, expected, "a property without its test, or a test without its property");
    // Every case is a device of 0.7 the reader reads.
    crate::common::device_from(&base(format).to_string());
    for path in paths {
        for c in cases_of(format, path) {
            device_of(format, &c);
        }
    }
    let rows: Vec<String> = table(page, HEADER).into_iter().map(|r| r[0].clone()).collect();
    let set: BTreeSet<String> = rows.iter().cloned().collect();
    let missing: Vec<&String> = expected.difference(&set).collect();
    let extra: Vec<&String> = set.difference(&expected).collect();
    assert!(missing.is_empty(), "docs/{page}.md: no row for {missing:?}");
    assert!(extra.is_empty(), "docs/{page}.md: rows that name no field {extra:?}");
    assert_eq!(rows.len(), set.len(), "docs/{page}.md: a field has two rows");
}

macro_rules! rows {
    ($module:ident, $format:expr, $page:literal, $commands:literal, $($test:ident => $path:literal,)+) => {
        mod $module {
            use super::*;

            const PATHS: &[&str] = &[$($path),+];

            #[test]
            fn the_table_has_one_row_per_property_of_the_device_schema_and_no_other() {
                check_rows($format, $page, $commands, PATHS);
            }

            $(
                #[test]
                fn $test() {
                    check($format, $page, $path);
                }
            )+
        }
    };
}

rows!(midi_csv_device_table, Format::MidiCsv, "midi-csv", false,
    the_row_of_schema_says_what_the_code_does => "/$schema",
    the_row_of_device_says_what_the_code_does => "/device",
    the_row_of_capabilities_says_what_the_code_does => "/capabilities",
    the_row_of_role_vocabulary_says_what_the_code_does => "/role_vocabulary",
    the_row_of_midi_of_says_what_the_code_does => "/midi_of",
    the_row_of_contexts_says_what_the_code_does => "/contexts",
    the_row_of_loads_sounds_of_says_what_the_code_does => "/loads_sounds_of",
    the_row_of_sound_transfers_says_what_the_code_does => "/sound_transfers",
    the_row_of_resources_says_what_the_code_does => "/resources",
    the_row_of_identity_says_what_the_code_does => "/identity",
    the_row_of_programs_says_what_the_code_does => "/programs",
    the_row_of_sysex_says_what_the_code_does => "/sysex",
    the_row_of_cc_params_says_what_the_code_does => "/cc_params",
    the_row_of_params_says_what_the_code_does => "/params",
    the_row_of_commands_says_what_the_code_does => "/commands",
    the_row_of_triggers_says_what_the_code_does => "/triggers",
    the_row_of_midi_out_says_what_the_code_does => "/midi_out",
    the_row_of_midi_in_says_what_the_code_does => "/midi_in",
    the_row_of_replies_says_what_the_code_does => "/replies",
    the_row_of_subscriptions_says_what_the_code_does => "/subscriptions",
    the_row_of_custom_commands_says_what_the_code_does => "/custom_commands",
    the_row_of_sources_says_what_the_code_does => "/sources",
    the_row_of_midi_of_sources_says_what_the_code_does => "/midi_of_sources",
    the_row_of_coverage_says_what_the_code_does => "/_coverage",
    the_row_of_top_level_underscore_notes_says_what_the_code_does => "/_*",
    the_row_of_device_name_says_what_the_code_does => "/device/name",
    the_row_of_device_vendor_says_what_the_code_does => "/device/vendor",
    the_row_of_device_osc_prefix_says_what_the_code_does => "/device/osc_prefix",
    the_row_of_device_device_id_says_what_the_code_does => "/device/device_id",
    the_row_of_device_rate_limit_hz_says_what_the_code_does => "/device/rate_limit_hz",
    the_row_of_device_kind_says_what_the_code_does => "/device/kind",
    the_row_of_device_stands_for_says_what_the_code_does => "/device/stands_for",
    the_row_of_device_fidelity_says_what_the_code_does => "/device/fidelity",
    the_row_of_device_transport_says_what_the_code_does => "/device/transport",
    the_row_of_device_underscore_notes_says_what_the_code_does => "/device/_*",
    the_row_of_cc_params_channel_says_what_the_code_does => "/cc_params/channel",
    the_row_of_cc_params_entries_says_what_the_code_does => "/cc_params/entries",
    the_row_of_cc_params_underscore_notes_says_what_the_code_does => "/cc_params/_*",
    the_row_of_control_osc_says_what_the_code_does => "/cc_params/entries/N/osc",
    the_row_of_control_title_says_what_the_code_does => "/cc_params/entries/N/title",
    the_row_of_control_description_says_what_the_code_does => "/cc_params/entries/N/description",
    the_row_of_control_value_description_says_what_the_code_does => "/cc_params/entries/N/value_description",
    the_row_of_control_notes_says_what_the_code_does => "/cc_params/entries/N/notes",
    the_row_of_control_scale_says_what_the_code_does => "/cc_params/entries/N/scale",
    the_row_of_control_range_says_what_the_code_does => "/cc_params/entries/N/range",
    the_row_of_control_transmit_says_what_the_code_does => "/cc_params/entries/N/transmit",
    the_row_of_control_recognize_says_what_the_code_does => "/cc_params/entries/N/recognize",
    the_row_of_control_nrpn_lsb_range_says_what_the_code_does => "/cc_params/entries/N/nrpn_lsb_range",
    the_row_of_control_default_says_what_the_code_does => "/cc_params/entries/N/default",
    the_row_of_control_priority_says_what_the_code_does => "/cc_params/entries/N/priority",
    the_row_of_control_step_count_says_what_the_code_does => "/cc_params/entries/N/step_count",
    the_row_of_control_values_says_what_the_code_does => "/cc_params/entries/N/values",
    the_row_of_control_polarity_says_what_the_code_does => "/cc_params/entries/N/polarity",
    the_row_of_control_channel_says_what_the_code_does => "/cc_params/entries/N/channel",
    the_row_of_control_section_says_what_the_code_does => "/cc_params/entries/N/section",
    the_row_of_control_expects_says_what_the_code_does => "/cc_params/entries/N/expects",
    the_row_of_control_transform_says_what_the_code_does => "/cc_params/entries/N/transform",
    the_row_of_control_role_says_what_the_code_does => "/cc_params/entries/N/role",
    the_row_of_control_ways_says_what_the_code_does => "/cc_params/entries/N/ways",
    the_row_of_control_readings_says_what_the_code_does => "/cc_params/entries/N/readings",
    the_row_of_control_underscore_notes_says_what_the_code_does => "/cc_params/entries/N/_*",
    the_row_of_way_ctrltype_says_what_the_code_does => "/cc_params/entries/N/ways/M/ctrlType",
    the_row_of_way_ctrlindex_says_what_the_code_does => "/cc_params/entries/N/ways/M/ctrlIndex",
    the_row_of_way_value_bits_says_what_the_code_does => "/cc_params/entries/N/ways/M/value_bits",
    the_row_of_way_range_says_what_the_code_does => "/cc_params/entries/N/ways/M/range",
    the_row_of_way_transmit_says_what_the_code_does => "/cc_params/entries/N/ways/M/transmit",
    the_row_of_way_recognize_says_what_the_code_does => "/cc_params/entries/N/ways/M/recognize",
    the_row_of_way_nrpn_lsb_range_says_what_the_code_does => "/cc_params/entries/N/ways/M/nrpn_lsb_range",
    the_row_of_way_in_says_what_the_code_does => "/cc_params/entries/N/ways/M/in",
    the_row_of_way_slice_says_what_the_code_does => "/cc_params/entries/N/ways/M/slice",
    the_row_of_way_underscore_notes_says_what_the_code_does => "/cc_params/entries/N/ways/M/_*",
    the_row_of_source_id_says_what_the_code_does => "/sources/N/id",
    the_row_of_source_type_says_what_the_code_does => "/sources/N/type",
    the_row_of_source_provider_says_what_the_code_does => "/sources/N/provider",
    the_row_of_source_url_says_what_the_code_does => "/sources/N/url",
    the_row_of_source_firmware_says_what_the_code_does => "/sources/N/firmware",
    the_row_of_source_author_says_what_the_code_does => "/sources/N/author",
    the_row_of_source_attribution_says_what_the_code_does => "/sources/N/attribution",
    the_row_of_source_license_says_what_the_code_does => "/sources/N/license",
    the_row_of_source_commit_says_what_the_code_does => "/sources/N/commit",
    the_row_of_source_profile_says_what_the_code_does => "/sources/N/profile",
    the_row_of_source_imported_at_says_what_the_code_does => "/sources/N/imported_at",
    the_row_of_source_notes_says_what_the_code_does => "/sources/N/notes",
    the_row_of_source_underscore_notes_says_what_the_code_does => "/sources/N/_*",
);

rows!(electra_one_device_table, Format::ElectraOne, "electra-one", true,
    the_row_of_schema_says_what_the_code_does => "/$schema",
    the_row_of_device_says_what_the_code_does => "/device",
    the_row_of_capabilities_says_what_the_code_does => "/capabilities",
    the_row_of_role_vocabulary_says_what_the_code_does => "/role_vocabulary",
    the_row_of_midi_of_says_what_the_code_does => "/midi_of",
    the_row_of_contexts_says_what_the_code_does => "/contexts",
    the_row_of_loads_sounds_of_says_what_the_code_does => "/loads_sounds_of",
    the_row_of_sound_transfers_says_what_the_code_does => "/sound_transfers",
    the_row_of_resources_says_what_the_code_does => "/resources",
    the_row_of_identity_says_what_the_code_does => "/identity",
    the_row_of_programs_says_what_the_code_does => "/programs",
    the_row_of_sysex_says_what_the_code_does => "/sysex",
    the_row_of_cc_params_says_what_the_code_does => "/cc_params",
    the_row_of_params_says_what_the_code_does => "/params",
    the_row_of_commands_says_what_the_code_does => "/commands",
    the_row_of_triggers_says_what_the_code_does => "/triggers",
    the_row_of_midi_out_says_what_the_code_does => "/midi_out",
    the_row_of_midi_in_says_what_the_code_does => "/midi_in",
    the_row_of_replies_says_what_the_code_does => "/replies",
    the_row_of_subscriptions_says_what_the_code_does => "/subscriptions",
    the_row_of_custom_commands_says_what_the_code_does => "/custom_commands",
    the_row_of_sources_says_what_the_code_does => "/sources",
    the_row_of_midi_of_sources_says_what_the_code_does => "/midi_of_sources",
    the_row_of_coverage_says_what_the_code_does => "/_coverage",
    the_row_of_top_level_underscore_notes_says_what_the_code_does => "/_*",
    the_row_of_device_name_says_what_the_code_does => "/device/name",
    the_row_of_device_vendor_says_what_the_code_does => "/device/vendor",
    the_row_of_device_osc_prefix_says_what_the_code_does => "/device/osc_prefix",
    the_row_of_device_device_id_says_what_the_code_does => "/device/device_id",
    the_row_of_device_rate_limit_hz_says_what_the_code_does => "/device/rate_limit_hz",
    the_row_of_device_kind_says_what_the_code_does => "/device/kind",
    the_row_of_device_stands_for_says_what_the_code_does => "/device/stands_for",
    the_row_of_device_fidelity_says_what_the_code_does => "/device/fidelity",
    the_row_of_device_transport_says_what_the_code_does => "/device/transport",
    the_row_of_device_underscore_notes_says_what_the_code_does => "/device/_*",
    the_row_of_cc_params_channel_says_what_the_code_does => "/cc_params/channel",
    the_row_of_cc_params_entries_says_what_the_code_does => "/cc_params/entries",
    the_row_of_cc_params_underscore_notes_says_what_the_code_does => "/cc_params/_*",
    the_row_of_control_osc_says_what_the_code_does => "/cc_params/entries/N/osc",
    the_row_of_control_title_says_what_the_code_does => "/cc_params/entries/N/title",
    the_row_of_control_description_says_what_the_code_does => "/cc_params/entries/N/description",
    the_row_of_control_value_description_says_what_the_code_does => "/cc_params/entries/N/value_description",
    the_row_of_control_notes_says_what_the_code_does => "/cc_params/entries/N/notes",
    the_row_of_control_scale_says_what_the_code_does => "/cc_params/entries/N/scale",
    the_row_of_control_range_says_what_the_code_does => "/cc_params/entries/N/range",
    the_row_of_control_transmit_says_what_the_code_does => "/cc_params/entries/N/transmit",
    the_row_of_control_recognize_says_what_the_code_does => "/cc_params/entries/N/recognize",
    the_row_of_control_nrpn_lsb_range_says_what_the_code_does => "/cc_params/entries/N/nrpn_lsb_range",
    the_row_of_control_default_says_what_the_code_does => "/cc_params/entries/N/default",
    the_row_of_control_priority_says_what_the_code_does => "/cc_params/entries/N/priority",
    the_row_of_control_step_count_says_what_the_code_does => "/cc_params/entries/N/step_count",
    the_row_of_control_values_says_what_the_code_does => "/cc_params/entries/N/values",
    the_row_of_control_polarity_says_what_the_code_does => "/cc_params/entries/N/polarity",
    the_row_of_control_channel_says_what_the_code_does => "/cc_params/entries/N/channel",
    the_row_of_control_section_says_what_the_code_does => "/cc_params/entries/N/section",
    the_row_of_control_expects_says_what_the_code_does => "/cc_params/entries/N/expects",
    the_row_of_control_transform_says_what_the_code_does => "/cc_params/entries/N/transform",
    the_row_of_control_role_says_what_the_code_does => "/cc_params/entries/N/role",
    the_row_of_control_ways_says_what_the_code_does => "/cc_params/entries/N/ways",
    the_row_of_control_readings_says_what_the_code_does => "/cc_params/entries/N/readings",
    the_row_of_control_underscore_notes_says_what_the_code_does => "/cc_params/entries/N/_*",
    the_row_of_way_ctrltype_says_what_the_code_does => "/cc_params/entries/N/ways/M/ctrlType",
    the_row_of_way_ctrlindex_says_what_the_code_does => "/cc_params/entries/N/ways/M/ctrlIndex",
    the_row_of_way_value_bits_says_what_the_code_does => "/cc_params/entries/N/ways/M/value_bits",
    the_row_of_way_range_says_what_the_code_does => "/cc_params/entries/N/ways/M/range",
    the_row_of_way_transmit_says_what_the_code_does => "/cc_params/entries/N/ways/M/transmit",
    the_row_of_way_recognize_says_what_the_code_does => "/cc_params/entries/N/ways/M/recognize",
    the_row_of_way_nrpn_lsb_range_says_what_the_code_does => "/cc_params/entries/N/ways/M/nrpn_lsb_range",
    the_row_of_way_in_says_what_the_code_does => "/cc_params/entries/N/ways/M/in",
    the_row_of_way_slice_says_what_the_code_does => "/cc_params/entries/N/ways/M/slice",
    the_row_of_way_underscore_notes_says_what_the_code_does => "/cc_params/entries/N/ways/M/_*",
    the_row_of_source_id_says_what_the_code_does => "/sources/N/id",
    the_row_of_source_type_says_what_the_code_does => "/sources/N/type",
    the_row_of_source_provider_says_what_the_code_does => "/sources/N/provider",
    the_row_of_source_url_says_what_the_code_does => "/sources/N/url",
    the_row_of_source_firmware_says_what_the_code_does => "/sources/N/firmware",
    the_row_of_source_author_says_what_the_code_does => "/sources/N/author",
    the_row_of_source_attribution_says_what_the_code_does => "/sources/N/attribution",
    the_row_of_source_license_says_what_the_code_does => "/sources/N/license",
    the_row_of_source_commit_says_what_the_code_does => "/sources/N/commit",
    the_row_of_source_profile_says_what_the_code_does => "/sources/N/profile",
    the_row_of_source_imported_at_says_what_the_code_does => "/sources/N/imported_at",
    the_row_of_source_notes_says_what_the_code_does => "/sources/N/notes",
    the_row_of_source_underscore_notes_says_what_the_code_does => "/sources/N/_*",
    the_row_of_command_osc_says_what_the_code_does => "/commands/N/osc",
    the_row_of_command_id_says_what_the_code_does => "/commands/N/id",
    the_row_of_command_title_says_what_the_code_does => "/commands/N/title",
    the_row_of_command_description_says_what_the_code_does => "/commands/N/description",
    the_row_of_command_section_says_what_the_code_does => "/commands/N/section",
    the_row_of_command_notes_says_what_the_code_does => "/commands/N/notes",
    the_row_of_command_args_says_what_the_code_does => "/commands/N/args",
    the_row_of_command_readings_says_what_the_code_does => "/commands/N/readings",
    the_row_of_command_pre_says_what_the_code_does => "/commands/N/pre",
    the_row_of_command_frame_says_what_the_code_does => "/commands/N/frame",
    the_row_of_command_forward_says_what_the_code_does => "/commands/N/forward",
    the_row_of_command_script_says_what_the_code_does => "/commands/N/script",
    the_row_of_command_underscore_notes_says_what_the_code_does => "/commands/N/_*",
    the_row_of_argument_name_says_what_the_code_does => "/commands/N/args/M/name",
    the_row_of_argument_type_says_what_the_code_does => "/commands/N/args/M/type",
    the_row_of_argument_range_says_what_the_code_does => "/commands/N/args/M/range",
    the_row_of_argument_values_says_what_the_code_does => "/commands/N/args/M/values",
    the_row_of_argument_default_says_what_the_code_does => "/commands/N/args/M/default",
    the_row_of_argument_underscore_notes_says_what_the_code_does => "/commands/N/args/M/_*",
);
