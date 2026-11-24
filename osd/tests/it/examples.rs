//! The repository's examples, one per kind: read strictly, written back
//! unchanged, and valid against the published schemas once written.

use crate::common;

use osd::Kind;
use serde_json::Value;

const EXAMPLES: [(&str, Kind); 5] = [
    ("examples/0.4/test_synth.json", Kind::Device),
    ("examples/0.4/warm-pad.sound.json", Kind::Sound),
    ("examples/0.4/minilogue.fingerprint.json", Kind::Fingerprint),
    ("examples/0.4/unknown-device.fingerprint.json", Kind::Fingerprint),
    ("examples/0.4/tribe-0.3-beta.firmware.json", Kind::Firmware),
];

const EXAMPLES_0_5: [(&str, Kind); 5] = [
    ("examples/0.5/synth.device.json", Kind::Device),
    ("examples/0.5/acme.profile.json", Kind::Profile),
    ("examples/0.5/warm-pad.sound.json", Kind::Sound),
    ("examples/0.5/minilogue.fingerprint.json", Kind::Fingerprint),
    ("examples/0.5/tribe-0.3-beta.firmware.json", Kind::Firmware),
];

#[test]
fn criterion_5_every_0_5_example_reads_writes_back_and_validates() {
    let schemas = common::Schemas::load("0.5");
    for (path, kind) in EXAMPLES_0_5 {
        let text = std::fs::read_to_string(common::repo().join(path)).unwrap();
        let read = osd::read(&text, kind).unwrap_or_else(|f| panic!("{path}: {f}"));
        assert_eq!(read.kind(), kind, "{path}");
        let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(&text).unwrap(), "{path}");
        assert_eq!(back["$schema"].as_str().unwrap().split('/').nth(2), Some("w3id.org"), "{path}");
        assert_eq!(schemas.check(kind, &back), None, "{path}");
    }
}

fn json_files_under(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            json_files_under(&path, out);
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
}

#[test]
fn every_example_sits_in_the_folder_of_its_version_and_names_that_version_in_its_schema() {
    let root = common::repo().join("examples");
    let mut files = Vec::new();
    json_files_under(&root, &mut files);
    assert!(!files.is_empty());
    for path in files {
        let rel = path.strip_prefix(&root).unwrap().display().to_string();
        let mut parts = rel.split('/');
        let folder = parts.next().unwrap();
        assert!(parts.next().is_some(), "{rel}: an example sits in the folder of its version, not at the root");
        assert!(["0.4", "0.5", "0.6"].contains(&folder), "{rel}: unknown version folder {folder}");
        if rel == "0.4/test_synth.json" {
            // The 0.4 device schema refuses `$schema`: the folder says the version.
            continue;
        }
        let value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let schema = value.get("$schema").and_then(Value::as_str).unwrap_or_else(|| panic!("{rel}: no $schema"));
        assert!(schema.contains(&format!("/osd/{folder}/")), "{rel}: $schema {schema} is not of {folder}");
    }
}

#[test]
fn examples_0_4_hold_the_device_sound_firmware_and_two_fingerprints_of_that_version() {
    let dir = common::repo().join("examples/0.4");
    let mut names: Vec<String> =
        std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "minilogue.fingerprint.json",
            "test_synth.json",
            "tribe-0.3-beta.firmware.json",
            "unknown-device.fingerprint.json",
            "warm-pad.sound.json"
        ]
    );
}

#[test]
fn examples_0_4_validate_against_the_0_4_schemas() {
    let schemas = common::Schemas::load("0.4");
    for (path, kind) in EXAMPLES {
        let value: Value = serde_json::from_str(&std::fs::read_to_string(common::repo().join(path)).unwrap()).unwrap();
        assert_eq!(schemas.check(kind, &value), None, "{path}");
    }
}

/// The kind of a 0.6 example, by the name of its file: `<name>.<kind>.json`.
fn kind_of_example(file: &str) -> Option<Kind> {
    let kind = file.strip_suffix(".json")?.rsplit('.').next()?;
    match kind {
        "device" => Some(Kind::Device),
        "firmware" => Some(Kind::Firmware),
        "sound" => Some(Kind::Sound),
        "fingerprint" => Some(Kind::Fingerprint),
        "assembly" => Some(Kind::Assembly),
        _ => None,
    }
}

#[test]
fn examples_0_6_hold_one_valid_example_of_each_kind_read_strictly_and_written_back_unchanged() {
    let schemas = common::Schemas::load("0.6");
    let dir = common::repo().join("examples/0.6");
    let mut seen: Vec<Kind> = Vec::new();
    let mut unread: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(kind) = kind_of_example(&file) else {
            unread.push(file);
            continue;
        };
        let text = std::fs::read_to_string(&path).unwrap();
        let read = osd::read(&text, kind).unwrap_or_else(|f| panic!("{file}: {f}"));
        assert_eq!(read.kind(), kind, "{file}");
        let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(&text).unwrap(), "{file}");
        assert!(back["$schema"].as_str().unwrap().starts_with("https://w3id.org/osd/0.6/"), "{file}");
        assert_eq!(schemas.check(kind, &back), None, "{file}");
        seen.push(kind);
    }
    assert!(unread.is_empty(), "files of examples/0.6 that name no kind: {unread:?}");
    for (kind, word) in [
        (Kind::Device, "device"),
        (Kind::Firmware, "firmware"),
        (Kind::Sound, "sound"),
        (Kind::Fingerprint, "fingerprint"),
        (Kind::Assembly, "assembly"),
    ] {
        assert!(seen.contains(&kind), "examples/0.6 holds no {word} example");
    }
}

