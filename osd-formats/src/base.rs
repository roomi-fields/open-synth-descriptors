//! The base `import_into` joins a file to, said in 0.7 by the converter chain of `osd`.

use crate::{Fault, FaultKind};
use osd::{Descriptor, Device, Kind, Version};

/// `base` in 0.7. A base of an earlier version is written and converted by [`osd::convert_to`],
/// which judges the converted text by the reader; that text is read back to obtain the typed
/// [`Device`]. A base of 0.7 is read back too, and comes back as it is.
pub(crate) fn in_0_7(base: Device) -> Result<Device, Fault> {
    if base.schema >= Some(Version::V0_7) {
        let text = osd::write(&Descriptor::Device(Box::new(base.clone())));
        osd::read(&text, Kind::Device).map_err(|f| refused(f, "does not read in version 0.7"))?;
        return Ok(base);
    }
    let text = osd::write(&Descriptor::Device(Box::new(base)));
    let converted = osd::convert_to(&text, Kind::Device, Version::V0_7).map_err(not_convertible)?;
    match osd::read(&converted, Kind::Device).map_err(not_convertible)? {
        Descriptor::Device(d) => Ok(*d),
        _ => unreachable!("a text read as a device is a device"),
    }
}

fn not_convertible(f: osd::Fault) -> Fault {
    refused(f, "does not convert to version 0.7")
}

/// The refusal of `osd`, at the pointer it gives in the base given, saying what the base does
/// not do.
fn refused(f: osd::Fault, why: &str) -> Fault {
    Fault {
        kind: FaultKind::NotConvertible,
        place: f.pointer,
        rule: format!("the base descriptor {why}: {}", f.rule),
    }
}
