# 0007 — A control and a command have a title and a description

## Status

- State: `candidate`
- Candidate since: 2026-11-24
- Under the trial: —

This proposal decides nothing until it is accepted. The owner answered its two questions (2026-11-24); each answer is written below where it decides, and none remains open. The proposal is a `candidate` since 2026-11-24, waiting at least 14 days, open to objections.

## What it changes

The version 0.7: the device schema (`schemas/0.7/device.schema.json`) and the text of 0.7 gain two optional fields, `title` and `description`, on an entry of `cc_params`, an entry of `params`, a command (an entry of `commands`) and a trigger, with the same form on the four; and a third optional field, `value_description`, on an entry of `cc_params`, a text for people, in CommonMark (0.28) like `description`, of what the values of the control mean. The converter 0.6 to 0.7 moves some existing human text into them, without loss, and writes no `title`. Nothing of 0.6 or 0.6.1 changes. The proposal 0006 names a reading's control or command by `title` for the same reason.

## Why

What this brings to the sharing of sounds on devices: every control and every command of a descriptor says its name, and what it does, in words a person reads, so a tool shows a device's controls by their names instead of their addresses.

What is wrong without it: in 0.6, neither an entry of `cc_params`, nor an entry of `params`, nor a command, nor a trigger has a field for its name; the OSC address stands for it, and an address addresses, it does not name. Human text, for its part, takes seven forms on the descriptors served: `notes` (programs, commands, sources), `_notes`, `_note`, `_comment`, `comments` (triggers), `_usage` and `_limitations`. Programs read three of them although the text of 0.6 says a key starting with `_` is a note a program never needs (`spec/0.6.md`, section 5.3): `_usage` on the entries of `cc_params`, given to agents as what the values of the control mean; `_name` on the entries of `params` and of `cc_params`, read by the converters of Electra One, MIDNAM and midi.guide as the name of the control; and `_description` on the entries of `cc_params` and the triggers, read by the converters of midi.guide's two CSV files.

The need comes from SyntHub, the catalogue service that serves descriptors. Its collection holds named controls (for example "Note Value", "FLT Width Offset") that today become addresses, and some of the contradictions between its catalogue and its collection are contradictions of name. The owner decided (2026-11-24): the standard is a facilitator of this need, never an obstacle (what SyntHub needs to say finds a place, the format decides how it is said); the standard holds no field specific to one product or vendor, so this proposal adds none; and before a field or a form is removed, the data of the partners (Electra One, midi.guide, KnobKraft Orm) is counted, a field a partner produces or reads staying, or changing form without loss.

## The change

### Where `title` and `description` live

An entry of `cc_params`, an entry of `params`, a command and a trigger MAY each carry:

| Field | Type | Says |
|---|---|---|
| `title` | string, optional | the name a person reads, of what the control or command does or of the physical control that sends it |
| `description` | string in CommonMark (0.28), optional | a description for a person, of the same thing |

An entry of `cc_params` MAY also carry, where `_usage` lives today:

| Field | Type | Says |
|---|---|---|
| `value_description` | string in CommonMark (0.28), optional | a text for people, of what the values of the control mean, beside `values` |

`description` stays the description of the control itself; `value_description` says what its values mean. The three are read by a program (they do not start with `_`), as fields of the schema. `title` is a plain line of text. `description` and `value_description` are texts for people: a reader may display them, and draws no structure from their content. A reader MUST refuse a `title`, a `description` or a `value_description` that is not a string, as `BadValue`.

The rule of 0.7 on the underscore, the same sentence as proposal 0008: a reader accepts a key that starts with `_`, keeps it and writes it back, and no program reads its meaning. No field of the schema 0.7 starts with `_`. The keys starting with `_` that programs read in 0.6 are listed in proposal 0008 with what each becomes; that list is a migration, not a reading that stays in 0.7. An empty `title` is refused as `BadValue`: an address is not a name, and a producer with no name leaves the field out.

### Is `title` required?

Decided by the owner (2026-11-24): `title` is optional in 0.7.

