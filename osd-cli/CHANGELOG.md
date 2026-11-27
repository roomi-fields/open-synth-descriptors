# Changelog

The changes of the `osd` command. The changes of the reader it is built on are in `osd/CHANGELOG.md`, where the entries of the command before this file was opened also are.

## Unreleased

- 0.7 (candidate): `validate` and `convert` print on the error output, for each source a maker profile, a sound, a fingerprint, a firmware or an assembly keeps in a note `_sources_<n>`, `<file>: warning at "<pointer>": the source is kept as a note and not carried as credit`; the standard output and the exit codes do not change. `osd conformance` writes it as `UncreditedSource`.

- `osd import` of a source the 0.7 reader refuses (a `--license` that is not well formed, or with `--into` a fault of the source added) writes no descriptor, prints the place `source/<field>` and the rule on the error output and exits 2.

- A control or a `params` entry with no message is no longer refused (see `osd/CHANGELOG.md`). `validate`, `convert`, `resolve`, `import` and `export` print on the error output, for each, `<file or id asked>: warning at "<pointer>": the control gives no message: it is to be completed`; the standard output and the exit codes do not change. `export midi-csv` writes its row with no CC nor NRPN and `export oscquery` its node, each naming in the losses what it cannot carry. `osd conformance` writes `warnings` for a reading case that states it.

- `osd import --into` converts a base before 0.7 to 0.7; a base that does not convert is exit 1, its fault printed under the name of the `--into` file, and no descriptor is written.

- `osd import` takes `--source-provider <id>`, who supplies the file (an identifier that is not well formed is a misuse, exit 2). `validate`, `convert`, `resolve`, `import` and `export` print on the error output one line per `provider` the table of providers does not hold; the standard output and the exit codes do not change.

- `osd import` writes a 0.7 descriptor with its credit under `sources`, and `osd export` names `/sources` in the losses it prints; the descriptors it reads before 0.7 keep `_sources` and are read as before.

- 0.7 (candidate): `osd validate`, `convert` and `resolve` take the rules on port names and the signs of MIDI 2.0 of proposal 0004, and `osd conformance` answers the cases `match_identity` and `same_product_instance_id`. The published version stays 0.6.

## 0.6.0 - 2026-11-23

- `osd convert` writes 0.6 by default (it wrote 0.5), and `--to <version>` names another; the templates of `osd init` are 0.6.

- The new word `osd resolve <id> --catalogue <folder>` writes on the standard output the complete descriptor of a device, without links: the device a client reads alone (`osd::complete_descriptor`). A fault of the resolution is reported as `osd validate` reports it, with the exit code of a fault.

- The new word `osd conformance` is the adapter of the reference reader for the conformance suite: it takes one case folder and writes one JSON object, as `conformance/RUNNING.md` asks of any reader.
