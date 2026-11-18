//! A MIDNAM file imports into an OSD device descriptor `osd` reads strictly, carrying the source
//! the caller gives and nothing the file says of credit; a file not valid by the DTD is
//! refused with its line.

use crate::common::midnam::*;
use crate::common::xml::{document_text, Node};
use crate::common::{electra::json_of, source};
use osd_formats::{import, FaultKind, Format};
use serde_json::{json, Value};

fn entries(d: &Value) -> Vec<Value> {
    d["cc_params"]["entries"].as_array().cloned().unwrap_or_default()
}

fn refused(text: &str) -> osd_formats::Fault {
    import(text, Format::Midnam, &source()).expect_err("the file is refused")
}

#[test]
fn the_import_of_a_document_valid_by_the_dtd_reads_strictly_with_the_vendor_and_the_name() {
    let (d, _) = imported(&rich_text());
    assert_eq!(d["device"]["vendor"], "Lofty");
    assert_eq!(d["device"]["name"], "Trundler");
    let prefix = d["device"]["osc_prefix"].as_str().unwrap();
    assert_eq!(prefix, "/trundler");
}

#[test]
fn the_address_prefix_is_built_from_the_name_in_lower_case_words_joined_by_underscores() {
    let text = rich_text().replace("Trundler", "Test Synth Mk-2");
    let (d, _) = imported(&text);
    assert_eq!(d["device"]["osc_prefix"], "/test_synth_mk_2");
}

#[test]
fn each_kind_of_control_becomes_an_entry_with_its_binding_its_range_and_its_name() {
    let kinds = [
        ("7bit", "74", json!({"cc": 74})),
        ("14bit", "5", json!({"cc": 5, "cc_lsb": 37})),
        ("NRPN", "300", json!({"nrpn_msb": 2, "nrpn_lsb": 44})),
        ("RPN", "130", json!({"rpn_msb": 1, "rpn_lsb": 2})),
    ];
    for (kind, number, binding) in kinds {
        let (d, _) = imported(&control_document(kind, number, "3", "99", "Some Control", None));
        let e = &entries(&d)[0];
        for (k, v) in binding.as_object().unwrap() {
            assert_eq!(&e[k], v, "{kind} {k}");
        }
        assert_eq!(e["range"], json!([3, 99]), "{kind}");
        assert_eq!(e["_name"], "Some Control", "{kind}");
        assert!(e.get("default").is_none(), "{kind}: a default the file does not give");
        assert!(e["osc"].as_str().unwrap().starts_with('/'), "{kind}");
    }
}

#[test]
fn a_control_without_a_type_is_a_7_bit_control_as_the_dtd_says() {
    let text = control_document("7bit", "7", "0", "127", "Volume", None).replace("Type=\"7bit\" ", "");
    assert!(!text.contains("Type="));
    let (d, _) = imported(&text);
    assert_eq!(entries(&d)[0]["cc"], 7);
}

#[test]
fn the_default_of_the_values_is_the_default_of_the_control() {
    let (d, _) = imported(&control_document("7bit", "74", "0", "127", "Cutoff", Some("64")));
    assert_eq!(entries(&d)[0]["default"], 64);
}

#[test]
fn the_controls_come_in_the_order_of_the_file_with_distinct_addresses_even_for_the_same_name() {
    let mut doc = rich_document();
    let list = doc.at_mut(&[1, 5]);
    list.children = ["A", "A", "B"]
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let number = i.to_string();
            Node::new("Control", &[("Number", number.as_str()), ("Name", *n)]).with(Node::new("Values", &[("Min", "0"), ("Max", "127")]))
        })
        .collect();
    let (d, _) = imported(&document_text(&doc));
    let es = entries(&d);
    let numbers: Vec<i64> = es.iter().map(|e| e["cc"].as_i64().unwrap()).collect();
    assert_eq!(numbers, vec![0, 1, 2]);
    let mut oscs: Vec<&str> = es.iter().map(|e| e["osc"].as_str().unwrap()).collect();
    oscs.sort();
    oscs.dedup();
    assert_eq!(oscs.len(), 3, "{es:?}");
}

#[test]
fn a_note_becomes_a_trigger_with_its_sound_and_its_note() {
    let (d, _) = imported(&rich_text());
    let t = &d["triggers"]["entries"][0];
    assert_eq!(t["sound"], "Kick");
    assert_eq!(t["note"], 36);
    assert_eq!(t["note_range"], json!([36, 36]));
    assert!(t["osc"].as_str().unwrap().starts_with('/'));
}

#[test]
fn a_patch_bank_becomes_a_bank_with_its_select_its_size_its_first_number_and_its_first_program_change() {
    let (d, _) = imported(&rich_text());
    let b = &d["programs"]["banks"][0];
    assert_eq!(b["name"], "Bank A");
    assert_eq!((b["msb"].clone(), b["lsb"].clone()), (json!(1), json!(2)));
    assert_eq!(b["size"], 2);
    assert_eq!(b["first_number"], 0);
    assert_eq!(b["first_pc"], 0);
    assert_eq!(d["programs"]["program_change"], true);
}

