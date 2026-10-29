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

A conformance case expects either a result or one of the faults the `osd` interface names (`UnknownField`, `BadValue`, `Missing`, `UnreadVersion`, `LinkMissing`, `LinkCycle`, `LinkTooDeep`); another reader maps them to its own.

## The guard

- A checksum per published schema file, recorded at publication; CI fails on any change.
- The examples and the inputs of every conformance case that expects a result validate against their version's schemas; a reading case that expects a fault has exactly one schema error, at the pointer it names.
- The `osd` reader passes every conformance case of every version it reads.

## Consumers

- `osd` (this repository): schemas, conformance cases
- osc-bridge, the SyntHub service and tools: schemas, through `osd`
- third-party readers and code editors (through SchemaStore): schemas, addresses, conformance cases
