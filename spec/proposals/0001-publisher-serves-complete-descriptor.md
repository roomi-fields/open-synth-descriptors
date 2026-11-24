# 0001 — Whoever publishes a catalogue should serve the complete descriptor

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

This draft decides nothing: the content of 0.7 is the owner's.

## What it changes

The text of 0.7: a recommendation (SHOULD) addressed to a publisher of a catalogue.

## Why

A client that looks for how a device speaks needs nothing of how the catalogue is built (`docs/ARCHITECTURE.md`, decision 4: « Inheritance is for authors; a client is served the complete descriptor »). A descriptor with links forces the client to resolve them.

## The change

A publisher of a catalogue SHOULD serve, for each device, the complete descriptor: the descriptor of the same version, without links, as `osd::complete_descriptor` derives it (`osd/docs/INTERFACE.md`).

## What a reference does

Not yet read at its source.

## Objections

None yet.
