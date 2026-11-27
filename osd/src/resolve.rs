//! A driver as its firmware runs it: its shared MIDI (`midi_of`) followed,
//! then its blocks completed from their maker profiles. Each file is read
//! strictly before it is merged (23.33); the merge works on the JSON documents,
//! by the rules of the section Resolution of `spec/0.6.md` (23.3 to 23.34),
//! each held by conformance cases.

use crate::convert::drop_lua_capability_no_field_requires;
use crate::device::PROFILE_OF_SOURCE;
use crate::json::{At, Object};
use crate::{convert_to, read_value, Capability, Descriptor, Device, Fault, FaultKind, Kind, Source, Version, VERSIONS};
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

/// The files of a catalogue, with the text of one id replaced: how an assembly
/// has a module's MIDI follow the MIDI accumulated before it.
pub(crate) struct Overlay<'a> {
    pub base: &'a dyn Files,
    pub id: &'a str,
    pub text: String,
}

impl Files for Overlay<'_> {
    fn get(&self, id: &str) -> Option<String> {
        if id == self.id { Some(self.text.clone()) } else { self.base.get(id) }
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
    /// The capabilities the merge requires that the driver's own text does
    /// not: those only a maker profile or a `midi_of` target brings.
    pub inherited: Vec<Capability>,
}

/// The `device` keys that say which firmware a file is, not how it speaks
/// MIDI: a file with `midi_of` keeps these and takes the rest from its target
/// (23.4, section Resolution of `spec/0.6.md`).
/// From 0.7 `revision` is no field and the keys are `IDENTITY_0_7`: a pair of two files of 0.6 keeps
/// `revision` among them (a published version does not change).
const IDENTITY: [&str; 5] = ["name", "vendor", "revision", "osc_prefix", "kind"];
/// The identity keys of a pair of which either file is of 0.7 or later.
const IDENTITY_0_7: [&str; 4] = ["name", "vendor", "osc_prefix", "kind"];

/// Route lists a `midi_of` file may add to its target's, by OSC address
/// (23.5, 23.7, 23.9; section Resolution of `spec/0.6.md`).
const OWN_LISTS: [&str; 2] = ["commands", "custom_commands"];
/// Route tables (`entries`) a `midi_of` file may add to its target's
/// (23.6, 23.7, 23.10; section Resolution of `spec/0.6.md`).
const OWN_TABLES: [&str; 3] = ["params", "cc_params", "triggers"];
/// Blocks a `midi_of` file may hold in place of its target's: how this
/// firmware is recognised and what its program changes recall (23.8, 23.23;
/// section Resolution of `spec/0.6.md`).
const OWN_BLOCKS: [&str; 2] = ["identity", "programs"];

/// The most `midi_of` links followed from one file.
const MIDI_OF_DEPTH: usize = 8;
/// The most profile links followed from one block.
const PROFILE_DEPTH: usize = 8;

/// The driver `id` as its firmware runs it, by the rules 23.3 to 23.34 of the
/// section Resolution of `spec/0.6.md`: `follow` applies those of
/// `midi_of` (23.4 to 23.22), `complete_from_profiles` those of profiles (23.23 to 23.32). A file naming another with
/// `midi_of` (its catalogue id, `m-vave/fm-1.vendor.fw-15`: a firmware that
/// shares that MIDI interface) keeps its own identity — the `_` keys and
/// the `device` keys of `IDENTITY` — and takes everything else from the
/// target, a driver of the same maker and device; the target may itself
/// name another, up to `MIDI_OF_DEPTH` links and never in a circle. The
/// file may add routes of its own (`OWN_LISTS`, the `entries` of
/// `OWN_TABLES`), appended to the target's, its own winning on the same
/// OSC address; its own `OWN_BLOCKS` replace the target's, and from 0.7 it names
/// `role_vocabulary` for the roles of its own routes: the state of its target, or any when the
/// target names none. The two files may
/// be of two versions: the older text is converted to the newer one's version
/// (`convert_to`, forward only) before they are joined, so the document states
/// the more recent of the two; a conversion that fails refuses the resolution
/// in the file it fails in. Then each block
/// naming a maker profile is completed from it (`complete_from_profiles`), every
/// text older than 0.7 carried to 0.7 first when the joined device or one of
/// those profiles is of 0.7.
pub fn resolve(id: &str, files: &dyn Files) -> Result<Resolved, Fault> {
    let (mut resolved, credit) = resolve_uncredited(id, files)?;
    credit_profiles(&mut resolved.device, credit);
    Ok(resolved)
}

