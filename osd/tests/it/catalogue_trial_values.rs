//! What the trial of 0.7 compares outside the entries of `cc_params` and
//! `params` and the sources: every value of a driver and of a maker profile, at
//! every depth, as `spec/0.7.md` says it is written (5.3.1, 5.3.2, 5.3.5, 5.3.7,
//! 5.4). Each case is the converted text of a small catalogue, left as it is or
//! altered, given to `judge`.

use crate::catalogue_trial::run;
use crate::catalogue_trial_reference::{
    added, alter, assert_each_named, differences, judge_text, removed, set, texts, Alter,
};
use osd::Kind;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const D05: &str = "https://w3id.org/osd/0.5/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";
const P05: &str = "https://w3id.org/osd/0.5/profile.schema.json";
const P06: &str = "https://w3id.org/osd/0.6/profile.schema.json";

/// The one field of Lua of this driver is the `transform_reverse` of its entries.
fn whole_driver() -> Value {
    json!({
        "$schema": D06,
        "_note": "on the device file",
        "_coverage": { "controls": 2, "deep": { "list": [1, 2] } },
        "device": {
            "name": "Whole", "vendor": "Acme", "revision": "2", "osc_prefix": "/whole",
            "manufacturer_id": [0, 32, 51], "device_id": [16], "rate_limit_hz": 50,
            "transport": { "kind": "osc", "host": "localhost", "port": 9000, "reply_port": 9001, "rate_limit_hz": 100, "passthrough_prefix": "/raw" },
            "_about": "a note"
        },
        "capabilities": ["lua"],
        "loads_sounds_of": ["other"],
        "sysex": { "header": [240, 67], "footer": [247], "_why": "kept" },
        "midi_out": { "default_channel": 1, "note_offset": 0 },
        "midi_in": ["note_on", "cc"],
        "identity": {
            "request": [240, 126, 127, 6, 1, 247],
            "reply": [240, 126, "any", 6, 2],
            "family": [1, 0], "member": [2, 0],
            "ports": ["Whole", "", "Whole MIDI"],
            "usb": [{ "vendor_id": 2372, "product_id": 5 }],
            "sends": { "clock": true },
            "_seen": "once"
        },
        "programs": {
            "program_change": true,
            "banks": [{ "name": "A", "msb": 0, "lsb": 1, "size": 100, "display": "A{n}", "writable": false }],
            "before": [[176, 0, 1]],
            "delay_ms": 20,
            "dump": { "kind": "request", "request": [240, 67, 16, 247], "reply_timeout_ms": 500 },
            "notes": "slow"
        },
        "cc_params": { "channel": 3, "_table": "note", "entries": [
            { "osc": "/volume", "cc": 7, "channel": 2, "default": 3, "step_count": 4, "values": { "Low": 0, "High": 127 }, "transform_reverse": "return value" }
        ] },
        "params": {
            "get_frame": [240, 66, { "arg": "pr" }, { "arg": "p" }, 247],
            "set_frame": [240, 67, { "arg": "pr" }, { "arg": "p" }, { "arg": "value" }, 247],
            "value_type": "u7",
            "_table": "note",
            "entries": [{ "osc": "/shape", "pr": 16, "p": 2 }]
        },
        "commands": [
            {
                "osc": "/ping", "section": "util", "notes": "no reply",
                "args": [{ "name": "level", "type": "enum", "range": [0, 3], "values": { "a/b~c": 1, "Off": 0 }, "default": 1, "_arg": "note" }],
                "frame": [240, 66, { "arg": "level" }, 247],
                "_name": "Ping", "_description": "the ping", "_usage": "any time", "_notes": "kept",
                "_electra": { "parameter": { "type": "sysex", "number": 12 } }
            },
            { "osc": "/virtual", "frame": [240, 247], "_electra": { "parameter": { "type": "virtual", "number": 3 } } },
            { "osc": "/text", "frame": [240, 247], "_electra": { "parameter": { "type": "sysex", "number": "4" } } },
            { "osc": "/bare", "forward": { "path": "/p", "args": [5, true, "text"] }, "_electra": { "other": 1 } }
        ],
        "triggers": { "_table": "note", "entries": [
            { "osc": "/kick", "sound": "kick", "section": "drums", "channel": 9, "note_range": [36, 37], "velocity_usage": "level", "comments": "low",
              "_name": "Kick", "_description": "the kick", "_usage": "hit", "_notes": "kept" },
            { "osc": "/snare", "sound": "snare", "note_range": [38, 38], "note": 38, "_name": "", "_description": 4 }
        ] }
    })
}

