# Conformance cases

What every OSD reader returns for a given input, version by version. A
reader in any language runs them; `osd` runs them with `cargo test -p osd`.

## Reading (`<version>/read/<case>/`)

- `descriptor.json`: the text the case reads, kept as written.
- `case.json`: `read`, the kind of descriptor it is (`device`, `profile`,
  `sound`, `fingerprint`, `firmware` or, from 0.6, `assembly`); `_about`, what the case shows; then
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

A result case may add `settings` (`{transfer, index, message, settings}`: the
settings `sound_settings` gives for that sound, each `{control, param_path,
value}`, in the order of the controls) and `comparisons` (`{transfer, a, b,
result}`: `a` and `b` each `{index, message}`, `result` one `{control,
param_path, outcome}` per setting, `outcome` being `equal`, `differ` or
`absent`).

A result case of a firmware (0.6) may add `comparisons` (`{a, b, order}`: two base versions and
their order, `less`, `equal` or `greater`) and `fits` (`{target, version, fits}`: whether the
version fits the target of that index in `targets`).

A reading case that expects a result reads a descriptor valid against its
version's schemas; one that expects a fault reads a descriptor with exactly
one schema error, at the pointer the fault names. The optional
`schema_pointer` gives the place where the schema signals that error when it
differs from the `pointer` of the `fault` (a dependency between fields, which
the schema signals at the object that holds them, or at the field it constrains);
absent, the reader of the case compares with the `pointer` of the `fault`. A case with
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

## Coding of bytes (`0.6/coding/<case>/`)

- `case.json`: `_about` (citing the norm's example it is built on), `coding` (`mcoded7` or
  `msb_first_7in8`), `bytes` (the bytes to code) and `coded` (the bytes sent), each in upper-case
  hexadecimal separated by spaces (empty when there is no byte). A reader codes `bytes` in 7 bits
  with `coding` and returns `coded`: `mcoded7` is the Mcoded7 of M2-103-UM v1.2, 6.1.7 (a group of
  seven bytes is sent as the byte of its sign bits, the first byte's sign at bit 6, then the seven
  low parts; a short last group is coded the same way); `msb_first_7in8` puts the sign of byte k at
  bit k.

## Judgement of needs (`0.6/judge_needs/<case>/`)

- `sound.json` and `assembly.json`: a sound and an assembly, each valid against its schema.
- `case.json`: `judge_needs: true`, `_about`, and `result`: one `{module, verdict}` per entry of the
  sound's `needs`, in order, the verdict being `satisfied`, `not_satisfied` or `unknown`. Only the
  assembly is read, the version it pins for the module; no catalogue is.

## Resolution (`<version>/resolve/<case>/`)

- `files/<maker>/<name>.json`: the catalogue the case reads, each file by
  its catalogue id (`acme/synth`, `acme/_profile.keys`).
- `case.json`: `resolve`, the id of the driver resolved, or (0.6)
  `resolve_assembly`, the id of the assembly whose MIDI is resolved (the
  stock firmware's, then each module's own routes in the listed order);
  `_about`, what the
  case shows; then one of:
  - `result`: the resolved device, equal to the reader's, key order aside;
    with `inherited`, the capabilities the merge requires that the driver's
    own text does not (those a profile or a `midi_of` target brings); absent
    means none;
  - `fault`: the refusal, by its `kind` (the faults of
    `osd/docs/INTERFACE.md`), the `file` it is in and the JSON `pointer` of
    the link refused;
  - `schema_refused` (beside `fault`): the files the schema of the version
    refuses on purpose, a list of `{file, pointer}`, each file by its catalogue
    id. It is the reverse of a reading case's `schema_accepts`: there the
    schema accepts what the reader refuses; here the schema itself refuses the
    file, with exactly one error, at that `pointer`. Only the pass whose kind
    selects the file judges it (`acme/synth` is a device: the profile pass never
    sees it); every other file the pass selects must be valid.

A resolution case that expects a result reads files valid against their
version's schemas; a file without `$schema` is a 0.4 file, judged by the 0.4
schemas. A case may hold files of two versions: the older is converted to the
newer's version before they are joined, and the result states that version.

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
