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

From 0.6, the vocabulary of roles is published beside the schemas: `schemas/vocabulary/<state>/` (its own schema and its terms), numbered states that grow without a new version of the format, and conformance cases under `conformance/0.6/vocabulary/`. A descriptor with a role names its state in `role_vocabulary`. The rule of entry and the publication of a state are in [vocabulary.md](vocabulary.md).

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

## What in a message is the sound (0.6)

From 0.6 a device may state, in `sound_transfers[]`, what in a message is the sound. It describes the structure (which bytes make the sound); how a device codes it stays the device's. One entry per form a sound travels in (alone, in a bank):

| Key | Meaning |
|---|---|
| `id` | the name of the form; a reader is asked for the identity of a sound by it |
| `carries` | `sound` (one sound) or `bank` |
| `sound_id` | the same `sound_id` in two forms declares that their data is the same sound |
| `match_frame` | the message recognised, one element per byte (the message is as long as the list); the bytes that a transport part covers are not compared |
| `elements` | the elements below |
| `count`, `stride` | a bank only, both required: the number of sounds, and the shift of each one: the sound `i` has its elements `i × stride` bytes further |

An element is `{from, length}` in the message (`F0` is 0) when its place is fixed, or `{script}`, a Lua script, when positions cannot say it; a descriptor with a script declares the `lua` capability. `data` is the sound itself and is required; `name`, `channel`, `device_number`, `header`, `checksum`, `bank_number` and `program_number` are optional, the last six being the transport parts.

`data` adds `packing`, a closed list: `none` (default), the bytes of the message are the data; `msb_first_7in8`, groups of 8 bytes, a first byte that carries the bits 7 of the seven that follow (its bit k is the bit 7 of the byte k) then those seven bytes with their bits 6 to 0, the last group possibly partial. Its piece is the KORG minilogue xd MIDI Implementation (revision 1.01, 2020.2.10), note 1 "7 bit data format conversion" (the NTS-1 note gives a partial last group: 11 bytes of MIDI for 9 of data). Any other packing is given by a script, and a word outside the list is refused.

**The identity of a sound** is the SHA-256 of its data element alone, in 64 lowercase hexadecimal digits: the data unpacked, and, when the name lies inside the data, without the name's bytes. The name, the device number, the channel, the header, the checksum, the bank and the program number never enter it, so the same sound renamed, on another channel or in a bank has one identity. This follows KnobKraft Orm, which keeps a patch's fingerprint to its data and masks the channel, and departs from it by its hash (it hashes with MD5): SHA-256 is a named hash with no known collisions, the same on every tool; the identity another tool gives (the MD5 of an adaptation) is carried beside it, not in its place.

A reader without Lua knows at load, per transfer, whether it computes the identity: it does not when `data` is a script; a script on a transport part or on the name leaves it computable, a name given by a script being taken as outside the data (a device whose name lies inside a data that positions cannot cut gives the data by a script that leaves the name out).

Refused, at the address of the fault: an element outside the message (the name included, the last sound of a bank included), an empty data, a data that overlaps the header, the checksum or a transport part, two transport parts that overlap (at the later, in the order of the table above), an element given by positions and by a script, a name that straddles the end of the data or that lies inside a packed data, a `packing` word outside the list, a script without the `lua` capability and the capability without a script. A descriptor of 0.4 or 0.5 refuses `sound_transfers` as an unknown field; `osd::convert` from 0.5 adds none.

## The identities other tools give a sound (0.6)

A 0.6 sound descriptor may hold `external_ids`, a list of the identities other tools give the same sound, so that a tool which holds it recognises it. Each entry is `{tool, by, algorithm, value}`: `tool` (required) names the tool, such as `KnobKraft Orm`; `by` names what in the tool computes the identity per device, such as the name of a KnobKraft adaptation; `algorithm` names the algorithm when it is known, such as `md5`; `value` (required) is the value as the tool writes it. The model is the external identifiers of Wikidata (an item keeps the identifier each other catalogue gives it, with that catalogue's name). For KnobKraft Orm, the piece is the guide of KnobKraft Orm (`docs/programming-guide.md`, revision 735e96c9), on `calculateFingerprint` (lines 872 and 924): each adaptation writes its own `calculateFingerprint`, which hashes the bytes it chooses with MD5 and returns 32 hexadecimal digits.

The list never replaces the rule of the identity of a sound above: the identity of OSD is the SHA-256 of its data, it is never written in this list, and the tool `OSD` (in any case) is refused. The list does not promise that two tools agree on what the data of a sound is: each value says only what that tool gives. The field is allowed for every `kind`.

A known algorithm fixes the form of the value (names read without case): `md5` 32, `sha1` 40, `sha256` 64 lower-case hexadecimal digits; a tool that writes capitals leaves `algorithm` out. An unknown or absent algorithm only requires a non-empty value. Refused, at the address of the fault: an entry without `tool` or `value` (`Missing`), an unknown key of an entry (`UnknownField`), the tool `OSD`, an empty value, a value outside the form of its algorithm, and two entries of the same computation (same `tool`, `by` and `algorithm`, each compared without case, an absent one equal to an absent one) with different values, at the later one (`BadValue`). The md5 and the sha1 of one adaptation are not the same computation. A descriptor of 0.4 or 0.5 refuses `external_ids` as an unknown field; `osd::convert` from 0.5 adds none.

## The guard

- A checksum per published schema file, recorded at publication; CI fails on any change.
- The examples and the inputs of every conformance case that expects a result validate against their version's schemas; a reading case that expects a fault has exactly one schema error, at the pointer it names.
- The `osd` reader passes every conformance case of every version it reads.

## Consumers

- `osd` (this repository): schemas, conformance cases
- osc-bridge, the SyntHub service and tools: schemas, through `osd`
- third-party readers and code editors (through SchemaStore): schemas, addresses, conformance cases
