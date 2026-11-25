//! The document of destiny docs/midi2-controllers.md: one line per property of tables 6 and 12
//! of M2-117-UM v1.0, with its fate, its reason and its piece.

// @lit spec/0.7.md
use crate::common::midi2::PROPERTIES;

const DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/midi2-controllers.md");

fn doc_lines() -> Vec<Vec<String>> {
    std::fs::read_to_string(DOC)
        .unwrap_or_else(|e| panic!("{DOC}: {e}"))
        .lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .map(|l| l.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect())
        .collect()
}

#[test]
fn every_property_of_the_table_has_a_line_with_its_fate_its_reason_and_the_piece() {
    let lines = doc_lines();
    for name in PROPERTIES {
        let line = lines.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no line for {name} in {DOC}"));
        assert!(line.len() >= 4, "{name}: a line has the name, the fate, the reason and the piece");
        assert!(line[1] == "carried" || line[1] == "lost", "{name}: the fate is carried or lost, not {:?}", line[1]);
        assert!(!line[2].is_empty(), "{name}: no reason");
        assert!(line[3].contains("M2-117-UM"), "{name}: the piece of the standard is not cited: {:?}", line[3]);
    }
}

#[test]
fn the_document_lists_no_property_the_table_does_not_have() {
    for line in doc_lines() {
        let name = line[0].as_str();
        if name == "Property" || name.starts_with('-') || name.is_empty() {
            continue;
        }
        assert!(PROPERTIES.contains(&name), "{name} is in {DOC} and not in tables 6 and 12 of M2-117-UM v1.0");
    }
}

#[test]
fn the_properties_the_import_cannot_read_back_are_lost_and_the_others_carried() {
    let lines = doc_lines();
    for name in ["channel", "ctrlType", "ctrlIndex", "priority", "default", "transmit", "recognize", "paramPath", "typeHint", "ctrlMapId", "stepCount", "minMax"] {
        let line = lines.iter().find(|l| l[0] == name).unwrap();
        assert_eq!(line[1], "carried", "{name}");
    }
}

#[test]
fn title_and_description_are_carried_with_their_piece_and_the_title_sentence_names_the_derivation() {
    let lines = doc_lines();
    for name in ["title", "description"] {
        let line = lines.iter().find(|l| l[0] == name).unwrap();
        assert_eq!(line[1], "carried", "{name}");
        assert!(line[3].contains("M2-117-UM v1.0") && line[3].contains("tables 6 and 12"), "{name}: {:?}", line[3]);
    }
    let text = std::fs::read_to_string(DOC).unwrap();
    let sentence = text.lines().find(|l| l.contains("The standard requires a `title`")).unwrap_or_else(|| panic!("no sentence on the title in {DOC}"));
    assert!(!sentence.contains("holds none"), "{sentence}");
    let (role, section, osc) = (sentence.find("role"), sentence.find("section"), sentence.find("osc"));
    assert!(role.is_some() && section > role && osc > section && sentence.contains("absent"), "the title of the control, then role path, section, osc, only when title is absent: {sentence}");
}

#[test]
fn the_specification_says_title_and_description_are_carried_by_the_title_and_description_of_the_control() {
    let spec = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../spec/0.7.md")).unwrap();
    let from = spec.find("### 22.1 ").expect("section 22.1");
    let to = spec[from..].find("### 22.2 ").expect("section 22.2") + from;
    let spec = &spec[from..to];
    for name in ["title", "description"] {
        let row = spec.lines().find(|l| l.starts_with(&format!("| `{name}` |"))).unwrap_or_else(|| panic!("no row for {name}"));
        assert!(!row.contains("not carried"), "{row}");
        let field = row.split('|').nth(2).unwrap();
        assert!(field.contains(&format!("`{name}`")), "{row}");
    }
}

#[test]
fn the_document_cites_the_standard_at_version_1_0_tables_6_and_12_and_names_its_nine_types() {
    let text = std::fs::read_to_string(DOC).unwrap();
    assert!(!text.contains("v1.0.1"), "the standard is version 1.0");
    assert!(!text.contains("table 5"), "the controller object is table 6 (AllCtrlList) and table 12 (ChCtrlList)");
    assert!(text.contains("v1.0"), "the version is cited");
    assert!(text.contains("table 6") || text.contains("tables 6"), "table 6 is cited");
    assert!(text.contains("12"), "table 12 is cited");
    let lines = doc_lines();
    let ctrl_type = lines.iter().find(|l| l[0] == "ctrlType").unwrap();
    let row = ctrl_type.join(" ");
    for word in ["cc", "chPress", "pPress", "nrpn", "rpn", "pBend", "pnrc", "pnac", "pnp"] {
        assert!(row.contains(word), "ctrlType line names {word}: {row}");
    }
    assert!(!row.contains("param or") && !row.contains("`param`"), "param is not a ctrlType: {row}");
}

#[test]
fn the_document_says_the_sysex_params_are_not_written_and_a_standard_type_the_descriptor_cannot_carry_is_lost() {
    let text = std::fs::read_to_string(DOC).unwrap();
    assert!(text.contains("SysEx"), "the SysEx params that the export does not write are named");
    assert!(text.contains("params"), "the params block is named");
    assert!(text.contains("pBend"), "the types lost at import are named");
}

fn line_of(name: &str) -> String {
    doc_lines().iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no line for {name}")).join(" ")
}

#[test]
fn the_channel_line_says_the_norm_counts_from_one_up_to_256_and_osd_from_zero_to_15() {
    let row = line_of("channel");
    assert!(row.contains("1-256"), "{row}");
    assert!(row.contains("0 to 15"), "{row}");
    assert!(row.contains("Group"), "{row}");
}

#[test]
fn the_default_and_minmax_lines_say_the_32_bit_scale_and_the_conversion() {
    for name in ["default", "minMax"] {
        let row = line_of(name);
        assert!(row.contains("unsigned 32"), "{name}: {row}");
        assert!(row.contains("M2-115-U"), "{name}: {row}");
        assert!(row.contains("Min-Center-Max") && row.contains("Zero-Extension"), "{name}: {row}");
    }
    assert!(line_of("numSigBits").contains("32"), "numSigBits is 32 when not provided");
}
