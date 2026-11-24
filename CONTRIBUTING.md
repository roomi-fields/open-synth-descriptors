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
