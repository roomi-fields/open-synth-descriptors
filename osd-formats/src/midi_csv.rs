//! The pencilresearch/midi CSV (`template.csv`): its importer and its exporter. The fate of
//! each column is written in docs/midi-csv.md.

use crate::csv;
use crate::{Fault, FaultKind, Loss, Translated};
use osd::{CcEntry, CcParams, Device, DeviceInfo, Orientation, Resolved, Source, Version, Way};

use serde_json::Value;
use std::collections::BTreeSet;

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

/// The remark of a control that holds each text column the entry has no field for.
const REMARKS: [(&str, usize); 4] = [("_name", NAME), ("_description", DESCRIPTION), ("_notes", NOTES), ("_usage", USAGE)];

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

/// The credit an import needs: a way to find the source and its licence.
pub(crate) fn check_source(source: &Source) -> Result<(), Fault> {
    let named = |o: &Option<String>| o.as_deref().is_some_and(|s| !s.trim().is_empty());
    let mut missing = Vec::new();
    if !(named(&source.url) || named(&source.author) || named(&source.attribution)) {
        missing.push("url, author or attribution");
    }
    if !named(&source.license) {
        missing.push("license");
    }
    if missing.is_empty() {
        return Ok(());
    }
    let place = format!("source: {}", missing.join(" and "));
    Err(fault(FaultKind::SourceIncomplete, place, "an import carries the source's credit and licence: the source names one of url, author or attribution, and a license"))
}

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

/// A minimum and a maximum, refused when the minimum is above the maximum.
fn bounds(cells: &[String], line: usize, min: usize, max: usize) -> Result<(Option<i64>, Option<i64>), Fault> {
    let (lo, hi) = (number(cells, line, min)?, number(cells, line, max)?);
    match (lo, hi) {
        (Some(a), Some(b)) if a > b => Err(invalid(cell_place(line, min), format!("the minimum {a} is above the maximum {b}"))),
        _ => Ok((lo, hi)),
    }
}

/// The address of a control: its section and name as path segments.
pub(crate) fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// An address not yet used: `base`, else `base-2`, `base-3`…
pub(crate) fn unique(base: String, used: &mut BTreeSet<String>) -> String {
    let mut address = base.clone();
    let mut n = 2;
    while !used.insert(address.clone()) {
        address = format!("{base}-{n}");
        n += 1;
    }
    address
}

fn lost(losses: &mut Vec<(usize, Loss)>, line: usize, column: usize, reason: &str) {
    losses.push((column, Loss { place: cell_place(line, column), reason: reason.to_string() }));
}

pub(crate) fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let (mut identity, mut used) = ((String::new(), String::new()), BTreeSet::new());
    let (entries, losses) = read(text, &mut identity, &mut used, &mut BTreeSet::new(), None)?;
    let (vendor, name) = identity;
    let mut device = blank_device(name, Some(vendor).filter(|v| !v.is_empty()), source);
    device.cc_params = Some(CcParams { channel: None, entries, remarks: Default::default() });
    Ok(Translated { value: device, lost: losses })
}

/// The device of a first import: its identity and its one source, no block.
pub(crate) fn blank_device(name: String, vendor: Option<String>, source: &Source) -> Device {
    let prefix = format!("/{}", slug(&name));
    Device {
        schema: Some(Version::V0_6),
        device: DeviceInfo {
            name,
            vendor,
            revision: None,
            osc_prefix: prefix,
            manufacturer_id: None,
            device_id: None,
            rate_limit_hz: None,
            kind: None,
            transport: None,
            stands_for: None,
            fidelity: None,
            remarks: Default::default(),
        },
        capabilities: None,
        midi_of: None,
        loads_sounds_of: None,
        sound_transfers: None,
        resources: None,
        identity: None,
        programs: None,
        sysex: None,
        cc_params: None,
        params: None,
        commands: None,
        triggers: None,
        midi_out: None,
        midi_in: None,
        replies: None,
        subscriptions: None,
        custom_commands: None,
        sources: Some(vec![source.clone()]),
        coverage: None,
        role_vocabulary: None,
        remarks: Default::default(),
    }
}

