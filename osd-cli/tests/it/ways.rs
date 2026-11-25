//! `osd` reads, converts and resolves the descriptors of 0.7 whose controls have `ways`, and
//! writes the formats by their ways, with the exit codes it has always had.

use crate::common::osd;
use serde_json::{json, Value};

fn with_file(name: &str, doc: &Value) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(name).to_string_lossy().into_owned();
    std::fs::write(&file, doc.to_string()).unwrap();
    (dir, file)
}

fn device_07(entries: Value) -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "community-re", "url": "https://example.org/d", "license": "CC0-1.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "cc_params": {"entries": entries},
    })
}

#[test]
fn validate_accepts_a_device_of_0_7_whose_controls_have_ways() {
    let doc = device_07(json!([{"osc": "/filter/cutoff", "ways": [
        {"ctrlType": "cc", "ctrlIndex": [74]},
        {"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14}
    ]}]));
    let (_dir, file) = with_file("d.json", &doc);
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(run.stdout.lines().any(|l| l == format!("{file}: ok")), "{:?}", run.stdout);
}

#[test]
fn validate_refuses_the_flat_form_in_0_7_at_the_field_with_exit_1() {
    let (_dir, file) = with_file("d.json", &device_07(json!([{"osc": "/a", "cc": 74, "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}]}])));
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 1, "{:?}", run.stdout);
    assert!(run.stderr.lines().any(|l| l.starts_with(&format!("{file}: ")) && l.contains("UnknownField at \"/cc_params/entries/0/cc\"")), "{:?}", run.stderr);
}

#[test]
fn validate_refuses_a_message_claimed_twice_at_the_later_way_naming_the_earlier() {
    let doc = device_07(json!([
        {"osc": "/a", "ways": [{"ctrlType": "cc", "ctrlIndex": [7]}]},
        {"osc": "/b", "ways": [{"ctrlType": "cc", "ctrlIndex": [7]}]}
    ]));
    let (_dir, file) = with_file("d.json", &doc);
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 1);
    let line = run.stderr.lines().find(|l| l.contains("BadValue at \"/cc_params/entries/1/ways/0\"")).unwrap_or_else(|| panic!("{:?}", run.stderr));
    assert!(line.contains("/cc_params/entries/0/ways/0"), "{line}");
}

#[test]
fn convert_to_0_7_writes_the_ways_of_a_0_6_control_as_osd_convert_to_renders_them() {
    let doc = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"entries": [{"osc": "/filter/cutoff", "cc": 74, "range": [0, 127], "transmit": "absolute", "recognize": "both",
            "also": [{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "range": [0, 16383]}]}]}
    });
    let (_dir, file) = with_file("d.json", &doc);
    let run = osd(&["convert", "--to", "0.7", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, osd::convert_to(&doc.to_string(), osd::Kind::Device, osd::Version::V0_7).unwrap());
    let out: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(
        out["cc_params"]["entries"][0]["ways"],
        json!([
            {"ctrlType": "cc", "ctrlIndex": [74], "range": [0, 127], "transmit": "absolute", "recognize": "both"},
            {"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14, "range": [0, 16383], "transmit": "absolute", "recognize": "both"}
        ])
    );
    assert!(out["cc_params"]["entries"][0].get("also").is_none());
}

#[test]
fn convert_leaves_a_device_of_0_7_with_ways_as_it_is() {
    let doc = device_07(json!([{"osc": "/a", "default": 64, "scale": [0, 127], "ways": [
        {"ctrlType": "cc", "ctrlIndex": [74]},
        {"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14}
    ]}]));
    let (_dir, file) = with_file("d.json", &doc);
    let run = osd(&["convert", "--to", "0.7", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let out: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(out["cc_params"], doc["cc_params"]);
}

#[test]
fn resolve_gives_the_ways_of_a_control_of_0_7_and_converts_a_0_6_file_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("acme");
    std::fs::create_dir_all(&root).unwrap();
    let a = json!({"$schema": "https://w3id.org/osd/0.7/device.schema.json", "device": {"name": "S", "osc_prefix": "/s"}, "midi_of": "acme/b"});
    let b = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "S", "osc_prefix": "/s"},
        "cc_params": {"entries": [{"osc": "/x", "cc": 1}]}
    });
    std::fs::write(root.join("a.json"), a.to_string()).unwrap();
    std::fs::write(root.join("b.json"), b.to_string()).unwrap();
    let folder = dir.path().to_string_lossy().into_owned();
    let run = osd(&["resolve", "acme/a", "--catalogue", &folder]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let out: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(out["cc_params"]["entries"][0]["ways"], json!([{"ctrlType": "cc", "ctrlIndex": [1]}]));
}

#[test]
fn export_midnam_writes_the_first_way_it_carries_and_names_the_others_on_the_error_output() {
    let doc = device_07(json!([{"osc": "/filter/cutoff", "ways": [
        {"ctrlType": "note", "ctrlIndex": [36]},
        {"ctrlType": "cc", "ctrlIndex": [74]}
    ]}]));
    let (_dir, file) = with_file("d.json", &doc);
    let run = osd(&["export", "midnam", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(run.stdout.contains("Number=\"74\""), "{}", run.stdout);
    assert!(run.stderr.lines().any(|l| l.starts_with("/cc_params/entries/0/ways/0: ")), "{:?}", run.stderr);
    assert!(!run.stderr.lines().any(|l| l.starts_with("/cc_params/entries/0/ways/1: ")), "{:?}", run.stderr);
}

#[test]
fn import_midnam_writes_a_control_as_one_way() {
    let sample = r#"<?xml version="1.0" encoding="UTF-8"?>
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
        <Values Min="0" Max="127"/>
      </Control>
    </ControlNameList>
  </MasterDeviceNames>
</MIDINameDocument>
"#;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("d.midnam").to_string_lossy().into_owned();
    std::fs::write(&file, sample).unwrap();
    let run = osd(&["import", "midnam", &file, "--source-url", "https://example.org/names", "--source-author", "A maker", "--license", "CC-BY-4.0"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let out: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(out["cc_params"]["entries"][0]["ways"][0]["ctrlType"], "cc");
    assert_eq!(out["cc_params"]["entries"][0]["ways"][0]["ctrlIndex"], json!([74]));
}
