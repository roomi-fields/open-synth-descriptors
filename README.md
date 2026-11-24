# Open Synth Descriptors (OSD)

[![check](https://github.com/roomi-fields/open-synth-descriptors/actions/workflows/check.yml/badge.svg)](https://github.com/roomi-fields/open-synth-descriptors/actions/workflows/check.yml)
[![licence: CC0](https://img.shields.io/badge/licence-CC0-blue)](LICENSE)
[![crate osd-cli](https://img.shields.io/crates/v/osd-cli?label=crates.io%20osd-cli)](https://crates.io/crates/osd-cli)
[![npm @roomi-fields/osd](https://img.shields.io/npm/v/@roomi-fields/osd?label=npm%20%40roomi-fields%2Fosd)](https://www.npmjs.com/package/@roomi-fields/osd)

Open, structured descriptions of what MIDI and OSC leave unsaid about
synthesizers, drum machines, controllers and music software: how a device
speaks, what a shared sound is and who made it, what a firmware changes.
Plain JSON, checked by JSON Schema, free for anyone to use (CC0). OSD
complements the MIDI standards; it does not replace them.

**Status: 0.6, published and frozen.** It is developed with the help of AI
agents, and the maintainer reviews every change ([GOVERNANCE.md](GOVERNANCE.md)).

## One descriptor

A shared sound, with its author and its licence:

```json
{
  "$schema": "https://w3id.org/osd/0.6/sound.schema.json",
  "name": "Warm pad",
  "kind": "sound",
  "device": { "descriptor": "korg/minilogue", "vendor": "KORG", "model": "minilogue" },
  "content": {
    "settings": { "/amp/attack": [90], "/amp/release": [100], "/filter/cutoff": [64] }
  },
  "author": { "name": "Example Author", "url": "https://example.org" },
  "license": "CC0-1.0",
  "category": "pad"
}
```

One example per kind of document: [`examples/0.6`](examples/0.6).

## Your door

- **Author**, you describe a device or share a sound: start with
  `osd init <kind>`, check with `osd validate`, then read
  [`spec/0.6.md`](spec/0.6.md) and [CONTRIBUTING.md](CONTRIBUTING.md).
- **Reader**, your tool reads descriptors: the Rust reader [`osd`](osd/) and
  the site [roomi-fields.github.io/open-synth-descriptors](https://roomi-fields.github.io/open-synth-descriptors/).
- **Implementer**, you write a reader in another language: the text
  [`spec/0.6.md`](spec/0.6.md) and the cases in [`conformance/`](conformance/).

## Install the command

```sh
cargo install osd-cli                 # from crates.io
npm install -g @roomi-fields/osd      # from npm
```

or download the binary of your platform from the
[releases](https://github.com/roomi-fields/open-synth-descriptors/releases)
page and check it against the `SHA256SUMS` file of the release. Then
`osd --version`. Each word of `osd` (`validate`, `convert`, `init`, `import`,
`export`): [`osd-cli/docs/INTERFACE.md`](osd-cli/docs/INTERFACE.md). How a
release is published: [`docs/RELEASE.md`](docs/RELEASE.md).

## Where to go

- The text of the format: [`spec/0.6.md`](spec/0.6.md); the schemas: [`schemas/`](schemas/).
- The reference reader and its numbers: [`reference/`](reference/).
- The site: <https://roomi-fields.github.io/open-synth-descriptors/>.
- The book of the specification: <https://roomi-fields.github.io/open-synth-descriptors/book/>.
- What each version changed: [`spec/CHANGELOG.md`](spec/CHANGELOG.md).
- Who decides, and what 1.0 awaits: [GOVERNANCE.md](GOVERNANCE.md).

## Licence

CC0 1.0 for the schemas, conformance cases, examples and documents
([`LICENSE`](LICENSE)); MIT or Apache-2.0 for the reader and the command
([`LICENSE-MIT`](LICENSE-MIT), [`LICENSE-APACHE`](LICENSE-APACHE)).
Published by [SynthHub](https://synthhub.roomi-fields.com/).
