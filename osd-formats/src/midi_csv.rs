//! The pencilresearch/midi CSV (`template.csv`): its importer and its exporter. The fate of
//! each column is written in docs/midi-csv.md.

use crate::csv;
use crate::importer::{adopt_identity, blank_device, check_new_id, check_source, read_back, slug, unique, used_addresses, way_claims, Claim};
use crate::{lose_what_no_way_says, Contract, Fault, FaultKind, Loss, Translated};
use osd::{CcEntry, CcParams, Device, Polarity, Resolved, Source, Way};

use std::collections::BTreeSet;

pub(crate) const CONTRACT: Contract = Contract {
    name: "midi-csv",
    media_type: "text/csv",
    extension: "csv",
    import,
    export,
    import_into: Some(import_into),
};

/// The header of `template.csv`, in order.
const HEADER: [&str; 18] = [
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

const MANUFACTURER: usize = 0;
const DEVICE: usize = 1;
const SECTION: usize = 2;
const NAME: usize = 3;
const DESCRIPTION: usize = 4;
const CC_MSB: usize = 5;
const CC_LSB: usize = 6;
const CC_MIN: usize = 7;
const CC_MAX: usize = 8;
const CC_DEFAULT: usize = 9;
const NRPN_MSB: usize = 10;
const NRPN_LSB: usize = 11;
const NRPN_MIN: usize = 12;
const NRPN_MAX: usize = 13;
const NRPN_DEFAULT: usize = 14;
const ORIENTATION: usize = 15;
const NOTES: usize = 16;
const USAGE: usize = 17;

/// The cell of a text column, `None` when it is empty: no field is written for an empty cell.
fn non_empty(cell: &str) -> Option<String> {
    Some(cell.to_string()).filter(|c| !c.is_empty())
}

fn fault(kind: FaultKind, place: impl Into<String>, rule: impl Into<String>) -> Fault {
    Fault { kind, place: place.into(), rule: rule.into() }
}

fn invalid(place: impl Into<String>, rule: impl Into<String>) -> Fault {
    fault(FaultKind::InvalidFile, place, rule)
}

fn cell_place(line: usize, column: usize) -> String {
    format!("line {line}, column {}", HEADER[column])
}

// ---- import -----------------------------------------------------------------------------

/// A number of a cell, `None` when the cell is empty.
fn number(cells: &[String], line: usize, column: usize) -> Result<Option<i64>, Fault> {
    let text = cells[column].trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<i64>().map(Some).map_err(|_| invalid(cell_place(line, column), format!("{text:?} is not a whole number")))
}

/// A CC or NRPN number of a cell: 0 to 127.
fn midi_number(cells: &[String], line: usize, column: usize) -> Result<Option<u8>, Fault> {
    match number(cells, line, column)? {
        Some(n) if !(0..=127).contains(&n) => Err(invalid(cell_place(line, column), format!("{n} is outside 0 to 127"))),
        n => Ok(n.map(|n| n as u8)),
    }
}

/// Whether the default of the NRPN cell, alone beside a CC, makes the NRPN the first way: the
/// ranges differ, or the row gives none and the default is above 127. The importer orders the ways
/// by it, and the exporter chooses the column of a default by it.
fn nrpn_default_comes_first(cc_range: Option<[i64; 2]>, nrpn_range: Option<[i64; 2]>, default: i64) -> bool {
    cc_range != nrpn_range || (cc_range.is_none() && default > 127)
}

/// A minimum and a maximum, refused when the minimum is above the maximum.
fn bounds(cells: &[String], line: usize, min: usize, max: usize) -> Result<(Option<i64>, Option<i64>), Fault> {
    let (lo, hi) = (number(cells, line, min)?, number(cells, line, max)?);
    match (lo, hi) {
        (Some(a), Some(b)) if a > b => Err(invalid(cell_place(line, min), format!("the minimum {a} is above the maximum {b}"))),
        _ => Ok((lo, hi)),
    }
}

fn lost(losses: &mut Vec<(usize, Loss)>, line: usize, column: usize, reason: &str) {
    losses.push((column, Loss { place: cell_place(line, column), reason: reason.to_string() }));
}

fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let (mut identity, mut used) = ((String::new(), String::new()), BTreeSet::new());
    let (entries, losses) = read(text, &mut identity, &mut used, &mut BTreeSet::new(), None)?;
    let (vendor, name) = identity;
    let mut device = blank_device(name, Some(vendor).filter(|v| !v.is_empty()), source);
    device.cc_params = Some(CcParams { channel: None, entries, remarks: Default::default() });
    read_back(Translated { value: device, lost: losses })
}

