# osd-cli — interface

**Langage** : cargo

`osd-cli` publishes one program, `osd`. Each word is one operation of `osd` on files.

## Overview

| Word | Shape | Returns | Refuses |
|---|---|---|---|
| `validate` | `osd validate [--kind <kind>] <file>…` | one line per file: `<file>: ok`, or on the error output each fault as `<file>: <Kind> at "<pointer>": <rule>`; the kind is read from the file's `$schema`, `--kind` gives it for a file without one; exit 0 or 1 | a path that is not a readable file, a file without `$schema` and without `--kind`, an unknown kind (exit 2) |
| `convert` | `osd convert [--kind <kind>] [--to <version>] <file> [-o <file>] [--force]` | the descriptor in the latest published version (0.5), or in the version `--to` names (the 0.6 draft is written only so), on the standard output or in the file named; the kind as for `validate` | a text `osd::convert_to` refuses, a target older than the file or 0.4 included (exit 1); an unknown `--to` version, an existing output file without `--force`, a file without kind (exit 2) |
| `init` | `osd init <kind> [-o <file>] [--force]` | a template of the kind (`device`, `profile`, `sound`, `fingerprint`, `firmware`), each place to fill a string that starts with `FILL: ` | an unknown kind (exit 2); an existing output file without `--force` (exit 2) |
| `import` | `osd import <format> <file> [--source-url <url>] [--source-author <name>] --license <spdx> [-o <file>] [--force]` | the OSD descriptor read from a file in another format (`midi-csv`, `electra-one`, `midnam`, `oscquery`), carrying the source given, on the standard output or in the file named; on the error output the list of what the source could not say, one line per loss | a file not valid in its format (exit 1); an unknown format, a missing source or license, a missing file, an existing output file without `--force` (exit 2) |
| `export` | `osd export <format> <file> [-o <file>] [--force]` | the descriptor file, resolved alone, written in the format (`midi-csv`, `electra-one`, `midnam`, `oscquery`), on the standard output or in the file named; on the error output the list of what the format cannot carry, one line per loss | a descriptor `osd` refuses, a link no file answers, a descriptor without `_sources` (exit 1); an unknown format, an existing output file without `--force` (exit 2) |

## The guard

- A test per word runs the program on the conformance cases: what `validate` says of each case equals what the case expects.
- The template of each kind, its marked places filled from the kind's example, validates against the latest schema.

## Consumers

- descriptor authors, people and agents
- partners' tools and CI, to check their files

<!-- généré: début -->
<!-- généré: fin -->
