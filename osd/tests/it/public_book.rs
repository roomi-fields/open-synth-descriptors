//! The book of the specification: the pinned tool and its installer, the build that assembles the
//! book from spec/ and the generated reference, the link check, and the workflows that call them.
//! Medium: the scripts run as processes on fixtures and on the real spec/.

use crate::common;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const PIN: &str = "tools/mdbook.pin";
const INSTALL: &str = "tools/install_mdbook.py";
const BUILD_BOOK: &str = "tools/build_book.py";
const CHECK_BOOK: &str = "tools/check_book.py";
const BUILD_SITE: &str = "tools/build_site.py";
const PAGES: &str = ".github/workflows/pages.yml";
const CHECK: &str = ".github/workflows/check.yml";
const REPO: &str = "https://github.com/roomi-fields/open-synth-descriptors";

struct Run {
    code: Option<i32>,
    out: String,
    stdout: String,
}

fn python(script: &str, args: &[&str]) -> Run {
    let script = common::repo().join(script);
    let mut last = None;
    for interpreter in ["python3", "python"] {
        match Command::new(interpreter).arg(&script).args(args).output() {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                return Run {
                    code: o.status.code(),
                    out: format!("{stdout}{}", String::from_utf8_lossy(&o.stderr)),
                    stdout,
                };
            }
            Err(e) => last = Some(e),
        }
    }
    panic!("no python interpreter: {}", last.unwrap());
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(common::repo().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
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

// ---- The pin and the installer ----

fn sha256_of(file: &Path) -> String {
    let o = Command::new("sha256sum").arg(file).output().expect("sha256sum");
    String::from_utf8_lossy(&o.stdout).split_whitespace().next().unwrap().to_string()
}

/// A fake release archive holding one executable `mdbook`, in a fresh cache folder, and a pin
/// that names it by its release name at an address that cannot be fetched.
struct Fixture {
    _dir: TempDir,
    cache: PathBuf,
    pin: PathBuf,
    archive: PathBuf,
}

fn fixture(pinned_sha: Option<&str>) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let src = dir.path().join("src");
    write(&src, "mdbook", "#!/bin/sh\necho fixture-mdbook\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(src.join("mdbook"), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let name = "mdbook-v9.9.9-x86_64-unknown-linux-gnu.tar.gz";
    let archive = cache.join(name);
    let status = Command::new("tar").arg("czf").arg(&archive).arg("-C").arg(&src).arg("mdbook").status().unwrap();
    assert!(status.success());
    let sha = pinned_sha.map(String::from).unwrap_or_else(|| sha256_of(&archive));
    let pin = dir.path().join("mdbook.pin");
    std::fs::write(
        &pin,
        format!("version = 9.9.9\nurl = http://127.0.0.1:1/releases/v9.9.9/{name}\nsha256 = {sha}\n"),
    )
    .unwrap();
    Fixture { _dir: dir, cache, pin, archive }
}

fn install(f: &Fixture) -> Run {
    python(INSTALL, &["--pin", f.pin.to_str().unwrap(), "--cache", f.cache.to_str().unwrap()])
}

#[test]
fn the_pin_holds_the_version_the_address_and_the_checksum_of_the_release_archive() {
    let pin = read(PIN);
    assert!(pin.contains("0.5.4"), "{pin}");
    assert!(
        pin.contains("https://github.com/rust-lang/mdBook/releases/download/v0.5.4/mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz"),
        "{pin}"
    );
    assert!(pin.contains("3f28de05dafca9d0f2eab99c662116b0e37b89b1d96a08f8f430b9eeae958cd7"), "{pin}");
}

#[test]
fn the_version_and_the_checksum_are_written_once_in_the_pin() {
    for rel in [INSTALL, BUILD_BOOK, CHECK_BOOK, PAGES, CHECK] {
        let text = read(rel);
        for literal in ["0.5.4", "3f28de05dafca9d0f2eab99c662116b0e37b89b1d96a08f8f430b9eeae958cd7", "mdbook-v0"] {
            assert!(!text.contains(literal), "{rel} holds the literal {literal}: it lives in {PIN}");
        }
    }
}

#[test]
fn an_archive_in_the_cache_that_matches_the_pin_is_used_without_any_download() {
    let f = fixture(None);
    let run = install(&f);
    assert_eq!(run.code, Some(0), "{}", run.out);
    let binary = PathBuf::from(run.stdout.lines().last().expect("the binary path").trim());
    assert!(binary.is_file(), "{binary:?} is not a file");
    assert_eq!(binary.file_name().unwrap(), "mdbook");
    let ran = Command::new(&binary).output().expect("the installed binary runs");
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "fixture-mdbook");
}

