//! What a property is, and the page of a kind: `osd_reference::properties` and `osd_reference::page`.

use serde_json::{json, Value};
use std::path::PathBuf;

fn schema_0_6(kind: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../schemas/0.6")
        .join(format!("{kind}.schema.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn described(schema: &Value) -> (usize, usize) {
    let all = osd_reference::properties(schema);
    let d = all.iter().filter(|p| p.description.is_some()).count();
    (d, all.len())
}

#[test]
fn a_property_is_a_key_of_any_properties_object_at_any_depth_defs_included() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string", "description": "A." },
            "b": { "type": "object", "properties": {
                "c": { "type": "integer" },
                "d": { "type": "array", "items": { "type": "object", "properties": {
                    "e": { "type": "string", "description": "E." } } } }
            } }
        },
        "$defs": { "thing": { "type": "object", "properties": { "f": { "type": "string" } } } }
    });
    assert_eq!(described(&schema), (2, 6));
}

#[test]
fn a_ref_without_its_own_description_is_undescribed_whatever_its_target_says() {
    let schema = json!({
        "properties": {
            "own": { "$ref": "#/$defs/t", "description": "Mine." },
            "borrowed": { "$ref": "#/$defs/t" }
        },
        "$defs": { "t": { "type": "string", "description": "The target." } }
    });
    let props = osd_reference::properties(&schema);
    assert_eq!(props.len(), 2);
    let described: Vec<_> = props.iter().filter(|p| p.description.is_some()).collect();
    assert_eq!(described.len(), 1);
    assert_eq!(described[0].description.as_deref(), Some("Mine."));
}

#[test]
fn composition_keywords_are_not_followed_for_a_description() {
    let schema = json!({
        "properties": {
            "p": { "allOf": [ { "description": "Inside allOf." } ] },
            "q": { "oneOf": [ { "description": "Inside oneOf." } ] },
            "r": { "if": { "description": "x" }, "then": { "description": "y" } }
        }
    });
    assert_eq!(described(&schema), (0, 3));
}

#[test]
fn properties_inside_composition_keywords_are_counted() {
    let schema = json!({
        "allOf": [ { "properties": { "x": { "description": "X." } } } ],
        "if": { "properties": { "y": {} } },
        "then": { "properties": { "z": {} } }
    });
    assert_eq!(described(&schema), (1, 3));
}

#[test]
fn a_schema_without_properties_has_none() {
    assert!(osd_reference::properties(&json!({ "type": "string" })).is_empty());
}

#[test]
fn the_device_schema_of_0_6_has_296_properties_144_described() {
    assert_eq!(described(&schema_0_6("device")), (144, 296));
}

#[test]
fn each_kind_page_lists_as_many_properties_as_its_schema_counts() {
    for kind in ["device", "profile", "sound", "fingerprint", "firmware", "assembly"] {
        let schema = schema_0_6(kind);
        let props = osd_reference::properties(&schema);
        assert!(!props.is_empty(), "{kind}");
        let page = osd_reference::page(kind, &schema);
        for p in &props {
            assert!(page.contains(&p.pointer), "{kind}: {} missing from the page", p.pointer);
        }
    }
}

#[test]
fn the_device_page_lists_the_296_properties() {
    let schema = schema_0_6("device");
    let props = osd_reference::properties(&schema);
    assert_eq!(props.len(), 296);
    let page = osd_reference::page("device", &schema);
    assert!(props.iter().all(|p| page.contains(&p.pointer)));
}

#[test]
fn a_property_without_description_is_listed_and_marked_undescribed() {
    let schema = json!({ "properties": {
        "bare": { "type": "string" },
        "told": { "type": "string", "description": "Said." }
    } });
    let page = osd_reference::page("x", &schema);
    assert!(page.contains("bare") && page.contains("told") && page.contains("Said."));
    assert!(page.contains("undescribed"));
}

#[test]
fn a_page_of_fully_described_properties_carries_no_undescribed_mark() {
    let schema = json!({ "properties": { "told": { "type": "string", "description": "Said." } } });
    assert!(!osd_reference::page("x", &schema).contains("undescribed"));
}

#[test]
fn the_same_schema_gives_the_same_page() {
    let schema = schema_0_6("profile");
    assert_eq!(osd_reference::page("profile", &schema), osd_reference::page("profile", &schema));
}
