//! The conformance cases of the repository (`conformance/<version>/`), each
//! run as the specification states it.

mod common;

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// The files of a case, by catalogue id.
fn files(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for maker in std::fs::read_dir(dir).unwrap() {
        let maker = maker.unwrap().path();
        for f in std::fs::read_dir(&maker).unwrap() {
            let f = f.unwrap().path();
            let id = format!("{}/{}", maker.file_name().unwrap().to_str().unwrap(), f.file_stem().unwrap().to_str().unwrap());
            out.insert(id, std::fs::read_to_string(&f).unwrap());
        }
    }
    out
}

/// What a resolution case asked for that the reader did not return, if anything.
fn run_resolve(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let id = case["resolve"].as_str().unwrap();
    let got = osd::resolve(id, &files(&dir.join("files")));
    match (got, case.get("result"), case.get("fault")) {
        (Ok(r), Some(want), None) => {
            let doc: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap();
            (doc != *want).then(|| format!("resolved into {doc}"))
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "file": f.file, "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        (Ok(_), None, Some(want)) => Some(format!("resolved, but should be refused with {want}")),
        (Err(f), Some(_), None) => Some(format!("refused: {f}")),
        _ => Some("case.json has neither result nor fault, or both".into()),
    }
}

/// The kind a case reads, by the name of its schema.
fn kind(word: &str) -> osd::Kind {
    match word {
        "device" => osd::Kind::Device,
        "profile" => osd::Kind::Profile,
        "sound" => osd::Kind::Sound,
        "fingerprint" => osd::Kind::Fingerprint,
        "firmware" => osd::Kind::Firmware,
        _ => panic!("{word} is not a kind"),
    }
}

/// What a reading case asked for that the reader did not return, if anything.
fn run_read(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let got = osd::read(&text, kind(case["read"].as_str().unwrap()));
    match (got, case.get("result"), case.get("fault")) {
        (Ok(d), Some(want), None) => {
            let doc: Value = serde_json::from_str(&osd::write(&d)).unwrap();
            (doc != *want).then(|| format!("read into {doc}"))
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        (Ok(_), None, Some(want)) => Some(format!("read, but should be refused with {want}")),
        (Err(f), Some(_), None) => Some(format!("refused: {f}")),
        _ => Some("case.json has neither result nor fault, or both".into()),
    }
}

/// Every case of a folder run by `run`, the failures reported together.
fn every_case(root: &Path, least: usize, run: fn(&Path) -> Option<String>) {
    let mut cases: Vec<_> = std::fs::read_dir(root).unwrap().map(|e| e.unwrap().path()).collect();
    cases.sort();
    assert!(cases.len() >= least, "{} cases", cases.len());
    let failures = cases.iter()
        .filter_map(|c| run(c).map(|why| format!("{}: {why}", c.file_name().unwrap().to_string_lossy())))
        .collect();
    common::report("cases failed", failures);
}

#[test]
fn every_reading_case_of_0_4_passes() {
    every_case(&common::repo().join("conformance/0.4/read"), 2, run_read);
}

/// The schema errors a reading case's descriptor has, against what its case
/// expects: none for a result, exactly one at the fault's pointer for a
/// fault.
fn schema_errors(dir: &Path, schemas: &common::Schemas) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("descriptor.json")).unwrap()).unwrap();
    let errors = schemas.errors(kind(case["read"].as_str().unwrap()), &doc);
    let want: Vec<&str> = case.get("fault").map(|f| f["pointer"].as_str().unwrap()).into_iter().collect();
    let got: Vec<&str> = errors.iter().map(|(at, _)| at.as_str()).collect();
    (got != want).then(|| format!("schema errors {errors:?}, expected at {want:?}"))
}

#[test]
fn every_reading_case_validates_as_its_case_expects() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for version in std::fs::read_dir(common::repo().join("conformance")).unwrap() {
        let version = version.unwrap().path();
        let read = version.join("read");
        if !read.is_dir() {
            continue;
        }
        let schemas = common::Schemas::load(version.file_name().unwrap().to_str().unwrap());
        for case in std::fs::read_dir(&read).unwrap() {
            let case = case.unwrap().path();
            if let Some(why) = schema_errors(&case, &schemas) {
                failures.push(format!("{}: {why}", case.display()));
            }
            checked += 1;
        }
    }
    assert!(checked >= 2, "{checked} cases");
    common::report("cases do not validate as they expect", failures);
}