#[test]
fn an_archive_whose_checksum_differs_from_the_pin_is_refused_and_kept_as_found() {
    let f = fixture(Some(&"0".repeat(64)));
    let before = std::fs::read(&f.archive).unwrap();
    let run = install(&f);
    assert_ne!(run.code, Some(0), "{}", run.out);
    assert!(run.out.to_lowercase().contains("checksum") || run.out.contains("sha256"), "{}", run.out);
    assert!(run.out.contains(&"0".repeat(64)), "the pinned checksum is named: {}", run.out);
    assert_eq!(std::fs::read(&f.archive).unwrap(), before, "the archive is replaced silently");
}

#[test]
fn a_tool_that_cannot_be_installed_fails_and_is_never_skipped() {
    let f = fixture(None);
    std::fs::remove_file(&f.archive).unwrap();
    let run = install(&f);
    assert_ne!(run.code, Some(0), "{}", run.out);
    assert!(!run.stdout.lines().any(|l| l.trim().ends_with("mdbook")), "a binary path is printed: {}", run.stdout);
}

#[test]
fn the_book_build_fails_when_its_tool_cannot_be_installed_and_writes_no_book() {
    let f = fixture(None);
    std::fs::remove_file(&f.archive).unwrap();
    let out = tempfile::tempdir().unwrap();
    let book = out.path().join("book");
    let run = python(
        BUILD_BOOK,
        &[book.to_str().unwrap(), "--pin", f.pin.to_str().unwrap(), "--cache", f.cache.to_str().unwrap()],
    );
    assert_ne!(run.code, Some(0), "{}", run.out);
    assert!(!book.join("index.html").exists(), "a book is written without its tool");
}

// ---- The link check, on fixtures ----

const GOOD_LIST: &str = "# a fixture list\nCONTRIBUTING.md\nexamples/\nschemas/\nspec/\n";

struct Book {
    dir: TempDir,
}

impl Book {
    fn new(pages: &[(&str, &str)]) -> Book {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in pages {
            write(&dir.path().join("book"), rel, text);
        }
        write(&dir.path().join("site"), "schemas/0.6/device.schema.json", "{}");
        write(dir.path(), "public.list", GOOD_LIST);
        Book { dir }
    }

    fn check(&self) -> Run {
        let root = self.dir.path();
        python(
            CHECK_BOOK,
            &[
                root.join("book").to_str().unwrap(),
                "--site",
                root.join("site").to_str().unwrap(),
                "--public-list",
                root.join("public.list").to_str().unwrap(),
            ],
        )
    }
}

fn page(links: &[&str]) -> String {
    let body: String = links.iter().map(|l| format!("<a href=\"{l}\">x</a>\n")).collect();
    format!("<html><body><h1 id=\"top\">T</h1>\n{body}</body></html>")
}

const SECOND: &str = "<html><body><h2 id=\"part\">P</h2></body></html>";

#[test]
fn a_book_whose_links_all_resolve_passes_the_link_check() {
    let book = Book::new(&[
        (
            "index.html",
            &page(&[
                "second.html",
                "second.html#part",
                "#top",
                "../schemas/0.6/device.schema.json",
                &format!("{REPO}/blob/main/CONTRIBUTING.md"),
                &format!("{REPO}/tree/main/examples/0.6"),
                "https://example.org/elsewhere",
            ]),
        ),
        ("second.html", SECOND),
    ]);
    let run = book.check();
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn a_link_to_a_page_the_book_does_not_hold_fails_naming_the_page_and_the_target() {
    let book = Book::new(&[("index.html", &page(&["missing-page.html"])), ("second.html", SECOND)]);
    let run = book.check();
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("index.html") && run.out.contains("missing-page.html"), "{}", run.out);
}

#[test]
fn an_anchor_that_no_heading_of_the_page_carries_fails_naming_the_page_and_the_target() {
    let book = Book::new(&[("index.html", &page(&["second.html#nowhere"])), ("second.html", SECOND)]);
    let run = book.check();
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("index.html") && run.out.contains("second.html#nowhere"), "{}", run.out);
}

#[test]
fn a_link_to_a_schema_the_site_does_not_hold_fails_naming_the_page_and_the_target() {
    let book = Book::new(&[("index.html", &page(&["../schemas/0.6/nope.schema.json"])), ("second.html", SECOND)]);
    let run = book.check();
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("index.html") && run.out.contains("nope.schema.json"), "{}", run.out);
}

#[test]
fn a_link_to_a_workshop_only_file_of_the_repository_fails_naming_the_page_and_the_target() {
    let charter = concat!("CLAUDE", ".md");
    let target = format!("{REPO}/blob/main/{charter}");
    let book = Book::new(&[("index.html", &page(&[&target])), ("second.html", SECOND)]);
    let run = book.check();
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("index.html") && run.out.contains(charter), "{}", run.out);
}

// ---- The real book ----

