# Changelog

## Unreleased

- Reads the role of a control (0.6): an optional object `role {part, index?, setting}` on a CC, NRPN or RPN entry and on a param entry, each of `part` and `setting` a term of the vocabulary state the descriptor names in the new top-level `role_vocabulary`, or an extension `x-<registrant>-<term>`. `roles` resolves each role to `RoleMeaning::Known` or `RoleMeaning::NoKnownRole` (an extension); a role without `role_vocabulary`, a state the reader does not hold, a term of the wrong type, a setting that does not apply to the part, a term neither known nor an extension and a role anywhere but on a control are refused at their pointer. `convert_to` from 0.5 adds none.

- Holds the states of the vocabulary of roles (`VOCABULARY_STATES`, `vocabulary_state`, `schemas/vocabulary/<state>/`; state 1 covers the defined controllers of MIDI 1.0 that set a sound) and reads one with `read_vocabulary`: a name or a maker's word present twice, a term without a one-sentence definition, a source or the origin of a word, a `setting` without `applies_to`, and a term that composes a part and a setting the state already holds are refused at the term. `Vocabulary::check_successor` checks that a state only adds. `resolve` refuses a `midi_of` file that names another state than its target's.

- Reads the 0.6 draft of the format and converts to it: `convert_to(text, kind, Version::V0_6)` carries a 0.4 or 0.5 descriptor to it, forward only. `convert` keeps writing the latest published version, 0.5, named by the new `PUBLISHED`; `osd convert --to 0.6` asks for the draft. A 0.6 device refuses a repeated `capabilities` entry, which 0.5 still reads once.

- `declared_kind` gives the kind that the `$schema` of a text declares (`None` when the text is not JSON, has no `$schema`, or one that is not an accepted address of a version read), so a command or a tool no longer keeps its own table of the addresses.

- `MidiType::template` gives the OSC address and arguments the specification fixes for a `midi_in` type (prefix not included), so an engine no longer keeps its own copy of the table; the 0.4 → 0.5 conversion reads it.

- `resolve` accepts `$schema` in a `midi_of` file and keeps the target's in the result: `convert` writes it in every 0.5 file, so a converted `midi_of` driver was refused. A key that belongs to the target is still refused; a `$schema` that is not a 0.5 schema URL is refused at `/$schema`.

- Reads the closed vocabularies of 0.5 (`decode`, `expects`, `selects`, `_sources[].type`, `_sources[].license`) and refuses a word outside a list unless it is a declared extension `x-<registrant>-<term>`; `convert` maps the makers' words of 0.4 by the tables of `README.md`.

- Reads OSD 0.4 strictly: device, maker profile, sound, fingerprint and firmware descriptors into their types, documentary fields included, and writes them back unchanged.
- Reads an integer by its value, as JSON Schema does: `1.0` and `1e2` are the integers 1 and 100, written back as `1` and `100`; a number with a fraction where an integer is expected stays refused.
- Resolves a driver's shared MIDI (`midi_of`) and maker profiles through a `Files` source, as osc-bridge 0.16 does; refusals are typed faults with their file and JSON pointer.
