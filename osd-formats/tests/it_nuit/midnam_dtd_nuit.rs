//! The exports of every device of the cases are valid by the DTD of the MIDI Association, as an
//! external validator (`xmllint --dtdvalid`) reads it, when the program is present; the test
//! says so when it is not.

use crate::common::credited_devices;
use crate::common::midnam::DTD;
use osd_formats::{export, Format};
use std::process::Command;

#[test]
fn the_exports_of_the_cases_are_valid_by_the_dtd_for_xmllint() {
    if Command::new("xmllint").arg("--version").output().is_err() {
        eprintln!("xmllint is not installed: the exports are checked by the verifier of the crate only, not by the DTD itself");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let devices = credited_devices();
    assert!(!devices.is_empty());
    for (name, device) in devices {
        let text = export(&device, Format::Midnam).unwrap_or_else(|f| panic!("{name}: {f}")).value;
        let file = dir.path().join("export.midnam");
        std::fs::write(&file, text).unwrap();
        let run = Command::new("xmllint").arg("--noout").arg("--dtdvalid").arg(DTD).arg(&file).output().unwrap();
        assert!(run.status.success(), "{name}: {}", String::from_utf8_lossy(&run.stderr));
    }
}
