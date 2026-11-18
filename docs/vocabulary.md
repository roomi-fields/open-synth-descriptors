# The vocabulary of roles

A control may say what it does in a word every device shares, so that a person or an agent asks "open the filter" without knowing the device, and what is written for one device works on another. That word is the control's **role**. The roles are a vocabulary written in OSD as data, under `schemas/vocabulary/<state>/`.

## What a role is

A role is a JSON object on a control (a CC, NRPN or RPN entry, a `params` entry), never a string:

```json
{ "osc": "/filter/cutoff", "cc": 74, "role": { "part": "filter", "setting": "cutoff" } }
{ "osc": "/fx2/depth", "cc": 92, "role": { "part": "effect", "index": 2, "setting": "depth" } }
```

- `part`: the part of the instrument (`filter`, `output`, `effect`…), a term of type `part`.
- `index`: its number, from 1, when the instrument has several (`effect` 2). Optional.
- `setting`: what is set on the part (`cutoff`, `pan`, `depth`…), a term of type `setting`.

A role is optional: a control without one stays valid, and a 0.5 descriptor converted to 0.6 by `osd::convert_to` receives none.

The descriptor names, once and in a top-level field `role_vocabulary` (`"1"`), the state of the vocabulary every role was written with. A descriptor with a role and no `role_vocabulary` is refused, and so is a state the reader does not hold. A maker profile carries no control, so no `role_vocabulary`; a driver and the profile or the `midi_of` target it links are each read against their own file, and the resolution rewrites no role and merges no state (a `midi_of` file that names another state than its target's is refused at `/role_vocabulary`).

## What a reader checks

`osd` checks each role against the named state:

- a term of `part` is a term of type `part`, a term of `setting` a term of type `setting`, otherwise the role is refused at the faulty field;
- the setting applies to the part (`applies_to` of the setting names the part): `filter` with `pan` is refused, at the role;
- a role of known terms is valid without being listed, and reads as a **known role**;
- a term the state lacks is written as an extension `x-<registrant>-<term>` (the form and the registry of 0.5: `^x-[a-z0-9]+-[a-z0-9-]+$`, `schemas/extensions.md`; `<registrant>` is the name the author chooses). A role with an extension is valid and reads as **no known role**; a term that is neither known nor an extension is refused at the term.

Nothing in a reader is written per role: `osd::roles` resolves a descriptor's roles against the state's data.

## A term

Each term of a state has:

| Field | Says |
|---|---|
| `name` | its one name |
| `type` | `part` or `setting` |
| `definition` | its definition, in one sentence |
| `applies_to` | for a setting, the parts (terms of type `part` of the same state) it applies to |
| `words` | the makers' words that mean it, each `{word, origin}`: the origin is the specification (the MIDI 1.0 table, Property Exchange) or the maker that uses the word |
| `source` | a non-empty list of `{controller, name}`, the entries of the MIDI 1.0 table of defined controllers whose composition uses the term |
| `replaced_by` | the term of the same state that replaces it, when a later decision replaces it |

A function is **composed**, not listed once per case. MIDI 1.0 "Brightness" (controller 74) is `filter` + `cutoff`: each cites controller 74 in its `source`, and "Brightness" is a maker's word of `cutoff`, not a term. Likewise controller 5 "Portamento Time" is `voice` + `portamento-time`, and controller 10 "Pan" is `output` + `pan`.

## The rule of entry

A role enters the vocabulary when it has a specific function that the existing roles cannot name, alone or put together. The open system must not let the same function in twice.

`osd` and the schema of the vocabulary hold what a machine can hold:

- a name present twice, a maker's word present under two terms (in any case), or a word that is the name of another term, is refused at the second one;
- a term without a definition of one sentence, a type, a source or the origin of each word is refused;
- a term whose name or word is the composition of a part and a setting the state already holds (`brightness` next to `filter` and `cutoff`), or a setting whose controllers a part and another setting already compose, is refused at the term;
- an `applies_to` or a `replaced_by` that names no term, or an `applies_to` that names a term that is not a part, is refused.

The schema of the vocabulary states the shapes (a source, an origin, a definition of one sentence, `applies_to` for a setting); comparing two terms is `osd`'s, since a JSON Schema cannot.

## How a term is proposed

A pull request adds one term to the next state (a new folder `schemas/vocabulary/<n+1>/`). It brings:

- the name, the one-sentence definition, the type, `applies_to` for a setting;
- the makers' words, each with its origin;
- the `source` (the controllers of the MIDI 1.0 table; a name of MIDI 2.0 Property Exchange enters as a maker's word with its origin, once read at its piece in the specification);
- the test that no existing term or maker's word says the same function: the term written with the existing terms (a part and a setting) does not say it, and no word of another term is its word.

`cargo test -p osd` checks the whole state; the pull request is not mergeable until it passes.

## How a state is published

A state is numbered (`1`, `2`…) and is a folder: `terms.json` and its schema `vocabulary.schema.json`. A published state is frozen: its checksum is recorded in `schemas/SHA256SUMS`, CI fails on any change, and the next state is a new folder. The vocabulary grows without a new version of the format. Publishing a state is the owner's decision; `osd` holds each published state (`osd::VOCABULARY_STATES`) and reads it for as long as it exists.

A state only adds (`Vocabulary::check_successor`): a term of a state stays in the following ones, with its name and type; a term a later state replaces keeps its place and carries `replaced_by`.

## From an extension to a term

When a state later holds the term an extension stood for, the author rewrites the role with the term and names that state in `role_vocabulary`. `osd` migrates nothing: a role is read as written, against the state named.
