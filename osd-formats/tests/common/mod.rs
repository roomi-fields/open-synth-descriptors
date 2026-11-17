//! What the tests of osd-formats share: the columns of the pencilresearch/midi CSV, rows
//! built cell by cell, a small CSV reader, and a source with its credit.

#![allow(dead_code)]

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

/// The controls of a device.
pub fn entries(d: &osd::Device) -> Vec<osd::CcEntry> {
    d.cc_params.as_ref().map(|c| c.entries.clone()).unwrap_or_default()
}
