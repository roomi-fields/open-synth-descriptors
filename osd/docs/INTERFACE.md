# osd — interface

`osd` offers one entry per operation on a descriptor: read, write, resolve, convert. Every operation returns its result or a typed fault; none performs I/O. The exports below are the target of the first epic: `read`, `write`, `resolve`, `convert` and the types of 0.4 and 0.5 exist. Once the framework's Rust tooling exists, the list between the generated markers is drawn from the crate's rustdoc comments.

## Overview

| Export | Shape | Returns | Refuses |
|---|---|---|---|
| `read` | `fn read(text: &str, kind: Kind) -> Result<Descriptor, Fault>` | the typed descriptor, in the version its `$schema` names (0.4 when it has none) | unknown field, wrong type, missing field, unread version |
| `write` | `fn write(d: &Descriptor) -> String` | the descriptor as JSON text: what `read` returned, key for key | — |
| `resolve` | `fn resolve(id: &str, files: &dyn Files) -> Result<Resolved, Fault>` | a `Resolved`: the device with profiles and shared MIDI applied, the ids of the files it was built from, its capabilities | each file as `read` refuses it, missing link, cycle, depth over 8, a key a `midi_of` file may not hold |
| `read_as` | `fn read_as(text: &str, kind: Kind, version: Version) -> Result<Descriptor, Fault>` | the typed descriptor, read in the version the caller names | as `read`; `Missing` at `/$schema` for a 0.5 descriptor without it; `BadValue` at `/$schema` when it names another version |
| `convert` | `fn convert(text: &str, kind: Kind) -> Result<String, Fault>` | the 0.5 text of a 0.4 descriptor: `$schema` (w3id) as first key, every 0.4 `{placeholder}` frame element and `pre` expression string turned into its structure, other values unchanged, laid out as `write`; a 0.5 text comes back as it is | as `read` for a text it cannot read; `NotConvertible`, at the JSON pointer of the 0.4 element, for `{checksum}` without an algorithm, `{function}` or a form with no structure; no text is returned then |
| `Files` | `trait Files { fn get(&self, id: &str) -> Option<String>; }` | the text of a descriptor by catalogue id (`korg/minilogue`; the profile `korg/logue` is `korg/_profile.logue`); implemented for `BTreeMap` and `HashMap` of ids to texts | — |
| `Kind`, `Version`, `Capability`, `FaultKind`, `Encoding`, `IdentityPart`, `Algorithm`, `BinOp` | closed enums (the last four are the words of a frame element and of a `pre` operation) | — | — |
| `Resolved` | `device`, `files`, `capabilities` | — | — |
| `Descriptor`, `Device`, `Profile`, `Sound`, `Fingerprint`, `Firmware`, `FrameToken`, `FramePart`, `Expr` | typed values of the format; `FrameToken` is a byte, a 0.4 string or a `FramePart` (the one-key structures and `any` of 0.5); `Expr` is a `pre` expression, a 0.4 string or a tree | — | — |
| `Fault` | typed fault: its `kind` (`FaultKind`), the `file` (the catalogue id, when met resolving; none for `read`), the JSON `pointer`, the `rule` | — | — |
| `Remarks` | the keys starting with `_` an object holds beside those its schema names, kept as they are | — | — |
| `VERSIONS` | the versions the crate reads | — | — |

## The faults

A fault's `kind` is one of:

- `Syntax` — the text is not JSON.
- `UnknownField` — a field the version does not define, or, in a `midi_of` file, a key that is its target's.
- `BadValue` — a value outside its type or its closed list, or a link that names no catalogue id, another maker's file or another device.
- `Missing` — a required field is absent.
- `UnreadVersion` — a version the crate does not read.
- `LinkMissing` — a shared-MIDI or profile link names no file, or a profile without the block named.
- `LinkCycle` — the links form a cycle.
- `LinkTooDeep` — the links go deeper than 8.
- `NotConvertible` — a field the target version cannot hold.

## The guard

- The conformance cases of every version it reads, run by `cargo test -p osd`.
- The SyntHub catalogue, when `SYNTHHUB_DEVICES` names it: every driver and profile read, written back unchanged, resolved, and valid against the schemas once written (`cargo test -p osd`); read and resolved within R16 (`cargo bench -p osd`); resolved as osc-bridge resolves it (`tools/osc-bridge-parity`, beside an osc-bridge checkout).
- The schema its types describe, compared with the published schema of the newest version.
- `cargo-semver-checks` on each release, once the crate is published.

## Consumers

- `synth-core` (osc-bridge): `read`, `resolve`, `Files`, `Device`, `Capability`, `Fault`
- `formats` (osc-bridge): `resolve`, `Device`, `Fault`
- the SyntHub service: `read`, `resolve`, `convert`, `Files`, `Device`, `Sound`, `Fingerprint`, `Fault`
- the SyntHub tools: `read`, `resolve`, `convert`, `Files`, `Device`, `Profile`, `Fault`

<!-- généré: début -->
<!-- généré: fin -->
