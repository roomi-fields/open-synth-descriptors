# The vocabulary of roles

A control may say what it does in words every device shares, so that a person or an agent asks "shorten the attack of the filter's envelope" without knowing the device, and what is written for one device works on another. That is the control's **role**. The roles are a vocabulary written in OSD as data, under `schemas/vocabulary/<state>/`, and a role is as precise as MIDI 2.0 lets a device say it: the names are those of the standard, and the form is the path of its Property Exchange.

## What a role is

A role is a **path**: one part or more, each with its number when the instrument has several, then one setting. It is a JSON object on a control (a CC, NRPN or RPN entry, a `params` entry), never a string to cut:

```json
{ "osc": "/filter/cutoff", "cc": 74, "role": { "parts": [{ "part": "filter" }], "setting": "cutoff" } }
{ "osc": "/fenv/attack", "cc": 20, "role": { "parts": [{ "part": "filter" }, { "part": "envelope" }], "setting": "attack-time" } }
{ "osc": "/aenv/attack", "cc": 16, "role": { "parts": [{ "part": "amplifier" }, { "part": "envelope" }], "setting": "attack-time" } }
{ "osc": "/osc2/pitch", "cc": 36, "role": { "parts": [{ "part": "oscillator", "index": 2 }], "setting": "pitch" } }
```

- `parts`: the path, from the outermost part to the innermost, at least one. Each is `{part, index?}`: a term of type `part`, and its number, from 1, when the instrument has several (`oscillator` 2).
- `setting`: what is set on the last part (`cutoff`, `attack-time`, `pan`…), a term of type `setting`.

The attack of the filter's envelope and the attack of the amplifier's are two roles, which the flat form of earlier drafts (one part, an index, a setting) could not tell apart: that form is gone, and the schema of 0.6 refuses it.

A role is optional: a control without one stays valid, and a 0.5 descriptor converted to 0.6 by `osd::convert_to` receives none.

