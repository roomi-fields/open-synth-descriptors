# osd-formats — interface

**Langage** : cargo

`osd-formats` offers one import and one export per format. Every operation returns its result with the list of what it could not carry, or a typed fault; none performs I/O. The formats come in this order: the pencilresearch/midi CSV, Electra One presets, MIDNAM and OSCQuery, KnobKraft last.

## Overview

| Export | Shape | Returns | Refuses |
|---|---|---|---|
| `import` | `fn import(text: &str, format: Format, source: &Source) -> Result<Translated<Device>, Fault>` | the OSD device, carrying `source` in `_sources` as given, and the list of what the source could not say | a source with none of `url`, `author`, `attribution` or with no `license` (`SourceIncomplete`); a file not valid in its format (`InvalidFile`, with its line, or its JSON pointer for a preset or a namespace; a MIDNAM file is judged by its DTD) |
| `export` | `fn export(device: &Resolved, format: Format) -> Result<Translated<String>, Fault>` | the text in the other format and the list of what it cannot carry | a device without a `_sources` entry (`NoSources`) |
| `credit_to_carry` | `fn credit_to_carry(device: &Resolved, lost: &[Loss]) -> Vec<String>` | one line per source of the device (author, attribution, url, licence, kind) when `lost` lists `/_sources`, else none | — |
| `Format` | closed enum of the formats read or written (`MidiCsv`, named `midi-csv`; `MidiTriggersCsv`, named `midi-triggers-csv`; `ElectraOne`, named `electra-one`; `Midnam`, named `midnam`; `OscQuery`, named `oscquery`), with `ALL`, `name` and `from_name` | — | — |
| `import_into` | `fn import_into(base: Device, text: &str, format: Format, source: &Source) -> Result<Translated<Device>, Fault>` | `base` with the entries of `text` added after those it holds in `cc_params` (`MidiCsv`) or `triggers` (`MidiTriggersCsv`), addresses unique over the whole descriptor, `source` added after its `_sources`; the identity of `base` wins, a different `vendor` or `name` in the file is a loss at its line and column; the other blocks are untouched | as `import`; any other format (`InvalidFile`) |
| `Translated<T>` | `value`, `lost`: the `Loss` of each element not carried, in the order of the source | — | — |
| `Loss` | `place`: `line N` or `line N, column <name>` in a CSV, a JSON pointer in a descriptor; `reason` | — | — |
| `Fault` | typed fault: its `kind` (`FaultKind`: `SourceIncomplete`, `InvalidFile`, `NoSources`), its `place`, its `rule` | — | — |

The fate of each column of the pencilresearch/midi CSV is in [midi-csv.md](midi-csv.md), and of its triggers CSV in [midi-triggers-csv.md](midi-triggers-csv.md). The fate of each kind of message of an Electra One preset, and the numbers its export lays out by, with the piece of the maker's page for each, are in [electra-one.md](electra-one.md). The fate of each element and attribute of the MIDNAM DTD, with its piece, is in [midnam.md](midnam.md); the fate of each attribute of an OSCQuery node and key of its range, with the piece of the proposal, is in [oscquery.md](oscquery.md).

## The guard

- A round-trip test per format: a descriptor exported then imported keeps everything the format can carry, and the list of losses names the rest.
- Every export is checked against its format's own rules: the header and the cell count of each row.
- A test reads every column of the CSV's header and needs its line in midi-csv.md; the same for the triggers CSV and midi-triggers-csv.md.
- A test reads every kind of message the maker documents for a preset and needs its line in electra-one.md, with its fate and the piece of the maker's page; the export is checked against the rules written from that page (version, message types, pages, control sets, pots, 432 controls, 32 devices).
- A test reads every element and attribute the MIDNAM DTD files declare and needs its line in the table of the verifier of an export and in midnam.md, with its fate and its piece; an altered MIDNAM (an unknown element, a missing attribute, a child out of content) is refused, and each export of the conformance cases passes. A night test runs `xmllint --dtdvalid` on the exports when the program is present.
- A test reads every attribute of an OSCQuery node and every key of a range and needs its line in oscquery.md; the export is checked against the rules written from the proposal.

## Consumers

- `osd-cli` (this repository): `import`, `import_into`, `export`, `credit_to_carry`, `Format`, `Fault`, `FaultKind`, `Loss`
- osc-bridge (the application): `export`, `Format`
- the SyntHub service and tools: `import`, `export`, `Format`

<!-- généré: début -->
<!-- généré: fin -->
