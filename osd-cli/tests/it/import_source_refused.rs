//! `osd import` of a source the 0.7 reader refuses: no descriptor written, the place and the rule
//! on the error output, exit 2.

use crate::common::{osd, Run};

const CC_HEADER: &str = "manufacturer,device,section,parameter_name,parameter_description,cc_msb,cc_lsb,cc_min_value,cc_max_value,cc_default_value,nrpn_msb,nrpn_lsb,nrpn_min_value,nrpn_max_value,nrpn_default_value,orientation,notes,usage";
const TRIGGER_HEADER: &str = "manufacturer,device,section,sound_name,sound_description,midi_channel,note_number_min_value,note_number_max_value,note_number_default_value,comments,velocity_usage";
const BASE: &str = r#"{"$schema": "https://w3id.org/osd/0.7/device.schema.json", "sources": [{"type": "vendor-doc", "url": "u", "license": "CC0-1.0"}],
    "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}}"#;
const BAD_LICENSES: [&str; 2] = ["cc by", "None"];

/// A folder with a file of each CSV format and a base.
struct Files {
    dir: tempfile::TempDir,
}

impl Files {
    fn new() -> Files {
        let files = Files { dir: tempfile::tempdir().unwrap() };
        std::fs::write(files.path("cc.csv"), format!("{CC_HEADER}\nLofty,Trundler,Filter,Resonance,,74,,0,127,,,,,,,,,\n")).unwrap();
        std::fs::write(files.path("triggers.csv"), format!("{TRIGGER_HEADER}\nLofty,Trundler,Drums,Kick,,10,36,36,,,\n")).unwrap();
        std::fs::write(files.path("base.device.json"), BASE).unwrap();
        files
    }

    fn path(&self, name: &str) -> String {
        self.dir.path().join(name).to_string_lossy().into_owned()
    }

    fn import(&self, format: &str, file: &str, license: &str, extra: &[&str]) -> Run {
        let file = self.path(file);
        let mut args = vec!["import", format, &file, "--source-url", "https://example.org/partner", "--license", license];
        args.extend_from_slice(extra);
        osd(&args)
    }
}

const FORMATS: [(&str, &str); 2] = [("midi-csv", "cc.csv"), ("midi-triggers-csv", "triggers.csv")];

/// The reader's rule for a license of this form.
fn license_rule() -> String {
    let text = BASE.replace("CC0-1.0", "cc by");
    osd::read(&text, osd::Kind::Device).expect_err("the reader refuses the license").rule
}

fn assert_refused(run: &Run, context: &str) {
    assert_eq!(run.code, 2, "{context}: {:?}", run.stderr);
    assert_eq!(run.stdout, "", "{context}: no descriptor is written");
    assert!(run.stderr.contains("source/license"), "{context}: the place: {:?}", run.stderr);
    assert!(run.stderr.contains(&license_rule()), "{context}: the rule: {:?}", run.stderr);
}

#[test]
fn import_with_a_license_the_reader_refuses_is_exit_2_naming_the_place_and_the_rule() {
    let files = Files::new();
    for (format, file) in FORMATS {
        for license in BAD_LICENSES {
            assert_refused(&files.import(format, file, license, &[]), &format!("{format} {license}"));
        }
    }
}

#[test]
fn import_with_a_license_id_in_lower_case_writes_it_as_given() {
    let files = Files::new();
    let base = files.path("base.device.json");
    for license in ["curl", "mit"] {
        for (format, file) in FORMATS {
            for extra in [vec![], vec!["--into", base.as_str()]] {
                let run = files.import(format, file, license, &extra);
                let context = format!("{format} {license} {extra:?}");
                assert_eq!(run.code, 0, "{context}: {:?}", run.stderr);
                let Ok(osd::Descriptor::Device(device)) = osd::read(&run.stdout, osd::Kind::Device) else {
                    panic!("{context}: no device is written: {:?}", run.stdout)
                };
                let last = device.sources.as_ref().and_then(|s| s.last()).expect("a source");
                assert_eq!(last.license.as_deref(), Some(license), "{context}");
            }
        }
    }
}

#[test]
fn import_into_with_a_license_the_reader_refuses_is_exit_2_and_leaves_the_base() {
    let files = Files::new();
    let base = files.path("base.device.json");
    for (format, file) in FORMATS {
        assert_refused(&files.import(format, file, "cc by", &["--into", &base]), format);
        assert_eq!(std::fs::read_to_string(&base).unwrap(), BASE);
    }
}

#[test]
fn import_of_a_refused_source_writes_nothing_in_the_file_named_by_o() {
    let files = Files::new();
    let (base, out) = (files.path("base.device.json"), files.path("out.json"));
    for into in [vec![], vec!["--into", base.as_str()]] {
        let mut extra = vec!["-o", out.as_str()];
        extra.extend(into);
        assert_refused(&files.import("midi-csv", "cc.csv", "cc by", &extra), &format!("{extra:?}"));
        assert!(!std::path::Path::new(&out).exists(), "{extra:?}");

        std::fs::write(&out, "keep me").unwrap();
        extra.push("--force");
        assert_refused(&files.import("midi-csv", "cc.csv", "cc by", &extra), &format!("{extra:?}"));
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "keep me");
        std::fs::remove_file(&out).unwrap();
    }
}
