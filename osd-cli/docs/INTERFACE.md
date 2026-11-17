# osd-cli — interface

**Langage** : cargo

`osd-cli` publishes one program, `osd`. Each word is one operation of `osd` on files.

## Overview

| Word | Shape | Returns | Refuses |
|---|---|---|---|
| `validate` | `osd validate <file>…` | one line per file: right, or each fault with its JSON pointer and its rule; exit 0 or 1 | a path that is not a file (exit 2) |
| `convert` | `osd convert <file> [-o <file>]` | the descriptor in the latest version, on the standard output or in the file named | a text `osd::convert` refuses (exit 1); an existing output file without `--force` (exit 2) |
| `init` | `osd init <kind> [-o <file>]` | a template of the kind (`device`, `profile`, `sound`, `fingerprint`, `firmware`) | an unknown kind (exit 2); an existing output file without `--force` (exit 2) |
| `import` | `osd import <format> <file> [-o <file>]` | the OSD descriptor read from a file in another format, and on the error output the list of what the source could not say | a format `osd-formats` does not read, a file not valid in its format (exit 1) |
| `export` | `osd export <format> <file> [-o <file>]` | the descriptor written in another format, and the list of what that format cannot carry | a format `osd-formats` does not write, a descriptor `osd` refuses (exit 1) |

## The guard

- A test per word runs the program on the conformance cases: what `validate` says of each case equals what the case expects.
- The template of each kind, its marked places filled from the kind's example, validates against the latest schema.

## Consumers

- descriptor authors, people and agents
- partners' tools and CI, to check their files

<!-- généré: début -->
<!-- généré: fin -->
