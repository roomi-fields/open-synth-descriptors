//! The cache entry of the tests of osd: every file or folder outside osd/ they read is covered
//! by a pattern of the `lit` list of osd/package.json, so a change of such a file alone replays
//! the tests.
//!
//! The reads are found, not listed. The guard lexes the sources of osd's tests (osd/tests/ and
//! the test modules under osd/src/) and requires each string literal that names an existing file
//! or folder of the repository outside osd/, a leading `../` removed; for a literal with a `{`,
//! the folder written before it. A folder that a line of .gitignore ending in `/` names is not
//! of the repository. A file is covered by its own pattern or by a `dir/**` above it;
//! a folder only by a `dir/**` of it or above it, since a test walks it or joins names to it.
//! A literal that names a path without reading it is set aside for the one source that writes it.
//! This file is not scanned: its literals are the data of the guard and of its fixtures.
//! It also requires what the scripts the tests launch read and no test source names, what this
//! file reads, and the targets of the relative links of README.md.
//!
//! The public tree holds the tests of osd and not osd/package.json: there is no list to judge
//! there, and the tests of the repository itself pass without judging.
//!
//! Its limit: a path taken from data other than the links of the README, or assembled from pieces
//! of which no literal names the path or its folder, is not seen.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

use crate::common;

const GUARD: &str = "osd/tests/it/lit_coverage.rs";

/// (source, path, why the literal is no read of the path).
const SET_ASIDE: [(&str, &str, &str); 5] = [
    ("osd/tests/it/resources.rs", "export", "the JSON key `export` of a resource"),
    ("osd/tests/it/file_write_steps.rs", "export", "the JSON key `export` of a resource"),
    ("osd/tests/it/readme.rs", "reference", "searched in the text of the README"),
    ("osd/tests/it/public_book.rs", "reference", "names pages of a built book and a word of a workflow step"),
    ("osd/tests/it/public_site.rs", "package.json", "in the list of files the site must not hold"),
];

/// (path, what reads it), for the paths read that no scanned source names: by a script the tests
/// launch, or by the guard itself.
const UNNAMED_READS: [(&str, &str); 3] = [
    // Without OSD_REFERENCE the script runs `cargo run -p reference`.
    ("reference/src", "tools/build_book.py"),
    ("reference/Cargo.toml", "tools/build_book.py"),
    (".gitignore", GUARD),
];

/// A path no pattern covers, and what reads it: a test source, a script, the guard, or the README.
type Refusal = (String, String);

/// `text` as a path from the root of the repository, outside osd/.
fn outside(text: &str) -> Option<&str> {
    let mut path = if text.starts_with("/../") { &text[1..] } else { text };
    while let Some(rest) = path.strip_prefix("../") {
        path = rest;
    }
    let path = path.trim_end_matches('/');
    let plain = path.split('/').all(|p| !matches!(p, "" | "." | ".."));
    (plain && path.split('/').next() != Some("osd")).then_some(path)
}

/// Whether a `dir/**` pattern of `path`, or of a folder above it, is in `lit`.
fn under_a_folder_pattern(lit: &[String], path: &str) -> bool {
    lit.iter()
        .filter_map(|pattern| pattern.strip_suffix("/**"))
        .any(|dir| path == dir || path.strip_prefix(dir).is_some_and(|rest| rest.starts_with('/')))
}

/// Whether `lit` covers the existing file or folder `path`.
fn covered(root: &Path, lit: &[String], path: &str) -> bool {
    let own = root.join(path).is_file() && lit.iter().any(|pattern| pattern == path);
    own || under_a_folder_pattern(lit, path)
}

/// Whether a folder line of .gitignore keeps `path` out of the repository: a line of one name
/// holds at any depth, a line of several from the root.
fn ignored(root: &Path, path: &str) -> bool {
    let text = std::fs::read_to_string(root.join(".gitignore")).unwrap_or_default();
    text.lines().filter_map(|line| line.trim().strip_suffix('/')).any(|folder| {
        let from_here = |at: &str| {
            let rest = at.strip_prefix(folder);
            rest.is_some_and(|rest| rest.starts_with('/') || rest.is_empty() && root.join(path).is_dir())
        };
        if folder.contains('/') {
            from_here(path)
        } else {
            from_here(path) || path.match_indices('/').any(|(at, _)| from_here(&path[at + 1..]))
        }
    })
}

