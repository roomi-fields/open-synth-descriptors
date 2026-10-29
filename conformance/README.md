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

A reading case that expects a result reads a descriptor valid against its
version's schemas; one that expects a fault reads a descriptor with exactly
one schema error, at the pointer the fault names.

## Resolution (`<version>/resolve/<case>/`)

- `files/<maker>/<name>.json`: the catalogue the case reads, each file by
  its catalogue id (`acme/synth`, `acme/_profile.keys`).
- `case.json`: `resolve`, the id of the driver resolved; `_about`, what the
  case shows; then one of:
  - `result`: the resolved device, equal to the reader's, key order aside;
  - `fault`: the refusal, by its `kind` (the faults of
    `osd/docs/INTERFACE.md`), the `file` it is in and the JSON `pointer` of
    the link refused.

A resolution case that expects a result reads files valid against their
version's schemas.

A maker profile `acme` is the file `acme/_profile`, and `acme/keys` the
file `acme/_profile.keys`.
