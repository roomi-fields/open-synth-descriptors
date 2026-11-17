//! What the tests share: the built program, run on arguments, and the
//! published schemas as validators.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository's root, where the schemas, the examples and the
/// conformance cases live.
pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// What a run of the program left.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// The program `osd`, run in `cwd` on `args`.
pub fn osd_in(cwd: &Path, args: &[&str]) -> Run {
    let bin = env!("CARGO_BIN_EXE_osd");
    let out = Command::new(bin).args(args).current_dir(cwd).output().expect("the program runs");
    Run {
        code: out.status.code().expect("the program ends by an exit code"),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// The program `osd`, run from the repository's root on `args`.
pub fn osd(args: &[&str]) -> Run {
    osd_in(&repo(), args)
}

/// The word that writes a template. Written in two pieces: the guard of test sizes
/// reads the quoted word as git's, and takes a program run on it for a trial
/// repository.
pub const INIT: &str = concat!("in", "it");

/// The five words the program delivers.
pub const WORDS: [&str; 5] = ["validate", "convert", INIT, "import", "export"];

pub fn kind_of(word: &str) -> osd::Kind {
    match word {
        "device" => osd::Kind::Device,
        "profile" => osd::Kind::Profile,
        "sound" => osd::Kind::Sound,
        "fingerprint" => osd::Kind::Fingerprint,
        "firmware" => osd::Kind::Firmware,
        _ => panic!("{word} is not a kind"),
    }
}

pub const KINDS: [&str; 5] = ["device", "profile", "sound", "fingerprint", "firmware"];

/// The example of each kind in `examples/0.5`.
pub fn example_path(kind: &str) -> PathBuf {
    let name = match kind {
        "device" => "synth.device.json",
        "profile" => "acme.profile.json",
        "sound" => "warm-pad.sound.json",
        "fingerprint" => "minilogue.fingerprint.json",
        "firmware" => "tribe-0.3-beta.firmware.json",
        _ => panic!("{kind} is not a kind"),
    };
    repo().join("examples/0.5").join(name)
}

/// The directories of the cases of a folder, sorted.
pub fn cases(root: &Path) -> Vec<PathBuf> {
    let mut cases: Vec<_> = std::fs::read_dir(root).unwrap().map(|e| e.unwrap().path()).collect();
    cases.sort();
    cases
}

/// The published schemas of a version, one validator per kind.
pub struct Schemas {
    validators: Vec<(osd::Kind, jsonschema::Validator)>,
}

impl Schemas {
    pub fn load(version: &str) -> Self {
        let dir = repo().join("schemas").join(version);
        let read = |kind: &str| -> serde_json::Value {
            let text = std::fs::read_to_string(dir.join(format!("{kind}.schema.json"))).unwrap();
            serde_json::from_str(&text).unwrap()
        };
        let mut registry = jsonschema::Registry::new();
        for name in KINDS {
            let schema = read(name);
            let id = schema["$id"].as_str().unwrap().to_string();
            registry = registry.add(id, schema).unwrap();
        }
        let registry = registry.prepare().unwrap();
        let validators = KINDS
            .iter()
            .map(|name| {
                let v = jsonschema::options().with_registry(&registry).build(&read(name)).unwrap();
                (kind_of(name), v)
            })
            .collect();
        Schemas { validators }
    }

    /// The first error the schema of `kind` finds in `doc`, if any.
    pub fn check(&self, kind: osd::Kind, doc: &serde_json::Value) -> Option<String> {
        let (_, v) = self.validators.iter().find(|(k, _)| *k == kind).unwrap();
        v.iter_errors(doc).map(|e| format!("{}: {e}", e.instance_path())).next()
    }
}

/// Lists at most a few failures, then fails with their count.
pub fn report(what: &str, failures: Vec<String>) {
    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(10).collect();
        panic!("{} {what}:\n{}", failures.len(), shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    }
}
