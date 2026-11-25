//! `osd` and a control with no message: each word that judges, renders or writes a descriptor
//! prints on the error output one line per control to be completed, the standard output and the
//! exit code unchanged.

use crate::common::{osd, Run};
use serde_json::{json, Value};

const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";
const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";

fn with_file(name: &str, text: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(name).to_string_lossy().into_owned();
    std::fs::write(&file, text).unwrap();
    (dir, file)
}

fn source() -> Value {
    json!({"type": "contributed", "url": "https://example.org/d", "license": "CC0-1.0"})
}

/// A device of 0.7 whose control 1 and `params` entry 0 have no way.
fn device_07() -> Value {
    json!({
        "$schema": D07,
        "sources": [source()],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "cc_params": {"entries": [
            {"osc": "/filter/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}]},
            {"osc": "/bank", "ways": [], "range": [0, 3]},
        ]},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [{"osc": "/shape", "ways": []}]},
    })
}

/// A device of 0.6 whose control 1 gives no message.
fn device_06() -> Value {
    json!({
        "$schema": D06,
        "_sources": [source()],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "cc_params": {"entries": [{"osc": "/filter/cutoff", "cc": 74}, {"osc": "/bank", "range": [0, 3]}]},
    })
}

const BOTH: [&str; 2] = ["/cc_params/entries/1", "/params/entries/0"];
const CC: [&str; 1] = ["/cc_params/entries/1"];

/// The line of a control to be completed, `asked` being the file or the id the word was given.
fn lines(asked: &str, pointers: &[&str]) -> Vec<String> {
    pointers.iter().map(|p| format!("{asked}: warning at \"{p}\": the control gives no message: it is to be completed")).collect()
}

/// The warning lines of the error output, which holds the losses of a word too.
fn warned(run: &Run) -> Vec<&str> {
    run.stderr.lines().filter(|l| l.contains(": warning at ")).collect()
}

#[test]
fn validate_prints_one_line_per_control_without_a_message_says_ok_and_exits_0() {
    let (_dir, file) = with_file("d.json", &device_07().to_string());
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, format!("{file}: ok\n"));
    assert_eq!(run.stderr.lines().collect::<Vec<_>>(), lines(&file, &BOTH));
}

#[test]
fn validate_warns_of_the_controls_of_0_4_to_0_6_and_names_each_file() {
    // A file of 0.4 holds no `$schema`: `--kind` says its kind.
    let old = json!({"device": {"name": "T", "osc_prefix": "/t"}, "cc_params": {"entries": [{"osc": "/a"}]}});
    let mut middle = device_06();
    middle["$schema"] = json!("https://w3id.org/osd/0.5/device.schema.json");
    let (_a, first) = with_file("a.json", &old.to_string());
    let (_b, second) = with_file("b.json", &middle.to_string());
    let (_c, third) = with_file("c.json", &device_06().to_string());
    let run = osd(&["validate", "--kind", "device", &first, &second, &third]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, format!("{first}: ok\n{second}: ok\n{third}: ok\n"));
    let mut want = lines(&first, &["/cc_params/entries/0"]);
    want.extend(lines(&second, &CC));
    want.extend(lines(&third, &CC));
    assert_eq!(run.stderr.lines().collect::<Vec<_>>(), want);
}

#[test]
fn the_lines_of_the_controls_come_before_the_line_of_an_unknown_provider_which_is_unchanged() {
    let mut doc = device_07();
    doc["sources"][0]["provider"] = json!("zz-one");
    let (_dir, file) = with_file("d.json", &doc.to_string());
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let mut want = lines(&file, &BOTH);
    want.push(format!("{file}: warning at \"/sources/0/provider\": the provider zz-one is not in the table of providers of {}", osd::PROVIDERS_VERSION));
    assert_eq!(run.stderr.lines().collect::<Vec<_>>(), want);
}

