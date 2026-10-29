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

Every version's schemas have a permanent address,
`https://w3id.org/osd/<version>/<name>.schema.json` (for example
[`https://w3id.org/osd/0.4/device.schema.json`](https://w3id.org/osd/0.4/device.schema.json)),
kept by the [W3C Permanent Identifier Community Group](https://w3id.org/)
and redirecting to the schemas served from this repository. From 0.5, it is
each schema's `$id`; versions 0.1 to 0.4 keep the `$id` they were published
with, `https://synthhub.roomi-fields.com/osd/<version>/<name>.schema.json`,
still served.

## Versions

A version number names a fixed set of schemas: a published version never
changes, a later one lives beside it.

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
asking. Published by [SynthHub](https://synthhub.roomi-fields.com/).