/// [`resolve`] before the credit of the profiles is appended, given beside the device for
/// [`credit_profiles`]: the device holds the sources of its files alone, which are those a reading
/// names and those whose `id` a file may not repeat.
pub(crate) fn resolve_uncredited(id: &str, files: &dyn Files) -> Result<(Resolved, Vec<ProfileCredit>), Fault> {
    let mut read = Vec::new();
    let (doc, typed) = fetch(id, files, Kind::Device, &mut read)?
        .ok_or_else(|| At::Root.fault(FaultKind::LinkMissing, format!("no file {id}")).in_file(id))?;
    let linked = doc.get("midi_of").is_some() || OWN_BLOCKS.iter().any(|k| doc.get(*k).and_then(|b| b.get("profile")).is_some());
    // What the driver's own text requires, linked or not: never inherited.
    let own = match &typed {
        Descriptor::Device(d) => d.derived_capabilities(),
        _ => Vec::new(),
    };
    let (device, credit) = match typed {
        // A driver that names no other file is already read.
        Descriptor::Device(d) if !linked => (*d, Vec::new()),
        _ => {
            let declared = doc.get("capabilities").cloned();
            let asked = version_of(&doc);
            // The blocks the file asked for writes itself, before its target's join them.
            let own_blocks: Vec<&str> = OWN_BLOCKS.into_iter().filter(|k| doc.get(*k).is_some()).collect();
            let mut doc = follow(id, doc, asked, files, &mut Vec::new(), &mut read)?;
            let (converted, credit) = complete_from_profiles(&mut doc, id, &own_blocks, files, &mut read)?;
            // The capabilities declared were checked on the driver's own text; the
            // merge keeps what the driver declared, not its target's.
            match declared {
                Some(c) => doc["capabilities"] = c,
                None => { doc.as_object_mut().map(|o| o.remove("capabilities")); }
            }
            if converted {
                // A field the conversion removes may have been the only reason of a declared `lua`.
                if let Some(o) = doc.as_object_mut() {
                    drop_lua_capability_no_field_requires(o, Kind::Device);
                }
            }
            crate::sound_transfer::check_controls(&doc, true).map_err(|f| f.in_file(id))?;
            let device = crate::device::read_device(&doc, &At::Root).map_err(|f| f.in_file(id))?;
            // The `source` of a reading is judged here, in the union of the file's sources and its target's.
            if device.schema >= Some(Version::V0_7) {
                crate::device::check_reading_sources(&device).map_err(|f| f.in_file(id))?;
            }
            (device, credit)
        }
    };
    // The device a virtual instrument stands for is a file of the catalogue.
    if let Some(target) = &device.device.stands_for {
        if files.get(target).is_none() {
            let rule = format!("stands_for {target}: no file {target}");
            return Err(Fault { kind: FaultKind::BadValue, file: Some(id.to_string()), pointer: "/device/stands_for".into(), rule });
        }
    }
    let capabilities = device.derived_capabilities();
    let inherited = capabilities.iter().copied().filter(|c| !own.contains(c)).collect();
    Ok((Resolved { device, files: read, capabilities, inherited }, credit))
}

/// The complete descriptor of a resolved device: the same device without its links, so that a
/// reader needs no catalogue. `midi_of` is removed (the routes of its target are already merged
/// in), the name of the profile is removed from `identity` and `programs` (the keys it brought
/// stay), and `capabilities` is the list the merge requires. `resolve` itself is unchanged
/// (23.24 keeps the profile name); the result reads without fault by `read` (section 21).
pub fn complete_descriptor(resolved: &Resolved) -> Descriptor {
    let own = Descriptor::Device(Box::new(resolved.device.clone()));
    let mut doc: Value = serde_json::from_str(&crate::write(&own)).unwrap_or(Value::Null);
    if let Some(o) = doc.as_object_mut() {
        o.remove("midi_of");
        for key in OWN_BLOCKS {
            if let Some(b) = o.get_mut(key).and_then(Value::as_object_mut) {
                b.remove("profile");
            }
        }
        if !resolved.capabilities.is_empty() && (resolved.device.capabilities.is_some() || resolved.device.schema != Some(Version::V0_4)) {
            let words: Vec<Value> = resolved.capabilities.iter().map(|c| Value::from(c.word())).collect();
            o.insert("capabilities".into(), Value::Array(words));
        }
    }
    read_value(&doc, Kind::Device).unwrap_or(own)
}

