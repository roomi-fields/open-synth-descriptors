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

## Pauses, reply time and banks that cannot be written (0.6)

- `programs.delay_ms` (0.5) is the one pause, in milliseconds, between the messages of a recall. A device that needs different pauses after different messages takes the longest: that need (the Yamaha TG33 in KnobKraft Orm's `messageSendDelays`) is known and waits for a second device before the format carries a pause per message or a sequence of steps.
- `programs.dump.reply_timeout_ms` (0.6, integer from 0) is how long the device may take to answer the request of the dump, in milliseconds (KnobKraft Orm's `replyTimeoutMs`). A reply time shorter than `programs.delay_ms` is refused at `/programs/dump/reply_timeout_ms` (`BadValue`): the schema cannot compare two fields, the reader does. This is a rule of meaning of OSD, not a fact KnobKraft Orm states; a missing pause counts as zero.
- `programs.banks[].writable` (0.6, boolean) says whether the bank can be written; absent, it can, so a descriptor that says nothing reads as before. `writable: false` is KnobKraft Orm's `isROM`, with the meaning turned to the positive word. A tool never offers to write into a bank that is not writable. OSD describes no sequence that stores a sound yet, so no reader refuses a bank as the target of one.
- `osd::convert` from 0.5 invents none of these fields.

## A virtual instrument and its fidelity (0.6)

A virtual instrument is an entry like another: a software device (`device.kind` `software`) that stands for a device of the catalogue says so, and says how true it is to it. The decision is the owner's (2026-11-17). Two optional fields of `device`:

- `stands_for` is the catalogue address `maker/device` of the device it stands for. The target may itself be a software device (an instrument may stand for another).
- `fidelity` is one of three words, each a claim about the method, not a grade:
  - `faithful`: the real firmware, emulated;
  - `modelled`: the behaviour of the device reproduced from study, without its firmware;
  - `principle`: a design that follows the device's principle without claiming its behaviour.

`stands_for` may be given without `fidelity`; `fidelity` without `stands_for` is refused at `/device/fidelity`. Both need `kind` to be `software`: on a hardware device, or a device that gives no `kind`, they are refused at the field; the schema says it with a dependency on `kind`. A `stands_for` that is not of the form `maker/device`, and a `fidelity` outside the three words, are refused at the field (`BadValue`). `read` checks the form alone; resolving a device whose `stands_for` names an address absent from the catalogue is refused at `/device/stands_for`. A descriptor of 0.4 or 0.5 refuses both as unknown fields; `osd::convert` from 0.5 adds neither.

A plugin or a DAW of its own, which stands for no other device, gives neither field and reads as in 0.5.

MAME states for each driver how well the emulation works (good, imperfect, preliminary), a grade of the result. OSD departs from it: the fidelity states how true a virtual instrument is by its method, a fact its maker knows and a reader can check against the sources, where a grade of the result is a judgement that changes with each listener. The departure keeps the vocabulary of the claim honest, and the descriptor still names the device it shows, as a Hugging Face Space names the model it demonstrates.

## Resolution

Resolving a driver gives the device as its firmware runs it: the MIDI it shares from another driver (`midi_of`), then its blocks completed from maker profiles. A resolver takes a catalogue id and a source of files by id: a driver `maker/name` is the file `<maker>/<name>.json`, the profile `maker` is `<maker>/_profile` and the profile `maker/family` is `<maker>/_profile.<family>`. It returns the device, the ids of the files it read (the driver first) and its capabilities, or a fault of the kinds named above, in the file where it arises, at the pointer of the faulty place. The rules below hold for every version read; the cases that hold them are those of `conformance/0.4/resolve/`, `conformance/0.5/resolve/` and `conformance/0.6/resolve/`. Each rule names its cases as `<version>/<case>`.

What a `midi_of` file may add and declare follows the version of its own text, read before any conversion (R3, R4, R7, R8, R12). In 0.6 it may add parameters (its `params` holds `entries` only), add scripted commands and declare `capabilities`. In 0.5 it may not: a 0.5 file with `midi_of` that declares `capabilities` is refused at `/capabilities` and one that gives `get_frame` in `params` is refused at `/params/get_frame`, even over a 0.6 target; a 0.6 file keeps the 0.6 rules over a 0.4 or 0.5 target, which is converted to 0.6 and then joined.

### R1 — A driver that names no other file is read as it is written

Cases: `0.5/resolve-plain-driver`

### R2 — A file with `midi_of` keeps its identity keys and takes every other key from its target

The identity keys are the keys of `device` that say which firmware the file is: `name`, `vendor`, `revision`, `osc_prefix` and `kind`; one the file does not write is absent from the result. Its `_` keys (notes for people) and its credit are its own. The target is a driver of the same maker and device, and it may itself name another.

Cases: `0.4/midi-of-own-keys`, `0.5/midi-of-identity-kept`, `0.5/midi-of-own-keys`

### R3 — A file with `midi_of` adds route lists to its target's

The lists are `commands` and `custom_commands`; the file's routes come after the target's. From 0.6 a file that adds a script (`custom_commands` with `script`) declares `capabilities: ["lua"]` itself, even when its target already brings Lua: without it, or with `[]`, it is refused at `/capabilities`.

Cases: `0.5/midi-of-routes-appended`, `0.6/midi-of-custom-commands-with-lua`, `0.6/midi-of-script-without-lua-refused`, `0.6/midi-of-script-with-empty-capabilities-refused`

### R4 — A file with `midi_of` adds entries to the route tables of its target

The tables are `params`, `cc_params` and `triggers`; only their `entries` are added, after the target's. A table the target does not have is taken from the file as it is. In 0.6 the `params` of a file with `midi_of` needs `entries` only, its message shape (`get_frame`, `set_frame`, `value_type`) coming from the target; a `params` taken as it is must then hold the three keys, and without one the resolution is refused as `Missing` at `/params/<key>`, in the file. A file without `midi_of` keeps the three keys required.

Cases: `0.5/midi-of-routes-appended`, `0.5/midi-of-table-absent-in-target`, `0.6/midi-of-params-entries-added`, `0.6/midi-of-params-without-target-frames-refused`

### R5 — At the same OSC address the route of the file wins

The target's route at that address is dropped and the file's comes after the target's other routes.

Cases: `0.5/midi-of-own-route-wins`

### R6 — A file with `midi_of` replaces the blocks `identity` and `programs` whole

The block of the file replaces its target's, not key by key: these are how this firmware is recognised and what its program changes recall.

Cases: `0.5/midi-of-own-block-replaces`

### R7 — A file with `midi_of` adds routes only and a key that is its target's is refused

Any key other than `$schema`, `device`, `midi_of`, a `_` key, a route list, a route table and the two replaceable blocks is refused as `UnknownField` at that key; from 0.6 `capabilities` is one of the keys it may hold, and in 0.5 it is refused at `/capabilities`.

Cases: `0.4/midi-of-target-keys-refused`, `0.5/midi-of-target-keys-refused`, `0.6/midi-of-key-outside-the-list-refused`, `0.6/midi-of-0-5-file-on-0-6-target-refused`

### R8 — Where the target has the route table the file adds entries only

Another key of that table is refused as `UnknownField` at that key; from 0.6 this holds for `get_frame`, `set_frame` and `value_type` too, which the file takes from its target and may not give (`/params/<key>`).

Cases: `0.5/midi-of-table-extra-key`, `0.6/midi-of-params-target-key-refused`

### R9 — A route of the file has an OSC address

An entry without `osc` is refused as `Missing`, at its `osc`.

Cases: `0.5/midi-of-route-without-osc`

### R10 — A file writes each OSC address once in its own routes

A second route at an address already written in the same list or table is refused as `BadValue`, at its `osc`.

Cases: `0.5/midi-of-route-twice`

### R11 — The credit of the target is kept beside the file's own

The `_sources` of the file stay its own; the `_sources` of the targets followed, and those they took from in turn, are credited in `_midi_of_sources`.

Cases: `0.4/midi-of-own-keys`, `0.5/midi-of-credit`

### R12 — The capabilities a driver declares are kept as declared

They were checked on its own text; the merge keeps them, not its target's or a profile's. A capability that only the merge requires (a profile or a target that brings Lua) is inherited: reported, not demanded. From 0.6 a file with `midi_of` may declare `capabilities`, judged on its own text like any file: the list is the one its fields require, so a file that adds a script declares `["lua"]` and one that adds none declares `[]` or nothing, else `BadValue` at `/capabilities`. The resolved `capabilities` are what the merged device requires, so those of both files, and `inherited` is what only the target brings. In 0.5 a file with `midi_of` declares none.

Cases: `0.5/midi-of-capabilities-declared-kept`, `0.5/profile-brings-lua`, `0.6/midi-of-capabilities-declared-over-target-lua`, `0.6/midi-of-lua-without-script-refused`, `0.6/midi-of-0-6-file-on-0-5-target`

### R13 — The links of `midi_of` are followed, the target first

The target is followed to its own `midi_of` before the file is joined to it, so a chain is joined from its end toward the file that was asked for.

Cases: `0.4/midi-of-depth-8`, `0.5/midi-of-chain`

### R14 — A `midi_of` is a catalogue id of two parts

It is a string `<maker>/<file>` without `.json`, each part non-empty and not starting with a dot; anything else is refused as `BadValue` at `/midi_of`.

Cases: `0.5/midi-of-id-malformed`, `0.5/midi-of-not-string`

### R15 — A `midi_of` names a driver of the maker of the file

Another maker's driver is refused as `BadValue` at `/midi_of`.

Cases: `0.5/midi-of-other-maker`

### R16 — A `midi_of` names a driver of the same device

A target whose `device.name` differs from the file's, when the file writes one, is refused as `BadValue` at `/midi_of`.

Cases: `0.5/midi-of-other-device`

### R17 — A `midi_of` whose target the catalogue does not hold is refused

The fault is `LinkMissing`, at `/midi_of`.

Cases: `0.4/midi-of-missing-refused`, `0.5/midi-of-target-missing`

### R18 — At most 8 `midi_of` links are followed from one file

A ninth is refused as `LinkTooDeep` at `/midi_of`, in the file that writes it.

Cases: `0.4/midi-of-depth-8`, `0.4/midi-of-depth-9-refused`, `0.5/midi-of-too-deep`

### R19 — Links that come back to a file already followed are refused

The fault is `LinkCycle` at `/midi_of`, in the file that writes the link back.

Cases: `0.4/midi-of-cycle-refused`, `0.5/midi-of-cycle`

### R20 — Two versions are joined in the more recent one

The older text is converted forward to the version of the newer one before the two are joined, so the document states the more recent version, each link being compared with its own target; a file of 0.5 keeps its `$schema` as the result's. A conversion that fails refuses the resolution in the file it fails in, with the fault of the conversion.

Cases: `0.5/midi-of-older-target-converted`, `0.5/midi-of-newer-target-converts-the-file`, `0.5/midi-of-chain-older-targets-converted`, `0.5/midi-of-older-target-credit-kept`, `0.5/midi-of-older-target-not-convertible-refused`, `0.5/midi-of-schema-not-0-5-refused`

### R21 — The profiles of the blocks are completed after the `midi_of` links

Once the links are followed, each of the blocks `identity` and `programs` that names a `profile` is completed from it.

Cases: `0.4/profile-own-keys-win`, `0.5/profile-own-keys-win`

### R22 — A block naming a profile is the profile's block completed with the block's own keys

The block's own keys stand in place of the profile's, and the block names the profile it wrote. The profile of a maker is the file `<maker>/_profile`, the profile of a family the file `<maker>/_profile.<family>`.

Cases: `0.4/profile-own-keys-win`, `0.5/profile-own-keys-win`, `0.5/profile-family-file`

### R23 — A key a block sets to null drops the profile's key

Cases: `0.4/profile-null-removes-key`, `0.5/profile-null-drops-key`

### R24 — A profile may itself name a profile and the nearest keys win

A family profile completes its own block from the profile it names, before the driver's block is completed from it.

Cases: `0.4/profile-of-a-family`, `0.5/profile-chain`

### R25 — A profile is a profile of the maker of the file

Another maker's profile is refused as `BadValue` at `/<block>/profile`.

Cases: `0.5/profile-other-maker`

### R26 — A profile is a string `<maker>` or `<maker>/<family>`

A value that is no string, or an id of three parts, is refused as `BadValue` at `/<block>/profile`.

Cases: `0.5/profile-id-malformed`, `0.5/profile-not-string`

### R27 — A profile that the catalogue does not hold is refused

The fault is `LinkMissing` at `/<block>/profile`.

Cases: `0.4/profile-missing-refused`, `0.5/profile-file-missing`

### R28 — A profile without the block named is refused

The fault is `LinkMissing` at `/<block>/profile`.

Cases: `0.4/profile-without-the-block-refused`, `0.5/profile-block-missing`

### R29 — At most 8 profile links are followed from one block

A ninth is refused as `LinkTooDeep` at `/<block>/profile`, in the file that writes it.

Cases: `0.4/profile-depth-8`, `0.4/profile-depth-9-refused`, `0.5/profile-too-deep`

### R30 — Profiles that name each other in a circle are refused

The fault is `LinkCycle` at `/<block>/profile`, in the file that writes the link back.

Cases: `0.4/profile-cycle-refused`, `0.5/profile-cycle`

### R31 — Each file is read strictly before it is merged

A target or a profile with a field its version does not define is refused by its reading, in its own file; a text that is not JSON is refused as `Syntax` at the root.

Cases: `0.5/target-file-read-refused`, `0.5/syntax-refused`

### R32 — A driver that the catalogue does not hold is refused

The fault is `LinkMissing` at the root of the file asked for.

Cases: `0.5/driver-missing`

## The guard

- A checksum per published schema file, recorded at publication; CI fails on any change.
- The examples and the inputs of every conformance case that expects a result validate against their version's schemas; a reading case that expects a fault has exactly one schema error, at the pointer it names.
- The `osd` reader passes every conformance case of every version it reads.

## Consumers

- `osd` (this repository): schemas, conformance cases
- osc-bridge, the SyntHub service and tools: schemas, through `osd`
- third-party readers and code editors (through SchemaStore): schemas, addresses, conformance cases