fn whole_profile() -> Value {
    json!({
        "$schema": P06,
        "_about": "the maker",
        "identity": {
            "reply_text": "acme",
            "family": [1, 0],
            "ports": ["", "Acme", "Acme MIDI"],
            "usb": [{ "vendor_id": 2372, "product_id": 5 }],
            "sends": { "clock": true }
        },
        "programs": {
            "program_change": true,
            "banks": [{ "name": "A", "size": 10 }],
            "delay_ms": 5
        }
    })
}

/// A driver with a `midi_of`, whose `device` gives a maker's id, with or without
/// its own `identity` block.
fn linked(identity: bool) -> Value {
    let mut doc = json!({
        "$schema": D06,
        "device": { "name": "Base", "osc_prefix": "/base", "manufacturer_id": [67] },
        "midi_of": "acme/base"
    });
    if identity {
        doc["identity"] = json!({ "family": [2, 0], "ports": null });
    }
    doc
}

/// A driver whose maker's id has no `identity` block to join, and whose Lua is a
/// `transform_reverse` of `params`.
fn lone_id_driver() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "Lone", "osc_prefix": "/lone", "manufacturer_id": [66] },
        "capabilities": ["lua"],
        "params": {
            "get_frame": [240, { "arg": "p" }, 247],
            "set_frame": [240, { "arg": "p" }, { "arg": "value" }, 247],
            "value_type": "u7",
            "entries": [{ "osc": "/b", "id": 3, "transform_reverse": "return value" }]
        }
    })
}

fn base_driver() -> Value {
    json!({ "$schema": D06, "device": { "name": "Base", "osc_prefix": "/base" }, "cc_params": { "entries": [{ "osc": "/v", "cc": 7 }] } })
}

#[test]
fn a_catalogue_with_every_prescribed_change_of_form_has_no_difference_with_its_0_6_reading() {
    let older_0_4 = json!({
        "device": { "name": "Four", "osc_prefix": "/four", "manufacturer_id": [67] },
        "midi_in": { "_notes": "from midi_in", "cc": "/cc/{num} {value} {channel}" },
        "identity": { "reply": [240, 126, "{dev}", 6, 2], "ports": ["Four", ""] },
        "programs": { "before": [[240, "{dev}", "{bank:u7+0x40}", 247]] },
        "commands": [
            { "osc": "/x", "pre": [{ "bind": "a", "expr": "v + 1" }], "_electra": { "parameter": { "type": "sysex", "number": 2 } } },
            { "osc": "/y", "forward": { "path": "/p", "args": ["{value}", 5] } }
        ],
        "replies": [{ "match_frame": [240, "{dev}", "{value:u14_msb}", 247], "emit_osc": "/r" }],
        "triggers": { "entries": [{ "osc": "/kick", "sound": "kick", "note_range": [36, 36], "note": 36, "_name": "Kick", "_description": "the kick" }] }
    });
    let older_0_5 = json!({
        "$schema": D05,
        "device": { "name": "Five", "osc_prefix": "/five", "revision": "1", "manufacturer_id": [68] },
        "capabilities": ["lua"],
        "sysex": { "header": [240], "footer": [247] },
        "midi_out": { "default_channel": 0, "note_offset": 2 },
        "identity": { "reply_text": "five", "ports": ["Five"], "sends": { "clock": false } },
        "cc_params": { "entries": [{ "osc": "/v", "cc": 7, "transform_reverse": "return value" }] }
    });
    let profile_0_5 = json!({ "$schema": P05, "identity": { "reply_text": "acme", "ports": [""], "sends": { "clock": true } } });
    let profile_0_4 = json!({ "identity": { "ports": ["Old"] } });
    let found = run(&[
        ("acme/whole", whole_driver()),
        ("acme/_profile", whole_profile()),
        ("acme/_profile.five", profile_0_5),
        ("acme/_profile.four", profile_0_4),
        ("acme/base", base_driver()),
        ("acme/linked", linked(false)),
        ("acme/own", linked(true)),
        ("acme/lone", lone_id_driver()),
        ("acme/four", older_0_4),
        ("acme/five", older_0_5),
    ]);
    assert_eq!(found.files, 10);
    assert!(found.refused_in_0_6.is_empty() && found.refused_in_0_7.is_empty(), "{found}");
    assert!(found.failures.is_empty(), "{found}");
    assert!(found.to_string().lines().any(|l| l == "differences with the 0.6 text: 0"), "{found}");
}

