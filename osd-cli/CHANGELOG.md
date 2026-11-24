# Changelog

The changes of the `osd` command. The changes of the reader it is built on are in `osd/CHANGELOG.md`, where the entries of the command before this file was opened also are.

## Unreleased

## 0.6.0 - 2026-11-23

- `osd convert` writes 0.6 by default (it wrote 0.5), and `--to <version>` names another; the templates of `osd init` are 0.6.

- The new word `osd resolve <id> --catalogue <folder>` writes on the standard output the complete descriptor of a device, without links: the device a client reads alone (`osd::complete_descriptor`). A fault of the resolution is reported as `osd validate` reports it, with the exit code of a fault.

- The new word `osd conformance` is the adapter of the reference reader for the conformance suite: it takes one case folder and writes one JSON object, as `conformance/RUNNING.md` asks of any reader.
