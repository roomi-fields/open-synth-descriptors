//! `osd import` and `osd export` on the pencilresearch/midi triggers CSV, and `--into`: its
//! losses on the error output, the credit rules, the two CSV of one device in one descriptor.

use crate::common::{osd, osd_in, repo};
use serde_json::Value;

const HEADER: &str = "manufacturer,device,section,sound_name,sound_description,midi_channel,note_number_min_value,note_number_max_value,note_number_default_value,comments,velocity_usage";
const CONTROLLER_HEADER: &str = "manufacturer,device,section,parameter_name,parameter_description,cc_msb,cc_lsb,cc_min_value,cc_max_value,cc_default_value,nrpn_msb,nrpn_lsb,nrpn_min_value,nrpn_max_value,nrpn_default_value,orientation,notes,usage";

/// A file with a full row, a row with only one bound, and a row whose channel is a word.
fn sample() -> String {
    format!(
        "{HEADER}\n\
         Lofty,Trundler,Drums,Kick,Big,10,36,38,37,choke,soft\n\
         Lofty,Trundler,Drums,Snare,,,40,,,,\n\
         Lofty,Trundler,Drums,Hat,,omni,42,42,,,\n"
    )
}

fn controllers() -> String {
    format!("{CONTROLLER_HEADER}\nLofty,Trundler,Filter,Cutoff,,74,,0,127,,,,,,,,,\n")
}

fn path_of(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

const CREDIT: [&str; 6] =
    ["--source-url", "https://github.com/pencilresearch/midi", "--source-author", "Pencilresearch", "--license", "CC-BY-SA-4.0"];

fn import_args<'a>(format: &'a str, file: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["import", format, file];
    args.extend_from_slice(&CREDIT);
    args.extend_from_slice(extra);
    args
}

fn with_csv(text: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = path_of(&dir.path().join("in.csv"));
    std::fs::write(&file, text).unwrap();
    (dir, file)
}

