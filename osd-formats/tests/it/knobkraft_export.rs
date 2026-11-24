//! An OSD device with its sound transfers exports to the text of an adaptation skeleton for
//! KnobKraft Orm: the functions the descriptor gives the constants of, each with the exact name
//! of the programming guide; the others named lost. The text is compared, never parsed nor run.

use crate::common::credited_devices;
use crate::common::knobkraft::*;
use osd_formats::{export, import, FaultKind, Format};
use serde_json::json;

#[test]
fn the_name_the_banks_and_the_request_are_written_as_the_functions_of_the_guide() {
    let (text, _) = exported(&device(json!({})));
    assert!(text.contains("def name():\n    return \"Trundler\"\n"), "{text}");
    assert!(text.contains("def numberOfBanks():\n    return 2\n"), "{text}");
    assert!(text.contains("def numberOfPatchesPerBank():\n    return 64\n"), "{text}");
    assert!(text.contains("def createEditBufferRequest(channel):\n    return [0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x10, 0xF7]\n"), "{text}");
}

#[test]
fn the_constants_follow_the_descriptor_and_not_the_example() {
    let d = device(json!({
        "device": {"name": "Other Box", "osc_prefix": "/o"},
        "programs": {
            "banks": [{"name": "I", "size": 128}, {"name": "II", "size": 128}, {"name": "III", "size": 128}],
            "dump": {"kind": "edit_buffer", "request": [240, 0, 32, 51, 127, 247]}
        }
    }));
    let (text, _) = exported(&d);
    assert!(text.contains("def name():\n    return \"Other Box\"\n"), "{text}");
    assert!(text.contains("def numberOfBanks():\n    return 3\n"), "{text}");
    assert!(text.contains("def numberOfPatchesPerBank():\n    return 128\n"), "{text}");
    assert!(text.contains("    return [0xF0, 0x00, 0x20, 0x33, 0x7F, 0xF7]\n"), "{text}");
}

#[test]
fn a_name_with_a_quote_or_a_backslash_is_escaped_in_the_text() {
    let d = device(json!({"device": {"name": "A \"B\" \\ C", "osc_prefix": "/o"}}));
    let (text, _) = exported(&d);
    assert!(text.contains("    return \"A \\\"B\\\" \\\\ C\"\n"), "{text}");
}

#[test]
fn the_functions_written_are_not_named_lost_and_each_other_function_is_named_lost_with_its_reason() {
    let (_, lost) = exported(&device(json!({})));
    let p = places(&lost);
    for f in WRITTEN {
        assert!(!p.contains(&f), "{f} is written: {p:?}");
    }
    for f in LEFT_TO_HAND {
        let l = lost.iter().find(|l| l.place == f).unwrap_or_else(|| panic!("{f} is not named lost: {p:?}"));
        assert!(!l.reason.is_empty(), "{f} has no reason");
    }
}

#[test]
fn a_function_left_to_hand_is_not_written_in_the_text() {
    let (text, _) = exported(&device(json!({})));
    for f in LEFT_TO_HAND {
        assert!(!text.contains(&format!("def {f}(")), "{f} is written: {text}");
    }
}

#[test]
fn banks_of_different_sizes_give_no_number_of_patches_per_bank() {
    let d = device(json!({"programs": {"banks": [{"name": "A", "size": 64}, {"name": "B", "size": 32}], "dump": {"kind": "none"}}}));
    let (text, lost) = exported(&d);
    assert!(text.contains("def numberOfBanks():\n    return 2\n"), "{text}");
    assert!(!text.contains("def numberOfPatchesPerBank("), "{text}");
    assert!(places(&lost).contains(&"numberOfPatchesPerBank"), "{lost:?}");
}

#[test]
fn a_device_with_no_banks_writes_neither_count_and_names_both_lost() {
    let d = device(json!({"programs": null}));
    let (text, lost) = exported(&d);
    assert!(!text.contains("def numberOfBanks(") && !text.contains("def numberOfPatchesPerBank("), "{text}");
    assert!(!text.contains("def createEditBufferRequest("), "{text}");
    let p = places(&lost);
    for f in ["numberOfBanks", "numberOfPatchesPerBank", "createEditBufferRequest"] {
        assert!(p.contains(&f), "{f} is not named lost: {p:?}");
    }
    assert!(text.contains("def name():"), "{text}");
}

