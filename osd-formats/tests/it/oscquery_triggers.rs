//! The triggers of a device in OSCQuery: each is a method of type `I` (an event with no value),
//! what the method does not carry is named in the losses, and the import reads such a method back
//! as a command.

use crate::common::midnam::lost_at;
use crate::common::oscquery::*;
use crate::common::{device_from, resolved};
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Map, Value};

/// A device of 0.7 under `/trundler`, with these blocks.
fn device(extra: Value) -> osd::Resolved {
    let mut doc = json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "community-re", "url": "https://example.org/d", "author": "Dee", "license": "CC-BY-4.0"}],
        "device": {"name": "Trundler", "osc_prefix": "/trundler"},
    });
    for (k, v) in extra.as_object().expect("an object of blocks") {
        doc[k.as_str()] = v.clone();
    }
    resolved(device_from(&doc.to_string()))
}

/// A trigger with the fields a trigger needs.
fn trigger(osc: &str) -> Value {
    json!({"osc": osc, "sound": "Kick", "note_range": [36, 36]})
}

fn drums(entries: Vec<Value>) -> osd::Resolved {
    device(json!({"triggers": {"entries": entries}}))
}

/// The method of a trigger, whole: nothing but its address, its access and the impulse tag.
fn impulse(path: &str) -> Value {
    json!({"FULL_PATH": path, "ACCESS": 2, "TYPE": "I"})
}

fn paths(ns: &Value) -> Vec<String> {
    let mut p: Vec<String> = methods(ns).into_iter().map(|(p, _)| p).collect();
    p.sort();
    p
}

fn under_triggers(lost: &[Loss]) -> Vec<&str> {
    let mut places: Vec<&str> = lost.iter().map(|l| l.place.as_str()).filter(|p| p.starts_with("/triggers")).collect();
    places.sort_unstable();
    places
}

fn the_reason<'a>(lost: &'a [Loss], place: &str) -> &'a str {
    let found = lost_at(lost, place);
    assert_eq!(found.len(), 1, "{place}: {lost:?}");
    &found[0].reason
}

// ---- the method ---------------------------------------------------------------------------

#[test]
fn a_device_of_triggers_only_exports_one_method_per_trigger() {
    let (ns, lost) = exported(&drums(vec![trigger("/drum/kick"), trigger("/drum/snare"), trigger("/clap")]));
    assert_eq!(paths(&ns), ["/trundler/clap", "/trundler/drum/kick", "/trundler/drum/snare"], "{ns}");
    assert!(lost_at(&lost, "/triggers").is_empty(), "{lost:?}");
}

#[test]
fn the_method_of_a_trigger_is_a_writable_impulse_with_no_range_no_argument_name_no_overload_and_no_description() {
    let full = json!({"osc": "/drum/kick", "title": "Kick drum", "description": "The low drum", "sound": "Kick", "section": "Drums",
        "channel": 10, "channel_mode": "fixed", "note_range": [35, 36], "note": 36, "velocity_usage": "level", "comments": "c"});
    let (ns, _) = exported(&drums(vec![full]));
    assert_eq!(node_at(&ns, "/trundler/drum/kick"), Some(&impulse("/trundler/drum/kick")), "{ns}");
    assert_eq!(node_at(&ns, "/trundler/drum"), Some(&container("/trundler/drum", vec![("kick", impulse("/trundler/drum/kick"))])), "{ns}");
}

#[test]
fn the_method_of_a_trigger_follows_the_whole_prefix_of_the_device() {
    let d = device(json!({"device": {"name": "T", "osc_prefix": "/rack/trundler"}, "triggers": {"entries": [trigger("/a/b/c")]}}));
    let (ns, _) = exported(&d);
    assert_eq!(node_at(&ns, "/rack/trundler/a/b/c"), Some(&impulse("/rack/trundler/a/b/c")), "{ns}");
    for parent in ["/rack/trundler/a", "/rack/trundler/a/b"] {
        assert!(node_at(&ns, parent).is_some_and(|n| n["ACCESS"] == 0 && n.get("TYPE").is_none()), "{parent}: {ns}");
    }
}