/// The maker of a catalogue id: `korg` for `korg/minilogue`.
fn maker_of(id: &str) -> Option<&str> {
    id.split_once('/').map(|(m, _)| m)
}

/// The document of file `id` and its strict reading as a `kind`, noted as read.
pub(crate) fn fetch(id: &str, files: &dyn Files, kind: Kind, read: &mut Vec<String>) -> Result<Option<(Value, Descriptor)>, Fault> {
    let Some(text) = files.get(id) else { return Ok(None) };
    let doc: Value = serde_json::from_str(&text).map_err(|e| At::Root.fault(FaultKind::Syntax, e.to_string()).in_file(id))?;
    let typed = read_value(&doc, kind).map_err(|f| f.in_file(id))?;
    if !read.iter().any(|r| r == id) {
        read.push(id.to_string());
    }
    Ok(Some((doc, typed)))
}

/// The version a document states in its `$schema`; 0.4 when it has none (a
/// document already read strictly names a version this crate reads).
fn version_of(doc: &Value) -> Version {
    doc.get("$schema").and_then(Value::as_str)
        .and_then(|s| VERSIONS.iter().copied().find(|v| v.address("device") == s))
        .unwrap_or(Version::V0_4)
}

/// The document `doc` of file `file`, of kind `kind`, carried forward to version
/// `to`; a conversion that fails is the fault of that file.
fn upgrade(doc: &Value, kind: Kind, to: Version, file: &str) -> Result<Value, Fault> {
    let text = convert_to(&doc.to_string(), kind, to).map_err(|f| f.in_file(file))?;
    serde_json::from_str(&text).map_err(|e| At::Root.fault(FaultKind::Syntax, e.to_string()).in_file(file))
}

