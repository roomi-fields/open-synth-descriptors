//! `osd import --into` on a base before 0.7: the base is converted to 0.7, and a base that does
//! not convert is exit 1, its fault on the error output under its own file name, no descriptor
//! written.

use crate::common::osd;
use serde_json::{json, Value};

const HEADER: &str = "manufacturer,device,section,parameter_name,parameter_description,cc_msb,cc_lsb,cc_min_value,cc_max_value,cc_default_value,nrpn_msb,nrpn_lsb,nrpn_min_value,nrpn_max_value,nrpn_default_value,orientation,notes,usage";

fn controllers() -> String {
    format!("{HEADER}\nLofty,Trundler,Filter,Resonance,,74,,0,127,,,,,,,,,\n")
}

/// A directory with the partner's file and a base of 0.4 whose one source has `license`.
struct Files {
    dir: tempfile::TempDir,
    csv: String,
    base: String,
}

fn files(csv_text: &str, license: &str) -> Files {
    let dir = tempfile::tempdir().unwrap();
    let path = |name: &str| dir.path().join(name).to_string_lossy().into_owned();
    let (csv, base) = (path("partner.csv"), path("base.device.json"));
    std::fs::write(&csv, csv_text).unwrap();
    let doc = json!({
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "_sources": [{"type": "vendor", "url": "https://example.org/base", "license": license}],
    });
    std::fs::write(&base, doc.to_string()).unwrap();
    Files { dir, csv, base }
}

fn import_args<'a>(f: &'a Files, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["import", "midi-csv", &f.csv, "--source-url", "https://example.org/partner", "--license", "CC0-1.0", "--into", &f.base];
    args.extend_from_slice(extra);
    args
}

#[test]
fn import_into_a_base_of_0_4_prints_a_descriptor_of_0_7_the_reader_reads() {
    let f = files(&controllers(), "none");
    let run = osd(&import_args(&f, &[]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(osd::read(&run.stdout, osd::Kind::Device).is_ok(), "{}", run.stdout);
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(doc["$schema"], "https://w3id.org/osd/0.7/device.schema.json");
    assert_eq!(doc["sources"][0]["type"], "vendor-doc");
    assert_eq!(doc["sources"][0]["license"], "NONE");
    assert_eq!(doc["sources"][1]["url"], "https://example.org/partner");
}

#[test]
fn import_into_a_base_that_does_not_convert_is_exit_1_naming_the_base_file_and_prints_no_descriptor() {
    let f = files(&controllers(), "proprietary");
    let run = osd(&import_args(&f, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    let line = run.stderr.lines().find(|l| l.contains("/_sources/0/license")).unwrap_or_else(|| panic!("{:?}", run.stderr));
    assert!(line.starts_with(&format!("{}: ", f.base)), "{line:?}");
    assert!(line.contains("NotConvertible"), "{line:?}");
    assert!(!run.stderr.contains(&f.csv), "the partner's file is not at fault: {:?}", run.stderr);
}

#[test]
fn import_into_a_base_that_does_not_convert_writes_nothing_in_the_file_named_by_o() {
    let f = files(&controllers(), "proprietary");
    let out = f.dir.path().join("out.json").to_string_lossy().into_owned();
    let run = osd(&import_args(&f, &["-o", &out]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.starts_with(&format!("{}: ", f.base)), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(!std::path::Path::new(&out).exists());

    std::fs::write(&out, "keep me").unwrap();
    let forced = osd(&import_args(&f, &["-o", &out, "--force"]));
    assert_eq!(forced.code, 1, "{:?}", forced.stderr);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "keep me");
}

#[test]
fn a_fault_of_the_partner_s_file_keeps_its_name_when_added_to_a_base_of_0_4() {
    let f = files(&format!("{HEADER}\nLofty,Trundler,only three\n"), "none");
    let run = osd(&import_args(&f, &[]));
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert!(run.stderr.starts_with(&format!("{}: ", f.csv)), "{:?}", run.stderr);
    assert!(run.stderr.contains("line 2"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}
