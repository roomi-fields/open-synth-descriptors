# Running the conformance suite

How the author of another reader proves it reads OSD without our code. The cases
(`<version>/<kind>/<case>/`, see `README.md`) are the contract; this file says how a reader is run
on one and how the run is reported. Nothing here needs `osd`.

## The reader

A reader is an executable, the adapter, that takes **one case folder as its only argument**. It
reads `case.json` and the files beside it, does what the case states (read, convert, resolve,
resolve an assembly, code bytes, judge needs, read a vocabulary, match an identity against a
fingerprint, compare two product instance ids) and writes **one JSON object on
its standard output**, then exits 0.

The object holds, by the names `case.json` uses, each key the case expects:

- `result` or `fault`, and the other keys the case states (`inherited`, `settings`, `comparisons`,
  `roles`, `coded`, `identities`, and so on). A reader may write keys the case does not expect:
  they are ignored.
- A fault is `{kind, pointer}` and, for a resolve case, `file` too.
- `warnings`, which a reading case may state, is the ordered list of `{kind, pointer}` of what the
  reader reports without refusing: `PartialControl` (an entry that reaches no message) and
  `UnknownProvider` (a `provider` the table of providers does not hold).

A folder that is not a case, or a case of a kind the reader does not know, is answered with
exit code 2 and a message on the standard error, never with a result. Any other exit code than 0
is a failed case.

### The kinds of fault

The reader maps each refusal of its own to the `kind` the case expects:

- `BadValue`: a value that breaks a rule of its field.
- `Missing`: a required field is absent.
- `UnknownField`: a field the version does not have.
- `Syntax`: the text is not JSON.
- `UnreadVersion`: the version of the document is one the reader does not read.
- `NotConvertible`: an element of the previous version has no counterpart in the next.
- `LinkMissing`: a link names a file or an element that does not exist.
- `LinkCycle`: links lead back to themselves.
- `LinkTooDeep`: links chain deeper than the limit.

## The comparison

A runner compares the output to the case, key by key. Every expectation key of the case is
compared; not compared are `_about`, the kind selector (`read`, `convert`, `resolve`,
`resolve_assembly`, `coding`, `judge_needs`, `match_identity`, `same_product_instance_id`), the inputs a case states (`bytes`), `schema_pointer`,
`schema_accepts` and `schema_refused` (they judge the schema, not the reader).

- Objects are equal key order aside; arrays are equal in order; numbers are equal by value;
  strings are equal exactly.
- A fault is equal by `kind`, by `pointer` and, for a resolve case, by `file`.

A case is **passed** when every key is equal, **failed** otherwise.

## The optional cases

A case is optional when its `case.json` carries `identity_support` or `identities`, or when a
descriptor of the case holds a JSON key named `lua` at any depth: they need a Lua interpreter or
the computation of a sound's identity. `optional.json` lists them, one path under `conformance/`
per entry (`0.6/read/sound-transfer-identity`).

A runner reports a listed case as **skipped**, never as passed. A reader that skips them claims
conformance **without the optional cases**.

## The report

A runner reports, for each version of the format it ran, the version of the format, the version of
the suite and the counts, for example:

    format 0.6 suite 1.0.0: 480 passed, 0 failed, 11 skipped

A claim of conformance names the version of the format and the version of the suite, and says
« without the optional cases » when it skipped them.

## The version of the suite

The suite has its own version, in `VERSION`: one line `MAJOR.MINOR.PATCH`. `SHA256SUMS` records
the SHA-256 of every case file, one line `<sha256>  <path under conformance/>`, so a reader's
author checks they hold the suite as published.

A published version of the suite is never changed in what a reader must do:

- A case added: the minor number rises.
- A correction of a case, a change of a case, or the removal of one: the patch number rises.
- The major number rises when this contract changes.

A case file is never changed nor removed to suit a reader: that is a correction, and it is
written as such.
