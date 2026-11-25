//! docs/CADRE.md R18: the list of formats is one invocation of `formats!` in src/lib.rs, one
//! line per format, `Variant => module: "documentation",`.

use osd_formats::Format;

/// The lines of the present formats, in their order, each with the public documentation of its
/// variant.
const PRESENT: [&str; 8] = [
    r#"MidiCsv => midi_csv: "The pencilresearch/midi CSV (`template.csv`).","#,
    r#"MidiTriggersCsv => midi_triggers_csv: "The pencilresearch/midi triggers CSV (`template.triggers.csv`).","#,
    r#"ElectraOne => electra_one: "The Electra One preset (JSON, versions 2 and 3).","#,
    r#"Midnam => midnam: "The MIDNAM names document of the MIDI Association (MIDINameDocument 1.0).","#,
    r#"OscQuery => oscquery: "The OSCQuery namespace (JSON).","#,
    r#"Midi2Controllers => midi2_controllers: "The controller resources of MIDI-CI Property Exchange (M2-117-UM): ChCtrlList and CtrlMapList (JSON).","#,
    r#"Deluge => deluge: "The Deluge device definition (XML `midiDevice` with `ccLabels`); exported only.","#,
    r#"Knobkraft => knobkraft: "The skeleton of a KnobKraft Orm adaptation (Python text); exported only, never run here.","#,
];

/// The lines of the one invocation of `formats!` in src/lib.rs, blank and comment lines left out.
fn lines_of_the_list() -> Vec<String> {
    let file = format!("{}/src/lib.rs", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let starts: Vec<usize> = (0..lines.len()).filter(|&n| lines[n].starts_with("formats!")).collect();
    assert_eq!(starts.len(), 1, "{file} invokes formats! exactly once");
    lines[starts[0] + 1..]
        .iter()
        .take_while(|l| !l.starts_with(['}', ')', ']']))
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .map(|l| l.to_string())
        .collect()
}

#[test]
fn the_list_holds_the_present_formats_in_order_each_with_its_documentation() {
    let lines = lines_of_the_list();
    assert!(lines.len() >= PRESENT.len(), "{lines:#?}");
    assert_eq!(lines[..PRESENT.len()], PRESENT);
}

#[test]
fn the_list_holds_one_line_per_format_naming_its_variant_and_its_module() {
    let lines = lines_of_the_list();
    assert_eq!(lines.len(), Format::ALL.len(), "{lines:#?}");
    for (line, format) in lines.iter().zip(Format::ALL) {
        let start = format!("{format:?} => {}: \"", format.name().replace('-', "_"));
        assert!(line.starts_with(&start) && line.ends_with("\","), "{line:?} is not the line of {}", format.name());
    }
}
