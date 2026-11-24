//! The public site: the publication list, the build script that writes the site folder, the
//! verification script that asks a base address (a local static server in these tests), the pages
//! workflow and the public check workflow. Medium: the scripts run as processes and the folder is
//! served by a local static server.

use crate::common;
use serde_yaml_ng::Value;
use std::collections::BTreeSet;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use tempfile::TempDir;

const LIST: &str = "tools/site.list";
const BUILD: &str = "tools/build_site.py";
const CHECK: &str = "tools/check_site.py";
const PAGES: &str = ".github/workflows/pages.yml";
const CHECK_WORKFLOW: &str = ".github/workflows/check.yml";

struct Run {
    code: Option<i32>,
    out: String,
}

fn python(script: &str, args: &[&str]) -> Run {
    let script = common::repo().join(script);
    let mut last = None;
    for interpreter in ["python3", "python"] {
        match Command::new(interpreter).arg(&script).args(args).output() {
            Ok(o) => {
                return Run {
                    code: o.status.code(),
                    out: format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
                }
            }
            Err(e) => last = Some(e),
        }
    }
    panic!("no python interpreter: {}", last.unwrap());
}

/// The site folder the build script writes, inside a folder the test removes.
fn built() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let site = dir.path().join("site");
    let run = python(BUILD, &[site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD} failed:\n{}", run.out);
    (dir, site)
}

