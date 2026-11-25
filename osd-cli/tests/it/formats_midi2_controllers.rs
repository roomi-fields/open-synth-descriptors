//! `osd import midi2-controllers` and `osd export midi2-controllers`: a controller list from the
//! command and back, its losses on the error output, one per line.

use crate::common::{osd, repo};
use serde_json::{json, Value};

const CREDIT: [&str; 6] = ["--source-url", "https://example.org/midi2/device", "--source-author", "A maker", "--license", "CC-BY-4.0"];

fn sample() -> String {
    json!({
        "ChCtrlList": [
            {"title": "Cutoff", "ctrlType": "cc", "ctrlIndex": [74], "numSigBits": 7, "minMax": [0u64, 4294967295u64], "paramPath": "/Filter/Cutoff"},
            {"title": "Shape", "ctrlType": "cc", "ctrlIndex": [75], "numSigBits": 7, "minMax": [0u64, 4294967295u64], "paramPath": "/Mystery/Shape"},
            {"title": 7, "ctrlType": "cc", "ctrlIndex": [76], "numSigBits": 7, "minMax": [0u64, 4294967295u64], "paramPath": "/Filter/Resonance"},
        ],
        "CtrlMapList": [],
    })
    .to_string()
}

fn with_file(name: &str, text: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(name).to_string_lossy().into_owned();
    std::fs::write(&file, text).unwrap();
    (dir, file)
}

fn import_args<'a>(file: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["import", "midi2-controllers", file];
    args.extend_from_slice(&CREDIT);
    args.extend_from_slice(extra);
    args
}

#[test]
fn import_prints_the_descriptor_read_strictly_with_the_credit_and_no_into_descriptor() {
    let (_dir, file) = with_file("list.json", &sample());
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(doc["cc_params"]["entries"].as_array().unwrap().len(), 3);
    assert_eq!(doc["cc_params"]["entries"][0]["title"], "Cutoff");
    assert_eq!(doc["cc_params"]["entries"][0]["role"]["setting"], "cutoff");
    assert_eq!(doc["sources"][0]["license"], "CC-BY-4.0");
}

#[test]
fn import_prints_each_loss_on_the_error_output_one_per_line_by_its_json_pointer() {
    let (_dir, file) = with_file("list.json", &sample());
    let run = osd(&import_args(&file, &[]));
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert!(lines.iter().all(|l| l.starts_with('/') && l.contains(": ")), "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("/ChCtrlList/2/title: ") && l.contains("not a string")), "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("/ChCtrlList/1: ") && l.contains("no known role")), "{lines:?}");
    assert!(!lines.iter().any(|l| l.starts_with("/ChCtrlList/0/title: ")), "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("/vendor: ")), "{lines:?}");
}

