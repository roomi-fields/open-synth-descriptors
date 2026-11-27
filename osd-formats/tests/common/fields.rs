//! What the tests of the two reviewed formats (the MIDI CSV and the Electra One) share: the
//! places a loss covers, the differences between a device and what the import of its export gives
//! back, the tables of the pages of the formats, the properties of the device schema, and a
//! device of 0.7 for each case of a field.

#![allow(dead_code)]

use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const SCHEMAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../schemas");

// ---- places -----------------------------------------------------------------------------

/// Whether `place` is `pointer` or lies under it.
pub fn under(place: &str, pointer: &str) -> bool {
    place == pointer || place.starts_with(&format!("{pointer}/"))
}

/// Whether a loss is at `pointer` or at the pointer of a block that holds it.
pub fn held(lost: &[Loss], pointer: &str) -> bool {
    lost.iter().any(|l| under(pointer, &l.place) && !l.reason.is_empty())
}

/// Whether a loss is at `pointer`, at a block that holds it, or at something it holds.
pub fn named(lost: &[Loss], pointer: &str) -> bool {
    held(lost, pointer) || lost.iter().any(|l| under(&l.place, pointer) && !l.reason.is_empty())
}

/// A key as a segment of a JSON pointer.
pub fn segment(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// The pointers where `back` is not `given`: a field missing, changed or added, at the
/// highest pointer that says it. A list, or the table `/cc_params`, that one side does not hold is
/// said item by item: what is lost is each of its items.
pub fn differences(given: &Value, back: &Value) -> Vec<String> {
    fn walk(a: Option<&Value>, b: Option<&Value>, at: String, out: &mut Vec<String>) {
        match (a, b) {
            (Some(Value::Object(x)), Some(Value::Object(y))) => {
                let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
                for k in keys {
                    walk(x.get(k), y.get(k), format!("{at}/{}", segment(k)), out);
                }
            }
            (Some(Value::Array(x)), Some(Value::Array(y))) => {
                for i in 0..x.len().max(y.len()) {
                    walk(x.get(i), y.get(i), format!("{at}/{i}"), out);
                }
            }
            (Some(Value::Array(x)), None) if !x.is_empty() => {
                for (i, item) in x.iter().enumerate() {
                    walk(Some(item), None, format!("{at}/{i}"), out);
                }
            }
            (Some(Value::Object(x)), None) if at == "/cc_params" => {
                for (k, v) in x {
                    walk(Some(v), None, format!("{at}/{}", segment(k)), out);
                }
            }
            (a, b) if a != b => out.push(at),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(Some(given), Some(back), String::new(), &mut out);
    out
}

// ---- a device through a format and back ---------------------------------------------------

/// The JSON a device is written as.
pub fn written(d: &osd::Device) -> Value {
    serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(d.clone())))).expect("a written device is JSON")
}

/// A device through a format: what was given, what importing the file written gives back (null
/// when the import refuses it) and the losses of the export.
pub struct Trip {
    pub given: Value,
    pub back: Value,
    pub lost: Vec<Loss>,
}

/// The source the import of the file is given: not that of the device.
fn import_source(format: Format) -> osd::Source {
    match format {
        Format::ElectraOne => super::electra::source(),
        _ => super::source(),
    }
}

pub fn trip(format: Format, device: osd::Device) -> Trip {
    let given = written(&device);
    let out = export(&super::resolved(device), format).unwrap_or_else(|f| panic!("{}: {f}\n{given}", format.name()));
    let back = match import(&out.value, format, &import_source(format)) {
        Ok(t) => written(&t.value),
        Err(_) => Value::Null,
    };
    Trip { given, back, lost: out.lost }
}

/// What breaks the rule of the losses of an export (docs/CADRE.md R8, docs/CADRE.md R16): a difference no loss
/// holds, and a loss that names a field given back unchanged.
pub fn broken_rules(t: &Trip) -> Vec<String> {
    let mut out = Vec::new();
    for d in differences(&t.given, &t.back) {
        if !held(&t.lost, &d) {
            out.push(format!("silent: {d} (given {:?}, back {:?})", t.given.pointer(&d), t.back.pointer(&d)));
        }
    }
    for l in &t.lost {
        if t.given.pointer(&l.place) == t.back.pointer(&l.place) {
            out.push(format!("named and unchanged: {} ({})", l.place, l.reason));
        }
    }
    out
}

// ---- the tables of a page -------------------------------------------------------------------

