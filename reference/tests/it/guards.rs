//! The guards: the floor of each kind and the committed list of the properties without description.

use super::build::{run, SCHEMAS};
use std::collections::BTreeMap;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const KINDS: [&str; 6] = ["device", "profile", "sound", "fingerprint", "firmware", "assembly"];

fn counts() -> BTreeMap<String, usize> {
    let res = run(&["count", SCHEMAS]);
    String::from_utf8(res.stdout)
        .unwrap()
        .lines()
        .map(|l| {
            let (k, rest) = l.split_once(": ").unwrap();
            let d: usize = rest.split(" of ").next().unwrap().parse().unwrap();
            (k.to_string(), d)
        })
        .collect()
}

fn floors() -> BTreeMap<String, usize> {
    let text = std::fs::read_to_string(format!("{ROOT}/reference/floors.json")).expect("floors.json committed");
    serde_json::from_str(&text).unwrap()
}

#[test]
fn each_kind_holds_at_least_its_committed_floor_of_described_properties() {
    let floors = floors();
    let now = counts();
    for k in KINDS {
        let floor = *floors.get(k).unwrap_or_else(|| panic!("no floor for {k}"));
        assert!(now[k] >= floor, "{k}: {} described, floor {floor}", now[k]);
    }
    assert_eq!(floors.len(), KINDS.len(), "one floor per kind, no global floor");
}

const SCHEMAS_0_7: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../schemas/0.7");

#[test]
fn the_0_7_schemas_hold_at_least_the_floor_of_described_properties_of_each_kind() {
    let res = run(&["count", SCHEMAS_0_7]);
    assert_eq!(res.status.code(), Some(0), "{}", String::from_utf8_lossy(&res.stderr));
    let text = String::from_utf8(res.stdout).unwrap();
    let floors = floors();
    for k in KINDS {
        let line = text.lines().find(|l| l.starts_with(&format!("{k}: "))).unwrap_or_else(|| panic!("no line for {k}"));
        let described: usize = line.split_once(": ").unwrap().1.split(" of ").next().unwrap().parse().unwrap();
        assert!(described >= floors[k], "{k}: {described} described, floor {}", floors[k]);
    }
}

#[test]
fn the_generator_builds_one_page_per_kind_from_the_0_7_schemas_beside_the_0_6_ones() {
    let (six, seven) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    assert_eq!(run(&["build", SCHEMAS, six.path().to_str().unwrap()]).status.code(), Some(0));
    let res = run(&["build", SCHEMAS_0_7, seven.path().to_str().unwrap()]);
    assert_eq!(res.status.code(), Some(0), "{}", String::from_utf8_lossy(&res.stderr));
    for k in KINDS {
        let page = std::fs::read_to_string(seven.path().join(format!("{k}.md"))).unwrap_or_else(|e| panic!("{k}.md: {e}"));
        assert!(!page.is_empty(), "{k}.md");
        assert!(std::fs::metadata(six.path().join(format!("{k}.md"))).is_ok(), "{k}.md of 0.6 is still built");
    }
}

#[test]
fn the_committed_list_of_undescribed_properties_of_0_7_is_what_the_generator_writes_now() {
    let committed = std::fs::read_to_string(format!("{ROOT}/reference/undescribed-0.7.txt")).expect("undescribed-0.7.txt committed");
    let now = String::from_utf8(run(&["undescribed", SCHEMAS_0_7]).stdout).unwrap();
    assert!(!now.is_empty());
    assert_eq!(committed, now);
}

#[test]
fn the_device_floor_is_144_today() {
    assert_eq!(floors()["device"], 144);
}

#[test]
fn the_committed_list_of_undescribed_properties_is_what_the_generator_writes_now() {
    let committed = std::fs::read_to_string(format!("{ROOT}/reference/undescribed-0.6.txt"))
        .expect("undescribed-0.6.txt committed");
    let now = String::from_utf8(run(&["undescribed", SCHEMAS]).stdout).unwrap();
    assert_eq!(committed, now);
}