#[test]
fn convert_warns_of_the_controls_of_the_text_it_renders_and_renders_it() {
    let (_dir, file) = with_file("d.json", &device_06().to_string());
    for (args, target) in [
        (vec!["convert", "--to", "0.7", file.as_str()], osd::Version::V0_7),
        (vec!["convert", "--to", "0.6", file.as_str()], osd::Version::V0_6),
    ] {
        let run = osd(&args);
        assert_eq!(run.code, 0, "{target:?}: {:?}", run.stderr);
        assert_eq!(run.stdout, osd::convert_to(&device_06().to_string(), osd::Kind::Device, target).unwrap());
        assert_eq!(run.stderr.lines().collect::<Vec<_>>(), lines(&file, &CC), "{target:?}");
    }
}

#[test]
fn convert_to_a_file_warns_of_the_controls_of_the_text_it_writes() {
    let (dir, file) = with_file("d.json", &device_06().to_string());
    let out = dir.path().join("out.json").to_string_lossy().into_owned();
    let run = osd(&["convert", "--to", "0.7", &file, "-o", &out]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr.lines().collect::<Vec<_>>(), lines(&file, &CC));
    let written: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(written["cc_params"]["entries"][1], json!({"osc": "/bank", "range": [0, 3], "ways": []}));
}

#[test]
fn resolve_warns_of_the_controls_of_the_complete_descriptor_naming_the_id_asked() {
    let dir = tempfile::tempdir().unwrap();
    let maker = dir.path().join("acme");
    std::fs::create_dir_all(&maker).unwrap();
    let mut target = device_06();
    target["device"] = json!({"name": "Trundler", "osc_prefix": "/b"});
    let driver = json!({"$schema": D07, "midi_of": "acme/b", "device": {"name": "Trundler", "osc_prefix": "/a"}});
    std::fs::write(maker.join("a.json"), driver.to_string()).unwrap();
    std::fs::write(maker.join("b.json"), target.to_string()).unwrap();
    let run = osd(&["resolve", "acme/a", "--catalogue", dir.path().to_str().unwrap()]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let out: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{e}: {}", run.stdout));
    assert_eq!(out["cc_params"]["entries"][1]["ways"], json!([]));
    assert_eq!(run.stderr.lines().collect::<Vec<_>>(), lines("acme/a", &CC));
}

#[test]
fn export_warns_of_the_controls_of_the_descriptor_it_writes_beside_its_losses() {
    for doc in [device_07(), device_06()] {
        let (_dir, file) = with_file("d.json", &doc.to_string());
        let want = if doc["$schema"] == D07 { lines(&file, &BOTH) } else { lines(&file, &CC) };
        for format in ["midi-csv", "oscquery"] {
            let run = osd(&["export", format, &file]);
            assert_eq!(run.code, 0, "{format}: {:?}", run.stderr);
            assert!(!run.stdout.is_empty() && !run.stdout.contains("to be completed"), "{format}: {}", run.stdout);
            assert_eq!(warned(&run), want, "{format}: {:?}", run.stderr);
        }
    }
}

#[test]
fn import_into_a_descriptor_warns_of_the_controls_of_the_text_it_renders() {
    let header = "manufacturer,device,section,parameter_name,parameter_description,cc_msb,cc_lsb,cc_min_value,cc_max_value,cc_default_value,nrpn_msb,nrpn_lsb,nrpn_min_value,nrpn_max_value,nrpn_default_value,orientation,notes,usage";
    let (_csv, file) = with_file("in.csv", &format!("{header}\nLofty,Trundler,Filter,Resonance,,71,,0,127,,,,,,,,,\n"));
    let (_base, base) = with_file("base.json", &device_06().to_string());
    let run = osd(&["import", "midi-csv", &file, "--source-url", "https://example.org/x", "--license", "CC0-1.0", "--into", &base]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let out: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{e}: {}", run.stdout));
    assert_eq!(out["cc_params"]["entries"][1], json!({"osc": "/bank", "range": [0, 3], "ways": []}));
    assert_eq!(warned(&run), lines(&file, &CC), "{:?}", run.stderr);
}

#[test]
fn a_descriptor_whose_controls_all_give_a_message_gets_no_warning() {
    let mut doc = device_06();
    doc["cc_params"]["entries"].as_array_mut().unwrap().pop();
    let (_dir, file) = with_file("d.json", &doc.to_string());
    for args in [vec!["validate", file.as_str()], vec!["convert", "--to", "0.7", file.as_str()]] {
        let run = osd(&args);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""), "{args:?}");
    }
}
