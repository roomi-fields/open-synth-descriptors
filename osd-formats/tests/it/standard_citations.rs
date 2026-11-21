//! The texts cite the MIDI 2.0 controller resources at their version and tables: M2-117-UM v1.0,
//! tables 6 and 12.

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

fn read(path: &str) -> String {
    let full = format!("{ROOT}/{path}");
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
}

const TEXTS: [&str; 3] = ["docs/controls.md", "docs/vocabulary.md", "osd-formats/docs/INTERFACE.md"];

#[test]
fn no_text_cites_table_5_or_the_version_1_0_1_of_the_controller_resources() {
    for path in TEXTS {
        let text = read(path);
        for banned in ["table 5", "table `5`", "tables 5", "M2-117-UM v1.0.1"] {
            assert!(!text.contains(banned), "{path} still contains {banned:?}");
        }
    }
}

#[test]
fn every_mention_of_the_controller_resources_says_v1_0_and_tables_6_and_12() {
    for path in TEXTS {
        let text = read(path);
        let mut mentions = 0;
        for line in text.lines().filter(|l| l.contains("M2-117-UM")) {
            mentions += 1;
            let v10 = line.match_indices("M2-117-UM").all(|(i, _)| {
                let rest = &line[i + "M2-117-UM".len()..];
                rest.starts_with(" v1.0") && !rest[" v1.0".len()..].starts_with('.')
            });
            assert!(v10, "{path}: a mention of M2-117-UM does not say v1.0: {line}");
            let flat = line.replace('`', "");
            assert!(
                flat.contains("tables 6 and 12"),
                "{path}: a mention of M2-117-UM does not say tables 6 and 12: {line}"
            );
        }
        assert!(mentions > 0, "{path} cites no M2-117-UM");
    }
}

#[test]
fn the_citation_of_the_midi_ci_table_of_defined_controllers_stays() {
    assert!(read("docs/vocabulary.md").contains("M2-104-UM v1.1.2"));
}

#[test]
fn controls_do_not_present_the_param_entry_as_a_control_of_the_standard() {
    let text = read("docs/controls.md");
    assert!(!text.contains("param entry"), "docs/controls.md still says the param entry");
    assert!(text.contains("(CC, 14-bit CC, NRPN, RPN)"), "the list of entries is CC, 14-bit CC, NRPN, RPN");
}