/// Adds the controls of `text` to `base`, which keeps its identity and its other blocks; a base
/// before 0.7 is first converted to 0.7, its blocks as the converter gives them.
fn import_into(base: Device, text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    // The entries added carry `title`, a field from 0.7: they join a base of 0.7.
    let mut base = crate::base::in_0_7(base)?;
    check_new_id(&base, source)?;
    let mut identity = (base.device.vendor.clone().unwrap_or_default(), base.device.name.clone());
    let mut used = used_addresses(&base);
    let channel = base.cc_params.as_ref().and_then(|c| c.channel);
    let mut taken = claimed_by(&base);
    let (entries, losses) = read(text, &mut identity, &mut used, &mut taken, channel)?;
    adopt_identity(&mut base, identity);
    base.cc_params.get_or_insert_with(|| CcParams { channel: None, entries: Vec::new(), remarks: Default::default() }).entries.extend(entries);
    base.sources.get_or_insert_with(Vec::new).push(source.clone());
    read_back(Translated { value: base, lost: losses })
}

/// The messages the controls of a device already claim.
pub(crate) fn claimed_by(base: &Device) -> BTreeSet<Claim> {
    let Some(c) = &base.cc_params else { return BTreeSet::new() };
    let mut out = BTreeSet::new();
    for e in &c.entries {
        let channel = e.channel.or(c.channel);
        out.extend(e.ways.iter().flat_map(|w| way_claims(w, channel)));
    }
    out
}

