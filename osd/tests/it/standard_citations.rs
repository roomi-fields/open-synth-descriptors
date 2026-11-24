//! The texts of the reader, the specification and its conformance cases cite the MIDI 2.0
//! controller resources at their version and tables: M2-117-UM v1.0, tables 6 and 12.

use std::path::Path;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

/// Published versions under checksum: their files never change.
const PUBLISHED: [&str; 5] = ["0.1", "0.2", "0.3", "0.4", "0.5"];

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name == "target" || name == "node_modules" || name == ".turbo" {
                continue;
            }
            if dir.ends_with("schemas") && PUBLISHED.contains(&name.as_str()) {
                continue;
            }
            collect(&path, out);
        } else if name != "standard_citations.rs" {
            out.push(path);
        }
    }
}

fn texts() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for dir in ["osd", "schemas", "docs", "conformance"] {
        collect(&Path::new(ROOT).join(dir), &mut files);
    }
    files
        .into_iter()
        .filter_map(|p| std::fs::read_to_string(&p).ok().map(|t| (p.display().to_string(), t)))
        .collect()
}

#[test]
fn the_collection_leaves_out_folders_that_git_ignores_and_a_build_leaves() {
    let tmp = tempfile::tempdir().unwrap();
    for ignored in [".turbo", "target", "node_modules"] {
        std::fs::create_dir(tmp.path().join(ignored)).unwrap();
        std::fs::write(tmp.path().join(ignored).join("build.log"), "Compiling x v1.0.1").unwrap();
    }
    std::fs::create_dir(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src").join("kept.md"), "text").unwrap();
    let mut files = Vec::new();
    collect(tmp.path(), &mut files);
    let names: Vec<String> =
        files.iter().map(|p| p.strip_prefix(tmp.path()).unwrap().display().to_string()).collect();
    assert_eq!(names, ["src/kept.md"], "only the folders git tracks are read");
}

#[test]
fn no_text_cites_table_5_or_the_version_1_0_1_of_the_controller_resources() {
    let texts = texts();
    assert!(texts.len() > 50, "the scan reads the texts of osd, schemas, docs and conformance");
    let banned = ["v1.0.1", "table 5", "tables 5", "table `5`", "\"table\": \"5\""];
    let mut found = Vec::new();
    for (path, text) in &texts {
        for (n, line) in text.lines().enumerate() {
            for b in banned {
                if line.contains(b) {
                    found.push(format!("{path}:{}: {b}", n + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "citations of the standard at the wrong version or table:\n{}", found.join("\n"));
}

#[test]
fn every_citation_of_the_controller_resources_with_a_table_says_tables_6_and_12() {
    for (path, text) in texts() {
        for line in text.lines().filter(|l| l.contains("M2-117-UM") && l.contains("table")) {
            let flat = line.replace('`', "");
            assert!(
                flat.contains("tables 6 and 12") || flat.contains("\"table\": \"6\"") || flat.contains("\"table\": \"12\""),
                "{path}: a citation of M2-117-UM with a table does not say tables 6 and 12: {line}"
            );
        }
    }
}