/// The text between the quotes of a string literal, raw or not; none for any other literal.
fn string_of(literal: &str) -> Option<&str> {
    let body = literal.strip_prefix('r').map_or(literal, |raw| raw.trim_matches('#'));
    body.strip_prefix('"')?.strip_suffix('"')
}

/// The string literals of a token stream, the text of the documentation comments left out.
fn literals(stream: TokenStream, out: &mut Vec<String>) {
    for token in stream {
        match token {
            TokenTree::Group(group) => {
                let documentation = group.delimiter() == Delimiter::Bracket
                    && matches!(group.stream().into_iter().next(), Some(TokenTree::Ident(i)) if i == "doc");
                if !documentation {
                    literals(group.stream(), out);
                }
            }
            TokenTree::Literal(literal) => out.extend(string_of(&literal.to_string()).map(String::from)),
            _ => {}
        }
    }
}

/// Every `.rs` file under `dir`.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The sources of osd's tests, named from the root.
fn test_sources(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    rust_files(&root.join("osd/tests"), &mut files);
    let mut under_src = Vec::new();
    rust_files(&root.join("osd/src"), &mut under_src);
    let source = root.join("osd/src");
    files.extend(under_src.into_iter().filter(|file| {
        let inside = file.strip_prefix(&source).unwrap();
        inside.file_name().is_some_and(|n| n == "tests.rs") || inside.components().any(|c| c.as_os_str() == "tests")
    }));
    let mut names: Vec<String> =
        files.iter().map(|f| f.strip_prefix(root).unwrap().to_str().unwrap().to_string()).collect();
    names.sort();
    names
}

/// The paths the literals of the test sources name and `lit` does not cover.
fn uncovered_reads(root: &Path, lit: &[String]) -> Vec<Refusal> {
    let mut refused = BTreeSet::new();
    for source in test_sources(root).into_iter().filter(|s| s != GUARD) {
        let text = std::fs::read_to_string(root.join(&source)).unwrap();
        let stream = TokenStream::from_str(&text).unwrap_or_else(|e| panic!("{source}: {e}"));
        let mut found = Vec::new();
        literals(stream, &mut found);
        for literal in found {
            let named = match literal.split_once('{') {
                Some((before, _)) => before
                    .rsplit_once('/')
                    .and_then(|(folder, _)| outside(folder))
                    .filter(|folder| root.join(folder).is_dir()),
                None => outside(&literal).filter(|path| root.join(path).exists()),
            };
            let Some(path) = named else { continue };
            let aside = SET_ASIDE.iter().any(|(s, p, _)| *s == source && *p == path);
            if !aside && !ignored(root, path) && !covered(root, lit, path) {
                refused.insert((path.to_string(), source.clone()));
            }
        }
    }
    refused.into_iter().collect()
}

/// What is read that no scanned source names and `lit` does not cover.
fn uncovered_unnamed_reads(root: &Path, lit: &[String]) -> Vec<Refusal> {
    UNNAMED_READS
        .iter()
        .filter(|(path, _)| !covered(root, lit, path))
        .map(|(path, reader)| (path.to_string(), reader.to_string()))
        .collect()
}

/// The targets of the relative links of README.md that `lit` does not cover. Of a linked folder
/// only the existence is read: any pattern that covers it or a path under it is enough.
fn uncovered_readme_links(root: &Path, lit: &[String]) -> Vec<Refusal> {
    let text = std::fs::read_to_string(root.join("README.md")).unwrap();
    let mut refused = BTreeSet::new();
    for part in text.split("](").skip(1) {
        let target = part.split(')').next().unwrap();
        if target.starts_with("http") || target.starts_with('#') || target.starts_with("mailto:") {
            continue;
        }
        let Some(path) = outside(target.split('#').next().unwrap()) else { continue };
        let below = format!("{path}/");
        let passes = covered(root, lit, path)
            || root.join(path).is_dir() && lit.iter().any(|pattern| pattern.starts_with(&below));
        if !passes {
            refused.insert((path.to_string(), "README.md".to_string()));
        }
    }
    refused.into_iter().collect()
}

