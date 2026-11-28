//! The RANGE and the EXTENDED_TYPE of a method follow its TYPE (OSCQueryProposal, "Other notes
//! on attributes"): their arrays nest as the brackets of the type do, or hold one entry per tag;
//! an object or a text written alone holds for each argument. What the export, which writes one
//! entry per tag, does not give back is named in the losses.

use crate::common::fields::held;
use crate::common::midnam::lost_at;
use crate::common::oscquery::*;
use crate::common::round_trip::{count, Figures};
use osd_formats::{import, Fault, FaultKind, Format, Loss};
use serde_json::{json, Value};

const NODE: &str = "/CONTENTS/synth/CONTENTS/x";

/// A namespace of one method at `/synth/x`, of this type and these attributes.
fn one(ty: &str, attributes: Value) -> Value {
    let mut node = json!({"FULL_PATH": "/synth/x", "ACCESS": 2, "TYPE": ty});
    for (k, v) in attributes.as_object().unwrap() {
        node[k.as_str()] = v.clone();
    }
    namespace(vec![("x", node)])
}

/// The arguments of the one command imported, as (name, range).
fn args(d: &Value) -> Vec<(String, Value)> {
    d["commands"][0]["args"]
        .as_array()
        .map(|a| a.iter().map(|a| (a["name"].as_str().unwrap().to_string(), a["range"].clone())).collect())
        .unwrap_or_default()
}

fn expected(items: &[(&str, Value)]) -> Vec<(String, Value)> {
    items.iter().map(|(n, r)| (n.to_string(), r.clone())).collect()
}

fn refused(ns: &Value) -> Fault {
    import(&ns.to_string(), Format::OscQuery, &source()).expect_err("the namespace is refused")
}

/// The losses at or under a pointer of the node.
fn under_node<'a>(lost: &'a [Loss], tail: &str) -> Vec<&'a Loss> {
    let at = format!("{NODE}/{tail}");
    lost.iter().filter(|l| l.place == at || l.place.starts_with(&format!("{at}/"))).collect()
}

fn range(min: i64, max: i64) -> Value {
    json!({"MIN": min, "MAX": max})
}

// ---- nested arrays --------------------------------------------------------------------------

fn nested() -> Value {
    one("i[ii]i", json!({
        "RANGE": [range(0, 1), [range(2, 3), range(4, 5)], range(6, 7)],
        "EXTENDED_TYPE": ["a", ["b", "c"], "d"],
    }))
}

#[test]
fn a_range_and_an_extended_type_nested_as_the_brackets_of_the_type_give_each_argument_the_bound_and_name_of_its_place() {
    let (d, _) = imported(&nested());
    assert_eq!(args(&d), expected(&[("a", json!([0, 1])), ("b", json!([2, 3])), ("c", json!([4, 5])), ("d", json!([6, 7]))]));
}

#[test]
fn arrays_nested_several_deep_and_side_by_side_give_each_argument_its_place() {
    let ns = one("[i[fi]][ii]", json!({
        "RANGE": [[range(0, 1), [null, range(4, 5)]], [range(6, 7), null]],
        "EXTENDED_TYPE": [["a", ["b", "c"]], ["d", "e"]],
    }));
    let (d, _) = imported(&ns);
    assert_eq!(args(&d), expected(&[("a", json!([0, 1])), ("b", Value::Null), ("c", json!([4, 5])), ("d", json!([6, 7])), ("e", Value::Null)]));
}

#[test]
fn a_range_and_an_extended_type_with_one_entry_per_tag_are_read_for_a_type_with_arrays() {
    let ns = one("i[ii]i", json!({
        "RANGE": [range(0, 1), range(2, 3), range(4, 5), range(6, 7)],
        "EXTENDED_TYPE": ["a", "b", "c", "d"],
    }));
    let (d, lost) = imported(&ns);
    assert_eq!(args(&d), expected(&[("a", json!([0, 1])), ("b", json!([2, 3])), ("c", json!([4, 5])), ("d", json!([6, 7]))]));
    assert!(under_node(&lost, "RANGE").is_empty() && under_node(&lost, "EXTENDED_TYPE").is_empty(), "{lost:?}");
}

