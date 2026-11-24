//! The judgement of a sound's needs against an assembly (`osd::judge_needs`): each need of the
//! sound is satisfied, not satisfied or unknown, judged on the version the assembly pins for the
//! module, never on a catalogue descriptor. A need without `min_version` is satisfied when the
//! module figures in the assembly; a module the assembly lacks is not satisfied; a pinned version
//! or a minimum that `compare_versions` cannot read leaves the need unknown.
//!
//! The conformance cases (`conformance/0.6/judge_needs/<case>/`: `sound.json`, `assembly.json`
//! and `case.json` with `judge_needs: true` and `result`, a `{module, verdict}` per need, the
//! verdict being `satisfied`, `not_satisfied` or `unknown`) are run here as the specification
//! states them.

use crate::common;

use osd::{Descriptor, Kind, Verdict};
use serde_json::{json, Value};
use std::path::Path;

fn sound(needs: Value) -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.6/sound.schema.json",
        "name": "Warm pad",
        "kind": "sound",
        "device": {"descriptor": "korg/minilogue", "vendor": "KORG", "model": "minilogue"},
        "content": {"settings": {"/amp/attack": [90]}},
        "needs": needs
    })
}

fn assembly(modules: &[(&str, &str)]) -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.6/assembly.schema.json",
        "stock": {"firmware": "acme/stock", "version": "1.0"},
        "modules": modules.iter().map(|(m, v)| json!({"module": m, "version": v})).collect::<Vec<_>>()
    })
}

fn judge(sound: &Value, assembly: &Value) -> Vec<(String, Verdict)> {
    let (Descriptor::Sound(s), Descriptor::Assembly(a)) = (
        osd::read(&sound.to_string(), Kind::Sound).unwrap_or_else(|f| panic!("{sound}: {f}")),
        osd::read(&assembly.to_string(), Kind::Assembly).unwrap_or_else(|f| panic!("{assembly}: {f}")),
    ) else {
        panic!("a sound and an assembly")
    };
    osd::judge_needs(&s, &a).into_iter().map(|j| (j.module, j.verdict)).collect()
}

/// One need `module` (with `min`, if any) judged against an assembly pinning `pins`.
fn one(module: &str, min: Option<&str>, pins: &[(&str, &str)]) -> Verdict {
    let need = match min {
        Some(m) => json!({"module": module, "min_version": m}),
        None => json!({"module": module}),
    };
    let got = judge(&sound(json!([need])), &assembly(pins));
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, module);
    got[0].1
}

#[test]
fn a_need_is_satisfied_when_the_pinned_version_is_not_lower_than_the_minimum() {
    let pin = |v| [("acme/mod-a", v)];
    assert_eq!(one("acme/mod-a", Some("1.2"), &pin("1.3")), Verdict::Satisfied);
    assert_eq!(one("acme/mod-a", Some("1.2"), &pin("1.2")), Verdict::Satisfied, "equal");
    assert_eq!(one("acme/mod-a", Some("1.2"), &pin("1.2.0")), Verdict::Satisfied, "1.2 = 1.2.0");
    assert_eq!(one("acme/mod-a", Some("1.9"), &pin("1.10")), Verdict::Satisfied, "numbers compare as numbers");
    assert_eq!(one("acme/mod-a", Some("1.40"), &pin("1.40B")), Verdict::Satisfied, "letters after the number");
}

#[test]
fn a_need_is_not_satisfied_when_the_pinned_version_is_lower_than_the_minimum() {
    let pin = |v| [("acme/mod-a", v)];
    assert_eq!(one("acme/mod-a", Some("1.2"), &pin("1.0")), Verdict::NotSatisfied);
    assert_eq!(one("acme/mod-a", Some("1.10"), &pin("1.9")), Verdict::NotSatisfied);
    assert_eq!(one("acme/mod-a", Some("1.40B"), &pin("1.40")), Verdict::NotSatisfied);
}

#[test]
fn a_need_without_a_minimum_is_satisfied_when_the_module_figures_in_the_assembly() {
    assert_eq!(one("acme/mod-a", None, &[("acme/mod-a", "0.1")]), Verdict::Satisfied);
}

#[test]
fn a_module_the_assembly_does_not_pin_is_not_satisfied_with_or_without_a_minimum() {
    assert_eq!(one("acme/mod-z", None, &[("acme/mod-a", "1.3")]), Verdict::NotSatisfied);
    assert_eq!(one("acme/mod-z", Some("1.0"), &[("acme/mod-a", "1.3")]), Verdict::NotSatisfied);
    assert_eq!(one("acme/mod-a", None, &[]), Verdict::NotSatisfied);
}

#[test]
fn a_version_the_comparison_cannot_read_leaves_the_need_unknown() {
    assert_eq!(one("acme/mod-a", Some("1.2"), &[("acme/mod-a", "nightly")]), Verdict::Unknown, "pinned");
    assert_eq!(one("acme/mod-a", Some("1.2"), &[("acme/mod-a", "0.3-beta")]), Verdict::Unknown, "pinned, with a dash");
}

#[test]
fn a_minimum_outside_the_grammar_is_refused_when_the_sound_is_read() {
    let need = json!([{"module": "acme/mod-a", "min_version": "nightly"}]);
    assert!(osd::read(&sound(need).to_string(), Kind::Sound).is_err());
}

