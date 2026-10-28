//! What the tests share: a catalogue read from a folder, and the published
//! schemas as validators.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The repository's root, where the schemas, the examples and the
/// conformance cases live.
pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// A catalogue folder (`<maker>/<name>.json`), given by `SYNTHHUB_DEVICES`;
/// none when it is not set, and the test that needs it is skipped.
pub fn catalogue() -> Option<Folder> {
    let dir = std::env::var_os("SYNTHHUB_DEVICES")?;
    Some(Folder(PathBuf::from(dir)))
}

/// Files kept in a folder by catalogue id: `korg/minilogue` is
/// `<folder>/korg/minilogue.json`.
pub struct Folder(pub PathBuf);

impl osd::Files for Folder {
    fn get(&self, id: &str) -> Option<String> {
        std::fs::read_to_string(self.0.join(format!("{id}.json"))).ok()
    }
}

impl Folder {
    /// Every `.json` file of every maker folder, with its catalogue id.
    fn ids(&self) -> Vec<String> {
        let mut out = Vec::new();
        for maker in std::fs::read_dir(&self.0).unwrap() {
            let maker = maker.unwrap().path();
            if !maker.is_dir() {
                continue;
            }
            for f in std::fs::read_dir(&maker).unwrap() {
                let f = f.unwrap().path();
                if f.extension().is_some_and(|e| e == "json") {
                    let m = maker.file_name().unwrap().to_str().unwrap();
                    let n = f.file_stem().unwrap().to_str().unwrap();
                    out.push(format!("{m}/{n}"));
                }
            }
        }
        out.sort();
        out
    }

    /// The drivers: every file whose name does not start with `_`.
    pub fn drivers(&self) -> Vec<String> {
        self.ids().into_iter().filter(|id| !name(id).starts_with('_')).collect()
    }

    /// The maker profiles: `_profile.json` and `_profile.<family>.json`.
    pub fn profiles(&self) -> Vec<String> {
        self.ids().into_iter().filter(|id| name(id).starts_with("_profile")).collect()
    }

    pub fn text(&self, id: &str) -> String {
        osd::Files::get(self, id).unwrap()
    }
}

fn name(id: &str) -> &str {
    id.split_once('/').unwrap().1
}

/// The published 0.4 schemas, one validator per kind.
pub struct Schemas {
    validators: Vec<(osd::Kind, jsonschema::Validator)>,
}

impl Schemas {
    pub fn load() -> Self {
        let dir = repo().join("schemas/0.4");
        let read = |kind: &str| -> serde_json::Value {
            let text = std::fs::read_to_string(dir.join(format!("{kind}.schema.json"))).unwrap();
            serde_json::from_str(&text).unwrap()
        };
        let kinds = [
            (osd::Kind::Device, "device"),
            (osd::Kind::Profile, "profile"),
            (osd::Kind::Sound, "sound"),
            (osd::Kind::Fingerprint, "fingerprint"),
            (osd::Kind::Firmware, "firmware"),
        ];
        let mut registry = jsonschema::Registry::new();
        for (_, name) in kinds {
            let schema = read(name);
            let id = schema["$id"].as_str().unwrap().to_string();
            registry = registry.add(id, schema).unwrap();
        }
        let registry = registry.prepare().unwrap();
        let validators = kinds
            .iter()
            .map(|(kind, name)| {
                let v = jsonschema::options().with_registry(&registry).build(&read(name)).unwrap();
                (*kind, v)
            })
            .collect();
        Schemas { validators }
    }

    /// The first error the schema of `kind` finds in `doc`, if any.
    pub fn check(&self, kind: osd::Kind, doc: &serde_json::Value) -> Option<String> {
        let (_, v) = self.validators.iter().find(|(k, _)| *k == kind).unwrap();
        v.iter_errors(doc).next().map(|e| format!("{}: {e}", e.instance_path()))
    }
}

/// Lists at most a few failures, then fails with their count.
pub fn report(what: &str, failures: Vec<String>) {
    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(10).collect();
        panic!("{} {what}:\n{}", failures.len(), shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    }
}
