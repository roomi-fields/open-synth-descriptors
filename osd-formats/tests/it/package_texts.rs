//! The two texts that present the crate, its README.md and the `description` of its Cargo.toml,
//! name every format of the list of src/lib.rs, in its order, and say which are exported only.
//! So does the README of the reader `osd`; both READMEs ask for the version of the workspace, and
//! the introduction of the interface gives the formats in the order of the list.
// @lit osd/README.md

use osd_formats::Format;

fn read(file: &str) -> String {
    let file = format!("{}/{file}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"))
}

fn on_one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// The README and the description of the package, each on one line, in lower case.
fn texts() -> [(&'static str, String); 2] {
    let manifest = read("Cargo.toml");
    let description = manifest
        .lines()
        .find_map(|l| l.strip_prefix("description = \""))
        .and_then(|l| l.trim_end().strip_suffix('"'))
        .expect("Cargo.toml holds its description on one line");
    [("README.md", on_one_line(&read("README.md"))), ("the description of Cargo.toml", on_one_line(description))]
}

/// What a text says to name a format: the words that mark its own mention, then the words of its
/// readable name.
fn readable_name(format: Format) -> (&'static str, &'static [&'static str]) {
    match format.name() {
        "midi-csv" => ("pencilresearch/midi", &["csv"]),
        "midi-triggers-csv" => ("trigger", &["csv"]),
        "electra-one" => ("electra one", &["preset"]),
        "midnam" => ("midnam", &[]),
        "oscquery" => ("oscquery", &[]),
        "midi2-controllers" => ("midi 2.0", &["controller"]),
        "deluge" => ("deluge", &["definition"]),
        "knobkraft" => ("knobkraft orm", &["adaptation", "skeleton"]),
        other => panic!("{other}: no readable name for this format"),
    }
}

/// Each format of the list is named in `text`, after the one before it.
fn assert_named_in_the_order_of_the_list(place: &str, text: &str, mention: impl Fn(Format) -> &'static str) {
    let mut rest = text;
    for format in Format::ALL {
        let mention = mention(format);
        let at = rest.find(mention).unwrap_or_else(|| panic!("{place} does not name {} (« {mention} ») in the order of the list: {text}", format.name()));
        rest = &rest[at + mention.len()..];
    }
}

/// `text` names every format by its readable name: the mention of each in the order of the list,
/// the words of each, and one « csv » for each of the two CSV.
fn assert_every_format_named_by_its_readable_name(place: &str, text: &str) {
    assert_named_in_the_order_of_the_list(place, text, |f| readable_name(f).0);
    for format in Format::ALL {
        for word in readable_name(format).1 {
            assert!(text.contains(word), "{place} does not say « {word} » for {}: {text}", format.name());
        }
    }
    assert!(text.matches("csv").count() >= 2, "{place} does not give each of the two CSV its own mention: {text}");
}

#[test]
fn the_readme_and_the_description_name_every_format_by_its_readable_name_in_the_order_of_the_list() {
    for (place, text) in texts() {
        assert_every_format_named_by_its_readable_name(place, &text);
    }
}

#[test]
fn the_readme_and_the_description_say_which_formats_are_exported_only_and_which_go_both_ways() {
    for (place, text) in texts() {
        assert!(text.contains("exported only"), "{place} says no format is « exported only »: {text}");
        assert!(
            text.contains("to and from") || text.contains("imported and exported"),
            "{place} does not say the other formats are imported and exported: {text}"
        );
    }
}

#[test]
fn the_readme_of_osd_names_every_format_of_osd_formats_in_the_order_of_the_list_and_says_which_are_exported_only() {
    let readme = read("../osd/README.md");
    let paragraph = readme
        .split("\n\n")
        .map(on_one_line)
        .find(|p| p.contains("`osd-formats`"))
        .expect("the README of osd names `osd-formats`");
    let place = "the paragraph of the README of osd on `osd-formats`";
    assert_every_format_named_by_its_readable_name(place, &paragraph);
    let sentence = paragraph.split(". ").find(|s| s.contains("exported only")).unwrap_or_else(|| panic!("{place} says no format is « exported only »: {paragraph}"));
    for format in Format::ALL.into_iter().filter(|f| ["deluge", "knobkraft"].contains(&f.name())) {
        let mention = readable_name(format).0;
        assert!(sentence.contains(mention), "{place} does not say that {} is exported only: {sentence}", format.name());
    }
}

#[test]
fn each_readme_asks_for_the_version_of_the_workspace() {
    let version = env!("CARGO_PKG_VERSION");
    let (minor, _patch) = version.rsplit_once('.').expect("a version of three numbers");
    for (file, name) in [("README.md", "osd-formats"), ("../osd/README.md", "osd")] {
        let asked = format!("{name} = \"{minor}\"");
        let readme = read(file);
        assert!(readme.lines().any(|l| l.trim() == asked), "{file} does not ask for `{asked}`, the workspace being at {version}:\n{readme}");
    }
}

#[test]
fn the_introduction_of_the_interface_gives_the_formats_in_the_order_of_the_list() {
    let interface = read("docs/INTERFACE.md");
    let order = interface
        .lines()
        .find_map(|l| l.split_once("The formats come in this order:"))
        .map(|(_, order)| on_one_line(order))
        .expect("the introduction of the interface says the order of the formats");
    // The introduction names KnobKraft by its short name.
    assert_named_in_the_order_of_the_list("the introduction of docs/INTERFACE.md", &order, |f| match f {
        Format::Knobkraft => "knobkraft",
        other => readable_name(other).0,
    });
}
