//! A driver as its firmware runs it: its shared MIDI (`midi_of`) followed,
//! then its blocks completed from their maker profiles. Each file is read
//! strictly before it is merged; the merge works on the JSON documents, by
//! the rules the conformance cases of `conformance/0.4/resolve/` fix.

use crate::json::{At, Object};
use crate::{read_value, Capability, Descriptor, Device, Fault, FaultKind, Kind};
use serde_json::Value;
use std::collections::HashSet;

/// Where the files of a catalogue come from: the text of a descriptor by its
/// catalogue id (`maker/name`, the file `<maker>/<name>.json`; the profile
/// `korg/logue` is the file `korg/_profile.logue`), or none.
pub trait Files {
    fn get(&self, id: &str) -> Option<String>;
}

impl Files for std::collections::BTreeMap<String, String> {
    fn get(&self, id: &str) -> Option<String> {
        std::collections::BTreeMap::get(self, id).cloned()
    }
}

impl Files for std::collections::HashMap<String, String> {
    fn get(&self, id: &str) -> Option<String> {
        std::collections::HashMap::get(self, id).cloned()
    }
}

/// A device with its shared MIDI and maker profiles applied.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub device: Device,
    /// The catalogue ids of the files it was built from, the driver first.
    pub files: Vec<String>,
    /// What it requires from its reader beyond the declarative fields.
    pub capabilities: Vec<Capability>,
}

/// The `device` keys that say which firmware a file is, not how it speaks
/// MIDI: a file with `midi_of` keeps these and takes the rest from its target.
const IDENTITY: [&str; 5] = ["name", "vendor", "revision", "osc_prefix", "kind"];

/// Route lists a `midi_of` file may add to its target's, by OSC address.
const OWN_LISTS: [&str; 2] = ["commands", "custom_commands"];
/// Route tables (`entries`) a `midi_of` file may add to its target's.
const OWN_TABLES: [&str; 3] = ["params", "cc_params", "triggers"];
/// Blocks a `midi_of` file may hold in place of its target's: how this
/// firmware is recognised and what its program changes recall.
const OWN_BLOCKS: [&str; 2] = ["identity", "programs"];

/// The most `midi_of` links followed from one file.
const MIDI_OF_DEPTH: usize = 8;
/// The most profile links followed from one block.
const PROFILE_DEPTH: usize = 8;

/// The driver `id` as its firmware runs it. A file naming another with
/// `midi_of` (its catalogue id, `m-vave/fm-1.vendor.fw-15`: a firmware that
/// shares that MIDI interface) keeps its own identity — the `_` keys and
/// the `device` keys of `IDENTITY` — and takes everything else from the
/// target, a driver of the same maker and device; the target may itself
/// name another, up to `MIDI_OF_DEPTH` links and never in a circle. The
/// file may add routes of its own (`OWN_LISTS`, the `entries` of
/// `OWN_TABLES`), appended to the target's, its own winning on the same
/// OSC address; its own `OWN_BLOCKS` replace the target's. Then each block
/// naming a maker profile is completed from it (`complete`).
pub fn resolve(id: &str, files: &dyn Files) -> Result<Resolved, Fault> {
    let mut read = Vec::new();
    let (doc, typed) = fetch(id, files, Kind::Device, &mut read)?
        .ok_or_else(|| At::Root.fault(FaultKind::LinkMissing, format!("no file {id}")).in_file(id))?;
    let linked = doc.get("midi_of").is_some() || OWN_BLOCKS.iter().any(|k| doc.get(*k).and_then(|b| b.get("profile")).is_some());
    let device = match typed {
        // A driver that names no other file is already read.
        Descriptor::Device(d) if !linked => *d,
        _ => {
            let mut doc = follow(id, doc, files, &mut Vec::new(), &mut read)?;
            let maker = maker_of(id).unwrap_or_default().to_string();
            for key in OWN_BLOCKS {
                if let Some(block) = doc.get(key).cloned() {
                    doc[key] = complete(&maker, key, block, id, files, &mut Vec::new(), &mut read)?;
                }
            }
            Device::read(&doc, &At::Root).map_err(|f| f.in_file(id))?
        }
    };
    let capabilities = device.derived_capabilities();
    Ok(Resolved { device, files: read, capabilities })
}

/// The maker of a catalogue id: `korg` for `korg/minilogue`.
fn maker_of(id: &str) -> Option<&str> {
    id.split_once('/').map(|(m, _)| m)
}

