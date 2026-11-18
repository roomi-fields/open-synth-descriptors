//! The pencilresearch/midi triggers CSV (`template.triggers.csv`): its importer and its exporter.
//! The fate of each column is written in docs/midi-triggers-csv.md.

use crate::csv;
use crate::midi_csv::{adopt_identity, blank_device, check_source, slug, unique, used_addresses};
use crate::{Fault, FaultKind, Loss, Translated};
use osd::{Device, Resolved, Source, Trigger, Triggers};
use serde_json::Value;
use std::collections::BTreeSet;

/// The header of `template.triggers.csv`, in order.
const HEADER: [&str; 11] = [
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

const MANUFACTURER: usize = 0;
const DEVICE: usize = 1;
const SECTION: usize = 2;
const NAME: usize = 3;
const DESCRIPTION: usize = 4;
const CHANNEL: usize = 5;
const NOTE_MIN: usize = 6;
const NOTE_MAX: usize = 7;
const NOTE_DEFAULT: usize = 8;
const COMMENTS: usize = 9;
const VELOCITY: usize = 10;

/// The remark of a trigger that holds the description the entry has no field for.
const DESCRIPTION_REMARK: &str = "_description";

fn fault(kind: FaultKind, place: impl Into<String>, rule: impl Into<String>) -> Fault {
    Fault { kind, place: place.into(), rule: rule.into() }
}

fn invalid(place: impl Into<String>, rule: impl Into<String>) -> Fault {
    fault(FaultKind::InvalidFile, place, rule)
}

fn cell_place(line: usize, column: usize) -> String {
    format!("line {line}, column {}", HEADER[column])
}

/// A number of a cell, `None` when the cell is empty.
fn number(cells: &[String], line: usize, column: usize) -> Result<Option<i64>, Fault> {
    let text = cells[column].trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<i64>().map(Some).map_err(|_| invalid(cell_place(line, column), format!("{text:?} is not a whole number")))
}

/// A note number of a cell: 0 to 127.
fn note(cells: &[String], line: usize, column: usize) -> Result<Option<u8>, Fault> {
    match number(cells, line, column)? {
        Some(n) if !(0..=127).contains(&n) => Err(invalid(cell_place(line, column), format!("{n} is outside 0 to 127"))),
        n => Ok(n.map(|n| n as u8)),
    }
}

fn lost(losses: &mut Vec<(usize, Loss)>, line: usize, column: usize, reason: &str) {
    losses.push((column, Loss { place: cell_place(line, column), reason: reason.to_string() }));
}

// ---- import -----------------------------------------------------------------------------

pub(crate) fn import(text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let (mut identity, mut used) = ((String::new(), String::new()), BTreeSet::new());
    let (entries, losses) = read(text, &mut identity, &mut used)?;
    let (vendor, name) = identity;
    let mut device = blank_device(name, Some(vendor).filter(|v| !v.is_empty()), source);
    device.triggers = Some(Triggers { entries, remarks: Default::default() });
    Ok(Translated { value: device, lost: losses })
}

/// Adds the triggers of `text` to `base`, which keeps its identity and its other blocks.
pub(crate) fn import_into(mut base: Device, text: &str, source: &Source) -> Result<Translated<Device>, Fault> {
    check_source(source)?;
    let mut identity = (base.device.vendor.clone().unwrap_or_default(), base.device.name.clone());
    let mut used = used_addresses(&base);
    let (entries, losses) = read(text, &mut identity, &mut used)?;
    adopt_identity(&mut base, identity);
    base.triggers.get_or_insert_with(|| Triggers { entries: Vec::new(), remarks: Default::default() }).entries.extend(entries);
    base.sources.get_or_insert_with(Vec::new).push(source.clone());
    Ok(Translated { value: base, lost: losses })
}

fn read(text: &str, identity: &mut (String, String), used: &mut BTreeSet<String>) -> Result<(Vec<Trigger>, Vec<Loss>), Fault> {
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
        let (lo, hi) = (note(cells, line, NOTE_MIN)?, note(cells, line, NOTE_MAX)?);
        let default = note(cells, line, NOTE_DEFAULT)?;
        if let (Some(a), Some(b)) = (lo, hi) {
            if a > b {
                return Err(invalid(cell_place(line, NOTE_MIN), format!("the minimum {a} is above the maximum {b}")));
            }
        }
        let (Some(lo), Some(hi)) = (lo, hi) else {
            let reason = match (lo, hi, default) {
                (Some(_), None, _) => "a trigger needs a note range: the maximum is empty",
                (None, Some(_), _) => "a trigger needs a note range: the minimum is empty",
                (None, None, Some(_)) => "a trigger needs a note range: the row has a default and no range",
                (None, None, None) => "a trigger needs a note range: the row has no note number",
                (Some(_), Some(_), _) => unreachable!(),
            };
            losses.push(Loss { place: format!("line {line}"), reason: reason.to_string() });
            continue;
        };

        let mut row_losses = Vec::new();
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

        let channel = match cells[CHANNEL].trim() {
            "" => None,
            text => match text.parse::<i64>() {
                Ok(n) if (1..=16).contains(&n) => Some((n - 1) as u8),
                _ => {
                    lost(&mut row_losses, line, CHANNEL, "OSD holds a channel from 1 to 16: this cell is not one");
                    None
                }
            },
        };
        let note_default = match default {
            Some(d) if (lo..=hi).contains(&d) => Some(d),
            Some(_) => {
                lost(&mut row_losses, line, NOTE_DEFAULT, "the default is outside the note range");
                None
            }
            None => None,
        };
        row_losses.sort_by_key(|(column, _)| *column);
        losses.extend(row_losses.into_iter().map(|(_, l)| l));

        let (section, name) = (slug(&cells[SECTION]), slug(&cells[NAME]));
        let joined: Vec<&str> = [section.as_str(), name.as_str()].into_iter().filter(|s| !s.is_empty()).collect();
        let path = if joined.is_empty() { format!("note-{lo}-{hi}") } else { joined.join("/") };
        let non_empty = |column: usize| Some(cells[column].clone()).filter(|s| !s.is_empty());
        let mut entry = Trigger {
            osc: unique(format!("/{path}"), used),
            sound: cells[NAME].clone(),
            section: non_empty(SECTION),
            channel,
            channel_mode: None,
            note_range: [lo as i64, hi as i64],
            note: note_default,
            velocity_usage: non_empty(VELOCITY),
            comments: non_empty(COMMENTS),
            remarks: Default::default(),
        };
        if let Some(d) = non_empty(DESCRIPTION) {
            entry.remarks.insert(DESCRIPTION_REMARK.to_string(), Value::String(d));
        }
        entries.push(entry);
    }
    Ok((entries, losses))
}

