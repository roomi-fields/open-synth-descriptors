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
    // `resolve_assembly` names an assembly instead of a driver: its MIDI is resolved, not a driver's.
    let files = files(&dir.join("files"));
    let got = match case["resolve_assembly"].as_str() {
        // `device`, when the case gives it, is the device descriptor the assembly is resolved for.
        Some(id) => osd::resolve_assembly(id, case["device"].as_str(), &files),
        None => osd::resolve(case["resolve"].as_str().unwrap(), &files),
    };
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
        "assembly" => osd::Kind::Assembly,
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
    settings_claims(device, case)
}

/// What a reading case states of the settings of a sound: `settings` (`{transfer, index, message,
/// settings: [{control, param_path, value}]}`, the list `sound_settings` gives, in order) and
/// `comparisons` (`{transfer, a: {index, message}, b: {index, message}, result: [{control,
/// param_path, outcome}]}`, `outcome` being `equal`, `differ` or `absent`).
fn settings_claims(device: &osd::Device, case: &Value) -> Option<String> {
    for want in case.get("settings").and_then(Value::as_array).into_iter().flatten() {
        let got = osd::sound_settings(device, want["transfer"].as_str().unwrap(), &hex_bytes(want["message"].as_str().unwrap()), want["index"].as_u64().unwrap() as usize);
        let got: Vec<Value> = match got {
            Ok(list) => list.into_iter().map(|s| serde_json::json!({"control": s.control, "param_path": s.param_path, "value": s.value})).collect(),
            Err(f) => return Some(format!("settings refused: {f}, the case states {want}")),
        };
        if Value::Array(got.clone()) != want["settings"] {
            return Some(format!("settings {got:?}, the case states {}", want["settings"]));
        }
    }
    for want in case.get("comparisons").and_then(Value::as_array).into_iter().flatten() {
        let transfer = want["transfer"].as_str().unwrap();
        let list = |side: &Value| osd::sound_settings(device, transfer, &hex_bytes(side["message"].as_str().unwrap()), side["index"].as_u64().unwrap() as usize);
        let (a, b) = match (list(&want["a"]), list(&want["b"])) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(f), _) | (_, Err(f)) => return Some(format!("settings refused: {f}")),
        };
        let got: Vec<Value> = osd::compare_settings(&a, &b)
            .into_iter()
            .map(|c| {
                let outcome = match c.outcome {
                    osd::Comparison::Equal => "equal",
                    osd::Comparison::Differ => "differ",
                    osd::Comparison::Absent => "absent",
                };
                serde_json::json!({"control": c.control, "param_path": c.param_path, "outcome": outcome})
            })
            .collect();
        if Value::Array(got.clone()) != want["result"] {
            return Some(format!("comparison {got:?}, the case states {}", want["result"]));
        }
    }
    None
}

/// What a reading case states of the base versions of a firmware: `comparisons` (`{a, b, order}`,
/// `order` being `less`, `equal`, `greater` or `unknown` outside the grammar) and `fits` (`{target, version, fits}`: whether the
/// version fits the target of that index, `yes`, `no` or `unknown`).
fn firmware_claims(d: &osd::Descriptor, case: &Value) -> Option<String> {
    let osd::Descriptor::Firmware(f) = d else { return None };
    for want in case.get("comparisons").and_then(Value::as_array).into_iter().flatten() {
        let got = match osd::compare_versions(want["a"].as_str().unwrap(), want["b"].as_str().unwrap()) {
            Some(std::cmp::Ordering::Less) => "less",
            Some(std::cmp::Ordering::Equal) => "equal",
            Some(std::cmp::Ordering::Greater) => "greater",
            None => "unknown",
        };
        if want["order"] != got {
            return Some(format!("compared {got}, the case states {want}"));
        }
    }
    let fits = case.get("fits").and_then(Value::as_array)?;
    let targets = f.targets.as_deref().unwrap_or_default();
    for want in fits {
        let Some(target) = targets.get(want["target"].as_u64().unwrap() as usize) else { return Some(format!("no target for {want}")) };
        let got = match osd::version_in_target(want["version"].as_str().unwrap(), target) {
            osd::Fit::Yes => "yes",
            osd::Fit::No => "no",
            osd::Fit::Unknown => "unknown",
        };
        if want["fits"] != got {
            return Some(format!("fits {got}, the case states {want}"));
        }
    }
    None
}

/// What a reading case states of the warnings of its descriptor, when it does: `warnings`, the
/// ordered list of `{kind, pointer}` that `osd::warnings` gives.
fn warning_claims(d: &osd::Descriptor, case: &Value) -> Option<String> {
    let want = case.get("warnings")?;
    let got: Vec<Value> = osd::warnings(d)
        .into_iter()
        .map(|w| {
            let kind = match w.kind {
                osd::WarningKind::PartialControl => "PartialControl",
                osd::WarningKind::UnknownProvider(_) => "UnknownProvider",
            };
            serde_json::json!({"kind": kind, "pointer": w.pointer})
        })
        .collect();
    let got = Value::Array(got);
    (got != *want).then(|| format!("warned {got}, the case states {want}"))
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
            warning_claims(&d, &case).or_else(|| sound_transfer_claims(&d, &case)).or_else(|| firmware_claims(&d, &case))
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
    every_case(&common::repo().join("conformance/0.5/resolve"), 42, run_resolve);
}

