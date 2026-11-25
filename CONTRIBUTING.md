# Contributing

[GOVERNANCE.md](GOVERNANCE.md) says who decides. Conduct follows
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Propose a change to the format

Open an [issue](https://github.com/roomi-fields/open-synth-descriptors/issues)
for a device the format cannot describe, a field a tool needs, or a name the
MIDI standards already use. Bring an example descriptor that shows the need.
The maintainer decides; an accepted change becomes a new version of the format,
since a published version never changes. For a question or an open idea, use
the [discussions](https://github.com/roomi-fields/open-synth-descriptors/discussions).

## How a term of the vocabulary of roles is proposed

A pull request adds one term to the next state (a new folder `schemas/vocabulary/<n+1>/`). It brings:

- the name, the one-sentence definition, the type, `applies_to` for a setting, `stands_under` for a part that stands under others;
- the makers' words, each with its origin;
- the `source` (a controller of the MIDI 1.0 table, a row of a published table with its document), or the `shared_by` of two devices of two makers;
- the test that no existing term or maker's word says the same function: the term written with the existing terms (parts and a setting) does not say it, and no word of another term is its word.

`cargo test -p osd` checks the whole state; the pull request is not mergeable until it passes. The vocabulary and its rule of entry are in [`spec/vocabulary/1.md`](spec/vocabulary/1.md).

## How a state of the vocabulary is published

A state is numbered (`1`, `2`…) and is a folder: `terms.json` and its schema `vocabulary.schema.json`. A published state is frozen: its checksum is recorded in `schemas/SHA256SUMS`, CI fails on any change, and the next state is a new folder. The vocabulary grows without a new version of the format. A state is published by a release of the project; `osd` holds each published state (`osd::VOCABULARY_STATES`) and reads it for as long as it exists. State 1 is under checksum: `schemas/SHA256SUMS` records its two files.

A state only adds (`Vocabulary::check_successor`): a term of a state stays in the following ones, with its name and type; a term a later state replaces keeps its place and carries `replaced_by`.

## From an extension to a term

When a state later holds the term an extension stood for, the author rewrites the role with the term and names that state in `role_vocabulary`. `osd` migrates nothing: a role is read as written, against the state named.

## Add a converter

A converter is the module of `osd-formats` that reads or writes one format of
others. To add one, or to change one, send one pull request, as
[Send a pull request](#send-a-pull-request) says, that holds all of this:

- **The module**, `osd-formats/src/<module>.rs` (and `osd-formats/src/<module>/`
  when it needs more files), where `<module>` is the name of the format with `-`
  written `_`. It declares `pub(crate) const CONTRACT: Contract`: the name of the
  format, the media type and the extension of the file it writes, its `import`,
  its `export`, and its `import_into` (`None` when a file of the format is not
  added to a descriptor). A format that is exported only declares an `import`
  that refuses every file with its reason. A helper that several formats share
  goes in a module that is no format (`importer`, `credit`, `xml`, `csv`). No
  other code names the format.
- **Its line in the list** of `osd-formats/src/lib.rs`, in `formats!`:
  `Variant => module: "documentation",`. The documentation is the public
  rustdoc of the variant: the file it reads or writes, its version, and
  "exported only" when it is.
- **Its page**, `osd-formats/docs/<name>.md`: what the format carries, what it
  loses, its source and its licence.
- **Its tests**, `osd-formats/tests/it/<module>.rs` or files named
  `<module>_*.rs` beside it, each declared in `osd-formats/tests/it/main.rs`.
  They fix the name, the media type and the extension of the format, and what it
  imports and exports.
- **The texts that list the formats and their order**:
  `osd-formats/docs/CADRE.md` (R1), `osd-formats/docs/INTERFACE.md` (the
  `Format` row and the media type table) and `osd-cli/docs/INTERFACE.md` (the
  `import` and `export` rows). The maintainer rereads the frame and the
  interface.

`cargo test -p osd-formats` and `cargo test -p osd-cli` run green; the tests of
`osd-formats` refuse a format without its page or its tests.

## Report a defect of the reader

Open an issue with the descriptor (or the smallest part of it) that shows the
defect, the command you ran, what you expected and what you got.

## Report a vulnerability

Privately, as [SECURITY.md](SECURITY.md) says.

## Send a pull request

This repository is exported from a private workshop, so a pull request is never
merged here. The maintainer reads it, imports it into the workshop, judges it
there, and exports the result. He then closes the pull request with the version
that carries the change, and the link to its commit.

If your change is imported, your name stays on the published commit when you
consent. The maintainer asks for your consent on the pull request before the
import.

Open an issue first for anything that changes the format. A pull request that
changes the reader runs `cargo test -p osd` and
`cargo clippy -p osd --all-targets -- -D warnings` green, and the pull-request
template lists the rest.

## Licence of a contribution

A contribution takes the licence of the part it changes, as the table in the
[README](README.md#licence) lists.