#[test]
fn a_request_with_a_byte_built_at_send_time_or_none_is_not_written_and_is_named_lost() {
    let built = json!({"banks": [{"name": "A", "size": 8}], "dump": {"kind": "request", "request": [240, 1, {"arg": "n"}, 247]}});
    for programs in [built, json!({"banks": [{"name": "A", "size": 8}], "dump": {"kind": "manual"}}), json!({"banks": [{"name": "A", "size": 8}]})] {
        let (text, lost) = exported(&device(json!({"programs": programs})));
        assert!(!text.contains("def createEditBufferRequest("), "{text}");
        assert!(places(&lost).contains(&"createEditBufferRequest"), "{lost:?}");
        assert!(text.contains("def numberOfBanks():\n    return 1\n"), "{text}");
    }
}

#[test]
fn the_text_names_knobkraft_orm_its_author_its_licence_and_the_credit_of_each_source() {
    let (text, _) = exported(&device(json!({})));
    assert!(text.contains("KnobKraft Orm"), "{text}");
    assert!(text.contains("Christof Ruch"), "{text}");
    assert!(text.contains("https://github.com/christofmuc/KnobKraft-orm"), "{text}");
    assert!(text.contains("AGPL"), "{text}");
    assert!(text.contains("Dee") && text.contains("CC-BY-4.0") && text.contains("https://example.org/d"), "{text}");
}

#[test]
fn the_credit_comes_before_the_first_function() {
    let (text, _) = exported(&device(json!({})));
    let def = text.find("def ").expect("a function");
    for word in ["KnobKraft Orm", "Christof Ruch", "Dee"] {
        assert!(text.find(word).unwrap() < def, "{word} comes after the first function: {text}");
    }
}

#[test]
fn a_device_without_sources_is_refused() {
    let d = device(json!({"_sources": null}));
    assert_eq!(export(&d, Format::Knobkraft).unwrap_err().kind, FaultKind::NoSources);
}

#[test]
fn a_device_without_sound_transfers_is_refused_with_its_own_fault() {
    let d = device(json!({"sound_transfers": null}));
    let f = export(&d, Format::Knobkraft).unwrap_err();
    assert_eq!(f.kind, FaultKind::NoSoundTransfers);
    assert_eq!(f.place, "/sound_transfers");
}

#[test]
fn a_device_without_sources_nor_sound_transfers_is_refused_for_its_sources_first() {
    let d = device(json!({"_sources": null, "sound_transfers": null}));
    assert_eq!(export(&d, Format::Knobkraft).unwrap_err().kind, FaultKind::NoSources);
}

#[test]
fn every_credited_device_of_the_cases_exports_when_it_has_sound_transfers_and_is_refused_otherwise() {
    for (name, d) in credited_devices() {
        let r = export(&d, Format::Knobkraft);
        if d.device.sound_transfers.as_ref().is_some_and(|t| !t.is_empty()) {
            let t = r.unwrap_or_else(|f| panic!("{name}: {f}"));
            assert!(t.value.contains("def name():") && t.value.contains("KnobKraft Orm"), "{name}");
        } else {
            assert_eq!(r.unwrap_err().kind, FaultKind::NoSoundTransfers, "{name}");
        }
    }
}

#[test]
fn a_knobkraft_adaptation_cannot_be_imported() {
    let f = import("def name():\n    return \"X\"\n", Format::Knobkraft, &crate::common::source()).unwrap_err();
    assert_eq!(f.kind, FaultKind::InvalidFile);
    let rule = f.rule.to_lowercase();
    assert!(rule.contains("knobkraft") && rule.contains("import"), "{}", f.rule);
}

#[test]
fn an_import_of_text_that_is_not_python_is_refused_the_same_way() {
    let f = import("{\"not\": \"an adaptation\"}", Format::Knobkraft, &crate::common::source()).unwrap_err();
    assert_eq!(f.kind, FaultKind::InvalidFile);
}