#[test]
fn every_file_of_a_resolution_case_that_expects_a_result_validates() {
    let schemas = common::Schemas::load("0.4");
    let mut failures = Vec::new();
    for case in std::fs::read_dir(common::repo().join("conformance/0.4/resolve")).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        if spec.get("result").is_none() {
            continue;
        }
        for (id, text) in files(&case.join("files")) {
            let kind = if id.contains("/_profile") { osd::Kind::Profile } else { osd::Kind::Device };
            if let Some(e) = schemas.check(kind, &serde_json::from_str(&text).unwrap()) {
                failures.push(format!("{}: {id}: {e}", case.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    common::report("files do not validate", failures);
}

#[test]
fn every_resolution_case_of_0_4_passes() {
    every_case(&common::repo().join("conformance/0.4/resolve"), 13, run_resolve);
}

#[test]
fn criterion_5_every_reading_case_of_0_5_passes() {
    every_case(&common::repo().join("conformance/0.5/read"), 6, run_read);
}

#[test]
fn criterion_5_the_reading_cases_of_0_5_cover_each_kind_and_a_refused_version() {
    let mut results = std::collections::BTreeSet::new();
    let mut unread = 0;
    for case in std::fs::read_dir(common::repo().join("conformance/0.5/read")).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        let text = std::fs::read_to_string(case.join("descriptor.json")).unwrap();
        assert!(text.contains("\"$schema\""), "{}: a 0.5 case carries its `$schema`", case.display());
        if spec.get("result").is_some() {
            results.insert(spec["read"].as_str().unwrap().to_string());
        }
        if spec["fault"]["kind"] == "UnreadVersion" {
            unread += 1;
        }
    }
    let all: std::collections::BTreeSet<String> =
        ["device", "profile", "sound", "fingerprint", "firmware"].iter().map(|s| s.to_string()).collect();
    assert_eq!(results, all);
    assert!(unread >= 1, "no case of an unread version");
}

/// What a conversion case asked for that `convert` did not return, if
/// anything. A result is the 0.5 document, equal to the converted text's,
/// key order aside; it reads as 0.5, validates against the 0.5 schema and
/// converts to itself. A fault is the refusal, by its `kind` and the JSON
/// `pointer` of the 0.4 element refused.
fn run_convert(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let kind = kind(case["convert"].as_str().unwrap());
    match (osd::convert(&text, kind), case.get("result"), case.get("fault")) {
        (Ok(out), Some(want), None) => {
            let doc: Value = serde_json::from_str(&out).unwrap();
            if doc != *want {
                return Some(format!("converted into {doc}"));
            }
            if let Err(f) = osd::read_as(&out, kind, osd::Version::V0_5) {
                return Some(format!("the result does not read: {f}"));
            }
            if let Some(e) = common::Schemas::load("0.5").check(kind, &doc) {
                return Some(format!("the result does not validate: {e}"));
            }
            (osd::convert(&out, kind).as_deref() != Ok(out.as_str())).then(|| "converting the result changes it".into())
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        (Ok(_), None, Some(want)) => Some(format!("converted, but should be refused with {want}")),
        (Err(f), Some(_), None) => Some(format!("refused: {f}")),
        _ => Some("case.json has neither result nor fault, or both".into()),
    }
}

/// The names of the cases of a folder.
fn case_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn criterion_4_every_conversion_case_of_0_5_passes() {
    every_case(&common::repo().join("conformance/0.5/convert"), 11, run_convert);
}

#[test]
fn criterion_4_the_conversion_cases_cover_each_frame_place_each_form_and_the_two_faults() {
    let root = common::repo().join("conformance/0.5/convert");
    let names = case_names(&root);
    for want in [
        "frame-commands", "frame-params", "frame-replies", "frame-identity", "frame-programs-before",
        "frame-programs-dump", "frame-profile", "pre-tree", "fault-checksum-without-algorithm", "fault-function",
    ] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
    }
    // criterion 12: `lsb` and `msb` are a refused encoding, with the address of the fault
    let name = "fault-encoding-lsb-msb";
    assert!(names.iter().any(|n| n == name), "no case {name} in {names:?}");
    let spec: Value = serde_json::from_str(&std::fs::read_to_string(root.join(name).join("case.json")).unwrap()).unwrap();
    assert_eq!(spec["fault"]["kind"], "NotConvertible", "{name}");
    assert!(spec["fault"]["pointer"].as_str().is_some_and(|p| p.starts_with("/commands/")), "{name}: {spec}");
    let text = std::fs::read_to_string(root.join(name).join("descriptor.json")).unwrap();
    assert!(text.contains(":lsb}") && text.contains(":msb}"), "{name}: shows neither lsb nor msb");
    let all: String = names
        .iter()
        .filter(|n| !n.starts_with("fault"))
        .map(|n| std::fs::read_to_string(root.join(n).join("case.json")).unwrap())
        .collect();
    for form in [
        "\"arg\"", "\"encoding\": \"u14_msb\"", "\"encoding\": \"u14_lsb\"", "\"encoding\": \"s32\"", "\"encoding\": \"ascii\"",
        "\"encoding\": \"bytes\"", "\"add\": 4", "\"capture\"", "\"device_id\"", "\"identity\": \"family\"", "\"identity\": \"member\"",
        "\"index\"", "\"checksum\": \"twos7\"", "\"from\"", "\"any\"", "\"sub\"", "\"mul\"", "\"gt\"", "\"lt\"",
        "\"eq\"", "\"ne\"", "\"if\"",
    ] {
        assert!(all.contains(form), "no conversion case shows {form}");
    }
}

/// A case's `case.json`.
fn spec(root: &Path, name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(root.join(name).join("case.json")).unwrap()).unwrap()
}

#[test]
fn criterion_12_the_conversion_cases_hold_each_address_and_argument_form_and_the_midi_in_list() {
    let root = common::repo().join("conformance/0.5/convert");
    let names = case_names(&root);
    for want in [
        "address-segments", "forward-args", "emit-args", "midi-in-list", "midi-in-empty",
        "fault-brace-unclosed", "fault-brace-empty", "fault-midi-template",
    ] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
    }
    let text = |n: &str| serde_json::to_string(&spec(&root, n)["result"]).unwrap();
    let all: String = ["address-segments", "forward-args", "emit-args"].iter().map(|n| text(n)).collect();
    assert!(all.contains("{\"capture\":"), "no segment capture in the address cases");
    assert!(text("forward-args").contains("{\"arg\":"), "forward-args shows no arg object");
    assert!(text("emit-args").contains("\"emit_args\""), "emit-args shows no emit_args");
    assert!(text("emit-args").contains("{\"capture\":"), "emit-args shows no capture argument");
    let literal = spec(&root, "forward-args")["result"].to_string();
    assert!(literal.contains("\"args\":[") && literal.chars().any(|c| c.is_ascii_digit()), "forward-args shows no literal");
    assert!(text("midi-in-list").contains("\"midi_in\":["), "midi-in-list shows no list");
    assert!(text("midi-in-empty").contains("\"midi_in\":[]"), "midi-in-empty shows no empty list");
    // the four address places each have a case that converts them
    let places = ["\"osc\":[", "\"path\":[", "\"match_osc\":[", "\"emit_osc\":["];
    let converted: String = names.iter().filter(|n| !n.starts_with("fault")).map(|n| spec(&root, n)["result"].to_string()).collect();
    for p in places {
        assert!(converted.contains(p), "no conversion case shows {p}");
    }
    for (name, kind) in [("fault-brace-unclosed", "NotConvertible"), ("fault-brace-empty", "NotConvertible"), ("fault-midi-template", "NotConvertible")] {
        let s = spec(&root, name);
        assert_eq!(s["fault"]["kind"], kind, "{name}");
        assert!(s["fault"]["pointer"].as_str().is_some_and(|p| p.starts_with('/')), "{name}: {s}");
    }
    assert!(spec(&root, "fault-midi-template")["fault"]["pointer"].as_str().unwrap().starts_with("/midi_in/"));
    let d = |n: &str| std::fs::read_to_string(root.join(n).join("descriptor.json")).unwrap();
    assert!(d("fault-brace-empty").contains("{}"), "fault-brace-empty shows no empty brace");
    assert!(d("fault-brace-unclosed").matches('{').count() > d("fault-brace-unclosed").matches('}').count(), "no unclosed brace");
}

#[test]
fn criterion_12_the_reading_cases_hold_each_form_and_refuse_each_malformed_one() {
    let root = common::repo().join("conformance/0.5/read");
    let names = case_names(&root);
    for want in ["address-segments", "args-forms", "midi-in-list", "midi-in-empty"] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
        assert!(spec(&root, want).get("result").is_some(), "{want}: a form read has a result");
    }
    assert!(spec(&root, "midi-in-empty")["result"]["midi_in"] == serde_json::json!([]));
    assert!(spec(&root, "midi-in-list")["result"]["midi_in"].as_array().is_some_and(|l| l.len() >= 2));
    let forms = spec(&root, "args-forms")["result"].to_string();
    assert!(forms.contains("{\"arg\":") && forms.contains("{\"capture\":"), "args-forms: {forms}");
    for (name, pointer_ends) in [
        ("address-brace-refused", "osc"),
        ("address-one-segment-refused", "osc"),
        ("arg-unknown-object-refused", ""),
        ("midi-in-unknown-refused", ""),
    ] {
        assert!(names.iter().any(|n| n == name), "no case {name} in {names:?}");
        let s = spec(&root, name);
        assert!(matches!(s["fault"]["kind"].as_str(), Some("BadValue" | "UnknownField")), "{name}: {s}");
        assert!(s["fault"]["pointer"].as_str().is_some_and(|p| p.ends_with(pointer_ends)), "{name}: {s}");
    }
    let p = |n: &str| spec(&root, n)["fault"]["pointer"].as_str().unwrap().to_string();
    assert!(p("arg-unknown-object-refused").contains("/args/"), "the fault is at the argument");
    assert!(p("midi-in-unknown-refused").starts_with("/midi_in/"), "the fault is at the type");
}

#[test]
fn criterion_5_and_6_the_reading_cases_of_0_5_hold_each_frame_form_and_refuse_each_malformed_element() {
    let root = common::repo().join("conformance/0.5/read");
    let names = case_names(&root);
    for want in [
        "frame-string-refused", "frame-two-keys-refused", "frame-unknown-key-refused", "frame-encoding-refused",
        "frame-algorithm-refused", "frame-index-refused", "pre-string-refused",
    ] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(root.join(want).join("case.json")).unwrap()).unwrap();
        assert!(spec.get("fault").is_some(), "{want}: a refusal is a fault");
    }
    for want in ["frame-forms", "frame-recognise", "pre-tree"] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
    }
}
