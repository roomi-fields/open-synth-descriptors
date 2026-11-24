//! `osd resolve <id> --catalogue <folder>`: the complete descriptor of a device on the standard
//! output; a fault of the resolution is reported as `osd validate` reports it.

use crate::common::osd;
use serde_json::{json, Value};
use std::path::Path;

const DEVICE: &str = "https://w3id.org/osd/0.6/device.schema.json";
const PROFILE: &str = "https://w3id.org/osd/0.6/profile.schema.json";

fn put(root: &Path, id: &str, doc: &Value) {
    let path = root.join(format!("{id}.json"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, doc.to_string()).unwrap();
}

fn catalogue_with_profile() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        "acme/synth",
        &json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"}, "identity": {"profile": "acme"}}),
    );
    put(
        dir.path(),
        "acme/_profile",
        &json!({"$schema": PROFILE, "identity": {"reply": [240, 126, "any", 6, 2], "script": "return reply[5] == 2"}}),
    );
    dir
}

#[test]
fn resolve_writes_the_complete_descriptor_of_a_device_on_the_standard_output() {
    let dir = catalogue_with_profile();
    let folder = dir.path().to_string_lossy().into_owned();
    let run = osd(&["resolve", "acme/synth", "--catalogue", &folder]);
    assert_eq!((run.code, run.stderr.as_str()), (0, ""));
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert!(doc["identity"].get("profile").is_none());
    assert_eq!(doc["capabilities"], json!(["lua"]));
    assert!(osd::read(&run.stdout, osd::Kind::Device).is_ok());
}

#[test]
fn resolve_writes_what_the_library_gives_of_the_resolution() {
    let dir = catalogue_with_profile();
    let folder = dir.path().to_string_lossy().into_owned();
    let run = osd(&["resolve", "acme/synth", "--catalogue", &folder]);
    struct Folder(std::path::PathBuf);
    impl osd::Files for Folder {
        fn get(&self, id: &str) -> Option<String> {
            std::fs::read_to_string(self.0.join(format!("{id}.json"))).ok()
        }
    }
    let r = osd::resolve("acme/synth", &Folder(dir.path().to_path_buf())).unwrap();
    let want: Value = serde_json::from_str(&osd::write(&osd::complete_descriptor(&r))).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&run.stdout).unwrap(), want);
}

#[test]
fn resolve_reports_a_fault_with_the_text_validate_gives_for_the_same_fault() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "acme/synth", &json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"}, "bogus": 1}));
    let folder = dir.path().to_string_lossy().into_owned();
    let path = dir.path().join("acme/synth.json").to_string_lossy().into_owned();
    let validate = osd(&["validate", &path]);
    assert_eq!(validate.code, 1);
    let run = osd(&["resolve", "acme/synth", "--catalogue", &folder]);
    assert_eq!((run.code, run.stdout.as_str()), (1, ""));
    assert_eq!(run.stderr, validate.stderr);
}

#[test]
fn resolve_exits_1_for_a_link_no_file_answers() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "acme/a", &json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"}, "midi_of": "acme/b"}));
    let folder = dir.path().to_string_lossy().into_owned();
    let run = osd(&["resolve", "acme/a", "--catalogue", &folder]);
    assert_eq!((run.code, run.stdout.as_str()), (1, ""));
    assert!(run.stderr.contains("LinkMissing at"), "{:?}", run.stderr);
}

#[test]
fn resolve_exits_2_for_an_unknown_option_a_missing_folder_or_a_missing_catalogue_option() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().to_string_lossy().into_owned();
    let missing = dir.path().join("none").to_string_lossy().into_owned();
    for args in [
        vec!["resolve", "acme/synth", "--catalogue", &folder, "--frobnicate"],
        vec!["resolve", "acme/synth", "--catalogue", &missing],
        vec!["resolve", "acme/synth"],
        vec!["resolve", "--catalogue", &folder],
    ] {
        let run = osd(&args);
        assert_eq!((run.code, run.stdout.as_str()), (2, ""), "{args:?}: {:?}", run.stderr);
    }
}