/// A driver whose commands carry a `sysex` `_electra` with one key more, beside `parameter` and
/// inside it.
fn electra_with_more_keys_driver() -> Value {
    json!({
        "$schema": D06,
        "device": { "name": "More", "osc_prefix": "/more" },
        "commands": [
            { "osc": "/label", "frame": [240, 247], "_electra": { "parameter": { "type": "sysex", "number": 1 }, "label": "x" } },
            { "osc": "/device", "frame": [240, 247], "_electra": { "parameter": { "type": "sysex", "number": 2, "deviceId": 3 } } }
        ]
    })
}

#[test]
fn a_sysex_electra_with_one_more_key_kept_beside_its_id_is_no_difference() {
    let doc = electra_with_more_keys_driver();
    let (old, new) = texts(Kind::Device, &doc);
    assert_eq!(differences(&judge_text(Kind::Device, &old, &new)), BTreeSet::new(), "{new}");
}

#[test]
fn an_output_that_removes_a_sysex_electra_with_one_more_key_is_a_difference() {
    let doc = electra_with_more_keys_driver();
    let (old, new) = texts(Kind::Device, &doc);
    for n in [0, 1] {
        // The text expected of the conversion, but for one command that loses its note.
        let mut altered = new.clone();
        for m in [0, 1] {
            altered["commands"][m]["id"] = json!(m + 1);
            altered["commands"][m]["_electra"] = doc["commands"][m]["_electra"].clone();
        }
        altered["commands"][n].as_object_mut().unwrap().remove("_electra");
        let named = BTreeSet::from([format!("/commands/{n}/_electra")]);
        assert_eq!(differences(&judge_text(Kind::Device, &old, &altered)), named, "{altered}");
    }
}

#[test]
fn a_converted_value_of_the_device_block_or_of_another_block_of_a_driver_is_named_at_its_deepest_0_6_key() {
    assert_each_named(
        Kind::Device,
        &whole_driver(),
        [
            set("/$schema", json!("https://w3id.org/osd/0.6/device.schema.json")),
            removed("/_note"),
            set("/_coverage/deep/list/1", json!(3)),
            removed("/_coverage/deep"),
            set("/_coverage/deep/list", json!([1])),
            set("/device/name", json!("Other")),
            removed("/device/vendor"),
            set("/device/osc_prefix", json!("/other")),
            set("/device/device_id/0", json!(17)),
            set("/device/rate_limit_hz", json!(51)),
            removed("/device/rate_limit_hz"),
            removed("/device/_about"),
            set("/device/transport/host", json!("elsewhere")),
            set("/device/transport/port", json!(9002)),
            removed("/device/transport/reply_port"),
            removed("/device/transport/passthrough_prefix"),
            removed("/device/transport"),
            set("/loads_sounds_of/0", json!("another")),
            set("/loads_sounds_of", json!([])),
            set("/sysex/header/1", json!(66)),
            removed("/sysex/_why"),
            removed("/sysex"),
            set("/midi_out/default_channel", json!(2)),
            removed("/midi_out/default_channel"),
            set("/midi_in/1", json!("note_off")),
            set("/midi_in", json!(["note_on"])),
            removed("/midi_in"),
        ],
    );
}

