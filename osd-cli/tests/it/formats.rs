//! `osd import` and `osd export`: the pencilresearch/midi CSV from the command, its losses on
//! the error output.

use crate::common::{osd, osd_in, repo};
use serde_json::Value;

const HEADER: &str = "manufacturer,device,section,parameter_name,parameter_description,cc_msb,cc_lsb,cc_min_value,cc_max_value,cc_default_value,nrpn_msb,nrpn_lsb,nrpn_min_value,nrpn_max_value,nrpn_default_value,orientation,notes,usage";

/// A CSV with a CC row (with a default), a row with neither, and an NRPN row.
fn sample() -> String {
    format!(
        "{HEADER}\n\
         Lofty,Trundler,Oscillators,Glide switch,Enables glide,65,,0,127,0,,,,,,0-based,a note,0-63: Off\n\
         Lofty,Trundler,Misc,Sysex only,,,,,,,,,,,,,,\n\
         Lofty,Trundler,Filter,Resonance,,,,,,,1,2,0,16383,,,,\n"
    )
}

fn path_of(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

const CREDIT: [&str; 6] =
    ["--source-url", "https://github.com/pencilresearch/midi", "--source-author", "Pencilresearch", "--license", "CC-BY-SA-4.0"];

fn import_args<'a>(file: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["import", "midi-csv", file];
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
fn import_prints_the_descriptor_read_strictly_with_the_source_given_in_the_options() {
    let (_dir, file) = with_csv(&sample());
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    let source = &doc["_sources"][0];
    assert_eq!(source["url"], "https://github.com/pencilresearch/midi");
    assert_eq!(source["author"], "Pencilresearch");
    assert_eq!(source["license"], "CC-BY-SA-4.0");
    assert!(source["type"].is_string());
    assert_eq!(doc["cc_params"]["entries"].as_array().unwrap().len(), 2);
}

#[test]
fn import_prints_each_loss_on_the_error_output_one_per_line() {
    let (_dir, file) = with_csv(&sample());
    let run = osd(&import_args(&file, &[]));
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("line 2, column cc_default_value: "), "{:?}", lines[0]);
    assert!(lines[1].starts_with("line 3: "), "{:?}", lines[1]);
    assert!(!run.stdout.contains("line 3"));
}

#[test]
fn import_refuses_without_a_license_or_without_any_credit_with_exit_2() {
    let (_dir, file) = with_csv(&sample());
    let no_license = osd(&["import", "midi-csv", &file, "--source-url", "https://example.org/x"]);
    assert_eq!(no_license.code, 2, "{:?}", no_license.stderr);
    assert!(no_license.stderr.contains("license"));
    assert_eq!(no_license.stdout, "");
    let no_credit = osd(&["import", "midi-csv", &file, "--license", "CC0-1.0"]);
    assert_eq!(no_credit.code, 2, "{:?}", no_credit.stderr);
    assert_eq!(no_credit.stdout, "");
}