/// The addresses a device already uses, controls and triggers.
pub(crate) fn used_addresses(base: &Device) -> BTreeSet<String> {
    let cc = base.cc_params.iter().flat_map(|c| c.entries.iter().map(|e| e.osc.clone()));
    let tr = base.triggers.iter().flat_map(|t| t.entries.iter().map(|e| e.osc.clone()));
    cc.chain(tr).collect()
}

/// Adds the controls of `text` to `base`, which keeps its identity and its other blocks.
pub(crate) fn import_into(mut base: Device, text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let mut identity = (base.device.vendor.clone().unwrap_or_default(), base.device.name.clone());
    let mut used = used_addresses(&base);
    let channel = base.cc_params.as_ref().and_then(|c| c.channel);
    let mut taken = claimed_by(&base);
    let (entries, losses) = read(text, &mut identity, &mut used, &mut taken, channel)?;
    adopt_identity(&mut base, identity);
    base.cc_params.get_or_insert_with(|| CcParams { channel: None, entries: Vec::new(), remarks: Default::default() }).entries.extend(entries);
    base.sources.get_or_insert_with(Vec::new).push(source.clone());
    Ok(Translated { value: base, lost: losses })
}

/// A message: its kind (CC, NRPN, RPN), its channel and its numbers. A 14-bit CC claims its CC
/// number and its LSB number, one message each.
pub(crate) type Claim = (u8, Option<u8>, Option<u8>, Option<u8>);

fn claims(cc: Option<u8>, cc_lsb: Option<u8>, nrpn: Option<(Option<u8>, Option<u8>)>, channel: Option<u8>) -> Vec<Claim> {
    let mut out: Vec<Claim> = [cc, cc_lsb].into_iter().flatten().map(|v| (0, channel, Some(v), None)).collect();
    if let Some((msb, lsb)) = nrpn {
        out.push((1, channel, msb, lsb));
    }
    out
}

/// The messages the controls of a device already claim.
pub(crate) fn claimed_by(base: &Device) -> BTreeSet<Claim> {
    let Some(c) = &base.cc_params else { return BTreeSet::new() };
    let mut out = BTreeSet::new();
    for e in &c.entries {
        let channel = e.channel.or(c.channel);
        for w in e.ways() {
            out.extend(claims(w.cc, w.cc_lsb, (w.nrpn_msb.is_some() || w.nrpn_lsb.is_some()).then_some((w.nrpn_msb, w.nrpn_lsb)), channel));
            if w.rpn_msb.is_some() || w.rpn_lsb.is_some() {
                out.insert((2, channel, w.rpn_msb, w.rpn_lsb));
            }
        }
    }
    out
}

