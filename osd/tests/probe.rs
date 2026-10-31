//! The probe of the reader against the schemas (OSD-b91.2.8): documents of
//! the examples, the conformance cases and (with `SYNTHHUB_DEVICES`) the
//! catalogue, mutated deterministically; `osd::read_as` and the published
//! schema of the document's version must accept or refuse each alike. One
//! test per version and kind, so a disagreement only the schema can fix is
//! quarantined for that pair alone.

mod common;

use common::probe::{bounds, probe, version_of, Doc, Judge, Outcome, GENRES};
use common::{catalogue, Schemas};
use osd::{Kind, Version};
use serde_json::Value;
use std::path::Path;

const ALL_KINDS: [Kind; 5] = [Kind::Device, Kind::Profile, Kind::Sound, Kind::Fingerprint, Kind::Firmware];

fn word(kind: Kind) -> &'static str {
    match kind {
        Kind::Device => "device",
        Kind::Profile => "profile",
        Kind::Sound => "sound",
        Kind::Fingerprint => "fingerprint",
        Kind::Firmware => "firmware",
    }
}

fn kind_of(w: &str) -> Kind {
    ALL_KINDS.into_iter().find(|k| word(*k) == w).unwrap_or_else(|| panic!("{w} is not a kind"))
}

fn json_of(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn doc(name: String, kind: Kind, value: Value, bounded: bool) -> Doc {
    Doc { name, version: version_of(&value), kind, value, bounded }
}

fn sorted_dirs(root: &Path) -> Vec<std::path::PathBuf> {
    let Ok(rd) = std::fs::read_dir(root) else { return Vec::new() };
    let mut v: Vec<_> = rd.map(|e| e.unwrap().path()).collect();
    v.sort();
    v
}

const EXAMPLES: [(&str, Kind); 10] = [
    ("examples/devices/test_synth.json", Kind::Device),
    ("examples/warm-pad.sound.json", Kind::Sound),
    ("examples/minilogue.fingerprint.json", Kind::Fingerprint),
    ("examples/unknown-device.fingerprint.json", Kind::Fingerprint),
    ("examples/tribe-0.3-beta.firmware.json", Kind::Firmware),
    ("examples/0.5/synth.device.json", Kind::Device),
    ("examples/0.5/acme.profile.json", Kind::Profile),
    ("examples/0.5/warm-pad.sound.json", Kind::Sound),
    ("examples/0.5/minilogue.fingerprint.json", Kind::Fingerprint),
    ("examples/0.5/tribe-0.3-beta.firmware.json", Kind::Firmware),
];

fn examples() -> Vec<Doc> {
    EXAMPLES.iter().map(|(p, k)| doc(p.to_string(), *k, json_of(&common::repo().join(p)), false)).collect()
}

/// The descriptors of the conformance cases: what reading cases read, what
/// conversion cases convert, the files of resolution cases.
fn conformances() -> Vec<Doc> {
    let root = common::repo().join("conformance");
    let mut out = Vec::new();
    for version in sorted_dirs(&root) {
        for case in sorted_dirs(&version.join("read")).into_iter().chain(sorted_dirs(&version.join("convert"))) {
            let spec = json_of(&case.join("case.json"));
            let w = spec.get("read").or(spec.get("convert")).and_then(Value::as_str).unwrap();
            let name = case.strip_prefix(common::repo()).unwrap().display().to_string();
            out.push(doc(name, kind_of(w), json_of(&case.join("descriptor.json")), false));
        }
        for case in sorted_dirs(&version.join("resolve")) {
            for maker in sorted_dirs(&case.join("files")) {
                for f in sorted_dirs(&maker) {
                    let kind = if f.file_name().unwrap().to_str().unwrap().starts_with("_profile") {
                        Kind::Profile
                    } else {
                        Kind::Device
                    };
                    let name = f.strip_prefix(common::repo()).unwrap().display().to_string();
                    out.push(doc(name, kind, json_of(&f), false));
                }
            }
        }
    }
    out
}

fn catalogue_docs() -> Vec<Doc> {
    let Some(dir) = catalogue() else { return Vec::new() };
    let mut out = Vec::new();
    for (kind, ids) in [(Kind::Device, dir.drivers()), (Kind::Profile, dir.profiles())] {
        for id in ids {
            out.push(doc(id.clone(), kind, serde_json::from_str(&dir.text(&id)).unwrap(), true));
        }
    }
    out
}

fn schema_file(version: Version, kind: Kind) -> Value {
    json_of(&common::repo().join("schemas").join(version.name()).join(format!("{}.schema.json", word(kind))))
}

/// Probes `docs` of one version and kind with the real reader and the
/// published schema of that version.
fn run(version: Version, kind: Kind, docs: Vec<Doc>) -> Outcome {
    let docs: Vec<Doc> = docs.into_iter().filter(|d| d.version == version && d.kind == kind).collect();
    let schemas = Schemas::load(version.name());
    let reader = |v: Version, k: Kind, d: &Value| osd::read_as(&d.to_string(), k, v).is_ok();
    let oracle = |_: Version, k: Kind, d: &Value| schemas.check(k, d).is_none();
    probe(&docs, &|v, k| bounds(&schema_file(v, k)), &reader, &oracle)
}

fn report_counts(version: Version, kind: Kind, o: &Outcome) {
    println!(
        "probe {} {}: {} mutations generated, {} played ({:?}); {} documents not valid on both sides",
        version.name(),
        word(kind),
        o.generated,
        o.played,
        o.played_by_genre,
        o.not_valid.len()
    );
}

/// Every source for a version and kind; the pair needs at least one document.
fn probe_pair(version: Version, kind: Kind) {
    let mut docs = examples();
    docs.extend(conformances());
    docs.extend(catalogue_docs());
    let o = run(version, kind, docs);
    report_counts(version, kind, &o);
    assert!(o.played > 0, "no document of version {} kind {} was probed", version.name(), word(kind));
    common::report("disagreements between the reader and the schema", o.disagreements);
}

macro_rules! pair {
    ($name:ident, $v:expr, $k:expr) => {
        #[test]
        fn $name() {
            probe_pair($v, $k);
        }
    };
}

pair!(criterion_1_probe_0_4_device, Version::V0_4, Kind::Device);
pair!(criterion_1_probe_0_4_profile, Version::V0_4, Kind::Profile);
pair!(criterion_1_probe_0_4_sound, Version::V0_4, Kind::Sound);
pair!(criterion_1_probe_0_4_fingerprint, Version::V0_4, Kind::Fingerprint);
pair!(criterion_1_probe_0_4_firmware, Version::V0_4, Kind::Firmware);
// Quarantine: the reader refuses a repeated `lua` in `capabilities` (osd/docs/INTERFACE.md,
// `derived_capabilities`), the published 0.5 schema has no `uniqueItems` there; only the schema
// can be corrected (0.5 is not published, the fix goes into 0.5 itself). The format ticket is
// opened under OSD-b91.2 by the supervisor, blocking OSD-b91.2.9; its id replaces the one below.
#[ignore = "OSD-b91.2 format ticket (uniqueItems of /capabilities in schemas/0.5/device.schema.json), blocks OSD-b91.2.9"]
#[test]
fn criterion_10_probe_0_5_device_quarantined_for_capabilities_uniqueness() {
    probe_pair(Version::V0_5, Kind::Device);
}
pair!(criterion_1_probe_0_5_profile, Version::V0_5, Kind::Profile);
pair!(criterion_1_probe_0_5_sound, Version::V0_5, Kind::Sound);
pair!(criterion_1_probe_0_5_fingerprint, Version::V0_5, Kind::Fingerprint);
pair!(criterion_1_probe_0_5_firmware, Version::V0_5, Kind::Firmware);

#[test]
fn criterion_6_every_genre_of_mutation_yields_a_document_on_the_examples() {
    let mut total = Outcome::default();
    for version in osd::VERSIONS {
        for kind in ALL_KINDS {
            total.merge(run(*version, kind, examples()));
        }
    }
    for g in GENRES {
        assert!(total.played_by_genre.get(g).copied().unwrap_or(0) > 0, "no mutation of genre {g} on the examples");
    }
}

fn synth_example() -> Vec<Doc> {
    examples().into_iter().filter(|d| d.name == "examples/0.5/synth.device.json").collect()
}

fn probe_synth(reader: &dyn Judge, oracle: &dyn Judge) -> Outcome {
    probe(&synth_example(), &|v, k| bounds(&schema_file(v, k)), reader, oracle)
}

fn real_reader(v: Version, k: Kind, d: &Value) -> bool {
    osd::read_as(&d.to_string(), k, v).is_ok()
}

#[test]
fn criterion_6_selfcheck_a_reader_that_flips_an_unknown_root_key_is_named() {
    let schemas = Schemas::load("0.5");
    let oracle = |_: Version, k: Kind, d: &Value| schemas.check(k, d).is_none();
    let flipped = |v: Version, k: Kind, d: &Value| {
        let r = real_reader(v, k, d);
        if d.get("~unknown").is_some() { !r } else { r }
    };
    let o = probe_synth(&flipped, &oracle);
    assert_eq!(o.disagreements.len(), 1, "{:?}", o.disagreements);
    let line = &o.disagreements[0];
    for part in ["0.5", "Device", "add-unknown-key", "\"\"", "reader accepts", "schema refuses"] {
        assert!(line.contains(part), "{part} missing from {line}");
    }
}

#[test]
fn criterion_6_selfcheck_an_oracle_that_flips_a_removed_cc_is_named() {
    let schemas = Schemas::load("0.5");
    let flipped = |_: Version, k: Kind, d: &Value| {
        let s = schemas.check(k, d).is_none();
        if d.pointer("/cc_params/entries/0/cc").is_none() && d.pointer("/cc_params/entries/0").is_some() { !s } else { s }
    };
    let o = probe_synth(&real_reader, &flipped);
    assert!(
        o.disagreements.iter().any(|l| l.contains("remove-key") && l.contains("/cc_params/entries/0/cc")),
        "{:?}",
        o.disagreements
    );
}

#[test]
fn criterion_6_selfcheck_agreeing_sides_yield_no_disagreement_on_a_neutral_pair() {
    let always = |_: Version, _: Kind, _: &Value| true;
    let o = probe_synth(&always, &always);
    assert!(o.disagreements.is_empty());
    assert!(o.played > 0);
}

#[test]
fn criterion_4_mutations_follow_the_document_and_the_schema_bounds() {
    // Mutations the ticket names on examples/0.5/synth.device.json.
    let seen = std::cell::RefCell::new(Vec::<Value>::new());
    let record = |_: Version, _: Kind, d: &Value| {
        seen.borrow_mut().push(d.clone());
        true
    };
    probe_synth(&record, &record);
    let seen = seen.into_inner();
    let has = |f: &dyn Fn(&Value) -> bool| seen.iter().any(f);
    assert!(has(&|d| d.pointer("/cc_params/entries/0/cc").is_none() && d.pointer("/cc_params/entries/0").is_some()));
    assert!(has(&|d| d.pointer("/device/~unknown").is_some()));
    assert!(has(&|d| d.pointer("/midi_out/default_channel") == Some(&Value::String("0".into()))));
    assert!(has(&|d| d.pointer("/cc_params/entries/0/cc") == Some(&Value::from(128))));
    assert!(has(&|d| d.pointer("/midi_in/0") == Some(&Value::String("~unlisted".into()))));
    assert!(has(&|d| d.pointer("/cc_params/entries/0/cc") == Some(&Value::from(0.5))));
    assert!(has(&|d| d.pointer("/$schema").is_some_and(|s| s != "https://w3id.org/osd/0.5/device.schema.json")));
}

#[test]
fn criterion_5_a_schema_mutated_to_another_version_is_refused_by_both() {
    let mut other = synth_example();
    other[0].value["$schema"] = Value::String("https://synthhub.roomi-fields.com/osd/0.4/device.schema.json".into());
    let schemas = Schemas::load("0.5");
    let oracle = |_: Version, k: Kind, d: &Value| schemas.check(k, d).is_none();
    assert!(!real_reader(Version::V0_5, Kind::Device, &other[0].value));
    assert!(!oracle(Version::V0_5, Kind::Device, &other[0].value));
}
