//! The repository's examples, one per kind: read strictly, written back
//! unchanged, and valid against the published schemas once written.

mod common;

use osd::Kind;
use serde_json::Value;

const EXAMPLES: [(&str, Kind); 5] = [
    ("examples/devices/test_synth.json", Kind::Device),
    ("examples/warm-pad.sound.json", Kind::Sound),
    ("examples/minilogue.fingerprint.json", Kind::Fingerprint),
    ("examples/unknown-device.fingerprint.json", Kind::Fingerprint),
    ("examples/tribe-0.3-beta.firmware.json", Kind::Firmware),
];

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
