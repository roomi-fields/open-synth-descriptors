//! The release workflow: its triggers, the order of its jobs, what it
//! publishes and when, how it authenticates, what it builds, and the npm
//! package it publishes.

use crate::common;
use serde_yaml_ng::Value;
use std::collections::BTreeSet;

const WORKFLOW: &str = ".github/workflows/release.yml";

fn text() -> String {
    std::fs::read_to_string(common::repo().join(WORKFLOW)).unwrap_or_else(|e| panic!("{WORKFLOW}: {e}"))
}

fn workflow() -> Value {
    serde_yaml_ng::from_str(&text()).unwrap()
}

fn key<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value.get(name)
}

/// The `on:` key, which a YAML 1.1 reader may read as the boolean `true`.
fn triggers(wf: &Value) -> &Value {
    key(wf, "on").or_else(|| wf.get(Value::Bool(true))).expect("on:")
}

fn jobs(wf: &Value) -> Vec<(String, &Value)> {
    key(wf, "jobs")
        .and_then(|j| j.as_mapping())
        .expect("jobs:")
        .iter()
        .map(|(k, v)| (k.as_str().unwrap().to_string(), v))
        .collect()
}

fn needs(job: &Value) -> Vec<String> {
    match key(job, "needs") {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Sequence(seq)) => seq.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        _ => vec![],
    }
}

/// Every job the named one waits for, directly or through others.
fn ancestors(wf: &Value, name: &str) -> BTreeSet<String> {
    let all = jobs(wf);
    let mut seen = BTreeSet::new();
    let mut todo = vec![name.to_string()];
    while let Some(n) = todo.pop() {
        let Some((_, job)) = all.iter().find(|(k, _)| *k == n) else { panic!("job {n} unknown") };
        for p in needs(job) {
            if seen.insert(p.clone()) {
                todo.push(p);
            }
        }
    }
    seen
}

fn job_text(job: &Value) -> String {
    serde_yaml_ng::to_string(job).unwrap()
}

/// The jobs whose text contains `needle`.
fn with<'a>(wf: &'a Value, needle: &str) -> Vec<(String, &'a Value)> {
    jobs(wf).into_iter().filter(|(_, j)| job_text(j).contains(needle)).collect()
}

fn one<'a>(wf: &'a Value, needle: &str) -> (String, &'a Value) {
    let found = with(wf, needle);
    assert_eq!(found.len(), 1, "exactly one job holds `{needle}`: {:?}", found.iter().map(|f| &f.0).collect::<Vec<_>>());
    found.into_iter().next().unwrap()
}

/// A real (not dry-run) publication command in a job's steps.
fn publishes(job: &Value, command: &str) -> bool {
    job_text(job).lines().any(|l| l.contains(command) && !l.contains("--dry-run"))
}

fn publication_jobs(wf: &Value) -> Vec<(String, &Value)> {
    jobs(wf)
        .into_iter()
        .filter(|(_, j)| publishes(j, "cargo publish") || publishes(j, "npm publish") || job_text(j).contains("gh release upload"))
        .collect()
}

fn runs_only_on_release(job: &Value) -> bool {
    key(job, "if").and_then(|v| v.as_str()).is_some_and(|c| c.contains("github.event_name == 'release'"))
}

#[test]
fn the_workflow_starts_on_a_release_and_on_a_manual_dispatch_with_a_dry_run_input_true_by_default() {
    let wf = workflow();
    let on = triggers(&wf);
    assert!(key(on, "release").is_some(), "release trigger");
    let dispatch = key(on, "workflow_dispatch").expect("workflow_dispatch");
    let input = &key(dispatch, "inputs").expect("inputs")["dry-run"];
    assert!(
        input["default"] == Value::Bool(true) || input["default"].as_str() == Some("true"),
        "dry-run defaults to true"
    );
}

