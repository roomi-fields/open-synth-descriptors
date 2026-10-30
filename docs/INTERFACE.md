# OSD specification — interface

The specification publishes three things: the schemas of each version, their permanent addresses, and the conformance cases. Any reader, in any language, needs only these.

## Overview

| Published | Form | Where |
|---|---|---|
| schemas | JSON Schema 2020-12, one file per kind: `device`, `profile`, `sound`, `fingerprint`, `firmware` | `schemas/<version>/<kind>.schema.json` |
| addresses | from 0.5, `https://w3id.org/osd/<version>/<kind>.schema.json`, redirecting to the schemas served by this repository's GitHub Pages; 0.1 to 0.4 at `https://synthhub.roomi-fields.com/osd/<version>/` | the `$id` of each schema |
| conformance cases | one folder per case: input descriptors, linked files, and the expected result or fault | `conformance/<version>/` |
| changes | what changed from the previous version, and how to convert | `README.md`, section Versions |

A descriptor names its version by `$schema`. A reader states the versions it reads.

## The faults

A conformance case expects either a result or one of the faults the `osd` interface names (`UnknownField`, `BadValue`, `Missing`, `UnreadVersion`, `LinkMissing`, `LinkCycle`, `LinkTooDeep`, `NotConvertible`); another reader maps them to its own.

## Frames and expressions (0.5)

From 0.5, structure is never written in a string. A message (a frame) is a list of elements, at eight places: `commands[].frame`, `params.get_frame`, `params.set_frame`, `replies[].match_frame`, `identity.request`, `identity.reply`, `programs.before[]` and `programs.dump.request`. `sysex.header` and `sysex.footer` stay lists of bytes. An element is a byte (0 to 255) or an object of one key of its nature, with the keys that qualify it; an object of two natures, of an unknown key or of a value outside the written lists is refused.

| Element | Meaning |
|---|---|
| `{"arg": name}` | the command's argument `name`, one 7-bit byte |
| `{"arg": name, "encoding": e}` | `e` is `u14_msb`, `u14_lsb`, `s32`, `ascii` or `bytes` |
| `{"arg": name, "add": n}` | the argument plus `n` (0 to 127), wrapping on 7 bits: `(v + n) & 0x7F` |
| `{"device_id": true}` | the device id: the rule `identity.device_id`, with its default value |
| `{"identity": "family"}`, `"member"` | all the bytes of the model; with `"index": n`, the byte `n` alone |
| `{"checksum": a, "from": n}` | `a` is `twos7` (Roland), `sum7` (Waldorf) or `xor7` (Fractal), on the bytes written from offset `n` (`F0` is 0), up to the checksum; an optional `"length": l` bounds the bytes summed |
| `"any"` | any byte; only in `identity.reply` and `replies[].match_frame` |
| `{"capture": name}` | the byte read and kept under `name`; only where a message is recognised |

A `commands[].pre[].expr` is a tree: an integer from 0 to 127, `{"arg": name}`, or an object of one operation: `add`, `sub`, `mul`, `gt`, `lt`, `eq`, `ne` (each a list of two expressions) or `if` (a list of three: the condition, the value when it is not 0, the value otherwise).

The value is a byte of MIDI data, so every result is bounded to 0..127: `add`, `sub` and `mul` clamp their result to 0..127 (they never refuse), `gt`, `lt`, `eq` and `ne` give 0 or 1, and a literal outside 0..127 is refused when the descriptor is read. This departs from JSON Logic, whose arithmetic is unbounded and whose `if` is the same list of three: the engine that runs these expressions (osc-bridge) writes MIDI data bytes, which 7 bits hold.

`osd::convert` turns each 0.4 string form into its structure. `{checksum}` without an algorithm and `{function}` have none: they are conversion faults (`NotConvertible`) at the address of the element, and no text is returned.

## The guard

- A checksum per published schema file, recorded at publication; CI fails on any change.
- The examples and the inputs of every conformance case that expects a result validate against their version's schemas; a reading case that expects a fault has exactly one schema error, at the pointer it names.
- The `osd` reader passes every conformance case of every version it reads.

## Consumers

- `osd` (this repository): schemas, conformance cases
- osc-bridge, the SyntHub service and tools: schemas, through `osd`
- third-party readers and code editors (through SchemaStore): schemas, addresses, conformance cases
