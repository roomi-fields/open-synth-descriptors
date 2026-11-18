//! The conformance cases of the repository (`conformance/<version>/`), each
//! run as the specification states it.

use crate::common;

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// The files of a case, by catalogue id.
fn files(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for maker in std::fs::read_dir(dir).unwrap() {
        let maker = maker.unwrap().path();
        for f in std::fs::read_dir(&maker).unwrap() {
            let f = f.unwrap().path();
            let id = format!("{}/{}", maker.file_name().unwrap().to_str().unwrap(), f.file_stem().unwrap().to_str().unwrap());
            out.insert(id, std::fs::read_to_string(&f).unwrap());
        }
    }
    out
}

/// What a resolution case asked for that the reader did not return, if anything.
fn run_resolve(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let id = case["resolve"].as_str().unwrap();
    let got = osd::resolve(id, &files(&dir.join("files")));
    match (got, case.get("result"), case.get("fault")) {
        (Ok(r), Some(want), None) => {
            let doc: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap();
            let inherited: Vec<&str> = r.inherited.iter().map(|c| c.word()).collect();
            let want_inherited: Vec<&str> = case.get("inherited").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
            if doc != *want {
                return Some(format!("resolved into {doc}"));
            }
            (inherited != want_inherited).then(|| format!("inherited {inherited:?}, the case states {want_inherited:?}"))
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "file": f.file, "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        (Ok(_), None, Some(want)) => Some(format!("resolved, but should be refused with {want}")),
        (Err(f), Some(_), None) => Some(format!("refused: {f}")),
        _ => Some("case.json has neither result nor fault, or both".into()),
    }
}

/// The kind a case reads, by the name of its schema.
fn kind(word: &str) -> osd::Kind {
    match word {
        "device" => osd::Kind::Device,
        "profile" => osd::Kind::Profile,
        "sound" => osd::Kind::Sound,
        "fingerprint" => osd::Kind::Fingerprint,
        "firmware" => osd::Kind::Firmware,
        _ => panic!("{word} is not a kind"),
    }
}

fn hex_bytes(text: &str) -> Vec<u8> {
    text.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
}

/// What a reading case states of the sound transfers of its descriptor, when it does:
/// `identity_support` (each transfer by its pointer, whether its identity is computable without
/// Lua) and `identities` (a transfer by its `id`, the index of the sound, a `message` in
/// hexadecimal, the `identity` expected). Absent, the descriptor has none and says none.
fn sound_transfer_claims(d: &osd::Descriptor, case: &Value) -> Option<String> {
    let osd::Descriptor::Device(device) = d else { return None };
    let support: Vec<Value> = osd::sound_identity_support(device)
        .into_iter()
        .map(|s| serde_json::json!({"pointer": s.pointer, "without_lua": s.without_lua}))
        .collect();
    let want_support = case.get("identity_support").and_then(Value::as_array).cloned().unwrap_or_default();
    if support != want_support {
        return Some(format!("identity_support {support:?}, the case states {want_support:?}"));
    }
    for want in case.get("identities").and_then(Value::as_array).into_iter().flatten() {
        let message = hex_bytes(want["message"].as_str().unwrap());
        let got = osd::sound_identity(device, want["transfer"].as_str().unwrap(), &message, want["index"].as_u64().unwrap() as usize);
        if got.as_deref().ok() != want["identity"].as_str() {
            return Some(format!("identity {got:?}, the case states {want}"));
        }
    }
    None
}

/// What a reading case asked for that the reader did not return, if anything.
fn run_read(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let got = osd::read(&text, kind(case["read"].as_str().unwrap()));
    match (got, case.get("result"), case.get("fault")) {
        (Ok(d), Some(want), None) => {
            let doc: Value = serde_json::from_str(&osd::write(&d)).unwrap();
            if doc != *want {
                return Some(format!("read into {doc}"));
            }
            // `roles`, when the case states it: each role's pointer, what it resolves to and its paramPath.
            let got: Vec<Value> = osd::roles(&d)
                .into_iter()
                .map(|r| {
                    let meaning = if r.meaning == osd::RoleMeaning::Known { "known" } else { "none" };
                    serde_json::json!({"pointer": r.pointer, "meaning": meaning, "paramPath": r.param_path})
                })
                .collect();
            let want_roles = case.get("roles").and_then(Value::as_array).cloned().unwrap_or_default();
            if got != want_roles {
                return Some(format!("roles {got:?}, the case states {want_roles:?}"));
            }
            sound_transfer_claims(&d, &case)
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        (Ok(_), None, Some(want)) => Some(format!("read, but should be refused with {want}")),
        (Err(f), Some(_), None) => Some(format!("refused: {f}")),
        _ => Some("case.json has neither result nor fault, or both".into()),
    }
}

/// The role cases of 0.6 (`conformance/0.6/read/<case>`): the descriptor's
/// shape, then what the case states. `(case, fault kind and pointer, or the
/// roles resolved, whether only the reader refuses it)`.
const ROLE_CASES: [(&str, &str, &str, bool); 16] = [
    ("role-known", "known", "", false),
    ("role-two-envelopes", "known known", "", false),
    ("role-composed-index", "known known", "", false),
    ("role-extension", "none", "", false),
    ("role-string-refused", "BadValue", "/cc_params/entries/0/role", false),
    ("role-unknown-key-refused", "UnknownField", "/cc_params/entries/0/role/colour", false),
    ("role-index-below-one-refused", "BadValue", "/cc_params/entries/0/role/parts/0/index", false),
    ("role-without-vocabulary-refused", "Missing", "/role_vocabulary", false),
    ("role-vocabulary-unknown-refused", "BadValue", "/role_vocabulary", true),
    ("role-setting-not-of-part-refused", "BadValue", "/cc_params/entries/0/role", true),
    ("role-unknown-term-refused", "BadValue", "/cc_params/entries/0/role/setting", true),
    ("role-term-of-wrong-type-refused", "BadValue", "/cc_params/entries/0/role/parts/0/part", true),
    ("role-part-not-under-refused", "BadValue", "/cc_params/entries/0/role/parts/1", true),
    ("role-first-part-not-at-top-refused", "BadValue", "/cc_params/entries/0/role/parts/0", true),
    ("role-flat-form-refused", "UnknownField", "/cc_params/entries/0/role/part", false),
    ("role-parts-empty-refused", "BadValue", "/cc_params/entries/0/role/parts", false),
];

/// The `paramPath` each result case states for its roles, in order.
const ROLE_PARAM_PATHS: [(&str, &[&str]); 4] = [
    ("role-known", &["/Filter/Cutoff"]),
    ("role-two-envelopes", &["/Filter/Envelope/AttackTime", "/Amplifier/Envelope/AttackTime"]),
    ("role-composed-index", &["/Oscillator/2/Envelope/1/AttackTime", "/Output/Pan"]),
    ("role-extension", &["/Filter/x-acme-ladder/Cutoff"]),
];

#[test]
fn the_role_cases_of_0_6_show_a_known_role_a_composed_one_an_extension_and_each_refusal() {
    let root = common::repo().join("conformance/0.6/read");
    let mut failures = Vec::new();
    for (name, outcome, pointer, reader_only) in ROLE_CASES {
        let dir = root.join(name);
        let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap_or_else(|e| panic!("{name}/{f}: {e}"));
        let case: Value = serde_json::from_str(&read("case.json")).unwrap();
        let doc: Value = serde_json::from_str(&read("descriptor.json")).unwrap();
        if case.get("read") != Some(&serde_json::json!("device")) {
            failures.push(format!("{name}: not a device case"));
        }
        if (case.get("schema_accepts") == Some(&serde_json::json!(true))) != reader_only {
            failures.push(format!("{name}: schema_accepts is not {reader_only}"));
        }
        if pointer.is_empty() {
            let want: Vec<&str> = outcome.split(' ').collect();
            let got: Vec<&str> = case["roles"].as_array().map_or(vec![], |r| r.iter().map(|x| x["meaning"].as_str().unwrap()).collect());
            if got != want || case.get("fault").is_some() {
                failures.push(format!("{name}: states {got:?}, expected {want:?}"));
            }
            // Each role states its paramPath.
            let paths: Vec<&str> = case["roles"].as_array().map_or(vec![], |r| r.iter().filter_map(|x| x["paramPath"].as_str()).collect());
            let want_paths = ROLE_PARAM_PATHS.iter().find(|(n, _)| *n == name).map(|(_, p)| p.to_vec());
            if Some(paths.clone()) != want_paths {
                failures.push(format!("{name}: states the paramPaths {paths:?}, expected {want_paths:?}"));
            }
        } else if case["fault"] != serde_json::json!({"kind": outcome, "pointer": pointer}) {
            failures.push(format!("{name}: states {}", case["fault"]));
        }
        // The descriptor shows what the case is named for.
        let role = &doc["cc_params"]["entries"][0]["role"];
        let parts = role["parts"].as_array().cloned().unwrap_or_default();
        let part = |n: usize| parts.get(n).map(|p| p["part"].as_str().unwrap_or(""));
        let shows = match name {
            "role-two-envelopes" => {
                let second = doc["cc_params"]["entries"][1]["role"]["parts"].as_array().cloned().unwrap_or_default();
                parts.len() == 2
                    && part(0) == Some("filter")
                    && part(1) == Some("envelope")
                    && second.len() == 2
                    && second[0]["part"] == "amplifier"
                    && second[1]["part"] == "envelope"
                    && role["setting"] == doc["cc_params"]["entries"][1]["role"]["setting"]
            }
            "role-composed-index" => parts.len() == 2 && parts.iter().all(|p| p["index"].is_u64()) && doc["params"]["entries"][0]["role"]["parts"].is_array(),
            "role-extension" => part(1).is_some_and(|p| p.starts_with("x-")) && parts.len() >= 2,
            "role-string-refused" => role.is_string(),
            "role-unknown-key-refused" => role.as_object().is_some_and(|o| o.keys().any(|k| !["parts", "setting"].contains(&k.as_str()))),
            "role-index-below-one-refused" => parts[0]["index"].as_i64().is_some_and(|i| i < 1),
            "role-without-vocabulary-refused" => role.is_object() && doc.get("role_vocabulary").is_none(),
            "role-vocabulary-unknown-refused" => doc["role_vocabulary"].is_string() && osd::vocabulary_state(doc["role_vocabulary"].as_str().unwrap()).is_none(),
            "role-term-of-wrong-type-refused" => part(0) == Some("cutoff"),
            "role-part-not-under-refused" => part(0) == Some("oscillator") && part(1) == Some("filter"),
            "role-first-part-not-at-top-refused" => part(0) == Some("envelope"),
            "role-flat-form-refused" => role.get("part").is_some() && role.get("parts").is_none(),
            "role-parts-empty-refused" => role["parts"] == serde_json::json!([]),
            "role-known" => parts.len() == 1 && doc["role_vocabulary"] == "1",
            _ => role.is_object() && doc["role_vocabulary"] == "1",
        };
        if !shows {
            failures.push(format!("{name}: the descriptor does not show its case"));
        }
    }
    common::report("role cases fail", failures);
}

/// Every case of a folder run by `run`, the failures reported together.
fn every_case(root: &Path, least: usize, run: fn(&Path) -> Option<String>) {
    let mut cases: Vec<_> = std::fs::read_dir(root).unwrap().map(|e| e.unwrap().path()).collect();
    cases.sort();
    assert!(cases.len() >= least, "{} cases", cases.len());
    let failures = cases.iter()
        .filter_map(|c| run(c).map(|why| format!("{}: {why}", c.file_name().unwrap().to_string_lossy())))
        .collect();
    common::report("cases failed", failures);
}

#[test]
fn every_reading_case_of_0_4_passes() {
    every_case(&common::repo().join("conformance/0.4/read"), 2, run_read);
}

/// The schema errors a reading case's descriptor has, against what its case
/// expects: none for a result, exactly one at the fault's pointer for a
/// fault.
fn schema_errors(dir: &Path, schemas: &common::Schemas) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("descriptor.json")).unwrap()).unwrap();
    let errors = schemas.errors(kind(case["read"].as_str().unwrap()), &doc);
    // A refusal the reader owes to a rule no schema states (a declared list
    // that differs from the derived one) has a descriptor the schema accepts.
    let schema_accepts = case.get("schema_accepts").and_then(Value::as_bool) == Some(true);
    // `schema_pointer`, when the case states it, is where the schema reports a rule it states as a
    // dependency between fields (at the object, or at the `kind` it requires), not at the field refused.
    let want: Vec<&str> = case
        .get("fault")
        .filter(|_| !schema_accepts)
        .map(|f| case.get("schema_pointer").unwrap_or(&f["pointer"]).as_str().unwrap())
        .into_iter()
        .collect();
    // One object may break two rules at once (the flat form of a role lacks `parts` and holds
    // keys the schema does not name): the schema reports it at that object, once.
    let mut got: Vec<&str> = errors.iter().map(|(at, _)| at.as_str()).collect();
    got.dedup();
    // A missing or an unknown field is reported by the schema at the object that lacks it or
    // holds it; the reader's pointer names the field itself.
    let of_field = matches!(case.get("fault").and_then(|f| f["kind"].as_str()), Some("Missing" | "UnknownField"));
    let at_parent: Vec<&str> = want.iter().map(|p| p.rsplit_once('/').map_or("", |(parent, _)| parent)).collect();
    if of_field && !schema_accepts && got == at_parent {
        return None;
    }
    (got != want).then(|| format!("schema errors {errors:?}, expected at {want:?}"))
}