#[test]
fn the_publication_jobs_chain_checks_crates_binaries_npm_and_registry_install_checks_by_needs() {
    let wf = workflow();
    let (crates, _) = one(&wf, "cargo publish -p osd-cli");
    let (attach, _) = one(&wf, "gh release upload");
    let (npm, _) = one(&wf, "npm publish --access public");
    let checks_dry = with(&wf, "cargo publish --workspace --dry-run");
    assert!(!checks_dry.is_empty(), "cargo publish --workspace --dry-run");
    let (check, _) = &checks_dry[0];
    assert!(!with(&wf, "npm publish --dry-run").is_empty(), "npm publish --dry-run");

    assert!(ancestors(&wf, &crates).contains(check), "crates wait for the checks");
    assert!(ancestors(&wf, &attach).contains(&crates), "binaries wait for the crates");
    assert!(ancestors(&wf, &npm).contains(&crates), "npm waits for the crates");
    assert!(ancestors(&wf, &npm).contains(&attach), "npm waits for the binaries");

    let after: Vec<_> = jobs(&wf)
        .into_iter()
        .filter(|(n, _)| ancestors(&wf, n).contains(&npm))
        .collect();
    assert!(!after.is_empty(), "install checks run after the npm publication");
}

#[test]
fn the_crates_publish_in_dependency_order() {
    let wf = workflow();
    let (_, job) = one(&wf, "cargo publish -p osd-cli");
    let t = job_text(job);
    let at = |s: &str| t.find(s).unwrap_or_else(|| panic!("{s} missing"));
    assert!(at("cargo publish -p osd\n") < at("cargo publish -p osd-formats"));
    assert!(at("cargo publish -p osd-formats") < at("cargo publish -p osd-cli"));
}

#[test]
fn every_publication_job_runs_only_on_a_release_event_and_never_on_a_dry_run() {
    let wf = workflow();
    let publications = publication_jobs(&wf);
    assert!(publications.len() >= 3, "crates, binaries and npm publish");
    for (name, job) in &publications {
        assert!(runs_only_on_release(job), "{name} is reachable from a dry run");
    }
    let (npm, _) = one(&wf, "npm publish --access public");
    for (name, job) in jobs(&wf) {
        if ancestors(&wf, &name).contains(&npm) {
            assert!(runs_only_on_release(job), "{name} (after the publication) is reachable from a dry run");
        }
    }
}

#[test]
fn the_checks_run_the_dry_runs_and_install_checks_on_the_artefacts_before_any_publication() {
    let wf = workflow();
    let (crates, _) = one(&wf, "cargo publish -p osd-cli");
    let before: Vec<_> = jobs(&wf).into_iter().filter(|(n, _)| ancestors(&wf, &crates).contains(n)).collect();
    let text: String = before.iter().map(|(_, j)| job_text(j)).collect();
    assert!(text.contains("cargo build"), "builds the command");
    assert!(text.contains("cargo install --path"), "cargo install check on the artefact");
    assert!(text.contains("npm pack"), "packs the npm package");
    assert!(text.contains("npm install"), "npm install check on the packed package");
    assert!(text.matches("osd --version").count() >= 3, "one version check per way");
}

#[test]
fn the_install_checks_run_again_from_the_registries_after_the_publication() {
    let wf = workflow();
    let (npm, _) = one(&wf, "npm publish --access public");
    let after: Vec<_> = jobs(&wf).into_iter().filter(|(n, _)| ancestors(&wf, n).contains(&npm)).collect();
    let text: String = after.iter().map(|(_, j)| job_text(j)).collect();
    assert!(text.contains("cargo install osd-cli"), "cargo install from crates.io");
    assert!(text.contains("npm install -g @roomi-fields/osd"), "npm install from the registry");
    assert!(text.contains("releases/download") || text.contains("gh release download"), "binary from the release");
    assert!(text.matches("osd --version").count() >= 3, "one version check per way");
}

