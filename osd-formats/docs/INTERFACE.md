# osd-formats — interface

**Langage** : cargo

`osd-formats` offers one import and one export per format. Every operation returns its result with the list of what it could not carry, or a typed fault; none performs I/O. The formats come in this order: the pencilresearch/midi CSV, its triggers CSV, Electra One presets, MIDNAM, OSCQuery, the MIDI 2.0 controller list and the Deluge device definition (export only), KnobKraft last.

## Overview

| Export | Shape | Returns | Refuses |
|---|---|---|---|
| `import` | `fn import(text: &str, format: Format, source: &Source) -> Result<Translated<Device>, Fault>` | the OSD device, carrying `source` in `sources` as given except its `imported_at`, for which 0.7 has no field: its value is written unchanged as the note `_imported_at` of that source (`_imported_at_2`, `_imported_at_3`, then the next number, when the key is taken), and is no loss, and the list of what the source could not say; no import guesses a `provider` from the format: the source carries the one the caller gives, or none | a source with none of `url`, `author`, `attribution` or with no `license` (`SourceIncomplete`); a source the 0.7 reader of `osd` refuses in the descriptor written, such as a `type` named after a product (`pencilresearch`) or outside the list without the form of an extension, a `license` or a `provider` that is not well formed, or an empty `id` (`SourceRefused`, at `source/<field>`, its rule carrying the reader's; no descriptor is returned, and a source without credit or licence stays `SourceIncomplete`); a file not valid in its format (`InvalidFile`, with its line, or its JSON pointer for a preset or a namespace; a MIDNAM file is judged by its DTD); a document the reader of `osd` refuses (no descriptor is returned): `electra-one`, `midi-csv` and `midi-triggers-csv` read back their result, `source` included, before returning it (`InvalidFile` at the empty place, `rule` carrying the rule of `osd` and its pointer); `midnam`, `oscquery` and `midi2-controllers` read the document before `source` is added to it, which the reading of the source covers (`SourceRefused`), and say `InvalidFile` at the line of the MIDNAM `MasterDeviceNames`, at the pointer of the namespace's device container, or at the pointer of the controller list, `rule` carrying the rule of `osd` and its pointer |
| `export` | `fn export(device: &Resolved, format: Format) -> Result<Translated<String>, Fault>` | the text in the other format and the list of what it cannot carry; of a device before 0.7, a message not fully known (an NRPN or an RPN given by one number, a `cc_lsb` without its `cc`) is not written, no number being added to complete it, and is a loss at its field, as is an `nrpn_lsb_range` no way takes | a device without a `sources` entry (`NoSources`) |
| `credit_to_carry` | `fn credit_to_carry(device: &Resolved, lost: &[Loss]) -> Vec<String>` | one line per source of the device (author, attribution, url, licence, kind, then its `provider` when it names one, as the credit text of the Deluge, KnobKraft, MIDNAM and OSCQuery exports does) when `lost` lists `/sources`, else none | — |
| `Format` | closed enum of the formats read or written (`MidiCsv`, named `midi-csv`; `MidiTriggersCsv`, named `midi-triggers-csv`; `ElectraOne`, named `electra-one`; `Midnam`, named `midnam`; `OscQuery`, named `oscquery`; `Midi2Controllers`, named `midi2-controllers`; `Deluge`, named `deluge`; `Knobkraft`, named `knobkraft`), with `ALL`, `name`, `from_name`, `media_type`, `extension` and `imports_into` | — | — |
| `Format::media_type` | `fn media_type(self) -> &'static str` | the media type of the file the format writes | — |
| `Format::extension` | `fn extension(self) -> &'static str` | the usual extension of that file, without a dot | — |
| `Format::imports_into` | `fn imports_into(self) -> bool` | whether a file of the format can be added to a descriptor by `import_into`: `MidiCsv` and `MidiTriggersCsv` | — |
| `import_into` | `fn import_into(base: Device, text: &str, format: Format, source: &Source) -> Result<Translated<Device>, Fault>` | `base` with the entries of `text` added after those it holds in `cc_params` (`MidiCsv`) or `triggers` (`MidiTriggersCsv`), addresses unique over the whole descriptor, `source` added after its `sources`, with the `provider` the caller gives or none, as given except its `imported_at`, for which 0.7 has no field: its value is written unchanged as the note `_imported_at` of that source (`_imported_at_2`, `_imported_at_3`, then the next number, when the key is taken), and is no loss; a base before 0.7 is first converted to 0.7 by the converter chain of `osd` (`write`, `convert_to`, `read`; an entry with no message is kept with `ways: []`), the taken addresses, claimed messages and identity being those of the converted base: its blocks come out as the chain gives them, what the chain removes is not listed as a loss, and no source it already held takes the `provider` of the file added; a base of 0.7 keeps its other blocks untouched; the identity of `base` wins, a different `vendor` or `name` in the file is a loss at its line and column | as `import`; a source whose `id` is already that of a source of the base (`SourceRefused` at `source/id`); a format that adds nothing to a base, that is without `imports_into` (`InvalidFile` at `format`, its rule naming the format refused and each format that can); a base that does not convert to 0.7 (`NotConvertible`: `place` is the JSON pointer `osd` gives, in the base given, `rule` says the base does not convert and carries the rule of `osd`); a base of 0.7 the reader of `osd` refuses, read back before the file is added (`NotConvertible` too, same `place`, `rule` saying the base does not read in 0.7 and carrying the rule of `osd`) |
| `Translated<T>` | `value`, `lost`: the `Loss` of each element not carried, in the order of the source | — | — |
| `Loss` | `place`: `line N` or `line N, column <name>` in a CSV, a JSON pointer in a descriptor; `reason` | — | — |
| `Fault` | typed fault: its `kind` (`FaultKind`: `SourceIncomplete`, `SourceRefused`, `InvalidFile`, `NoSources`, `NoSoundTransfers`, `NotConvertible`), its `place`, its `rule` | — | — |

The fate of each column of the pencilresearch/midi CSV is in [midi-csv.md](midi-csv.md), and of its triggers CSV in [midi-triggers-csv.md](midi-triggers-csv.md). The fate of each kind of message of an Electra One preset, and the numbers its export lays out by, with the piece of the maker's page for each, are in [electra-one.md](electra-one.md), with what a SysEx template becomes (checksum, value byte, losses) and what the former SyntHub importer kept; the import writes into the source the `sha256` of the preset text, the `parameter_numbers` of its commands, and keeps the `revision` the caller passes in its `Source`; the `imported_at` it passes is kept as the note `_imported_at` of the source, 0.7 having no field for it. The fate of each element and attribute of the MIDNAM DTD, with its piece, is in [midnam.md](midnam.md); the fate of each attribute of an OSCQuery node and key of its range, with the piece of the proposal, is in [oscquery.md](oscquery.md); the fate of each property of tables 6 and 12 of the MIDI 2.0 controller resources (M2-117-UM v1.0), with its piece, is in [midi2-controllers.md](midi2-controllers.md); the fate of each tag and attribute the Deluge reader accepts for a device definition, with its piece, is in [deluge.md](deluge.md). The Deluge is exported only: its import is refused (`InvalidFile`). The fate of each function of the interface of a KnobKraft Orm adaptation, with its piece, and what of an adaptation is data of a device and what is code only its host runs, are in [knobkraft.md](knobkraft.md). The export of `knobkraft` refuses a device without `sources` (`NoSources`) and a device without `sound_transfers` (`NoSoundTransfers`, at `/sound_transfers`); its import is refused (`InvalidFile`), and nothing of an adaptation is read, parsed or run. The skeleton states its own licence, `# SPDX-License-Identifier: AGPL-3.0-only` (the licence of KnobKraft Orm, piece in knobkraft.md), before the credit of the sources; a source whose licence is not allowed under the AGPL-3.0, or not recognised, gets a warning line under its own line and a loss at `/sources/N/license`; the export warns and never refuses.

## Media type and extension

An export is one file: one type and one extension per format; two formats may share an extension.

| Format | Type | Extension |
|---|---|---|
| `MidiCsv`, `MidiTriggersCsv` | `text/csv` (IANA, RFC 4180) | `csv` |
| `ElectraOne` | `application/json` (IANA, RFC 8259) | `epr` |
| `Midnam` | `application/xml` (IANA, RFC 7303) | `midnam` |
| `OscQuery`, `Midi2Controllers` | `application/json` (IANA, RFC 8259) | `json` |
| `Deluge` | `application/xml` (IANA, RFC 7303) | `xml` |
| `Knobkraft` | `text/x-python` | `py` |

Pieces and departures:

- `epr`: the maker's repository keeps its presets under this extension (electraone/firmware-mk1, `tools/compat/presets/compat.epr`; jorisroling/bitwig-electra-one, `Bitwig Control.epr`). The maker's documentation page names no extension.
- `midnam`: the extension of the MIDINameDocument ([midnam.md](midnam.md)).
- `application/xml` for MIDNAM: a departure, the MIDI Association declares no media type and the document is XML.
- `text/x-python`: a departure from IANA, which registers no type for Python; the usual one, that of Python's standard `mimetypes` module.

## The guard

- A test reads every tag and attribute the Deluge reader accepts for `ccLabels` and needs its line in deluge.md, with its fate and its piece; the export is read back with an XML parser, in ascending CC order, one attribute per CC.
- A test compares the text of the KnobKraft export to the text expected, function by function, and never parses nor runs it; knobkraft.md needs a line per function, with its fate and its piece. The skeleton passes the generic test suite of KnobKraft only by a night test outside this crate.
- A test refuses a format of `Format::ALL` without its page `docs/<name>.md` or without a test file `tests/it/<module>.rs` or `tests/it/<module>_*.rs` that `tests/it/main.rs` declares and that holds a test.
- A test of this crate, and one of `osd-cli`, refuses a format's name, module or variant in a source outside that format's module and its line in the list.
- A round-trip test per format: a descriptor exported then imported keeps everything the format can carry, and the list of losses names the rest.
- Every export is checked against its format's own rules: the header and the cell count of each row.
- A test reads every column of the CSV's header and needs its line in midi-csv.md; the same for the triggers CSV and midi-triggers-csv.md.
- A test reads every kind of message the maker documents for a preset and needs its line in electra-one.md, with its fate and the piece of the maker's page; the export is checked against the rules written from that page (version, message types, pages, control sets, pots, 432 controls, 32 devices).
- A test reads every element and attribute the MIDNAM DTD files declare and needs its line in the table of the verifier of an export and in midnam.md, with its fate and its piece; an altered MIDNAM (an unknown element, a missing attribute, a child out of content) is refused, and each export of the conformance cases passes. A night test runs `xmllint --dtdvalid` on the exports when the program is present.
- A test reads every attribute of an OSCQuery node and every key of a range and needs its line in oscquery.md; the export is checked against the rules written from the proposal.
- A test reads every property of tables 6 and 12 of M2-117-UM v1.0 and needs its line in midi2-controllers.md, with its fate and its piece; a list imported then exported gives back the same entries.

## Consumers

- `osd-cli` (this repository): `import`, `import_into`, `export`, `credit_to_carry`, `Format`, `Fault`, `FaultKind`, `Loss`
- osc-bridge (the application): `export`, `Format`
- the SyntHub service and tools: `import`, `export`, `Format`

<!-- généré: début -->
<!-- généré: fin -->
