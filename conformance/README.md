# Conformance cases

What every OSD reader returns for a given input, version by version. A
reader in any language runs them; `osd` runs them with `cargo test -p osd`.

## Reading (`<version>/read/<case>/`)

- `descriptor.json`: the text the case reads, kept as written.
- `case.json`: `read`, the kind of descriptor it is (`device`, `profile`,
  `sound`, `fingerprint` or `firmware`); `_about`, what the case shows; then
  one of:
  - `result`: the descriptor written back, equal to the reader's, key order
    aside, each number written as the result writes it (`1`, not `1.0`);
  - `fault`: the refusal, by its `kind` and the JSON `pointer` of the value
    refused.

A result case of a device with `sound_transfers` (0.6) adds `identity_support`
(one `{pointer, without_lua}` per transfer: whether the reader computes the
identity without Lua) and, when it computes, `identities`
(`{transfer, index, message, identity}`: the `id` of a transfer, the index of
the sound, the message in hexadecimal and the SHA-256 expected).

A reading case that expects a result reads a descriptor valid against its
version's schemas; one that expects a fault reads a descriptor with exactly
one schema error, at the pointer the fault names. A case with
`"schema_accepts": true` is the exception: its fault is one the schema cannot
express (a declared list that differs from the derived one), so the
descriptor validates against the schema and only the reader refuses it.

## Conversion (`<version>/convert/<case>/`)

- `descriptor.json`: the text of the previous version the case converts.
- `case.json`: `convert`, the kind of descriptor; `_about`; then one of:
  - `result`: the converted descriptor, equal to the converter's, key order
    aside; it reads in the version, validates against its schemas, and
    converts to itself;
  - `fault`: the refusal, by its `kind` (`NotConvertible`) and the JSON
    `pointer` of the element of the previous version refused; no text is
    returned.

## Resolution (`<version>/resolve/<case>/`)

- `files/<maker>/<name>.json`: the catalogue the case reads, each file by
  its catalogue id (`acme/synth`, `acme/_profile.keys`).
- `case.json`: `resolve`, the id of the driver resolved; `_about`, what the
  case shows; then one of:
  - `result`: the resolved device, equal to the reader's, key order aside;
    with `inherited`, the capabilities the merge requires that the driver's
    own text does not (those a profile or a `midi_of` target brings); absent
    means none;
  - `fault`: the refusal, by its `kind` (the faults of
    `osd/docs/INTERFACE.md`), the `file` it is in and the JSON `pointer` of
    the link refused.

A resolution case that expects a result reads files valid against their
version's schemas.

A maker profile `acme` is the file `acme/_profile`, and `acme/keys` the
file `acme/_profile.keys`.

## Roles (0.6)

A role is a path: `{parts: [{part, index?}…], setting}`, one part or more and
one setting. A reading case that expects a result may state `roles`: each role
of the descriptor by the JSON `pointer` of the role, with its `meaning`, `known`
(every term is a term of the state named by `role_vocabulary`) or `none` (the
role holds an extension: no known role), and its `paramPath`, the text of the
path as the `paramPath` of MIDI-CI Property Exchange (M2-117-UM): each term as
its CamelCase segment (`attack-time` is `AttackTime`), the index of a part as a
number after it, the setting last, an extension as written
(`/Filter/Envelope/AttackTime`, `/Oscillator/2/Pitch`). A case that states none
has no role. The refusals of a role are at the part or the setting that breaks
a rule: a part that does not stand under the one before it, or cannot open a
path, at that part; a setting that does not apply to the last part, at the role.

## Vocabulary states (`0.6/vocabulary/<case>/`)

- `vocabulary.json`: the state of the vocabulary of roles the case reads.
- `case.json`: `_about`, what the case shows; then one of:
  - `result`: the state is valid; `terms` lists the names read;
  - `fault`: the refusal, by its `kind` and the JSON `pointer` of the value
    refused; with `"schema_accepts": true`, the schema of the vocabulary
    validates the state and only the reader refuses it (comparing two terms is
    not a schema's), otherwise the schema has an error at the pointer or at the
    object that holds it.
