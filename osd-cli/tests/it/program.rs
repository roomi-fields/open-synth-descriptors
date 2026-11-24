//! The program as a whole: its help, its version, its misuse codes, its words
//! as the interface and the README write them, its dependencies.

use crate::common::{osd, repo, INIT, WORDS};

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-').filter(|w| !w.is_empty()).map(str::to_string).collect()
}

#[test]
fn osd_help_lists_every_word() {
    let run = osd(&["--help"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let w = words(&run.stdout);
    for word in WORDS {
        assert!(w.iter().any(|x| x == word), "{word} missing from {:?}", run.stdout);
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
fn the_interface_shapes_of_import_and_export_carry_the_source_and_output_options() {
    let rows = interface_rows();
    let shape = |w: &str| rows.iter().find(|r| r.0 == w).unwrap_or_else(|| panic!("no row for {w}")).1.clone();
    for option in ["--source-url", "--source-author", "--license", "-o", "--force"] {
        assert!(shape("import").contains(option), "{option} not in {}", shape("import"));
    }
    assert!(shape("export").contains("--force"), "{}", shape("export"));
    for word in ["import", "export"] {
        let (_, _, line) = rows.iter().find(|r| r.0 == word).unwrap();
        assert!(!line.contains("not yet delivered"), "{line}");
    }
}

#[test]
fn the_interface_writes_the_shape_of_each_word_that_reads_or_writes_a_file() {
    let rows = interface_rows();
    for word in ["validate", "convert", INIT, "import", "export"] {
        let (_, shape, _) = rows.iter().find(|r| r.0 == word).unwrap_or_else(|| panic!("no row for {word}"));
        assert!(shape.starts_with(&format!("osd {word}")), "{shape:?} is not the shape of {word}");
    }
}

#[test]
fn convert_writes_the_latest_published_version_by_default_says_the_interface() {
    let rows = interface_rows();
    let (_, _, line) = rows.iter().find(|r| r.0 == "convert").unwrap();
    assert!(line.contains("latest published version (0.6)"), "{line}");
    assert!(!line.contains("0.6 draft"), "{line}");
}

fn format_names(word: &str) -> Vec<String> {
    let rows = interface_rows();
    let (_, _, line) = rows.iter().find(|r| r.0 == word).unwrap_or_else(|| panic!("no row for {word}"));
    let returns = line.split('|').map(str::trim).nth(3).unwrap();
    returns
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|t| t.starts_with(|c: char| c.is_ascii_lowercase()) && t.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
        .map(str::to_string)
        .collect()
}

fn import_export_text() -> String {
    let rows = interface_rows();
    ["import", "export"]
        .iter()
        .map(|w| rows.iter().find(|r| r.0 == *w).unwrap_or_else(|| panic!("no row for {w}")).2.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_interface_rows_of_import_and_export_name_every_format() {
    let text = import_export_text();
    let mut names = format_names("import");
    names.extend(format_names("export"));
    for expected in ["midi-csv", "midi-triggers-csv", "electra-one", "midnam", "oscquery", "midi2-controllers", "deluge"] {
        assert!(names.iter().any(|n| n == expected), "{expected} not read from the interface: {names:?}");
        assert!(text.contains(&format!("`{expected}`")), "{expected} not in the rows");
    }
}

#[test]
fn the_interface_states_the_limits_of_import_and_export() {
    let rows = interface_rows();
    let import = &rows.iter().find(|r| r.0 == "import").unwrap().2;
    let export = &rows.iter().find(|r| r.0 == "export").unwrap().2;
    assert!(import.contains("`deluge` is refused"), "deluge exists only at export: {import}");
    assert!(export.contains("`deluge`"), "{export}");
    assert!(
        import.contains("`--into <descriptor>` (`midi-csv` and `midi-triggers-csv` only)"),
        "--into holds only for midi-csv and midi-triggers-csv: {import}"
    );
    assert!(import.contains("error output") && export.contains("error output"), "the losses go to the error output");
}

#[test]
fn the_crate_depends_on_no_network_library() {
    let manifest = std::fs::read_to_string(repo().join("osd-cli/Cargo.toml")).unwrap();
    let deps = manifest.split("[dev-dependencies]").next().unwrap();
    for net in ["reqwest", "hyper", "ureq", "curl", "isahc", "surf", "attohttpc", "tokio", "async-std", "openssl", "rustls", "native-tls", "h2", "tungstenite"] {
        assert!(!deps.lines().any(|l| l.trim_start().starts_with(&format!("{net} ")) || l.trim_start().starts_with(&format!("{net}="))), "{net}");
    }
}