#[test]
fn a_document_with_no_note_no_control_and_no_patch_gives_a_device_with_none_of_those_blocks() {
    let mut doc = rich_document();
    let master = doc.at_mut(&[1]);
    master.children.truncate(4);
    master.at_mut(&[3]).children.retain(|c| c.name == "AvailableForChannels");
    master.at_mut(&[3]).children.push(Node::new("PatchBank", &[]).with(Node::new("PatchNameList", &[])));
    let (d, _) = imported(&document_text(&doc));
    assert!(d.get("cc_params").is_none() && d.get("triggers").is_none(), "{d}");
    assert!(d["programs"]["banks"].as_array().is_none_or(|b| b.is_empty()), "{d}");
}

#[test]
fn the_source_given_is_the_only_source_and_what_the_file_says_of_its_author_is_not_written() {
    let (d, _) = imported(&rich_text());
    let src = source();
    assert_eq!(d["_sources"].as_array().unwrap().len(), 1);
    assert_eq!(d["_sources"][0]["license"], src.license.clone().unwrap());
    assert_eq!(d["_sources"][0]["url"], src.url.clone().unwrap());
    assert_eq!(d["_sources"][0]["author"], src.author.clone().unwrap());
    assert!(!d.to_string().contains("Someone"), "the author of the file is written: {d}");
    assert!(!d.to_string().contains("CC0-1.0"), "the licence of the file is written: {d}");
}

#[test]
fn a_source_without_a_credit_or_without_a_licence_is_refused() {
    let mut no_license = source();
    no_license.license = None;
    let f = import(&rich_text(), Format::Midnam, &no_license).unwrap_err();
    assert_eq!(f.kind, FaultKind::SourceIncomplete);
    let mut no_credit = source();
    (no_credit.url, no_credit.author, no_credit.attribution) = (None, None, None);
    assert_eq!(import(&rich_text(), Format::Midnam, &no_credit).unwrap_err().kind, FaultKind::SourceIncomplete);
    let mut blank = source();
    blank.license = Some(String::new());
    assert_eq!(import(&rich_text(), Format::Midnam, &blank).unwrap_err().kind, FaultKind::SourceIncomplete);
}

#[test]
fn a_file_that_is_not_xml_is_refused_with_the_line_of_the_fault() {
    let f = refused("<?xml version=\"1.0\"?>\n<MIDINameDocument>\n<Author>x</Author>\n<MasterDeviceNames>\n</MIDINameDocument>\n");
    assert_eq!(f.kind, FaultKind::InvalidFile);
    assert!(f.place.contains("line "), "{f}");
    assert_eq!(refused("not xml at all").kind, FaultKind::InvalidFile);
    assert_eq!(refused("").kind, FaultKind::InvalidFile);
}

#[test]
fn a_file_with_another_root_is_refused() {
    let f = refused("<?xml version=\"1.0\"?>\n<Other/>\n");
    assert_eq!(f.kind, FaultKind::InvalidFile);
    assert!(f.place.contains("line 2"), "{f}");
}

#[test]
fn an_unknown_element_a_missing_attribute_a_value_out_of_its_list_and_a_child_out_of_content_are_refused_at_their_line() {
    let base = rich_text();
    let cases = [
        ("an unknown element", base.replace("<Note Number=\"36\"", "<Bogus/>\n<Note Number=\"36\"")),
        ("a missing attribute", base.replace("Number=\"74\" ", "")),
        ("a value out of its list", base.replace("Type=\"NRPN\"", "Type=\"9bit\"")),
        ("a child out of content", base.replace("<Values Min=\"0\" Max=\"16383\"/>", "<Values Min=\"0\" Max=\"16383\"><Control Number=\"1\" Name=\"x\"/></Values>")),
    ];
    for (what, text) in cases {
        assert_ne!(text, base, "{what}: the alteration did nothing");
        let f = refused(&text);
        assert_eq!(f.kind, FaultKind::InvalidFile, "{what}: {f}");
        let line: usize = f.place.strip_prefix("line ").unwrap_or_else(|| panic!("{what}: place {:?}", f.place)).split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap();
        let offending = text.lines().position(|l| match what {
            "an unknown element" => l.contains("<Bogus"),
            "a missing attribute" => l.contains("Name=\"Filter Cutoff\""),
            "a value out of its list" => l.contains("9bit"),
            _ => l.contains("<Control Number=\"1\""),
        });
        assert_eq!(Some(line), offending.map(|n| n + 1), "{what}: {f}");
    }
}