#[test]
fn a_nested_array_that_does_not_follow_the_brackets_is_refused_at_its_pointer() {
    let ok = range(0, 1);
    let cases: Vec<(&str, &str, Value, &str)> = vec![
        ("a RANGE group that is too short", "RANGE", json!([ok, [ok], ok]), "RANGE/1"),
        ("a RANGE group that is too long", "RANGE", json!([ok, [ok, ok, ok], ok]), "RANGE/1"),
        ("a RANGE array where the type has one tag", "RANGE", json!([[ok], [ok, ok], ok]), "RANGE/0"),
        ("a RANGE array deeper than the type", "RANGE", json!([ok, [[ok], ok], ok]), "RANGE/1/0"),
        ("an EXTENDED_TYPE group of another length", "EXTENDED_TYPE", json!(["a", ["b"], "d"]), "EXTENDED_TYPE/1"),
        ("an EXTENDED_TYPE array where the type has one tag", "EXTENDED_TYPE", json!([["a"], ["b", "c"], "d"]), "EXTENDED_TYPE/0"),
    ];
    for (what, attribute, value, place) in cases {
        let mut attributes = json!({});
        attributes[attribute] = value;
        let f = refused(&one("i[ii]i", attributes));
        assert_eq!((f.kind, f.place.clone()), (FaultKind::InvalidFile, format!("{NODE}/{place}")), "{what}: {f}");
        assert!(!f.rule.is_empty(), "{what}");
    }
}

#[test]
fn the_rules_of_a_range_entry_hold_in_a_nested_array_at_its_pointer() {
    let ok = range(0, 1);
    let cases = [
        ("an entry that is no object", json!(7), "RANGE/1/1"),
        ("a MIN that is no number", json!({"MIN": "low", "MAX": 1}), "RANGE/1/1/MIN"),
        ("a MAX that is no number", json!({"MIN": 0, "MAX": true}), "RANGE/1/1/MAX"),
        ("a MIN above its MAX", range(5, 1), "RANGE/1/1"),
    ];
    for (what, entry, place) in cases {
        let ns = one("i[ii]i", json!({"RANGE": [ok, [ok, entry], ok]}));
        // A nested entry that keeps the rules is read.
        imported(&one("i[ii]i", json!({"RANGE": [ok, [ok, null], ok]})));
        let f = refused(&ns);
        assert_eq!((f.kind, f.place.clone()), (FaultKind::InvalidFile, format!("{NODE}/{place}")), "{what}: {f}");
    }
}

#[test]
fn the_losses_of_a_nested_entry_are_at_its_pointer_in_the_file() {
    let ns = one("i[ii]", json!({
        "RANGE": [null, [{"MIN": 0, "MAX": 3, "VALS": [0, 3]}, {"MIN": 0.5, "MAX": 3}]],
        "EXTENDED_TYPE": ["a", ["not a name", "c"]],
    }));
    let (d, lost) = imported(&ns);
    assert_eq!(args(&d), expected(&[("a", Value::Null), ("arg2", json!([0, 3])), ("c", Value::Null)]));
    assert_eq!(lost_at(&lost, &format!("{NODE}/RANGE/1/0/VALS")).len(), 1, "{lost:?}");
    assert!(!lost_at(&lost, &format!("{NODE}/RANGE/1/1")).is_empty(), "the bounds that are not whole: {lost:?}");
    assert!(!lost_at(&lost, &format!("{NODE}/EXTENDED_TYPE/1/0")).is_empty(), "the text that is no name: {lost:?}");
}

#[test]
fn the_capture_with_a_nested_range_and_three_containers_at_its_root_is_refused_for_its_root_alone() {
    let node = json!({"FULL_PATH": "/tuples/first", "ACCESS": 2, "TYPE": "i[ii]i", "RANGE": [null, [range(0, 1), range(0, 1)], null]});
    let ns = container("/", vec![
        ("tuples", container("/tuples", vec![("first", node)])),
        ("b", container("/b", vec![])),
        ("c", container("/c", vec![])),
    ]);
    let f = refused(&ns);
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/CONTENTS"), "{f}");
}

