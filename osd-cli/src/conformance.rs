//! The adapter of the reference reader to the contract of `conformance/RUNNING.md`: it takes one
//! case folder, runs `osd` on what the case states, and writes one JSON object (`result` or
//! `fault`, and the other keys the case expects) on the standard output.

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

/// Why the adapter cannot answer: the folder is not a case, or a case of a kind it does not know.
pub struct NotACase(pub String);

fn read_text(path: &Path) -> Result<String, NotACase> {
    std::fs::read_to_string(path).map_err(|e| NotACase(format!("{}: {e}", path.display())))
}

fn read_json(path: &Path) -> Result<Value, NotACase> {
    serde_json::from_str(&read_text(path)?).map_err(|e| NotACase(format!("{}: {e}", path.display())))
}

fn kind_named(word: &str) -> Result<osd::Kind, NotACase> {
    Ok(match word {
        "device" => osd::Kind::Device,
        "profile" => osd::Kind::Profile,
        "sound" => osd::Kind::Sound,
        "fingerprint" => osd::Kind::Fingerprint,
        "firmware" => osd::Kind::Firmware,
        "assembly" => osd::Kind::Assembly,
        _ => return Err(NotACase(format!("`{word}` is not a kind of descriptor"))),
    })
}

fn fault(f: &osd::Fault) -> Value {
    json!({"fault": {"kind": format!("{:?}", f.kind), "file": f.file, "pointer": f.pointer}})
}

fn hex_bytes(text: &str) -> Vec<u8> {
    text.split_whitespace().filter_map(|b| u8::from_str_radix(b, 16).ok()).collect()
}