/// The resolution cases whose files are of two versions: a file without
/// `$schema` is a 0.4 file.
const MIXED_VERSION_CASES: [&str; 5] = [
    "midi-of-older-target-converted", "midi-of-newer-target-converts-the-file", "midi-of-chain-older-targets-converted",
    "midi-of-older-target-credit-kept", "midi-of-older-target-not-convertible-refused",
];

#[test]
fn the_mixed_version_cases_resolve_into_one_stated_version_or_refuse_where_the_conversion_fails() {
    let root = common::repo().join("conformance/0.5/resolve");
    for name in MIXED_VERSION_CASES {
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(root.join(name).join("case.json")).unwrap_or_else(|_| panic!("no case {name}"))).unwrap();
        let texts = files(&root.join(name).join("files"));
        let (mut older, mut newer) = (0, 0);
        for text in texts.values() {
            if text.contains("\"$schema\"") { newer += 1 } else { older += 1 }
        }
        assert!(older >= 1 && newer >= 1, "{name}: files of two versions ({older} without $schema, {newer} with)");
        if name == "midi-of-older-target-not-convertible-refused" {
            assert_eq!(spec["fault"], serde_json::json!({"kind": "NotConvertible", "file": "acme/fw-1", "pointer": "/params/get_frame/1"}));
        } else {
            assert_eq!(spec["result"]["$schema"], "https://w3id.org/osd/0.5/device.schema.json", "{name}");
        }
    }
    let credit: Value = serde_json::from_str(&std::fs::read_to_string(root.join("midi-of-older-target-credit-kept/case.json")).unwrap()).unwrap();
    assert!(credit["result"]["_sources"].is_array() && credit["result"]["_midi_of_sources"].is_array());
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
const RESOLVE_CASES_WITH_FILES_THE_SCHEMA_REFUSES: [&str; 9] = [
    "syntax-refused", "target-file-read-refused", "midi-of-schema-not-0-5-refused", "midi-of-route-without-osc",
    "midi-of-table-extra-key", "midi-of-not-string", "profile-not-string", "programs-dump-null-refused",
    "identity-request-null-refused",
];