#[test]
fn a_converted_value_of_the_identity_or_of_the_programs_of_a_driver_is_named_at_its_deepest_0_6_key() {
    assert_each_named(
        Kind::Device,
        &whole_driver(),
        [
            set("/identity/request/3", json!(7)),
            set("/identity/reply/2", json!(0)),
            removed("/identity/reply"),
            set("/identity/family/0", json!(3)),
            removed("/identity/member"),
            set("/identity/usb/0/product_id", json!(6)),
            set("/identity/usb", json!([])),
            removed("/identity/_seen"),
            set("/programs/program_change", json!(false)),
            set("/programs/banks/0/size", json!(99)),
            removed("/programs/banks/0/writable"),
            set("/programs/banks", json!([])),
            set("/programs/before/0/2", json!(2)),
            removed("/programs/before"),
            set("/programs/delay_ms", json!(21)),
            set("/programs/dump/request/2", json!(17)),
            removed("/programs/dump/reply_timeout_ms"),
            removed("/programs/notes"),
            removed("/programs"),
        ],
    );
}

#[test]
fn a_converted_setting_of_a_table_is_named_at_its_0_6_key() {
    assert_each_named(
        Kind::Device,
        &whole_driver(),
        [
            set("/cc_params/channel", json!(4)),
            removed("/cc_params/channel"),
            removed("/cc_params/_table"),
            set("/cc_params/entries/0/channel", json!(5)),
            removed("/cc_params/entries/0/default"),
            set("/cc_params/entries/0/step_count", json!(5)),
            set("/params/get_frame/1", json!(65)),
            set("/params/get_frame/2/arg", json!("p")),
            set("/params/get_frame", json!([240, 247])),
            set("/params/set_frame/4/arg", json!("p")),
            removed("/params/set_frame/4/arg"),
            set("/params/value_type", json!("u14")),
            removed("/params/_table"),
            removed("/triggers/_table"),
        ],
    );
}

#[test]
fn a_converted_field_of_a_command_or_of_a_trigger_is_named_at_its_0_6_key() {
    assert_each_named(
        Kind::Device,
        &whole_driver(),
        [
            set("/commands/0/section", json!("other")),
            removed("/commands/0/notes"),
            set("/commands/0/args/0/name", json!("other")),
            set("/commands/0/args/0/type", json!("int")),
            set("/commands/0/args/0/range/1", json!(4)),
            removed("/commands/0/args/0/default"),
            set("/commands/0/args/0/values/Off", json!(2)),
            removed("/commands/0/args/0/values/a~1b~0c"),
            removed("/commands/0/args/0/_arg"),
            set("/commands/0/args", json!([])),
            removed("/commands/0/args"),
            set("/commands/0/frame/1", json!(65)),
            set("/commands/0/frame/2/arg", json!("other")),
            set("/commands/0/frame", json!([240, 247])),
            set("/commands/1/_electra/parameter/number", json!(4)),
            set("/commands/3/forward/args/0", json!(6)),
            removed("/commands/3/forward"),
            set("/triggers/entries/0/sound", json!("snare")),
            removed("/triggers/entries/0/section"),
            set("/triggers/entries/0/channel", json!(10)),
            set("/triggers/entries/0/note_range/1", json!(38)),
            removed("/triggers/entries/0/note_range"),
            removed("/triggers/entries/0/velocity_usage"),
            set("/triggers/entries/0/comments", json!("high")),
            removed("/triggers/entries/0/_notes"),
            set("/triggers/entries/1/note", json!(39)),
            removed("/triggers/entries/1/_name"),
            set("/triggers/entries/1/_description", json!(5)),
        ],
    );
}

#[test]
fn a_key_the_correspondence_does_not_give_is_named_at_its_0_7_pointer() {
    assert_each_named(
        Kind::Device,
        &whole_driver(),
        [
            added("/_added", json!(1)),
            added("/_coverage/more", json!(1)),
            added("/device/_added", json!(1)),
            added("/device/transport/_added", json!(1)),
            added("/identity/reply_text", json!("added")),
            added("/identity/endpoint_name", json!({ "operator": "equals", "value": "x" })),
            added("/identity/ports/0/direction", json!("input")),
            added("/programs/banks/0/first_pc", json!(1)),
            added("/cc_params/_added", json!(1)),
            added("/params/_added", json!(1)),
            added("/commands/0/args/0/values/More", json!(2)),
            added("/commands/1/section", json!("x")),
            added("/triggers/entries/1/section", json!("x")),
            added("/triggers/entries/1/title", json!("Snare")),
            // The removed fields, written all the same.
            added("/device/revision", json!("2")),
            added("/device/transport/rate_limit_hz", json!(100)),
            added("/sysex/footer", json!([247])),
            added("/midi_out/note_offset", json!(0)),
            added("/identity/sends", json!({ "clock": true })),
        ],
    );
}