#[test]
fn a_need_without_a_minimum_is_satisfied_even_when_the_pinned_version_cannot_be_read() {
    assert_eq!(one("acme/mod-a", None, &[("acme/mod-a", "nightly")]), Verdict::Satisfied);
}

#[test]
fn every_need_gets_its_verdict_in_the_order_of_the_sound() {
    let needs = json!([
        {"module": "acme/mod-b", "min_version": "2"},
        {"module": "acme/mod-a", "min_version": "1.2"},
        {"module": "acme/mod-z"},
        {"module": "acme/mod-c", "min_version": "1"}
    ]);
    let got = judge(&sound(needs), &assembly(&[("acme/mod-a", "1.3"), ("acme/mod-b", "1.0"), ("acme/mod-c", "nightly")]));
    assert_eq!(
        got,
        [
            ("acme/mod-b".to_string(), Verdict::NotSatisfied),
            ("acme/mod-a".to_string(), Verdict::Satisfied),
            ("acme/mod-z".to_string(), Verdict::NotSatisfied),
            ("acme/mod-c".to_string(), Verdict::Unknown),
        ]
    );
}

#[test]
fn a_sound_without_needs_or_with_an_empty_list_has_no_verdict() {
    let mut bare = sound(json!([]));
    bare.as_object_mut().unwrap().remove("needs");
    assert!(judge(&bare, &assembly(&[("acme/mod-a", "1")])).is_empty());
    assert!(judge(&sound(json!([])), &assembly(&[])).is_empty());
}

#[test]
fn the_judgement_reads_only_the_assembly_the_made_on_of_the_sound_does_not_count() {
    // The sound was made on mod-a 1.0; the assembly it is judged against pins 1.3.
    let mut s = sound(json!([{"module": "acme/mod-a", "min_version": "1.2"}]));
    s["made_on"] = json!({"stock": {"firmware": "acme/stock", "version": "1.0"}, "modules": [{"module": "acme/mod-a", "version": "1.0"}]});
    assert_eq!(judge(&s, &assembly(&[("acme/mod-a", "1.3")])), [("acme/mod-a".to_string(), Verdict::Satisfied)]);
    assert_eq!(judge(&s, &assembly(&[("acme/mod-a", "1.0")])), [("acme/mod-a".to_string(), Verdict::NotSatisfied)]);
}

// ---- the conformance cases ----

/// What a `judge_needs` case asked for that the reader did not return, if anything.
fn run_case(dir: &Path) -> Option<String> {
    let text = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap_or_else(|e| panic!("{}/{f}: {e}", dir.display()));
    let json = |f: &str| -> Value { serde_json::from_str(&text(f)).unwrap() };
    let (case, sound, assembly) = (json("case.json"), json("sound.json"), json("assembly.json"));
    if case["judge_needs"] != json!(true) {
        return Some("case.json does not say `judge_needs: true`".into());
    }
    let schemas = common::Schemas::load("0.6");
    for (kind, doc, name) in [(Kind::Sound, &sound, "sound.json"), (Kind::Assembly, &assembly, "assembly.json")] {
        if let Some(e) = schemas.check(kind, doc) {
            return Some(format!("{name} does not validate: {e}"));
        }
    }
    let word = |v: Verdict| match v {
        Verdict::Satisfied => "satisfied",
        Verdict::NotSatisfied => "not_satisfied",
        Verdict::Unknown => "unknown",
    };
    let got: Vec<Value> = judge(&sound, &assembly).into_iter().map(|(m, v)| json!({"module": m, "verdict": word(v)})).collect();
    (Value::Array(got.clone()) != case["result"]).then(|| format!("judged {got:?}, the case states {}", case["result"]))
}

#[test]
fn every_judgement_case_of_0_6_passes_and_the_cases_show_each_verdict() {
    let root = common::repo().join("conformance/0.6/judge_needs");
    let mut failures = Vec::new();
    let mut names = Vec::new();
    for case in std::fs::read_dir(&root).unwrap_or_else(|e| panic!("{}: {e}", root.display())) {
        let case = case.unwrap().path();
        names.push(case.file_name().unwrap().to_string_lossy().into_owned());
        if let Some(why) = run_case(&case) {
            failures.push(format!("{}: {why}", names.last().unwrap()));
        }
    }
    common::report("judgement cases fail", failures);
    for want in [
        "need-met-by-pinned-version",
        "need-not-met-by-pinned-version",
        "need-without-min-version-module-present",
        "need-module-absent",
        "need-pinned-version-unreadable",
    ] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
    }
}

#[test]
fn the_judgement_cases_state_the_verdict_their_name_gives() {
    let root = common::repo().join("conformance/0.6/judge_needs");
    let verdicts = |name: &str| -> Vec<String> {
        let text = std::fs::read_to_string(root.join(name).join("case.json")).unwrap_or_else(|e| panic!("{name}: {e}"));
        let case: Value = serde_json::from_str(&text).unwrap();
        case["result"].as_array().unwrap().iter().map(|r| r["verdict"].as_str().unwrap().to_string()).collect()
    };
    assert_eq!(verdicts("need-met-by-pinned-version"), ["satisfied"]);
    assert_eq!(verdicts("need-not-met-by-pinned-version"), ["not_satisfied"]);
    assert_eq!(verdicts("need-without-min-version-module-present"), ["satisfied"]);
    assert_eq!(verdicts("need-module-absent"), ["not_satisfied"]);
    assert_eq!(verdicts("need-pinned-version-unreadable"), ["unknown"]);
}
