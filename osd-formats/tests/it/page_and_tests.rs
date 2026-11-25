//! docs/CADRE.md R19: each format has its page, `docs/<name>.md`, and its tests, a file
//! `tests/it/<module>.rs` or `tests/it/<module>_*.rs` that tests/it/main.rs declares and that holds
//! a test; the module is the name with `-` written `_`.

use osd_formats::Format;
use std::path::Path;

/// Why the crate at `root` refuses the format `name`: what it lacks, each lack naming the format.
fn lacks(root: &Path, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    if !root.join("docs").join(format!("{name}.md")).is_file() {
        out.push(format!("{name}: no page docs/{name}.md"));
    }
    let module = name.replace('-', "_");
    let tests = root.join("tests/it");
    let main = std::fs::read_to_string(tests.join("main.rs")).unwrap_or_default();
    let tested = std::fs::read_dir(&tests).into_iter().flatten().any(|entry| {
        let path = entry.unwrap().path();
        let Some(stem) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".rs")) else { return false };
        let its_own = stem == module || stem.starts_with(&format!("{module}_"));
        let declared = main.lines().any(|l| l.trim() == format!("mod {stem};"));
        its_own
            && declared
            && std::fs::read_to_string(&path).is_ok_and(|t| t.lines().any(|l| l.trim_start().starts_with("#[test]")))
    });
    if !tested {
        out.push(format!("{name}: no test file tests/it/{module}.rs or tests/it/{module}_*.rs declared in tests/it/main.rs with a test"));
    }
    out
}

#[test]
fn every_format_of_the_list_has_its_page_and_its_tests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let found: Vec<String> = Format::ALL.iter().flat_map(|f| lacks(root, f.name())).collect();
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// A crate holding the files given.
fn crate_of(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

const PAGE: (&str, &str) = ("docs/beta-two.md", "# beta-two\n");
const MAIN: (&str, &str) = ("tests/it/main.rs", "mod beta_two_export;\n");
const TEST: (&str, &str) = ("tests/it/beta_two_export.rs", "#[test]\nfn exports() {}\n");

#[test]
fn the_guard_accepts_a_format_with_its_page_and_a_declared_test_file_of_either_name() {
    let prefixed = crate_of(&[PAGE, MAIN, TEST]);
    assert_eq!(lacks(prefixed.path(), "beta-two"), Vec::<String>::new());
    let bare = crate_of(&[PAGE, ("tests/it/main.rs", "mod beta_two;\n"), ("tests/it/beta_two.rs", TEST.1)]);
    assert_eq!(lacks(bare.path(), "beta-two"), Vec::<String>::new());
}

#[test]
fn the_guard_refuses_a_format_without_its_page_and_names_it() {
    for files in [vec![MAIN, TEST], vec![MAIN, TEST, ("docs/beta_two.md", "# beta-two\n")]] {
        let dir = crate_of(&files);
        let found = lacks(dir.path(), "beta-two");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("beta-two: no page"), "{found:?}");
    }
}

#[test]
fn the_guard_refuses_a_format_without_tests_and_names_it() {
    let cases: [Vec<(&str, &str)>; 5] = [
        vec![PAGE],
        vec![PAGE, TEST],
        vec![PAGE, MAIN, ("tests/it/beta_two_export.rs", "// #[test]\nfn helper() {}\n")],
        vec![PAGE, ("tests/it/main.rs", "mod beta_twofold;\n"), ("tests/it/beta_twofold.rs", TEST.1)],
        vec![PAGE, MAIN, ("tests/common/beta_two_export.rs", TEST.1)],
    ];
    for files in cases {
        let dir = crate_of(&files);
        let found = lacks(dir.path(), "beta-two");
        assert_eq!(found.len(), 1, "{files:?}: {found:?}");
        assert!(found[0].starts_with("beta-two: no test file"), "{found:?}");
    }
}