// ---- the addresses taken ------------------------------------------------------------------

fn taken(lost: &[Loss], place: &str) {
    let reason = the_reason(lost, place);
    assert!(reason.contains("taken"), "{place}: {reason}");
}

#[test]
fn a_trigger_at_the_address_of_a_control_a_parameter_or_a_command_is_a_loss_and_their_methods_do_not_change() {
    let blocks = json!({
        "cc_params": {"entries": [{"osc": "/a", "ways": [{"ctrlType": "cc", "ctrlIndex": [1], "range": [0, 127]}]}]},
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
            "entries": [{"osc": "/p", "ways": [{"ctrlType": "sysex", "id": 5, "range": [0, 63]}]}]},
        "commands": [{"osc": "/c", "args": [{"name": "v", "type": "u7"}], "frame": [240, {"arg": "v"}, 247]}],
    });
    let (before, _) = exported(&device(blocks.clone()));
    let mut with = blocks;
    with["triggers"] = json!({"entries": [trigger("/a"), trigger("/p"), trigger("/c"), trigger("/free")]});
    let (ns, lost) = exported(&device(with));
    for address in ["/trundler/a", "/trundler/p", "/trundler/c"] {
        assert_eq!(node_at(&ns, address), node_at(&before, address), "{address}: {ns}");
    }
    assert_eq!(node_at(&ns, "/trundler/free"), Some(&impulse("/trundler/free")), "{ns}");
    for n in 0..3 {
        taken(&lost, &format!("/triggers/entries/{n}"));
    }
    assert!(lost_at(&lost, "/triggers/entries/3").is_empty(), "{lost:?}");
}

#[test]
fn a_trigger_whose_address_is_a_container_of_other_addresses_is_a_loss() {
    let d = device(json!({
        "commands": [{"osc": "/kick/tune", "frame": [240, 247]}],
        "triggers": {"entries": [trigger("/kick")]},
    }));
    let (ns, lost) = exported(&d);
    assert_eq!(paths(&ns), ["/trundler/kick/tune"], "{ns}");
    assert!(node_at(&ns, "/trundler/kick").is_some_and(|n| n["ACCESS"] == 0 && n.get("TYPE").is_none()), "{ns}");
    taken(&lost, "/triggers/entries/0");
}

