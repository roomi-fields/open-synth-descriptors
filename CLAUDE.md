# Open Synth Descriptors (OSD)

OSD is an open, universal format that describes synthesizers, drum machines, controllers and music software beyond what MIDI and OSC say: how a device speaks, how it is recognised, how its programs are recalled, what a shared sound is and who made it, what a firmware changes. This repository holds the specification (JSON schemas, CC0), its conformance cases, and `osd`, the reference reader (Rust, MIT or Apache-2.0). OSD stands alone: any tool reads it, and no reader is privileged.

**Owner**: Romain Peyrichou. He validates this charter, `docs/ARCHITECTURE.md`, each `CADRE.md` and `INTERFACE.md`, every new version of the format, and each ticket labelled `a-valider`; the supervisor validates the other documents. So the owner decides what the format is.

The framework's rules: @node_modules/@roomi-fields/grillhouse/charte.md

## What decides

The metaframe (`node_modules/@roomi-fields/grillhouse/METACADRE.md`), then the project's frame (`docs/ARCHITECTURE.md`, `osd/docs/CADRE.md`, `osd/docs/INTERFACE.md`), then the specification (the schemas of the newest version and its conformance cases), then the code. A published version never changes; a change of the format is a new version, decided in the architecture first. So the intent decides, and the schemas and the reader follow it.

## How we arbitrate

In this order:

1. **What does the mature reference do?** For a specification with a reference reader and a test suite: the WebAssembly specification repository and TOML with `toml-test`; for describing devices: MIDI 2.0 Property Exchange, MIDNAM and OSCQuery, whose names OSD reuses where they overlap. What they do is the default answer; a departure carries its written reason.
2. **What already exists?** A standard, an off-the-shelf tool, a proven library, a convention of the neighbouring projects is taken as is.
3. **Does the domain require it?** In this order of priority:
   - **compatibility**: a descriptor valid in one version reads in the next through its converter, and a reader states the versions it reads;
   - **one meaning per field**: structure lives in JSON structures that the schema checks, never in mini-languages inside strings;
   - **credit**: every descriptor names its sources and their licence.
   A choice that weakens them is measured and stated.
4. **All the reader's code is Rust.** So agents work in the language where they are most effective.

## What goes up to the supervisor

A question no step of the circuit settles goes up to the supervisor, with its evidence. Goes up to the owner, who alone decides it: a new version of the format, a field's meaning, a licence, anything published (a release, a schema URL, a message to another project).

## Commands

- `cargo test -p osd` (the reader, its conformance cases, the schema against the types); `cargo clippy -p osd -- -D warnings`.
- `python tools/validate.py <kind> FILE_OR_DIR…` checks descriptors against the schemas until the reader replaces it.
- The framework does not know Rust yet: `npm run verifier`, `grillhouse maj` and the night pass wait for its Rust tooling.
- Ticket prefix: `SH-`.
- Push policy: each commit goes to main and is pushed; ask before rewriting history, deleting a branch, publishing a release or a version of the format, or acting outside the repository.
