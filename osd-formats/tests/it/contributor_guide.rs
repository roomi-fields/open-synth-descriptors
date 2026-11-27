//! docs/CADRE.md R20: a format is changed or added by a pull request, and CONTRIBUTING.md,
//! « Add a converter », says how.

/// The section « Add a converter » of CONTRIBUTING.md, on one line, in lower case.
fn section() -> String {
    let file = format!("{}/../CONTRIBUTING.md", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
    let mut lines = text.lines().skip_while(|l| l.trim_end() != "## Add a converter");
    assert!(lines.next().is_some(), "{file} has no section « Add a converter »");
    let body: Vec<&str> = lines.take_while(|l| !l.starts_with("## ")).collect();
    body.join(" ").split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn says(words: &[&str]) {
    let section = section();
    for word in words {
        assert!(section.contains(&word.to_lowercase()), "« Add a converter » does not say {word}");
    }
}

#[test]
fn the_guide_to_add_a_converter_names_the_module_the_page_the_tests_the_line_and_the_pull_request() {
    says(&["module", "docs/<", "tests/it", "src/lib.rs", "pull request", "media type", "extension"]);
}

#[test]
fn the_guide_to_add_a_converter_names_the_texts_that_list_the_formats() {
    says(&[
        "osd-formats/docs/CADRE.md",
        "osd-formats/docs/INTERFACE.md",
        "osd-cli/docs/INTERFACE.md",
        "osd-formats/README.md",
        "description",
        "osd-formats/Cargo.toml",
    ]);
}
