//! A triggers CSV imported then exported gives back the same rows.

use crate::common::{parse, resolved, source, trigger_csv, trigger_row};
use osd_formats::{export, import, Format};

fn base(extra: &[(&'static str, &'static str)]) -> Vec<String> {
    let mut cells = vec![("manufacturer", "Lofty"), ("device", "Trundler")];
    cells.extend_from_slice(extra);
    trigger_row(&cells)
}

fn through(text: &str) -> String {
    let imported = import(text, Format::MidiTriggersCsv, &source()).expect("imports");
    export(&resolved(imported.value), Format::MidiTriggersCsv).expect("exports").value
}

#[test]
fn rows_with_every_kind_of_cell_come_back_the_same_in_order() {
    let rows = vec![
        base(&[
            ("section", "Drums"),
            ("sound_name", "Kick"),
            ("sound_description", "The big one"),
            ("midi_channel", "10"),
            ("note_number_min_value", "36"),
            ("note_number_max_value", "38"),
            ("note_number_default_value", "37"),
            ("comments", "Choke group, 1"),
            ("velocity_usage", "Soft: damped; hard: open"),
        ]),
        base(&[("section", "Drums"), ("sound_name", "Snare"), ("midi_channel", "1"), ("note_number_min_value", "40"), ("note_number_max_value", "40"), ("note_number_default_value", "40")]),
        base(&[("section", "Perc"), ("sound_name", "Clap \"X\""), ("midi_channel", "16"), ("note_number_min_value", "0"), ("note_number_max_value", "127")]),
        base(&[("sound_name", "Bare"), ("note_number_min_value", "5"), ("note_number_max_value", "6")]),
        base(&[("section", "Drums"), ("sound_name", "Kick"), ("note_number_min_value", "50"), ("note_number_max_value", "50")]),
        base(&[("section", "Drums"), ("sound_name", "Kick"), ("note_number_min_value", "50"), ("note_number_max_value", "50")]),
    ];
    assert_eq!(parse(&through(&trigger_csv(&rows))), parse(&trigger_csv(&rows)));
}

#[test]
fn the_rows_and_cells_that_are_lost_are_the_only_ones_missing_on_the_way_back() {
    let keep = base(&[("section", "Drums"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "38")]);
    let no_range = base(&[("sound_name", "Lost row"), ("note_number_min_value", "36")]);
    let omni = base(&[("sound_name", "Omni"), ("midi_channel", "omni"), ("note_number_min_value", "1"), ("note_number_max_value", "2")]);
    let off_default = base(&[("sound_name", "Off"), ("note_number_min_value", "1"), ("note_number_max_value", "2"), ("note_number_default_value", "9")]);
    let imported = import(&trigger_csv(&[keep.clone(), no_range, omni.clone(), off_default.clone()]), Format::MidiTriggersCsv, &source()).unwrap();
    assert_eq!(imported.lost.len(), 3, "{:?}", imported.lost);
    let out = parse(&export(&resolved(imported.value), Format::MidiTriggersCsv).unwrap().value);
    let mut omni_back = omni;
    omni_back[5] = String::new();
    let mut off_back = off_default;
    off_back[8] = String::new();
    assert_eq!(out[1..].to_vec(), vec![keep, omni_back, off_back]);
}
