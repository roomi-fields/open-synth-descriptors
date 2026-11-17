# osd-formats — interface

**Langage** : cargo

`osd-formats` offers one import and one export per format. Every operation returns its result with the list of what it could not carry, or a typed fault; none performs I/O. The formats come in this order: the pencilresearch/midi CSV, Electra One presets, MIDNAM and OSCQuery, KnobKraft last.

## Overview

| Export | Shape | Returns | Refuses |
|---|---|---|---|
| `import` | `fn import(text: &str, format: Format) -> Result<Translated<Device>, Fault>` | the OSD device and the list of what the source could not say | a file not valid in its format; a format without importer |
| `export` | `fn export(device: &Resolved, format: Format) -> Result<Translated<String>, Fault>` | the text in the other format and the list of what it cannot carry | a device whose sources forbid sharing in that format; a format without exporter |
| `Format` | closed enum of the formats read or written | — | — |
| `Translated<T>` | `value`, `lost`: each element not carried, by its JSON pointer or its place in the source, with the reason | — | — |
| `Fault` | typed fault: its kind, the place in the source, the rule | — | — |

## The guard

- A round-trip test per format: a descriptor exported then imported keeps everything the format can carry, and the list of losses names the rest.
- Every export is checked against its format's own schema or DTD.

## Consumers

- `osd-cli` (this repository): `import`, `export`, `Format`
- osc-bridge (the application): `export`, `Format`
- the SyntHub service and tools: `import`, `export`, `Format`

<!-- généré: début -->
<!-- généré: fin -->
