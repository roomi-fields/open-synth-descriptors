//! `tools/candidates.list`, the one place that says a version of the format is a candidate: its
//! form, its agreement with the versions the reader reads, and `tools/validate.py`, which checks
//! against a candidate only when asked. Medium for the script, which runs as a process.
// @lit tools/*

use crate::common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const CANDIDATES: &str = "tools/candidates.list";

fn read(rel: &str) -> String {
    std::fs::read_to_string(common::repo().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The lines of the list that are no comment, each as its version and the paths it names.
fn lines() -> Vec<(String, Vec<String>)> {
    read(CANDIDATES)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            let mut words = line.split_whitespace().map(String::from);
            (words.next().unwrap(), words.collect())
        })
        .collect()
}

// ---- The list ----

#[test]
fn the_list_of_candidates_names_the_versions_the_reader_reads_after_the_published_one() {
    let listed: Vec<String> = lines().into_iter().map(|(version, _)| version).collect();
    let once: BTreeSet<&str> = listed.iter().map(String::as_str).collect();
    assert_eq!(once.len(), listed.len(), "a version is listed twice: {listed:?}");
    let unpublished: BTreeSet<&str> =
        osd::VERSIONS.iter().filter(|v| **v > osd::PUBLISHED).map(|v| v.name()).collect();
    assert_eq!(once, unpublished, "{CANDIDATES} and the versions osd reads after {}", osd::PUBLISHED.name());
}

#[test]
fn each_path_a_line_of_the_list_names_is_a_file_under_schemas_outside_the_folder_of_its_version() {
    for (version, paths) in lines() {
        for path in paths {
            assert!(path.starts_with("schemas/"), "{version}: {path}");
            assert!(!path.starts_with(&format!("schemas/{version}/")), "{version}: {path} is in its folder");
            assert!(common::repo().join(&path).is_file(), "{version}: {path} is no file");
        }
    }
}

#[test]
fn the_line_of_0_7_names_the_table_of_providers_and_its_schema() {
    for (version, paths) in lines() {
        if version != "0.7" {
            continue;
        }
        let paths: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
        assert_eq!(paths, BTreeSet::from(["schemas/providers.json", "schemas/providers.schema.json"]));
    }
}

#[test]
fn the_public_workflows_give_no_list_of_candidates_to_the_scripts() {
    for workflow in [".github/workflows/check.yml", ".github/workflows/pages.yml", ".github/workflows/release.yml"] {
        assert!(!read(workflow).contains("--candidates"), "{workflow} gives its own list");
    }
}

// ---- tools/validate.py ----

struct Run {
    code: Option<i32>,
    out: String,
}

/// Runs the script with `OSD_VERSION` set to `version`, or removed.
fn python(script: &Path, args: &[&str], version: Option<&str>) -> Run {
    let mut command = Command::new("python3");
    command.arg(script).args(args).env_remove("OSD_VERSION");
    if let Some(version) = version {
        command.env("OSD_VERSION", version);
    }
    let o = command.output().expect("python3 runs");
    Run {
        code: o.status.code(),
        out: format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
    }
}

/// A repository with the scripts of `tools/`, the versions 0.9, 0.10 and 0.11, whose device schema
/// each asks for a property named by its version, the folder `vocabulary`, and one descriptor of
/// each version. 0.10 and 0.11 are listed as candidates.
struct Fixture(TempDir);

const VERSIONS: [&str; 3] = ["0.9", "0.10", "0.11"];

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        std::fs::create_dir_all(root.join("tools")).unwrap();
        for entry in std::fs::read_dir(common::repo().join("tools")).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                std::fs::copy(&path, root.join("tools").join(path.file_name().unwrap())).unwrap();
            }
        }
        write(CANDIDATES, "# the candidates\n0.11\n0.10 schemas/table.json\n");
        for version in VERSIONS {
            let schema = serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "$id": format!("https://example.org/{version}/device.schema.json"),
                "type": "object",
                "required": [version],
            });
            write(&format!("schemas/{version}/device.schema.json"), &schema.to_string());
            write(&format!("{version}.json"), &serde_json::json!({ version: true }).to_string());
        }
        write("schemas/vocabulary/1/terms.json", "{}\n");
        write("none.list", "# no candidate\n");
        write("eleven.list", "0.11\n");
        Fixture(dir)
    }

    fn path(&self, rel: &str) -> String {
        self.0.path().join(rel).to_str().unwrap().to_string()
    }

    /// Asserts that the script, with these options, checks against `version` and no other.
    fn assert_checks_against(&self, version: &str, list: Option<&str>, asked: Option<&str>) {
        for other in VERSIONS {
            let descriptor = self.path(&format!("{other}.json"));
            let list = list.map(|l| self.path(l));
            let mut args = vec!["device", descriptor.as_str()];
            if let Some(list) = &list {
                args.extend(["--candidates", list.as_str()]);
            }
            let run = python(&self.0.path().join("tools/validate.py"), &args, asked);
            let expected = if other == version { 0 } else { 1 };
            assert_eq!(run.code, Some(expected), "the descriptor of {other}, against {version}: {}", run.out);
            assert!(!run.out.contains("Traceback"), "{}", run.out);
        }
    }
}

#[test]
fn validate_checks_by_default_against_the_newest_version_not_listed_as_candidate() {
    Fixture::new().assert_checks_against("0.9", None, None);
}

#[test]
fn validate_takes_its_default_from_the_list_given_to_it() {
    Fixture::new().assert_checks_against("0.10", Some("eleven.list"), None);
}

#[test]
fn with_no_candidate_validate_checks_by_default_against_the_newest_version_by_number() {
    Fixture::new().assert_checks_against("0.11", Some("none.list"), None);
}

#[test]
fn osd_version_picks_any_version_of_validate_a_candidate_included() {
    let fixture = Fixture::new();
    for version in VERSIONS {
        fixture.assert_checks_against(version, None, Some(version));
    }
}

#[test]
fn validate_without_osd_version_checks_the_examples_of_0_6_of_the_repository() {
    if !lines().iter().any(|(version, _)| version == "0.7") {
        return;
    }
    let script = common::repo().join("tools/validate.py");
    let example = common::repo().join("examples/0.6/minilogue.device.json");
    let run = python(&script, &["device", example.to_str().unwrap()], None);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn the_help_of_validate_says_that_the_default_version_leaves_the_candidates_out() {
    let run = python(&common::repo().join("tools/validate.py"), &[], None);
    assert_ne!(run.code, Some(0), "{}", run.out);
    assert!(!run.out.contains("Traceback"), "{}", run.out);
    assert!(run.out.contains("OSD_VERSION") && run.out.to_lowercase().contains("candidate"), "{}", run.out);
}
