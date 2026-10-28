# OSD specification — frame

## Role

- **R1** The specification says what an OSD descriptor is: one JSON Schema per descriptor kind (device, maker profile, sound, fingerprint, firmware) and version, and the conformance cases that give each field its meaning.

## Receives

- **R2** Changes of the format, decided in `docs/ARCHITECTURE.md` and validated by the owner, each as a new version.

## Returns

- **R3** Published versions under `schemas/<version>/`, each with its conformance cases, its converter from the previous version (in `osd`), and the list of what changed.

## Knows

- **R4** The kinds of descriptor, their fields, their meaning, and the standards whose names it reuses (MIDI 2.0 Property Exchange, MIDNAM, OSCQuery).

## Does not know

- **R5** Any reader's internals, any engine, any catalogue's contents, any service.

## Refuses

- **R6** Changing a published version: a change is a new version.
- **R7** A field whose meaning no conformance case shows.
- **R8** Structure hidden inside a string (a mini-language), and open vocabularies where a closed list can hold the values.
- **R9** Executable content not declared as a capability.

## Invariants

- **R10** Every published version stays served at its address, unchanged.
- **R11** From version 0.5, each schema's `$id` is `https://w3id.org/osd/<version>/<kind>.schema.json`.
- **R12** A descriptor valid in one version converts to the next.

## Cost

- **R13** A version's schemas validate the whole SyntHub catalogue with an off-the-shelf JSON Schema validator in under 10 s.
