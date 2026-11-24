//! How the part is declared: its manifest and its place in the public tree.

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).unwrap()
}

#[test]
fn the_part_is_not_published_and_names_its_binary_osd_reference() {
    let manifest: toml::Table = read("reference/Cargo.toml").parse().unwrap();
    assert_eq!(manifest["package"]["publish"].as_bool(), Some(false));
    let bins = manifest["bin"].as_array().expect("a [[bin]]");
    assert!(bins.iter().any(|b| b["name"].as_str() == Some("osd-reference")));
}

#[test]
fn the_built_pages_are_ignored_by_git() {
    assert!(read(".gitignore").lines().any(|l| l.trim().trim_start_matches('/').starts_with("spec/reference")));
}
