//! `osd import oscquery` and `osd export oscquery`: a namespace from the command and back, its
//! losses on the error output, one per line.

use crate::common::{osd, repo};
use serde_json::{json, Value};

const CREDIT: [&str; 6] = ["--source-url", "http://127.0.0.1:9000", "--source-author", "A tool", "--license", "CC0-1.0"];

fn sample() -> String {
    json!({
        "FULL_PATH": "/", "ACCESS": 0,
        "CONTENTS": {"synth": {"FULL_PATH": "/synth", "ACCESS": 0, "CONTENTS": {
            "cutoff": {"FULL_PATH": "/synth/cutoff", "ACCESS": 2, "TYPE": "i", "RANGE": [{"MIN": 0, "MAX": 127}], "DESCRIPTION": "Cutoff"},
            "status": {"FULL_PATH": "/synth/status", "ACCESS": 1, "TYPE": "s"},
        }}}
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
    let mut args = vec!["import", "oscquery", file];
    args.extend_from_slice(&CREDIT);
    args.extend_from_slice(extra);
    args
}

#[test]
fn import_prints_the_descriptor_read_strictly_with_the_command_and_the_credit() {
    let (_dir, file) = with_file("ns.json", &sample());
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(doc["device"]["osc_prefix"], "/synth");
    assert_eq!(doc["commands"].as_array().unwrap().len(), 1);
    assert_eq!(doc["commands"][0]["osc"], "/cutoff");
    assert_eq!(doc["sources"][0]["license"], "CC0-1.0");
}

#[test]
fn import_prints_each_loss_on_the_error_output_one_per_line_by_its_json_pointer() {
    let (_dir, file) = with_file("ns.json", &sample());
    let run = osd(&import_args(&file, &[]));
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert!(lines.iter().all(|l| l.starts_with('/') && l.contains(": ")), "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("/CONTENTS/synth/CONTENTS/cutoff/DESCRIPTION: ")), "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("/CONTENTS/synth/CONTENTS/status: ")), "{lines:?}");
    assert!(!run.stdout.contains("/CONTENTS/"));
}

#[test]
fn import_refuses_without_a_license_with_exit_2_and_a_json_that_is_not_a_namespace_with_exit_1() {
    let (_dir, file) = with_file("ns.json", &sample());
    let run = osd(&["import", "oscquery", &file, "--source-url", "http://127.0.0.1:9000"]);
    assert_eq!(run.code, 2, "{:?}", run.stderr);
    assert!(run.stderr.contains("license"));
    assert_eq!(run.stdout, "");
    let (_d, bad) = with_file("bad.json", r#"{"FULL_PATH": "/", "ACCESS": 7}"#);
    let run = osd(&import_args(&bad, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("/ACCESS"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn export_writes_a_namespace_of_the_descriptor_and_prints_its_losses_on_the_error_output() {
    let file = repo().join("examples/0.5/synth.device.json").to_string_lossy().into_owned();
    let run = osd(&["export", "oscquery", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let ns: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(ns["FULL_PATH"], "/");
    assert_eq!(ns["CONTENTS"]["test_synth"]["CONTENTS"]["filter"]["CONTENTS"]["cutoff"]["TYPE"], "i");
    assert!(!run.stderr.is_empty());
    assert!(run.stderr.lines().all(|l| l.starts_with('/') && l.contains(": ")), "{:?}", run.stderr);
}

#[test]
fn export_of_what_import_wrote_gives_back_the_method() {
    let (dir, file) = with_file("ns.json", &sample());
    let json = dir.path().join("d.json").to_string_lossy().into_owned();
    assert_eq!(osd(&import_args(&file, &["-o", &json])).code, 0);
    let run = osd(&["export", "oscquery", &json]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let ns: Value = serde_json::from_str(&run.stdout).unwrap();
    let cutoff = &ns["CONTENTS"]["synth"]["CONTENTS"]["cutoff"];
    assert_eq!((cutoff["ACCESS"].clone(), cutoff["TYPE"].clone(), cutoff["RANGE"].clone()), (json!(2), json!("i"), json!([{"MIN": 0, "MAX": 127}])));
}

#[test]
fn export_writes_the_file_named_by_o_and_refuses_an_existing_one_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let file = repo().join("examples/0.5/synth.device.json").to_string_lossy().into_owned();
    let out = dir.path().join("out.json").to_string_lossy().into_owned();
    let run = osd(&["export", "oscquery", &file, "-o", &out]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(serde_json::from_str::<Value>(&std::fs::read_to_string(&out).unwrap()).is_ok());
    assert_eq!(osd(&["export", "oscquery", &file, "-o", &out]).code, 2);
    assert_eq!(osd(&["export", "oscquery", &file, "-o", &out, "--force"]).code, 0);
}

#[test]
fn the_unknown_format_message_and_the_helps_list_oscquery() {
    let (_dir, file) = with_file("ns.json", &sample());
    for word in ["import", "export"] {
        let run = osd(&[word, "no-such-format", &file]);
        assert_eq!(run.code, 2, "{word}: {:?}", run.stderr);
        assert!(run.stderr.contains("oscquery"), "{word}: {:?}", run.stderr);
        let help = osd(&[word, "--help"]);
        assert!(help.stdout.contains("oscquery") || help.stderr.contains("oscquery"), "{word}");
    }
    assert!(osd(&["--help"]).stdout.contains("oscquery"));
}