fn read(text: &str, identity: &mut (String, String), used: &mut BTreeSet<String>, taken: &mut BTreeSet<Claim>, channel: Option<u8>) -> Result<(Vec<CcEntry>, Vec<Loss>), Fault> {
    let records = csv::parse(text).map_err(|u| invalid(format!("line {}", u.0), "a quoted cell is not closed"))?;
    let Some(head) = records.first() else {
        return Err(invalid("line 1", "the file is empty: the header of the format is missing"));
    };
    if head.cells != HEADER {
        return Err(invalid("line 1", format!("the header is not the format's: {}", HEADER.join(","))));
    }

    let (mut entries, mut losses) = (Vec::new(), Vec::new());
    for record in &records[1..] {
        let (line, cells) = (record.line, &record.cells);
        if cells.len() != HEADER.len() {
            return Err(invalid(format!("line {line}"), format!("{} cells, the header has {}", cells.len(), HEADER.len())));
        }
        let mut cc = midi_number(cells, line, CC_MSB)?;
        let mut cc_lsb = midi_number(cells, line, CC_LSB)?;
        let nrpn_msb = midi_number(cells, line, NRPN_MSB)?;
        let nrpn_lsb = midi_number(cells, line, NRPN_LSB)?;
        let (cc_lo, cc_hi) = bounds(cells, line, CC_MIN, CC_MAX)?;
        let (nrpn_lo, nrpn_hi) = bounds(cells, line, NRPN_MIN, NRPN_MAX)?;
        let cc_default = number(cells, line, CC_DEFAULT)?;
        let nrpn_default = number(cells, line, NRPN_DEFAULT)?;
        if cc.is_none() && cc_lsb.is_some() {
            return Err(invalid(cell_place(line, CC_LSB), "a CC LSB without its CC"));
        }
        let mut has_nrpn = nrpn_msb.is_some() || nrpn_lsb.is_some();
        if cc.is_none() && !has_nrpn {
            losses.push(Loss {
                place: format!("line {line}"),
                reason: "the row has neither a CC nor an NRPN: it is no control".to_string(),
            });
            continue;
        }

        let mut row_losses = Vec::new();
        // A 14-bit pair is two distinct numbers: a CC LSB equal to its CC leaves one 7-bit CC.
        if cc.is_some() && cc_lsb == cc {
            lost(&mut row_losses, line, CC_LSB, "a 14-bit pair takes two distinct numbers: the CC is written alone, on 7 bits");
            cc_lsb = None;
        }
        // An NRPN is its MSB and its LSB: half of one is no message.
        if has_nrpn && (nrpn_msb.is_none() || nrpn_lsb.is_none()) {
            let column = if nrpn_msb.is_none() { NRPN_MSB } else { NRPN_LSB };
            lost(&mut row_losses, line, column, "an NRPN needs its MSB and its LSB: this way is left out");
            has_nrpn = false;
            if cc.is_none() {
                row_losses.sort_by_key(|(column, _)| *column);
                losses.extend(row_losses.into_iter().map(|(_, l)| l));
                continue;
            }
        }
        // A message is claimed once: a way whose message an earlier control holds is left out.
        let claims_of = |kind: &str, numbers: [Option<u8>; 2]| {
            let numbers = numbers.into_iter().flatten().map(i64::from).collect();
            way_claims(&Way { ctrl_type: kind.into(), ctrl_index: Some(numbers), ..Default::default() }, channel)
        };
        let cc_claims = claims_of("cc", [cc, cc_lsb]);
        let nrpn_claims = if has_nrpn { claims_of("nrpn", [nrpn_msb, nrpn_lsb]) } else { Vec::new() };
        if cc_claims.iter().any(|c| taken.contains(c)) {
            lost(&mut row_losses, line, CC_MSB, "the CC is already claimed by an earlier control: this way is left out");
            (cc, cc_lsb) = (None, None);
        }
        if nrpn_claims.iter().any(|c| taken.contains(c)) {
            lost(&mut row_losses, line, NRPN_MSB, "the NRPN is already claimed by an earlier control: this way is left out");
            has_nrpn = false;
        }
        if cc.is_none() && !has_nrpn {
            row_losses.sort_by_key(|(column, _)| *column);
            losses.extend(row_losses.into_iter().map(|(_, l)| l));
            continue;
        }
        taken.extend(if cc.is_some() { cc_claims } else { Vec::new() });
        taken.extend(if has_nrpn { nrpn_claims } else { Vec::new() });
        // An empty cell names nothing; the first name a row gives is the descriptor's.
        for (column, known, reason) in [
            (MANUFACTURER, &mut identity.0, "the descriptor names one manufacturer: the first row's"),
            (DEVICE, &mut identity.1, "the descriptor names one device: the first row's"),
        ] {
            let given = &cells[column];
            if given.is_empty() {
                continue;
            }
            if known.is_empty() {
                known.clone_from(given);
            } else if known != given {
                lost(&mut row_losses, line, column, reason);
            }
        }

        let range = |lo: Option<i64>, hi: Option<i64>, min: usize, max: usize, row_losses: &mut Vec<(usize, Loss)>| {
            match (lo, hi) {
                (Some(a), Some(b)) => Some([a, b]),
                (Some(_), None) => {
                    lost(row_losses, line, min, "a range needs both bounds: the maximum is empty");
                    None
                }
                (None, Some(_)) => {
                    lost(row_losses, line, max, "a range needs both bounds: the minimum is empty");
                    None
                }
                (None, None) => None,
            }
        };
        let polarity = match cells[ORIENTATION].trim() {
            "" => None,
            "0-based" => Some(Polarity::Unipolar),
            "centered" => Some(Polarity::Bipolar),
            _ => {
                lost(&mut row_losses, line, ORIENTATION, "OSD holds `0-based` and `centered`: this word is neither");
                None
            }
        };
        let cc_range = if cc.is_some() {
            range(cc_lo, cc_hi, CC_MIN, CC_MAX, &mut row_losses)
        } else {
            for column in [CC_MIN, CC_MAX] {
                if !cells[column].trim().is_empty() {
                    lost(&mut row_losses, line, column, "the row has no CC for this range");
                }
            }
            None
        };
        let nrpn_range = if has_nrpn {
            range(nrpn_lo, nrpn_hi, NRPN_MIN, NRPN_MAX, &mut row_losses)
        } else {
            for column in [NRPN_MIN, NRPN_MAX] {
                if !cells[column].trim().is_empty() {
                    lost(&mut row_losses, line, column, "the row has no NRPN for this range");
                }
            }
            None
        };
        // A row with a CC and an NRPN is one control with two ways; the one range a row gives
        // goes to both ways.
        let (cc_range, nrpn_range) = if cc.is_some() && has_nrpn { (cc_range.or(nrpn_range), nrpn_range.or(cc_range)) } else { (cc_range, nrpn_range) };
        // The CSV does not say the width of an NRPN value: the full 14 bits.
        let nrpn_way = has_nrpn.then(|| Way {
            ctrl_type: "nrpn".into(),
            ctrl_index: Some(vec![i64::from(nrpn_msb.unwrap_or_default()), i64::from(nrpn_lsb.unwrap_or_default())]),
            value_bits: Some(14),
            range: nrpn_range,
            ..Default::default()
        });
        let cc_way = cc.map(|_| Way {
            ctrl_type: "cc".into(),
            ctrl_index: Some([cc, cc_lsb].into_iter().flatten().map(i64::from).collect()),
            value_bits: cc_lsb.map(|_| 14),
            range: cc_range,
            ..Default::default()
        });
        // A default goes to the control of its address, when inside that control's scale: its
        // range, else the whole of what its message carries.
        let mut default_of = |default: Option<i64>, column: usize, way: Option<&Way>| {
            let value = default?;
            let Some(way) = way else {
                lost(&mut row_losses, line, column, "no control carries this default: the row has no such address");
                return None;
            };
            let [lo, hi] = way.scale();
            if value < lo || value > hi {
                let reason = match way.range {
                    Some(_) => "the default is outside the range of its control",
                    None => "the default is outside the scale of its control",
                };
                lost(&mut row_losses, line, column, reason);
                return None;
            }
            Some(value)
        };
        let cc_default = default_of(cc_default, CC_DEFAULT, cc_way.as_ref());
        let nrpn_default = default_of(nrpn_default, NRPN_DEFAULT, nrpn_way.as_ref());
        // One control has one default, in the space of its first way's range.
        // A lone default in the NRPN cell makes the NRPN the first way when the ranges differ, or
        // when there is none and the default does not fit 7 bits.
        let mut nrpn_first = false;
        let (cc_default, nrpn_default) = match (cc.is_some() && has_nrpn, cc_default, nrpn_default) {
            (true, None, Some(v)) if !nrpn_default_comes_first(cc_range, nrpn_range, v) => (Some(v), None),
            (true, None, Some(v)) => {
                nrpn_first = true;
                (None, Some(v))
            }
            (true, _, Some(_)) => {
                lost(&mut row_losses, line, NRPN_DEFAULT, "a control has one default, in the space of its first way: the NRPN's is left");
                (cc_default, None)
            }
            _ => (cc_default, nrpn_default),
        };
        row_losses.sort_by_key(|(column, _)| *column);
        losses.extend(row_losses.into_iter().map(|(_, l)| l));

        let base = {
            let (section, name) = (slug(&cells[SECTION]), slug(&cells[NAME]));
            let joined: Vec<&str> = [section.as_str(), name.as_str()].into_iter().filter(|s| !s.is_empty()).collect();
            if joined.is_empty() {
                None
            } else {
                Some(joined.join("/"))
            }
        };
        // The CC is the first way, the NRPN the way after it; a row with an NRPN alone has it first.
        let mut ways: Vec<Way> = cc_way.into_iter().chain(nrpn_way).collect();
        if nrpn_first {
            ways.reverse();
        }
        let fallback = match cc {
            Some(n) => format!("cc-{n}"),
            None => format!("nrpn-{}-{}", nrpn_msb.map_or(String::new(), |n| n.to_string()), nrpn_lsb.map_or(String::new(), |n| n.to_string())),
        };
        let control = |ways: Vec<Way>, default: Option<i64>, used: &mut BTreeSet<String>| {
            let path = base.clone().unwrap_or(fallback);
            // The default is in the space of the first way's range: when another way is more
            // precise, that range is the scale the control is read in.
            let scale = default.and_then(|_| {
                let first = ways[0].scale();
                let rest = ways[1..].iter().map(|w| w.scale()[1]).max();
                rest.is_some_and(|r| r > first[1]).then_some(first)
            });
            // The literal survives the addition of a field: `..Default::default()` closes it.
            #[allow(clippy::needless_update)]
            let entry = CcEntry {
                osc: unique(format!("/{path}"), used),
                title: non_empty(&cells[NAME]),
                description: non_empty(&cells[DESCRIPTION]),
                value_description: non_empty(&cells[USAGE]),
                notes: non_empty(&cells[NOTES]),
                scale,
                ways,
                default,
                priority: None,
                step_count: None,
                values: None,
                polarity,
                channel: None,
                section: Some(cells[SECTION].clone()).filter(|s| !s.is_empty()),
                expects: None,
                transform: None,
                transform_reverse: None,
                role: None,
                remarks: Default::default(),
                ..Default::default()
            };
            entry
        };
        entries.push(control(ways, cc_default.or(nrpn_default), used));
    }

    Ok((entries, losses))
}

