# Conformance cases

What every OSD reader returns for a given input, version by version. A
reader in any language runs them; `osd` runs them with `cargo test -p osd`.

## Resolution (`<version>/resolve/<case>/`)

- `files/<maker>/<name>.json`: the catalogue the case reads, each file by
  its catalogue id (`acme/synth`, `acme/_profile.keys`).
- `case.json`: `resolve`, the id of the driver resolved; `_about`, what the
  case shows; then one of:
  - `result`: the resolved device, equal to the reader's, key order aside;
  - `fault`: the refusal, by its `kind` (the faults of
    `osd/docs/INTERFACE.md`), the `file` it is in and the JSON `pointer` of
    the link refused.

A maker profile `acme` is the file `acme/_profile`, and `acme/keys` the
file `acme/_profile.keys`.