/// The document of file `id` and its strict reading as a `kind`, noted as read.
fn fetch(id: &str, files: &dyn Files, kind: Kind, read: &mut Vec<String>) -> Result<Option<(Value, Descriptor)>, Fault> {
    let Some(text) = files.get(id) else { return Ok(None) };
    let doc: Value = serde_json::from_str(&text).map_err(|e| At::Root.fault(FaultKind::Syntax, e.to_string()).in_file(id))?;
    let typed = read_value(&doc, kind).map_err(|f| f.in_file(id))?;
    if !read.iter().any(|r| r == id) {
        read.push(id.to_string());
    }
    Ok(Some((doc, typed)))
}

fn follow(id: &str, doc: Value, files: &dyn Files, chain: &mut Vec<String>, read: &mut Vec<String>) -> Result<Value, Fault> {
    let Some(target) = doc.get("midi_of") else { return Ok(doc) };
    let fault = |kind: FaultKind, pointer: String, why: String| Fault { kind, file: Some(id.to_string()), pointer, rule: format!("midi_of {target}: {why}") };
    let link = |kind: FaultKind, why: &str| fault(kind, "/midi_of".into(), why.into());
    let tid = target.as_str().ok_or_else(|| link(FaultKind::BadValue, "not a catalogue id"))?;
    let parts: Vec<&str> = tid.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|c| c.is_empty() || c.starts_with('.')) {
        return Err(link(FaultKind::BadValue, "not a catalogue id (<maker>/<file name without .json>)"));
    }
    if maker_of(id) != Some(parts[0]) {
        return Err(link(FaultKind::BadValue, "names another maker's driver"));
    }
    let allowed = |k: &str| k.starts_with('_') || ["device", "midi_of"].contains(&k)
        || OWN_LISTS.contains(&k) || OWN_TABLES.contains(&k) || OWN_BLOCKS.contains(&k);
    if let Some(own) = doc.as_object().and_then(|o| o.keys().find(|k| !allowed(k))) {
        return Err(fault(FaultKind::UnknownField, At::Key(&At::Root, own).pointer(), format!("{own} is the target's (a midi_of file adds routes only)")));
    }
    // The file's own routes: each address once.
    let osc = |e: &Value| e.get("osc").and_then(Value::as_str).map(str::to_string);
    let entries_of = |v: &Value, key: &str| -> Vec<Value> {
        let list = if OWN_LISTS.contains(&key) { v.get(key) } else { v.get(key).and_then(|t| t.get("entries")) };
        list.and_then(Value::as_array).cloned().unwrap_or_default()
    };
    for key in OWN_LISTS.iter().chain(OWN_TABLES.iter()) {
        let mut seen = HashSet::new();
        for (i, e) in entries_of(&doc, key).iter().enumerate() {
            let place = if OWN_LISTS.contains(key) { format!("/{key}/{i}") } else { format!("/{key}/entries/{i}") };
            let a = osc(e).ok_or_else(|| fault(FaultKind::Missing, format!("{place}/osc"), format!("a {key} entry has no osc address")))?;
            if !seen.insert(a.clone()) {
                return Err(fault(FaultKind::BadValue, format!("{place}/osc"), format!("{a} is twice in its own {key}")));
            }
        }
    }
    let (out, _) = fetch(tid, files, Kind::Device, read)?.ok_or_else(|| link(FaultKind::LinkMissing, &format!("no file {tid}")))?;
    chain.push(id.to_string());
    if chain.iter().any(|p| p == tid) {
        return Err(link(FaultKind::LinkCycle, "the links come back to this file (a circle)"));
    }
    if chain.len() > MIDI_OF_DEPTH {
        return Err(link(FaultKind::LinkTooDeep, &format!("more than {MIDI_OF_DEPTH} links")));
    }
    let mut out = follow(tid, out, files, chain, read)?;
    let name = |d: &Value| d.pointer("/device/name").and_then(Value::as_str).map(str::to_string);
    if name(&doc).is_some() && name(&doc) != name(&out) {
        return Err(link(FaultKind::BadValue, "the target is another device"));
    }
    // Appended to the target's routes, its own replacing one at the same address.
    let merge = |base: Vec<Value>, own: Vec<Value>| -> Vec<Value> {
        let mine: HashSet<_> = own.iter().filter_map(osc).collect();
        base.into_iter().filter(|e| osc(e).is_none_or(|a| !mine.contains(&a))).chain(own).collect()
    };
    let obj = out.as_object_mut().ok_or_else(|| link(FaultKind::BadValue, "the target is not a driver"))?;
    // Credit for the MIDI taken from the target: its sources and those it took from.
    let shared: Vec<Value> = ["_sources", "_midi_of_sources"].iter()
        .filter_map(|k| obj.get(*k).and_then(Value::as_array)).flatten().cloned().collect();
    obj.retain(|k, _| !k.starts_with('_'));
    let mut device = obj.remove("device").unwrap_or_else(|| Value::Object(Default::default()));
    for (k, v) in doc.as_object().into_iter().flatten() {
        if k == "device" {
            for key in IDENTITY {
                match v.get(key) {
                    Some(x) => { device[key] = x.clone(); }
                    None => { device.as_object_mut().map(|d| d.remove(key)); }
                }
            }
        } else if OWN_LISTS.contains(&k.as_str()) {
            let base = match obj.get_mut(k).map(Value::take) { Some(Value::Array(a)) => a, _ => Vec::new() };
            obj.insert(k.clone(), Value::Array(merge(base, entries_of(&doc, k))));
        } else if OWN_TABLES.contains(&k.as_str()) {
            match obj.get_mut(k).filter(|t| t.is_object()) {
                Some(table) => {
                    if let Some(extra) = v.as_object().and_then(|t| t.keys().find(|x| *x != "entries")) {
                        return Err(fault(FaultKind::UnknownField, format!("/{k}/{extra}"), format!("{k}.{extra} is the target's (a midi_of file adds entries only)")));
                    }
                    let base = match table.get_mut("entries").map(Value::take) { Some(Value::Array(a)) => a, _ => Vec::new() };
                    table["entries"] = Value::Array(merge(base, entries_of(&doc, k)));
                }
                None => { obj.insert(k.clone(), v.clone()); }
            }
        } else {
            obj.insert(k.clone(), v.clone());
        }
    }
    if !shared.is_empty() {
        obj.insert("_midi_of_sources".into(), Value::Array(shared));
    }
    obj.insert("device".into(), device);
    Ok(out)
}

