//! The matching of the identity of a device against a fingerprint (proposal 0004, sections 1, 4
//! and 5): each rule is matched, not matched or not applied; the comparison folds the case of both
//! texts; two product instance ids are compared exactly.

use osd::{Device, Fingerprint, Kind, MatchVerdict, RuleMatch, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

use MatchVerdict::{Matched, NotApplied, NotMatched};

fn address(version: &str, kind: &str) -> String {
    format!("https://w3id.org/osd/{version}/{kind}.schema.json")
}

fn device_text(version: &str, identity: Value) -> String {
    let mut doc = json!({"device": {"name": "Synth", "osc_prefix": "/synth"}, "identity": identity});
    // 0.4 has no `$schema`.
    if version != "0.4" {
        doc["$schema"] = json!(address(version, "device"));
    }
    doc.to_string()
}

fn read_device(text: &str) -> Device {
    match osd::read(text, Kind::Device).unwrap_or_else(|f| panic!("{text}: {f}")) {
        osd::Descriptor::Device(d) => *d,
        _ => panic!("not a device"),
    }
}

fn device(version: &str, identity: Value) -> Device {
    read_device(&device_text(version, identity))
}

fn fingerprint(extra: Value) -> Fingerprint {
    let mut doc = json!({"$schema": address("0.7", "fingerprint"), "observed": "2026-11-25",
        "device": {"vendor": "Acme", "model": "Synth", "firmware": "1.0"}});
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    match osd::read(&doc.to_string(), Kind::Fingerprint).unwrap_or_else(|f| panic!("{doc}: {f}")) {
        osd::Descriptor::Fingerprint(f) => *f,
        _ => panic!("not a fingerprint"),
    }
}

fn rule(operator: &str, value: &str) -> Value {
    json!({"operator": operator, "value": value})
}

fn verdicts(d: &Device, f: &Fingerprint) -> Vec<(String, MatchVerdict)> {
    osd::match_identity(d, f).into_iter().map(|m: RuleMatch| (m.pointer, m.verdict)).collect()
}

fn only(d: &Device, f: &Fingerprint) -> MatchVerdict {
    let got = verdicts(d, f);
    assert_eq!(got.len(), 1, "{got:?}");
    got[0].1
}

/// The verdict of one 0.7 rule against the name of an input port.
fn on_name(operator: &str, value: &str, name: &str) -> MatchVerdict {
    let d = device("0.7", json!({"ports": [rule(operator, value)]}));
    only(&d, &fingerprint(json!({"ports": {"inputs": [name]}})))
}

#[test]
fn each_rule_has_one_verdict_by_its_pointer_the_ports_in_order_then_the_endpoint_name() {
    let d = device("0.7", json!({"endpoint_name": rule("equals", "Synth"),
        "ports": [rule("equals", "no"), rule("equals", "yes"), {"operator": "equals", "value": "yes", "platform": "macos"}]}));
    let f = fingerprint(json!({"endpoint_name": "synth", "ports": {"outputs": ["yes"]}}));
    let want = [("/identity/ports/0", NotMatched), ("/identity/ports/1", Matched), ("/identity/ports/2", NotApplied), ("/identity/endpoint_name", Matched)];
    assert_eq!(verdicts(&d, &f), want.map(|(p, v)| (p.to_string(), v)));
}

#[test]
fn a_device_without_a_rule_has_no_verdict() {
    let f = fingerprint(json!({"ports": {"inputs": ["synth"]}, "endpoint_name": "synth"}));
    let bare = read_device(&json!({"$schema": address("0.7", "device"), "device": {"name": "Synth", "osc_prefix": "/synth"}}).to_string());
    assert!(verdicts(&bare, &f).is_empty());
    for identity in [json!({}), json!({"reply_text": "synth"}), json!({"ports": null, "endpoint_name": null}), json!({"ports": []})] {
        assert!(verdicts(&device("0.7", identity.clone()), &f).is_empty(), "{identity}");
    }
    assert!(verdicts(&device("0.6", json!({"ports": null})), &f).is_empty());
}

#[test]
fn the_comparison_ignores_the_case_of_both_texts_under_every_operator() {
    for operator in ["equals", "contains", "starts_with", "ends_with"] {
        assert_eq!(on_name(operator, "APC Mini MK2", "apc mini mk2"), Matched, "{operator}");
        assert_eq!(on_name(operator, "apc mini mk2", "APC MINI Mk2"), Matched, "{operator}");
    }
}

#[test]
fn the_comparison_applies_the_simple_case_folding_of_unicode_whatever_the_bytes_of_each_letter() {
    // The Kelvin sign is three bytes and folds to `k`, one byte.
    assert_eq!(on_name("starts_with", "\u{212A}or", "korg"), Matched);
    assert_eq!(on_name("ends_with", "oo\u{212A}", "book"), Matched);
    assert_eq!(on_name("contains", "k", "o\u{212A}o"), Matched);
    assert_eq!(on_name("equals", "\u{17F}ynth", "SYNTH"), Matched);
    for (value, name) in [("\u{3A3}", "\u{3C2}"), ("\u{3C2}", "\u{3C3}"), ("\u{3C3}", "\u{3A3}")] {
        assert_eq!(on_name("equals", value, name), Matched, "{value} {name}");
    }
    assert_eq!(on_name("contains", "\u{39F}\u{394}\u{39F}\u{3A3}", "x \u{3BF}\u{3B4}\u{3BF}\u{3C2} y"), Matched);
    // The capital sharp s folds to the small one.
    assert_eq!(on_name("equals", "\u{1E9E}", "\u{DF}"), Matched);
}

#[test]
fn the_comparison_neither_folds_a_letter_into_two_nor_normalises_nor_reads_a_turkish_rule() {
    assert_eq!(on_name("equals", "SS", "\u{DF}"), NotMatched);
    assert_eq!(on_name("contains", "\u{DF}", "ss"), NotMatched);
    assert_eq!(on_name("equals", "caf\u{E9}", "cafe\u{301}"), NotMatched);
    assert_eq!(on_name("equals", "cafe\u{301}", "caf\u{E9}"), NotMatched);
    assert_eq!(on_name("equals", "\u{130}", "i"), NotMatched);
    assert_eq!(on_name("equals", "I", "\u{131}"), NotMatched);
}

#[test]
fn a_value_is_a_text_and_never_a_pattern() {
    for (value, name) in [(".*", "synth"), ("s.nth", "synth"), ("synth$", "synth"), ("[s]", "s"), ("a|b", "a")] {
        assert_eq!(on_name("contains", value, name), NotMatched, "{value}");
        assert_eq!(on_name("contains", value, &format!("x {value} y")), Matched, "{value}");
    }
}

#[test]
fn a_value_longer_than_the_name_is_not_matched() {
    for operator in ["equals", "contains", "starts_with", "ends_with"] {
        assert_eq!(on_name(operator, "synth one", "synth"), NotMatched, "{operator}");
        assert_eq!(on_name(operator, "synth", ""), NotMatched, "{operator}");
    }
}

#[test]
fn a_rule_the_system_does_not_apply_is_not_applied_whatever_the_names() {
    let d = device("0.7", json!({"ports": [{"operator": "contains", "value": "synth", "platform": "linux", "stack": "jack"}]}));
    for names in [json!({"inputs": ["synth"]}), json!({"inputs": ["other"]}), json!({})] {
        for system in [json!({}), json!({"platform": "linux"}), json!({"platform": "linux", "stack": "alsa-seq"}), json!({"platform": "macos", "stack": "coremidi"})] {
            let mut extra = system.clone();
            extra["ports"] = names.clone();
            assert_eq!(only(&d, &fingerprint(extra)), NotApplied, "{system} {names}");
        }
    }
}

#[test]
fn a_rule_of_a_direction_is_not_applied_to_a_fingerprint_without_that_list() {
    for (direction, other) in [("input", "outputs"), ("output", "inputs")] {
        let d = device("0.7", json!({"ports": [{"operator": "contains", "value": "synth", "direction": direction}]}));
        assert_eq!(only(&d, &fingerprint(json!({"ports": {other: ["synth"]}}))), NotApplied, "{direction}");
        assert_eq!(only(&d, &fingerprint(json!({}))), NotApplied, "{direction}: no ports at all");
    }
}

#[test]
fn a_rule_without_direction_is_not_applied_to_a_fingerprint_with_neither_list() {
    let d = device("0.7", json!({"ports": [rule("contains", "synth")]}));
    assert_eq!(only(&d, &fingerprint(json!({}))), NotApplied);
    assert_eq!(only(&d, &fingerprint(json!({"ports": {}}))), NotApplied);
    for list in ["inputs", "outputs"] {
        assert_eq!(only(&d, &fingerprint(json!({"ports": {list: []}}))), NotMatched, "{list}");
    }
}

#[test]
fn a_plain_name_of_a_device_before_0_7_is_a_contains_rule_with_no_direction_and_no_system() {
    for version in ["0.4", "0.5", "0.6"] {
        let d = device(version, json!({"ports": ["Electra Controller"]}));
        for extra in [
            json!({"ports": {"inputs": ["midiin2 electra controller"]}}),
            json!({"platform": "windows", "stack": "winrt", "ports": {"outputs": ["electra controller port"], "inputs": []}}),
            json!({"platform": "macos", "ports": {"inputs": ["electra controller"]}}),
        ] {
            assert_eq!(verdicts(&d, &fingerprint(extra.clone())), [("/identity/ports/0".to_string(), Matched)], "{version} {extra}");
        }
        assert_eq!(only(&d, &fingerprint(json!({"ports": {"inputs": ["electra"]}}))), NotMatched, "{version}");
    }
}

#[test]
fn an_empty_name_of_a_device_before_0_7_is_never_matched_and_is_not_applied_without_names() {
    for version in ["0.4", "0.5", "0.6"] {
        let d = device(version, json!({"ports": [""]}));
        assert_eq!(only(&d, &fingerprint(json!({"ports": {"inputs": ["synth"]}}))), NotMatched, "{version}");
        assert_eq!(only(&d, &fingerprint(json!({"ports": {"outputs": [""]}}))), NotMatched, "{version}");
        assert_eq!(only(&d, &fingerprint(json!({}))), NotApplied, "{version}");
    }
}

#[test]
fn a_0_6_fingerprint_is_matched_by_a_rule_with_no_platform_and_not_applied_to_one_with() {
    let text = json!({"$schema": address("0.6", "fingerprint"), "observed": "2026-11-25", "ports": {"inputs": ["synth"]},
        "device": {"vendor": "Acme", "model": "Synth", "firmware": "1.0"}});
    let osd::Descriptor::Fingerprint(f) = osd::read(&text.to_string(), Kind::Fingerprint).unwrap() else { panic!("a fingerprint") };
    let d = device("0.7", json!({"ports": [rule("equals", "Synth"), {"operator": "equals", "value": "Synth", "platform": "linux"}],
        "endpoint_name": rule("equals", "Synth")}));
    assert_eq!(verdicts(&d, &f).into_iter().map(|(_, v)| v).collect::<Vec<_>>(), [Matched, NotApplied, NotApplied]);
}

#[test]
fn the_conversion_to_0_7_changes_no_recognition_an_empty_name_included() {
    let old = device_text("0.6", json!({"ports": ["", "Synth", "", "DIN Thru"]}));
    let new = osd::convert_to(&old, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"));
    let (old, new) = (read_device(&old), read_device(&new));
    for extra in [
        json!({"ports": {"inputs": ["synth"]}}),
        json!({"ports": {"outputs": ["din thru"]}}),
        json!({"ports": {"inputs": [""]}}),
        json!({"ports": {"inputs": ["other"]}}),
        json!({}),
    ] {
        let f = fingerprint(extra.clone());
        let matched = |d: &Device| verdicts(d, &f).iter().filter(|(_, v)| *v == Matched).count();
        assert_eq!(matched(&old), matched(&new), "{extra}");
    }
}

#[test]
fn the_names_of_a_profile_before_0_7_are_contains_rules_once_resolved() {
    for version in ["0.5", "0.6"] {
        let files: BTreeMap<String, String> = [
            ("acme/synth".to_string(), device_text(version, json!({"profile": "acme"}))),
            ("acme/_profile".to_string(), json!({"$schema": address(version, "profile"), "identity": {"ports": ["", "Synth"]}}).to_string()),
        ]
        .into();
        let resolved = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
        let f = fingerprint(json!({"platform": "linux", "ports": {"outputs": ["a synth port", ""]}}));
        assert_eq!(verdicts(&resolved.device, &f).into_iter().map(|(_, v)| v).collect::<Vec<_>>(), [NotMatched, Matched], "{version}");
    }
}

fn with_id(id: Option<&str>) -> Fingerprint {
    fingerprint(match id {
        Some(i) => json!({"product_instance_id": i}),
        None => json!({}),
    })
}

fn same(a: Option<&str>, b: Option<&str>) -> Option<bool> {
    osd::same_product_instance_id(&with_id(a), &with_id(b))
}

#[test]
fn two_product_instance_ids_are_the_same_only_when_their_texts_are_equal_exactly() {
    assert_eq!(same(Some("SN-0001"), Some("SN-0001")), Some(true));
    for other in ["SN-0002", "sn-0001", "SN-0001 ", " SN-0001", "SN-000", "SN-00011"] {
        assert_eq!(same(Some("SN-0001"), Some(other)), Some(false), "{other:?}");
        assert_eq!(same(Some(other), Some("SN-0001")), Some(false), "{other:?}");
    }
    assert_eq!(same(Some("\u{212A}"), Some("k")), Some(false), "no case folding");
    assert_eq!(same(Some("caf\u{E9}"), Some("cafe\u{301}")), Some(false), "no normalisation");
}

#[test]
fn an_absent_or_empty_product_instance_id_leaves_the_comparison_unknown() {
    for (a, b) in [(None, None), (None, Some("A")), (Some("A"), None), (Some(""), Some("")), (Some(""), Some("A")), (Some("A"), Some("")), (Some(""), None)] {
        assert_eq!(same(a, b), None, "{a:?} {b:?}");
    }
}

#[test]
fn the_comparison_of_product_instance_ids_reads_no_other_field() {
    let a = fingerprint(json!({"product_instance_id": "A", "platform": "linux", "stack": "jack", "endpoint_name": "one", "ports": {"inputs": ["one"]}}));
    let b = fingerprint(json!({"product_instance_id": "A", "platform": "macos", "endpoint_name": "two", "usb": {"vendor_id": 1, "product_id": 2},
        "device": {"vendor": "Other", "model": "Thing", "firmware": "9"}}));
    assert_eq!(osd::same_product_instance_id(&a, &b), Some(true));
    let c = fingerprint(json!({"platform": "linux", "endpoint_name": "one", "ports": {"inputs": ["one"]}}));
    assert_eq!(osd::same_product_instance_id(&c, &c), None);
}