#[test]
fn each_bound_and_name_the_flat_export_writes_at_another_place_is_a_loss_and_those_at_the_same_place_are_not() {
    // Equal bounds: the place alone differs, which no comparison of values tells.
    for bounds in [[range(0, 127), range(0, 127), range(0, 127), range(0, 127)], [range(0, 1), range(2, 3), range(4, 5), range(6, 7)]] {
        let [a, b, c, d] = bounds;
        let ns = one("i[ii]i", json!({"RANGE": [a, [b, c], d], "EXTENDED_TYPE": ["a", ["b", "c"], "d"]}));
        let (_, lost) = imported(&ns);
        for place in ["RANGE/1/0", "RANGE/1/1", "RANGE/2", "EXTENDED_TYPE/1/0", "EXTENDED_TYPE/1/1", "EXTENDED_TYPE/2"] {
            let pointers: Vec<String> = if place.starts_with("RANGE") {
                ["MIN", "MAX"].iter().map(|k| format!("{NODE}/{place}/{k}")).collect()
            } else {
                vec![format!("{NODE}/{place}")]
            };
            for pointer in pointers {
                assert!(held(&lost, &pointer), "{pointer} is written elsewhere: {lost:?}");
            }
        }
        for pointer in ["RANGE/0/MIN", "RANGE/0/MAX", "EXTENDED_TYPE/0"] {
            assert!(!held(&lost, &format!("{NODE}/{pointer}")), "{pointer} comes back at its place: {lost:?}");
        }
    }
}

#[test]
fn a_namespace_with_nested_arrays_loses_no_value_without_a_name() {
    let got = count(Format::OscQuery, &nested().to_string());
    assert!(got.unnamed.is_empty(), "lost and named by no loss:\n{}", got.unnamed.join("\n"));
    for (kind, figures) in [("/RANGE/N/MIN", Figures::of(1, 1, 0)), ("/RANGE/N/N/MIN", Figures::of(0, 2, 0)), ("/EXTENDED_TYPE/N", Figures::of(1, 1, 0)), ("/EXTENDED_TYPE/N/N", Figures::of(0, 2, 0))] {
        assert_eq!(got.kinds.get(kind), Some(&figures), "{kind}:\n{}", got.markdown());
    }
    let deep = one("[ii]i", json!({"RANGE": [[range(0, 1), range(2, 3)], range(4, 5)], "EXTENDED_TYPE": [["a", "b"], "c"]}));
    let got = count(Format::OscQuery, &deep.to_string());
    assert!(got.unnamed.is_empty(), "lost and named by no loss:\n{}", got.unnamed.join("\n"));
}

#[test]
fn a_namespace_with_nulls_in_its_nested_arrays_loses_no_value_without_a_name() {
    let r = || range(0, 1);
    let cases = [
        ("a null at the same place and one the flat export writes elsewhere", "i[ii]i", json!({"RANGE": [null, [r(), r()], null]})),
        ("nulls inside the group", "i[ii]i", json!({"RANGE": [r(), [null, null], r()]})),
        ("nulls alone, a RANGE the export does not write", "i[ii]i", json!({"RANGE": [null, [null, null], null]})),
        ("nulls several deep", "[i[fi]][ii]", json!({"RANGE": [[r(), [null, r()]], [r(), null]]})),
        ("names that are null", "i[ii]i", json!({"EXTENDED_TYPE": [null, ["b", null], null]})),
        ("names that are all null", "i[ii]i", json!({"EXTENDED_TYPE": [null, [null, null], null]})),
        ("bounds and names that are null", "i[ii]i", json!({"RANGE": [null, [r(), r()], null], "EXTENDED_TYPE": [null, ["b", "c"], null]})),
    ];
    let mut unnamed = Vec::new();
    for (what, ty, attributes) in cases {
        let got = count(Format::OscQuery, &one(ty, attributes).to_string());
        unnamed.extend(got.unnamed.iter().map(|u| format!("{what}: {u}")));
    }
    assert!(unnamed.is_empty(), "lost and named by no loss:\n{}", unnamed.join("\n"));
}