/// Where profile `id` is kept, next to its maker's drivers: `korg` is the
/// file `korg/_profile`, `korg/logue` the file `korg/_profile.logue`.
fn profile_file(id: &str) -> Option<String> {
    let (maker, family) = match id.split_once('/') { Some((m, f)) => (m, Some(f)), None => (id, None) };
    let ok = |s: &str| !s.is_empty() && !s.starts_with('.') && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.+".contains(&b));
    if !ok(maker) || family.is_some_and(|f| !ok(f) || f.contains('/')) { return None; }
    Some(match family { Some(f) => format!("{maker}/_profile.{f}"), None => format!("{maker}/_profile") })
}

/// What a maker's models share is written once, in a profile kept in the
/// maker's folder (`profile_file`): `"identity": {"profile": "roland",
/// "family": [89, 1], "member": [0, 0]}` takes the Roland identity request,
/// reply pattern and firmware rule, the model giving its bytes. A block
/// naming a profile is that profile's block (itself completed from the
/// profile it names, up to `PROFILE_DEPTH` links, never in a circle) with
/// the block's own keys in place of the profile's; a key set to null drops
/// the profile's. The profile is the same maker's. `from` is the file that
/// writes the block.
fn complete(maker: &str, key: &str, block: Value, from: &str, files: &dyn Files, chain: &mut Vec<String>, read: &mut Vec<String>) -> Result<Value, Fault> {
    let Some(pid) = block.get("profile").cloned() else { return Ok(block) };
    let link = |kind: FaultKind, why: String| Fault { kind, file: Some(from.to_string()), pointer: format!("/{key}/profile"), rule: format!("profile {pid}: {why}") };
    let pid = pid.as_str().ok_or_else(|| link(FaultKind::BadValue, "not a profile id".into()))?.to_string();
    let file = profile_file(&pid).ok_or_else(|| link(FaultKind::BadValue, "not a profile id (<maker> or <maker>/<name>)".into()))?;
    if file.split('/').next() != Some(maker) { return Err(link(FaultKind::BadValue, "another maker's".into())); }
    if chain.contains(&pid) { return Err(link(FaultKind::LinkCycle, "the links come back to it (a circle)".into())); }
    chain.push(pid.clone());
    if chain.len() > PROFILE_DEPTH { return Err(link(FaultKind::LinkTooDeep, format!("more than {PROFILE_DEPTH} profile links"))); }
    let (prof, _) = fetch(&file, files, Kind::Profile, read)?.ok_or_else(|| link(FaultKind::LinkMissing, format!("no file {file}")))?;
    let base = prof.get(key).cloned().ok_or_else(|| link(FaultKind::LinkMissing, format!("the profile has no {key}")))?;
    let mut out = complete(maker, key, base, &file, files, chain, read)?;
    let obj = out.as_object_mut().ok_or_else(|| link(FaultKind::BadValue, format!("its {key} is not an object")))?;
    for (k, v) in block.as_object().into_iter().flatten() {
        match v {
            Value::Null => { obj.remove(k); }
            v => { obj.insert(k.clone(), v.clone()); }
        }
    }
    obj.insert("profile".into(), Value::String(pid));
    Ok(out)
}
