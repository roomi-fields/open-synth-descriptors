//! `osd` and the `provider` of a source: `osd import --source-provider` says who supplies the
//! file, and each word that judges, renders or writes a descriptor prints on the error output one
//! line per provider its table does not hold, the standard output and the exit code unchanged.

use crate::common::osd;
use serde_json::{json, Value};

const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const UNKNOWN: &str = "electra-oen";

const HEADER: &str = "manufacturer,device,section,parameter_name,parameter_description,cc_msb,cc_lsb,cc_min_value,cc_max_value,cc_default_value,nrpn_msb,nrpn_lsb,nrpn_min_value,nrpn_max_value,nrpn_default_value,orientation,notes,usage";

fn with_file(name: &str, text: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(name).to_string_lossy().into_owned();
    std::fs::write(&file, text).unwrap();
    (dir, file)
}

fn device(sources: Value) -> Value {
    json!({
        "$schema": D07,
        "sources": sources,
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "cc_params": {"entries": [{"osc": "/filter/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}]}]},
    })
}

fn source(provider: &str) -> Value {
    json!({"type": "contributed", "provider": provider, "url": "https://example.org/d", "license": "CC0-1.0"})
}

/// The warnings of the error output: the lines that name the version of the table.
fn warnings(stderr: &str) -> Vec<&str> {
    stderr.lines().filter(|l| l.contains(osd::PROVIDERS_VERSION)).collect()
}

/// `line` names what was asked for, the pointer and the unknown provider.
fn assert_warning(line: &str, asked: &str, pointer: &str, provider: &str) {
    for part in [asked, pointer, provider] {
        assert!(line.contains(part), "{part} is not in {line:?}");
    }
}

// ---- validate ----

#[test]
fn validate_warns_of_an_unknown_provider_says_ok_and_exits_0() {
    let (_dir, file) = with_file("d.json", &device(json!([source(UNKNOWN)])).to_string());
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, format!("{file}: ok\n"));
    let lines = warnings(&run.stderr);
    assert_eq!(lines.len(), 1, "{:?}", run.stderr);
    assert_warning(lines[0], &file, "/sources/0/provider", UNKNOWN);
}

#[test]
fn validate_prints_one_line_per_unknown_provider_of_each_file() {
    let mut doc = device(json!([source("electra-one"), source("zz-one"), source("zz-two")]));
    doc["midi_of_sources"] = json!([source("zz-one")]);
    let (_a, first) = with_file("a.json", &doc.to_string());
    let (_b, second) = with_file("b.json", &device(json!([source("zz-b")])).to_string());
    let run = osd(&["validate", &first, &second]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let lines = warnings(&run.stderr);
    assert_eq!(lines.len(), 4, "{:?}", run.stderr);
    assert_warning(lines[0], &first, "/sources/1/provider", "zz-one");
    assert_warning(lines[1], &first, "/sources/2/provider", "zz-two");
    assert_warning(lines[2], &first, "/midi_of_sources/0/provider", "zz-one");
    assert_warning(lines[3], &second, "/sources/0/provider", "zz-b");
}

#[test]
fn validate_says_nothing_of_a_provider_of_the_table_or_of_a_descriptor_before_0_7() {
    let known = device(json!([source("electra-one"), source("pencilresearch"), source("knobkraft")]));
    let old = json!({"$schema": "https://w3id.org/osd/0.6/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"},
        "_sources": [{"type": "knobkraft", "provider": UNKNOWN}]});
    for doc in [known, old] {
        let (_dir, file) = with_file("d.json", &doc.to_string());
        let run = osd(&["validate", &file]);
        assert_eq!(run.code, 0, "{:?}", run.stderr);
        assert_eq!(run.stderr, "", "{doc}");
    }
}

// ---- convert ----

#[test]
fn convert_warns_of_an_unknown_provider_of_the_text_it_renders() {
    let doc = json!({"$schema": "https://w3id.org/osd/0.6/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"},
        "_sources": [{"type": "knobkraft"}, {"type": "knobkraft", "provider": UNKNOWN}]});
    let (_dir, file) = with_file("d.json", &doc.to_string());
    let run = osd(&["convert", "--to", "0.7", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, osd::convert_to(&doc.to_string(), osd::Kind::Device, osd::Version::V0_7).unwrap());
    let lines = warnings(&run.stderr);
    assert_eq!(lines.len(), 1, "{:?}", run.stderr);
    assert_warning(lines[0], &file, "/sources/1/provider", UNKNOWN);
    let to_06 = osd(&["convert", "--to", "0.6", &file]);
    assert_eq!(to_06.code, 0, "{:?}", to_06.stderr);
    assert!(warnings(&to_06.stderr).is_empty(), "{:?}", to_06.stderr);
}

// ---- resolve ----

#[test]
fn resolve_warns_of_the_providers_of_the_complete_descriptor_naming_the_id_asked() {
    let dir = tempfile::tempdir().unwrap();
    let maker = dir.path().join("acme");
    std::fs::create_dir_all(&maker).unwrap();
    let driver = json!({"$schema": D07, "midi_of": "acme/b", "device": {"name": "B", "osc_prefix": "/a"}, "sources": [source("zz-driver")]});
    let mut target = device(json!([source("zz-target")]));
    target["device"] = json!({"name": "B", "osc_prefix": "/b"});
    std::fs::write(maker.join("a.json"), driver.to_string()).unwrap();
    std::fs::write(maker.join("b.json"), target.to_string()).unwrap();
    let run = osd(&["resolve", "acme/a", "--catalogue", dir.path().to_str().unwrap()]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(matches!(osd::read(&run.stdout, osd::Kind::Device), Ok(osd::Descriptor::Device(_))), "{}", run.stdout);
    let lines = warnings(&run.stderr);
    assert_eq!(lines.len(), 2, "{:?}", run.stderr);
    assert_warning(lines[0], "acme/a", "/sources/0/provider", "zz-driver");
    assert_warning(lines[1], "acme/a", "/midi_of_sources/0/provider", "zz-target");
}

// ---- import ----

fn csv() -> (tempfile::TempDir, String) {
    with_file("in.csv", &format!("{HEADER}\nLofty,Trundler,Filter,Cutoff,,74,,0,127,,,,,,,,,\n"))
}

fn import<'a>(file: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["import", "midi-csv", file, "--source-url", "https://example.org/x", "--license", "CC0-1.0"];
    args.extend_from_slice(extra);
    args
}

#[test]
fn import_writes_the_provider_given_by_source_provider_and_none_without_it() {
    let (_dir, file) = csv();
    let run = osd(&import(&file, &["--source-provider", "pencilresearch"]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let out: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(out["sources"][0]["provider"], "pencilresearch");
    assert!(warnings(&run.stderr).is_empty(), "{:?}", run.stderr);

    let plain = osd(&import(&file, &[]));
    assert_eq!(plain.code, 0, "{:?}", plain.stderr);
    let out: Value = serde_json::from_str(&plain.stdout).unwrap();
    assert!(out["sources"][0].get("provider").is_none(), "{out}");
}

#[test]
fn import_into_gives_the_provider_to_the_source_it_adds_only() {
    let (_dir, file) = csv();
    let (_base, base) = with_file("base.json", &device(json!([{"type": "vendor-doc", "url": "u", "license": "CC0-1.0"}])).to_string());
    let run = osd(&import(&file, &["--source-provider", "pencilresearch", "--into", &base]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let out: Value = serde_json::from_str(&run.stdout).unwrap();
    assert!(out["sources"][0].get("provider").is_none(), "{out}");
    assert_eq!(out["sources"][1]["provider"], "pencilresearch");
}

#[test]
fn import_warns_of_an_unknown_provider_of_the_text_it_renders() {
    let (_dir, file) = csv();
    let run = osd(&import(&file, &["--source-provider", UNKNOWN]));
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let out: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(out["sources"][0]["provider"], UNKNOWN);
    let lines = warnings(&run.stderr);
    assert_eq!(lines.len(), 1, "{:?}", run.stderr);
    assert_warning(lines[0], &file, "/sources/0/provider", UNKNOWN);
}

#[test]
fn the_help_of_import_names_source_provider() {
    let run = osd(&["import", "--help"]);
    assert_eq!(run.code, 0);
    assert!(run.stdout.contains("--source-provider"), "{}", run.stdout);
}

#[test]
fn source_provider_is_an_option_of_import_alone() {
    let (_dir, file) = with_file("d.json", &device(json!([source("electra-one")])).to_string());
    let run = osd(&["export", "midi-csv", &file, "--source-provider", "electra-one"]);
    assert_eq!(run.code, 2, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}

// ---- export ----

#[test]
fn export_warns_of_an_unknown_provider_and_writes_the_same_file() {
    let (_dir, file) = with_file("d.json", &device(json!([source("electra-one"), source(UNKNOWN)])).to_string());
    for format in ["midi-csv", "midnam"] {
        let run = osd(&["export", format, &file]);
        assert_eq!(run.code, 0, "{format}: {:?}", run.stderr);
        assert!(!run.stdout.is_empty() && !run.stdout.contains(osd::PROVIDERS_VERSION), "{format}: {}", run.stdout);
        let lines = warnings(&run.stderr);
        assert_eq!(lines.len(), 1, "{format}: {:?}", run.stderr);
        assert_warning(lines[0], &file, "/sources/1/provider", UNKNOWN);
    }
}