#[test]
fn a_trigger_whose_path_has_a_method_on_it_is_a_loss() {
    let d = device(json!({
        "cc_params": {"entries": [{"osc": "/kick", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]},
        "triggers": {"entries": [trigger("/kick/accent"), trigger("/kick/accent/hard")]},
    }));
    let (ns, lost) = exported(&d);
    assert_eq!(paths(&ns), ["/trundler/kick"], "{ns}");
    assert!(node_at(&ns, "/trundler/kick").is_some_and(|n| n["TYPE"] == "i" && n.get("CONTENTS").is_none()), "{ns}");
    taken(&lost, "/triggers/entries/0");
    taken(&lost, "/triggers/entries/1");
}

#[test]
fn of_two_triggers_one_under_the_other_the_first_written_takes_the_node() {
    let (ns, lost) = exported(&drums(vec![trigger("/kick"), trigger("/kick/soft")]));
    assert_eq!(paths(&ns), ["/trundler/kick"], "{ns}");
    taken(&lost, "/triggers/entries/1");
    assert!(lost_at(&lost, "/triggers/entries/0").is_empty(), "{lost:?}");
    let (ns, lost) = exported(&drums(vec![trigger("/kick/soft"), trigger("/kick")]));
    assert_eq!(paths(&ns), ["/trundler/kick/soft"], "{ns}");
    taken(&lost, "/triggers/entries/1");
    assert!(lost_at(&lost, "/triggers/entries/0").is_empty(), "{lost:?}");
}

// ---- the losses ---------------------------------------------------------------------------

#[test]
fn each_field_of_a_trigger_the_method_does_not_carry_is_a_loss_at_its_own_place() {
    let full = json!({"osc": "/drum/kick", "title": "Kick drum", "description": "The low drum", "sound": "Kick", "section": "Drums",
        "channel": 10, "channel_mode": "fixed", "note_range": [36, 36], "note": 36, "velocity_usage": "level", "comments": "c",
        "_zz": "n", "_a/b~c": "n"});
    let d = device(json!({"triggers": {"entries": [full], "_block": "n", "_x/y": "n"}}));
    let (_, lost) = exported(&d);
    let fields = ["_a~1b~0c", "_zz", "channel", "channel_mode", "comments", "description", "note", "note_range", "section", "sound", "title", "velocity_usage"];
    let mut want: Vec<String> = fields.iter().map(|f| format!("/triggers/entries/0/{f}")).collect();
    want.extend(["/triggers/_block".to_string(), "/triggers/_x~1y".to_string()]);
    want.sort();
    assert_eq!(under_triggers(&lost), want, "{lost:?}");
    assert!(lost.iter().all(|l| !l.reason.is_empty()), "{lost:?}");
}

#[test]
fn a_field_a_trigger_does_not_have_is_no_loss_and_each_trigger_is_named_at_its_own_index() {
    let (_, lost) = exported(&drums(vec![trigger("/kick"), json!({"osc": "/snare", "sound": "Snare", "note_range": [38, 38], "note": 38})]));
    assert_eq!(
        under_triggers(&lost),
        [
            "/triggers/entries/0/note_range",
            "/triggers/entries/0/sound",
            "/triggers/entries/1/note",
            "/triggers/entries/1/note_range",
            "/triggers/entries/1/sound",
        ],
        "{lost:?}"
    );
}

#[test]
fn a_range_of_several_notes_with_no_note_says_the_method_does_not_say_which_note_to_play() {
    let entry = |osc: &str, range: [u8; 2], note: Option<u8>| {
        let mut o: Map<String, Value> = trigger(osc).as_object().unwrap().clone();
        o.insert("note_range".into(), json!(range));
        if let Some(n) = note {
            o.insert("note".into(), json!(n));
        }
        Value::Object(o)
    };
    let (_, lost) = exported(&drums(vec![
        entry("/tom", [41, 43], None),
        entry("/kick", [35, 36], Some(36)),
        entry("/snare", [38, 38], None),
        entry("/hat", [42, 43], None),
    ]));
    for n in [0, 3] {
        let reason = the_reason(&lost, &format!("/triggers/entries/{n}/note_range"));
        assert!(reason.contains("which note"), "{n}: {reason}");
    }
    for n in [1, 2] {
        let reason = the_reason(&lost, &format!("/triggers/entries/{n}/note_range"));
        assert!(!reason.contains("which note"), "{n}: {reason}");
    }
}

#[test]
fn the_loss_of_the_sound_says_the_import_reads_the_method_back_as_a_command_not_a_trigger() {
    let (_, lost) = exported(&drums(vec![trigger("/kick")]));
    let reason = the_reason(&lost, "/triggers/entries/0/sound");
    for word in ["import", "command", "trigger"] {
        assert!(reason.contains(word), "{word}: {reason}");
    }
}

// ---- the way back -------------------------------------------------------------------------

const NODE: &str = "/CONTENTS/synth/CONTENTS/x";

fn one(node: Value) -> Value {
    namespace(vec![("x", node)])
}

#[test]
fn a_writable_method_of_type_impulse_imports_as_a_command_with_no_argument_and_no_frame() {
    for access in [2, 3] {
        let (d, lost) = imported(&one(json!({"FULL_PATH": "/synth/x", "ACCESS": access, "TYPE": "I"})));
        assert_eq!(d["commands"], json!([{"osc": "/x"}]), "ACCESS {access}: {d}");
        let reason = the_reason(&lost, &format!("{NODE}/TYPE"));
        assert!(reason.contains("TYPE") && reason.to_lowercase().contains("impulse"), "ACCESS {access}: {reason}");
    }
}

#[test]
fn the_command_read_from_an_impulse_method_is_exported_without_a_type() {
    let (d, _) = import_of(json!({"FULL_PATH": "/synth/x", "ACCESS": 2, "TYPE": "I"}));
    let (ns, _) = exported(&resolved(d));
    assert_eq!(node_at(&ns, "/synth/x"), Some(&json!({"FULL_PATH": "/synth/x", "ACCESS": 2})), "{ns}");
}

fn import_of(node: Value) -> (osd::Device, Vec<Loss>) {
    let t = import(&one(node).to_string(), Format::OscQuery, &source()).expect("imports");
    (t.value, t.lost)
}

#[test]
fn each_range_entry_with_a_key_and_each_argument_name_of_an_impulse_method_is_named_at_its_place() {
    for entry in [json!({"MIN": 0, "MAX": 1}), json!({"MIN": 0}), json!({"VALS": [1]})] {
        let (d, lost) = imported(&one(json!({"FULL_PATH": "/synth/x", "ACCESS": 2, "TYPE": "I", "RANGE": [entry], "EXTENDED_TYPE": ["hit"]})));
        assert_eq!(d["commands"], json!([{"osc": "/x"}]), "{entry}: {d}");
        the_reason(&lost, &format!("{NODE}/RANGE/0"));
        the_reason(&lost, &format!("{NODE}/EXTENDED_TYPE/0"));
        the_reason(&lost, &format!("{NODE}/TYPE"));
    }
}

#[test]
fn a_range_entry_of_an_impulse_method_that_holds_nothing_is_no_loss() {
    for entry in [json!(null), json!({})] {
        let (d, lost) = imported(&one(json!({"FULL_PATH": "/synth/x", "ACCESS": 2, "TYPE": "I", "RANGE": [entry]})));
        assert_eq!(d["commands"], json!([{"osc": "/x"}]), "{entry}: {d}");
        assert!(!lost.iter().any(|l| l.place.starts_with(&format!("{NODE}/RANGE"))), "{entry}: {lost:?}");
    }
}

#[test]
fn a_nil_tag_and_an_impulse_beside_other_tags_still_import_no_command() {
    for tags in ["N", "II", "iI", "Ii", "IN", "[I]"] {
        let (d, lost) = imported(&one(json!({"FULL_PATH": "/synth/x", "ACCESS": 2, "TYPE": tags})));
        assert!(d.get("commands").is_none(), "{tags}: {d}");
        assert!(lost.iter().any(|l| l.place == format!("{NODE}/TYPE") && l.reason.contains("not imported")), "{tags}: {lost:?}");
    }
}

#[test]
fn a_read_only_impulse_method_is_no_command() {
    let (d, lost) = imported(&one(json!({"FULL_PATH": "/synth/x", "ACCESS": 1, "TYPE": "I"})));
    assert!(d.get("commands").is_none(), "{d}");
    the_reason(&lost, NODE);
}

#[test]
fn a_device_of_triggers_only_comes_back_as_one_command_per_trigger_and_no_triggers_block() {
    let d = drums(vec![trigger("/drum/kick"), trigger("/drum/snare"), trigger("/clap")]);
    let out = export(&d, Format::OscQuery).expect("exports");
    let t = import(&out.value, Format::OscQuery, &source()).expect("imports");
    let back = crate::common::electra::json_of(&t.value);
    assert!(back.get("triggers").is_none(), "{back}");
    let mut commands = back["commands"].as_array().unwrap_or_else(|| panic!("no commands: {back}")).clone();
    commands.sort_by_key(|c| c["osc"].to_string());
    assert_eq!(commands, [json!({"osc": "/clap"}), json!({"osc": "/drum/kick"}), json!({"osc": "/drum/snare"})], "{back}");
    for node in ["/CONTENTS/trundler/CONTENTS/clap", "/CONTENTS/trundler/CONTENTS/drum/CONTENTS/kick", "/CONTENTS/trundler/CONTENTS/drum/CONTENTS/snare"] {
        the_reason(&t.lost, &format!("{node}/TYPE"));
    }
}
