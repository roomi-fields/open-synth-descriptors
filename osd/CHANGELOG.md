# Changelog

## Unreleased

- Reads the default value of a control (0.6): an optional integer `default` on a CC, NRPN or RPN entry and on a param entry, the raw value on an initialised sound, in the space of `range` (14 bits for a `cc` + `cc_lsb` pair, `range` as it is for `orientation: centered`). A default outside `range`, bounds included, or that is not an integer is refused at its pointer; without `range` it is compared to nothing; 0.4 and 0.5 refuse it as `UnknownField`. `convert_to` from 0.5 invents none. `osd-formats` carries the pencilresearch/midi CSV's `cc_default_value` and `nrpn_default_value` through it, both ways.

- Reads the role of a control (0.6) as a path: an optional object `role {parts: [{part, index?}…], setting}` on a CC, NRPN or RPN entry and on a param entry, one part or more and one setting, each a term of the vocabulary state the descriptor names in the new top-level `role_vocabulary`, or an extension `x-<registrant>-<term>`. `roles` gives each role as a `RoleEntry` (pointer, `RoleMeaning::Known` or `RoleMeaning::NoKnownRole`, its `paramPath` of MIDI-CI Property Exchange M2-117-UM, the role) and `role_from_param_path` reads a `paramPath` back to the same role. A role without `role_vocabulary`, a state the reader does not hold, an empty list of parts, the flat form `{part, index, setting}`, a term of the wrong type, a first part that does not open a path, a part that does not stand under the one before it, a setting that does not apply to the last part, a term neither known nor an extension and a role anywhere but on a control are refused at their pointer. `convert_to` from 0.5 adds none.

- Holds the states of the vocabulary of roles (`VOCABULARY_STATES`, `vocabulary_state`, `schemas/vocabulary/<state>/`; state 1 covers the defined controllers of MIDI 1.0 and the names of M2-104-UM table 22 that set a sound, and the parts and settings the standard leaves to each maker, each with the two devices of two makers that share it) and reads one with `read_vocabulary`: a name or a maker's word present twice, two terms of one `paramPath` segment, a term without a one-sentence definition, without a `source` or a `shared_by`, or a word without its origin, a `shared_by` of fewer than two makers, a `setting` without `applies_to`, a `stands_under` naming no part, and a term that composes a path of parts and a setting the state already holds are refused at the term. `Vocabulary::check_successor` checks that a state only adds. `resolve` refuses a `midi_of` file that names another state than its target's.

- Reads the 0.6 draft of the format and converts to it: `convert_to(text, kind, Version::V0_6)` carries a 0.4 or 0.5 descriptor to it, forward only. `convert` keeps writing the latest published version, 0.5, named by the new `PUBLISHED`; `osd convert --to 0.6` asks for the draft. A 0.6 device refuses a repeated `capabilities` entry, which 0.5 still reads once.

- `declared_kind` gives the kind that the `$schema` of a text declares (`None` when the text is not JSON, has no `$schema`, or one that is not an accepted address of a version read), so a command or a tool no longer keeps its own table of the addresses.

- `MidiType::template` gives the OSC address and arguments the specification fixes for a `midi_in` type (prefix not included), so an engine no longer keeps its own copy of the table; the 0.4 → 0.5 conversion reads it.

- `resolve` accepts `$schema` in a `midi_of` file and keeps the target's in the result: `convert` writes it in every 0.5 file, so a converted `midi_of` driver was refused. A key that belongs to the target is still refused; a `$schema` that is not a 0.5 schema URL is refused at `/$schema`.

- Reads the closed vocabularies of 0.5 (`decode`, `expects`, `selects`, `_sources[].type`, `_sources[].license`) and refuses a word outside a list unless it is a declared extension `x-<registrant>-<term>`; `convert` maps the makers' words of 0.4 by the tables of `README.md`.

- Reads OSD 0.4 strictly: device, maker profile, sound, fingerprint and firmware descriptors into their types, documentary fields included, and writes them back unchanged.
- Reads an integer by its value, as JSON Schema does: `1.0` and `1e2` are the integers 1 and 100, written back as `1` and `100`; a number with a fraction where an integer is expected stays refused.
- Resolves a driver's shared MIDI (`midi_of`) and maker profiles through a `Files` source, as osc-bridge 0.16 does; refusals are typed faults with their file and JSON pointer.
