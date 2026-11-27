# osd-cli — interface

**Langage** : cargo

`osd-cli` publishes one program, `osd`. Each word is one operation of `osd` on files.

## Overview

| Word | Shape | Returns | Refuses |
|---|---|---|---|
| `validate` | `osd validate [--kind <kind>] <file>…` | one line per file: `<file>: ok`, or on the error output each fault as `<file>: <Kind> at "<pointer>": <rule>`; the kind is read from the file's `$schema`, `--kind` gives it for a file without one; exit 0 or 1 | a path that is not a readable file, a file without `$schema` and without `--kind`, an unknown kind (exit 2) |
| `convert` | `osd convert [--kind <kind>] [--to <version>] <file> [-o <file>] [--force]` | the descriptor in the latest published version (0.6), or in the version `--to` names, on the standard output or in the file named; the kind as for `validate` | a text `osd::convert_to` refuses, a target older than the file or 0.4 included (exit 1); an unknown `--to` version, an existing output file without `--force`, a file without kind (exit 2) |
| `init` | `osd init <kind> [-o <file>] [--force]` | a template of the kind (`device`, `profile`, `sound`, `fingerprint`, `firmware`), each place to fill a string that starts with `FILL: ` | an unknown kind (exit 2); an existing output file without `--force` (exit 2) |
| `import` | `osd import <format> <file> [--source-url <url>] [--source-author <name>] [--source-provider <id>] --license <spdx> [--into <descriptor>] [-o <file>] [--force]` | the OSD descriptor read from a file in another format (`midi-csv`, `midi-triggers-csv`, `electra-one`, `midnam`, `oscquery`, `midi2-controllers`; `deluge` is refused, the Deluge does not import), carrying the source given, on the standard output or in the file named; on the error output the list of what the source could not say, one line per loss; with `--into <descriptor>` (`midi-csv` and `midi-triggers-csv` only), the entries of the file are added to that descriptor, which keeps its identity and its other blocks (one before 0.7 is first converted to 0.7, its blocks as the converter gives them), and its `sources` name both files; `--source-provider <id>` says who supplies the file, the `provider` of the source added (an identifier of the table of providers), none without it | a file not valid in its format, a descriptor `--into` names that `osd` refuses or that does not convert to 0.7 (the fault under the name of that descriptor, none written), another format with `--into` (exit 1); an unknown format, a missing source or license, a `--source-provider` that is not well formed, a source the 0.7 reader refuses, such as a `--license` that is not well formed (with `--into`, a fault of the source added; the place `source/<field>` and the rule on the error output, no descriptor written), a missing file or `--into` descriptor, an existing output file without `--force` (exit 2) |
| `export` | `osd export <format> <file> [-o <file>] [--force]` | the descriptor file, resolved alone, written in the format (`midi-csv`, `midi-triggers-csv`, `electra-one`, `midnam`, `oscquery`, `midi2-controllers`, `deluge`), on the standard output or in the file named; on the error output the list of what the format cannot carry, one line per loss | a descriptor `osd` refuses, a link no file answers, a descriptor without `sources` (exit 1); an unknown format, an existing output file without `--force` (exit 2) |
| `conformance` | `osd conformance <case folder>` | the adapter of `osd` to `conformance/RUNNING.md`: for the one case folder given (`conformance/<version>/<kind>/<case>/`), one JSON object on the standard output, `result` or `fault` and the other keys the case expects (every kind of case: read, convert, resolve, resolve_assembly, coding, judge_needs, match_identity, same_product_instance_id, vocabulary); exit 0 whenever it answered | a folder that is not a case, a case of a kind it does not know, no folder or more than one (exit 2, a message on the error output, nothing on the standard output) |
| `resolve` | `osd resolve <id> --catalogue <folder>` | the complete descriptor of the device `<id>` (`maker/name`, the file `<folder>/<id>.json`) on the standard output, by `osd::resolve` then `osd::complete_descriptor`: no link left, nothing to read beside it; exit 0 | a fault of the resolution, reported on the error output as `validate` reports it (`<file>: <Kind> at "<pointer>": <rule>`, exit 1); an unknown option, a missing id, a missing `--catalogue` or a folder that does not exist (exit 2) |

## The warnings

`validate`, `convert`, `resolve`, `import` and `export` print on the error output one line per warning of `osd::warnings` on the descriptor they judge, render or write (the complete descriptor for `resolve` and `export`, the text rendered for `convert` and `import`). A control with no way (`PartialControl`), of 0.4 to 0.7, is `<file or id asked>: warning at "<pointer>": the control gives no message: it is to be completed`, one line per control, the pointer being its entry; an unknown provider (`UnknownProvider`) is `<file or id asked>: warning at "<pointer>": the provider <id> is not in the table of providers of <version of the table>`; a source not carried as credit (`UncreditedSource`: from 0.7, an entry of a note `_sources_<n>` of a maker profile, a sound, a fingerprint, a firmware or an assembly, the `made_on` of a sound included) is `<file or id asked>: warning at "<pointer>": the source is kept as a note and not carried as credit`. In a device the controls come first (`cc_params`, `params`), then `sources` and `midi_of_sources`; in another kind the providers come first, then the notes, a sound before its `made_on`. Such an entry gives, in 0.7, `ways: []` and may carry `range`, `transmit`, `recognize` and, in `cc_params`, `nrpn_lsb_range`; an export names in its losses what the format cannot carry of it. The standard output and the exit code do not change. `osd conformance` writes `warnings`, the list of `{kind, pointer}`, for a reading case that states it.

## The guard

- A test per word runs the program on the conformance cases: what `validate` says of each case equals what the case expects.
- The template of each kind, its marked places filled from the kind's example, validates against the latest schema.

## Consumers

- descriptor authors, people and agents
- partners' tools and CI, to check their files

<!-- généré: début -->
<!-- généré: fin -->