/// The key of the manifest section that holds the list, assembled so that this published source
/// does not spell it.
const SECTION: &str = concat!("grill", "house");

/// The `lit` patterns of the manifest of osd under `root`; none in a tree that holds no such
/// manifest.
fn lit_of_osd(root: &Path) -> Option<Vec<String>> {
    let text = match std::fs::read_to_string(root.join("osd/package.json")) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => panic!("osd/package.json: {error}"),
    };
    let package: serde_json::Value = serde_json::from_str(&text).unwrap();
    let patterns = package[SECTION]["lit"].as_array().expect("the `lit` list of osd/package.json");
    Some(patterns.iter().map(|v| v.as_str().unwrap().to_string()).collect())
}

/// Judges the repository itself by `guard`, where it holds the list.
fn judge_the_repository(guard: fn(&Path, &[String]) -> Vec<Refusal>) {
    let root = common::repo();
    if let Some(lit) = lit_of_osd(&root) {
        assert_all_covered(guard(&root, &lit));
    }
}

fn assert_all_covered(refused: Vec<Refusal>) {
    let lines: Vec<String> = refused.iter().map(|(path, by)| format!("{path}, read by {by}")).collect();
    assert!(lines.is_empty(), "no `lit` pattern of osd/package.json covers:\n{}", lines.join("\n"));
}

#[test]
fn every_path_outside_osd_a_test_source_names_is_covered_by_a_lit_pattern_of_osd() {
    judge_the_repository(uncovered_reads);
}

#[test]
fn what_the_launched_scripts_and_the_guard_read_and_no_source_names_is_covered_by_a_lit_pattern_of_osd() {
    judge_the_repository(uncovered_unnamed_reads);
}

#[test]
fn every_relative_link_of_the_readme_is_covered_by_a_lit_pattern_of_osd() {
    judge_the_repository(uncovered_readme_links);
}

// ---- The guard, on a fixture repository ----

#[test]
fn a_tree_without_the_manifest_of_osd_holds_no_list_to_judge() {
    let root = fixture(&[("osd/tests/it/other.rs", "")]);
    assert_eq!(lit_of_osd(root.path()), None);
}

