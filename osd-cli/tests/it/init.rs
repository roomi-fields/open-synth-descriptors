//! `osd init`: a template per kind, its marked places filled from the kind's example.

use crate::common::{self, osd, osd_in, Schemas, INIT, KINDS};
use serde_json::Value;

/// The JSON pointers of every string that starts with `FILL: `.
fn fill_places(v: &Value, at: &mut String, out: &mut Vec<String>) {
    match v {
        Value::String(s) if s.starts_with("FILL: ") => out.push(at.clone()),
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                let n = at.len();
                at.push_str(&format!("/{i}"));
                fill_places(x, at, out);
                at.truncate(n);
            }
        }
        Value::Object(o) => {
            for (k, x) in o {
                let n = at.len();
                at.push('/');
                at.push_str(&k.replace('~', "~0").replace('/', "~1"));
                fill_places(x, at, out);
                at.truncate(n);
            }
        }
        _ => {}
    }
}

fn template(kind: &str) -> Value {
    let run = osd(&[INIT, kind]);
    assert_eq!(run.code, 0, "{kind}: {:?}", run.stderr);
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{kind}: the template is not JSON: {e}"))
}

#[test]
fn init_writes_a_template_of_each_kind_with_the_latest_schema() {
    let latest = osd::VERSIONS.last().unwrap().name();
    for kind in KINDS {
        let t = template(kind);
        assert_eq!(t["$schema"], format!("https://w3id.org/osd/{latest}/{kind}.schema.json"), "{kind}");
    }
}

#[test]
fn init_marks_each_place_to_fill_with_the_help_after_fill() {
    for kind in KINDS {
        let mut places = Vec::new();
        fill_places(&template(kind), &mut String::new(), &mut places);
        assert!(!places.is_empty(), "{kind}: no place to fill");
        let t = template(kind);
        for p in places {
            let help = t.pointer(&p).unwrap().as_str().unwrap();
            assert!(help.len() > "FILL: ".len(), "{kind} {p}: no help after FILL: ");
        }
    }
}

#[test]
fn init_template_filled_from_the_example_reads_and_validates_against_the_schema() {
    let schemas = Schemas::load("0.5");
    for kind in KINDS {
        let example: Value = serde_json::from_str(&std::fs::read_to_string(common::example_path(kind)).unwrap()).unwrap();
        let mut filled = template(kind);
        let mut places = Vec::new();
        fill_places(&filled, &mut String::new(), &mut places);
        for p in &places {
            let value = example.pointer(p).unwrap_or_else(|| panic!("{kind}: the example has nothing at {p:?}")).clone();
            *filled.pointer_mut(p).unwrap() = value;
        }
        let text = serde_json::to_string_pretty(&filled).unwrap();
        osd::read(&text, common::kind_of(kind)).unwrap_or_else(|f| panic!("{kind}: {f}"));
        assert_eq!(schemas.check(common::kind_of(kind), &filled), None, "{kind}");

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("filled.json");
        std::fs::write(&file, &text).unwrap();
        let file = file.to_string_lossy().into_owned();
        let run = osd(&["validate", &file]);
        assert_eq!((run.code, run.stdout), (0, format!("{file}: ok\n")), "{kind}: {:?}", run.stderr);
    }
}

#[test]
fn init_with_an_unknown_kind_exits_2_listing_the_five_kinds() {
    let run = osd(&[INIT, "gadget"]);
    assert_eq!(run.code, 2);
    assert_eq!(run.stdout, "");
    for kind in KINDS {
        assert!(run.stderr.contains(kind), "{kind} missing from {:?}", run.stderr);
    }
}

#[test]
fn init_without_o_prints_and_creates_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let run = osd_in(dir.path(), &[INIT, "device"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(!run.stdout.is_empty());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn init_writes_the_file_named_by_o_and_prints_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("new.json").to_string_lossy().into_owned();
    let want = osd(&[INIT, "sound"]).stdout;
    let run = osd(&[INIT, "sound", "-o", &out]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), want);
}

#[test]
fn init_leaves_an_existing_output_file_unless_forced() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("new.json");
    std::fs::write(&file, "keep me").unwrap();
    let out = file.to_string_lossy().into_owned();

    let run = osd(&[INIT, "device", "-o", &out]);
    assert_eq!(run.code, 2);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "keep me");

    let run = osd(&[INIT, "device", "-o", &out, "--force"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), osd(&[INIT, "device"]).stdout);
}
