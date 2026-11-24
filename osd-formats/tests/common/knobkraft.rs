//! What the KnobKraft tests share: the functions of the interface of an adaptation (KnobKraft
//! Orm, "Adaptation Programming Guide", by Christof Ruch), the four the export writes, and the
//! devices the tests build. The text an export writes is compared, never parsed nor run.

#![allow(dead_code)]

use serde_json::{json, Value};

/// The functions the export writes, in the order it writes them.
pub const WRITTEN: [&str; 4] = ["name", "numberOfBanks", "numberOfPatchesPerBank", "createEditBufferRequest"];

/// The functions of the interface of an adaptation the export names lost.
pub const LEFT_TO_HAND: [&str; 20] = [
    "createDeviceDetectMessage",
    "needsChannelSpecificDetection",
    "channelIfValidDeviceResponse",
    "deviceDetectWaitMilliseconds",
    "isEditBufferDump",
    "convertToEditBuffer",
    "nameFromDump",
    "renamePatch",
    "isDefaultName",
    "calculateFingerprint",
    "createProgramDumpRequest",
    "isSingleProgramDump",
    "convertToProgramDump",
    "numberFromDump",
    "createBankDumpRequest",
    "isPartOfBankDump",
    "isBankDumpFinished",
    "extractPatchesFromBank",
    "convertPatchesToBankDump",
    "friendlyBankName",
];

/// A device with sound transfers, two banks of 64 programs and a request of fixed bytes.
pub fn base() -> Value {
    json!({
        "role_vocabulary": "1",
        "programs": {
            "banks": [{"name": "A", "size": 64}, {"name": "B", "size": 64}],
            "dump": {"kind": "request", "request": [240, 66, 48, 0, 1, 81, 16, 247]}
        },
        "sound_transfers": [{
            "id": "single", "sound_id": "program", "carries": "sound",
            "match_frame": [240, 66, "any", 0, 1, 81, 64, "any", "any", "any", "any", "any", "any", "any", "any", "any", 247],
            "elements": {"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}}
        }]
    })
}

/// `base` with the blocks of `extra` laid over it (a `null` removes a block, one `device_with`
/// gives by default included).
pub fn device(extra: Value) -> osd::Resolved {
    let mut blocks = base();
    for (k, v) in extra.as_object().expect("an object of blocks") {
        blocks[k] = v.clone();
    }
    super::midnam::device_with(blocks)
}

/// What an export wrote: the text and the losses.
pub fn exported(d: &osd::Resolved) -> (String, Vec<osd_formats::Loss>) {
    let t = osd_formats::export(d, osd_formats::Format::Knobkraft).unwrap_or_else(|f| panic!("{f}"));
    (t.value, t.lost)
}

/// The places of the losses.
pub fn places(lost: &[osd_formats::Loss]) -> Vec<&str> {
    lost.iter().map(|l| l.place.as_str()).collect()
}
