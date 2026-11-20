//! The skeleton `osd export knobkraft` writes passes the generic test suite of KnobKraft Orm
//! (Christof Ruch, AGPL-3.0, https://github.com/christofmuc/KnobKraft-orm), for the functions it
//! writes. The suite is Python run by KnobKraft's own tooling, so only the night runs it; the
//! text of the skeleton is judged by the day tests of osd-formats.
//!
//! KnobKraft is pinned to one revision and cloned once under `~/.cache/grillhouse/`; the night
//! then runs offline. The first run needs the network and `git`; the suite needs `python3` with
//! `pytest`. Nothing of KnobKraft is copied into this repository.

use crate::common::osd_in;
use std::path::{Path, PathBuf};
use std::process::Command;

const REVISION: &str = "9e2522cd44765603162b4afd31419c3597ff85fc";
const URL: &str = "https://github.com/christofmuc/KnobKraft-orm";

fn run(program: &str, args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new(program).args(args).current_dir(cwd).output().unwrap_or_else(|e| panic!("{program}: {e}"))
}

fn git_ok(args: &[&str], cwd: &Path) -> bool {
    run("git", args, cwd).status.success()
}

/// The checkout of KnobKraft Orm at the pinned revision, cloned the first time.
fn checkout() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME");
    let cache = Path::new(&home).join(".cache/grillhouse");
    std::fs::create_dir_all(&cache).unwrap();
    let dir = cache.join(format!("knobkraft-orm-{}", &REVISION[..12]));
    if !dir.join(".git").exists() {
        assert!(git_ok(&["clone", "--quiet", "--no-checkout", URL, dir.to_str().unwrap()], &cache), "clone of {URL}");
    }
    if !git_ok(&["cat-file", "-e", REVISION], &dir) {
        assert!(git_ok(&["fetch", "--quiet", "origin", REVISION], &dir), "fetch of {REVISION}");
    }
    assert!(git_ok(&["checkout", "--quiet", "--force", REVISION], &dir), "checkout of {REVISION}");
    dir
}

const DEVICE: &str = r#"{
  "$schema": "https://w3id.org/osd/0.6/device.schema.json",
  "_sources": [{"type": "vendor-doc", "note": "A test device."}],
  "role_vocabulary": "1",
  "device": {"name": "Night Synth", "osc_prefix": "/night"},
  "programs": {
    "banks": [{"name": "A", "size": 64}, {"name": "B", "size": 64}],
    "dump": {"kind": "request", "request": [240, 66, 48, 0, 1, 81, 16, 247]}
  },
  "sound_transfers": [{
    "id": "single", "sound_id": "program", "carries": "sound",
    "match_frame": [240, 66, "any", 0, 1, 81, 64, "any", "any", "any", "any", "any", "any", "any", "any", "any", 247],
    "elements": {"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}}
  }]
}"#;

#[test]
fn an_exported_skeleton_passes_the_generic_test_suite_of_knobkraft_for_the_functions_it_writes() {
    let kk = checkout();
    let work = tempfile::tempdir().unwrap();
    let device = work.path().join("night.device.json");
    std::fs::write(&device, DEVICE).unwrap();
    let export = osd_in(work.path(), &["export", "knobkraft", device.to_str().unwrap()]);
    assert_eq!(export.code, 0, "{:?}", export.stderr);
    let skeleton = work.path().join("night_synth.py");
    std::fs::write(&skeleton, &export.stdout).unwrap();

    let adaptations = kk.join("adaptations");
    let out = run("python3", &["-m", "pytest", "-q", "test_adaptations.py", "--adaptation", skeleton.to_str().unwrap()], &adaptations);
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "the generic suite fails on the skeleton:\n{text}");
    assert!(text.contains("passed"), "the generic suite ran no test on the skeleton:\n{text}");
    assert!(!text.contains("failed") && !text.contains("error"), "{text}");
}
