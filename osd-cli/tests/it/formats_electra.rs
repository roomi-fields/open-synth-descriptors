//! `osd import electra-one` and `osd export electra-one`: a preset from the command, its
//! losses on the error output.

use crate::common::{osd, repo};
use serde_json::{json, Value};

const CREDIT: [&str; 6] = ["--source-url", "https://app.electra.one/preset/abc", "--source-author", "A maker", "--license", "CC-BY-4.0"];

fn sample() -> String {
    json!({
        "version": 3, "name": "P", "projectId": "p1",
        "pages": [{"id": 1, "name": "Page 1"}],
        "groups": [{"id": 1, "pageId": 1, "name": "Filter", "bounds": [0, 0, 300, 100]}],
        "overlays": [], "devices": [{"id": 1, "name": "D", "port": 1, "channel": 1}],
        "controls": [
            {"id": 1, "type": "fader", "name": "Cutoff", "color": "FF0000", "bounds": [0, 0, 146, 56], "pageId": 1,
             "controlSetId": 1, "groupId": 1, "inputs": [{"potId": 1, "valueId": "value"}],
             "values": [{"id": "value", "min": 0, "max": 127, "message": {"type": "cc7", "deviceId": 1, "parameterNumber": 74}}]},
            {"id": 2, "type": "pad", "name": "Dump", "bounds": [0, 0, 146, 56], "pageId": 1, "controlSetId": 1,
             "inputs": [{"potId": 2, "valueId": "value"}],
             "values": [{"id": "value", "message": {"type": "sysex", "deviceId": 1, "data": [240, 247]}}]},
            {"id": 3, "type": "pad", "name": "Note", "bounds": [0, 0, 146, 56], "pageId": 1, "controlSetId": 1,
             "inputs": [{"potId": 3, "valueId": "value"}],
             "values": [{"id": "value", "message": {"type": "note", "deviceId": 1, "parameterNumber": 60}}]},
        ],
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
    let mut args = vec!["import", "electra-one", file];
    args.extend_from_slice(&CREDIT);
    args.extend_from_slice(extra);
    args
}

#[test]
fn import_prints_the_descriptor_read_strictly_with_the_control_the_command_and_the_credit() {
    let (_dir, file) = with_file("p.json", &sample());
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(doc["cc_params"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(doc["cc_params"]["entries"][0]["section"], "Filter");
    assert_eq!(doc["commands"][0]["frame"], json!([240, 247]));
    assert_eq!(doc["sources"][0]["license"], "CC-BY-4.0");
}

#[test]
fn import_prints_each_loss_on_the_error_output_one_per_line_by_its_json_pointer() {
    let (_dir, file) = with_file("p.json", &sample());
    let run = osd(&import_args(&file, &[]));
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert!(lines.iter().all(|l| l.starts_with('/')), "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("/controls/2/values/0/message: ")), "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("/controls/0/bounds: ")), "{lines:?}");
    assert!(!run.stdout.contains("/controls/"));
}

#[test]
fn import_refuses_without_a_license_with_exit_2_and_a_file_that_is_not_json_with_exit_1() {
    let (_dir, file) = with_file("p.json", &sample());
    let run = osd(&["import", "electra-one", &file, "--source-url", "https://example.org/x"]);
    assert_eq!(run.code, 2, "{:?}", run.stderr);
    assert!(run.stderr.contains("license"));
    assert_eq!(run.stdout, "");
    let (_d, bad) = with_file("bad.json", "{not json");
    let run = osd(&import_args(&bad, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn import_of_a_preset_without_controls_is_exit_1_with_the_pointer() {
    let (_dir, file) = with_file("p.json", r#"{"version": 3}"#);
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("/controls"), "{:?}", run.stderr);
}

#[test]
fn export_writes_a_preset_of_the_descriptor_and_prints_its_losses_on_the_error_output() {
    let file = repo().join("examples/0.5/synth.device.json").to_string_lossy().into_owned();
    let run = osd(&["export", "electra-one", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let preset: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(preset["version"], 3);
    assert!(!preset["controls"].as_array().unwrap().is_empty());
    let (losses, credit) = match run.stderr.split_once("credit to carry with the file:") {
        Some((before, after)) => (before, Some(after)),
        None => (run.stderr.as_str(), None),
    };
    assert!(losses.lines().all(|l| l.starts_with('/')), "{:?}", run.stderr);
    if let Some(after) = credit {
        assert!(after.lines().skip(1).all(|l| l.starts_with(char::is_whitespace)), "{:?}", run.stderr);
    }
}

#[test]
fn export_of_what_import_wrote_gives_back_the_carried_controls_and_commands() {
    let (dir, file) = with_file("p.json", &sample());
    let json = dir.path().join("d.json").to_string_lossy().into_owned();
    assert_eq!(osd(&import_args(&file, &["-o", &json])).code, 0);
    let run = osd(&["export", "electra-one", &json]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let preset: Value = serde_json::from_str(&run.stdout).unwrap();
    let kinds: Vec<&str> = preset["controls"].as_array().unwrap().iter().map(|c| c["values"][0]["message"]["type"].as_str().unwrap()).collect();
    assert_eq!(kinds.len(), 2);
    assert!(kinds.contains(&"cc7") && kinds.contains(&"sysex"), "{kinds:?}");
}

#[test]
fn export_writes_the_file_named_by_o_and_refuses_an_existing_one_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let file = repo().join("examples/0.5/synth.device.json").to_string_lossy().into_owned();
    let out = dir.path().join("out.json").to_string_lossy().into_owned();
    let run = osd(&["export", "electra-one", &file, "-o", &out]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(serde_json::from_str::<Value>(&std::fs::read_to_string(&out).unwrap()).is_ok());
    assert_eq!(osd(&["export", "electra-one", &file, "-o", &out]).code, 2);
    assert_eq!(osd(&["export", "electra-one", &file, "-o", &out, "--force"]).code, 0);
}

#[test]
fn the_unknown_format_message_and_the_helps_list_electra_one() {
    let (_dir, file) = with_file("p.json", &sample());
    for word in ["import", "export"] {
        let run = osd(&[word, "no-such-format", &file]);
        assert_eq!(run.code, 2, "{word}: {:?}", run.stderr);
        assert!(run.stderr.contains("electra-one"), "{word}: {:?}", run.stderr);
        let help = osd(&[word, "--help"]);
        assert!(help.stdout.contains("electra-one") || help.stderr.contains("electra-one"), "{word}");
    }
    assert!(osd(&["--help"]).stdout.contains("electra-one"));
}
