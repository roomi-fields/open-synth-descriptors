//! The assembly descriptor of OSD 0.6 (`schemas/0.6/assembly.schema.json`): a
//! stock firmware and the modules applied to it, each by its catalogue id,
//! written by the builder of the firmware they make. Its MIDI is the stock
//! MIDI followed by each module's own routes, in the listed order.

use crate::json::{list, obj, object, At, Opt, STRING};
use crate::resolve::{fetch, Files, Overlay, Resolved};
use crate::{compare_versions, resolve, Descriptor, Fault, FaultKind, Firmware, FirmwareKind, Kind, SchemaUrl, Sound, Version};
use serde_json::Value;
use std::cmp::Ordering;

object! {
    /// The firmware a builder made: the stock firmware it started from and the modules it applied.
    Assembly {
        "$schema" schema: Option<Version> = Opt(SchemaUrl("assembly")),
        /// The stock firmware, by its catalogue id and its exact version.
        "stock" stock: PinnedFirmware = obj(),
        /// The modules applied, each by its catalogue id and its exact version, in the order applied.
        "modules" modules: Vec<PinnedModule> = list(obj()),
    }
    check from_0_6
}

object! {
    /// The stock firmware of an assembly, pinned: its descriptor's catalogue id and its exact version.
    PinnedFirmware {
        /// The catalogue id of the stock firmware's descriptor (kind `firmware`).
        "firmware" firmware: String = STRING,
        /// The exact version of the stock firmware.
        "version" version: String = STRING,
    }
}

object! {
    /// A module of an assembly, pinned: its descriptor's catalogue id and its exact version.
    PinnedModule {
        /// The catalogue id of the module's descriptor (kind `module`).
        "module" module: String = STRING,
        /// The exact version of the module.
        "version" version: String = STRING,
    }
}

/// What a sound needs of a module, judged against an assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The assembly pins the module, at a version not lower than the minimum if one is written.
    Satisfied,
    /// The assembly lacks the module, or pins a version lower than the minimum.
    NotSatisfied,
    /// A version the comparison cannot read: neither can be told.
    Unknown,
}

/// The verdict on one need of a sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedJudgement {
    /// The module the need names.
    pub module: String,
    pub verdict: Verdict,
}

/// The needs of `sound` judged against `assembly`, one per need in the sound's order. Only the
/// assembly is read: the version pinned for the module, never a catalogue descriptor.
pub fn judge_needs(sound: &Sound, assembly: &Assembly) -> Vec<NeedJudgement> {
    sound
        .needs
        .iter()
        .flatten()
        .map(|need| {
            let verdict = match assembly.modules.iter().find(|m| m.module == need.module) {
                None => Verdict::NotSatisfied,
                Some(pinned) => match need.min_version.as_deref() {
                    None => Verdict::Satisfied,
                    Some(min) => match compare_versions(&pinned.version, min) {
                        Some(Ordering::Less) => Verdict::NotSatisfied,
                        Some(_) => Verdict::Satisfied,
                        None => Verdict::Unknown,
                    },
                },
            };
            NeedJudgement { module: need.module.clone(), verdict }
        })
        .collect()
}

/// An assembly exists from 0.6: its `$schema` names that version.
/// Read inside a sound's `made_on`, it is the assembly as written, without `$schema`.
fn from_0_6(a: &Assembly, at: &At) -> Result<(), Fault> {
    if !matches!(at, At::Root) {
        return Ok(());
    }
    let key = At::Key(at, "$schema");
    match a.schema {
        Some(Version::V0_6) => Ok(()),
        None => Err(key.fault(FaultKind::Missing, "an assembly carries `$schema`")),
        Some(v) => Err(key.fault(FaultKind::BadValue, format!("an assembly is not a descriptor of {}: it exists from 0.6", v.name()))),
    }
}

/// The MIDI descriptor a firmware of the assembly names for `device`, `descriptor` or `same_as`, in
/// the target of that device and the index of that target. `device` is the descriptor id of the device
/// resolved for; `None` takes the only target of the firmware.
fn midi_of(file: &str, fw: &Firmware, kind: FirmwareKind, device: Option<&str>) -> Result<(usize, Option<String>, Option<String>), Fault> {
    let got = fw.kind.unwrap_or(FirmwareKind::Firmware);
    if got != kind {
        let want = if kind == FirmwareKind::Module { "module" } else { "firmware" };
        return Err(At::Key(&At::Root, "kind").fault(FaultKind::BadValue, format!("an assembly names a {want} here")).in_file(file));
    }
    let targets = fw.targets.as_deref().unwrap_or_default();
    let refuse = |why: String| At::Key(&At::Root, "targets").fault(FaultKind::BadValue, why).in_file(file);
    let index = match device {
        None if targets.len() == 1 => 0,
        None => return Err(refuse("the firmware has several targets: the device to resolve for is needed".into())),
        Some(id) => {
            let mut found = targets.iter().enumerate().filter(|(_, t)| t.device.descriptor.as_deref() == Some(id)).map(|(i, _)| i);
            match (found.next(), found.next()) {
                (Some(i), None) => i,
                (None, _) => return Err(refuse(format!("no target of the firmware is for {id}"))),
                (Some(_), Some(_)) => return Err(refuse(format!("two targets of the firmware are for {id}"))),
            }
        }
    };
    let midi = targets[index].midi.as_ref();
    Ok((index, midi.and_then(|m| m.descriptor.clone()), midi.and_then(|m| m.same_as.clone())))
}