#[test]
fn every_reading_case_validates_as_its_case_expects() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for version in std::fs::read_dir(common::repo().join("conformance")).unwrap() {
        let version = version.unwrap().path();
        let read = version.join("read");
        if !read.is_dir() {
            continue;
        }
        let schemas = common::Schemas::load(version.file_name().unwrap().to_str().unwrap());
        for case in std::fs::read_dir(&read).unwrap() {
            let case = case.unwrap().path();
            if let Some(why) = schema_errors(&case, &schemas) {
                failures.push(format!("{}: {why}", case.display()));
            }
            checked += 1;
        }
    }
    assert!(checked >= 2, "{checked} cases");
    common::report("cases do not validate as they expect", failures);
}

#[test]
fn every_file_of_a_resolution_case_that_expects_a_result_validates() {
    let schemas = common::Schemas::load("0.4");
    let mut failures = Vec::new();
    for case in std::fs::read_dir(common::repo().join("conformance/0.4/resolve")).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        if spec.get("result").is_none() {
            continue;
        }
        for (id, text) in files(&case.join("files")) {
            let kind = if id.contains("/_profile") { osd::Kind::Profile } else { osd::Kind::Device };
            if let Some(e) = schemas.check(kind, &serde_json::from_str(&text).unwrap()) {
                failures.push(format!("{}: {id}: {e}", case.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    common::report("files do not validate", failures);
}

#[test]
fn every_resolution_case_of_0_4_passes() {
    every_case(&common::repo().join("conformance/0.4/resolve"), 13, run_resolve);
}

#[test]
fn every_resolution_case_of_0_5_passes() {
    every_case(&common::repo().join("conformance/0.5/resolve"), 37, run_resolve);
}

/// The resolution cases of 0.5 that expect a result, one folder each.
const RESOLVE_RESULT_CASES: [&str; 15] = [
    "midi-of-own-keys", "profile-brings-lua", "resolve-plain-driver", "midi-of-identity-kept", "midi-of-routes-appended",
    "midi-of-own-route-wins", "midi-of-table-absent-in-target", "midi-of-own-block-replaces", "midi-of-chain", "midi-of-credit",
    "midi-of-capabilities-declared-kept", "profile-own-keys-win", "profile-null-drops-key", "profile-family-file", "profile-chain",
];

/// The resolution cases of 0.5 that expect a refusal, with the kind of the fault.
const RESOLVE_FAULT_CASES: [(&str, &str); 22] = [
    ("midi-of-target-keys-refused", "UnknownField"),
    ("midi-of-schema-not-0-5-refused", "UnreadVersion"),
    ("driver-missing", "LinkMissing"),
    ("syntax-refused", "Syntax"),
    ("target-file-read-refused", "UnknownField"),
    ("midi-of-not-string", "BadValue"),
    ("midi-of-id-malformed", "BadValue"),
    ("midi-of-other-maker", "BadValue"),
    ("midi-of-route-without-osc", "Missing"),
    ("midi-of-route-twice", "BadValue"),
    ("midi-of-target-missing", "LinkMissing"),
    ("midi-of-cycle", "LinkCycle"),
    ("midi-of-too-deep", "LinkTooDeep"),
    ("midi-of-other-device", "BadValue"),
    ("midi-of-table-extra-key", "UnknownField"),
    ("profile-not-string", "BadValue"),
    ("profile-id-malformed", "BadValue"),
    ("profile-other-maker", "BadValue"),
    ("profile-cycle", "LinkCycle"),
    ("profile-too-deep", "LinkTooDeep"),
    ("profile-file-missing", "LinkMissing"),
    ("profile-block-missing", "LinkMissing"),
];

#[test]
fn the_resolution_cases_of_0_5_hold_each_rule_as_a_result_and_each_refusal_as_its_fault() {
    let root = common::repo().join("conformance/0.5/resolve");
    let names = case_names(&root);
    let spec_of = |n: &str| -> Value {
        let text = std::fs::read_to_string(root.join(n).join("case.json")).unwrap_or_else(|_| panic!("no case {n} in {names:?}"));
        serde_json::from_str(&text).unwrap()
    };
    let mut missing = Vec::new();
    for want in RESOLVE_RESULT_CASES {
        if !names.iter().any(|n| n == want) {
            missing.push(want);
            continue;
        }
        let spec = spec_of(want);
        assert!(spec.get("result").is_some() && spec.get("fault").is_none(), "{want} expects a result");
    }
    for (want, _) in RESOLVE_FAULT_CASES {
        if !names.iter().any(|n| n == want) {
            missing.push(want);
        }
    }
    assert!(missing.is_empty(), "no case {missing:?} in {names:?}");
    for (want, kind) in RESOLVE_FAULT_CASES {
        let spec = spec_of(want);
        assert!(spec.get("result").is_none(), "{want} expects a fault, not a result");
        assert_eq!(spec["fault"]["kind"], kind, "{want}");
        assert!(spec["fault"]["file"].is_string() && spec["fault"]["pointer"].is_string(), "{want}: {spec}");
    }
}

/// The cases whose files the 0.5 schemas are not meant to accept: the text is
/// not JSON, the refusal is the schema's own, or a key the schema forbids.
const RESOLVE_CASES_WITH_FILES_THE_SCHEMA_REFUSES: [&str; 7] = [
    "syntax-refused", "target-file-read-refused", "midi-of-schema-not-0-5-refused", "midi-of-route-without-osc",
    "midi-of-table-extra-key", "midi-of-not-string", "profile-not-string",
];

#[test]
fn every_file_of_a_resolution_case_of_0_5_validates_except_those_whose_refusal_the_schema_states() {
    let schemas = common::Schemas::load("0.5");
    let root = common::repo().join("conformance/0.5/resolve");
    let mut failures = Vec::new();
    let names = case_names(&root);
    for case in &names {
        if RESOLVE_CASES_WITH_FILES_THE_SCHEMA_REFUSES.contains(&case.as_str()) {
            continue;
        }
        for (id, text) in files(&root.join(case).join("files")) {
            let kind = if id.contains("/_profile") { osd::Kind::Profile } else { osd::Kind::Device };
            match serde_json::from_str::<Value>(&text) {
                Err(e) => failures.push(format!("{case}: {id}: not JSON: {e}")),
                Ok(doc) => {
                    if let Some(e) = schemas.check(kind, &doc) {
                        failures.push(format!("{case}: {id}: {e}"));
                    }
                }
            }
        }
    }
    common::report("files do not validate", failures);
}

#[test]
fn a_resolution_case_of_0_5_states_the_lua_a_profile_brings_as_inherited_with_valid_files() {
    let case = common::repo().join("conformance/0.5/resolve/profile-brings-lua");
    let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
    assert_eq!(spec["inherited"], serde_json::json!(["lua"]));
    let schemas = common::Schemas::load("0.5");
    let mut seen = 0;
    for (id, text) in files(&case.join("files")) {
        let kind = if id.contains("/_profile") { osd::Kind::Profile } else { osd::Kind::Device };
        let doc: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(schemas.check(kind, &doc), None, "{id}");
        seen += 1;
    }
    assert_eq!(seen, 2);
}

#[test]
fn the_conformance_readme_describes_the_inherited_key_of_a_resolution_case() {
    let readme = std::fs::read_to_string(common::repo().join("conformance/README.md")).unwrap();
    assert!(readme.contains("`inherited`"), "README.md describes `inherited`");
}

#[test]
fn the_resolution_cases_of_0_5_carry_schema_and_cover_a_result_a_target_key_and_a_bad_schema() {
    let (mut result, mut target_key, mut bad_schema) = (0, 0, 0);
    for case in std::fs::read_dir(common::repo().join("conformance/0.5/resolve")).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        for (id, text) in files(&case.join("files")) {
            assert!(text.contains("\"$schema\""), "{}: {id} carries its `$schema`", case.display());
        }
        if spec["result"]["$schema"] == "https://w3id.org/osd/0.5/device.schema.json" {
            result += 1;
        }
        if spec["fault"]["pointer"] == "/sysex" {
            target_key += 1;
        }
        if spec["fault"]["pointer"] == "/$schema" {
            bad_schema += 1;
        }
    }
    assert!(result >= 1, "a result that keeps the target's $schema");
    assert!(target_key >= 1, "a refused target key");
    assert!(bad_schema >= 1, "a refused $schema");
}

#[test]
fn criterion_5_every_reading_case_of_0_5_passes() {
    every_case(&common::repo().join("conformance/0.5/read"), 44, run_read);
}

#[test]
fn every_reading_case_of_the_0_6_draft_passes() {
    every_case(&common::repo().join("conformance/0.6/read"), 66, run_read);
}

#[test]
fn criterion_5_the_reading_cases_of_0_5_cover_each_kind_and_a_refused_version() {
    let mut results = std::collections::BTreeSet::new();
    let mut unread = 0;
    for case in std::fs::read_dir(common::repo().join("conformance/0.5/read")).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        let text = std::fs::read_to_string(case.join("descriptor.json")).unwrap();
        assert!(text.contains("\"$schema\""), "{}: a 0.5 case carries its `$schema`", case.display());
        if spec.get("result").is_some() {
            results.insert(spec["read"].as_str().unwrap().to_string());
        }
        if spec["fault"]["kind"] == "UnreadVersion" {
            unread += 1;
        }
    }
    let all: std::collections::BTreeSet<String> =
        ["device", "profile", "sound", "fingerprint", "firmware"].iter().map(|s| s.to_string()).collect();
    assert_eq!(results, all);
    assert!(unread >= 1, "no case of an unread version");
}

/// What a conversion case asked for that `convert` did not return, if
/// anything. A result is the 0.5 document, equal to the converted text's,
/// key order aside; it reads as 0.5, validates against the 0.5 schema and
/// converts to itself. A fault is the refusal, by its `kind` and the JSON
/// `pointer` of the 0.4 element refused.
fn run_convert(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let kind = kind(case["convert"].as_str().unwrap());
    match (osd::convert(&text, kind), case.get("result"), case.get("fault")) {
        (Ok(out), Some(want), None) => {
            let doc: Value = serde_json::from_str(&out).unwrap();
            if doc != *want {
                return Some(format!("converted into {doc}"));
            }
            if let Err(f) = osd::read_as(&out, kind, osd::Version::V0_5) {
                return Some(format!("the result does not read: {f}"));
            }
            if let Some(e) = common::Schemas::load("0.5").check(kind, &doc) {
                return Some(format!("the result does not validate: {e}"));
            }
            (osd::convert(&out, kind).as_deref() != Ok(out.as_str())).then(|| "converting the result changes it".into())
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        (Ok(_), None, Some(want)) => Some(format!("converted, but should be refused with {want}")),
        (Err(f), Some(_), None) => Some(format!("refused: {f}")),
        _ => Some("case.json has neither result nor fault, or both".into()),
    }
}

/// The names of the cases of a folder.
fn case_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn criterion_4_every_conversion_case_of_0_5_passes() {
    every_case(&common::repo().join("conformance/0.5/convert"), 24, run_convert);
}

#[test]
fn criterion_4_the_conversion_cases_cover_each_frame_place_each_form_and_the_two_faults() {
    let root = common::repo().join("conformance/0.5/convert");
    let names = case_names(&root);
    for want in [
        "frame-commands", "frame-params", "frame-replies", "frame-identity", "frame-programs-before",
        "frame-programs-dump", "frame-profile", "pre-tree", "fault-checksum-without-algorithm", "fault-function",
    ] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
    }
    // criterion 12: `lsb` and `msb` are a refused encoding, with the address of the fault
    let name = "fault-encoding-lsb-msb";
    assert!(names.iter().any(|n| n == name), "no case {name} in {names:?}");
    let spec: Value = serde_json::from_str(&std::fs::read_to_string(root.join(name).join("case.json")).unwrap()).unwrap();
    assert_eq!(spec["fault"]["kind"], "NotConvertible", "{name}");
    assert!(spec["fault"]["pointer"].as_str().is_some_and(|p| p.starts_with("/commands/")), "{name}: {spec}");
    let text = std::fs::read_to_string(root.join(name).join("descriptor.json")).unwrap();
    assert!(text.contains(":lsb}") && text.contains(":msb}"), "{name}: shows neither lsb nor msb");
    let all: String = names
        .iter()
        .filter(|n| !n.starts_with("fault"))
        .map(|n| std::fs::read_to_string(root.join(n).join("case.json")).unwrap())
        .collect();
    for form in [
        "\"arg\"", "\"encoding\": \"u14_msb\"", "\"encoding\": \"u14_lsb\"", "\"encoding\": \"s32\"", "\"encoding\": \"ascii\"",
        "\"encoding\": \"bytes\"", "\"add\": 4", "\"capture\"", "\"device_id\"", "\"identity\": \"family\"", "\"identity\": \"member\"",
        "\"index\"", "\"checksum\": \"twos7\"", "\"from\"", "\"any\"", "\"sub\"", "\"mul\"", "\"gt\"", "\"lt\"",
        "\"eq\"", "\"ne\"", "\"if\"",
    ] {
        assert!(all.contains(form), "no conversion case shows {form}");
    }
}

#[test]
fn criterion_4_and_8_the_cases_read_noassertion_and_none_and_refuse_none_in_lowercase_for_each_kind() {
    let root = common::repo().join("conformance/0.5/read");
    let names = case_names(&root);
    for kind in ["device", "sound", "firmware"] {
        for (word, text) in [("noassertion", "NOASSERTION"), ("none", "NONE")] {
            let name = format!("license-{word}-{kind}");
            assert!(names.contains(&name), "no case {name} in {names:?}");
            let s = spec(&root, &name);
            assert_eq!(s["read"], kind, "{name}");
            assert_eq!(run_read(&root.join(&name)), None, "{name}");
            let doc = std::fs::read_to_string(root.join(&name).join("descriptor.json")).unwrap();
            assert!(doc.contains(&format!("\"license\": \"{text}\"")), "{name}: {doc}");
            assert!(s.get("result").is_some(), "{name} is read");
        }
        let name = format!("license-none-word-{kind}-refused");
        assert!(names.contains(&name), "no case {name} in {names:?}");
        let s = spec(&root, &name);
        assert_eq!(s["read"], kind, "{name}");
        let pointer = if kind == "device" { "/_sources/0/license" } else { "/license" };
        assert_eq!(s["fault"], serde_json::json!({"kind": "BadValue", "pointer": pointer}), "{name}");
        let doc = std::fs::read_to_string(root.join(&name).join("descriptor.json")).unwrap();
        assert!(doc.contains("\"license\": \"none\""), "{name}: {doc}");
    }
}

#[test]
fn criterion_4_and_8_a_conversion_case_maps_the_0_4_license_none_to_none() {
    let root = common::repo().join("conformance/0.5/convert");
    let name = "license-none";
    assert!(case_names(&root).iter().any(|n| n == name), "no case {name}");
    assert_eq!(run_convert(&root.join(name)), None);
    let s = spec(&root, name);
    assert_eq!(s["result"]["_sources"][0]["license"], "NONE");
    let doc = std::fs::read_to_string(root.join(name).join("descriptor.json")).unwrap();
    assert!(doc.contains("\"license\": \"none\""), "{doc}");
}

/// A case's `case.json`.
fn spec(root: &Path, name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(root.join(name).join("case.json")).unwrap()).unwrap()
}

#[test]
fn a_conversion_case_carries_the_notes_of_a_0_4_midi_in_to_the_device() {
    let root = common::repo().join("conformance/0.5/convert");
    let carries = case_names(&root).iter().any(|n| {
        let dir = root.join(n);
        let Ok(text) = std::fs::read_to_string(dir.join("descriptor.json")) else { return false };
        let from: Value = serde_json::from_str(&text).unwrap();
        let note = from.get("midi_in").and_then(Value::as_object).is_some_and(|m| m.keys().any(|k| k.starts_with('_')));
        let result = &spec(&root, n)["result"];
        note && result.as_object().is_some_and(|o| o.keys().any(|k| k.starts_with("_midi_in_")))
    });
    assert!(carries, "no conversion case with a note in midi_in carried to a _midi_in_ key");
}

#[test]
fn criterion_12_the_conversion_cases_hold_each_address_and_argument_form_and_the_midi_in_list() {
    let root = common::repo().join("conformance/0.5/convert");
    let names = case_names(&root);
    for want in [
        "address-segments", "forward-args", "emit-args", "midi-in-list", "midi-in-empty",
        "fault-brace-unclosed", "fault-brace-empty", "fault-midi-template",
    ] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
    }
    let text = |n: &str| serde_json::to_string(&spec(&root, n)["result"]).unwrap();
    let all: String = ["address-segments", "forward-args", "emit-args"].iter().map(|n| text(n)).collect();
    assert!(all.contains("{\"capture\":"), "no segment capture in the address cases");
    assert!(text("forward-args").contains("{\"arg\":"), "forward-args shows no arg object");
    assert!(text("emit-args").contains("\"emit_args\""), "emit-args shows no emit_args");
    assert!(text("emit-args").contains("{\"capture\":"), "emit-args shows no capture argument");
    let literal = spec(&root, "forward-args")["result"].to_string();
    assert!(literal.contains("\"args\":[") && literal.chars().any(|c| c.is_ascii_digit()), "forward-args shows no literal");
    assert!(text("midi-in-list").contains("\"midi_in\":["), "midi-in-list shows no list");
    assert!(text("midi-in-empty").contains("\"midi_in\":[]"), "midi-in-empty shows no empty list");
    // the four address places each have a case that converts them
    let places = ["\"osc\":[", "\"path\":[", "\"match_osc\":[", "\"emit_osc\":["];
    let converted: String = names.iter().filter(|n| !n.starts_with("fault")).map(|n| spec(&root, n)["result"].to_string()).collect();
    for p in places {
        assert!(converted.contains(p), "no conversion case shows {p}");
    }
    for (name, kind) in [("fault-brace-unclosed", "NotConvertible"), ("fault-brace-empty", "NotConvertible"), ("fault-midi-template", "NotConvertible")] {
        let s = spec(&root, name);
        assert_eq!(s["fault"]["kind"], kind, "{name}");
        assert!(s["fault"]["pointer"].as_str().is_some_and(|p| p.starts_with('/')), "{name}: {s}");
    }
    assert!(spec(&root, "fault-midi-template")["fault"]["pointer"].as_str().unwrap().starts_with("/midi_in/"));
    let d = |n: &str| std::fs::read_to_string(root.join(n).join("descriptor.json")).unwrap();
    assert!(d("fault-brace-empty").contains("{}"), "fault-brace-empty shows no empty brace");
    assert!(d("fault-brace-unclosed").matches('{').count() > d("fault-brace-unclosed").matches('}').count(), "no unclosed brace");
}