fn hex_text(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn index_of(v: &Value) -> usize {
    v.get("index").and_then(Value::as_u64).unwrap_or_default() as usize
}

/// Writes what `osd` answers on the case in `dir`.
pub fn answer(dir: &Path) -> Result<Value, NotACase> {
    let case = read_json(&dir.join("case.json"))?;
    let object = case.as_object().ok_or_else(|| NotACase("case.json is not an object".into()))?;
    if object.contains_key("read") {
        read_case(dir, &case)
    } else if object.contains_key("convert") {
        convert_case(dir, &case)
    } else if object.contains_key("resolve") || object.contains_key("resolve_assembly") {
        resolve_case(dir, &case)
    } else if object.contains_key("coding") {
        Ok(coding_case(&case))
    } else if object.contains_key("judge_needs") {
        judge_case(dir)
    } else if object.contains_key("match_identity") {
        match_case(dir)
    } else if object.contains_key("same_product_instance_id") {
        same_product_instance_id_case(dir)
    } else if dir.join("vocabulary.json").is_file() {
        vocabulary_case(dir)
    } else {
        Err(NotACase("case.json states no kind of case this reader knows".into()))
    }
}

fn read_case(dir: &Path, case: &Value) -> Result<Value, NotACase> {
    let kind = kind_named(str_of(case, "read"))?;
    let text = read_text(&dir.join("descriptor.json"))?;
    let d = match osd::read(&text, kind) {
        Ok(d) => d,
        Err(f) => return Ok(fault(&f)),
    };
    let mut out = Map::new();
    out.insert("result".into(), serde_json::from_str(&osd::write(&d)).unwrap_or(Value::Null));
    let roles: Vec<Value> = osd::roles(&d)
        .into_iter()
        .map(|r| {
            let meaning = if r.meaning == osd::RoleMeaning::Known { "known" } else { "none" };
            json!({"pointer": r.pointer, "meaning": meaning, "paramPath": r.param_path})
        })
        .collect();
    out.insert("roles".into(), Value::Array(roles));
    if case.get("warnings").is_some() {
        out.insert("warnings".into(), warnings_of(&d));
    }
    if let osd::Descriptor::Device(device) = &d {
        sound_claims(device, case, &mut out);
    }
    if let osd::Descriptor::Firmware(f) = &d {
        firmware_claims(f, case, &mut out);
    }
    Ok(Value::Object(out))
}

/// The warnings of a descriptor, as `{kind, pointer}`, in the order the reader gives them.
fn warnings_of(d: &osd::Descriptor) -> Value {
    osd::warnings(d)
        .into_iter()
        .map(|w| {
            let kind = match w.kind {
                osd::WarningKind::PartialControl => "PartialControl",
                osd::WarningKind::UnknownProvider(_) => "UnknownProvider",
            };
            json!({"kind": kind, "pointer": w.pointer})
        })
        .collect()
}

/// `identity_support`, `identities`, `settings` and `comparisons` of a device with sound transfers.
fn sound_claims(device: &osd::Device, case: &Value, out: &mut Map<String, Value>) {
    let support: Vec<Value> = osd::sound_identity_support(device)
        .into_iter()
        .map(|s| json!({"pointer": s.pointer, "without_lua": s.without_lua}))
        .collect();
    out.insert("identity_support".into(), Value::Array(support));
    let list = |key: &str| case.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    let identities: Vec<Value> = list("identities")
        .into_iter()
        .map(|mut w| {
            let got = osd::sound_identity(device, str_of(&w, "transfer"), &hex_bytes(str_of(&w, "message")), index_of(&w));
            w["identity"] = got.map(Value::String).unwrap_or(Value::Null);
            w
        })
        .collect();
    if case.get("identities").is_some() {
        out.insert("identities".into(), Value::Array(identities));
    }
    let settings = |transfer: &str, side: &Value| osd::sound_settings(device, transfer, &hex_bytes(str_of(side, "message")), index_of(side));
    let settings_json = |l: Vec<osd::Setting>| -> Vec<Value> {
        l.into_iter().map(|s| json!({"control": s.control, "param_path": s.param_path, "value": s.value})).collect()
    };
    if case.get("settings").is_some() {
        let got: Vec<Value> = list("settings")
            .into_iter()
            .map(|mut w| {
                let got = settings(str_of(&w, "transfer"), &w);
                w["settings"] = got.map(|l| Value::Array(settings_json(l))).unwrap_or(Value::Null);
                w
            })
            .collect();
        out.insert("settings".into(), Value::Array(got));
    }
    if case.get("comparisons").is_some() && case.get("comparisons").and_then(|c| c.get(0)).is_some_and(|c| c.get("transfer").is_some()) {
        let got: Vec<Value> = list("comparisons")
            .into_iter()
            .map(|mut w| {
                let transfer = str_of(&w, "transfer").to_string();
                let result = match (settings(&transfer, &w["a"]), settings(&transfer, &w["b"])) {
                    (Ok(a), Ok(b)) => Value::Array(
                        osd::compare_settings(&a, &b)
                            .into_iter()
                            .map(|c| {
                                let outcome = match c.outcome {
                                    osd::Comparison::Equal => "equal",
                                    osd::Comparison::Differ => "differ",
                                    osd::Comparison::Absent => "absent",
                                };
                                json!({"control": c.control, "param_path": c.param_path, "outcome": outcome})
                            })
                            .collect(),
                    ),
                    _ => Value::Null,
                };
                w["result"] = result;
                w
            })
            .collect();
        out.insert("comparisons".into(), Value::Array(got));
    }
}

/// `comparisons` (the order of two base versions) and `fits` of a firmware.
fn firmware_claims(f: &osd::Firmware, case: &Value, out: &mut Map<String, Value>) {
    let list = |key: &str| case.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    if case.get("comparisons").and_then(|c| c.get(0)).is_some_and(|c| c.get("order").is_some()) {
        let got: Vec<Value> = list("comparisons")
            .into_iter()
            .map(|mut w| {
                w["order"] = json!(match osd::compare_versions(str_of(&w, "a"), str_of(&w, "b")) {
                    Some(std::cmp::Ordering::Less) => "less",
                    Some(std::cmp::Ordering::Equal) => "equal",
                    Some(std::cmp::Ordering::Greater) => "greater",
                    None => "unknown",
                });
                w
            })
            .collect();
        out.insert("comparisons".into(), Value::Array(got));
    }
    if case.get("fits").is_some() {
        let targets = f.targets.as_deref().unwrap_or_default();
        let got: Vec<Value> = list("fits")
            .into_iter()
            .map(|mut w| {
                let target = w.get("target").and_then(Value::as_u64).unwrap_or_default() as usize;
                w["fits"] = match targets.get(target) {
                    Some(t) => json!(match osd::version_in_target(str_of(&w, "version"), t) {
                        osd::Fit::Yes => "yes",
                        osd::Fit::No => "no",
                        osd::Fit::Unknown => "unknown",
                    }),
                    None => Value::Null,
                };
                w
            })
            .collect();
        out.insert("fits".into(), Value::Array(got));
    }
}

fn convert_case(dir: &Path, case: &Value) -> Result<Value, NotACase> {
    let kind = kind_named(str_of(case, "convert"))?;
    let version_name = dir.canonicalize().ok().and_then(|p| p.parent()?.parent()?.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_default();
    let target = osd::VERSIONS
        .iter()
        .copied()
        .find(|v| v.name() == version_name)
        .ok_or_else(|| NotACase(format!("`{version_name}`, the folder of the version, is not a version of the format")))?;
    let text = read_text(&dir.join("descriptor.json"))?;
    Ok(match osd::convert_to(&text, kind, target) {
        Ok(out) => json!({"result": serde_json::from_str::<Value>(&out).unwrap_or(Value::Null)}),
        Err(f) => fault(&f),
    })
}

fn resolve_case(dir: &Path, case: &Value) -> Result<Value, NotACase> {
    let mut files = BTreeMap::new();
    let root = dir.join("files");
    let makers = std::fs::read_dir(&root).map_err(|e| NotACase(format!("{}: {e}", root.display())))?;
    for maker in makers.flatten() {
        let Ok(entries) = std::fs::read_dir(maker.path()) else { continue };
        for f in entries.flatten() {
            let id = format!("{}/{}", maker.file_name().to_string_lossy(), f.path().file_stem().unwrap_or_default().to_string_lossy());
            files.insert(id, read_text(&f.path())?);
        }
    }
    let got = match case.get("resolve_assembly").and_then(Value::as_str) {
        Some(id) => osd::resolve_assembly(id, case.get("device").and_then(Value::as_str), &files),
        None => osd::resolve(str_of(case, "resolve"), &files),
    };
    Ok(match got {
        Ok(r) => {
            let inherited: Vec<&str> = r.inherited.iter().map(|c| c.word()).collect();
            let doc = osd::write(&osd::Descriptor::Device(Box::new(r.device)));
            json!({"result": serde_json::from_str::<Value>(&doc).unwrap_or(Value::Null), "inherited": inherited})
        }
        Err(f) => fault(&f),
    })
}

fn coding_case(case: &Value) -> Value {
    let coding = match str_of(case, "coding") {
        "mcoded7" => Some(osd::Coding::Mcoded7),
        "msb_first_7in8" => Some(osd::Coding::MsbFirst7in8),
        _ => None,
    };
    match coding {
        Some(c) => json!({"coded": hex_text(&osd::encode_chunk(c, &hex_bytes(str_of(case, "bytes"))))}),
        None => json!({"coded": null}),
    }
}

fn judge_case(dir: &Path) -> Result<Value, NotACase> {
    let sound = read_text(&dir.join("sound.json"))?;
    let assembly = read_text(&dir.join("assembly.json"))?;
    let (sound, assembly) = match (osd::read(&sound, osd::Kind::Sound), osd::read(&assembly, osd::Kind::Assembly)) {
        (Ok(osd::Descriptor::Sound(s)), Ok(osd::Descriptor::Assembly(a))) => (s, a),
        (Err(f), _) | (_, Err(f)) => return Ok(fault(&f)),
        _ => return Err(NotACase("sound.json and assembly.json are not a sound and an assembly".into())),
    };
    let result: Vec<Value> = osd::judge_needs(&sound, &assembly)
        .into_iter()
        .map(|j| {
            let verdict = match j.verdict {
                osd::Verdict::Satisfied => "satisfied",
                osd::Verdict::NotSatisfied => "not_satisfied",
                osd::Verdict::Unknown => "unknown",
            };
            json!({"module": j.module, "verdict": verdict})
        })
        .collect();
    Ok(json!({"result": result}))
}

/// The descriptor of file `name` of the case, read as a `kind`, or the answer its fault is.
fn input(dir: &Path, name: &str, kind: osd::Kind) -> Result<Result<osd::Descriptor, Value>, NotACase> {
    Ok(osd::read(&read_text(&dir.join(name))?, kind).map_err(|f| fault(&f)))
}

fn match_case(dir: &Path) -> Result<Value, NotACase> {
    let device = input(dir, "device.json", osd::Kind::Device)?;
    let fingerprint = input(dir, "fingerprint.json", osd::Kind::Fingerprint)?;
    let (device, fingerprint) = match (device, fingerprint) {
        (Ok(osd::Descriptor::Device(d)), Ok(osd::Descriptor::Fingerprint(f))) => (d, f),
        (Err(f), _) | (_, Err(f)) => return Ok(f),
        _ => return Err(NotACase("device.json and fingerprint.json are not a device and a fingerprint".into())),
    };
    let result: Vec<Value> = osd::match_identity(&device, &fingerprint)
        .into_iter()
        .map(|m| {
            let verdict = match m.verdict {
                osd::MatchVerdict::Matched => "matched",
                osd::MatchVerdict::NotMatched => "not_matched",
                osd::MatchVerdict::NotApplied => "not_applied",
            };
            json!({"pointer": m.pointer, "verdict": verdict})
        })
        .collect();
    Ok(json!({"result": result}))
}

fn same_product_instance_id_case(dir: &Path) -> Result<Value, NotACase> {
    let a = input(dir, "a.json", osd::Kind::Fingerprint)?;
    let b = input(dir, "b.json", osd::Kind::Fingerprint)?;
    let (a, b) = match (a, b) {
        (Ok(osd::Descriptor::Fingerprint(a)), Ok(osd::Descriptor::Fingerprint(b))) => (a, b),
        (Err(f), _) | (_, Err(f)) => return Ok(f),
        _ => return Err(NotACase("a.json and b.json are not two fingerprints".into())),
    };
    let result = match osd::same_product_instance_id(&a, &b) {
        Some(true) => "same",
        Some(false) => "different",
        None => "unknown",
    };
    Ok(json!({"result": result}))
}

fn vocabulary_case(dir: &Path) -> Result<Value, NotACase> {
    let text = read_text(&dir.join("vocabulary.json"))?;
    Ok(match osd::read_vocabulary(&text) {
        Ok(v) => json!({"result": {"terms": v.terms.iter().map(|t| t.name.clone()).collect::<Vec<_>>()}}),
        Err(f) => fault(&f),
    })
}
