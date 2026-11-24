//! `osd import midnam` and `osd export midnam`: a MIDNAM file from the command and back, its
//! losses on the error output, one per line.

use crate::common::{osd, repo};
use serde_json::Value;

const CREDIT: [&str; 6] = ["--source-url", "https://example.org/names", "--source-author", "A maker", "--license", "CC-BY-4.0"];

const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE MIDINameDocument PUBLIC "-//MIDI Manufacturers Association//DTD MIDINameDocument 1.0//EN" "http://www.midi.org/dtds/MIDINameDocument10.dtd">
<MIDINameDocument>
  <Author>Someone</Author>
  <MasterDeviceNames>
    <Manufacturer>Lofty</Manufacturer>
    <Model>Trundler</Model>
    <CustomDeviceMode Name="Default">
      <ChannelNameSetAssignments>
        <ChannelNameSetAssign Channel="1" NameSet="Names"/>
      </ChannelNameSetAssignments>
    </CustomDeviceMode>
    <ChannelNameSet Name="Names">
      <AvailableForChannels>
        <AvailableChannel Channel="1" Available="true"/>
      </AvailableForChannels>
      <UsesControlNameList Name="Controls"/>
      <PatchBank Name="Programs">
        <PatchNameList/>
      </PatchBank>
    </ChannelNameSet>
    <ControlNameList Name="Controls">
      <Control Type="7bit" Number="74" Name="Cutoff">
        <Values Min="0" Max="127" Units="Hz">
          <ValueNameList>
            <Value Number="0" Name="Closed"/>
          </ValueNameList>
        </Values>
      </Control>
    </ControlNameList>
  </MasterDeviceNames>
</MIDINameDocument>
"#;

fn with_file(name: &str, text: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(name).to_string_lossy().into_owned();
    std::fs::write(&file, text).unwrap();
    (dir, file)
}

fn import_args<'a>(file: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["import", "midnam", file];
    args.extend_from_slice(&CREDIT);
    args.extend_from_slice(extra);
    args
}

#[test]
fn import_prints_the_descriptor_read_strictly_with_the_control_and_the_credit() {
    let (_dir, file) = with_file("d.midnam", SAMPLE);
    let run = osd(&import_args(&file, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(doc["device"]["name"], "Trundler");
    assert_eq!(doc["cc_params"]["entries"][0]["cc"], 74);
    assert_eq!(doc["_sources"][0]["license"], "CC-BY-4.0");
    assert_eq!(doc["_sources"][0]["author"], "A maker");
}

#[test]
fn import_prints_each_loss_on_the_error_output_one_per_line_by_its_line_in_the_file() {
    let (_dir, file) = with_file("d.midnam", SAMPLE);
    let run = osd(&import_args(&file, &[]));
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert!(!lines.is_empty(), "the units are lost");
    assert!(lines.iter().all(|l| l.starts_with("line ") && l.contains(": ")), "{lines:?}");
    let units = SAMPLE.lines().position(|l| l.contains("<Values ")).unwrap() + 1;
    assert!(lines.iter().any(|l| l.starts_with(&format!("line {units}: "))), "{lines:?}");
    assert!(!run.stdout.contains("line "));
}

#[test]
fn import_refuses_without_a_license_with_exit_2_and_a_file_not_valid_by_the_dtd_with_exit_1() {
    let (_dir, file) = with_file("d.midnam", SAMPLE);
    let run = osd(&["import", "midnam", &file, "--source-url", "https://example.org/x"]);
    assert_eq!(run.code, 2, "{:?}", run.stderr);
    assert!(run.stderr.contains("license"));
    assert_eq!(run.stdout, "");
    let (_d, bad) = with_file("bad.midnam", &SAMPLE.replace("<Author>Someone</Author>", "<Bogus/>"));
    let run = osd(&import_args(&bad, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.contains("line 4"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn export_writes_a_midnam_document_of_the_descriptor_and_prints_its_losses_on_the_error_output() {
    let file = repo().join("examples/0.5/synth.device.json").to_string_lossy().into_owned();
    let run = osd(&["export", "midnam", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(run.stdout.contains("<MIDINameDocument>"), "{}", run.stdout);
    assert!(run.stdout.contains("DTD MIDINameDocument 1.0"), "{}", run.stdout);
    assert!(run.stdout.contains("Filter Cutoff"), "{}", run.stdout);
    assert!(!run.stderr.is_empty());
    assert!(run.stderr.lines().all(|l| l.starts_with('/') && l.contains(": ")), "{:?}", run.stderr);
}

#[test]
fn export_of_what_import_wrote_gives_back_the_control() {
    let (dir, file) = with_file("d.midnam", SAMPLE);
    let json = dir.path().join("d.json").to_string_lossy().into_owned();
    assert_eq!(osd(&import_args(&file, &["-o", &json])).code, 0);
    let run = osd(&["export", "midnam", &json]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(run.stdout.contains("Number=\"74\"") && run.stdout.contains("Name=\"Cutoff\""), "{}", run.stdout);
}

#[test]
fn export_writes_the_file_named_by_o_and_refuses_an_existing_one_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let file = repo().join("examples/0.5/synth.device.json").to_string_lossy().into_owned();
    let out = dir.path().join("out.midnam").to_string_lossy().into_owned();
    let run = osd(&["export", "midnam", &file, "-o", &out]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(std::fs::read_to_string(&out).unwrap().contains("<MIDINameDocument>"));
    assert_eq!(osd(&["export", "midnam", &file, "-o", &out]).code, 2);
    assert_eq!(osd(&["export", "midnam", &file, "-o", &out, "--force"]).code, 0);
}

#[test]
fn the_unknown_format_message_and_the_helps_list_midnam() {
    let (_dir, file) = with_file("d.midnam", SAMPLE);
    for word in ["import", "export"] {
        let run = osd(&[word, "no-such-format", &file]);
        assert_eq!(run.code, 2, "{word}: {:?}", run.stderr);
        assert!(run.stderr.contains("midnam"), "{word}: {:?}", run.stderr);
        let help = osd(&[word, "--help"]);
        assert!(help.stdout.contains("midnam") || help.stderr.contains("midnam"), "{word}");
    }
    assert!(osd(&["--help"]).stdout.contains("midnam"));
}