The descriptor names, once and in a top-level field `role_vocabulary` (`"1"`), the state of the vocabulary every role was written with. A descriptor with a role and no `role_vocabulary` is refused, and so is a state the reader does not hold. A maker profile carries no control, so no `role_vocabulary`; a driver and the profile or the `midi_of` target it links are each read against their own file, and the resolution rewrites no role and merges no state (a `midi_of` file that names another state than its target's is refused at `/role_vocabulary`).

## The role as a `paramPath`

MIDI-CI Property Exchange (Controller Resources, M2-117-UM v1.0.1, table 5) gives each control a `paramPath`, a path of parts ending in what is set; its example is `/Oscillator/A/Envelope/Attack`. OSD roles are the same path, kept as structure in the JSON, and `osd` gives and reads the text:

| Role | `paramPath` |
|---|---|
| parts `filter`, setting `cutoff` | `/Filter/Cutoff` |
| parts `filter`, `envelope`, setting `attack-time` | `/Filter/Envelope/AttackTime` |
| parts `amplifier`, `envelope`, setting `attack-time` | `/Amplifier/Envelope/AttackTime` |
| parts `oscillator` 2, setting `pitch` | `/Oscillator/2/Pitch` |
| parts `oscillator` 2, `envelope` 1, setting `attack-time` | `/Oscillator/2/Envelope/1/AttackTime` |
| parts `filter`, `x-acme-ladder`, setting `cutoff` | `/Filter/x-acme-ladder/Cutoff` |

- each term is its CamelCase segment: the words between hyphens start with a capital letter and the hyphens go (`attack-time` is `AttackTime`);
- the number of a part is a numeric segment after it;
- the setting is the last segment;
- an extension `x-<registrant>-<term>` is written as it is, its case unchanged.

`osd::roles` gives each role of a descriptor with its `paramPath`. `osd::role_from_param_path(path, state)` reads a text back to the same role against a named state: a CamelCase segment is a term of the state, a segment of the form of an extension is an extension, a number is the index of the part before it, and any other segment is refused at its place. Two terms of a state that say the same segment (`x1` and `x-1`, both `X1`) make the state refused, so the text always reads back to one role.

## What a reader checks

`osd` checks each role against the named state:

- a `part` is a term of type `part`, the `setting` a term of type `setting`, otherwise the role is refused at the faulty field;
- each part stands where its term says (`stands_under`, `at_top`, below): the first part opens a path, each next part stands under the one before it, and a part that does not is refused at that part; the index is carried by each part and does not change where a part stands;
- the setting applies to the last part (`applies_to` of the setting names it): `filter` with `pan`, or `filter` and `envelope` with `cutoff`, is refused at the role;
- a role of known terms is valid without being listed, and reads as a **known role**;
- a term the state lacks is written as an extension `x-<registrant>-<term>` (the form and the registry of 0.5: `^x-[a-z0-9]+-[a-z0-9-]+$`, `schemas/extensions.md`; `<registrant>` is the name the author chooses). A role with an extension, even in the middle of a path, is valid and reads as **no known role**; what an extension touches is not checked. A term that is neither known nor an extension is refused at the term.

Nothing in a reader is written per role: `osd::roles` resolves a descriptor's roles against the state's data.

## Ten roles of a real device

The Korg minilogue has a filter envelope (its EG, aimed at the cutoff) and an amplifier envelope; `examples/0.6/minilogue.device.json` gives its controls with their roles. Ten of them:

```json
{ "osc": "/amp/env/attack", "cc": 16, "role": { "parts": [{ "part": "amplifier" }, { "part": "envelope" }], "setting": "attack-time" } }
{ "osc": "/amp/env/release", "cc": 19, "role": { "parts": [{ "part": "amplifier" }, { "part": "envelope" }], "setting": "release-time" } }
{ "osc": "/eg/attack", "cc": 20, "role": { "parts": [{ "part": "filter" }, { "part": "envelope" }], "setting": "attack-time" } }
{ "osc": "/eg/decay", "cc": 21, "role": { "parts": [{ "part": "filter" }, { "part": "envelope" }], "setting": "decay-time" } }
{ "osc": "/lfo/rate", "cc": 24, "role": { "parts": [{ "part": "lfo" }], "setting": "rate" } }
{ "osc": "/delay/time", "cc": 31, "role": { "parts": [{ "part": "delay" }], "setting": "delay-time" } }
{ "osc": "/vco1/pitch", "cc": 34, "role": { "parts": [{ "part": "oscillator", "index": 1 }], "setting": "pitch" } }
{ "osc": "/vco2/pitch", "cc": 36, "role": { "parts": [{ "part": "oscillator", "index": 2 }], "setting": "pitch" } }
{ "osc": "/filter/cutoff", "cc": 43, "role": { "parts": [{ "part": "filter" }], "setting": "cutoff" } }
{ "osc": "/filter/resonance", "cc": 44, "role": { "parts": [{ "part": "filter" }], "setting": "resonance" } }
```

Their `paramPath`s are `/Amplifier/Envelope/AttackTime`, `/Amplifier/Envelope/ReleaseTime`, `/Filter/Envelope/AttackTime`, `/Filter/Envelope/DecayTime`, `/Lfo/Rate`, `/Delay/DelayTime`, `/Oscillator/1/Pitch`, `/Oscillator/2/Pitch`, `/Filter/Cutoff` and `/Filter/Resonance`.

## A term

Each term of a state has:

| Field | Says |
|---|---|
| `name` | its one name |
| `type` | `part` or `setting` |
| `definition` | its definition, in one sentence |
| `applies_to` | for a setting, the parts (terms of type `part` of the same state) it applies to; only the last part of a path is held against it |
| `stands_under` | for a part, the parts it may come under in a path (`envelope` under `filter`, `amplifier`, `oscillator`, `voice`) |
| `at_top` | for a part, whether it may open a path; true when `stands_under` is absent, false when it is present, unless the part writes it (`lfo` stands under `oscillator`, `filter` and `amplifier` and opens a path too) |
| `words` | the makers' words that mean it, each `{word, origin}`: the origin is the specification (the MIDI 1.0 table, M2-104-UM, Property Exchange) or the maker that uses the word |
| `source` | where it comes from: `{controller, name}`, the entries of the MIDI 1.0 table of defined controllers whose composition uses the term; and `{document, table, number?, name}`, a row of a published table (`M2-104-UM v1.1.2`, table `22`, row `73`, `Sound Controller 4 (Attack Time)`; `M2-117-UM v1.0.1`, table `5`). The documents are cited, never copied |
| `shared_by` | where a term the standard does not name is shared: a list of `{maker, device}`, two devices of two makers at least |
| `replaced_by` | the term of the same state that replaces it, when a later decision replaces it |

A term has a `source` or a `shared_by`. A term taken from a standard cites it; a term the standard leaves to each maker (`oscillator`, `mixer`, `arpeggiator`…) names two devices of two different makers that have it, and a term without that is not added.

A function is **composed**, not listed once per case. MIDI 1.0 "Brightness" (controller 74) is `filter` + `cutoff`: each cites controller 74 in its `source`, and "Brightness" is a maker's word of `cutoff`, not a term. Likewise controller 5 "Portamento Time" is `voice` + `portamento-time`, and controller 10 "Pan" is `output` + `pan`.

## What State 1 takes from MIDI 2.0

The names of M2-104-UM v1.1.2, table 22 (Registered Per-Note Controllers) that set a sound are held, each as a term or a maker's word of a term, with its row in `source`:

| Row | Name | Held as |
|---|---|---|
| 1, 2, 11 | Modulation, Breath, Expression | settings `modulation`, `breath`, `expression` of the part `performance`, what a player moves |
| 3 | Pitch 7.25 | the word of `pitch`, on `voice` |
| 7, 8, 10 | Volume, Balance, Pan | `level` (Volume is its word), `balance` and `pan`, on `output` |
| 70 | Sound Controller 1 (Sound Variation) | `variation` on `sound` |
| 71, 74 | Timbre/Harmonic Intensity, Brightness | `resonance` and `cutoff` on `filter` |
| 72, 73, 75 | Release, Attack, Decay Time | `release-time`, `attack-time`, `decay-time` on `envelope` |
| 76, 77, 78 | Vibrato Rate, Depth, Delay | `rate`, `depth`, `delay-time` on the part `vibrato` |
| 91 to 95 | Effects 1 to 5 Depth | `depth` on `reverb`, `effect` (Tremolo, Celeste, Phaser are its words), `chorus`; Reverb Send Level and Chorus Send Level are words of `depth` |

Row 79, Sound Controller 10, is undefined: it has no term. The standard names no oscillator, LFO, mixer or sequencer, which it leaves to each maker: the parts `oscillator`, `noise`, `mixer`, `filter`, `amplifier`, `envelope`, `lfo`, `effect` (with `delay`, `reverb`, `chorus` and `equalizer`, kinds of effect that open a path), `arpeggiator`, `sequencer`, `voice`, `output` and the settings they carry are state 1's, each with the devices that share it.

## The rule of entry

A role enters the vocabulary when it has a specific function that the existing roles cannot name, alone or put together. The open system must not let the same function in twice.

`osd` and the schema of the vocabulary hold what a machine can hold:

- a name present twice, a maker's word present under two terms (in any case), or a word that is the name of another term, is refused at the second one;
- two terms whose CamelCase segments are the same are refused at the second name;
- a term without a definition of one sentence, a type, a `source` or a `shared_by`, or the origin of each word is refused; a `shared_by` of fewer than two devices or of one maker is refused at the term;
- a term whose name or word is a path of parts and a setting the state already holds (`brightness` next to `filter` and `cutoff`, `filter-envelope-attack-time` next to `filter`, `envelope` and `attack-time`), or a setting whose controllers a part and another setting already compose, is refused at the term;
- an `applies_to`, a `stands_under` or a `replaced_by` that names no term, or one that names a term that is not a part, is refused; a setting that writes `stands_under` or `at_top` is refused.

The schema of the vocabulary states the shapes (a source by controller or by document, an origin, a definition of one sentence, `applies_to` for a setting, `stands_under` and `at_top` for a part, `shared_by`); comparing two terms is `osd`'s, since a JSON Schema cannot.

## How a term is proposed

A pull request adds one term to the next state (a new folder `schemas/vocabulary/<n+1>/`). It brings:

- the name, the one-sentence definition, the type, `applies_to` for a setting, `stands_under` for a part that stands under others;
- the makers' words, each with its origin;
- the `source` (a controller of the MIDI 1.0 table, a row of a published table with its document), or the `shared_by` of two devices of two makers;
- the test that no existing term or maker's word says the same function: the term written with the existing terms (parts and a setting) does not say it, and no word of another term is its word.

`cargo test -p osd` checks the whole state; the pull request is not mergeable until it passes.

## How a state is published

A state is numbered (`1`, `2`…) and is a folder: `terms.json` and its schema `vocabulary.schema.json`. A published state is frozen: its checksum is recorded in `schemas/SHA256SUMS`, CI fails on any change, and the next state is a new folder. The vocabulary grows without a new version of the format. Publishing a state is the owner's decision; `osd` holds each published state (`osd::VOCABULARY_STATES`) and reads it for as long as it exists. State 1 is not published yet, and changes in place until it is.

A state only adds (`Vocabulary::check_successor`): a term of a state stays in the following ones, with its name and type; a term a later state replaces keeps its place and carries `replaced_by`.

## From an extension to a term

When a state later holds the term an extension stood for, the author rewrites the role with the term and names that state in `role_vocabulary`. `osd` migrates nothing: a role is read as written, against the state named.