#[test]
fn a_null_of_a_nested_range_that_comes_back_at_its_place_counts_carried_and_one_written_elsewhere_lost() {
    let ns = one("i[ii]i", json!({"RANGE": [null, [range(0, 1), range(2, 3)], null]}));
    let (d, lost) = imported(&ns);
    assert_eq!(args(&d), expected(&[("arg1", Value::Null), ("arg2", json!([0, 1])), ("arg3", json!([2, 3])), ("arg4", Value::Null)]));
    assert!(!held(&lost, &format!("{NODE}/RANGE/0")), "the null comes back at its place: {lost:?}");
    let got = count(Format::OscQuery, &ns.to_string());
    assert!(got.unnamed.is_empty(), "lost and named by no loss:\n{}", got.unnamed.join("\n"));
    assert_eq!(got.kinds.get("/RANGE/N"), Some(&Figures::of(1, 1, 0)), "{}", got.markdown());
}

// ---- a value written alone ------------------------------------------------------------------

#[test]
fn an_extended_type_written_as_one_text_names_the_argument_and_is_no_loss() {
    let ns = one("f", json!({"EXTENDED_TYPE": "position"}));
    let (d, lost) = imported(&ns);
    assert_eq!(args(&d), expected(&[("position", Value::Null)]));
    assert!(under_node(&lost, "EXTENDED_TYPE").is_empty(), "{lost:?}");
    let got = count(Format::OscQuery, &ns.to_string());
    assert!(got.unnamed.is_empty(), "{:?}", got.unnamed);
    assert_eq!(got.kinds.get("/EXTENDED_TYPE"), Some(&Figures::of(1, 0, 0)), "{}", got.markdown());
}

#[test]
fn an_extended_type_text_that_a_second_argument_cannot_take_is_a_loss_at_its_pointer() {
    let ns = one("ff", json!({"EXTENDED_TYPE": "position"}));
    let (d, lost) = imported(&ns);
    assert_eq!(args(&d), expected(&[("position", Value::Null), ("arg2", Value::Null)]));
    assert!(!lost_at(&lost, &format!("{NODE}/EXTENDED_TYPE")).is_empty(), "{lost:?}");
    let got = count(Format::OscQuery, &ns.to_string());
    assert!(got.unnamed.is_empty(), "{:?}", got.unnamed);
    assert_eq!(got.kinds.get("/EXTENDED_TYPE"), Some(&Figures::of(0, 1, 0)), "{}", got.markdown());
}

#[test]
fn an_extended_type_text_that_is_no_name_is_a_loss_at_its_pointer() {
    let (d, lost) = imported(&one("f", json!({"EXTENDED_TYPE": "not a name"})));
    assert_eq!(args(&d), expected(&[("arg1", Value::Null)]));
    assert!(!lost_at(&lost, &format!("{NODE}/EXTENDED_TYPE")).is_empty(), "{lost:?}");
}

#[test]
fn an_extended_type_text_beside_an_empty_type_is_read_and_is_a_loss_at_its_pointer() {
    let ns = one("", json!({"EXTENDED_TYPE": "list"}));
    let (d, lost) = imported(&ns);
    assert!(args(&d).is_empty(), "{d}");
    assert_eq!(d["commands"][0]["osc"], "/x", "{d}");
    assert!(!lost_at(&lost, &format!("{NODE}/EXTENDED_TYPE")).is_empty(), "{lost:?}");
    let got = count(Format::OscQuery, &ns.to_string());
    assert!(got.unnamed.is_empty(), "{:?}", got.unnamed);
    assert_eq!(got.kinds.get("/EXTENDED_TYPE"), Some(&Figures::of(0, 1, 0)), "{}", got.markdown());
}

