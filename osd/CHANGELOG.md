# Changelog

## Unreleased

- `MidiType::template` gives the OSC address and arguments the specification fixes for a `midi_in` type (prefix not included), so an engine no longer keeps its own copy of the table; the 0.4 → 0.5 conversion reads it.

- `resolve` accepts `$schema` in a `midi_of` file and keeps the target's in the result: `convert` writes it in every 0.5 file, so a converted `midi_of` driver was refused. A key that belongs to the target is still refused; a `$schema` that is not a 0.5 schema URL is refused at `/$schema`.

- Reads the closed vocabularies of 0.5 (`decode`, `expects`, `selects`, `_sources[].type`, `_sources[].license`) and refuses a word outside a list unless it is a declared extension `x-<registrant>-<term>`; `convert` maps the makers' words of 0.4 by the tables of `README.md`.

- Reads OSD 0.4 strictly: device, maker profile, sound, fingerprint and firmware descriptors into their types, documentary fields included, and writes them back unchanged.
- Reads an integer by its value, as JSON Schema does: `1.0` and `1e2` are the integers 1 and 100, written back as `1` and `100`; a number with a fraction where an integer is expected stays refused.
- Resolves a driver's shared MIDI (`midi_of`) and maker profiles through a `Files` source, as osc-bridge 0.16 does; refusals are typed faults with their file and JSON pointer.
