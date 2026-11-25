//! docs/CADRE.md R18: no place of the crate names a format but its own module (`src/<module>.rs`
//! and `src/<module>/`) and its line in the list of src/lib.rs. Test files and comment lines are
//! left out.

use osd_formats::Format;
use std::path::{Path, PathBuf};

/// The three ways a format is named.
struct Named {
    name: String,
    module: String,
    variant: String,
}

fn named(name: &str, variant: &str) -> Named {
    Named { name: name.to_string(), module: name.replace('-', "_"), variant: variant.to_string() }
}

/// Whether `text` holds `word` with no lower case letter nor digit against it: `a_dtd` and
/// `crate::a::b` name `a`, `abc` does not.
fn names(text: &str, word: &str) -> bool {
    let glued = |c: Option<char>| c.is_some_and(|c| c.is_lowercase() || c.is_ascii_digit());
    text.match_indices(word)
        .any(|(at, _)| !glued(text[..at].chars().next_back()) && !glued(text[at + word.len()..].chars().next()))
}

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

/// Each place of the sources under `src` that names a format which is not its own.
fn violations(src: &Path, formats: &[Named]) -> Vec<String> {
    let mut files = Vec::new();
    rust_files(src, &mut files);
    files.sort();
    assert!(!files.is_empty(), "{}: no source", src.display());
    let mut found = Vec::new();
    for file in files {
        let relative = file.strip_prefix(src).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&file).unwrap();
        for format in formats {
            let own = relative == format!("{}.rs", format.module) || relative.starts_with(&format!("{}/", format.module));
            if own {
                continue;
            }
            let words = [&format.name, &format.module, &format.variant];
            if words.iter().any(|w| names(&relative, w)) {
                found.push(format!("{relative}: a file of {} outside its module", format.name));
            }
            let its_line = format!("{} => {}: \"", format.variant, format.module);
            for (n, line) in text.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") || (relative == "lib.rs" && code.starts_with(&its_line)) {
                    continue;
                }
                if words.iter().any(|w| names(code, w)) {
                    found.push(format!("{relative}:{}: names {}: {code}", n + 1, format.name));
                }
            }
        }
    }
    found
}

#[test]
fn no_source_of_the_crate_names_a_format_outside_its_module_and_its_line_in_the_list() {
    let formats: Vec<Named> = Format::ALL.iter().map(|f| named(f.name(), &format!("{f:?}"))).collect();
    let found = violations(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &formats);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// A source folder holding the files given, and what the guard finds in it for two formats.
fn found_in(files: &[(&str, &str)]) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    violations(dir.path(), &[named("alpha", "Alpha"), named("beta-two", "BetaTwo")])
}

#[test]
fn the_guard_of_names_allows_a_format_in_its_module_its_list_line_comments_and_test_files() {
    let found = found_in(&[
        ("lib.rs", "//! alpha and beta-two\nformats! {\n    Alpha => alpha: \"Alpha.\",\n    BetaTwo => beta_two: \"Two.\",\n}\n"),
        ("alpha.rs", "mod dtd;\nconst NAME: &str = \"alpha\";\n"),
        ("alpha/dtd.rs", "use crate::alpha::NAME;\n"),
        ("beta_two.rs", "// as alpha does\nconst NAME: &str = \"beta-two\";\n"),
        ("shared.rs", "    /// Alpha, beta-two\nconst ALPHABET: &str = \"alphabet\";\n"),
        ("tests.rs", "use crate::alpha;\n"),
        ("tests/probe.rs", "use crate::beta_two;\n"),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_guard_of_names_refuses_a_name_a_module_or_a_variant_anywhere_else() {
    for (file, text) in [
        ("shared.rs", "const NAME: &str = \"beta-two\";\n"),
        ("shared.rs", "use crate::alpha::check;\n"),
        ("alpha.rs", "use crate::beta_two::check;\n"),
        ("lib.rs", "mod alpha;\n"),
        ("lib.rs", "mod alpha_dtd;\n"),
        ("lib.rs", "        Format::BetaTwo => 2,\n"),
        ("lib.rs", "    Alpha => alpha: \"Like BetaTwo.\",\n"),
        ("shared.rs", "    Alpha => alpha: \"Alpha.\",\n"),
        ("alpha_dtd.rs", "const TABLE: u8 = 0;\n"),
    ] {
        assert!(!found_in(&[(file, text)]).is_empty(), "{file} passes: {text:?}");
    }
}
