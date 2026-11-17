//! The program as a whole: its help, its version, its misuse codes, its words
//! as the interface and the README write them, its dependencies.

use crate::common::{osd, repo, INIT, WORDS};

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-').filter(|w| !w.is_empty()).map(str::to_string).collect()
}

#[test]
fn osd_help_lists_the_three_words_and_neither_import_nor_export() {
    let run = osd(&["--help"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let w = words(&run.stdout);
    for word in WORDS {
        assert!(w.iter().any(|x| x == word), "{word} missing from {:?}", run.stdout);
    }
    for word in ["import", "export"] {
        assert!(!w.iter().any(|x| x == word), "{word} listed in {:?}", run.stdout);
    }
}

#[test]
fn each_word_answers_help_with_exit_0() {
    for word in WORDS {
        let run = osd(&[word, "--help"]);
        assert_eq!(run.code, 0, "{word}: {:?}", run.stderr);
        assert!(run.stdout.contains(word), "{word}");
    }
}

#[test]
fn osd_without_a_word_exits_2_with_the_help_on_the_error_output() {
    let run = osd(&[]);
    assert_eq!(run.code, 2);
    assert_eq!(run.stdout, "");
    for word in WORDS {
        assert!(run.stderr.contains(word), "{word} missing from {:?}", run.stderr);
    }
}

#[test]
fn osd_with_an_unknown_word_or_option_exits_2_with_the_help_on_the_error_output() {
    for args in [&["frobnicate"][..], &["--frobnicate"][..], &["validate", "--frobnicate", "x.json"][..], &[INIT, "device", "--frobnicate"][..]] {
        let run = osd(args);
        assert_eq!(run.code, 2, "{args:?}");
        assert!(run.stderr.contains("validate"), "{args:?}: {:?}", run.stderr);
    }
}

#[test]
fn osd_version_names_the_program_and_each_version_the_format_reads() {
    let run = osd(&["--version"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(run.stdout.starts_with("osd"), "{:?}", run.stdout);
    assert!(run.stdout.contains(env!("CARGO_PKG_VERSION")), "{:?}", run.stdout);
    for v in osd::VERSIONS {
        assert!(words(&run.stdout).iter().any(|w| w == v.name()) || run.stdout.contains(v.name()), "{} in {:?}", v.name(), run.stdout);
    }
    let all: Vec<&str> = osd::VERSIONS.iter().map(|v| v.name()).collect();
    assert!(run.stdout.contains(&all.join(", ")), "{:?}", run.stdout);
}

/// The rows of the interface's overview table, above the generated section:
/// the word and the shape written between backticks.
fn interface_rows() -> Vec<(String, String, String)> {
    let text = std::fs::read_to_string(repo().join("osd-cli/docs/INTERFACE.md")).unwrap();
    let text = text.split("<!-- généré: début -->").next().unwrap();
    text.lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            (cells[1].trim_matches('`').to_string(), cells[2].trim_matches('`').to_string(), l.to_string())
        })
        .collect()
}

#[test]
fn each_delivered_word_shows_in_its_help_the_shape_the_interface_writes() {
    let rows = interface_rows();
    for word in WORDS {
        let (_, shape, _) = rows.iter().find(|r| r.0 == word).unwrap_or_else(|| panic!("no row for {word}"));
        let help = osd(&[word, "--help"]);
        assert_eq!(help.code, 0);
        assert!(help.stdout.contains(shape.as_str()), "{shape:?} not in {:?}", help.stdout);
    }
}

#[test]
fn the_interface_shapes_carry_the_kind_and_force_options() {
    let rows = interface_rows();
    let shape = |w: &str| rows.iter().find(|r| r.0 == w).unwrap().1.clone();
    assert!(shape("validate").contains("--kind"), "{}", shape("validate"));
    assert!(shape("convert").contains("--kind") && shape("convert").contains("--force"), "{}", shape("convert"));
    assert!(shape(INIT).contains("--force"), "{}", shape(INIT));
}

#[test]
fn the_interface_keeps_import_and_export_marked_not_yet_delivered() {
    let rows = interface_rows();
    for word in ["import", "export"] {
        let (_, _, line) = rows.iter().find(|r| r.0 == word).unwrap_or_else(|| panic!("no row for {word}"));
        assert!(line.contains("not yet delivered"), "{line}");
    }
}

#[test]
fn the_readme_presents_the_three_words_with_the_interface_shapes() {
    let readme = std::fs::read_to_string(repo().join("README.md")).unwrap();
    let part = readme.split("## Checking a file").nth(1).expect("the README has `Checking a file`");
    let part = part.split("\n## ").next().unwrap();
    for (word, shape, _) in interface_rows() {
        if WORDS.contains(&word.as_str()) {
            assert!(part.contains(&shape), "{shape:?} not in the README part on checking a file");
        }
    }
}

#[test]
fn the_crate_depends_on_no_network_library() {
    let manifest = std::fs::read_to_string(repo().join("osd-cli/Cargo.toml")).unwrap();
    let deps = manifest.split("[dev-dependencies]").next().unwrap();
    for net in ["reqwest", "hyper", "ureq", "curl", "isahc", "surf", "attohttpc", "tokio", "async-std", "openssl", "rustls", "native-tls", "h2", "tungstenite"] {
        assert!(!deps.lines().any(|l| l.trim_start().starts_with(&format!("{net} ")) || l.trim_start().starts_with(&format!("{net}="))), "{net}");
    }
}