#[test]
fn import_prints_the_descriptor_with_its_triggers_and_the_source_given() {
    let (_dir, file) = with_csv(&sample());
    let run = osd(&import_args("midi-triggers-csv", &file, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    let entries = doc["triggers"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["sound"], "Kick");
    assert_eq!(entries[0]["channel"], 9);
    assert_eq!(entries[0]["note_range"], serde_json::json!([36, 38]));
    assert_eq!(entries[1].get("channel"), None);
    assert_eq!(doc["_sources"][0]["license"], "CC-BY-SA-4.0");
    assert_eq!(doc["_sources"][0]["url"], "https://github.com/pencilresearch/midi");
}

#[test]
fn import_prints_each_loss_on_the_error_output_one_per_line() {
    let (_dir, file) = with_csv(&sample());
    let run = osd(&import_args("midi-triggers-csv", &file, &[]));
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("line 3: "), "{:?}", lines[0]);
    assert!(lines[1].starts_with("line 4, column midi_channel: "), "{:?}", lines[1]);
}

#[test]
fn import_refuses_without_a_license_or_without_any_credit_with_exit_2() {
    let (_dir, file) = with_csv(&sample());
    let no_license = osd(&["import", "midi-triggers-csv", &file, "--source-url", "https://example.org/x"]);
    assert_eq!(no_license.code, 2, "{:?}", no_license.stderr);
    assert!(no_license.stderr.contains("license"));
    assert_eq!(no_license.stdout, "");
    let no_credit = osd(&["import", "midi-triggers-csv", &file, "--license", "CC0-1.0"]);
    assert_eq!(no_credit.code, 2, "{:?}", no_credit.stderr);
    assert_eq!(no_credit.stdout, "");
}

#[test]
fn import_of_a_file_not_valid_in_its_format_is_exit_1_with_the_line() {
    let (_dir, file) = with_csv(&format!("{HEADER}\nLofty,Trundler,only three\n"));
    let run = osd(&import_args("midi-triggers-csv", &file, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("line 2"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn a_controller_file_given_as_a_triggers_file_is_exit_1_at_line_1() {
    let (_dir, file) = with_csv(&controllers());
    let run = osd(&import_args("midi-triggers-csv", &file, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("line 1"), "{:?}", run.stderr);
}

#[test]
fn export_writes_the_triggers_csv_of_what_import_wrote_and_names_the_losses_of_the_other_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let (_keep, csv) = with_csv(&sample());
    let json = path_of(&dir.path().join("d.json"));
    assert_eq!(osd(&import_args("midi-triggers-csv", &csv, &["-o", &json])).code, 0);
    let run = osd(&["export", "midi-triggers-csv", &json]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines, [HEADER, "Lofty,Trundler,Drums,Kick,Big,10,36,38,37,choke,soft", "Lofty,Trundler,Drums,Hat,,,42,42,,,"]);
}

#[test]
fn export_of_an_example_prints_its_losses_on_the_error_output() {
    let file = path_of(&repo().join("examples/0.5/synth.device.json"));
    let run = osd(&["export", "midi-triggers-csv", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(run.stdout.starts_with(&format!("{HEADER}\n")), "{}", run.stdout);
    assert!(run.stdout.lines().count() > 1);
    assert!(run.stderr.lines().any(|l| l.starts_with("/sysex: ")), "{:?}", run.stderr);
}

#[test]
fn export_of_a_descriptor_without_sources_is_exit_1() {
    let dir = tempfile::tempdir().unwrap();
    let file = path_of(&dir.path().join("d.json"));
    std::fs::write(&file, r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}}"#).unwrap();
    let run = osd_in(dir.path(), &["export", "midi-triggers-csv", &file]);
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("_sources"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn into_adds_the_triggers_to_the_descriptor_of_the_controller_file_with_both_sources() {
    let dir = tempfile::tempdir().unwrap();
    let (_k1, cc) = with_csv(&controllers());
    let json = path_of(&dir.path().join("d.json"));
    assert_eq!(osd(&import_args("midi-csv", &cc, &["-o", &json])).code, 0);
    let (_k2, tr) = with_csv(&sample());
    let run = osd(&import_args("midi-triggers-csv", &tr, &["--into", &json]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))));
    assert_eq!(doc["cc_params"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(doc["triggers"]["entries"].as_array().unwrap().len(), 2);
    assert_eq!(doc["_sources"].as_array().unwrap().len(), 2);
    assert!(run.stderr.lines().any(|l| l.starts_with("line 3: ")), "{:?}", run.stderr);
}

#[test]
fn into_adds_the_controllers_to_the_descriptor_of_the_triggers_file() {
    let dir = tempfile::tempdir().unwrap();
    let (_k1, tr) = with_csv(&sample());
    let json = path_of(&dir.path().join("d.json"));
    assert_eq!(osd(&import_args("midi-triggers-csv", &tr, &["-o", &json])).code, 0);
    let (_k2, cc) = with_csv(&controllers());
    let run = osd(&import_args("midi-csv", &cc, &["--into", &json]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(doc["cc_params"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(doc["triggers"]["entries"].as_array().unwrap().len(), 2);
    assert_eq!(doc["_sources"].as_array().unwrap().len(), 2);
}

#[test]
fn into_a_missing_descriptor_is_exit_2_and_into_an_invalid_one_is_exit_1() {
    let (_k, tr) = with_csv(&sample());
    let missing = osd(&import_args("midi-triggers-csv", &tr, &["--into", "/nonexistent/d.json"]));
    assert_eq!(missing.code, 2, "{:?}", missing.stderr);
    assert!(missing.stderr.contains("/nonexistent/d.json"), "{:?}", missing.stderr);
    assert_eq!(missing.stdout, "");
    let dir = tempfile::tempdir().unwrap();
    let bad = path_of(&dir.path().join("bad.json"));
    std::fs::write(&bad, "{}").unwrap();
    let run = osd_in(dir.path(), &import_args("midi-triggers-csv", &tr, &["--into", &bad]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn into_is_refused_for_a_format_other_than_the_two_csv() {
    let dir = tempfile::tempdir().unwrap();
    let (_k, tr) = with_csv(&sample());
    let json = path_of(&dir.path().join("d.json"));
    assert_eq!(osd(&import_args("midi-triggers-csv", &tr, &["-o", &json])).code, 0);
    let run = osd(&import_args("midnam", &tr, &["--into", &json]));
    assert!(run.code == 1 || run.code == 2, "{}: {:?}", run.code, run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn into_still_needs_the_source_and_the_license() {
    let dir = tempfile::tempdir().unwrap();
    let (_k, tr) = with_csv(&sample());
    let json = path_of(&dir.path().join("d.json"));
    assert_eq!(osd(&import_args("midi-triggers-csv", &tr, &["-o", &json])).code, 0);
    let run = osd(&["import", "midi-triggers-csv", &tr, "--into", &json, "--source-url", "https://example.org/x"]);
    assert_eq!(run.code, 2, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn the_help_and_an_unknown_format_name_list_midi_triggers_csv() {
    let (_dir, file) = with_csv(&sample());
    for word in ["import", "export"] {
        let run = osd(&[word, "no-such-format", &file]);
        assert_eq!(run.code, 2, "{word}: {:?}", run.stderr);
        assert!(run.stderr.contains("midi-triggers-csv"), "{word}: {:?}", run.stderr);
        let help = osd(&[word, "--help"]);
        assert!(help.stdout.contains("midi-triggers-csv") || help.stderr.contains("midi-triggers-csv"), "{word}");
    }
    assert!(osd(&["import", "--help"]).stdout.contains("--into"));
}
