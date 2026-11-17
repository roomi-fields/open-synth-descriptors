# osd-formats — interface

**Langage** : cargo

`osd-formats` offers one import and one export per format. Every operation returns its result with the list of what it could not carry, or a typed fault; none performs I/O. The formats come in this order: the pencilresearch/midi CSV, Electra One presets, MIDNAM and OSCQuery, KnobKraft last.

## Overview

| Export | Shape | Returns | Refuses |
|---|---|---|---|
| `import` | `fn import(text: &str, format: Format, source: &Source) -> Result<Translated<Device>, Fault>` | the OSD device, carrying `source` in `_sources` as given, and the list of what the source could not say | a source with none of `url`, `author`, `attribution` or with no `license` (`SourceIncomplete`); a file not valid in its format (`InvalidFile`, with its line) |
| `export` | `fn export(device: &Resolved, format: Format) -> Result<Translated<String>, Fault>` | the text in the other format and the list of what it cannot carry | a device without a `_sources` entry (`NoSources`) |
| `Format` | closed enum of the formats read or written (`MidiCsv`, named `midi-csv`), with `ALL`, `name` and `from_name` | — | — |
| `Translated<T>` | `value`, `lost`: the `Loss` of each element not carried, in the order of the source | — | — |
| `Loss` | `place`: `line N` or `line N, column <name>` in a CSV, a JSON pointer in a descriptor; `reason` | — | — |
| `Fault` | typed fault: its `kind` (`FaultKind`: `SourceIncomplete`, `InvalidFile`, `NoSources`), its `place`, its `rule` | — | — |

The fate of each column of the pencilresearch/midi CSV is in [midi-csv.md](midi-csv.md).

## The guard

- A round-trip test per format: a descriptor exported then imported keeps everything the format can carry, and the list of losses names the rest.
- Every export is checked against its format's own rules: the header and the cell count of each row.
- A test reads every column of the CSV's header and needs its line in midi-csv.md.

## Consumers

- `osd-cli` (this repository): `import`, `export`, `Format`, `Fault`, `FaultKind`, `Loss`
- osc-bridge (the application): `export`, `Format`
- the SyntHub service and tools: `import`, `export`, `Format`

<!-- généré: début -->
<!-- généré: fin -->