/// The site folder and the book folder, side by side in a folder the test removes.
fn built() -> (TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let site = dir.path().join("site");
    let book = dir.path().join("book");
    let run = python(BUILD_SITE, &[site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_SITE}: {}", run.out);
    let before = files_under(&site);
    let run = python(BUILD_BOOK, &[book.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_BOOK} failed:\n{}", run.out);
    assert_eq!(files_under(&site), before, "the book build wrote into the site folder");
    (dir, site, book)
}

fn text_of(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'")
}

fn hrefs(html: &str) -> Vec<String> {
    html.split("href=\"").skip(1).map(|s| s.split('"').next().unwrap().replace("&amp;", "&")).collect()
}

#[test]
fn the_book_holds_the_text_of_the_spec_one_page_per_text_and_one_reference_page_per_kind() {
    let (_dir, _site, book) = built();
    let page = std::fs::read_to_string(book.join("0.6.html")).expect("the page of 0.6");
    let text = text_of(&page);
    let spec = read("spec/0.6.md");
    let mut in_code = false;
    let mut headings = 0;
    for line in spec.lines() {
        if line.starts_with("```") {
            in_code = !in_code;
        }
        if in_code || !line.starts_with('#') {
            continue;
        }
        let heading = line.trim_start_matches('#').trim().replace('`', "");
        headings += 1;
        assert!(text.contains(&heading), "the page of 0.6 lacks the heading {heading:?}");
    }
    assert!(headings > 20, "{headings} headings read");
    let sentence = "The suite checks itself: the examples and the inputs of every conformance case";
    assert!(spec.contains(sentence));
    assert!(text.contains(sentence), "the page of 0.6 lacks {sentence:?}");
    for rel in ["PROCESS.html", "ERRATA.html", "CHANGELOG.html", "vocabulary/1.html"] {
        assert!(book.join(rel).is_file(), "{rel} is not a page of the book");
    }
    for kind in ["device", "profile", "sound", "fingerprint", "firmware", "assembly"] {
        let rel = format!("reference/{kind}.html");
        assert!(book.join(&rel).is_file(), "{rel} is not a page of the book");
    }
}

#[test]
fn links_of_the_spec_to_schemas_and_to_repository_files_are_rendered_at_their_addresses() {
    let (_dir, _site, book) = built();
    let links = hrefs(&std::fs::read_to_string(book.join("0.6.html")).unwrap());
    let schema = links.iter().find(|l| l.ends_with("schemas/0.6/device.schema.json")).expect("the device schema link");
    assert!(!schema.starts_with("http"), "a schema link leaves the site: {schema}");
    for tail in ["schemas/extensions.md", "conformance/README.md", "CONTRIBUTING.md"] {
        let want = format!("{REPO}/blob/main/{tail}");
        assert!(links.contains(&want), "no link {want} in {links:?}");
    }
    assert!(
        links.iter().any(|l| l.starts_with(&format!("{REPO}/")) && l.ends_with("main/examples/0.6")),
        "no link to examples/0.6 in the repository"
    );
    let spec = read("spec/0.6.md");
    assert!(spec.contains("](../schemas/extensions.md)"), "the spec text is rewritten");
}

#[test]
fn the_link_check_is_green_on_the_real_book_and_the_site_stays_as_it_was() {
    let (_dir, site, book) = built();
    let list = common::repo().join("export/public.list");
    let mut args = vec![book.to_str().unwrap().to_string(), "--site".into(), site.to_str().unwrap().to_string()];
    if list.is_file() {
        args.push("--public-list".into());
        args.push(list.to_str().unwrap().to_string());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let run = python(CHECK_BOOK, &refs);
    assert_eq!(run.code, Some(0), "{}", run.out);
    let run = python("tools/check_site.py", &["--site", site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

/// The site folder with the book written under its `book/` folder, as the pages workflow uploads it.
type SiteFiles = Vec<(String, Vec<u8>)>;

fn built_with_book() -> (TempDir, PathBuf, BTreeSet<String>, SiteFiles) {
    let dir = tempfile::tempdir().unwrap();
    let site = dir.path().join("site");
    let run = python(BUILD_SITE, &[site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_SITE}: {}", run.out);
    let before = files_under(&site);
    let bytes: SiteFiles =
        before.iter().map(|f| (f.clone(), std::fs::read(site.join(f)).unwrap())).collect();
    let run = python(BUILD_BOOK, &[site.join("book").to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_BOOK} failed:\n{}", run.out);
    (dir, site, before, bytes)
}

#[test]
fn the_site_that_holds_the_book_keeps_every_file_of_the_site_built_alone_at_its_path_with_its_bytes() {
    let (_dir, site, before, bytes) = built_with_book();
    let after = files_under(&site);
    assert!(before.is_subset(&after));
    for (rel, old) in bytes {
        assert_eq!(std::fs::read(site.join(&rel)).unwrap(), old, "{rel} changed");
    }
    assert!(after.contains("book/index.html"), "no book under book/: {after:?}");
    assert!(before.contains("schemas/SHA256SUMS") && before.contains("schemas/extensions.md"));
    assert!(after.iter().filter(|f| !before.contains(*f)).all(|f| f.starts_with("book/")));
}

#[test]
fn the_link_check_and_the_site_check_are_green_on_the_book_written_under_the_site() {
    let (_dir, site, _before, _bytes) = built_with_book();
    let book = site.join("book");
    let list = common::repo().join("export/public.list");
    let mut args = vec![book.to_str().unwrap(), "--site", site.to_str().unwrap()];
    if list.is_file() {
        args.push("--public-list");
        args.push(list.to_str().unwrap());
    }
    let run = python(CHECK_BOOK, &args);
    assert_eq!(run.code, Some(0), "{}", run.out);
    let run = python("tools/check_site.py", &["--site", site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn the_links_of_the_book_to_a_schema_resolve_to_the_schema_path_in_the_site() {
    let (_dir, site, _before, _bytes) = built_with_book();
    let spec = std::fs::read_to_string(site.join("book/0.6.html")).unwrap();
    let mut seen = 0;
    for href in hrefs(&spec).iter().filter(|h| h.contains("schemas/") && !h.starts_with("http")) {
        let path = href.split('#').next().unwrap();
        let resolved = site.join("book").join(path);
        assert!(resolved.canonicalize().map(|p| p.is_file()).unwrap_or(false), "{href} resolves to no file");
        assert!(resolved.canonicalize().unwrap().starts_with(site.canonicalize().unwrap().join("schemas")));
        seen += 1;
    }
    assert!(seen > 0, "no schema link in the book");
}

#[test]
fn the_home_page_links_the_book_relatively_and_the_readme_links_it_by_the_site_address() {
    let dir = tempfile::tempdir().unwrap();
    let site = dir.path().join("site");
    let run = python(BUILD_SITE, &[site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_SITE}: {}", run.out);
    let home = std::fs::read_to_string(site.join("index.html")).unwrap();
    assert!(home.contains("href=\"book/index.html\""), "home page: {home}");
    assert!(
        read("README.md").contains("https://roomi-fields.github.io/open-synth-descriptors/book/"),
        "README does not link the book"
    );
}

// ---- The workflows ----

fn run_lines(rel: &str) -> Vec<String> {
    let wf: serde_yaml_ng::Value = serde_yaml_ng::from_str(&read(rel)).unwrap();
    let mut out = Vec::new();
    for job in wf["jobs"].as_mapping().unwrap().values() {
        for step in job.get("steps").and_then(|s| s.as_sequence()).cloned().unwrap_or_default() {
            if let Some(run) = step.get("run").and_then(|r| r.as_str()) {
                out.extend(run.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from));
            }
        }
    }
    out
}

fn position(lines: &[String], needle: &str) -> usize {
    lines.iter().position(|l| l.contains(needle)).unwrap_or_else(|| panic!("no step runs {needle}: {lines:?}"))
}

#[test]
fn the_pages_and_check_workflows_build_the_reference_install_the_tool_build_the_book_and_check_its_links() {
    for rel in [PAGES, CHECK] {
        let lines = run_lines(rel);
        let cargo = lines
            .iter()
            .position(|l| l.starts_with("cargo") && l.contains("reference"))
            .unwrap_or_else(|| panic!("{rel}: no cargo step builds the reference: {lines:?}"));
        let install = position(&lines, INSTALL);
        let build = position(&lines, BUILD_BOOK);
        let check = position(&lines, CHECK_BOOK);
        assert!(cargo < build && install < build && build < check, "{rel}: order {lines:?}");
    }
}

#[test]
fn the_pages_workflow_uploads_the_site_folder_that_holds_the_book_under_book_and_checks_it_there() {
    let wf: serde_yaml_ng::Value = serde_yaml_ng::from_str(&read(PAGES)).unwrap();
    let upload = wf["jobs"]["build"]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|s| s.get("uses").and_then(|u| u.as_str()).is_some_and(|u| u.starts_with("actions/upload-pages-artifact")))
        .expect("an upload step");
    assert_eq!(upload["with"]["path"].as_str(), Some("_site"));
    let lines = run_lines(PAGES);
    let build = &lines[position(&lines, BUILD_BOOK)];
    assert!(build.split_whitespace().any(|w| w == "_site/book"), "the book is not written to _site/book: {build}");
    let site = position(&lines, BUILD_SITE);
    assert!(site < position(&lines, BUILD_BOOK), "the site is built before the book is written into it");
    let check = &lines[position(&lines, CHECK_BOOK)];
    assert!(check.split_whitespace().any(|w| w == "_site/book"), "the check reads another folder: {check}");
    assert!(check.contains("--site _site"), "{check}");
}
