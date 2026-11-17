//! A CSV imported then exported gives back the same rows.

use crate::common::{csv, parse, resolved, row, source};
use osd_formats::{export, import, Format};

fn base(extra: &[(&'static str, &'static str)]) -> Vec<String> {
    let mut cells = vec![("manufacturer", "Lofty"), ("device", "Trundler")];
    cells.extend_from_slice(extra);
    row(&cells)
}

fn through(text: &str) -> String {
    let imported = import(text, Format::MidiCsv, &source()).expect("imports");
    export(&resolved(imported.value), Format::MidiCsv).expect("exports").value
}

#[test]
fn rows_with_a_cc_alone_or_an_nrpn_alone_come_back_the_same_in_order() {
    let rows = vec![
        base(&[
            ("section", "Oscillators"),
            ("parameter_name", "Glide switch"),
            ("parameter_description", "Enables or disables glide"),
            ("cc_msb", "65"),
            ("cc_min_value", "0"),
            ("cc_max_value", "127"),
            ("orientation", "0-based"),
            ("notes", "Typo in manual (says CC 66; 65 is correct)"),
            ("usage", "0-63: Off; 64-127: On"),
        ]),
        base(&[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "1"), ("cc_lsb", "33"), ("cc_min_value", "0"), ("cc_max_value", "16383"), ("orientation", "centered")]),
        base(&[("section", "Filter"), ("parameter_name", "Resonance, \"Q\""), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_min_value", "0"), ("nrpn_max_value", "16383")]),
        base(&[("parameter_name", "Bare"), ("cc_msb", "7")]),
        base(&[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74")]),
    ];
    assert_eq!(parse(&through(&csv(&rows))), parse(&csv(&rows)));
}

#[test]
fn a_row_with_both_gives_two_rows_back_and_a_row_with_neither_is_dropped_into_the_losses() {
    let both = base(&[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_min_value", "0"), ("nrpn_max_value", "16383"), ("notes", "n")]);
    let neither = base(&[("parameter_name", "Sysex only")]);
    let alone = base(&[("parameter_name", "Alone"), ("cc_msb", "5")]);
    let text = csv(&[alone.clone(), neither, both.clone()]);
    let imported = import(&text, Format::MidiCsv, &source()).unwrap();
    assert!(imported.lost.iter().any(|l| l.place == "line 3"));
    let out = parse(&export(&resolved(imported.value), Format::MidiCsv).unwrap().value);
    let cc_row = row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("notes", "n")]);
    let nrpn_row = row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Cutoff"), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_min_value", "0"), ("nrpn_max_value", "16383"), ("notes", "n")]);
    assert_eq!(out[1..].to_vec(), vec![alone, cc_row, nrpn_row]);
}