#[test]
fn a_range_written_as_one_object_holds_for_each_argument_and_is_no_loss() {
    for ty in ["i", "ff", "i[ii]"] {
        let ns = one(ty, json!({"RANGE": range(0, 1)}));
        let (d, lost) = imported(&ns);
        let ranges: Vec<Value> = args(&d).into_iter().map(|a| a.1).collect();
        assert_eq!(ranges, vec![json!([0, 1]); ty.chars().filter(|c| c.is_alphabetic()).count()], "{ty}: {d}");
        assert!(under_node(&lost, "RANGE").is_empty(), "{ty}: {lost:?}");
        let got = count(Format::OscQuery, &ns.to_string());
        assert!(got.unnamed.is_empty(), "{ty}: {:?}", got.unnamed);
        for kind in ["/RANGE/MIN", "/RANGE/MAX"] {
            assert_eq!(got.kinds.get(kind), Some(&Figures::of(1, 0, 0)), "{ty} {kind}: {}", got.markdown());
        }
    }
}

#[test]
fn the_vals_of_a_range_written_alone_is_a_loss_at_its_pointer_that_covers_neither_bound() {
    let ns = one("ii", json!({"RANGE": {"MIN": 0, "MAX": 3, "VALS": [0, 3]}}));
    let (d, lost) = imported(&ns);
    assert_eq!(args(&d), expected(&[("arg1", json!([0, 3])), ("arg2", json!([0, 3]))]));
    assert_eq!(lost_at(&lost, &format!("{NODE}/RANGE/VALS")).len(), 1, "{lost:?}");
    for pointer in ["RANGE/MIN", "RANGE/MAX"] {
        assert!(!held(&lost, &format!("{NODE}/{pointer}")), "{pointer}: {lost:?}");
    }
    let got = count(Format::OscQuery, &ns.to_string());
    assert!(got.unnamed.is_empty(), "{:?}", got.unnamed);
    assert_eq!(got.kinds.get("/RANGE/MIN"), Some(&Figures::of(1, 0, 0)), "{}", got.markdown());
}

#[test]
fn a_range_written_alone_that_no_argument_takes_is_a_loss_at_its_pointer() {
    for (what, ty, value) in [
        ("an empty type", "", range(0, 1)),
        ("bounds that are not whole", "ff", json!({"MIN": 0.5, "MAX": 1})),
        ("a lone bound", "ii", json!({"MIN": 0})),
    ] {
        let ns = one(ty, json!({"RANGE": value}));
        let (d, lost) = imported(&ns);
        assert!(args(&d).iter().all(|a| a.1.is_null()), "{what}: {d}");
        assert!(!lost_at(&lost, &format!("{NODE}/RANGE")).is_empty(), "{what}: {lost:?}");
        let got = count(Format::OscQuery, &ns.to_string());
        assert!(got.unnamed.is_empty(), "{what}: {:?}", got.unnamed);
        assert_eq!(got.kinds.get("/RANGE/MIN").map(|f| f.carried), Some(0), "{what}: {}", got.markdown());
    }
}

#[test]
fn a_value_written_alone_of_another_type_or_breaking_the_rules_of_an_entry_is_refused_at_its_pointer() {
    let cases = [
        ("RANGE", json!(3), "RANGE"),
        ("RANGE", json!("0..1"), "RANGE"),
        ("RANGE", range(5, 1), "RANGE"),
        ("RANGE", json!({"MIN": "low", "MAX": 1}), "RANGE/MIN"),
        ("EXTENDED_TYPE", json!(5), "EXTENDED_TYPE"),
        ("EXTENDED_TYPE", json!(true), "EXTENDED_TYPE"),
        ("EXTENDED_TYPE", json!({"name": "a"}), "EXTENDED_TYPE"),
    ];
    // The accepted forms beside them.
    imported(&one("ff", json!({"RANGE": range(0, 1), "EXTENDED_TYPE": "position"})));
    for (attribute, value, place) in cases {
        let mut attributes = json!({});
        attributes[attribute] = value.clone();
        let f = refused(&one("ff", attributes));
        assert_eq!((f.kind, f.place.clone()), (FaultKind::InvalidFile, format!("{NODE}/{place}")), "{attribute} {value}: {f}");
    }
}
