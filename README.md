# Open Synth Descriptors (OSD)

Open, structured descriptions of what MIDI and OSC leave unsaid about
synthesizers, drum machines, controllers and music software: how a device
speaks, what a shared sound is and who made it, what a firmware changes.
Plain JSON, checked by JSON Schema, free for anyone to use (CC0).

OSD complements the MIDI standards; it does not replace them. MIDI says how
messages travel and, with MIDI 2.0 Property Exchange, lets a recent device
describe some of itself. OSD covers the rest: the thousands of devices that
never will, the maker-specific SysEx that holds most of a synth, and the
sounds and firmwares people share.

**Status: 0.4, draft.** It describes the format the
[SynthHub](https://synthhub.roomi-fields.com/) catalogue is written in:
every device of it is a valid OSD device descriptor, checked by its tests.

## What is described

| Descriptor | What it says | Schema |
|---|---|---|
| Device | Identity and how to recognise it (identify reply, port names, USB identity, what it sends by itself); controls (CC, 14-bit CC, NRPN, RPN, SysEx parameters); named messages and checksums; programs and banks; drum and sample notes; what it sends back; where each piece of data comes from and how far it was checked | [`device.schema.json`](schemas/0.4/device.schema.json) |
| Maker profile | What a maker's or a family's devices share (identify request, program scheme), extended by each device | [`profile.schema.json`](schemas/0.4/profile.schema.json) |
| Sound | A shared sound, kit, pattern, project, bank or MIDI clip: its data (a `.syx`, a `.mid`, or settings by control address), device and firmware, author, licence, origin, what it derives from, how to play it | [`sound.schema.json`](schemas/0.4/sound.schema.json) |
| Fingerprint | What a detection observed of a device plugged in (port names, replies, USB identity, the controls its knobs sent) and the model the person confirmed: recognises devices that do not identify themselves, and teaches the next detection | [`fingerprint.schema.json`](schemas/0.4/fingerprint.schema.json) |
| Firmware | A firmware, mod or script: device, author, repository, licence, what it changes to the MIDI, how to install and revert it, where its users meet. It points to the author's release, never carries it | [`firmware.schema.json`](schemas/0.4/firmware.schema.json) |

Examples: [a device](examples/devices/test_synth.json),
[a sound](examples/warm-pad.sound.json),
[a firmware](examples/tribe-0.3-beta.firmware.json),
[a fingerprint](examples/minilogue.fingerprint.json),
[a device being taught](examples/unknown-device.fingerprint.json).

## Where MIDI ends and OSD starts

| Need | In the MIDI standards | In OSD |
|---|---|---|
| A device describing itself | MIDI 2.0 Property Exchange, for devices that implement it | Device descriptor, for any device, written from its manual, its maker's tools or tests |
| Maker-specific SysEx: parameters, dumps, checksums | Not described | `params`, `commands`, frame placeholders |
| Patch, note and control names | MIDNAM | Device descriptor; exported to MIDNAM |
| OSC addresses of a device | OSCQuery | Device descriptor; exported to OSCQuery |
| A sound's author, licence, origin, device | Nothing | Sound descriptor |
| Firmwares and what they change | Nothing | Firmware descriptor, `midi_of` |
| Recognising a device that does not identify itself | Nothing (the Identity Request only, when answered) | Fingerprint descriptor, `identity` of the device |
| Where data comes from, how far it was checked | Nothing | `_sources` of every descriptor |

Where a MIDI standard already names something, OSD aims to use the same
name so that exporting stays a translation, not a guess.

## Reading a descriptor

- One device file per device and source; a firmware with its own MIDI is
  its own file, and `midi_of` names the file whose MIDI it answers to
  unchanged.
- Fields starting with `_` are notes for people (`_sources`, `_usage`,
  `_limitations`); programs never need them.
- Addresses read alike from one device to the next (`/filter/cutoff`,
  `/program/recall`): the descriptor turns each into the device's own CC,
  NRPN, note or SysEx.
- Fields marked `x-osd-extension` in the schemas (a few Lua expressions and
  scripts) belong to [osc-bridge](https://github.com/roomi-fields/osc-bridge),
  the program that runs these descriptors; other programs may ignore them.

## Checking a file

```sh
pip install jsonschema
python tools/validate.py device path/to/devices/
python tools/validate.py sound my-sound.json
OSD_VERSION=0.1 python tools/validate.py device old-file.json   # an older version
```

In Rust, the reference reader [`osd`](osd/) reads a descriptor strictly
into typed values and resolves a device's shared MIDI and maker profiles
(`osd::read`, `osd::resolve`); the cases every reader must pass are in
[`conformance/`](conformance/).

Every version's schemas have a permanent address,
`https://w3id.org/osd/<version>/<name>.schema.json` (for example
[`https://w3id.org/osd/0.4/device.schema.json`](https://w3id.org/osd/0.4/device.schema.json)),
kept by the [W3C Permanent Identifier Community Group](https://w3id.org/)
and redirecting to the schemas served from this repository. From 0.5, it is
each schema's `$id`; versions 0.1 to 0.4 keep the `$id` they were published
with, `https://synthhub.roomi-fields.com/osd/<version>/<name>.schema.json`,
still served.

## Closed vocabularies (0.5)

Five values of a 0.5 descriptor take their word from a closed list, each a set
of defined terms. A value outside a list is refused (`BadValue`), except as a
declared extension (below). `osd::convert` maps the makers' words of 0.4 to
the terms; a word outside the tables is `NotConvertible`, at the pointer of the
value. The converter neither guesses nor normalises the case. This section is
the one place where the lists and the tables are written.

### Extension

A value outside a list (not `decode`, which has no extension) is valid as
`x-<registrant>-<term>`, matching `^x-[a-z0-9]+-[a-z0-9-]+$`, on the model of
MIDI-CI's `X-` manufacturer resources (M2-103 ResourceList), with a registrant
added. A reader accepts an unknown extension and treats it as the default of
its field: `selects`, a plain recall; `expects`, `continuous`;
`_sources[].type`, the lowest reliability. [`schemas/extensions.md`](schemas/extensions.md)
lists extensions, each with a one-sentence definition and a stable URL; being
listed is required to become a core term in a later version, and the converter
then maps it. This departs from RFC 6648, which retired the `X-` prefix for
protocol parameters: a closed list that a registry grows by version, with no
central authority to ask, needs a prefix that tells a reader at once that a
word is not core, and the registrant says whose it is.

### `identity.firmware.decode`

Closed, no extension. `json` needs `field`; `lua` needs `script`.

| Term | Meaning |
|---|---|
| `raw_bytes` | the bytes at `at`, as they are |
| `json` | the reply from `at` to F7 is a JSON object; the version is the text of `field` |
| `korg_minor_major` | two bytes, minor then major |
| `yamaha_tenths` | one byte, in tenths |
| `ascii` | the bytes at `at` are the version's characters |
| `lua` | the script in `script` reads the version from the reply |

0.4 to 0.5: a term is kept; any other word is `NotConvertible`.

### `expects`

| Term | Meaning |
|---|---|
| `continuous` | a value over a range (the default) |
| `switch` | two states, held |
| `momentary` | on while held |
| `trigger` | an event with no value |
| `clock` | a clock or tempo input |
| `discrete` | one of a few named values |

0.4 to 0.5: a term is kept; any other word is `NotConvertible`.

### `selects` and the sound `kind`

`programs.selects`, `banks[].selects` and the `kind` of a sound share one list.

| Term | Meaning |
|---|---|
| `sound` | a single playable sound |
| `kit` | a set of sounds mapped across the keys, typically percussion |
| `sequence` | stored note or event data |
| `project` | a whole working state with its sounds and sequences |
| `bank` | a group of the above selectable as a unit |
| `clip` | a launchable looped fragment |
| `performance` | several sounds assigned to parts or channels, playable together |
| `scene` | a stored snapshot of the device's state, recalled as a whole |
| `setlist` | an ordered list of slots, each pointing to another recallable item |

0.4 to 0.5, by the maker's word:

| Maker's word | Term |
|---|---|
| `preset`, `patch`, `program`, `voice`, `tone`, `instrument` | `sound` |
| `pattern`, `song` | `sequence` |
| `drum kit`, `rhythm set`, `drum voice` | `kit` |
| `studio set`, `combination`, `multi`, `setup`, `mixing voice`, `performance` | `performance` |
| `snapshot` | `scene` |
| `playlist slot`, `set list slot`, `live set` | `setlist` |
| `sampler element`, `track`, `canvas`, `session`, `synth slot`, `drum slot` | the extension `x-osd-<word>`, spaces as hyphens (`x-osd-sampler-element`, `x-osd-track`, `x-osd-canvas`, `x-osd-session`, `x-osd-synth-slot`, `x-osd-drum-slot`) |

A term is kept; any other word (`tone preset`, `Preset`) is `NotConvertible`.
Makers with no published terms stay out of the table.

### `_sources[].type`

| Term | Meaning |
|---|---|
| `hardware-verified` | tested on the device itself |
| `hardware-observed` | read from what the device sent, not tested by sending |
| `software-verified` | tested against the running software |
| `vendor-doc` | the maker's documentation |
| `vendor-osc-api` | the maker's published OSC API |
| `vendor-self-described` | the device or software describes itself |
| `third-party-osc` | an OSC description by someone other than the maker |
| `community-re` | reverse engineering by the community |
| `electra-preset` | an Electra One preset |
| `pencilresearch` | the pencilresearch MIDI controller maps |
| `knobkraft` | the KnobKraft orm adaptations |
| `midnam` | a MIDNAM file |
| `contributed` | contributed by a person or an agent, unchecked |
| `test-fixture` | made for a test |

0.4 to 0.5: a term is kept; `vendor` becomes `vendor-doc`; `agent-contributed`
becomes `contributed`; `hardware-verified-partial` becomes `hardware-verified`
(the source's `notes` say what was tested); any other word is `NotConvertible`.

A `license` (a source's, a sound's, a firmware's) is an SPDX id, `public-domain`,
`NOASSERTION` or `NONE`; the reader checks the id's shape (a capital first, then
letters, digits, `.`, `+`, `-`), not the SPDX registry. `NOASSERTION` and `NONE`
are SPDX's two special values, taken with their SPDX meaning (SPDX
specification, https://spdx.github.io/spdx-spec/): `NOASSERTION` means the source states no licence; nothing is asserted;
`NONE` means the source declares that it grants no licence. The
values are written in SPDX's canonical case; other spellings are refused
(`none`, `None`, `Noassertion`, `Public-Domain`), a departure from SPDX, which
compares its ids without regard to case: one value has one spelling. 0.4 to 0.5: `none` becomes
`NONE`; another admitted value is kept; any other text is `NotConvertible`.

## Versions

A version number names a fixed set of schemas: a published version never
changes, a later one lives beside it. `schemas/SHA256SUMS` records the
SHA-256 of every published schema file, and `cargo test -p osd` fails,
naming the file, when one changes, disappears, or joins a published version.
A new version is recorded when it is published, from the `schemas` folder:

```sh
sha256sum <version>/*.schema.json >> SHA256SUMS
```

- **0.4**: the firmware rule of an `identity` may read a reply carrying
  JSON (`decode: json`, the version in `field`), as the Electra One answers.
- **0.3**: a fingerprint may name a device no descriptor describes yet
  (`device.vendor`, `device.model`) and the controls the person named while
  teaching it (`names`): the start of a new device descriptor.
- **0.2**: the device's `identity` adds `usb` (USB vendor and product ids)
  and `sends` (active sensing, clock, how it ends a note); new fingerprint
  descriptor. Every 0.1 device descriptor is a valid 0.2 one.
- **0.1**: device, maker profile, sound, firmware.

Until 1.0, a new version may rename or remove fields; each says what changed
and how to convert.

## Contributing

Open an issue or a pull request: a device the format cannot describe, a
field a tool needs, a name the MIDI standards already use. Proposals come
with an example descriptor that shows the need.

## Licence

The schemas, examples and this text are dedicated to the public domain
under [CC0 1.0](LICENSE): use them in any product, open or closed, without
asking. The `osd` reader is licensed MIT or Apache-2.0, at your option
([`osd/LICENSE-MIT`](osd/LICENSE-MIT), [`osd/LICENSE-APACHE`](osd/LICENSE-APACHE)). Published by [SynthHub](https://synthhub.roomi-fields.com/).
