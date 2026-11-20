//! What the tests of osd-formats share: the columns of the pencilresearch/midi CSV, rows
//! built cell by cell, a small CSV reader, and a source with its credit.

#![allow(dead_code)]

pub mod deluge;
pub mod electra;
pub mod knobkraft;
pub mod midi2;
pub mod midnam;
pub mod oscquery;
pub mod xml;

/// The devices of this repository's conformance cases and examples, as (name, text): every
/// device a reading case returns a result for, and the example devices.
pub fn case_devices() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut out = Vec::new();
    for version in ["0.5", "0.6"] {
        let dir = root.join("conformance").join(version).join("read");
        let mut cases: Vec<_> = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).map(|e| e.unwrap().path()).collect();
        cases.sort();
        for case in cases {
            let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
            if meta["read"] == "device" && meta.get("result").is_some() {
                let text = std::fs::read_to_string(case.join("descriptor.json")).unwrap();
                out.push((format!("{version}/{}", case.file_name().unwrap().to_string_lossy()), text));
            }
        }
    }
    for example in ["examples/0.5/synth.device.json", "examples/devices/test_synth.json"] {
        out.push((example.to_string(), std::fs::read_to_string(root.join(example)).unwrap()));
    }
    out
}

/// The devices of `case_devices` that name a source, which an export needs.
pub fn credited_devices() -> Vec<(String, osd::Resolved)> {
    case_devices()
        .into_iter()
        .filter_map(|(name, text)| {
            let d = device_from(&text);
            d.sources.as_ref().is_some_and(|s| !s.is_empty()).then(|| (name, resolved(d)))
        })
        .collect()
}

/// The first line of `template.csv` of github.com/pencilresearch/midi (commit 215c64e8d3b3).
pub const HEADER: [&str; 18] = [
    "manufacturer",
    "device",
    "section",
    "parameter_name",
    "parameter_description",
    "cc_msb",
    "cc_lsb",
    "cc_min_value",
    "cc_max_value",
    "cc_default_value",
    "nrpn_msb",
    "nrpn_lsb",
    "nrpn_min_value",
    "nrpn_max_value",
    "nrpn_default_value",
    "orientation",
    "notes",
    "usage",
];

/// A source with its credit and licence.
pub fn source() -> osd::Source {
    osd::Source {
        kind: "community-re".to_string(),
        url: Some("https://github.com/pencilresearch/midi".to_string()),
        firmware: None,
        author: Some("Pencilresearch".to_string()),
        attribution: None,
        license: Some("CC-BY-SA-4.0".to_string()),
        commit: None,
        imported_at: None,
        notes: None,
        extra: Default::default(),
    }
}

/// One CSV line from its cells, quoted where a cell needs it.
pub fn line(cells: &[&str]) -> String {
    cells
        .iter()
        .map(|c| {
            if c.contains([',', '"', '\n']) {
                format!("\"{}\"", c.replace('"', "\"\""))
            } else {
                c.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub fn header_line() -> String {
    line(&HEADER)
}

/// A row of 18 cells, each empty but those named.
pub fn row(cells: &[(&str, &str)]) -> Vec<String> {
    let mut out = vec![String::new(); HEADER.len()];
    for (name, value) in cells {
        let at = HEADER.iter().position(|h| h == name).unwrap_or_else(|| panic!("{name} is not a column"));
        out[at] = value.to_string();
    }
    out
}

/// A file: the header, then each row, one line each.
pub fn csv(rows: &[Vec<String>]) -> String {
    let mut text = header_line();
    text.push('\n');
    for r in rows {
        let cells: Vec<&str> = r.iter().map(|c| c.as_str()).collect();
        text.push_str(&line(&cells));
        text.push('\n');
    }
    text
}

/// The records of a CSV text (RFC 4180 quoting; `\n` or `\r\n` ends a record).
pub fn parse(text: &str) -> Vec<Vec<String>> {
    let (mut records, mut record, mut cell) = (Vec::new(), Vec::new(), String::new());
    let (mut quoted, mut chars) = (false, text.chars().peekable());
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            ('"', true) => quoted = false,
            ('"', false) if cell.is_empty() => quoted = true,
            (',', false) => record.push(std::mem::take(&mut cell)),
            ('\r', false) => {}
            ('\n', false) => {
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
            }
            _ => cell.push(c),
        }
    }
    if !cell.is_empty() || !record.is_empty() {
        record.push(cell);
        records.push(record);
    }
    records
}

/// The device an import returned, as the program `osd export` would read it.
pub fn resolved(device: osd::Device) -> osd::Resolved {
    osd::Resolved { device, files: vec![], capabilities: vec![], inherited: vec![] }
}

/// A device from its JSON text, read strictly.
pub fn device_from(text: &str) -> osd::Device {
    match osd::read(text, osd::Kind::Device) {
        Ok(osd::Descriptor::Device(d)) => *d,
        Ok(_) => panic!("not a device"),
        Err(f) => panic!("the fixture is refused: {f:?}"),
    }
}

/// The first line of `template.triggers.csv` of github.com/pencilresearch/midi (commit 215c64e8d3b3).
pub const TRIGGER_HEADER: [&str; 11] = [
    "manufacturer",
    "device",
    "section",
    "sound_name",
    "sound_description",
    "midi_channel",
    "note_number_min_value",
    "note_number_max_value",
    "note_number_default_value",
    "comments",
    "velocity_usage",
];

pub fn trigger_header_line() -> String {
    line(&TRIGGER_HEADER)
}

/// A row of 11 cells, each empty but those named.
pub fn trigger_row(cells: &[(&str, &str)]) -> Vec<String> {
    let mut out = vec![String::new(); TRIGGER_HEADER.len()];
    for (name, value) in cells {
        let at = TRIGGER_HEADER.iter().position(|h| h == name).unwrap_or_else(|| panic!("{name} is not a trigger column"));
        out[at] = value.to_string();
    }
    out
}

/// A triggers file: the header, then each row, one line each.
pub fn trigger_csv(rows: &[Vec<String>]) -> String {
    let mut text = trigger_header_line();
    text.push('\n');
    for r in rows {
        let cells: Vec<&str> = r.iter().map(|c| c.as_str()).collect();
        text.push_str(&line(&cells));
        text.push('\n');
    }
    text
}

/// The trigger entries of a device.
pub fn triggers(d: &osd::Device) -> Vec<osd::Trigger> {
    d.triggers.as_ref().map(|t| t.entries.clone()).unwrap_or_default()
}

/// The controls of a device.
pub fn entries(d: &osd::Device) -> Vec<osd::CcEntry> {
    d.cc_params.as_ref().map(|c| c.entries.clone()).unwrap_or_default()
}
