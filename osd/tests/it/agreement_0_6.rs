//! What 0.6 freezes says one thing in its prose, its schemas and its examples: the text of
//! 0.6 (`spec/0.6.md`), the schemas and the examples of 0.6 agree before the freeze.

use crate::common;

use serde_json::Value;

fn text(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn json(path: &str) -> Value {
    serde_json::from_str(&text(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The section « What 0.6 changes » of `spec/0.6.md`, up to the next heading.
fn readme_what_0_6_changes() -> String {
    let spec = text("spec/0.6.md");
    let from = spec.find("What 0.6 changes").expect("the section What 0.6 changes");
    let rest = &spec[from + 4..];
    let to = rest.find("\n#").unwrap_or(rest.len());
    rest[..to].to_string()
}

/// A text with its runs of whitespace folded to one space, in lower case.
fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

const KINDS: [&str; 6] = ["firmware", "mod", "module", "application", "script", "extension"];

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric()).map(str::to_string).collect()
}

#[test]
fn the_readme_list_of_what_0_6_changes_names_the_application_and_the_host_device_without_midi() {
    let section = flat(&readme_what_0_6_changes());
    assert!(section.contains("`application`"), "the word `application` is not named");
    assert!(section.contains("without midi") || section.contains("no midi"), "the host device without MIDI is not named");
}

#[test]
fn the_readme_list_of_what_0_6_changes_attributes_no_modules_field_to_0_5() {
    let section = flat(&readme_what_0_6_changes());
    for wrong in ["0.5 `modules`", "`modules` of 0.5", "`modules` field of 0.5"] {
        assert!(!section.contains(wrong), "the list still says {wrong:?}");
    }
}

#[test]
fn the_readme_says_what_the_converter_does_with_a_lower_case_hex_a_mixed_control_and_a_sound_without_content() {
    let section = readme_what_0_6_changes();
    let at = section.find("- **One spelling**").expect("the bullet One spelling");
    let bullet = &section[at..];
    let bullet = flat(&bullet[..bullet[3..].find("\n- **").map_or(bullet.len(), |n| n + 3)]);
    for need in ["lower-case", "converter", "refus"] {
        assert!(bullet.contains(need), "One spelling does not say {need:?} of the conversion: {bullet}");
    }
}

#[test]
fn the_interface_names_the_three_conversion_cases_of_the_tightened_writings() {
    let interface = text("spec/0.6.md");
    for case in ["0.6/hex-lowercase-becomes-uppercase", "0.6/control-mixing-cc-and-nrpn-refused", "0.6/sound-without-content-refused"] {
        assert!(interface.contains(case), "spec/0.6.md does not name the conversion case {case}");
    }
    assert!(flat(&interface).contains("`notconvertible`") || interface.contains("NotConvertible"));
}

#[test]
fn the_firmware_schema_defines_each_word_of_kind_in_the_description_of_kind() {
    let schema = json("schemas/0.6/firmware.schema.json");
    let description = schema["properties"]["kind"]["description"].as_str().expect("`kind` has a description");
    for word in KINDS {
        assert!(description.contains(&format!("`{word}`")), "the description of `kind` does not define `{word}`: {description}");
    }
    // each word is defined, not only listed: the description is longer than the bare list
    assert!(description.len() > 150, "{description}");
}

#[test]
fn the_firmware_schema_description_names_every_kind_of_firmware() {
    let schema = json("schemas/0.6/firmware.schema.json");
    let description = schema["description"].as_str().unwrap();
    let named = words(description);
    for word in KINDS {
        assert!(named.iter().any(|w| w == word), "the description of the firmware schema does not name {word}: {description}");
    }
}

#[test]
fn the_assembly_schema_says_the_origin_firmware_is_of_kind_firmware_or_application() {
    let schema = json("schemas/0.6/assembly.schema.json");
    let stock = &schema["$defs"]["assembly"]["properties"]["stock"];
    let mut descriptions = vec![stock["description"].as_str().unwrap().to_string()];
    if let Some(d) = stock["properties"]["firmware"]["description"].as_str() {
        descriptions.push(d.to_string());
    }
    for d in &descriptions {
        let d = flat(d);
        if d.contains("kind") {
            assert!(d.contains("firmware") && d.contains("application"), "a description names a kind of origin without both: {d}");
        }
    }
    assert!(descriptions.iter().any(|d| d.contains("application")), "no description of the stock names `application`: {descriptions:?}");
}

/// The example card an assembly entry names: `examples/0.6/<name>-<version>.firmware.json`, the
/// name being the last segment of its catalogue id.
fn card_of(entry: &Value, key: &str) -> Option<Value> {
    let id = entry[key].as_str()?;
    let name = id.rsplit('/').next()?;
    let path = format!("examples/0.6/{name}-{}.firmware.json", entry["version"].as_str()?);
    common::repo().join(&path).is_file().then(|| json(&path))
}

fn kind_of_card(card: &Value) -> &str {
    card["kind"].as_str().unwrap_or("firmware")
}

#[test]
fn the_assembly_example_names_a_module_whose_card_is_of_kind_module_and_a_stock_of_kind_firmware_or_application() {
    let assembly = json("examples/0.6/tribe-on-fm1.assembly.json");
    let mut modules = 0;
    for module in assembly["modules"].as_array().unwrap() {
        if let Some(card) = card_of(module, "module") {
            modules += 1;
            assert_eq!(kind_of_card(&card), "module", "the card {} is named as a module", module["module"]);
        }
    }
    assert!(modules >= 1, "no module of the assembly example has its card in examples/0.6");
    if let Some(card) = card_of(&assembly["stock"], "firmware") {
        assert!(["firmware", "application"].contains(&kind_of_card(&card)), "the stock is of kind {}", kind_of_card(&card));
    }
}

#[test]
fn the_interface_no_longer_states_that_identity_unpack_and_the_coding_give_the_same_bytes() {
    // the reader exposes no decoding by `identity.unpack` to a test: no case can measure the claim
    let interface = text("spec/0.6.md");
    for line in interface.lines() {
        let line = flat(line);
        assert!(!(line.contains("lsb_8to7") && line.contains("same decoded bytes")), "the interface states what no case measures: {line}");
    }
}