fn files_under(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else {
                out.insert(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out
}

/// Every `$id` of the published schemas, as `(id, path after the host's /osd/)`: the w3id rule
/// maps `https://w3id.org/osd/<version>/<name>` to `<Pages root>/schemas/<version>/<name>`.
fn ids() -> Vec<(String, String)> {
    let schemas = common::repo().join("schemas");
    let mut out = Vec::new();
    for rel in files_under(&schemas) {
        if !rel.ends_with(".json") {
            continue;
        }
        let text = std::fs::read_to_string(schemas.join(&rel)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        if let Some(id) = value.get("$id").and_then(|v| v.as_str()) {
            let tail = id.split_once("/osd/").unwrap_or_else(|| panic!("{rel}: $id {id}")).1;
            out.push((id.to_string(), format!("schemas/{tail}")));
        }
    }
    assert!(out.len() >= 30, "expected the $id of every version, got {}", out.len());
    out
}

struct Server {
    child: Child,
    base: String,
}

impl Server {
    fn serve(dir: &Path) -> Server {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut child = None;
        for interpreter in ["python3", "python"] {
            if let Ok(c) = Command::new(interpreter)
                .args(["-m", "http.server", &port.to_string(), "--bind", "127.0.0.1", "--directory"])
                .arg(dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                child = Some(c);
                break;
            }
        }
        let server = Server { child: child.expect("a python interpreter"), base: format!("http://127.0.0.1:{port}/") };
        for _ in 0..100 {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return server;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("the local server did not start");
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---- The publication list and the build script ----

#[test]
fn publication_list_is_one_file_that_names_the_schemas_and_the_home_page_source() {
    let text = std::fs::read_to_string(common::repo().join(LIST)).unwrap_or_else(|e| panic!("{LIST}: {e}"));
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
    assert!(lines.iter().any(|l| l.starts_with("schemas/")), "the list names schemas/");
    assert!(lines.iter().any(|l| l.contains("index.html")), "the list names the home page source");
    assert!(lines.len() <= 3, "the list names only schemas/ and the home page: {lines:?}");
}

#[test]
fn site_folder_holds_every_schema_file_under_its_version_and_nothing_outside_the_list() {
    let (_dir, site) = built();
    let published = files_under(&site);
    let schemas = files_under(&common::repo().join("schemas"));
    for rel in &schemas {
        assert!(published.contains(&format!("schemas/{rel}")), "schemas/{rel} missing from the site folder");
    }
    for rel in &published {
        assert!(rel == "index.html" || rel.starts_with("schemas/"), "{rel} is outside the list");
    }
    assert!(published.contains("schemas/0.6/device.schema.json"));
    assert!(published.contains("schemas/vocabulary/1/terms.json"));
    assert!(published.contains("schemas/extensions.md"));
    assert!(published.contains("index.html"));
    for private in [concat!("CLAUDE", ".md"), concat!(".be", "ads"), "docs", "osd", "package.json"] {
        assert!(!site.join(private).exists(), "{private} is in the site folder");
    }
}

#[test]
fn home_page_links_the_schemas_of_every_version() {
    let (_dir, site) = built();
    let page = std::fs::read_to_string(site.join("index.html")).unwrap();
    for (id, path) in ids() {
        assert!(page.contains(&path), "index.html does not link {path} ({id})");
    }
}

#[test]
fn build_is_the_same_twice() {
    let (_a, one) = built();
    let (_b, two) = built();
    let names = files_under(&one);
    assert_eq!(names, files_under(&two));
    for rel in names {
        assert_eq!(std::fs::read(one.join(&rel)).unwrap(), std::fs::read(two.join(&rel)).unwrap(), "{rel}");
    }
}

// ---- The verification script ----

#[test]
fn every_id_path_is_a_file_of_the_site_folder() {
    let (_dir, site) = built();
    let run = python(CHECK, &["--site", site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn a_missing_file_fails_naming_the_id_whose_file_is_missing() {
    let (_dir, site) = built();
    std::fs::remove_file(site.join("schemas/0.5/sound.schema.json")).unwrap();
    let run = python(CHECK, &["--site", site.to_str().unwrap()]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("https://w3id.org/osd/0.5/sound.schema.json"), "{}", run.out);
    assert!(!run.out.contains("https://w3id.org/osd/0.5/device.schema.json"), "{}", run.out);
}

#[test]
fn a_missing_file_of_an_old_version_names_its_id_too() {
    let (_dir, site) = built();
    std::fs::remove_file(site.join("schemas/0.4/firmware.schema.json")).unwrap();
    let run = python(CHECK, &["--site", site.to_str().unwrap()]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("https://synthhub.roomi-fields.com/osd/0.4/firmware.schema.json"), "{}", run.out);
}

#[test]
fn a_local_server_gives_200_on_the_home_page_and_every_id_path_and_404_on_the_workshop() {
    let (_dir, site) = built();
    let server = Server::serve(&site);
    let run = python(CHECK, &[&server.base]);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn a_served_workshop_file_fails_naming_its_path() {
    let (_dir, site) = built();
    let charter = concat!("CLAUDE", ".md");
    std::fs::write(site.join(charter), "charter").unwrap();
    let server = Server::serve(&site);
    let run = python(CHECK, &[&server.base]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains(charter), "{}", run.out);
}

#[test]
fn a_served_ticket_file_fails_naming_its_path() {
    let (_dir, site) = built();
    let store = concat!(".be", "ads");
    std::fs::create_dir_all(site.join(store)).unwrap();
    std::fs::write(site.join(store).join("interactions.jsonl"), "{}").unwrap();
    let server = Server::serve(&site);
    let run = python(CHECK, &[&server.base]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains(&format!("{store}/interactions.jsonl")), "{}", run.out);
}

#[test]
fn a_served_site_without_an_id_file_fails_naming_the_path() {
    let (_dir, site) = built();
    std::fs::remove_file(site.join("schemas/0.6/assembly.schema.json")).unwrap();
    let server = Server::serve(&site);
    let run = python(CHECK, &[&server.base]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("schemas/0.6/assembly.schema.json"), "{}", run.out);
}

#[test]
fn a_served_site_without_a_home_page_fails() {
    let (_dir, site) = built();
    std::fs::remove_file(site.join("index.html")).unwrap();
    let server = Server::serve(&site);
    let run = python(CHECK, &[&server.base]);
    assert_eq!(run.code, Some(1), "{}", run.out);
}

// ---- The workflows ----

fn read(rel: &str) -> String {
    std::fs::read_to_string(common::repo().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn all_steps(workflow: &Value) -> Vec<Value> {
    workflow
        .get("jobs")
        .and_then(|j| j.as_mapping())
        .expect("jobs:")
        .values()
        .flat_map(|job| job.get("steps").and_then(|s| s.as_sequence()).cloned().unwrap_or_default())
        .collect()
}

#[test]
fn pages_workflow_deploys_the_built_folder_on_main_only() {
    let text = read(PAGES);
    let wf: Value = serde_yaml_ng::from_str(&text).unwrap();
    let on = wf.get("on").or_else(|| wf.get(Value::Bool(true))).expect("on:");
    let branches: Vec<String> = on["push"]["branches"]
        .as_sequence()
        .expect("push.branches")
        .iter()
        .map(|b| b.as_str().unwrap().to_string())
        .collect();
    assert_eq!(branches, vec!["main"]);
    assert!(on.get("pull_request").is_none(), "a pull request does not deploy");
    let steps = all_steps(&wf);
    let uses: Vec<&str> = steps.iter().filter_map(|s| s.get("uses").and_then(|u| u.as_str())).collect();
    assert!(uses.iter().any(|u| u.starts_with("actions/upload-pages-artifact")), "{uses:?}");
    assert!(uses.iter().any(|u| u.starts_with("actions/deploy-pages")), "{uses:?}");
    assert!(!uses.iter().any(|u| u.starts_with("peaceiris/actions-gh-pages")), "no branch publication");
    let permissions = text.contains("pages: write") && text.contains("id-token: write");
    assert!(permissions, "deploy-pages needs pages: write and id-token: write");
}

#[test]
fn pages_workflow_builds_in_one_job_and_deploys_in_another_that_needs_it() {
    let wf: Value = serde_yaml_ng::from_str(&read(PAGES)).unwrap();
    let jobs = wf.get("jobs").and_then(|j| j.as_mapping()).expect("jobs:");
    let names: BTreeSet<&str> = jobs.keys().filter_map(|k| k.as_str()).collect();
    assert_eq!(names, BTreeSet::from(["build", "deploy"]));
    let uses_of = |job: &str| -> Vec<String> {
        wf["jobs"][job]["steps"]
            .as_sequence()
            .expect("steps")
            .iter()
            .filter_map(|s| s.get("uses").and_then(|u| u.as_str()).map(String::from))
            .collect()
    };
    let runs_build = wf["jobs"]["build"]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .any(|s| s.get("run").and_then(|r| r.as_str()).is_some_and(|r| r.contains(BUILD)));
    assert!(runs_build, "build runs {BUILD}");
    let build = uses_of("build");
    assert!(build.iter().any(|u| u.starts_with("actions/checkout")), "{build:?}");
    assert!(build.iter().any(|u| u.starts_with("actions/upload-pages-artifact")), "{build:?}");
    assert!(!build.iter().any(|u| u.starts_with("actions/deploy-pages")), "{build:?}");
    let deploy = uses_of("deploy");
    assert!(deploy.iter().any(|u| u.starts_with("actions/deploy-pages")), "{deploy:?}");
    assert!(!deploy.iter().any(|u| u.starts_with("actions/upload-pages-artifact")), "{deploy:?}");
    let needs = &wf["jobs"]["deploy"]["needs"];
    let needs_build = needs.as_str() == Some("build")
        || needs.as_sequence().is_some_and(|n| n.iter().any(|v| v.as_str() == Some("build")));
    assert!(needs_build, "deploy needs build: {needs:?}");
    let perms = &wf["jobs"]["deploy"]["permissions"];
    assert_eq!(perms["pages"].as_str(), Some("write"), "pages: write on deploy");
    assert_eq!(perms["id-token"].as_str(), Some("write"), "id-token: write on deploy");
    assert_eq!(wf["jobs"]["deploy"]["environment"]["name"].as_str(), Some("github-pages"));
    assert!(wf["jobs"]["build"].get("environment").is_none(), "the environment is on deploy");
}

#[test]
fn pages_workflow_calls_the_build_script_and_names_no_path_of_the_list() {
    let text = read(PAGES);
    assert!(text.contains(BUILD), "the workflow calls {BUILD}");
    for word in ["schemas", "index.html", "site.list", "CLAUDE", concat!(".be", "ads")] {
        assert!(!text.contains(word), "the workflow names {word}: the list is written once, in {LIST}");
    }
}

#[test]
fn build_and_verification_scripts_read_the_list_and_keep_no_copy_of_it() {
    for script in [BUILD, CHECK] {
        let text = read(script);
        assert!(text.contains("site.list"), "{script} reads the list");
        assert!(!text.contains("index.html"), "{script} keeps the home page name: it lives in the list");
    }
}

#[test]
fn public_check_runs_only_cargo_python_and_cargo_deny() {
    let text = read(CHECK_WORKFLOW);
    let wf: Value = serde_yaml_ng::from_str(&text).unwrap();
    let mut commands = Vec::new();
    for step in all_steps(&wf) {
        if let Some(run) = step.get("run").and_then(|r| r.as_str()) {
            for line in run.lines().map(str::trim).filter(|l| !l.is_empty()) {
                commands.push(line.to_string());
            }
        }
        let env = serde_yaml_ng::to_string(step.get("env").unwrap_or(&Value::Null)).unwrap();
        assert!(!env.contains("SYNTHHUB_DEVICES"), "a step sets SYNTHHUB_DEVICES");
    }
    assert!(!text.contains("SYNTHHUB_DEVICES"));
    for command in &commands {
        let first = command.split_whitespace().next().unwrap();
        assert!(
            ["cargo", "python", "python3", "pip"].contains(&first),
            "a step runs `{command}`: only cargo, python and cargo deny"
        );
        for banned in ["npm", "npx", concat!("grill", "house"), "bd "] {
            assert!(!command.contains(banned), "a step calls {banned}: `{command}`");
        }
    }
    let has = |needle: &str| commands.iter().any(|c| c.starts_with(needle));
    assert!(has("cargo test --workspace --lib --bins --test it"), "{commands:?}");
    assert!(has("cargo test --workspace --doc"), "{commands:?}");
    assert!(has("cargo clippy -p osd --all-targets -- -D warnings"), "{commands:?}");
    assert!(has("cargo build -p osd --target wasm32-unknown-unknown"), "{commands:?}");
    assert!(has("cargo deny check"), "{commands:?}");
}