#[test]
fn import_refuses_without_a_license_with_exit_2_and_a_json_that_is_not_a_list_with_exit_1() {
    let (_dir, file) = with_file("list.json", &sample());
    let run = osd(&["import", "midi2-controllers", &file, "--source-url", "https://example.org/x"]);
    assert_eq!(run.code, 2, "{:?}", run.stderr);
    assert!(run.stderr.contains("license"));
    assert_eq!(run.stdout, "");
    let (_d, bad) = with_file("bad.json", r#"{"Other": 1}"#);
    let run = osd(&import_args(&bad, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn export_writes_the_object_of_the_descriptor_and_prints_its_losses_on_the_error_output() {
    let file = repo().join("examples/0.6/minilogue.device.json").to_string_lossy().into_owned();
    let run = osd(&["export", "midi2-controllers", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let v: Value = serde_json::from_str(&run.stdout).unwrap();
    assert!(v["ChCtrlList"].as_array().is_some_and(|l| !l.is_empty()), "{}", run.stdout);
    assert!(v["CtrlMapList"].is_array());
    let lines: Vec<&str> = run.stderr.lines().collect();
    let credit = lines.iter().position(|l| *l == "credit to carry with the file:");
    let (losses, block) = lines.split_at(credit.unwrap_or(lines.len()));
    assert!(losses.iter().all(|l| l.starts_with('/') && l.contains(": ")), "{:?}", run.stderr);
    assert!(losses.iter().any(|l| l.starts_with("/sources: ")), "{:?}", run.stderr);
    assert!(credit.is_some(), "{:?}", run.stderr);
    assert!(block.len() > 1 && block[1..].iter().all(|l| l.starts_with("  ")), "{:?}", run.stderr);
}

#[test]
fn export_of_what_import_wrote_gives_back_the_entry() {
    let (dir, file) = with_file("list.json", &sample());
    let json = dir.path().join("d.json").to_string_lossy().into_owned();
    assert_eq!(osd(&import_args(&file, &["-o", &json])).code, 0);
    let run = osd(&["export", "midi2-controllers", &json]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let v: Value = serde_json::from_str(&run.stdout).unwrap();
    let first = &v["ChCtrlList"][0];
    assert_eq!((first["ctrlType"].clone(), first["ctrlIndex"].clone(), first["paramPath"].clone()), (json!("cc"), json!([74]), json!("/Filter/Cutoff")));
    assert_eq!(v["ChCtrlList"][1]["paramPath"], "/Mystery/Shape");
}

fn device_with_sysex_params() -> String {
    json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [{"type": "community-re", "url": "https://example.org/d", "author": "Dee", "license": "CC-BY-4.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "cc_params": {"entries": [{"osc": "/a", "cc": 7}]},
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
                   "entries": [{"osc": "/p1", "id": 1}, {"osc": "/p2", "id": 2}]},
    })
    .to_string()
}

#[test]
fn export_writes_no_param_entry_and_names_one_loss_for_the_sysex_params() {
    let (_dir, file) = with_file("d.json", &device_with_sysex_params());
    let run = osd(&["export", "midi2-controllers", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let v: Value = serde_json::from_str(&run.stdout).unwrap();
    let entries = v["ChCtrlList"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{}", run.stdout);
    assert_eq!(entries[0]["ctrlType"], "cc");
    let lines: Vec<&str> = run.stderr.lines().filter(|l| l.starts_with("/params")).collect();
    assert_eq!(lines.len(), 1, "{:?}", run.stderr);
    assert!(lines[0].starts_with("/params: "), "{lines:?}");
}

#[test]
fn import_refuses_a_param_ctrl_type_with_exit_1_at_its_pointer_and_prints_no_descriptor() {
    let list = json!([{"title": "P", "ctrlType": "param", "ctrlIndex": [12]}]).to_string();
    let (_dir, file) = with_file("p.json", &list);
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(run.stderr.contains("/0/ctrlType"), "{:?}", run.stderr);
}

#[test]
fn import_of_a_standard_type_the_descriptor_cannot_carry_prints_the_descriptor_and_the_loss_at_its_pointer() {
    let list = json!([{"ctrlType": "cc", "ctrlIndex": [7]}, {"ctrlType": "pnrc", "ctrlIndex": [1]}]).to_string();
    let (_dir, file) = with_file("b.json", &list);
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    assert!(run.stderr.lines().any(|l| l.starts_with("/1: ")), "{:?}", run.stderr);
}

#[test]
fn a_device_with_sysex_params_exported_then_imported_has_no_params_block() {
    let (dir, file) = with_file("d.json", &device_with_sysex_params());
    let exported = osd(&["export", "midi2-controllers", &file]);
    assert_eq!(exported.code, 0, "{:?}", exported.stderr);
    let list = dir.path().join("list.json").to_string_lossy().into_owned();
    std::fs::write(&list, &exported.stdout).unwrap();
    let run = osd(&import_args(&list, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert!(doc.get("params").is_none(), "{}", run.stdout);
}

#[test]
fn the_unknown_format_message_and_the_helps_list_midi2_controllers() {
    let (_dir, file) = with_file("list.json", &sample());
    for word in ["import", "export"] {
        let run = osd(&[word, "no-such-format", &file]);
        assert_eq!(run.code, 2, "{word}: {:?}", run.stderr);
        assert!(run.stderr.contains("midi2-controllers"), "{word}: {:?}", run.stderr);
        let help = osd(&[word, "--help"]);
        assert!(help.stdout.contains("midi2-controllers") || help.stderr.contains("midi2-controllers"), "{word}");
    }
    assert!(osd(&["--help"]).stdout.contains("midi2-controllers"));
}
