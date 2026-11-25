//! The command names no format of its own: it takes the formats, their names and those that add a
//! file to a descriptor from the list of osd-formats.

use crate::common::{osd, repo};
use osd_formats::Format;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "tests") {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != "tests.rs") {
            out.push(path);
        }
    }
}

/// Whether `text` holds `word` with no lower case letter nor digit against it.
fn names(text: &str, word: &str) -> bool {
    let glued = |c: Option<char>| c.is_some_and(|c| c.is_lowercase() || c.is_ascii_digit());
    text.match_indices(word)
        .any(|(at, _)| !glued(text[..at].chars().next_back()) && !glued(text[at + word.len()..].chars().next()))
}

#[test]
fn no_source_of_the_command_names_a_format() {
    let mut files = Vec::new();
    rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(!files.is_empty(), "the crate has no source");
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for format in Format::ALL {
                let name = format.name();
                if [name.to_string(), name.replace('-', "_"), format!("{format:?}")].iter().any(|w| names(code, w)) {
                    found.push(format!("{}:{}: names {name}: {code}", file.display(), n + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn the_help_of_import_names_the_formats_that_add_a_file_to_a_descriptor() {
    let help = osd(&["import", "--help"]);
    assert_eq!(help.code, 0, "{:?}", help.stderr);
    assert!(help.stdout.contains("(for midi-csv and midi-triggers-csv)"), "{}", help.stdout);
}

#[test]
fn into_with_a_format_that_adds_nothing_names_it_and_each_format_that_can() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.txt").to_string_lossy().into_owned();
    std::fs::write(&file, "{}").unwrap();
    let base = repo().join("examples/0.6/minilogue.device.json").to_string_lossy().into_owned();
    for format in ["midnam", "oscquery", "deluge"] {
        let run = osd(&["import", format, &file, "--source-url", "https://example.org/x", "--license", "CC0-1.0", "--into", &base]);
        assert_eq!(run.code, 1, "{format}: {:?}", run.stderr);
        assert_eq!(run.stdout, "", "{format}");
        for word in [format, "midi-csv", "midi-triggers-csv"] {
            assert!(run.stderr.contains(word), "{format}: {word} is not named: {:?}", run.stderr);
        }
    }
}