/// The document of `id` joined to its `midi_of` target, followed to the end:
/// rules 23.4 to 23.22 of the section Resolution of `spec/0.6.md`. `text` is the
/// document as the file writes it and `asked` the version of the file that follows
/// it (its own for the file asked for): an older text is converted to `asked`
/// before it is joined. What the file may hold is judged on `text`, in its own
/// version, before any conversion (23.5, 23.6, 23.9, 23.10, 23.14): from 0.6
/// `capabilities`, and a `params` table without its three shape keys when the target
/// has one.
fn follow(id: &str, text: Value, asked: Version, files: &dyn Files, chain: &mut Vec<String>, read: &mut Vec<String>) -> Result<Value, Fault> {
    let text_version = version_of(&text);
    let from_06 = text_version >= Version::V0_6;
    let text_07 = text_version >= Version::V0_7;
    let carried = |doc: &Value| if text_version < asked { upgrade(doc, Kind::Device, asked, id) } else { Ok(doc.clone()) };
    let Some(target) = text.get("midi_of") else { return carried(&text) };
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
    let allowed = |k: &str| k.starts_with('_') || ["$schema", "device", "midi_of"].contains(&k) || (k == "sources" && text_07)
        || (from_06 && k == "capabilities") || OWN_LISTS.contains(&k) || OWN_TABLES.contains(&k) || OWN_BLOCKS.contains(&k);
    let targets_own = |own: &str| fault(FaultKind::UnknownField, At::Key(&At::Root, own).pointer(), format!("{own} is the target's (a midi_of file adds routes only)"));
    // `role_vocabulary` waits for the target: another state than its own is refused as such (23.35),
    // and a file before 0.7 may not name one at all (23.9).
    if let Some(own) = text.as_object().and_then(|o| o.keys().find(|k| *k != "role_vocabulary" && !allowed(k))) {
        return Err(targets_own(own));
    }
    let doc = carried(&text)?;
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
    // The older of the two texts is carried to the newer one's version before
    // they are joined (the target as it is followed, the file after that).
    let own_version = version_of(&doc);
    let target_text_version = version_of(&out);
    chain.push(id.to_string());
    if chain.iter().any(|p| p == tid) {
        return Err(link(FaultKind::LinkCycle, "the links come back to this file (a circle)"));
    }
    if chain.len() > MIDI_OF_DEPTH {
        return Err(link(FaultKind::LinkTooDeep, &format!("more than {MIDI_OF_DEPTH} links")));
    }
    let mut out = follow(tid, out, own_version, files, chain, read)?;
    if let Some(state) = text.get("role_vocabulary") {
        // Each file's roles are read against the state it names: states are never merged.
        if out.get("role_vocabulary").is_some_and(|t| t != state) {
            return Err(fault(FaultKind::BadValue, "/role_vocabulary".into(), "the file names another state of the vocabulary than its target's".into()));
        }
        // From 0.7 the file adds the roles of its own routes: it names the state of its target,
        // or the one of the result when its target names none.
        if !text_07 {
            return Err(targets_own("role_vocabulary"));
        }
    }
    let target_version = version_of(&out);
    let doc = if own_version < target_version { upgrade(&doc, Kind::Device, target_version, id)? } else { doc.clone() };
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
    // From 0.7 the credit is `sources` and `midi_of_sources`; before, the `_` keys.
    let (sources_key, shared_key) = if target_version >= Version::V0_7 { ("sources", "midi_of_sources") } else { ("_sources", "_midi_of_sources") };
    let mut shared: Vec<Value> = [sources_key, shared_key].iter()
        .filter_map(|k| obj.get(*k).and_then(Value::as_array)).flatten().cloned().collect();
    // From 0.7 the `id` of the sources of the file and of its target are one set: each is unique.
    if target_version >= Version::V0_7 {
        let taken: HashSet<&str> = shared.iter().filter_map(|s| s.get("id").and_then(Value::as_str)).collect();
        for key in [sources_key, shared_key] {
            let own = doc.get(key).and_then(Value::as_array).into_iter().flatten();
            for (n, id) in own.enumerate().filter_map(|(n, s)| s.get("id").and_then(Value::as_str).map(|i| (n, i))) {
                if taken.contains(id) {
                    // The key as the file's text writes it: before 0.7, under its `_`.
                    let written = if text_07 { key.to_string() } else { format!("_{key}") };
                    return Err(fault(FaultKind::BadValue, format!("/{written}/{n}"), format!("the id {id} is already a source id of the target")));
                }
            }
        }
    }
    // The credit the file itself says it took from others (before 0.7, `_midi_of_sources`)
    // follows its target's.
    let own_shared = doc.get(shared_key).and_then(Value::as_array).cloned().unwrap_or_default();
    obj.retain(|k, _| !k.starts_with('_') && !(k == "sources" || k == "midi_of_sources"));
    let mut device = obj.remove("device").unwrap_or_else(|| Value::Object(Default::default()));
    for (k, v) in doc.as_object().into_iter().flatten() {
        if k == "$schema" {
            // Both texts are of one version by now (the older was converted): the target's stands.
        } else if k == "device" {
            // The rule follows the declared version of the pair, read before the conversion
            // that removes `revision` made the two texts one version.
            let keys: &[&str] = if own_version.max(target_text_version) >= Version::V0_7 { &IDENTITY_0_7 } else { &IDENTITY };
            for &key in keys {
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
                    if let Some(extra) = text.get(k).and_then(Value::as_object).and_then(|t| t.keys().find(|x| *x != "entries")) {
                        return Err(fault(FaultKind::UnknownField, format!("/{k}/{extra}"), format!("{k}.{extra} is the target's (a midi_of file adds entries only)")));
                    }
                    let base = match table.get_mut("entries").map(Value::take) { Some(Value::Array(a)) => a, _ => Vec::new() };
                    table["entries"] = Value::Array(merge(base, entries_of(&doc, k)));
                }
                None => {
                    // Taken as it is: from 0.6 it must then hold the shape keys its target would have given.
                    if k == "params" && from_06 {
                        // Its entries are read with the device, in the form of the version it names.
                        let mut shape = v.clone();
                        if let Some(o) = shape.as_object_mut().filter(|o| o.get("entries").is_some_and(Value::is_array)) {
                            o.insert("entries".into(), Value::Array(Vec::new()));
                        }
                        let table = crate::device::Params::read(&shape, &At::Key(&At::Root, "params")).map_err(|f| f.in_file(id))?;
                        crate::device::check_params_shape(&table).map_err(|f| f.in_file(id))?;
                    }
                    obj.insert(k.clone(), v.clone());
                }
            }
        } else {
            obj.insert(k.clone(), v.clone());
        }
    }
    if !shared.is_empty() {
        shared.extend(own_shared);
        obj.insert(shared_key.into(), Value::Array(shared));
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

/// The sources of one profile of a resolution of 0.7, each marked with the profile.
pub(crate) struct ProfileCredit {
    /// The profile file they come from.
    file: String,
    /// Whether the file asked for writes the block that names the chain: its sources are then the
    /// result's own, else the credit of MIDI taken from a target.
    own: bool,
    sources: Vec<Source>,
}

/// The credit of each profile of `chains` (a block, then its chain, the nearest profile first), each
/// profile file once, in the order met; a profile of an own block and of an inherited one is own.
/// `chains` are of 0.7: a text is carried to it before.
fn credit_of_profiles(chains: &[(&str, Vec<ProfileLink>)], own_blocks: &[&str]) -> Result<Vec<ProfileCredit>, Fault> {
    let own: HashSet<&str> = chains.iter().filter(|(key, _)| own_blocks.contains(key)).flat_map(|(_, c)| c).map(|l| l.file.as_str()).collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for link in chains.iter().flat_map(|(_, c)| c).filter(|l| seen.insert(l.file.as_str())) {
        let list = link.doc.get("sources").and_then(Value::as_array).into_iter().flatten();
        let sources = list
            .map(|v| {
                let mut source = Source::read(v, &At::Root).map_err(|f| f.in_file(&link.file))?;
                source.extra.insert(PROFILE_OF_SOURCE.into(), Value::String(link.name.clone()));
                Ok(source)
            })
            .collect::<Result<_, Fault>>()?;
        out.push(ProfileCredit { file: link.file.clone(), own: own.contains(link.file.as_str()), sources });
    }
    Ok(out)
}

/// The credit of an assembly once a module's MIDI file, whose resolution gives `step`, is applied
/// over the MIDI `earlier` credits: that credit is now the one of MIDI taken from a target, and a
/// profile file it holds is not credited again.
pub(crate) fn credit_over(earlier: &mut Vec<ProfileCredit>, step: Vec<ProfileCredit>) {
    earlier.iter_mut().for_each(|c| c.own = false);
    for credit in step {
        if !earlier.iter().any(|c| c.file == credit.file) {
            earlier.push(credit);
        }
    }
}

/// Appends the credit of the profiles to `device`, after its own: to `sources`, or to
/// `midi_of_sources` for a profile of a target's block. A source the device already credits, equal in
/// every key but `id`, is not added again; one whose `id` is taken is given without it.
pub(crate) fn credit_profiles(device: &mut Device, credit: Vec<ProfileCredit>) {
    let nameless = |s: &Source| Source { id: None, ..s.clone() };
    let held: Vec<&Source> = [&device.sources, &device.midi_of_sources].into_iter().flatten().flatten().collect();
    let mut ids: HashSet<String> = held.iter().filter_map(|s| s.id.clone()).collect();
    let held: Vec<Source> = held.into_iter().map(nameless).collect();
    for ProfileCredit { own, sources, .. } in credit {
        for mut source in sources {
            if held.contains(&nameless(&source)) {
                continue;
            }
            if source.id.as_ref().is_some_and(|id| !ids.insert(id.clone())) {
                source.id = None;
            }
            let list = if own { &mut device.sources } else { &mut device.midi_of_sources };
            list.get_or_insert_with(Vec::new).push(source);
        }
    }
}

/// A profile of the chain of a block: the name that links it, its file, its text and its version.
struct ProfileLink {
    name: String,
    file: String,
    doc: Value,
    version: Version,
}

/// The profiles `block` names, the nearest first: the one it names, then the one that profile's
/// block names, up to `PROFILE_DEPTH` links, never in a circle. The profile is the same maker's.
/// `from` is the file that writes the block.
fn profile_chain(maker: &str, key: &str, block: &Value, from: &str, files: &dyn Files, read: &mut Vec<String>) -> Result<Vec<ProfileLink>, Fault> {
    let mut chain: Vec<ProfileLink> = Vec::new();
    let (mut block, mut from) = (block.clone(), from.to_string());
    while let Some(pid) = block.get("profile").cloned() {
        let link = |kind: FaultKind, why: String| Fault { kind, file: Some(from.clone()), pointer: format!("/{key}/profile"), rule: format!("profile {pid}: {why}") };
        let name = pid.as_str().ok_or_else(|| link(FaultKind::BadValue, "not a profile id".into()))?.to_string();
        let file = profile_file(&name).ok_or_else(|| link(FaultKind::BadValue, "not a profile id (<maker> or <maker>/<name>)".into()))?;
        if file.split('/').next() != Some(maker) { return Err(link(FaultKind::BadValue, "another maker's".into())); }
        if chain.iter().any(|l| l.name == name) { return Err(link(FaultKind::LinkCycle, "the links come back to it (a circle)".into())); }
        if chain.len() >= PROFILE_DEPTH { return Err(link(FaultKind::LinkTooDeep, format!("more than {PROFILE_DEPTH} profile links"))); }
        let (doc, typed) = fetch(&file, files, Kind::Profile, read)?.ok_or_else(|| link(FaultKind::LinkMissing, format!("no file {file}")))?;
        block = doc.get(key).cloned().ok_or_else(|| link(FaultKind::LinkMissing, format!("the profile has no {key}")))?;
        from = file.clone();
        chain.push(ProfileLink { name, file, doc, version: typed.schema().unwrap_or(Version::V0_4) });
    }
    Ok(chain)
}

/// Rules 23.23 to 23.32 of the section Resolution of `spec/0.6.md`, on each block of the joined
/// device `doc`. What a maker's models share is written once, in a profile kept in the maker's
/// folder (`profile_file`): `"identity": {"profile": "roland", "family": [89, 1], "member": [0,
/// 0]}` takes the Roland identity request, reply pattern and firmware rule, the model giving its
/// bytes. A block naming a profile is that profile's block (itself completed from the profile it
/// names) with the block's own keys in place of the profile's; a key set to null drops the
/// profile's.
///
/// When the joined device or one of the profiles is of 0.7, every older text is first carried to
/// the most recent version among them, so that one form is merged (proposal 0004); the answer says
/// whether the device was carried, and gives the credit of the profiles of a result of 0.7
/// ([`credit_of_profiles`], `own_blocks` being those the file asked for writes).
fn complete_from_profiles(doc: &mut Value, id: &str, own_blocks: &[&str], files: &dyn Files, read: &mut Vec<String>) -> Result<(bool, Vec<ProfileCredit>), Fault> {
    let maker = maker_of(id).unwrap_or_default();
    let mut chains = Vec::new();
    for key in OWN_BLOCKS {
        if let Some(block) = doc.get(key) {
            chains.push((key, profile_chain(maker, key, block, id, files, read)?));
        }
    }
    let device_version = version_of(doc);
    let to = chains.iter().flat_map(|(_, c)| c).map(|l| l.version).fold(device_version, Version::max);
    let joins_0_7 = to >= Version::V0_7;
    let converted = joins_0_7 && device_version < to;
    if converted {
        *doc = upgrade_joined(doc, to, id)?;
    }
    for (key, chain) in &mut chains {
        let key = *key;
        if joins_0_7 {
            for l in chain.iter_mut().filter(|l| l.version < to) {
                l.doc = upgrade(&l.doc, Kind::Profile, to, &l.file)?;
            }
        }
        let Some(deepest) = chain.last() else { continue };
        let mut out = deepest.doc[key].clone();
        for n in (0..chain.len()).rev() {
            let (block, from) = match n.checked_sub(1) {
                Some(above) => (&chain[above].doc[key], chain[above].file.as_str()),
                None => (&doc[key], id),
            };
            let name = &chain[n].name;
            let obj = out.as_object_mut().ok_or_else(|| Fault {
                kind: FaultKind::BadValue,
                file: Some(from.to_string()),
                pointer: format!("/{key}/profile"),
                rule: format!("profile {name}: its {key} is not an object"),
            })?;
            for (k, v) in block.as_object().into_iter().flatten() {
                match v {
                    Value::Null => { obj.remove(k); }
                    v => { obj.insert(k.clone(), v.clone()); }
                }
            }
            obj.insert("profile".into(), Value::String(name.clone()));
        }
        doc[key] = out;
    }
    let credit = if joins_0_7 { credit_of_profiles(&chains, own_blocks)? } else { Vec::new() };
    Ok((converted, credit))
}

/// The joined device carried to `to`. Its links are followed, so `midi_of` is set aside while it
/// is converted. The converter reads a device alone, whose `capabilities` are those its fields
/// require: it is given that list, and the caller puts back what the driver declared.
fn upgrade_joined(doc: &Value, to: Version, id: &str) -> Result<Value, Fault> {
    let mut alone = doc.clone();
    let link = alone.as_object_mut().and_then(|o| {
        o.remove("capabilities");
        o.remove("midi_of")
    });
    let required = crate::device::read_device(&alone, &At::Root).map_err(|f| f.in_file(id))?.derived_capabilities();
    if crate::is_05_form(Some(version_of(&alone))) && !required.is_empty() {
        alone["capabilities"] = required.iter().map(|c| Value::from(c.word())).collect();
    }
    let mut out = upgrade(&alone, Kind::Device, to, id)?;
    if let (Some(o), Some(link)) = (out.as_object_mut(), link) {
        o.insert("midi_of".into(), link);
    }
    Ok(out)
}