// ---- export -----------------------------------------------------------------------------

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
        ("/cc_params", d.cc_params.is_some()),
    ] {
        if present {
            gone(&mut losses, pointer.to_string(), "the CSV carries triggers only");
        }
    }

    if let Some(triggers) = &d.triggers {
        for key in triggers.remarks.keys() {
            gone(&mut losses, format!("/triggers/{key}"), "the CSV has no column for it");
        }
        for (n, e) in triggers.entries.iter().enumerate() {
            let at = |field: &str| format!("/triggers/entries/{n}/{field}");
            let mut row = vec![String::new(); HEADER.len()];
            row[MANUFACTURER] = d.device.vendor.clone().unwrap_or_default();
            row[DEVICE] = d.device.name.clone();
            row[SECTION] = e.section.clone().unwrap_or_default();
            row[NAME] = e.sound.clone();
            row[DESCRIPTION] = e.remarks.get(DESCRIPTION_REMARK).and_then(Value::as_str).unwrap_or_default().to_string();
            if let Some(c) = e.channel {
                row[CHANNEL] = (u16::from(c) + 1).to_string();
            }
            row[NOTE_MIN] = e.note_range[0].to_string();
            row[NOTE_MAX] = e.note_range[1].to_string();
            if let Some(v) = e.note {
                row[NOTE_DEFAULT] = v.to_string();
            }
            row[COMMENTS] = e.comments.clone().unwrap_or_default();
            row[VELOCITY] = e.velocity_usage.clone().unwrap_or_default();
            if e.channel_mode.is_some() {
                gone(&mut losses, at("channel_mode"), "the CSV has no column for it");
            }
            for key in e.remarks.keys().filter(|k| k.as_str() != DESCRIPTION_REMARK) {
                gone(&mut losses, at(key), "the CSV has no column for it");
            }
            if e.remarks.get(DESCRIPTION_REMARK).is_some_and(|v| !v.is_string()) {
                gone(&mut losses, at(DESCRIPTION_REMARK), "a cell holds text: this remark is not text");
            }
            let cells: Vec<&str> = row.iter().map(String::as_str).collect();
            out.push_str(&csv::line(&cells));
        }
    }

    for (pointer, present) in [
        ("/params", d.params.is_some()),
        ("/commands", d.commands.is_some()),
        ("/midi_out", d.midi_out.is_some()),
        ("/midi_in", d.midi_in.is_some()),
        ("/replies", d.replies.is_some()),
        ("/subscriptions", d.subscriptions.is_some()),
        ("/custom_commands", d.custom_commands.is_some()),
        ("/_coverage", d.coverage.is_some()),
    ] {
        if present {
            gone(&mut losses, pointer.to_string(), "the CSV carries triggers only");
        }
    }
    for key in d.remarks.keys() {
        gone(&mut losses, format!("/{key}"), "the CSV has no column for it");
    }
    Ok(Translated { value: out, lost: losses })
}
