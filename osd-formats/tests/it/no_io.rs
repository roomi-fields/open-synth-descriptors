//! Docs/CADRE.md R13: the crate reads no file, writes none and opens no port.

use std::path::{Path, PathBuf};

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_crate_sources_hold_no_file_network_or_process_use() {
    let mut files = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(!files.is_empty(), "the crate has no source");
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            let grouped = code.contains("std::{") && ["fs", "net", "process"].iter().any(|w| code.contains(w));
            if ["std::fs", "std::net", "std::process"].iter().any(|w| code.contains(w)) || grouped {
                found.push(format!("{}:{}: {code}", file.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}