#[test]
fn criterion_12_the_reading_cases_hold_each_form_and_refuse_each_malformed_one() {
    let root = common::repo().join("conformance/0.5/read");
    let names = case_names(&root);
    for want in ["address-segments", "args-forms", "midi-in-list", "midi-in-empty"] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
        assert!(spec(&root, want).get("result").is_some(), "{want}: a form read has a result");
    }
    assert!(spec(&root, "midi-in-empty")["result"]["midi_in"] == serde_json::json!([]));
    assert!(spec(&root, "midi-in-list")["result"]["midi_in"].as_array().is_some_and(|l| l.len() >= 2));
    let forms = spec(&root, "args-forms")["result"].to_string();
    assert!(forms.contains("{\"arg\":") && forms.contains("{\"capture\":"), "args-forms: {forms}");
    for (name, pointer_ends) in [
        ("address-brace-refused", "osc"),
        ("address-one-segment-refused", "osc"),
        ("arg-unknown-object-refused", ""),
        ("midi-in-unknown-refused", ""),
    ] {
        assert!(names.iter().any(|n| n == name), "no case {name} in {names:?}");
        let s = spec(&root, name);
        assert!(matches!(s["fault"]["kind"].as_str(), Some("BadValue" | "UnknownField")), "{name}: {s}");
        assert!(s["fault"]["pointer"].as_str().is_some_and(|p| p.ends_with(pointer_ends)), "{name}: {s}");
    }
    let p = |n: &str| spec(&root, n)["fault"]["pointer"].as_str().unwrap().to_string();
    assert!(p("arg-unknown-object-refused").contains("/args/"), "the fault is at the argument");
    assert!(p("midi-in-unknown-refused").starts_with("/midi_in/"), "the fault is at the type");
}

#[test]
fn criterion_5_and_6_the_reading_cases_of_0_5_hold_each_frame_form_and_refuse_each_malformed_element() {
    let root = common::repo().join("conformance/0.5/read");
    let names = case_names(&root);
    for want in [
        "frame-string-refused", "frame-two-keys-refused", "frame-unknown-key-refused", "frame-encoding-refused",
        "frame-algorithm-refused", "frame-index-refused", "pre-string-refused",
    ] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(root.join(want).join("case.json")).unwrap()).unwrap();
        assert!(spec.get("fault").is_some(), "{want}: a refusal is a fault");
    }
    for want in ["frame-forms", "frame-recognise", "pre-tree"] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
    }
}