Reason: a 0.6 descriptor has no name for its controls, and the converter must produce a valid 0.7 descriptor from it. Making `title` required would force the converter to invent a name or to produce an invalid descriptor. The OSC address is not a name, and the converter never copies it into `title`. This is a departure from MIDI 2.0 Property Exchange, where `title` is required (see "What a reference does"); the departure is written here with its reason, compatibility.

What the converter writes for a 0.6 entry that has no name: nothing. The entry stays without `title`. What pushes a producer to write it: a reader makes the `title` available to the tool; the conformance case of a control without `title` is accepted, never refused; and a tool that shows a control without `title` says so, as an address that is not named, instead of passing the address as a name. Requiring `title` for the entries born in 0.7 (two regimes: required for new entries, optional for those converted) may be considered for a later version.

### What becomes of each existing form of human text

One rule sorts them, validated by the owner (2026-11-24): a text that says what the object is or does goes to `title` or `description`; a text of provenance, of reserve or of history stays a note for people.

| Form | On | Becomes in 0.7 |
|---|---|---|
| `_usage` | entry of `cc_params` | `value_description`, by the converter, unchanged and without reading its content, whether the entry has `values` or not. It says what the values mean, and a program already reads it. It never becomes `description`. |
| `_name` | entry of `params`, entry of `cc_params` | `title`, by the converter, unchanged, when the entry has no `title`; it names the control, and a program already reads it (the converters of Electra One, MIDNAM and midi.guide read it on `cc_params`). When the entry already has a `title`, the existing `title` wins and `_name` stays as a note, kept as it is. |
| `_description` | entry of `cc_params`, entry of `params`, trigger | `description`, by the converter, unchanged, when the entry has no `description`; a program already reads it (the converters of midi.guide's two CSV files). When the entry already has a `description`, the existing `description` wins and `_description` stays as a note, kept as it is. |
| `_notes` | entry of `cc_params` (a control) | `notes`, by the converter, unchanged: the same decision as proposal 0008 (the owner decided it last, 2026-11-24). `notes` keeps the meaning it has elsewhere: reserve, history. When the control already has a `notes`, the existing `notes` wins and `_notes` stays as a note, kept as it is. |
| `notes` | program, command, source | stays a note for people. `notes` without `_` is a field read by the schema of 0.6; in 0.7 the same name keeps its place where it already is. |
| `comments` | trigger | stays a note for people, kept as it is. |
| `_notes` elsewhere than on a control, `_note`, `_comment` | any | stay notes for people, kept as they are. |
| `_limitations` | any | outside this proposal. |
| `_electra` | any | outside this proposal: proposal 0008 says what it becomes (a general structure, its source kept). |

The status "note for people" of 0.6 is for the keys that start with `_`. After 0.7, no key a program reads starts with `_`: `_usage`, `_name`, `_description` and the `_notes` of a control, the ones a program read, leave the underscore for `value_description`, `title`, `description` and `notes`. A text that a program does not read stays under its `_` or, for `notes` and `comments`, where the schema of 0.6 already reads it.

This proposal removes nothing of the notes. The only keys it renames are those of the table above that a program reads. `_note` and `_comment` are neither renamed nor removed; proposal 0008 keeps them as notes.

The converter deletes no text written for people. Where two sources of text meet on one entry, the field that already exists is kept and the other stays a note, as it was.

What the partners' converters read after 0.7 follows the table: Electra One, MIDNAM and midi.guide read `title` of an entry of `cc_params` where they read `_name`; midi.guide's two CSV files read `description` where they read `_description`, `value_description` where they read `_usage`, and `notes` where they read `_notes`.

### The converter 0.6 to 0.7, before and after, with collisions

An entry of `cc_params` carrying `_usage`, `_name`, `_description` and `_notes`:

```json
{ "cc": 74, "osc": "/filter/cutoff", "_name": "Filter Cutoff", "_description": "Opens the filter.", "_usage": "0 closes, 127 opens.", "_notes": "Range checked on firmware 1.2." }
```

becomes

```json
{ "cc": 74, "osc": "/filter/cutoff", "title": "Filter Cutoff", "description": "Opens the filter.", "value_description": "0 closes, 127 opens.", "notes": "Range checked on firmware 1.2." }
```

`_usage` becomes `value_description`, whether the entry has `values` or not, without its content being read; `_name` becomes `title`, `_description` becomes `description`, and `_notes`, a note of reserve, becomes `notes`, each unchanged. When the target field already exists, it wins and the `_` key stays, as the next example shows.

An entry of `params` carrying `_name` and already a `title`:

```json
{ "osc": "/osc1/shape", "title": "Shape", "_name": "Osc 1 Shape" }
```

becomes

```json
{ "osc": "/osc1/shape", "title": "Shape", "_name": "Osc 1 Shape" }
```

The existing `title` wins; `_name` stays as a note, kept as it is, and nothing is lost. The same entry without a `title` becomes `{ "osc": "/osc1/shape", "title": "Osc 1 Shape" }`. When an entry already has a `value_description` and a `_usage`, the `value_description` wins and `_usage` stays as a note: both are kept. `description` is never written from `_usage`.

### The cases of conformance, as they would be written

Under `conformance/0.7/read/`, each with its input and its expected verdict:

1. `control-with-title-and-description-accepted` — a device descriptor of 0.7 with an entry of `cc_params` carrying `title: "Filter Cutoff"` and `description: "Opens the filter."`. Verdict: accepted; the reader returns both.
2. `control-without-title-accepted` — the same descriptor with no `title`. Verdict: accepted; the reader returns the control with no title.
3. `title-not-a-string-refused` — the same descriptor with `title: 74`. Verdict: refused as `BadValue`, at the pointer of the title.
4. `description-not-a-string-refused` — the same with `description: ["Opens"]`. Verdict: refused as `BadValue`, at the pointer of the description.
5. `command-with-title-accepted` — a command with `title: "Request dump"`. Verdict: accepted.

And, under `conformance/0.7/convert/`:

6. `usage-becomes-value-description` — a 0.6 entry of `cc_params` with `values` and `_usage: "0 closes, 127 opens."`. Verdict: converted with `value_description: "0 closes, 127 opens."` and no `_usage`.
7. `usage-name-description-notes-become-fields` — the entry of the example above. Verdict: `value_description`, `title`, `description` and `notes` from the four `_` keys, none of them left; the same verdict as case 11 of proposal 0008 for `_notes`. With a `notes` already on the control, `notes` is unchanged and `_notes` stays.
8. `usage-without-values-still-becomes-value-description` — a 0.6 entry with `_usage` and no `values`. Verdict: converted with `value_description`, no `description`.
9. `usage-with-existing-value-description-keeps-both` — an entry with `_usage` and already a `value_description`. Verdict: `value_description` unchanged, `_usage` kept as a note.
10. `name-with-existing-title-keeps-both` — the `params` entry of the example above, and an entry of `cc_params` with `_name` and `_description` and no `title`. Verdict: on the first, the `title` unchanged and `_name` kept as a note; on the second, `title` and `description` written from them and no `_name`, no `_description`.
11. `entry-without-name-gets-no-title` — a 0.6 entry with an address only. Verdict: converted without `title`.

And, under `conformance/0.7/read/`:

12. `value-description-not-a-string-refused` — an entry of `cc_params` with `value_description: 7`. Verdict: refused as `BadValue`, at the pointer of the field.
13. `underscore-key-is-kept` — an entry carrying a key starting with `_` (`_note: "checked"`). Verdict: accepted; the reader keeps the key and writes it back as it is, and no program reads its meaning. The same verdict as case 13 of proposal 0008.

## The tensions with the intentions of OSD

- **One meaning per field.** `osc` and `title` side by side: `osc` addresses the control, `title` names it; two fields, two meanings, and an address is never copied into a `title`. `description` in CommonMark is a text for people, as in MIDI 2.0 Property Exchange: no program reads its content, and a reader draws no structure from it. The formatting is not a mini-language of the format, it is the text a person reads. The tension with "structure lives in JSON structures that the schema checks" is real: the proposal holds that a text for people is not a structure, and nothing a program decides is put in it. `value_description` says what the values mean, `description` what the control is: two meanings, two fields. `notes` stays beside `description`: two texts for people on one entry, told apart by the rule (what the object is or does, against provenance, reserve and history).
- **Names reused from MIDI 2.0, MIDNAM and OSCQuery.** `title` and `description` are the names of Property Exchange. MIDNAM names with `Name` and has no description; OSCQuery describes with `DESCRIPTION` and has no name. Neither's name is taken: Property Exchange reaches both with one pair.
- **A departure from the references: `value_description`.** MIDI 2.0 Property Exchange, MIDNAM and OSCQuery, as quoted below, have no field for what the values of a control mean. The default answer of the references is therefore none, and the proposal departs from it for this reason: the owner decided that `_usage` says what the values mean, so it takes its own field beside `values` rather than being folded into `description`, which says what the control is.
- **No reader privileged.** No behaviour of a reader is the reference; a tool that ignores `title` and shows the address stays a conforming reader.
- **One meaning per field, again.** In 0.7 a key starting with `_` is accepted, kept and written back, and no program reads its meaning; `_usage`, `_name`, `_description` and the `_notes` of a control, which a program read, take fields of the schema.
- **Compatibility.** A descriptor valid in 0.6 reads in 0.7 through its converter, without loss and without invention: `title` optional, no text written for people deleted, no key renamed that a partner may produce and that the table does not name (`_note` and `_comment` stay).
- **Credit.** Unchanged: the text moved to `description` or `title` keeps the credit of the descriptor's `_sources`.

## What a reference does

- **MIDI 2.0 Property Exchange**, controller resources (M2-117-UM v1.0, table "Controller Message Data Object"), read at its source by the supervisor (2026-11-24), never copied here. Each controller has `title`, a string, required: the name a person reads, of the effect of the messages the device recognizes or of the physical control that sends them, which a connected device may display; and `description`, a string in CommonMark (0.28), optional. Every example of the document names its controller by `title` ("Volume", "Filter Cutoff", "Hold Pedal").
  - What the proposal takes: the two names, their forms, and their meaning.
  - Where it departs: `title` is optional in 0.7 where Property Exchange requires it. A device answers with its own controllers and always has a title to say; a descriptor converted from 0.6 has none, and the converter invents no name. The departure is written with its reason, and requiring it for the new entries may be considered for a later version.
  - A second departure: `value_description` has no equivalent in Property Exchange as quoted. The reason: the owner decided that `_usage` says what the values mean, so it takes its own field beside `values` and is not `description`.
- **MIDNAM** (MIDINameDocument10.dtd, version 1.0), read likewise. A `Control` of a `ControlNameList` has `Type` (7bit, 14bit, RPN, NRPN), `Number` and `Name`, the last two required; a `Patch` and a `Note` also have a required `Name`. No description.
- **OSCQuery** (the proposal of Vidvox, local copy), read likewise. A node has an optional `DESCRIPTION`, a human-readable description of the container or method; no name apart from its path.

None of the three has a field for what the values of a control mean, as quoted: `value_description` is a departure, with the reason above.

The three name a control; two of them describe it; the proposal follows Property Exchange.

## Objections

None yet.

## Questions for the owner

The owner answered the two questions of the draft (2026-11-24); none remains open.

1. **Is `title` required in 0.7?** Answer: no, it is optional (the converter writes no title when a 0.6 entry has none, and invents no name); the departure from Property Exchange is written in "Is `title` required?" with its reason. Requiring it only for the entries born in 0.7 may be considered for a later version.
2. **How are the forms of human text sorted?** Answer: the rule is validated: what the object is or does goes to `title` or `description`; provenance, reserve and history stay notes for people. `_note` and `_comment` are neither renamed nor removed; proposal 0008 keeps them as notes.

Decided with these answers, to keep the proposal consistent: a key starting with `_` is accepted, kept and written back, and no program reads its meaning, in 0.7; `_usage` becomes `value_description`, not `description`; `_name` of a `params` or `cc_params` entry becomes `title` and `_description` becomes `description`. The owner's later decision (2026-11-24) on the `_notes` of a control: it becomes `notes`, as proposal 0008 says.

The proposal is a `candidate` since 2026-11-24, waiting at least 14 days, open to objections.