#[test]
fn the_0_6_device_example_writes_its_source_comment_as_notes() {
    let text = std::fs::read_to_string(common::repo().join("examples/0.6/minilogue.device.json")).unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    for source in doc["_sources"].as_array().unwrap() {
        assert!(source.get("note").is_none(), "a source writes `note`, the schema's word is `notes`");
        assert!(source["notes"].is_string(), "a source carries `notes`");
    }
}

/// The device examples of 0.6, `examples/0.6/*.device.json`.
fn devices_0_6() -> Vec<(String, String)> {
    let dir = common::repo().join("examples/0.6");
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut out: Vec<(String, String)> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().ends_with(".device.json")))
        .map(|p| (p.display().to_string(), std::fs::read_to_string(&p).unwrap()))
        .collect();
    out.sort();
    out
}

#[test]
fn a_0_6_device_example_gives_ten_roles_of_a_real_device_with_its_two_envelopes() {
    let schemas = common::Schemas::load("0.6");
    let examples = devices_0_6();
    assert!(!examples.is_empty(), "no examples/0.6/*.device.json");
    let mut best = 0;
    let mut with_two_envelopes = false;
    for (path, text) in &examples {
        let d = osd::read(text, Kind::Device).unwrap_or_else(|f| panic!("{path}: {f}"));
        let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(text).unwrap(), "{path}");
        assert_eq!(schemas.check(Kind::Device, &back), None, "{path}");
        let roles = osd::roles(&d);
        best = best.max(roles.len());
        assert!(roles.iter().all(|r| r.meaning == osd::RoleMeaning::Known), "{path}: a role that is not known");
        for r in &roles {
            assert_eq!(osd::role_from_param_path(&r.param_path, "1").unwrap(), r.role, "{path}: {}", r.param_path);
        }
        let paths: Vec<&str> = roles.iter().map(|r| r.param_path.as_str()).collect();
        with_two_envelopes |= roles.len() >= 10
            && paths.iter().any(|p| p.starts_with("/Filter/Envelope/"))
            && paths.iter().any(|p| p.starts_with("/Amplifier/Envelope/"));
    }
    assert!(best >= 10, "{best} roles");
    assert!(with_two_envelopes, "a device of ten roles that holds a filter envelope and an amplifier envelope");
}

/// The roles written in a document, each read from the `"role":` it follows.
fn roles_written_in(text: &str) -> Vec<Value> {
    text.match_indices("\"role\":")
        .filter_map(|(at, key)| serde_json::Deserializer::from_str(text[at + key.len()..].trim_start()).into_iter::<Value>().next()?.ok())
        .collect()
}

#[test]
fn each_role_written_in_the_vocabulary_guide_is_read_without_fault_and_given_as_a_param_path() {
    let text = std::fs::read_to_string(common::repo().join("spec/vocabulary/1.md")).unwrap();
    let roles = roles_written_in(&text);
    assert!(roles.len() >= 3, "{} roles in spec/vocabulary/1.md", roles.len());
    for (n, role) in roles.iter().enumerate() {
        let doc = serde_json::json!({
            "$schema": "https://w3id.org/osd/0.6/device.schema.json",
            "role_vocabulary": "1",
            "device": {"name": "T", "osc_prefix": "/t"},
            "cc_params": {"entries": [{"osc": "/a", "cc": 1, "role": role}]}
        });
        let d = osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("role {n} {role}: {f}"));
        assert_eq!(osd::roles(&d).len(), 1, "{role}");
    }
}

#[test]
fn the_documents_state_the_correspondence_between_a_role_and_the_param_path_of_property_exchange() {
    let guide = std::fs::read_to_string(common::repo().join("spec/vocabulary/1.md")).unwrap();
    for needle in ["paramPath", "M2-117-UM", "/Filter/Envelope/AttackTime", "/Oscillator/2/Pitch", "stands_under", "at_top", "shared_by"] {
        assert!(guide.contains(needle), "spec/vocabulary/1.md does not say {needle}");
    }
    assert!(!guide.contains("\"role\": { \"part\"") && !guide.contains("\"role\": {\"part\""), "the guide still writes the flat form");
    let interface = std::fs::read_to_string(common::repo().join("osd/docs/INTERFACE.md")).unwrap();
    assert!(interface.contains("paramPath"), "osd/docs/INTERFACE.md does not say paramPath");
}

#[test]
fn every_example_reads_writes_back_and_validates() {
    let schemas = common::Schemas::load("0.4");
    for (path, kind) in EXAMPLES {
        let text = std::fs::read_to_string(common::repo().join(path)).unwrap();
        let read = osd::read(&text, kind).unwrap_or_else(|f| panic!("{path}: {f}"));
        assert_eq!(read.kind(), kind, "{path}");
        let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(&text).unwrap(), "{path}");
        assert_eq!(schemas.check(kind, &back), None, "{path}");
    }
}