/// The page of a format, `osd-formats/docs/<name>.md`.
pub fn page(name: &str) -> String {
    let path = format!("{ROOT}/osd-formats/docs/{name}.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The rows of the table whose header's first cell is `header`: the cells of each row, each
/// stripped of its backticks, the header and its rule left out.
pub fn table(page_name: &str, header: &str) -> Vec<Vec<String>> {
    let cells = |l: &str| -> Vec<String> { l.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect() };
    let text = page(page_name);
    let mut lines = text.lines().map(str::trim);
    let mut found = false;
    let mut out = Vec::new();
    for l in lines.by_ref() {
        if !l.starts_with('|') {
            if found {
                break;
            }
            continue;
        }
        let row = cells(l);
        if !found {
            found = row[0] == header;
        } else if !row[0].starts_with('-') {
            out.push(row);
        }
    }
    assert!(found, "osd-formats/docs/{page_name}.md has no table whose header begins with `{header}`");
    out
}

/// The row of `name` in that table, the only one.
pub fn row_of(page_name: &str, header: &str, name: &str) -> Vec<String> {
    let mut rows: Vec<Vec<String>> = table(page_name, header).into_iter().filter(|r| r[0] == name).collect();
    assert_eq!(rows.len(), 1, "osd-formats/docs/{page_name}.md, table `{header}`: one row for `{name}`");
    rows.remove(0)
}

// ---- the properties of the device schema ----------------------------------------------------

/// The device schema of the highest version folder of schemas/.
pub fn highest_device_schema() -> Value {
    let version = std::fs::read_dir(SCHEMAS)
        .unwrap_or_else(|e| panic!("{SCHEMAS}: {e}"))
        .filter_map(|e| {
            let name = e.unwrap().file_name().to_string_lossy().into_owned();
            let (major, minor) = name.split_once('.')?;
            Some(((major.parse::<u32>().ok()?, minor.parse::<u32>().ok()?), name))
        })
        .max()
        .expect("a version folder in schemas/")
        .1;
    let path = format!("{SCHEMAS}/{version}/device.schema.json");
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))).unwrap()
}

/// The rows the table of the OSD device of a format holds: one per property of the schema in each
/// object the format enters, named by its path, and one for the `_` notes of each such object.
/// `commands`: the format enters a command and its arguments.
pub fn device_rows(commands: bool) -> BTreeSet<String> {
    let s = highest_device_schema();
    let mut objects: Vec<(&str, &Value)> = vec![
        ("", &s["properties"]),
        ("/device", &s["$defs"]["device"]["properties"]),
        ("/cc_params", &s["properties"]["cc_params"]["properties"]),
        ("/cc_params/entries/N", &s["$defs"]["cc_entry"]["properties"]),
        ("/cc_params/entries/N/ways/M", &s["$defs"]["cc_way"]["properties"]),
        ("/sources/N", &s["$defs"]["source"]["properties"]),
    ];
    if commands {
        objects.push(("/commands/N", &s["$defs"]["command"]["properties"]));
        objects.push(("/commands/N/args/M", &s["$defs"]["arg"]["properties"]));
    }
    let mut out = BTreeSet::new();
    for (at, properties) in objects {
        let properties = properties.as_object().unwrap_or_else(|| panic!("no properties for `{at}` in the device schema"));
        out.extend(properties.keys().map(|k| format!("{at}/{k}")));
        out.insert(format!("{at}/_*"));
    }
    out
}

// ---- the devices of the cases ---------------------------------------------------------------

/// A device of one control that its format gives back whole but for its sources, read strictly.
pub fn base(format: Format) -> Value {
    let mut way = json!({"ctrlType": "cc", "ctrlIndex": [74]});
    let mut doc = json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "vendor-doc", "url": "https://example.org/d", "license": "CC0-1.0"}],
        "device": {"name": "Trundler", "osc_prefix": "/trundler"},
    });
    if format == Format::ElectraOne {
        way["value_bits"] = json!(7);
        doc["commands"] = json!([{"osc": "/filter/dump", "title": "Dump", "section": "Filter", "frame": [240, 1, 247]}]);
    } else {
        doc["device"]["vendor"] = json!("Lofty");
    }
    doc["cc_params"] = json!({"entries": [{"osc": "/filter/cutoff", "title": "Cutoff", "section": "Filter", "ways": [way]}]});
    doc
}

/// Sets the value at a pointer, creating the objects on the way and appending to an array at its
/// length.
pub fn set(doc: &mut Value, pointer: &str, value: Value) {
    let (parent, last) = pointer.rsplit_once('/').expect("a pointer");
    let mut at = doc;
    for part in parent.split('/').skip(1) {
        at = match at {
            Value::Array(a) => &mut a[part.parse::<usize>().unwrap()],
            other => &mut other[part],
        };
    }
    match at {
        Value::Array(a) => {
            let i: usize = last.parse().unwrap();
            if i == a.len() {
                a.push(value);
            } else {
                a[i] = value;
            }
        }
        other => other[last.replace("~1", "/").replace("~0", "~")] = value,
    }
}

pub fn remove(doc: &mut Value, pointer: &str) {
    let (parent, last) = pointer.rsplit_once('/').expect("a pointer");
    let parent = doc.pointer_mut(parent).unwrap_or_else(|| panic!("{pointer}"));
    parent.as_object_mut().unwrap().remove(last);
}

/// The device of the base of a format, changed.
pub fn changed(format: Format, edits: &[(&str, Value)]) -> osd::Device {
    let mut doc = base(format);
    for (pointer, value) in edits {
        if value.is_null() {
            remove(&mut doc, pointer);
        } else {
            set(&mut doc, pointer, value.clone());
        }
    }
    super::device_from(&doc.to_string())
}