#[test]
fn import_takes_a_source_with_only_an_author() {
    let (_dir, file) = with_csv(&sample());
    let run = osd(&["import", "midi-csv", &file, "--source-author", "Someone", "--license", "CC0-1.0"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
}

#[test]
fn import_of_a_file_not_valid_in_its_format_is_exit_1_with_the_line() {
    let (_dir, file) = with_csv(&format!("{HEADER}\nLofty,Trundler,only three\n"));
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("line 2"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn import_of_a_missing_file_is_exit_2() {
    let run = osd(&import_args("/nonexistent/in.csv", &[]));
    assert_eq!(run.code, 2, "{:?}", run.stderr);
    assert!(run.stderr.contains("/nonexistent/in.csv"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn import_writes_the_file_named_by_o_prints_no_descriptor_and_refuses_an_existing_one_without_force() {
    let (dir, file) = with_csv(&sample());
    let out = path_of(&dir.path().join("out.json"));
    let run = osd(&import_args(&file, &["-o", &out]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(osd::read(&written, osd::Kind::Device).is_ok());
    assert!(run.stderr.contains("line 3"), "the losses stay on the error output: {:?}", run.stderr);

    std::fs::write(&out, "keep me").unwrap();
    let refused = osd(&import_args(&file, &["-o", &out]));
    assert_eq!(refused.code, 2);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "keep me");
    let forced = osd(&import_args(&file, &["-o", &out, "--force"]));
    assert_eq!(forced.code, 0, "{:?}", forced.stderr);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), written);
}

#[test]
fn export_writes_the_csv_of_a_descriptor_and_prints_its_losses_on_the_error_output() {
    let file = path_of(&repo().join("examples/0.5/synth.device.json"));
    let run = osd(&["export", "midi-csv", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(run.stdout.starts_with(&format!("{HEADER}\n")), "{}", run.stdout);
    assert!(run.stdout.lines().count() > 1);
    assert!(run.stderr.lines().any(|l| l.starts_with("/sysex: ")), "{:?}", run.stderr);
}

#[test]
fn export_of_what_import_wrote_gives_back_the_rows_of_the_cc_and_nrpn_controls() {
    let dir = tempfile::tempdir().unwrap();
    let (_keep, csv) = with_csv(&sample());
    let json = path_of(&dir.path().join("d.json"));
    assert_eq!(osd(&import_args(&csv, &["-o", &json])).code, 0);
    let run = osd(&["export", "midi-csv", &json]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[1].starts_with("Lofty,Trundler,Oscillators,Glide switch,Enables glide,65,"), "{:?}", lines[1]);
    assert!(lines[2].starts_with("Lofty,Trundler,Filter,Resonance,"), "{:?}", lines[2]);
}

#[test]
fn export_writes_the_file_named_by_o_and_refuses_an_existing_one_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let file = path_of(&repo().join("examples/0.5/synth.device.json"));
    let out = path_of(&dir.path().join("out.csv"));
    let run = osd(&["export", "midi-csv", &file, "-o", &out]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(std::fs::read_to_string(&out).unwrap().starts_with(HEADER));
    let refused = osd(&["export", "midi-csv", &file, "-o", &out]);
    assert_eq!(refused.code, 2);
    assert_eq!(osd(&["export", "midi-csv", &file, "-o", &out, "--force"]).code, 0);
}

#[test]
fn export_of_a_descriptor_without_sources_is_exit_1() {
    let dir = tempfile::tempdir().unwrap();
    let file = path_of(&dir.path().join("d.json"));
    std::fs::write(&file, r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}}"#).unwrap();
    let run = osd_in(dir.path(), &["export", "midi-csv", &file]);
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("_sources"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn export_of_a_descriptor_whose_link_no_file_answers_is_exit_1() {
    let dir = tempfile::tempdir().unwrap();
    let file = path_of(&dir.path().join("d.json"));
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json",
        "_sources": [{"type": "community-re", "url": "u", "license": "CC0-1.0"}],
        "midi_of": "acme/other", "device": {"name": "T", "osc_prefix": "/t"}}"#;
    std::fs::write(&file, text).unwrap();
    let run = osd_in(dir.path(), &["export", "midi-csv", &file]);
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("LinkMissing"), "{:?}", run.stderr);
}

#[test]
fn an_unknown_format_name_is_exit_2_and_the_help_lists_midi_csv() {
    let (_dir, file) = with_csv(&sample());
    for word in ["import", "export"] {
        let run = osd(&[word, "no-such-format", &file]);
        assert_eq!(run.code, 2, "{word}: {:?}", run.stderr);
        assert!(run.stderr.contains("midi-csv"), "{word}: {:?}", run.stderr);
        let help = osd(&[word, "--help"]);
        assert!(help.stdout.contains("midi-csv") || help.stderr.contains("midi-csv"), "{word}");
    }
    let general = osd(&["--help"]);
    assert!(general.stdout.contains("import") && general.stdout.contains("export") && general.stdout.contains("midi-csv"));
}