#[test]
fn a_moved_value_changed_or_removed_is_named_at_its_0_6_pointer() {
    use Alter::{Remove, Set};
    assert_each_named(
        Kind::Device,
        &whole_driver(),
        [
            (Set("/identity/manufacturerId/1", json!(33)), "/device/manufacturer_id"),
            (Set("/identity/manufacturerId", json!([67])), "/device/manufacturer_id"),
            (Remove("/identity/manufacturerId"), "/device/manufacturer_id"),
            (Set("/commands/0/id", json!(13)), "/commands/0/_electra"),
            (Remove("/commands/0/id"), "/commands/0/_electra"),
            // The rules keep the rank of their name in 0.6, an empty name between them.
            (Set("/identity/ports/0/value", json!("Other")), "/identity/ports/0"),
            (Set("/identity/ports/1/value", json!("Other")), "/identity/ports/2"),
            (Set("/identity/ports/1/operator", json!("equals")), "/identity/ports/2"),
            (Remove("/identity/ports/1/value"), "/identity/ports/2"),
            (Remove("/identity/ports/1/operator"), "/identity/ports/2"),
            (Set("/triggers/entries/0/title", json!("Other")), "/triggers/entries/0/_name"),
            (Remove("/triggers/entries/0/title"), "/triggers/entries/0/_name"),
            (Set("/triggers/entries/0/description", json!("Other")), "/triggers/entries/0/_description"),
            (Remove("/triggers/entries/0/description"), "/triggers/entries/0/_description"),
        ],
    );
}

#[test]
fn a_maker_id_in_a_block_the_conversion_makes_or_kept_as_a_note_is_named_at_its_0_6_pointer() {
    use Alter::{Remove, Set};
    assert_each_named(
        Kind::Device,
        &lone_id_driver(),
        [
            (Set("/identity/manufacturerId", json!([67])), "/device/manufacturer_id"),
            (Remove("/identity/manufacturerId"), "/device/manufacturer_id"),
            (Remove("/identity"), "/device/manufacturer_id"),
            added("/identity/family", json!([1, 0])),
        ],
    );
    assert_each_named(
        Kind::Device,
        &linked(false),
        [
            (Set("/device/_manufacturer_id", json!([68])), "/device/manufacturer_id"),
            (Remove("/device/_manufacturer_id"), "/device/manufacturer_id"),
            added("/identity", json!({ "manufacturerId": [67] })),
            set("/midi_of", json!("acme/other")),
        ],
    );
    assert_each_named(
        Kind::Device,
        &linked(true),
        [
            (Remove("/identity/manufacturerId"), "/device/manufacturer_id"),
            added("/device/_manufacturer_id", json!([67])),
            removed("/identity/ports"),
            set("/identity/ports", json!([])),
        ],
    );
}

#[test]
fn a_list_whose_length_changes_is_named_at_the_list() {
    use Alter::{Add, Remove};
    assert_each_named(
        Kind::Device,
        &whole_driver(),
        [
            (Remove("/identity/ports/1"), "/identity/ports"),
            (Remove("/identity/ports/0"), "/identity/ports"),
            (Add("/identity/ports/-", json!({ "operator": "contains", "value": "More" })), "/identity/ports"),
            (Remove("/commands/3"), "/commands"),
            (Add("/triggers/entries/-", json!({ "osc": "/tom", "sound": "tom", "note_range": [41, 41] })), "/triggers/entries"),
            (Remove("/params/entries/0"), "/params/entries"),
            (Add("/identity/usb/-", json!({ "vendor_id": 1, "product_id": 1 })), "/identity/usb"),
            (Add("/programs/before/-", json!([192, 1])), "/programs/before"),
        ],
    );
}

