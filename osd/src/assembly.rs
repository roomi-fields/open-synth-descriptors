//! The assembly descriptor of OSD 0.6 (`schemas/0.6/assembly.schema.json`): a
//! stock firmware and the modules applied to it, each by its catalogue id,
//! written by the builder of the firmware they make. Its MIDI is the stock
//! MIDI followed by each module's own routes, in the listed order.

use crate::json::{list, object, At, Opt, STRING};
use crate::resolve::{fetch, Files, Overlay, Resolved};
use crate::{resolve, Descriptor, Fault, FaultKind, Firmware, FirmwareKind, Kind, SchemaUrl, Version};
use serde_json::Value;

object! {
    /// The firmware a builder made: the stock firmware it started from and the modules it applied.
    Assembly {
        "$schema" schema: Option<Version> = Opt(SchemaUrl("assembly")),
        /// The catalogue id of the stock firmware's descriptor.
        "stock" stock: String = STRING,
        /// The catalogue ids of the modules' firmware descriptors, in the order applied.
        "modules" modules: Vec<String> = list(STRING),
    }
    check from_0_6
}

/// An assembly exists from 0.6: its `$schema` names that version.
fn from_0_6(a: &Assembly, at: &At) -> Result<(), Fault> {
    let key = At::Key(at, "$schema");
    match a.schema {
        Some(Version::V0_6) => Ok(()),
        None => Err(key.fault(FaultKind::Missing, "an assembly carries `$schema`")),
        Some(v) => Err(key.fault(FaultKind::BadValue, format!("an assembly is not a descriptor of {}: it exists from 0.6", v.name()))),
    }
}

/// The MIDI descriptor a firmware of the assembly names, `descriptor` or `same_as`.
fn midi_of(file: &str, fw: &Firmware, kind: FirmwareKind) -> Result<(Option<String>, Option<String>), Fault> {
    let got = fw.kind.unwrap_or(FirmwareKind::Firmware);
    if got != kind {
        let want = if kind == FirmwareKind::Mod { "mod" } else { "firmware" };
        return Err(At::Key(&At::Root, "kind").fault(FaultKind::BadValue, format!("an assembly names a {want} here")).in_file(file));
    }
    let midi = fw.midi.as_ref();
    Ok((midi.and_then(|m| m.descriptor.clone()), midi.and_then(|m| m.same_as.clone())))
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
pub fn resolve_assembly(id: &str, files: &dyn Files) -> Result<Resolved, Fault> {
    let mut read = Vec::new();
    let Some((_, Descriptor::Assembly(asm))) = fetch(id, files, Kind::Assembly, &mut read)? else {
        return Err(At::Root.fault(FaultKind::LinkMissing, format!("no file {id}")).in_file(id));
    };
    let stock = firmware(&asm.stock, files, &mut read)?;
    let bad = |file: &str, pointer: &str, why: &str| Fault { kind: FaultKind::BadValue, file: Some(file.to_string()), pointer: pointer.into(), rule: why.into() };
    let (stock_midi, _) = midi_of(&asm.stock, &stock, FirmwareKind::Firmware)?;
    let stock_midi = stock_midi.ok_or_else(|| bad(&asm.stock, "/midi/descriptor", "the stock firmware of an assembly names its MIDI descriptor"))?;
    let base = resolve(&stock_midi, files)?;
    let mut identity = serde_json::from_str::<Value>(&crate::write(&Descriptor::Device(Box::new(base.device.clone())))).unwrap_or_default();
    identity.as_object_mut().map(|o| o.remove("midi_of"));
    let mut acc = identity.clone();
    for module in &asm.modules {
        let fw = firmware(module, files, &mut read)?;
        let (descriptor, same_as) = midi_of(module, &fw, FirmwareKind::Mod)?;
        match (descriptor, same_as) {
            (None, Some(s)) if s == stock_midi => {}
            (None, Some(_)) => return Err(bad(module, "/midi/same_as", "a module is made for this stock firmware: its MIDI is the same as the stock's")),
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
            _ => return Err(bad(module, "/midi", "a module names its MIDI by `descriptor`, or `same_as` the stock's")),
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