/// A base that names no vendor or no device takes the first the file gave.
pub(crate) fn adopt_identity(base: &mut Device, identity: (String, String)) {
    if base.device.vendor.is_none() {
        base.device.vendor = Some(identity.0).filter(|v| !v.is_empty());
    }
    if base.device.name.is_empty() {
        base.device.name = identity.1;
    }
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
        // A message is claimed once: a way whose message an earlier control holds is left out.
        let cc_claims = claims(cc, cc_lsb, None, channel);
        let nrpn_claims = claims(None, None, has_nrpn.then_some((nrpn_msb, nrpn_lsb)), channel);
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
        let orientation = match cells[ORIENTATION].trim() {
            "" => None,
            "0-based" => Some(Orientation::ZeroBased),
            "centered" => Some(Orientation::Centered),
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
        // A default goes to the control of its address, when inside that control's range.
        let mut default_of = |default: Option<i64>, column: usize, has: bool, range: Option<[i64; 2]>| {
            let value = default?;
            if !has {
                lost(&mut row_losses, line, column, "no control carries this default: the row has no such address");
                return None;
            }
            match range {
                Some([lo, hi]) if value < lo || value > hi => {
                    lost(&mut row_losses, line, column, "the default is outside the range of its control");
                    None
                }
                _ => Some(value),
            }
        };
        let cc_default = default_of(cc_default, CC_DEFAULT, cc.is_some(), cc_range);
        let nrpn_default = default_of(nrpn_default, NRPN_DEFAULT, has_nrpn, nrpn_range);
        // One control has one default, in the space of its first way's range.
        // A lone default in the NRPN cell, with ranges that differ, makes the NRPN the first way.
        let mut nrpn_first = false;
        let (cc_default, nrpn_default) = match (cc.is_some() && has_nrpn, cc_default, nrpn_default) {
            (true, None, Some(v)) if cc_range == nrpn_range => (Some(v), None),
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
        let nrpn_way = Way { cc: None, cc_lsb: None, nrpn_msb, nrpn_lsb, rpn_msb: None, rpn_lsb: None, range: nrpn_range, remarks: Default::default() };
        let cc_way = || Way { cc, cc_lsb, nrpn_msb: None, nrpn_lsb: None, rpn_msb: None, rpn_lsb: None, range: cc_range, remarks: Default::default() };
        let (first, also) = match cc {
            Some(_) if nrpn_first => (nrpn_way, Some(vec![cc_way()])),
            Some(_) => (cc_way(), has_nrpn.then_some(vec![nrpn_way])),
            None => (nrpn_way, None),
        };
        let fallback = match cc {
            Some(n) => format!("cc-{n}"),
            None => format!("nrpn-{}-{}", nrpn_msb.map_or(String::new(), |n| n.to_string()), nrpn_lsb.map_or(String::new(), |n| n.to_string())),
        };
        let control = |first: Way, also: Option<Vec<Way>>, default, used: &mut BTreeSet<String>| {
            let path = base.clone().unwrap_or(fallback);
            let mut entry = CcEntry {
                osc: unique(format!("/{path}"), used),
                cc: first.cc,
                cc_lsb: first.cc_lsb,
                nrpn_msb: first.nrpn_msb,
                nrpn_lsb: first.nrpn_lsb,
                nrpn_lsb_range: None,
                rpn_msb: None,
                rpn_lsb: None,
                range: first.range,
                also,
                default,
                priority: None,
                step_count: None,
                transmit: None,
                recognize: None,
                values: None,
                orientation,
                channel: None,
                section: Some(cells[SECTION].clone()).filter(|s| !s.is_empty()),
                expects: None,
                transform: None,
                transform_reverse: None,
                role: None,
                remarks: Default::default(),
            };
            for (key, column) in REMARKS {
                if !cells[column].is_empty() {
                    entry.remarks.insert(key.to_string(), Value::String(cells[column].clone()));
                }
            }
            entry
        };
        entries.push(control(first, also, cc_default.or(nrpn_default), used));
    }

    Ok((entries, losses))
}

// ---- export -----------------------------------------------------------------------------

fn text_remark(entry: &CcEntry, key: &str) -> String {
    entry.remarks.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn put<T: ToString>(row: &mut [String], column: usize, value: Option<T>) {
    if let Some(v) = value {
        row[column] = v.to_string();
    }
}

fn gone(losses: &mut Vec<Loss>, place: String, reason: &str) {
    losses.push(Loss { place, reason: reason.to_string() });
}

pub(crate) fn export(resolved: &Resolved) -> Result<Translated<String>, Fault> {
    let d = &resolved.device;
    if d.sources.as_ref().is_none_or(|s| s.is_empty()) {
        return Err(fault(
            FaultKind::NoSources,
            "/_sources",
            "a device without sources is not written: its credit and licence cannot be",
        ));
    }
    let mut losses = Vec::new();
    let mut out = csv::line(&HEADER);
    gone(&mut losses, "/_sources".to_string(), "the CSV has no column for the credit or the licence of the sources");

    // The blocks of the device, in the order of its schema.
    let info = &d.device;
    for (pointer, present) in [
        ("/device/revision", info.revision.is_some()),
        ("/device/manufacturer_id", info.manufacturer_id.is_some()),
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
            let mut row = vec![String::new(); HEADER.len()];
            row[MANUFACTURER] = d.device.vendor.clone().unwrap_or_default();
            row[DEVICE] = d.device.name.clone();
            row[SECTION] = e.section.clone().unwrap_or_default();
            row[NAME] = match text_remark(e, "_name") {
                n if n.is_empty() => e.osc.clone(),
                n => n,
            };
            row[DESCRIPTION] = text_remark(e, "_description");
            row[NOTES] = text_remark(e, "_notes");
            row[USAGE] = text_remark(e, "_usage");
            // The row holds the first way with a CC and the first way with an NRPN; any other way
            // is a loss at its pointer.
            let ways = e.ways();
            let has_nrpn = |w: &Way| w.nrpn_msb.is_some() || w.nrpn_lsb.is_some();
            let cc_at = ways.iter().position(|w| w.cc.is_some());
            let nrpn_at = ways.iter().position(has_nrpn);
            for m in 1..ways.len() {
                if Some(m) != cc_at && Some(m) != nrpn_at {
                    gone(&mut losses, at(&format!("also/{}", m - 1)), "the row holds one CC way and one NRPN way: this way has no column");
                }
            }
            let (cc_way, nrpn_way) = (cc_at.map(|m| &ways[m]), nrpn_at.map(|m| &ways[m]));
            if let Some(w) = cc_way {
                put(&mut row, CC_MSB, w.cc);
                put(&mut row, CC_LSB, w.cc_lsb);
            }
            if let Some(w) = nrpn_way {
                put(&mut row, NRPN_MSB, w.nrpn_msb);
                put(&mut row, NRPN_LSB, w.nrpn_lsb);
            }
            put(&mut row, ORIENTATION, e.orientation.map(|o| if o == Orientation::Centered { "centered" } else { "0-based" }));
            // A range the two ways share is written once, in the CC columns.
            let cc_range = cc_way.and_then(|w| w.range);
            let nrpn_range = nrpn_way.and_then(|w| w.range).filter(|r| cc_range != Some(*r));
            for (range, min, max) in [(cc_range, CC_MIN, CC_MAX), (nrpn_range, NRPN_MIN, NRPN_MAX)] {
                if let Some([lo, hi]) = range {
                    row[min] = lo.to_string();
                    row[max] = hi.to_string();
                }
            }
            if cc_at.is_some() && cc_at == nrpn_at && cc_range.is_some() {
                gone(&mut losses, at("range"), "the one range goes to the CC columns; the NRPN columns stay empty");
            }
            if e.range.is_some() && cc_way.is_none() && nrpn_way.is_none() {
                gone(&mut losses, at("range"), "the entry has neither a CC nor an NRPN to give the range to");
            }
            // The CSV has no column for the order of the ways: the import puts the CC first, except
            // when a default in the NRPN column, beside a CC of another range, makes the NRPN first.
            let mut order_kept = false;
            if let Some(v) = e.default {
                // The default is in the space of the first way's range: it goes to the column of
                // the first way. An NRPN first way whose range differs from the CC's keeps it in
                // the NRPN column; with one range the import puts the CC first, so the CC column.
                let first_is_cc = cc_at == Some(0);
                let first_is_nrpn = nrpn_at == Some(0);
                if first_is_cc {
                    row[CC_DEFAULT] = v.to_string();
                } else if first_is_nrpn {
                    let same_range = cc_way.is_some() && cc_range == nrpn_way.and_then(|w| w.range);
                    if cc_way.is_some() && same_range {
                        row[CC_DEFAULT] = v.to_string();
                    } else {
                        row[NRPN_DEFAULT] = v.to_string();
                    }
                    order_kept = cc_way.is_some() && !same_range;
                } else if cc_way.is_some() || nrpn_way.is_some() {
                    gone(&mut losses, at("default"), "the default is in the space of the first way, which is neither a CC nor an NRPN: no column carries it");
                } else {
                    gone(&mut losses, at("default"), "the entry has neither a CC nor an NRPN to give the default to");
                }
            }
            if let (Some(c), Some(r)) = (cc_at, nrpn_at) {
                if r < c && !order_kept {
                    gone(&mut losses, format!("/cc_params/entries/{n}"),"the CSV does not say the order of the ways: the import puts the CC first");
                }
            }
            for (field, present) in [
                ("nrpn_lsb_range", e.nrpn_lsb_range.is_some()),
                ("rpn_msb", e.rpn_msb.is_some()),
                ("rpn_lsb", e.rpn_lsb.is_some()),
                ("channel", e.channel.is_some()),
                ("expects", e.expects.is_some()),
                ("transform", e.transform.is_some()),
                ("transform_reverse", e.transform_reverse.is_some()),
            ] {
                if present {
                    gone(&mut losses, at(field), "the CSV has no column for it");
                }
            }
            for key in e.remarks.keys().filter(|k| !REMARKS.iter().any(|(r, _)| r == k)) {
                gone(&mut losses, at(key), "the CSV has no column for it");
            }
            // A remark that is not text is no cell.
            for (key, _) in REMARKS {
                if e.remarks.get(key).is_some_and(|v| !v.is_string()) {
                    gone(&mut losses, at(key), "a cell holds text: this remark is not text");
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
