# osd — interface

`osd` offers one entry per operation on a descriptor: read, resolve, convert. Every operation returns its result or a typed fault; none performs I/O. The exports below are the target of the first epic; once the crate exists, the list between the generated markers is drawn from its rustdoc comments.

## Overview

| Export | Shape | Returns | Refuses |
|---|---|---|---|
| `read` | `fn read(text: &str, kind: Kind) -> Result<Descriptor, Fault>` | the typed descriptor | unknown field, wrong type, missing field, unread version |
| `resolve` | `fn resolve(id: &str, files: &dyn Files) -> Result<Resolved, Fault>` | the device with profiles and shared MIDI applied, its sources, its capabilities | missing link, cycle, depth over 8 |
| `convert` | `fn convert(text: &str, to: Version) -> Result<String, Fault>` | the descriptor written in version `to` | a field the target cannot hold |
| `Files` | `trait Files { fn get(&self, id: &str) -> Option<String>; }` | the text of a descriptor by catalogue id | — |
| `Kind`, `Version`, `Capability` | closed enums | — | — |
| `Descriptor`, `Device`, `Profile`, `Sound`, `Fingerprint`, `Firmware` | typed values of the format | — | — |
| `Fault` | typed fault: file, JSON pointer, rule | — | — |
| `VERSIONS` | the versions the crate reads | — | — |

## The faults

- `Syntax` — the text is not JSON.
- `UnknownField` — a field the version does not define.
- `BadValue` — a value outside its type or its closed list.
- `Missing` — a required field is absent.
- `UnreadVersion` — a version the crate does not read.
- `LinkMissing` — a shared-MIDI or profile link names no file.
- `LinkCycle` — the links form a cycle.
- `LinkTooDeep` — the links go deeper than 8.
- `NotConvertible` — a field the target version cannot hold.

## The guard

- The conformance cases of every version it reads, run by `cargo test -p osd`.
- The schema its types describe, compared with the published schema of the newest version.
- `cargo-semver-checks` on each release, once the crate is published.

## Consumers

- `synth-core` (osc-bridge): `read`, `resolve`, `Files`, `Device`, `Capability`, `Fault`
- `formats` (osc-bridge): `resolve`, `Device`, `Fault`
- the SyntHub service: `read`, `resolve`, `convert`, `Files`, `Device`, `Sound`, `Fingerprint`, `Fault`
- the SyntHub tools: `read`, `resolve`, `convert`, `Files`, `Device`, `Profile`, `Fault`

<!-- généré: début -->
<!-- généré: fin -->