#[test]
fn a_removal_or_a_rewriting_the_correspondence_does_not_prescribe_is_named() {
    use Alter::{Add, Remove};
    assert_each_named(
        Kind::Device,
        &whole_driver(),
        [
            // An `_electra` of another form gets no `id` and stays.
            added("/commands/1/id", json!(3)),
            added("/commands/2/id", json!(4)),
            added("/commands/3/id", json!(1)),
            removed("/commands/1/_electra"),
            removed("/commands/2/_electra"),
            removed("/commands/3/_electra"),
            // The texts of a command do not move.
            removed("/commands/0/_name"),
            removed("/commands/0/_description"),
            added("/commands/0/title", json!("Ping")),
            // A capability the conversion removes, written back.
            added("/capabilities", json!(["lua"])),
            (Remove("/identity/ports/1"), "/identity/ports"),
        ],
    );
    // No `transform_reverse`, or one beside a field that keeps the Lua: `lua` stays.
    let mut kept = whole_driver();
    kept["cc_params"]["entries"][0]["transform"] = json!("value");
    let mut never_reversed = kept.clone();
    never_reversed["cc_params"]["entries"][0].as_object_mut().unwrap().remove("transform_reverse");
    for doc in [kept, never_reversed] {
        assert_each_named(
            Kind::Device,
            &doc,
            [
                removed("/capabilities"),
                (Alter::Set("/capabilities", json!([])), "/capabilities"),
                (Add("/capabilities/-", json!("midi")), "/capabilities"),
            ],
        );
    }
}

#[test]
fn every_field_that_holds_lua_keeps_the_capability_a_removed_reverse_transform_leaves() {
    let fields = [
        ("params", json!({ "get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [{ "osc": "/p", "id": 1, "transform": "value" }] })),
        ("commands", json!([{ "osc": "/c", "script": "return {}" }])),
        ("replies", json!([{ "match_frame": [240, 247], "emit_osc": "/r", "script": "return {}" }])),
        ("custom_commands", json!([{ "osc": "/c", "script": "return {}" }])),
        ("identity", json!({ "reply": [240, 126], "script": "return true" })),
        ("identity", json!({ "reply": [240, 126], "firmware": { "decode": "lua", "at": [5], "script": "return ''" } })),
        ("programs", json!({ "recall_script": "return {}" })),
        ("programs", json!({ "dump": { "kind": "request", "script": "return {}" } })),
        ("sound_transfers", json!([{ "id": "one", "carries": "sound", "match_frame": [240, { "capture": "body" }, 247], "elements": { "data": { "script": "return message" } } }])),
    ];
    for (key, field) in fields {
        let mut doc = lone_id_driver();
        doc.as_object_mut().unwrap().remove("params");
        doc["cc_params"] = json!({ "entries": [{ "osc": "/v", "cc": 7, "transform_reverse": "return value" }] });
        doc[key] = field;
        let (old, new) = texts(Kind::Device, &doc);
        assert_eq!(new["capabilities"], json!(["lua"]), "{key}");
        assert_eq!(differences(&judge_text(Kind::Device, &old, &new)), BTreeSet::new(), "{doc}");
        let mut altered = new.clone();
        alter(&mut altered, &Alter::Remove("/capabilities"));
        let named = BTreeSet::from(["/capabilities".to_string()]);
        assert_eq!(differences(&judge_text(Kind::Device, &old, &altered)), named, "{doc}");
    }
}

#[test]
fn a_value_of_a_profile_changed_removed_or_added_is_one_difference_at_its_pointer() {
    use Alter::{Add, Remove, Set};
    assert_each_named(
        Kind::Profile,
        &whole_profile(),
        [
            set("/$schema", json!(P06)),
            removed("/_about"),
            set("/identity/reply_text", json!("other")),
            removed("/identity/family"),
            set("/identity/usb/0/vendor_id", json!(1)),
            set("/programs/program_change", json!(false)),
            set("/programs/banks/0/size", json!(11)),
            removed("/programs/delay_ms"),
            removed("/programs"),
            (Set("/identity/ports/0/value", json!("Other")), "/identity/ports/1"),
            (Remove("/identity/ports/1/operator"), "/identity/ports/2"),
            (Remove("/identity/ports/1"), "/identity/ports"),
            (Add("/identity/ports/-", json!({ "operator": "contains", "value": "More" })), "/identity/ports"),
            removed("/identity/ports"),
            added("/identity/ports/1/platform", json!("linux")),
            added("/identity/sends", json!({ "clock": true })),
            added("/identity/manufacturerId", json!([67])),
            added("/programs/notes", json!("added")),
            added("/_added", json!(1)),
        ],
    );
}