// ---- export -----------------------------------------------------------------------------

fn put<T: ToString>(row: &mut [String], column: usize, value: Option<T>) {
    if let Some(v) = value {
        row[column] = v.to_string();
    }
}

fn gone(losses: &mut Vec<Loss>, place: String, reason: &str) {
    losses.push(Loss { place, reason: reason.to_string() });
}

fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    if d.sources.as_ref().is_none_or(|s| s.is_empty()) {
        return Err(fault(
            FaultKind::NoSources,
            "/sources",
            "a device without sources is not written: its credit and licence cannot be",
        ));
    }
    let mut losses = Vec::new();
    let mut out = csv::line(&HEADER);
    gone(&mut losses, "/sources".to_string(), "the CSV has no column for the credit or the licence of the sources");

    // The blocks of the device, in the order of its schema.
    let info = &d.device;
    for (pointer, present) in [
        ("/device/revision", info.revision.is_some()),
        ("/identity/manufacturerId", d.identity.as_ref().is_some_and(|i| i.manufacturer_id.is_some())),
        ("/device/device_id", info.device_id.is_some()),
        ("/device/rate_limit_hz", info.rate_limit_hz.is_some()),
        ("/device/kind", info.kind.is_some()),
        ("/device/transport", info.transport.is_some()),
    ] {
        if present {
            gone(&mut losses, pointer.to_string(), "the CSV has no column for it");
        }
    }
    for key in info.remarks.keys() {
        gone(&mut losses, format!("/device/{key}"), "the CSV has no column for it");
    }
    for (pointer, present) in [
        ("/capabilities", d.capabilities.is_some()),
        ("/midi_of", d.midi_of.is_some()),
        ("/loads_sounds_of", d.loads_sounds_of.is_some()),
        ("/identity", d.identity.is_some()),
        ("/programs", d.programs.is_some()),
        ("/sysex", d.sysex.is_some()),
    ] {
        if present {
            gone(&mut losses, pointer.to_string(), "the CSV carries controls only");
        }
    }

    if let Some(cc) = &d.cc_params {
        if cc.channel.is_some() {
            gone(&mut losses, "/cc_params/channel".to_string(), "the CSV has no column for a channel");
        }
        for key in cc.remarks.keys() {
            gone(&mut losses, format!("/cc_params/{key}"), "the CSV has no column for it");
        }
        for (n, e) in cc.entries.iter().enumerate() {
            let at = |field: &str| format!("/cc_params/entries/{n}/{field}");
            lose_what_no_way_says(e, n, &mut losses);
            let mut row = vec![String::new(); HEADER.len()];
            row[MANUFACTURER] = d.device.vendor.clone().unwrap_or_default();
            row[DEVICE] = d.device.name.clone();
            row[SECTION] = e.section.clone().unwrap_or_default();
            row[NAME] = e.title.clone().unwrap_or_else(|| e.osc.clone());
            row[DESCRIPTION] = e.description.clone().unwrap_or_default();
            row[NOTES] = e.notes.clone().unwrap_or_default();
            row[USAGE] = e.value_description.clone().unwrap_or_default();
            // The row holds the first way with a CC and the first way with an NRPN; any other way
            // is a loss at its pointer.
            let ways = &e.ways;
            let index = |w: &Way, n: usize| w.ctrl_index.as_ref().and_then(|i| i.get(n)).copied();
            let cc_at = ways.iter().position(|w| w.ctrl_type == "cc");
            let nrpn_at = ways.iter().position(|w| w.ctrl_type == "nrpn");
            for (m, w) in ways.iter().enumerate() {
                if Some(m) != cc_at && Some(m) != nrpn_at {
                    let reason = match w.ctrl_type.as_str() {
                        "cc" | "nrpn" => "the row holds one CC way and one NRPN way: this way has no column".to_string(),
                        kind => format!("the row holds a CC way and an NRPN way: a {kind} message has no column"),
                    };
                    gone(&mut losses, at(&format!("ways/{m}")), &reason);
                }
            }
            let (cc_way, nrpn_way) = (cc_at.map(|m| &ways[m]), nrpn_at.map(|m| &ways[m]));
            if let Some(w) = cc_way {
                put(&mut row, CC_MSB, index(w, 0));
                put(&mut row, CC_LSB, index(w, 1));
            }
            // Where the loss of the width of the NRPN way goes, once the bounds of the row are known.
            let width_loss_at = losses.len();
            if let Some(w) = nrpn_way {
                put(&mut row, NRPN_MSB, index(w, 0));
                put(&mut row, NRPN_LSB, index(w, 1));
                if w.nrpn_lsb_range.is_some() {
                    gone(&mut losses, at(&format!("ways/{}/nrpn_lsb_range", nrpn_at.unwrap_or_default())), "the CSV has no column for it");
                }
            }
            put(&mut row, ORIENTATION, e.polarity.map(|o| if o == Polarity::Bipolar { "centered" } else { "0-based" }));
            // The ranges of the row. The import gives the one range of a row to both ways: when
            // one way alone has a range and the message of the other implies another scale, both
            // scales are written.
            let (mut cc_range, mut nrpn_range) = (cc_way.and_then(|w| w.range), nrpn_way.and_then(|w| w.range));
            if let (Some(c), Some(n)) = (cc_way, nrpn_way) {
                if c.range.is_some() != n.range.is_some() && c.scale() != n.scale() {
                    (cc_range, nrpn_range) = (Some(c.scale()), Some(n.scale()));
                }
            }
            // The CSV has no column for the order of the ways: the import puts the CC first, except
            // when a default in the NRPN column makes the NRPN first ([`nrpn_default_comes_first`]).
            let mut order_kept = false;
            if let Some(v) = e.default {
                // The default is in the reference scale of the control. It goes, as the way carries
                // it, to the column of the way whose scale is the reference one: the NRPN's when its
                // scale differs from the CC's, else the CC's (the import puts the CC first).
                let reference = e.reference_scale();
                let cc_scale = cc_way.map(Way::scale);
                // The way that gives the reference scale is the first way of that scale.
                let reference_at = ways.iter().position(|w| w.scale() == reference);
                // The import gives the one range of a row to both ways.
                let nrpn_column_keeps_order = nrpn_default_comes_first(cc_range.or(nrpn_range), nrpn_range.or(cc_range), v);
                let shared = cc_scale == Some(reference) && reference_at == nrpn_at && !nrpn_column_keeps_order;
                let (in_cc, in_nrpn) = (reference_at == cc_at || shared, reference_at == nrpn_at);
                if in_cc && reference_at.is_some() {
                    row[CC_DEFAULT] = v.to_string();
                } else if in_nrpn && reference_at.is_some() {
                    row[NRPN_DEFAULT] = v.to_string();
                    // Alone on its row, an NRPN of 7 bits without a range gets no bounds, and
                    // the import reads it on 14 bits.
                    if cc_way.is_none() && nrpn_way.is_some_and(|w| w.value_bits != Some(14) && w.range.is_none()) {
                        gone(&mut losses, at("default"), "the CSV does not say the width of an NRPN value: the import reads this default on the scale 0..16383, not 0..127");
                    }
                    // A default up to 127 alone beside a CC, with no range on the row, would
                    // come back on the CC: the ranges the ways imply are written, which differ,
                    // so the import puts the NRPN first and the default keeps its scale.
                    if let (Some(c), Some(n)) = (cc_way, nrpn_way) {
                        if c.range.is_none() && n.range.is_none() && !nrpn_default_comes_first(None, None, v) {
                            (cc_range, nrpn_range) = (Some(c.scale()), Some(n.scale()));
                        }
                    }
                    if cc_way.is_some() {
                        order_kept = nrpn_at < cc_at;
                        if cc_at < nrpn_at {
                            gone(&mut losses, format!("/cc_params/entries/{n}"), "the CSV does not say the order of the ways: the default in the NRPN column makes the import put the NRPN first");
                        }
                    }
                } else if cc_way.is_some() || nrpn_way.is_some() {
                    gone(&mut losses, at("default"), "the default is in the reference scale of a way that is neither the CC nor the NRPN written: no column carries it");
                } else {
                    gone(&mut losses, at("default"), "the entry has neither a CC nor an NRPN to give the default to");
                }
            }
            // The CSV does not say the width of an NRPN value: the import reads 14 bits. No range
            // is added for 7 bits: on a way of 14 bits, 0..127 would send the data LSB alone.
            if let Some(m) = nrpn_at.filter(|&m| ways[m].value_bits != Some(14)) {
                let mut reason = "the CSV has no column for the width of a value: the import reads this NRPN on 14 bits (data MSB and LSB)".to_string();
                // The import gives the one range of a row to both ways.
                if cc_range.is_none() && nrpn_range.is_none() {
                    reason.push_str(", and its scale becomes 0..16383");
                }
                losses.insert(width_loss_at, Loss { place: at(&format!("ways/{m}/value_bits")), reason });
            }
            // A range the two ways share is written once, in the CC columns.
            let nrpn_range = nrpn_range.filter(|r| cc_range != Some(*r));
            for (range, min, max) in [(cc_range, CC_MIN, CC_MAX), (nrpn_range, NRPN_MIN, NRPN_MAX)] {
                if let Some([lo, hi]) = range {
                    row[min] = lo.to_string();
                    row[max] = hi.to_string();
                }
            }
            if let (Some(c), Some(r)) = (cc_at, nrpn_at) {
                if r < c && !order_kept {
                    gone(&mut losses, format!("/cc_params/entries/{n}"),"the CSV does not say the order of the ways: the import puts the CC first");
                }
            }
            for (field, present) in [
                ("channel", e.channel.is_some()),
                ("expects", e.expects.is_some()),
                ("transform", e.transform.is_some()),
                ("transform_reverse", e.transform_reverse.is_some()),
            ] {
                if present {
                    gone(&mut losses, at(field), "the CSV has no column for it");
                }
            }
            for key in e.remarks.keys() {
                gone(&mut losses, at(key), "the CSV has no column for it");
            }
            // A control with no way writes its row, which no import takes for a control, and the
            // fields only such an entry carries.
            if ways.is_empty() {
                gone(&mut losses, format!("/cc_params/entries/{n}"), "the control gives no message: the row has neither a CC nor an NRPN, and an import drops it");
                for (field, present) in [
                    ("scale", e.scale.is_some()),
                    ("range", e.range.is_some()),
                    ("transmit", e.transmit.is_some()),
                    ("recognize", e.recognize.is_some()),
                    ("nrpn_lsb_range", e.nrpn_lsb_range.is_some()),
                ] {
                    if present {
                        gone(&mut losses, at(field), "the CSV writes it for no message: the row does not carry it");
                    }
                }
            }
            let cells: Vec<&str> = row.iter().map(String::as_str).collect();
            out.push_str(&csv::line(&cells));
        }
    }

    for (pointer, present) in [
        ("/params", d.params.is_some()),
        ("/commands", d.commands.is_some()),
        ("/triggers", d.triggers.is_some()),
        ("/midi_out", d.midi_out.is_some()),
        ("/midi_in", d.midi_in.is_some()),
        ("/replies", d.replies.is_some()),
        ("/subscriptions", d.subscriptions.is_some()),
        ("/custom_commands", d.custom_commands.is_some()),
        ("/_coverage", d.coverage.is_some()),
    ] {
        if present {
            gone(&mut losses, pointer.to_string(), "the CSV carries controls only");
        }
    }
    for key in d.remarks.keys() {
        gone(&mut losses, format!("/{key}"), "the CSV has no column for it");
    }
    Ok(Translated { value: out, lost: losses })
}