#[test]
fn every_element_of_a_valid_document_refuses_an_attribute_it_does_not_declare_a_missing_required_attribute_and_a_child_it_does_not_allow() {
    let base = rich_document();
    assert!(import(&document_text(&base), Format::Midnam, &source()).is_ok());
    let mut tried = 0;
    for path in base.paths() {
        let name = {
            let mut n = base.clone();
            n.at_mut(&path).name.clone()
        };
        let mut alterations: Vec<(String, Node)> = Vec::new();
        let mut unknown_attr = base.clone();
        unknown_attr.at_mut(&path).attrs.push(("Bogus".into(), "1".into()));
        alterations.push((format!("{name}: an attribute Bogus"), unknown_attr));
        let mut bogus_child = base.clone();
        bogus_child.at_mut(&path).children.push(Node::new("Bogus", &[]));
        alterations.push((format!("{name}: a child Bogus"), bogus_child));
        let required: Vec<String> =
            rule(&name).unwrap().2.iter().map(|a| attribute_parts(a)).filter(|(_, _, d)| d == "#REQUIRED").map(|(n, _, _)| n).collect();
        for attr in required {
            let mut missing = base.clone();
            missing.at_mut(&path).attrs.retain(|(k, _)| *k != attr);
            alterations.push((format!("{name}: {attr} missing"), missing));
        }
        for (what, doc) in alterations {
            let f = import(&document_text(&doc), Format::Midnam, &source()).expect_err(&what);
            assert_eq!(f.kind, FaultKind::InvalidFile, "{what}: {f}");
            assert!(f.place.starts_with("line "), "{what}: {f}");
            tried += 1;
        }
    }
    assert!(tried > 60, "{tried} alterations");
}

#[test]
fn the_document_does_not_need_a_document_type_to_be_read() {
    let text = rich_text().lines().filter(|l| !l.starts_with("<!DOCTYPE")).collect::<Vec<_>>().join("\n");
    assert!(import(&text, Format::Midnam, &source()).is_ok());
}

#[test]
fn each_element_the_format_carries_and_the_descriptor_cannot_hold_has_a_loss_at_its_line() {
    let text = rich_text();
    let (_, lost) = imported(&text);
    let line_of = |needle: &str| text.lines().position(|l| l.contains(needle)).unwrap() + 1;
    for (what, needle) in [
        ("the units and the mapping of the values", "Units=\"dB\""),
        ("a patch name", "Name=\"Warm Pad\""),
        ("the commands that enable a mode", "<DeviceModeEnable"),
    ] {
        let place = format!("line {}", line_of(needle));
        let found: Vec<_> = lost.iter().filter(|l| l.place == place || l.place.starts_with(&format!("{place},"))).collect();
        assert!(!found.is_empty(), "{what}: no loss at {place}: {lost:?}");
        assert!(found.iter().all(|l| !l.reason.is_empty()));
    }
}

#[test]
fn a_patch_named_as_the_export_names_it_is_not_a_loss_and_a_second_model_is() {
    let text = rich_text();
    let (_, lost) = imported(&text);
    let line = text.lines().position(|l| l.contains("Name=\"Bank A 0\"")).unwrap() + 1;
    let here = |p: &str| p == format!("line {line}") || p.starts_with(&format!("line {line},"));
    assert!(lost.iter().all(|l| !here(&l.place)), "{lost:?}");
    let mut doc = rich_document();
    let second = {
        let mut m = Node::new("Model", &[]);
        m.text = "Trundler II".into();
        m
    };
    doc.at_mut(&[1]).children.insert(2, second);
    let text = document_text(&doc);
    let (d, lost) = imported(&text);
    assert_eq!(d["device"]["name"], "Trundler");
    let at = text.lines().position(|l| l.contains("Trundler II")).unwrap() + 1;
    assert!(lost.iter().any(|l| l.place.starts_with(&format!("line {at}"))), "{lost:?}");
}

#[test]
fn what_the_file_does_not_say_is_not_invented() {
    let (d, _) = imported(&rich_text());
    let device = &d["device"];
    for key in ["revision", "manufacturer_id", "device_id", "kind", "transport"] {
        assert!(device.get(key).is_none(), "{key} is invented: {device}");
    }
    for key in ["identity", "sysex", "commands", "midi_out", "midi_in", "replies", "params"] {
        assert!(d.get(key).is_none(), "{key} is invented");
    }
    let e = &entries(&d)[1];
    assert!(e.get("section").is_none() && e.get("expects").is_none() && e.get("orientation").is_none() && e.get("channel").is_none(), "{e}");
}

#[test]
fn the_import_is_read_strictly_in_the_latest_version() {
    let t = import(&rich_text(), Format::Midnam, &source()).unwrap();
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value.clone())));
    assert!(osd::read(&text, osd::Kind::Device).is_ok(), "{text}");
    assert_eq!(json_of(&t.value)["$schema"], "https://w3id.org/osd/0.6/device.schema.json");
}

#[test]
fn a_document_of_extending_names_only_is_refused_for_a_device_has_no_name_to_read() {
    let author = Node::new("Author", &[]);
    let ext = Node::new("ExtendingDeviceNames", &[]).with(Node::new("NoteNameList", &[]).with(Node::new("Note", &[("Number", "1"), ("Name", "x")])));
    let doc = Node::new("MIDINameDocument", &[]).with(author).with(ext);
    assert_eq!(violations(&document_text(&doc)), Vec::<String>::new(), "the fixture is valid by the DTD");
    let f = refused(&document_text(&doc));
    assert_eq!(f.kind, FaultKind::InvalidFile);
}
