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

## Addresses, arguments and `midi_in` (0.5)

An address that takes captures is written at four places: `commands[].osc`, `forward.path` (in `commands[]` and `subscriptions[]`), `replies[].match_osc` and `replies[].emit_osc`. It is either a string without brace (an address without capture) or an array of segments, each a string without brace or `{"capture": name}`; `name` follows `[A-Za-z_][A-Za-z0-9_]*`. The address is the concatenation of its segments, never split on a slash. An array holds one to 16 segments and at least one capture (an address without capture is a string); for `commands[].osc` the first segment is a string starting with `/`. Where an address is recognised (`commands[].osc`, `replies[].match_osc`), two captures side by side are refused, as the first would take everything; where it is emitted (`forward.path`, `replies[].emit_osc`) they are read. A string with a brace is refused at the address.

The arguments of `forward.args` and `replies[].emit_args` are each a JSON literal (number, boolean, string without brace), `{"arg": name}` (the value of an argument of the command) or `{"capture": name}` (a field captured in the message recognised); any other object, or an array, is refused at the argument. `emit_osc` is the address alone (a string without space or brace, or segments); `emit_args` lists its arguments, and absent means none.

`midi_in` is the list of the MIDI messages the device sends that are reported, each of `note_on`, `note_off`, `cc`, `pitchbend`, `aftertouch`, `poly_aftertouch`, `program_change`, once at most; `[]` reports nothing. The OSC address of each is fixed by the specification, under the device's prefix; what follows the space are its OSC arguments, in this order:

| Type | Address and arguments |
|---|---|
| `note_on` | `/note/on {note} {velocity} {channel}` |
| `note_off` | `/note/off {note} {velocity} {channel}` |
| `cc` | `/cc/{num} {value} {channel}` |
| `pitchbend` | `/pitchbend {value_u14} {channel}` |
| `aftertouch` | `/aftertouch {value} {channel}` |
| `poly_aftertouch` | `/poly_aftertouch {note} {value} {channel}` |
| `program_change` | `/program_change {program} {channel}` |

`osd::convert` cuts each 0.4 address that holds `{name}` into segments and leaves one without brace as it is. In `forward.args` a `{name}` becomes `{"arg": name}`; a 0.4 `emit_osc` is cut at its first space, the address before it and each `{name}` after it a `{"capture": name}` of `emit_args`. A brace unclosed, empty, nested, stray or around anything but a name, an argument that mixes a brace with text, an argument of `emit_osc` that is not one `{name}`, and a `midi_in` template other than the canonical one of its type are conversion faults (`NotConvertible`) at the address of the element, which names the address or the template; no text is returned.

## The guard

- A checksum per published schema file, recorded at publication; CI fails on any change.
- The examples and the inputs of every conformance case that expects a result validate against their version's schemas; a reading case that expects a fault has exactly one schema error, at the pointer it names.
- The `osd` reader passes every conformance case of every version it reads.

## Consumers

- `osd` (this repository): schemas, conformance cases
- osc-bridge, the SyntHub service and tools: schemas, through `osd`
- third-party readers and code editors (through SchemaStore): schemas, addresses, conformance cases