#[test]
fn the_list_is_read_from_the_manifest_of_osd() {
    let manifest = format!(r#"{{"{SECTION}": {{"lit": ["top.md", "data/**"]}}}}"#);
    let root = fixture(&[("osd/package.json", &manifest)]);
    assert_eq!(lit_of_osd(root.path()), Some(lit(&["top.md", "data/**"])));
}

/// A repository holding `files`, each with its text.
fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

/// A test source that writes each of `literals`.
fn source_naming(literals: &[&str]) -> String {
    format!("fn read() {{ let _ = {literals:?}; }}\n")
}

fn lit(patterns: &[&str]) -> Vec<String> {
    patterns.iter().map(|p| p.to_string()).collect()
}

fn refusals(pairs: &[(&str, &str)]) -> Vec<Refusal> {
    pairs.iter().map(|(path, by)| (path.to_string(), by.to_string())).collect()
}

const SOURCE: &str = "osd/tests/it/other.rs";

#[test]
fn an_uncovered_file_a_test_source_names_is_refused_with_its_path_and_the_source() {
    let text = source_naming(&["notes/a.md", "../top.md", "../../deep/er/b.json", "/../c.md"]);
    let root = fixture(&[
        (SOURCE, &text),
        ("notes/a.md", ""),
        ("top.md", ""),
        ("deep/er/b.json", ""),
        ("c.md", ""),
    ]);
    // A pattern of a folder's name, or of a neighbour, covers no file under it.
    let patterns = lit(&["notes", "top.md.bak", "deep/e/**"]);
    assert_eq!(
        uncovered_reads(root.path(), &patterns),
        refusals(&[("c.md", SOURCE), ("deep/er/b.json", SOURCE), ("notes/a.md", SOURCE), ("top.md", SOURCE)])
    );
}

#[test]
fn an_uncovered_folder_a_test_source_names_is_refused_with_its_path_and_the_source() {
    let text = source_naming(&["data", "../shared/deep/", "built/{name}.json", "pages/old/{}"]);
    let root = fixture(&[
        (SOURCE, &text),
        ("data/a.json", ""),
        ("shared/deep/a.json", ""),
        ("built/a.json", ""),
        ("pages/old/a.html", ""),
    ]);
    // Neither the folder's own name nor patterns of what it holds cover it.
    let patterns = lit(&["data", "data/a.json", "shared/deep/a.json", "built/a.json", "pages/old/new/**"]);
    assert_eq!(
        uncovered_reads(root.path(), &patterns),
        refusals(&[("built", SOURCE), ("data", SOURCE), ("pages/old", SOURCE), ("shared/deep", SOURCE)])
    );
}

#[test]
fn a_file_with_its_own_pattern_or_a_folder_pattern_above_it_and_a_folder_with_one_of_it_or_above_pass() {
    let text = source_naming(&["top.md", "data/a/b.json", "data/a", "other", "other/{x}.json", "more/c.md"]);
    let root = fixture(&[
        (SOURCE, &text),
        ("top.md", ""),
        ("data/a/b.json", ""),
        ("other/x.json", ""),
        ("more/c.md", ""),
    ]);
    let patterns = lit(&["top.md", "data/**", "other/**", "more/**"]);
    assert_eq!(uncovered_reads(root.path(), &patterns), refusals(&[]));
}

#[test]
fn the_test_modules_under_src_and_every_source_under_tests_are_scanned_and_the_code_of_src_is_not() {
    let root = fixture(&[
        ("osd/src/lib.rs", &source_naming(&["code.md"])),
        ("osd/src/part/tests.rs", &source_naming(&["unit.md"])),
        ("osd/src/part/tests/deeper.rs", &source_naming(&["nested.md"])),
        ("osd/tests/common/mod.rs", &source_naming(&["shared.md"])),
        ("osd/tests/it_budget/main.rs", &source_naming(&["bench.md"])),
        ("code.md", ""),
        ("unit.md", ""),
        ("nested.md", ""),
        ("shared.md", ""),
        ("bench.md", ""),
    ]);
    assert_eq!(
        uncovered_reads(root.path(), &lit(&[])),
        refusals(&[
            ("bench.md", "osd/tests/it_budget/main.rs"),
            ("nested.md", "osd/src/part/tests/deeper.rs"),
            ("shared.md", "osd/tests/common/mod.rs"),
            ("unit.md", "osd/src/part/tests.rs"),
        ])
    );
}

#[test]
fn a_raw_string_and_a_literal_inside_a_macro_are_reads_and_a_comment_is_not() {
    let text = "//! \"doc.md\"\n/// doc.md\n// \"line.md\"\n\
                fn read() { assert_eq!(vec![r\"raw.md\", r#\"hashed.md\"#], [include_str!(\"../../../in.md\")]); }\n";
    let root = fixture(&[
        (SOURCE, text),
        ("doc.md", ""),
        (" doc.md", ""),
        ("line.md", ""),
        ("raw.md", ""),
        ("hashed.md", ""),
        ("in.md", ""),
    ]);
    assert_eq!(
        uncovered_reads(root.path(), &lit(&[])),
        refusals(&[("hashed.md", SOURCE), ("in.md", SOURCE), ("raw.md", SOURCE)])
    );
}

#[test]
fn a_literal_that_names_nothing_existing_outside_osd_requires_nothing() {
    let text = source_naming(&["", ".", "..", "../", "/data", "absent.md", "gone/{x}", "{root}/data", "osd", "osd/docs/a.md", "../osd/docs", "dat"]);
    let root = fixture(&[(SOURCE, &text), ("data/a.json", ""), ("osd/docs/a.md", "")]);
    assert_eq!(uncovered_reads(root.path(), &lit(&[])), refusals(&[]));
}

#[test]
fn a_literal_set_aside_for_one_source_is_refused_when_another_source_names_it() {
    let aside = [
        ("osd/tests/it/resources.rs", "export"),
        ("osd/tests/it/file_write_steps.rs", "export"),
        ("osd/tests/it/readme.rs", "reference/"),
        ("osd/tests/it/public_book.rs", "reference/{kind}.html"),
        ("osd/tests/it/public_site.rs", "package.json"),
    ];
    let mut files: Vec<(&str, String)> = aside.iter().map(|(s, l)| (*s, source_naming(&[l]))).collect();
    files.push((SOURCE, source_naming(&["export", "reference/", "package.json"])));
    // Set aside for the literal it writes, a source still requires every other path.
    files[0].1 = source_naming(&["export", "package.json"]);
    files.push(("export/public.list", String::new()));
    files.push(("reference/a.md", String::new()));
    files.push(("package.json", String::new()));
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
    let root = fixture(&files);
    assert_eq!(
        uncovered_reads(root.path(), &lit(&[])),
        refusals(&[
            ("export", SOURCE),
            ("package.json", SOURCE),
            ("package.json", "osd/tests/it/resources.rs"),
            ("reference", SOURCE),
        ])
    );
}

#[test]
fn a_folder_that_gitignore_names_is_not_of_the_repository_and_requires_nothing() {
    let text = source_naming(&["target", "target/debug/x", "pages/node_modules", "doc/built/a", "built/a", "targets", "pages/keep"]);
    let root = fixture(&[
        (SOURCE, &text),
        (".gitignore", "# built\ntarget/\nnode_modules/\ndoc/built/\n"),
        ("target/debug/x", ""),
        ("pages/node_modules/a", ""),
        ("pages/keep", ""),
        ("doc/built/a", ""),
        ("built/a", ""),
        ("targets", ""),
    ]);
    assert_eq!(
        uncovered_reads(root.path(), &lit(&[])),
        refusals(&[("built/a", SOURCE), ("pages/keep", SOURCE), ("targets", SOURCE)])
    );
}

#[test]
fn what_no_source_names_is_refused_with_its_reader_until_a_pattern_covers_it() {
    let root = fixture(&[("reference/src/lib.rs", ""), ("reference/Cargo.toml", ""), (".gitignore", "")]);
    assert_eq!(
        uncovered_unnamed_reads(root.path(), &lit(&["reference/src/lib.rs"])),
        refusals(&[
            ("reference/src", "tools/build_book.py"),
            ("reference/Cargo.toml", "tools/build_book.py"),
            (".gitignore", GUARD),
        ])
    );
    let patterns = lit(&["reference/src/**", "reference/Cargo.toml", ".gitignore"]);
    assert_eq!(uncovered_unnamed_reads(root.path(), &patterns), refusals(&[]));
}

#[test]
fn a_readme_link_to_an_uncovered_file_or_folder_is_refused_by_name() {
    let readme = "[a](GUIDE.md) [b](docs/deep.md#part) [c](empty/) [d](osd/) [e](#here) \
                  [f](https://example.org/x.md) [g](mailto:a@example.org)\n";
    let root = fixture(&[
        ("README.md", readme),
        ("GUIDE.md", ""),
        ("docs/deep.md", ""),
        ("empty/a", ""),
        ("osd/a", ""),
    ]);
    // A path under a linked file, or a neighbour of a linked folder, covers neither.
    let patterns = lit(&["GUIDE.md/x", "docs/other.md", "empty-not/**"]);
    assert_eq!(
        uncovered_readme_links(root.path(), &patterns),
        refusals(&[("GUIDE.md", "README.md"), ("docs/deep.md", "README.md"), ("empty", "README.md")])
    );
}

#[test]
fn a_linked_folder_passes_when_a_pattern_covers_it_or_a_path_under_it() {
    let root = fixture(&[
        ("README.md", "[a](one/) [b](two) [c](three/in) [d](four/) [e](file.md)\n"),
        ("one/a", ""),
        ("two/a/b", ""),
        ("three/in/a", ""),
        ("four/a", ""),
        ("file.md", ""),
    ]);
    let patterns = lit(&["one/a", "two/a/**", "three/**", "four/**", "file.md"]);
    assert_eq!(uncovered_readme_links(root.path(), &patterns), refusals(&[]));
}
