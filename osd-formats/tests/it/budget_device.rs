//! The device the time budgets translate (common/midnam.rs): 200 controls, each reached by a
//! message of its own, read strictly, and translated both ways without losing one.

use crate::common::midnam::device_of_200_controls;
use crate::common::{entries, source};
use osd_formats::{export, import, Format};
use std::collections::BTreeSet;

/// The message of a control: its kind and its numbers, as the fields give them.
fn message(e: &osd::CcEntry) -> String {
    format!("{:?}", (crate::common::messages_of(e), e.channel))
}

fn messages(d: &osd::Device) -> BTreeSet<String> {
    entries(d).iter().map(message).collect()
}

#[test]
fn the_budget_device_reads_strictly_with_200_controls_each_on_a_message_of_its_own() {
    let device = device_of_200_controls();
    assert_eq!(entries(&device.device).len(), 200);
    assert_eq!(messages(&device.device).len(), 200);
    assert!(entries(&device.device).iter().all(|e| e.channel.is_none()));
}

#[test]
fn the_budget_device_makes_the_midnam_round_trip_with_its_200_controls_on_the_same_messages() {
    let device = device_of_200_controls();
    let out = export(&device, Format::Midnam).expect("the device exports");
    let back = import(&out.value, Format::Midnam, &source()).expect("the document imports");
    assert_eq!(entries(&back.value).len(), 200);
    assert_eq!(messages(&back.value), messages(&device.device));
}

#[test]
fn the_budget_device_exports_to_oscquery_and_imports_back() {
    let device = device_of_200_controls();
    let out = export(&device, Format::OscQuery).expect("the device exports");
    import(&out.value, Format::OscQuery, &crate::common::oscquery::source()).expect("the namespace imports");
}