#[test]
fn every_file_of_a_resolution_case_of_0_5_validates_except_those_whose_refusal_the_schema_states() {
    let schemas = common::Schemas::load("0.5");
    let schemas_04 = common::Schemas::load("0.4");
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
                    // A file without `$schema` is a 0.4 file, which the 0.4 schemas judge.
                    let own = if doc.get("$schema").is_none() { &schemas_04 } else { &schemas };
                    if let Some(e) = own.check(kind, &doc) {
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
            let mixed = MIXED_VERSION_CASES.iter().any(|m| case.ends_with(m));
            assert!(mixed || text.contains("\"$schema\""), "{}: {id} carries its `$schema`", case.display());
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
    every_case(&common::repo().join("conformance/0.6/read"), 161, run_read);
}

#[test]
fn the_0_6_cases_state_equal_lower_higher_letters_and_a_version_outside_the_grammar() {
    let root = common::repo().join("conformance/0.6/read");
    let orders = |name: &str| -> Vec<(String, String, String)> {
        let text = std::fs::read_to_string(root.join(name).join("case.json")).unwrap_or_else(|e| panic!("{name}: {e}"));
        let case: Value = serde_json::from_str(&text).unwrap();
        case["comparisons"].as_array().into_iter().flatten().map(|c| (c["a"].as_str().unwrap().into(), c["b"].as_str().unwrap().into(), c["order"].as_str().unwrap().into())).collect()
    };
    let has = |all: &[(String, String, String)], a: &str, b: &str, o: &str| {
        assert!(all.iter().any(|(x, y, z)| (x, y, z) == (&a.to_string(), &b.to_string(), &o.to_string())), "no comparison {a} {o} {b}");
    };
    let all = orders("firmware-targets-compare-versions");
    for (a, b, o) in [("1.40", "1.40.0", "equal"), ("1", "1.0.0", "equal"), ("1.40b", "1.40B", "equal"), ("1.40B", "1.40C", "less"), ("1.40C", "1.41", "less"), ("10", "10a", "less"), ("1.9", "1.10", "less"), ("1.42", "1.41", "greater"), ("1.2", "1.2.1", "less")] {
        has(&all, a, b, o);
    }
    has(&orders("firmware-version-outside-grammar-unknown"), "1.2", "0.3-beta", "unknown");
    let fault: Value = serde_json::from_str(&std::fs::read_to_string(root.join("sound-modules-min-version-outside-grammar-refused/case.json")).unwrap()).unwrap();
    assert_eq!(fault["fault"], serde_json::json!({"kind": "BadValue", "pointer": "/needs/0/min_version"}));
}

#[test]
fn every_resolution_case_of_0_6_passes() {
    every_case(&common::repo().join("conformance/0.6/resolve"), 26, run_resolve);
}

#[test]
fn every_file_of_a_0_6_resolution_case_that_expects_a_result_validates_against_the_schema_of_its_own_version() {
    let (s5, s6) = (common::Schemas::load("0.5"), common::Schemas::load("0.6"));
    let mut failures = Vec::new();
    for case in std::fs::read_dir(common::repo().join("conformance/0.6/resolve")).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        if spec.get("result").is_none() {
            continue;
        }
        for (id, text) in files(&case.join("files")) {
            let doc: Value = serde_json::from_str(&text).unwrap();
            let schemas = if doc["$schema"].as_str().is_some_and(|s| s.contains("/0.5/")) { &s5 } else { &s6 };
            let kind = if id.contains("/_profile") { osd::Kind::Profile } else { osd::declared_kind(&text).unwrap_or(osd::Kind::Device) };
            if let Some(e) = schemas.check(kind, &doc) {
                failures.push(format!("{}: {id}: {e}", case.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    common::report("files do not validate", failures);
}

#[test]
fn the_assembly_cases_of_0_6_hold_the_order_the_empty_list_the_same_as_rule_the_shared_address_and_the_guard() {
    let resolve = common::repo().join("conformance/0.6/resolve");
    let read = common::repo().join("conformance/0.6/read");
    let spec = |root: &Path, n: &str| -> Value {
        let text = std::fs::read_to_string(root.join(n).join("case.json")).unwrap_or_else(|_| panic!("no case {n}"));
        serde_json::from_str(&text).unwrap()
    };
    let entries = |v: &Value| -> Vec<String> { v["result"]["params"]["entries"].as_array().unwrap().iter().map(|e| e["osc"].as_str().unwrap().to_string()).collect() };
    for n in ["assembly-stock-and-two-modules-in-order", "assembly-module-same-as-adds-nothing", "assembly-no-modules", "assembly-two-modules-same-address-no-refusal"] {
        let s = spec(&resolve, n);
        assert!(s["resolve_assembly"].is_string() && s.get("result").is_some(), "{n} resolves an assembly into a result");
    }
    assert_eq!(entries(&spec(&resolve, "assembly-stock-and-two-modules-in-order")), ["/p1", "/p2", "/p3", "/p4"]);
    assert_eq!(entries(&spec(&resolve, "assembly-module-same-as-adds-nothing")), entries(&spec(&resolve, "assembly-no-modules")));
    let shared = entries(&spec(&resolve, "assembly-two-modules-same-address-no-refusal"));
    assert_eq!(shared.iter().filter(|a| a.as_str() == "/p3").count(), 1, "one route at the shared address: {shared:?}");
    // One module on two neighbouring devices of a maker: each resolves to the routes of its own target.
    let on_synth = spec(&resolve, "assembly-one-module-on-digitone");
    let on_keys = spec(&resolve, "assembly-one-module-on-digitone-keys");
    assert_eq!((on_synth["device"].as_str(), on_keys["device"].as_str()), (Some("elektron/digitone"), Some("elektron/digitone-keys")));
    assert_eq!(on_synth["resolve_assembly"], on_keys["resolve_assembly"], "the same assembly");
    assert!(entries(&on_synth) != entries(&on_keys) && !entries(&on_synth).is_empty() && !entries(&on_keys).is_empty());
    let no_target = spec(&resolve, "assembly-module-without-target-for-device-refused");
    assert!(no_target["device"].is_string() && no_target.get("result").is_none());
    assert_eq!(no_target["fault"]["kind"], "BadValue");
    assert_eq!(no_target["fault"]["pointer"], "/targets");
    for (n, kind) in [("assembly-module-same-as-another-refused", "BadValue"), ("assembly-module-midi-guard-refused", "UnknownField")] {
        let s = spec(&resolve, n);
        assert!(s["resolve_assembly"].is_string() && s.get("result").is_none(), "{n} is refused");
        assert_eq!(s["fault"]["kind"], kind, "{n}");
        assert!(s["fault"]["file"].is_string() && s["fault"]["pointer"].is_string(), "{n}");
    }
    let valid = spec(&read, "assembly");
    assert_eq!(valid["read"], "assembly");
    assert!(valid.get("result").is_some());
    for n in ["assembly-stock-absent-refused", "assembly-unknown-field-refused", "assembly-modules-malformed-refused"] {
        let s = spec(&read, n);
        assert_eq!(s["read"], "assembly", "{n}");
        assert!(s.get("fault").is_some(), "{n}");
    }
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
    match (osd::convert_to(&text, kind, osd::Version::V0_5), case.get("result"), case.get("fault")) {
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
            (osd::convert_to(&out, kind, osd::Version::V0_5).as_deref() != Ok(out.as_str())).then(|| "converting the result changes it".into())
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

/// What a conversion case of 0.6 asked for that `convert_to` did not return, if anything: the
/// result is the 0.6 document, equal to the converted text's, key order aside; it reads as 0.6,
/// validates against the 0.6 schema and converts to itself. A fault is the refusal.
fn run_convert_to_0_6(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let kind = kind(case["convert"].as_str().unwrap());
    match (osd::convert_to(&text, kind, osd::Version::V0_6), case.get("result"), case.get("fault")) {
        (Ok(out), Some(want), None) => {
            let doc: Value = serde_json::from_str(&out).unwrap();
            if doc != *want {
                return Some(format!("converted into {doc}"));
            }
            if let Err(f) = osd::read_as(&out, kind, osd::Version::V0_6) {
                return Some(format!("the result does not read: {f}"));
            }
            if let Some(e) = common::Schemas::load("0.6").check(kind, &doc) {
                return Some(format!("the result does not validate: {e}"));
            }
            (osd::convert_to(&out, kind, osd::Version::V0_6).as_deref() != Ok(out.as_str())).then(|| "converting the result changes it".into())
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        _ => Some("the case and the converter disagree".into()),
    }
}

/// What a conversion case of 0.7 asked for that `convert_to` did not return: the result is the
/// 0.7 document, it reads as 0.7, validates against the 0.7 schema and converts to itself.
fn run_convert_to_0_7(dir: &Path) -> Option<String> {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let kind = kind(case["convert"].as_str().unwrap());
    match (osd::convert_to(&text, kind, osd::Version::V0_7), case.get("result"), case.get("fault")) {
        (Ok(out), Some(want), None) => {
            let doc: Value = serde_json::from_str(&out).unwrap();
            if doc != *want {
                return Some(format!("converted into {doc}"));
            }
            if let Err(f) = osd::read_as(&out, kind, osd::Version::V0_7) {
                return Some(format!("the result does not read: {f}"));
            }
            if let Some(e) = common::Schemas::load("0.7").check(kind, &doc) {
                return Some(format!("the result does not validate: {e}"));
            }
            (osd::convert_to(&out, kind, osd::Version::V0_7).as_deref() != Ok(out.as_str())).then(|| "converting the result changes it".into())
        }
        (Err(f), None, Some(want)) => {
            let got = serde_json::json!({"kind": format!("{:?}", f.kind), "pointer": f.pointer});
            (got != *want).then(|| format!("refused with {got} ({f})"))
        }
        _ => Some("the case and the converter disagree".into()),
    }
}

#[test]
fn every_conversion_case_of_0_7_passes() {
    every_case(&common::repo().join("conformance/0.7/convert"), 44, run_convert_to_0_7);
}

#[test]
fn the_0_7_conversion_cases_of_a_mixed_control_pass_from_0_4_and_from_0_5() {
    let root = common::repo().join("conformance/0.7/convert");
    let (mut from_0_4, mut from_0_5) = (0, 0);
    let mut failures = Vec::new();
    for name in [
        "mixed-cc-and-nrpn-control-of-0-4-becomes-two-ways",
        "mixed-cc-and-nrpn-control-of-0-5-becomes-two-ways",
        "mixed-cc-and-rpn-control-becomes-two-ways",
        "mixed-nrpn-and-rpn-control-becomes-two-ways",
        "mixed-three-kinds-control-becomes-three-ways",
        "mixed-control-with-nrpn-of-one-number-refused",
        "mixed-control-with-cc-lsb-without-cc-refused",
        "mixed-control-with-block-without-nrpn-refused",
        "mixed-control-message-claimed-twice-refused",
    ] {
        let dir = root.join(name);
        let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap_or_else(|e| panic!("{name}: {e}"));
        let doc: Value = serde_json::from_str(&text).unwrap();
        match doc["$schema"].as_str() {
            None => from_0_4 += 1,
            Some(s) => {
                assert!(s.contains("/0.5/"), "{name}: {s}");
                from_0_5 += 1;
            }
        }
        failures.extend(run_convert_to_0_7(&dir).map(|why| format!("{name}: {why}")));
    }
    assert!(from_0_4 >= 2 && from_0_5 >= 2, "{from_0_4} cases from 0.4, {from_0_5} from 0.5");
    common::report("cases failed", failures);
}

#[test]
fn every_reading_case_of_0_7_passes() {
    every_case(&common::repo().join("conformance/0.7/read"), 86, run_read);
}

/// What a matching case asked for that `match_identity` did not return, if anything.
fn run_match(dir: &Path) -> Option<String> {
    let json = |f: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(dir.join(f)).unwrap()).unwrap() };
    let case = json("case.json");
    let device = match osd::read(&json("device.json").to_string(), osd::Kind::Device) {
        Ok(osd::Descriptor::Device(d)) => d,
        other => return Some(format!("device.json: {other:?}")),
    };
    let fingerprint = match osd::read(&json("fingerprint.json").to_string(), osd::Kind::Fingerprint) {
        Ok(osd::Descriptor::Fingerprint(f)) => f,
        other => return Some(format!("fingerprint.json: {other:?}")),
    };
    let got: Vec<Value> = osd::match_identity(&device, &fingerprint)
        .into_iter()
        .map(|m| {
            let verdict = match m.verdict {
                osd::MatchVerdict::Matched => "matched",
                osd::MatchVerdict::NotMatched => "not_matched",
                osd::MatchVerdict::NotApplied => "not_applied",
            };
            serde_json::json!({"pointer": m.pointer, "verdict": verdict})
        })
        .collect();
    if case["match_identity"] != true {
        return Some("the case does not state match_identity: true".into());
    }
    (Value::Array(got.clone()) != case["result"]).then(|| format!("matched {got:?}, the case states {}", case["result"]))
}

/// What a case on two product instance ids asked for that the reader did not return, if anything.
fn run_same_product_instance_id(dir: &Path) -> Option<String> {
    let json = |f: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(dir.join(f)).unwrap()).unwrap() };
    let case = json("case.json");
    let read = |f: &str| match osd::read(&json(f).to_string(), osd::Kind::Fingerprint) {
        Ok(osd::Descriptor::Fingerprint(x)) => x,
        other => panic!("{}/{f}: {other:?}", dir.display()),
    };
    let (a, b) = (read("a.json"), read("b.json"));
    let got = match osd::same_product_instance_id(&a, &b) {
        Some(true) => "same",
        Some(false) => "different",
        None => "unknown",
    };
    if case["same_product_instance_id"] != true {
        return Some("the case does not state same_product_instance_id: true".into());
    }
    (case["result"] != got).then(|| format!("answered {got}, the case states {}", case["result"]))
}

#[test]
fn every_identity_matching_case_of_0_7_passes() {
    every_case(&common::repo().join("conformance/0.7/match"), 22, run_match);
}

#[test]
fn every_product_instance_id_case_of_0_7_passes() {
    every_case(&common::repo().join("conformance/0.7/same_product_instance_id"), 7, run_same_product_instance_id);
}

#[test]
fn every_input_of_a_matching_or_instance_id_case_validates_against_the_schema_of_its_own_version() {
    let root = common::repo().join("conformance/0.7");
    let mut failures = Vec::new();
    let mut checked = 0;
    for (kind_folder, inputs) in [
        ("match", [("device.json", osd::Kind::Device), ("fingerprint.json", osd::Kind::Fingerprint)]),
        ("same_product_instance_id", [("a.json", osd::Kind::Fingerprint), ("b.json", osd::Kind::Fingerprint)]),
    ] {
        for case in std::fs::read_dir(root.join(kind_folder)).unwrap() {
            let case = case.unwrap().path();
            for (name, kind) in inputs {
                let doc: Value = serde_json::from_str(&std::fs::read_to_string(case.join(name)).unwrap()).unwrap();
                let v = if doc["$schema"].as_str().is_some_and(|s| s.contains("/0.7/")) { "0.7" } else { "0.6" };
                checked += 1;
                if let Some(e) = common::Schemas::load(v).check(kind, &doc) {
                    failures.push(format!("{}: {name}: {e}", case.display()));
                }
            }
        }
    }
    assert!(checked >= 58, "{checked} files");
    common::report("inputs do not validate", failures);
}

/// One name of case per row of the table of cases of proposal 0004 (more than one where a row
/// names several fields or forms).
#[test]
fn every_row_of_the_table_of_cases_of_proposal_0004_is_a_case_of_0_7() {
    let root = common::repo().join("conformance/0.7");
    for case in [
        "read/port-rules-each-operator-and-direction-accepted",
        "read/port-rule-with-platform-accepted",
        "read/port-rule-with-platform-and-stack-accepted",
        "convert/port-name-becomes-contains-rule",
        "read/port-rule-operator-outside-list-refused",
        "read/port-rule-stack-without-platform-refused",
        "read/port-rule-stack-of-another-platform-refused",
        "read/port-rule-platform-outside-list-refused",
        "read/port-rule-stack-outside-list-refused",
        "read/fingerprint-with-platform-accepted",
        "read/fingerprint-with-platform-and-stack-accepted",
        "read/fingerprint-without-platform-accepted",
        "read/fingerprint-stack-without-platform-refused",
        "read/fingerprint-stack-of-another-platform-refused",
        "match/rule-without-platform-applies-to-a-fingerprint-without",
        "match/platform-rule-applies-to-the-same-platform-without-stack",
        "match/platform-rule-applies-to-the-same-platform-with-a-stack",
        "match/platform-rule-not-applied-to-another-platform",
        "match/stack-rule-applies-to-the-same-platform-and-stack",
        "match/stack-rule-not-applied-without-stack",
        "match/platform-rule-not-applied-without-platform",
        "match/stack-rule-not-applied-to-another-stack",
        "match/case-is-ignored",
        "read/port-rule-empty-value-refused",
        "read/endpoint-name-equals-accepted",
        "read/endpoint-name-contains-accepted",
        "read/endpoint-name-starts-with-accepted",
        "read/endpoint-name-ends-with-accepted",
        "read/endpoint-name-with-direction-refused",
        "read/endpoint-name-with-platform-refused",
        "read/endpoint-name-with-stack-refused",
        "read/fingerprint-with-endpoint-name-and-product-instance-id-accepted",
        "read/device-product-instance-id-in-identity-refused",
        "match/endpoint-rule-not-applied-without-endpoint-name",
        "same_product_instance_id/other-case-is-different",
        "convert/fingerprint-changes-only-its-schema",
        "convert/empty-port-name-is-removed",
        "resolve/profile-0-6-port-names-under-a-0-7-device",
        "resolve/profile-0-7-under-a-0-6-device",
        "resolve/profile-0-6-extending-a-0-7-profile",
        "resolve/identity-from-a-0-7-profile-programs-from-a-0-6-one",
    ] {
        assert!(root.join(case).join("case.json").is_file(), "{case}");
    }
}

#[test]
fn the_0_7_cases_of_the_sources_exist_under_their_names() {
    let root = common::repo().join("conformance/0.7");
    for case in [
        "convert/sources-key-renamed",
        "convert/source-product-keys-become-general-fields",
        "convert/script-text-unchanged",
        "read/underscore-sources-refused",
        "read/underscore-sources-beside-sources-refused",
        "read/underscore-key-is-kept",
        "resolve/midi-of-0-7-file-with-underscore-sources-refused",
        "resolve/midi-of-0-7-file-adds-sources",
    ] {
        assert!(root.join(case).join("case.json").is_file(), "{case}");
    }
}

#[test]
fn the_0_7_cases_of_the_changes_of_form_exist_under_their_names() {
    let root = common::repo().join("conformance/0.7");
    for case in [
        "convert/polarity-from-orientation",
        "convert/orientation-other-word-not-convertible",
        "convert/electra-number-becomes-id",
        "convert/electra-other-shape-stays-a-note",
        "convert/manufacturer-id-moves-to-identity",
        "convert/manufacturer-id-joins-an-existing-identity-block",
        "convert/manufacturer-id-moves-in-a-midi-of-file-with-an-identity-block",
        "convert/manufacturer-id-of-a-midi-of-file-without-identity-stays-a-note",
        "convert/device-without-manufacturer-id-gets-no-identity",
        "read/polarity-accepted",
        "read/orientation-refused",
        "read/polarity-other-word-refused",
        "read/identity-manufacturer-id-accepted",
        "read/identity-manufacturer-id-null-accepted",
        "read/device-manufacturer-id-refused",
        "read/command-id-accepted",
        "read/command-id-not-an-integer-refused",
        "resolve/midi-of-0-7-identity-block-replaces-the-target-manufacturer-id",
        "resolve/midi-of-0-7-file-without-identity-takes-the-target-manufacturer-id",
        "resolve/profile-null-drops-manufacturer-id",
        "resolve/profile-null-manufacturer-id-on-a-profile-without-it-accepted",
    ] {
        assert!(root.join(case).join("case.json").is_file(), "{case}");
    }
}

#[test]
fn every_reading_case_of_0_7_that_expects_a_result_validates_against_the_0_7_schema() {
    let schemas = common::Schemas::load("0.7");
    let mut failures = Vec::new();
    for case in std::fs::read_dir(common::repo().join("conformance/0.7/read")).unwrap() {
        let case = case.unwrap().path();
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(case.join("descriptor.json")).unwrap()).unwrap();
        let errors = schemas.errors(kind(spec["read"].as_str().unwrap()), &doc);
        let name = case.file_name().unwrap().to_string_lossy().to_string();
        // A case that says `schema_accepts` is refused by the reader alone (a claim between ways
        // is no matter for a schema, which cannot compare two entries): the schema accepts it.
        let reader_alone = spec.get("schema_accepts").and_then(Value::as_bool) == Some(true);
        match (spec.get("result").is_some(), errors.is_empty()) {
            (true, false) => failures.push(format!("{name}: {errors:?}")),
            (false, false) if reader_alone => failures.push(format!("{name}: the schema refuses what it should accept: {errors:?}")),
            (false, true) if reader_alone => {}
            // A refusal of `_sources` is the schema's too: no 0.7 text carries the old key.
            (false, true) => failures.push(format!("{name}: the schema accepts what the reader refuses")),
            _ => {}
        }
    }
    common::report("reading cases of 0.7 disagree with the schema", failures);
}

#[test]
fn every_resolution_case_of_0_7_passes() {
    every_case(&common::repo().join("conformance/0.7/resolve"), 16, run_resolve);
}

#[test]
fn the_0_7_resolution_cases_join_a_0_6_target_to_a_0_7_file_and_the_reverse() {
    let root = common::repo().join("conformance/0.7/resolve");
    for (name, file_v, target_v) in [("midi-of-0-7-file-on-0-6-target", "0.7", "0.6"), ("midi-of-0-6-file-on-0-7-target", "0.6", "0.7")] {
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(root.join(name).join("case.json")).unwrap()).unwrap();
        assert_eq!(spec["result"]["$schema"], "https://w3id.org/osd/0.7/device.schema.json", "{name}");
        assert!(spec["result"]["midi_of_sources"].is_array(), "{name}: the target's credit is kept");
        assert!(spec["result"]["_midi_of_sources"].is_null() && spec["result"]["_sources"].is_null(), "{name}: no key of the old form");
        let a = files(&root.join(name).join("files"));
        assert!(a["acme/a"].contains(&format!("/osd/{file_v}/")), "{name}");
        assert!(a["acme/b"].contains(&format!("/osd/{target_v}/")), "{name}");
    }
}

#[test]
fn every_file_of_a_0_7_resolution_case_validates_against_the_schema_of_its_own_version() {
    let mut failures = Vec::new();
    for case in std::fs::read_dir(common::repo().join("conformance/0.7/resolve")).unwrap() {
        let case = case.unwrap().path();
        // A case that expects a refusal holds a file the schema refuses too.
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        if spec.get("fault").is_some() {
            continue;
        }
        for (id, text) in files(&case.join("files")) {
            let doc: Value = serde_json::from_str(&text).unwrap();
            let v = if doc["$schema"].as_str().is_some_and(|s| s.contains("/0.7/")) { "0.7" } else { "0.6" };
            let kind = if id.contains("/_profile") { osd::Kind::Profile } else { osd::Kind::Device };
            if let Some(e) = common::Schemas::load(v).check(kind, &doc) {
                failures.push(format!("{}: {id}: {e}", case.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    common::report("files do not validate", failures);
}

#[test]
fn every_conversion_case_of_0_6_passes() {
    every_case(&common::repo().join("conformance/0.6/convert"), 3, run_convert_to_0_6);
}

#[test]
fn a_0_5_file_with_a_lower_case_hex_converts_to_upper_case_and_a_mixed_control_or_an_empty_content_is_refused_at_its_address() {
    let root = common::repo().join("conformance/0.6/convert");
    for name in [
        "hex-lowercase-becomes-uppercase", "control-mixing-cc-and-nrpn-refused", "sound-without-content-refused",
        "fingerprint-control-mixing-cc-and-nrpn-refused", "fingerprint-name-control-mixing-cc-and-nrpn-refused",
    ] {
        assert_eq!(run_convert_to_0_6(&root.join(name)), None, "{name}");
    }
    let spec = |n: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(root.join(n).join("case.json")).unwrap()).unwrap() };
    assert_eq!(spec("hex-lowercase-becomes-uppercase")["result"]["content"]["sysex"], "F0 42 3C 00 F7");
    assert_eq!(spec("control-mixing-cc-and-nrpn-refused")["fault"], serde_json::json!({"kind": "NotConvertible", "pointer": "/cc_params/entries/0"}));
    assert_eq!(spec("fingerprint-control-mixing-cc-and-nrpn-refused")["fault"], serde_json::json!({"kind": "NotConvertible", "pointer": "/controls/1"}));
    assert_eq!(spec("fingerprint-name-control-mixing-cc-and-nrpn-refused")["fault"], serde_json::json!({"kind": "NotConvertible", "pointer": "/names/0/control"}));
    assert_eq!(spec("sound-without-content-refused")["fault"], serde_json::json!({"kind": "NotConvertible", "pointer": "/content"}));
}

#[test]
fn the_conversion_cases_of_0_6_turn_a_0_5_and_a_0_4_firmware_into_one_target() {
    let root = common::repo().join("conformance/0.6/convert");
    let names = case_names(&root);
    for want in ["firmware-device-becomes-target", "firmware-0-4-device-becomes-target"] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
        let spec: Value = serde_json::from_str(&std::fs::read_to_string(root.join(want).join("case.json")).unwrap()).unwrap();
        assert_eq!(spec["convert"], "firmware", "{want}");
        assert_eq!(spec["result"]["targets"].as_array().map(Vec::len), Some(1), "{want}");
        assert!(spec["result"].get("device").is_none(), "{want}");
    }
    let from_05: Value = serde_json::from_str(&std::fs::read_to_string(root.join("firmware-device-becomes-target/descriptor.json")).unwrap()).unwrap();
    assert_eq!(from_05["$schema"], "https://w3id.org/osd/0.5/firmware.schema.json");
    assert!(from_05["based_on"]["version"].is_string(), "the 0.5 case converts a based_on with its version");
    let out: Value = serde_json::from_str(&std::fs::read_to_string(root.join("firmware-device-becomes-target/case.json")).unwrap()).unwrap();
    assert_eq!(out["result"]["targets"][0]["based_on"], serde_json::json!({"name": "Elektron OS", "version": "1.43", "url": "https://www.elektron.se"}));
    assert!(out["result"]["targets"][0].get("runs_on").is_none(), "the conversion invents no constraint");
    let with_midi = names.iter().any(|n| n == "firmware-midi-moves-into-the-target");
    assert!(with_midi, "no case firmware-midi-moves-into-the-target");
    let moved: Value = serde_json::from_str(&std::fs::read_to_string(root.join("firmware-midi-moves-into-the-target/case.json")).unwrap()).unwrap();
    assert!(moved["result"].get("midi").is_none() && moved["result"]["targets"][0]["midi"].is_object());
}

#[test]
fn the_reading_cases_of_0_6_hold_each_form_of_a_target_and_refuse_a_target_without_device() {
    let root = common::repo().join("conformance/0.6/read");
    let names = case_names(&root);
    let spec = |n: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(root.join(n).join("case.json")).unwrap()).unwrap() };
    for want in ["firmware-targets-list", "firmware-targets-range", "firmware-targets-several-devices", "firmware-targets-compare-versions", "firmware-target-without-device-refused"] {
        assert!(names.iter().any(|n| n == want), "no case {want} in {names:?}");
        assert_eq!(spec(want)["read"], "firmware", "{want}");
    }
    assert!(spec("firmware-targets-list")["result"]["targets"][0]["runs_on"]["versions"].is_array());
    assert!(spec("firmware-targets-range")["result"]["targets"][0]["runs_on"]["versions"]["min"].is_string());
    let several = spec("firmware-targets-several-devices");
    assert!(several["result"]["targets"].as_array().is_some_and(|t| t.len() >= 2));
    assert!(several["result"]["targets"].as_array().unwrap().iter().filter(|t| t["midi"].is_object()).count() >= 2, "each target carries its own midi");
    assert!(several["result"].get("midi").is_none());
    let lineage = spec("firmware-target-lineage-apart-from-runs-on");
    assert_eq!(lineage["read"], "firmware");
    assert!(lineage["result"]["targets"][0]["based_on"]["version"].is_string() && lineage["result"]["targets"][0]["runs_on"]["versions"].is_array());
    assert!(lineage["result"]["targets"][0]["based_on"].get("versions").is_none());
    let midi_up = spec("firmware-midi-at-first-level-refused");
    assert_eq!(midi_up["fault"], serde_json::json!({"kind": "UnknownField", "pointer": "/midi"}));
    let based_versions = spec("firmware-based-on-versions-refused");
    assert_eq!(based_versions["fault"], serde_json::json!({"kind": "UnknownField", "pointer": "/targets/0/based_on/versions"}));
    let compare = spec("firmware-targets-compare-versions");
    assert!(compare["comparisons"].as_array().is_some_and(|c| c.iter().any(|x| x["a"] == "1.40" && x["b"] == "1.40.0" && x["order"] == "equal")));
    assert!(compare["fits"].as_array().is_some_and(|c| {
        let fits = |t: u64, v: &str, f: &str| c.iter().any(|x| x["target"] == t && x["version"] == v && x["fits"] == f);
        fits(0, "1.40C", "yes") && fits(0, "1.42", "no") && c.iter().any(|x| x["target"] == 1)
            && c.iter().any(|x| x["fits"] == "unknown")
    }));
    assert_eq!(compare["result"]["targets"][0]["runs_on"]["versions"], serde_json::json!({"min": "1.40B", "max": "1.41"}));
    assert!(compare["result"]["targets"][1]["runs_on"]["versions"].get("max").is_none());
    assert_eq!(spec("firmware-target-without-device-refused")["fault"], serde_json::json!({"kind": "Missing", "pointer": "/targets/0/device"}));
}

#[test]
fn the_firmware_cases_of_0_6_that_the_0_5_twins_have_read_as_targets() {
    // A 0.5 firmware reading case has its 0.6 twin, whose descriptor names targets, not `device`.
    let root = common::repo().join("conformance");
    for n in case_names(&root.join("0.5/read")) {
        let case = |v: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(root.join(v).join("read").join(&n).join("case.json")).unwrap()).unwrap() };
        if case("0.5")["read"] != "firmware" {
            continue;
        }
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(root.join("0.6/read").join(&n).join("descriptor.json")).unwrap()).unwrap();
        assert!(doc.get("device").is_none() && doc["targets"].is_array(), "{n}");
    }
}