#[test]
fn the_publication_authenticates_by_trusted_publishing_and_reads_no_secret() {
    let wf = workflow();
    let t = text();
    assert!(!t.contains("secrets."), "a token is read from the secrets");
    assert!(!t.contains("NPM_TOKEN"));
    let top = key(&wf, "permissions");
    assert!(top.and_then(|p| p.get("id-token")).is_none(), "id-token at the top level");
    for (name, job) in jobs(&wf) {
        let writes = key(job, "permissions").and_then(|p| p.get("id-token")).and_then(|v| v.as_str()) == Some("write");
        let must = publishes(job, "cargo publish") || publishes(job, "npm publish");
        assert_eq!(writes, must, "{name}: id-token: write belongs to the crates and npm publication jobs only");
    }
}

#[test]
fn the_command_is_built_for_exactly_four_platforms_on_node_26() {
    let wf = workflow();
    let (_, job) = one(&wf, "cargo build");
    let include = key(job, "strategy").and_then(|s| s.get("matrix")).and_then(|m| m.get("include")).expect("matrix include");
    let targets: BTreeSet<String> = include
        .as_sequence()
        .unwrap()
        .iter()
        .map(|i| i["target"].as_str().unwrap().to_string())
        .collect();
    let expected: BTreeSet<String> = [
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(targets, expected);
    assert_eq!(include.as_sequence().unwrap().len(), 4);

    let t = text();
    assert!(t.contains("setup-node"));
    for line in t.lines().filter(|l| l.contains("node-version")) {
        assert!(line.contains("26"), "{line}");
    }
}

#[test]
fn each_binary_is_attached_to_the_release_with_one_sha256_checksum_file() {
    let wf = workflow();
    let (_, job) = one(&wf, "gh release upload");
    let t = job_text(job);
    assert!(t.contains("SHA256SUMS"), "one checksum file named SHA256SUMS");
    assert!(t.contains("sha256sum") || t.contains("shasum -a 256"), "the checksums are computed");
    assert!(t.matches("SHA256SUMS").count() >= 1);
}

#[test]
fn the_npm_version_is_set_from_the_tag_after_checking_it_equals_the_workspace_version() {
    let wf = workflow();
    let (_, job) = one(&wf, "npm publish --access public");
    let t = job_text(job);
    assert!(t.contains("npm version") || t.contains("npm pkg set version"), "sets the version of the package");
    let anywhere = with(&wf, "tag_name").into_iter().chain(with(&wf, "ref_name")).collect::<Vec<_>>();
    assert!(!anywhere.is_empty(), "reads the release tag");
    let checked: String = jobs(&wf).iter().map(|(_, j)| job_text(j)).collect();
    assert!(checked.contains("Cargo.toml"), "reads the workspace version");
    assert!(checked.contains("exit 1"), "refuses a tag that differs");
}

#[test]
fn the_release_workflow_is_declared_with_the_crates_it_exercises() {
    let text = std::fs::read_to_string(common::repo().join("package.json")).unwrap();
    let package: serde_json::Value = serde_json::from_str(&text).unwrap();
    let packages: Vec<&str> = package["grillhouse"]["flux"][WORKFLOW]
        .as_array()
        .expect("flux entry")
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for name in ["osd", "osd-formats", "osd-cli"] {
        assert!(packages.contains(&name), "{name}");
    }
}

fn npm_package() -> serde_json::Value {
    let path = common::repo().join("npm/package.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn the_npm_package_is_public_and_named_in_the_owners_scope_with_a_node_launcher() {
    let package = npm_package();
    assert_eq!(package["name"], "@roomi-fields/osd");
    assert_ne!(package["private"], serde_json::Value::Bool(true));
    let bin = package["bin"]["osd"].as_str().expect("bin.osd");
    assert!(common::repo().join("npm").join(bin).is_file(), "launcher {bin} missing");
    assert!(package["scripts"]["postinstall"].is_string(), "the binary is downloaded at install");
}
