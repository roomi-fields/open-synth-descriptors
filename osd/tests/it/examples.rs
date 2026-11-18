//! The repository's examples, one per kind: read strictly, written back
//! unchanged, and valid against the published schemas once written.

use crate::common;

use osd::Kind;
use serde_json::Value;

const EXAMPLES: [(&str, Kind); 5] = [
    ("examples/devices/test_synth.json", Kind::Device),
    ("examples/warm-pad.sound.json", Kind::Sound),
    ("examples/minilogue.fingerprint.json", Kind::Fingerprint),
    ("examples/unknown-device.fingerprint.json", Kind::Fingerprint),
    ("examples/tribe-0.3-beta.firmware.json", Kind::Firmware),
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
    let text = std::fs::read_to_string(common::repo().join("docs/vocabulary.md")).unwrap();
    let roles = roles_written_in(&text);
    assert!(roles.len() >= 3, "{} roles in docs/vocabulary.md", roles.len());
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
    let guide = std::fs::read_to_string(common::repo().join("docs/vocabulary.md")).unwrap();
    for needle in ["paramPath", "M2-117-UM", "/Filter/Envelope/AttackTime", "/Oscillator/2/Pitch", "stands_under", "at_top", "shared_by"] {
        assert!(guide.contains(needle), "docs/vocabulary.md does not say {needle}");
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
