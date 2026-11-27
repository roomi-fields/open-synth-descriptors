//! The SyntHub catalogue read by `osd`: every driver and maker profile
//! strictly typed, written back unchanged, resolved, converted forward, and
//! valid against the published schemas of the version it declares (0.4 when it
//! declares none). The catalogue is the folder `SYNTHHUB_DEVICES` names;
//! without it these tests say they are skipped. The checks they play
//! (`common::checks`, `common::trial`) are proved below on small catalogues.

use crate::common::checks::{
    at_mixed_control, conversions_to_0_5, converted_catalogue, documentary, judge_converted, judge_written,
    load_schemas, models, published_conversion_fault, published_conversions, read_strictly, resolutions,
    written_back, Conversions, Converted, Documentary,
};
use crate::common::trial::trial;
use crate::common::{catalogue_of, report, Folder};
use osd::{Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

/// The catalogue the value `named` of `SYNTHHUB_DEVICES` gives to the test `test`; without one,
/// `out` is told that this test is skipped.
fn catalogue_named(test: &str, named: Option<OsString>, out: &mut dyn Write) -> Option<Folder> {
    if named.is_none() {
        writeln!(out, "{test}: skipped, SYNTHHUB_DEVICES is not set").unwrap();
    }
    named.map(|dir| Folder(PathBuf::from(dir)))
}

/// The name of the running test: its harness names its thread after it.
fn running_test() -> String {
    std::thread::current().name().expect("a test runs on a named thread").to_string()
}

/// The catalogue of the running test. The skip line is written to stderr itself, so it shows on a
/// green run too.
fn catalogue() -> Option<Folder> {
    catalogue_named(&running_test(), std::env::var_os("SYNTHHUB_DEVICES"), &mut std::io::stderr())
}

#[test]
fn the_catalogue_converted_to_0_7_is_refused_nowhere_0_6_accepts_it_and_loses_nothing() {
    let Some(dir) = catalogue() else { return };
    let found = trial(&dir);
    // Written to stderr itself, so the counts show on a green run too.
    writeln!(std::io::stderr(), "{found}").unwrap();
    assert!(found.files > 0, "no descriptor in the catalogue");
    assert!(found.failures.is_empty(), "the trial of 0.7:\n{found}");
}

#[test]
fn every_driver_and_profile_reads_strictly() {
    let Some(dir) = catalogue() else { return };
    let (drivers, profiles) = (dir.drivers(), dir.profiles());
    assert!(drivers.len() > 1000 && !profiles.is_empty(), "{} drivers, {} profiles", drivers.len(), profiles.len());
    report("files not read", read_strictly(&dir));
}

#[test]
fn documentary_fields_are_in_the_types() {
    let Some(dir) = catalogue() else { return };
    let found = documentary(&dir);
    let Documentary { sources, coverage, sections, labels, notes } = found;
    assert!(sources > 1000 && coverage > 1000 && sections > 1000 && notes > 1000 && labels > 0, "{found:?}");
}

#[test]
fn every_file_written_back_equals_the_original() {
    let Some(dir) = catalogue() else { return };
    report("files changed by a read and a write", written_back(&dir));
}

#[test]
fn every_driver_resolves_into_a_device_valid_against_the_schemas_of_its_version() {
    let Some(dir) = catalogue() else { return };
    report("drivers not resolved", resolutions(&dir).failures);
}

#[test]
fn every_file_model_validates_against_the_schemas_of_its_version() {
    let Some(dir) = catalogue() else { return };
    report("models not valid", models(&dir).failures);
}

/// The counts show on a green run too; any failure fails.
fn conversion_verdict(what: &str, found: Conversions) {
    let mut shown = std::io::stderr();
    writeln!(shown, "{found}").unwrap();
    report(what, found.failures);
}

#[test]
fn every_file_of_0_5_or_older_converts_to_0_5_but_for_the_expected_faults() {
    let Some(dir) = catalogue() else { return };
    conversion_verdict("files not converted into 0.5", conversions_to_0_5(&dir));
}

#[test]
fn every_file_of_the_published_version_or_older_converts_to_it_but_for_the_expected_faults() {
    let Some(dir) = catalogue() else { return };
    conversion_verdict("files not converted as expected", published_conversions(&dir));
}

// The checks above, proved without the SyntHub catalogue.

const D05: &str = "https://w3id.org/osd/0.5/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";
const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const P05: &str = "https://w3id.org/osd/0.5/profile.schema.json";
const P06: &str = "https://w3id.org/osd/0.6/profile.schema.json";
const P07: &str = "https://w3id.org/osd/0.7/profile.schema.json";

/// `doc` declaring `schema`, or no `$schema`.
fn declaring(schema: Option<&str>, mut doc: Value) -> Value {
    if let Some(s) = schema {
        doc["$schema"] = s.into();
    }
    doc
}

/// A driver of one plain control, in the version `schema` names (0.4 without one).
fn driver(schema: Option<&str>) -> Value {
    let control = if schema == Some(D07) {
        json!({"osc": "/volume", "ways": [{"ctrlType": "cc", "ctrlIndex": [7]}]})
    } else {
        json!({"osc": "/volume", "cc": 7})
    };
    declaring(schema, json!({"device": {"name": "S", "osc_prefix": "/s"}, "cc_params": {"entries": [control]}}))
}

fn profile(schema: Option<&str>) -> Value {
    declaring(schema, json!({"identity": {"reply_text": "acme"}}))
}

/// A driver of 0.4 or 0.5 with these controls.
fn controls(schema: Option<&str>, entries: Value) -> Value {
    declaring(schema, json!({"device": {"name": "S", "osc_prefix": "/s"}, "cc_params": {"entries": entries}}))
}

/// A 0.4 driver with a command whose frame holds the string `element`.
fn framed(element: &str) -> Value {
    json!({"device": {"name": "S", "osc_prefix": "/s"}, "commands": [{"osc": "/dump", "frame": [240, element, 247]}]})
}

/// A right catalogue that mixes files without `$schema` and files of 0.5, 0.6 and 0.7.
fn mixed() -> Vec<(&'static str, Value)> {
    vec![
        ("acme/old", driver(None)),
        ("acme/five", driver(Some(D05))),
        ("acme/six", driver(Some(D06))),
        ("acme/seven", driver(Some(D07))),
        ("acme/_profile", profile(None)),
        ("acme/_profile.five", profile(Some(P05))),
        ("acme/_profile.six", profile(Some(P06))),
        ("acme/_profile.seven", profile(Some(P07))),
    ]
}

fn versions(of: &[(&str, Version)]) -> BTreeMap<String, Version> {
    of.iter().map(|(id, v)| (id.to_string(), *v)).collect()
}

/// The files the failures name, in the order of the failures.
fn named(failures: &[String]) -> Vec<&str> {
    failures.iter().map(|f| f.split_once(": ").unwrap().0).collect()
}

#[test]
fn a_right_catalogue_of_every_version_reads_and_is_written_back_without_failure() {
    let (_tmp, dir) = catalogue_of(&mixed());
    assert_eq!(read_strictly(&dir), Vec::<String>::new());
    assert_eq!(written_back(&dir), Vec::<String>::new());
}

#[test]
fn each_model_is_judged_in_the_version_its_file_declares_0_4_when_it_declares_none() {
    let (_tmp, dir) = catalogue_of(&mixed());
    let found = models(&dir);
    assert_eq!(found.failures, Vec::<String>::new());
    use Version::{V0_4, V0_5, V0_6, V0_7};
    assert_eq!(
        found.versions,
        versions(&[
            ("acme/old", V0_4),
            ("acme/five", V0_5),
            ("acme/six", V0_6),
            ("acme/seven", V0_7),
            ("acme/_profile", V0_4),
            ("acme/_profile.five", V0_5),
            ("acme/_profile.six", V0_6),
            ("acme/_profile.seven", V0_7),
        ])
    );
}

#[test]
fn a_right_catalogue_of_every_version_resolves_without_failure() {
    let (_tmp, dir) = catalogue_of(&mixed());
    let found = resolutions(&dir);
    assert_eq!(found.failures, Vec::<String>::new());
    use Version::{V0_4, V0_5, V0_6, V0_7};
    assert_eq!(
        found.versions,
        versions(&[("acme/old", V0_4), ("acme/five", V0_5), ("acme/six", V0_6), ("acme/seven", V0_7)])
    );
}

#[test]
fn a_right_catalogue_of_every_version_converts_without_failure() {
    let (_tmp, dir) = catalogue_of(&mixed());
    for (found, converted, newer) in [(conversions_to_0_5(&dir), 4, 4), (published_conversions(&dir), 6, 2)] {
        assert_eq!(found.failures, Vec::<String>::new(), "{found}");
        assert_eq!((found.converted, found.expected, found.newer), (converted, 0, newer), "{found}");
    }
}

/// A device that holds `contexts`, a field from 0.7.
fn with_contexts(schema: Option<&str>) -> Value {
    declaring(
        schema,
        json!({
            "device": {"name": "S", "osc_prefix": "/s"},
            "contexts": {"part": {"values": [{"id": "track1"}, {"id": "track2"}]}}
        }),
    )
}

#[test]
fn a_written_document_is_refused_by_the_schemas_of_the_version_it_declares() {
    assert_eq!(judge_written(Kind::Device, &with_contexts(Some(D07))), Ok(Version::V0_7));
    for (schema, version) in [(Some(D06), "0.6"), (Some(D05), "0.5"), (None, "0.4")] {
        let why = judge_written(Kind::Device, &with_contexts(schema)).expect_err(version);
        assert!(why.contains(version) && why.contains("contexts"), "{why}");
    }
}

#[test]
fn a_written_document_is_judged_by_the_schema_of_its_kind() {
    assert_eq!(judge_written(Kind::Profile, &profile(None)), Ok(Version::V0_4));
    assert_eq!(judge_written(Kind::Profile, &profile(Some(P06))), Ok(Version::V0_6));
    assert!(judge_written(Kind::Device, &profile(None)).is_err(), "a profile is no device");
}

#[test]
fn a_schema_of_a_version_without_published_schemas_is_refused_not_brought_back_to_0_4() {
    // The content is a right driver of 0.4: only its `$schema` is wrong.
    assert_eq!(judge_written(Kind::Device, &driver(None)), Ok(Version::V0_4));
    for schema in [
        json!("https://w3id.org/osd/0.8/device.schema.json"),
        json!("https://example.com/device.schema.json"),
        // The schema of another kind identifies no schema of a device.
        json!(P06),
        json!(6),
        Value::Null,
    ] {
        let mut doc = driver(None);
        doc["$schema"] = schema.clone();
        let why = judge_written(Kind::Device, &doc).expect_err(&schema.to_string());
        assert!(why.contains("$schema"), "{why}");
    }
}

#[test]
fn the_model_and_resolution_checks_name_each_file_they_refuse_and_no_other() {
    let (_tmp, dir) = catalogue_of(&[
        ("acme/right", driver(Some(D06))),
        ("acme/early", with_contexts(Some(D06))),
        ("acme/unpublished", driver(Some("https://w3id.org/osd/0.8/device.schema.json"))),
        ("acme/_profile", json!({"no_such_field": 1})),
    ]);
    let found = models(&dir);
    assert_eq!(named(&found.failures), ["acme/early", "acme/unpublished", "acme/_profile"], "{:?}", found.failures);
    assert_eq!(found.versions, versions(&[("acme/right", Version::V0_6)]));
    let found = resolutions(&dir);
    assert_eq!(named(&found.failures), ["acme/early", "acme/unpublished"], "{:?}", found.failures);
    assert_eq!(named(&read_strictly(&dir)), ["acme/early", "acme/unpublished", "acme/_profile"]);
}

#[test]
fn a_driver_whose_midi_of_target_is_missing_fails_the_resolution_check_by_name() {
    let mut variant = json!({"device": {"name": "S", "osc_prefix": "/s"}});
    variant["midi_of"] = json!("acme/gone");
    let (_tmp, dir) = catalogue_of(&[("acme/variant", variant)]);
    assert_eq!(models(&dir).failures, Vec::<String>::new());
    assert_eq!(named(&resolutions(&dir).failures), ["acme/variant"]);
}

/// The version the resolution check judges `acme/variant` in, a driver of the version `own` that
/// shares the MIDI of a driver of the version `target`.
fn judged_through_midi_of(own: Option<&str>, target: Option<&str>) -> Version {
    let variant = declaring(own, json!({"device": {"name": "S", "osc_prefix": "/s"}, "midi_of": "acme/base"}));
    let (_tmp, dir) = catalogue_of(&[("acme/base", driver(target)), ("acme/variant", variant)]);
    let found = resolutions(&dir);
    assert_eq!(found.failures, Vec::<String>::new(), "{own:?} on {target:?}");
    found.versions["acme/variant"]
}

#[test]
fn a_driver_resolved_through_midi_of_is_judged_in_the_newer_version_of_the_file_and_its_target() {
    use Version::{V0_6, V0_7};
    for (own, target, judged) in [
        (None, Some(D06), V0_6),
        (Some(D06), None, V0_6),
        (Some(D05), Some(D07), V0_7),
        (Some(D07), Some(D05), V0_7),
    ] {
        assert_eq!(judged_through_midi_of(own, target), judged, "{own:?} on {target:?}");
    }
}

/// The version the resolution check judges `acme/synth` in, a driver of the version `own`
/// completed by a maker profile of the version `of_profile`.
fn judged_through_a_profile(own: Option<&str>, of_profile: Option<&str>) -> Version {
    let synth = declaring(own, json!({"device": {"name": "S", "osc_prefix": "/s"}, "identity": {"profile": "acme"}}));
    let (_tmp, dir) = catalogue_of(&[("acme/_profile", profile(of_profile)), ("acme/synth", synth)]);
    let found = resolutions(&dir);
    assert_eq!(found.failures, Vec::<String>::new(), "{own:?} with {of_profile:?}");
    found.versions["acme/synth"]
}

#[test]
fn a_driver_completed_by_a_profile_older_than_0_7_is_judged_in_its_own_version() {
    use Version::{V0_4, V0_5, V0_6};
    for (own, of_profile, judged) in
        [(None, Some(P05), V0_4), (None, Some(P06), V0_4), (Some(D05), Some(P06), V0_5), (Some(D06), None, V0_6)]
    {
        assert_eq!(judged_through_a_profile(own, of_profile), judged, "{own:?} with {of_profile:?}");
    }
}

#[test]
fn a_driver_joined_to_a_text_of_0_7_by_a_profile_is_judged_in_0_7() {
    for (own, of_profile) in [(Some(D06), Some(P07)), (None, Some(P07)), (Some(D07), None), (Some(D07), Some(P06))] {
        assert_eq!(judged_through_a_profile(own, of_profile), Version::V0_7, "{own:?} with {of_profile:?}");
    }
}

#[test]
fn a_file_newer_than_the_target_fails_no_conversion_and_its_number_is_printed() {
    // Files the reader refuses, in a version the conversion does not reach.
    let (_tmp, dir) = catalogue_of(&[
        ("acme/seven", json!({"$schema": D07, "no_such_field": 1})),
        ("acme/_profile", json!({"$schema": P07, "no_such_field": 1})),
        ("acme/six", driver(Some(D06))),
    ]);
    let (to_0_5, published) = (conversions_to_0_5(&dir), published_conversions(&dir));
    assert_eq!((to_0_5.converted, to_0_5.newer, &to_0_5.failures), (0, 3, &Vec::new()), "{to_0_5}");
    assert_eq!((published.converted, published.newer, &published.failures), (1, 2, &Vec::new()), "{published}");
    assert!(to_0_5.to_string().contains("3 newer than 0.5"), "{to_0_5}");
    assert!(published.to_string().contains("2 newer than 0.6"), "{published}");
    // The model and the resolution still judge them.
    assert_eq!(named(&models(&dir).failures), ["acme/seven", "acme/_profile"]);
    assert_eq!(named(&resolutions(&dir).failures), ["acme/seven"]);
}

#[test]
fn a_file_no_newer_than_the_target_is_converted_and_judged() {
    let wrong = |schema| json!({"$schema": schema, "no_such_field": 1});
    let (_tmp, dir) = catalogue_of(&[("acme/five", wrong(D05)), ("acme/six", wrong(D06))]);
    assert_eq!(named(&conversions_to_0_5(&dir).failures), ["acme/five"]);
    assert_eq!(named(&published_conversions(&dir).failures), ["acme/five", "acme/six"]);
}

#[test]
fn a_file_of_an_unpublished_version_fails_the_conversions() {
    let (_tmp, dir) = catalogue_of(&[("acme/unpublished", driver(Some("https://w3id.org/osd/0.8/device.schema.json")))]);
    for found in [conversions_to_0_5(&dir), published_conversions(&dir)] {
        assert_eq!(named(&found.failures), ["acme/unpublished"], "{found}");
        assert_eq!(found.newer, 0, "{found}");
    }
}

#[test]
fn a_catalogue_without_an_expected_fault_or_a_file_to_convert_passes_the_conversions() {
    let newer = [("acme/seven", driver(Some(D07))), ("acme/_profile", profile(Some(P07)))];
    for files in [&[][..], &newer[..], &[("acme/old", driver(None))]] {
        let (_tmp, dir) = catalogue_of(files);
        for found in [conversions_to_0_5(&dir), published_conversions(&dir)] {
            assert_eq!(found.failures, Vec::<String>::new(), "{found}");
            assert_eq!(found.expected, 0, "{found}");
        }
    }
}

/// Two kinds of message on one control.
fn mixed_entries() -> Value {
    json!([{"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 93, "nrpn_msb": 1, "nrpn_lsb": 102}])
}

#[test]
fn the_expected_faults_are_admitted_by_the_conversions_that_refuse_them() {
    let (_tmp, dir) = catalogue_of(&[
        ("acme/checksum", framed("{checksum}")),
        ("acme/function", framed("{function}")),
        ("acme/mixed", controls(None, mixed_entries())),
        ("acme/mixed-five", controls(Some(D05), mixed_entries())),
    ]);
    // 0.5 holds a mixed control; 0.6 refuses it.
    let (to_0_5, published) = (conversions_to_0_5(&dir), published_conversions(&dir));
    assert_eq!(to_0_5.failures, Vec::<String>::new(), "{to_0_5}");
    assert_eq!((to_0_5.converted, to_0_5.expected), (2, 2), "{to_0_5}");
    assert_eq!(published.failures, Vec::<String>::new(), "{published}");
    assert_eq!((published.converted, published.expected), (0, 4), "{published}");
}

#[test]
fn any_other_refusal_of_a_conversion_fails_it() {
    // A `midi_in` template other than the canonical one: no structure, and no expected fault.
    let mut template = driver(None);
    template["midi_in"] = json!({"cc": "/other {num}"});
    // A file that holds an expected fault and is refused for another one.
    let mut beside = framed("{checksum}");
    beside["no_such_field"] = json!(1);
    let (_tmp, dir) = catalogue_of(&[("acme/beside", beside), ("acme/template", template)]);
    for found in [conversions_to_0_5(&dir), published_conversions(&dir)] {
        assert_eq!(named(&found.failures), ["acme/beside", "acme/template"], "{found}: {:?}", found.failures);
        assert_eq!((found.converted, found.expected), (0, 0), "{found}");
    }
}

#[test]
fn a_converted_file_that_holds_an_expected_fault_fails_the_conversion() {
    // A note keeps the text: the file converts, and still holds it.
    let mut noted = driver(None);
    noted["_notes"] = json!("{checksum}");
    let (_tmp, dir) = catalogue_of(&[("acme/noted", noted)]);
    for found in [conversions_to_0_5(&dir), published_conversions(&dir)] {
        assert_eq!(named(&found.failures), ["acme/noted"], "{found}");
        assert!(found.failures[0].contains("expected fault"), "{:?}", found.failures);
    }
}

#[test]
fn the_catalogue_conversion_admits_the_refusal_of_a_mixed_control_at_that_control() {
    for schema in [None, Some(D05)] {
        let text = controls(schema, mixed_entries()).to_string();
        assert_eq!(published_conversion_fault(&text, Kind::Device), None, "{schema:?}");
    }
    let plain = controls(None, json!([{"osc": "/a", "cc": 1}])).to_string();
    assert_eq!(published_conversion_fault(&plain, Kind::Device), None);
}

#[test]
fn the_catalogue_conversion_admits_that_refusal_at_no_other_control() {
    // Two controls claim one message: 0.6 refuses the later one, which mixes nothing.
    let twice = controls(Some(D05), json!([{"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 1}])).to_string();
    assert!(published_conversion_fault(&twice, Kind::Device).is_some());
    // The same beside a mixed control that comes after the refused one.
    let beside = json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 1}, {"osc": "/c", "cc": 2, "rpn_msb": 0, "rpn_lsb": 1}
    ]}});
    assert!(!at_mixed_control(&beside, "/cc_params/entries/1"));
    assert!(at_mixed_control(&beside, "/cc_params/entries/2"));
    assert!(!at_mixed_control(&beside, "/cc_params/entries/2/cc"));
}

#[test]
fn the_documentary_fields_of_the_drivers_are_counted() {
    let mut full = controls(None, json!([{"osc": "/a", "cc": 1, "section": "Filter"}, {"osc": "/b", "cc": 2}]));
    full["_sources"] = json!([{"type": "vendor-doc"}, {"type": "community"}]);
    full["_note"] = json!("n");
    let (_tmp, dir) = catalogue_of(&[("acme/full", full), ("acme/bare", driver(None))]);
    let found = documentary(&dir);
    assert_eq!((found.sources, found.sections, found.coverage, found.labels), (2, 1, 0, 0), "{found:?}");
    assert!(found.notes >= 1, "{found:?}");
}

#[test]
fn every_file_the_conversion_accepts_is_given_converted_and_judged_without_failure() {
    let (_tmp, dir) = catalogue_of(&mixed());
    let converted = converted_catalogue(&dir);
    use Kind::{Device, Profile};
    // The files of 0.7 are newer than the published version: the conversion refuses them.
    let given: Vec<(&str, Kind)> = converted.iter().map(|c| (c.id.as_str(), c.kind)).collect();
    assert_eq!(
        given,
        [
            ("acme/five", Device),
            ("acme/old", Device),
            ("acme/six", Device),
            ("acme/_profile", Profile),
            ("acme/_profile.five", Profile),
            ("acme/_profile.six", Profile),
        ]
    );
    for c in &converted {
        let text = osd::convert(&dir.text(&c.id), c.kind).unwrap();
        assert_eq!(c.doc, serde_json::from_str::<Value>(&text).unwrap(), "{}", c.id);
    }
    load_schemas();
    assert_eq!(judge_converted(&converted), Vec::<String>::new());
}

#[test]
fn a_file_the_conversion_refuses_is_not_given_converted() {
    let (_tmp, dir) = catalogue_of(&[
        ("acme/checksum", framed("{checksum}")),
        ("acme/mixed", controls(Some(D05), mixed_entries())),
        ("acme/right", driver(None)),
        ("acme/unknown", json!({"$schema": D06, "no_such_field": 1})),
        ("acme/unpublished", driver(Some("https://w3id.org/osd/0.8/device.schema.json"))),
        ("acme/_profile", json!({"no_such_field": 1})),
    ]);
    let given: Vec<String> = converted_catalogue(&dir).into_iter().map(|c| c.id).collect();
    assert_eq!(given, ["acme/right"]);
}

/// A converted document written by hand.
fn document(id: &str, kind: Kind, doc: Value) -> Converted {
    Converted { id: id.to_string(), kind, doc }
}

#[test]
fn a_right_document_of_the_published_version_passes_the_judgement_of_the_converted_documents() {
    let documents = [
        document("acme/synth", Kind::Device, driver(Some(D06))),
        document("acme/_profile", Kind::Profile, profile(Some(P06))),
    ];
    assert_eq!(judge_converted(&documents), Vec::<String>::new());
    assert_eq!(judge_converted(&[]), Vec::<String>::new());
}

#[test]
fn a_document_its_published_schemas_refuse_fails_naming_its_file_and_that_version() {
    let documents = [
        document("acme/right", Kind::Device, driver(Some(D06))),
        document("acme/early", Kind::Device, with_contexts(Some(D06))),
        document("acme/_profile", Kind::Profile, declaring(Some(P06), json!({"no_such_field": 1}))),
    ];
    let failures = judge_converted(&documents);
    assert_eq!(named(&failures), ["acme/early", "acme/_profile"], "{failures:?}");
    assert!(failures[0].contains("0.6") && failures[0].contains("contexts"), "{}", failures[0]);
    assert!(failures[1].contains("0.6") && failures[1].contains("no_such_field"), "{}", failures[1]);
}

#[test]
fn a_document_valid_in_another_version_than_the_published_one_fails_naming_both_versions() {
    for (kind, doc, declared) in [
        (Kind::Device, driver(Some(D05)), "0.5"),
        (Kind::Device, driver(None), "0.4"),
        (Kind::Device, driver(Some(D07)), "0.7"),
        (Kind::Profile, profile(Some(P05)), "0.5"),
        (Kind::Profile, profile(None), "0.4"),
    ] {
        // Right in the version it declares: only that version is wrong.
        assert!(judge_written(kind, &doc).is_ok(), "{doc}");
        let failures = judge_converted(&[document("acme/other", kind, doc)]);
        assert_eq!(named(&failures), ["acme/other"], "{declared}: {failures:?}");
        assert!(failures[0].contains(declared) && failures[0].contains("0.6"), "{declared}: {failures:?}");
    }
}

#[test]
fn a_document_of_an_unpublished_schema_fails_the_judgement_of_the_converted_documents() {
    let doc = driver(Some("https://w3id.org/osd/0.8/device.schema.json"));
    let failures = judge_converted(&[document("acme/unpublished", Kind::Device, doc)]);
    assert_eq!(named(&failures), ["acme/unpublished"], "{failures:?}");
    assert!(failures[0].contains("$schema"), "{}", failures[0]);
}

#[test]
fn without_synthhub_devices_a_catalogue_test_is_told_skipped_by_name() {
    let mut out = Vec::new();
    assert!(catalogue_named("catalogue::some_check", None, &mut out).is_none());
    assert_eq!(String::from_utf8(out).unwrap(), "catalogue::some_check: skipped, SYNTHHUB_DEVICES is not set\n");
}

#[test]
fn with_synthhub_devices_the_folder_it_names_is_read_and_nothing_is_told() {
    let (tmp, _) = catalogue_of(&[("acme/synth", driver(None))]);
    let mut out = Vec::new();
    let named = Some(tmp.path().as_os_str().to_os_string());
    let dir = catalogue_named("catalogue::some_check", named, &mut out).expect("a catalogue");
    assert_eq!(dir.drivers(), ["acme/synth"]);
    assert!(out.is_empty(), "{}", String::from_utf8_lossy(&out));
}

#[test]
fn the_skip_line_names_the_running_test() {
    assert!(running_test().ends_with("catalogue::the_skip_line_names_the_running_test"), "{}", running_test());
}

#[test]
fn a_community_file_is_neither_a_driver_nor_a_profile() {
    let (_tmp, dir) = catalogue_of(&[
        ("acme/synth", driver(None)),
        ("acme/_profile", profile(None)),
        ("acme/_community", json!({"links": []})),
        ("acme/_community.discord", json!({"links": []})),
    ]);
    assert_eq!(dir.drivers(), ["acme/synth"]);
    assert_eq!(dir.profiles(), ["acme/_profile"]);
}
