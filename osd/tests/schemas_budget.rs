//! Docs/CADRE.md R13: the 0.5 schemas validate the converted SyntHub catalogue
//! in under 10 s (OSD-b91.2.9, criterion 3). A time budget: it is played by
//! name, and a failure under load is to be replayed.

mod common;

use common::{catalogue, Schemas};
use osd::Kind;
use std::time::{Duration, Instant};

#[test]
fn criterion_3_the_0_5_schemas_validate_the_converted_catalogue_in_under_10_s() {
    let Some(dir) = catalogue() else { return };
    let schemas = Schemas::load("0.5");
    let mut converted = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        for id in ids {
            // the two expected faults refuse to convert and are left out
            if let Ok(out) = osd::convert(&dir.text(&id), kind) {
                converted.push((kind, serde_json::from_str::<serde_json::Value>(&out).unwrap()));
            }
        }
    }
    assert!(converted.len() > 1000, "{} converted files", converted.len());
    let start = Instant::now();
    let bad = converted.iter().filter(|(k, doc)| schemas.check(*k, doc).is_some()).count();
    let took = start.elapsed();
    assert_eq!(bad, 0, "{bad} files not valid");
    assert!(took < Duration::from_secs(10), "validation took {took:?}");
}
