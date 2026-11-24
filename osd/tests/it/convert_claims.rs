//! `osd::convert` (0.5 to 0.6) never writes a text `osd::read` refuses: a 0.5 table whose two controls
//! claim the same message (same kind, same channel, same numbers; a 14-bit CC claims its CC and its LSB)
//! is refused as `NotConvertible` at the later control, naming the earlier, as `read` would refuse it
//! (`spec/0.6.md`, "A reader MUST refuse the later claim of a message").

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

fn device(table_channel: Option<u8>, entries: Value) -> String {
    let mut table = json!({ "entries": entries });
    if let Some(c) = table_channel {
        table["channel"] = json!(c);
    }
    json!({
        "$schema": "https://w3id.org/osd/0.5/device.schema.json",
        "device": { "name": "T", "osc_prefix": "/t" },
        "cc_params": table
    })
    .to_string()
}

/// The refusal convert gives, which must be the later entry's `NotConvertible` naming the earlier one.
fn refused_at(text: &str, later: usize, earlier: usize) {
    let fault = osd::convert(text, Kind::Device).expect_err("a text whose two controls claim one message is not converted");
    assert_eq!(fault.kind, FaultKind::NotConvertible, "{fault}");
    assert_eq!(fault.pointer, format!("/cc_params/entries/{later}"), "{fault}");
    assert!(fault.rule.contains(&format!("/cc_params/entries/{earlier}")), "the refusal names the earlier entry: {}", fault.rule);
}

/// A text convert returns is a text read accepts.
fn converts_and_reads(text: &str) {
    let out = osd::convert(text, Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    osd::read(&out, Kind::Device).unwrap_or_else(|f| panic!("read refuses what convert wrote: {f}"));
}

#[test]
fn two_controls_on_the_same_cc_alone_are_not_convertible() {
    refused_at(&device(Some(0), json!([{ "osc": "/a", "cc": 7 }, { "osc": "/b", "cc": 7 }])), 1, 0);
    refused_at(&device(Some(0), json!([{ "osc": "/a", "cc": 1 }, { "osc": "/b", "cc": 2 }, { "osc": "/c", "cc": 2 }])), 2, 1);
}

#[test]
fn two_identical_nrpn_pairs_are_not_convertible() {
    refused_at(
        &device(Some(0), json!([{ "osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2 }, { "osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 2 }])),
        1,
        0,
    );
}

#[test]
fn a_14_bit_cc_clashes_with_a_plain_cc_on_either_of_its_two_numbers() {
    refused_at(&device(Some(0), json!([{ "osc": "/a", "cc": 8, "cc_lsb": 40 }, { "osc": "/b", "cc": 8 }])), 1, 0);
    refused_at(&device(Some(0), json!([{ "osc": "/a", "cc": 8, "cc_lsb": 40 }, { "osc": "/b", "cc": 40 }])), 1, 0);
    refused_at(&device(Some(0), json!([{ "osc": "/a", "cc": 40 }, { "osc": "/b", "cc": 8, "cc_lsb": 40 }])), 1, 0);
}

#[test]
fn an_entry_channel_and_the_table_channel_that_are_equal_make_one_message() {
    refused_at(&device(Some(3), json!([{ "osc": "/a", "cc": 7 }, { "osc": "/b", "cc": 7, "channel": 3 }])), 1, 0);
    refused_at(&device(None, json!([{ "osc": "/a", "cc": 7, "channel": 3 }, { "osc": "/b", "cc": 7, "channel": 3 }])), 1, 0);
}

#[test]
fn messages_that_differ_by_channel_kind_or_number_are_converted_and_read() {
    // a different channel (an entry's own against the table's)
    converts_and_reads(&device(Some(0), json!([{ "osc": "/a", "cc": 7 }, { "osc": "/b", "cc": 7, "channel": 1 }])));
    converts_and_reads(&device(None, json!([{ "osc": "/a", "cc": 7, "channel": 1 }, { "osc": "/b", "cc": 7, "channel": 2 }])));
    // a different kind on the same numbers
    converts_and_reads(&device(Some(0), json!([{ "osc": "/a", "cc": 1 }, { "osc": "/b", "nrpn_msb": 0, "nrpn_lsb": 1 }])));
    // a different number
    converts_and_reads(&device(Some(0), json!([{ "osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2 }, { "osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 3 }])));
    converts_and_reads(&device(Some(0), json!([{ "osc": "/a", "cc": 8, "cc_lsb": 40 }, { "osc": "/b", "cc": 9, "cc_lsb": 41 }])));
}

#[test]
fn a_descriptor_without_overlap_keeps_converting() {
    converts_and_reads(&device(Some(0), json!([{ "osc": "/a", "cc": 1 }, { "osc": "/b", "cc": 2 }])));
}

#[test]
fn the_refusal_of_convert_is_the_one_read_makes_on_the_same_claims_written_in_0_6() {
    // The planted overlap written in 0.6 is refused by read at the same pointer with the same rule.
    let text_05 = device(Some(0), json!([{ "osc": "/a", "cc": 7 }, { "osc": "/b", "cc": 7 }]));
    let text_06 = text_05.replace("/0.5/", "/0.6/");
    let read_fault = osd::read(&text_06, Kind::Device).expect_err("read refuses the later claim");
    let convert_fault = osd::convert(&text_05, Kind::Device).expect_err("convert refuses it too");
    assert_eq!(convert_fault.pointer, read_fault.pointer);
    assert_eq!(convert_fault.rule, read_fault.rule);
}

/// Every 0.5 descriptor of the repository: the examples and the device cases of the 0.5 conformance
/// suite, as text.
fn texts_05() -> Vec<(String, String)> {
    let repo = common::repo();
    let mut out = Vec::new();
    let mut folders = vec![repo.join("examples/0.5"), repo.join("conformance/0.5")];
    while let Some(dir) = folders.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().map(Result::unwrap) {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                let name = path.strip_prefix(&repo).unwrap().display().to_string();
                let text = std::fs::read_to_string(&path).unwrap();
                if name.ends_with("device.json") || name.ends_with("/descriptor.json") {
                    out.push((name, text));
                }
            }
        }
    }
    out.sort();
    out
}

#[test]
fn read_accepts_every_text_convert_returns_on_the_0_5_descriptors_of_the_repository() {
    let texts = texts_05();
    assert!(texts.len() >= 20, "{} descriptors", texts.len());
    let mut converted = 0;
    let mut failures = Vec::new();
    for (name, text) in &texts {
        let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(text) else { continue };
        if doc.get("$schema").and_then(Value::as_str).is_none_or(|s| !s.contains("/0.5/")) {
            continue;
        }
        // Only a device holds a `cc_params` table.
        if !doc["$schema"].as_str().unwrap().contains("/device.schema") {
            continue;
        }
        let kind = Kind::Device;
        if osd::read(text, kind).is_err() {
            continue;
        }
        if let Ok(out) = osd::convert(text, kind) {
            converted += 1;
            if let Err(f) = osd::read(&out, kind) {
                failures.push(format!("{name}: {f}"));
            }
        }
    }
    assert!(converted >= 5, "{converted} descriptors converted");
    common::report("descriptors converted into a text read refuses", failures);
}