fn firmware(id: &str, files: &dyn Files, read: &mut Vec<String>) -> Result<Firmware, Fault> {
    match fetch(id, files, Kind::Firmware, read)? {
        Some((_, Descriptor::Firmware(f))) => Ok(*f),
        _ => Err(At::Root.fault(FaultKind::LinkMissing, format!("no file {id}")).in_file(id)),
    }
}

/// The MIDI of the assembly `id`: the stock firmware's MIDI descriptor resolved
/// (`resolve`), then for each module in order the routes its own MIDI file adds.
/// A module's MIDI file follows the stock MIDI (`midi_of`) and is read by the
/// rules of `midi_of` against the MIDI accumulated so far, so it adds only its own
/// routes, each replacing one at the same address; a module whose MIDI is
/// `same_as` the stock MIDI adds nothing. Two modules on one address are the
/// builder's business: neither is refused.
///
/// `device` is the descriptor id of the device the assembly is resolved for: the MIDI taken from the
/// stock firmware and from each module is that of their target for it. It may be `None` when the
/// stock firmware has one target; that target's device is then the one resolved for.
pub fn resolve_assembly(id: &str, device: Option<&str>, files: &dyn Files) -> Result<Resolved, Fault> {
    let mut read = Vec::new();
    let Some((_, Descriptor::Assembly(asm))) = fetch(id, files, Kind::Assembly, &mut read)? else {
        return Err(At::Root.fault(FaultKind::LinkMissing, format!("no file {id}")).in_file(id));
    };
    let stock_id = asm.stock.firmware.as_str();
    let stock = firmware(stock_id, files, &mut read)?;
    let bad = |file: &str, pointer: &str, why: &str| Fault { kind: FaultKind::BadValue, file: Some(file.to_string()), pointer: pointer.into(), rule: why.into() };
    let (at, stock_midi, _) = midi_of(stock_id, &stock, FirmwareKind::Firmware, device)?;
    let device = device.map(str::to_string).or_else(|| stock.targets.as_deref().and_then(|t| t[at].device.descriptor.clone()));
    let stock_midi = stock_midi.ok_or_else(|| bad(stock_id, &format!("/targets/{at}/midi/descriptor"), "the stock firmware of an assembly names its MIDI descriptor"))?;
    let base = resolve(&stock_midi, files)?;
    let mut identity = serde_json::from_str::<Value>(&crate::write(&Descriptor::Device(Box::new(base.device.clone())))).unwrap_or_default();
    identity.as_object_mut().map(|o| o.remove("midi_of"));
    let mut acc = identity.clone();
    for pinned in &asm.modules {
        let module = pinned.module.as_str();
        let fw = firmware(module, files, &mut read)?;
        let (at, descriptor, same_as) = midi_of(module, &fw, FirmwareKind::Module, device.as_deref())?;
        match (descriptor, same_as) {
            (None, Some(s)) if s == stock_midi => {}
            (None, Some(_)) => return Err(bad(module, &format!("/targets/{at}/midi/same_as"), "a module is made for this stock firmware: its MIDI is the same as the stock's")),
            (Some(d), None) => {
                // The module's MIDI file follows the stock MIDI, here the MIDI accumulated.
                let own = fetch(&d, files, Kind::Device, &mut read)?
                    .ok_or_else(|| At::Root.fault(FaultKind::LinkMissing, format!("no file {d}")).in_file(module))?.0;
                if own.get("midi_of").and_then(Value::as_str) != Some(stock_midi.as_str()) {
                    return Err(bad(&d, "/midi_of", "the MIDI of a module follows the MIDI of the stock firmware"));
                }
                let overlay = Overlay { base: files, id: &stock_midi, text: acc.to_string() };
                let r = resolve(&d, &overlay)?;
                acc = serde_json::from_str(&crate::write(&Descriptor::Device(Box::new(r.device)))).unwrap_or_default();
                acc.as_object_mut().map(|o| o.remove("midi_of"));
                // The module changes routes, not which device it is.
                if let Some(device) = identity.get("device") {
                    acc["device"] = device.clone();
                }
                for f in r.files {
                    if f != stock_midi && !read.contains(&f) {
                        read.push(f);
                    }
                }
            }
            _ => return Err(bad(module, &format!("/targets/{at}/midi"), "a module names its MIDI by `descriptor`, or `same_as` the stock's")),
        }
    }
    let Descriptor::Device(device) = crate::read(&acc.to_string(), Kind::Device)? else { unreachable!("a device is read") };
    let mut files_read = vec![id.to_string()];
    files_read.extend(read.into_iter().filter(|f| f != id));
    for f in base.files {
        if !files_read.contains(&f) {
            files_read.push(f);
        }
    }
    let capabilities = device.derived_capabilities();
    Ok(Resolved { inherited: capabilities.clone(), device: *device, files: files_read, capabilities })
}
