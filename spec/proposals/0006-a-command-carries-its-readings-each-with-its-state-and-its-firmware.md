# 0006 — A command carries its readings, each with its state and its firmware

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

Amended on 2026-11-25 with the precisions the building of 0.7 brought to the `range` and the keys of a control's reading. This change of a `candidate` text puts it back to `draft` ([PROCESS.md](../PROCESS.md), section 2): the delay of 14 days starts again the day it enters `candidate` again.

Amended with proposal 0008: the human name of a source is its `title`, as 0008 says of every source. Where this text writes `_sources`, 0008 renames it `sources`; the rest of this text is unchanged.

This proposal decides nothing until it is accepted. Every choice below is a recommendation; the questions that belong to the owner are listed apart, in the last section.

## What it changes

The version 0.7: the device schema (`schemas/0.7/device.schema.json`) and the text of 0.7 gain one optional field, `readings`, on a command (an entry of `commands`) and on a control (an entry of `cc_params` or of `params`), with the same form, and one field of an entry of `_sources`, `id`. The converter 0.6 to 0.7 gains no rule: it writes no `readings`. Nothing of 0.6 or 0.6.1 changes.

## Why

What this brings to the sharing of sounds on devices: a descriptor says which of its facts the device has confirmed, and on which firmware; and it keeps two contradictory readings of one command or one control side by side, so that a tool knows what to trust and nothing found is lost.

What is wrong without it: the text of 0.6 says "a device file describes one device from one source" and "a key that starts with `_` is a note for people" (`spec/0.6.md`, section 4). The only provenance (`_sources[]`, with its `firmware`) is a note a program does not read, and it covers the whole descriptor. No field says the state of one command or one control. A catalogue that holds two accounts of one command or control (a title or a range that differ) must choose one and drop the other, or publish a second descriptor for the same device and firmware.

The need comes from SyntHub, the catalogue service that serves descriptors. Its owner decided there (2026-11-24): one state per reading, with its firmware, in the descriptors served; two readings of one command go side by side in one descriptor, never a second descriptor for one device and firmware; the threshold of confirmation (three accounts on one firmware) belongs to the service, not to the format; two concordant sources do not make a confirmation.

## The change

### Where a reading lives

A command of `commands` and a control (an entry of `cc_params` or of `params`) MAY each carry `readings`, a non-empty array, with the same form on both. What a reading covers is a command or a control, never the descriptor as a whole. A reading is an object:

| Field | Type | Says |
|---|---|---|
| `state` | one of three words, required | what the reading is worth (below) |
| `firmware` | text, required | the firmware this reading holds for, written as `_sources[].firmware` is written; `unknown` when it is not known |
| `source` | text, optional | the `id` of an entry of `_sources` that gave the reading |
| `title` | text, optional | the title of the command or control in this reading (the field is `title`, as in proposal 0007) |
| `args` | array of `arg`, optional, on a command only | the arguments (with their ranges) of the command in this reading; a reading of a control has none |
| `range` | `[lowest, highest]`, two integers, optional, on a control | the range of the control in this reading |

A reading carries only what two accounts of one command or control can differ on (its title, its arguments or its range). What does not differ (the address of the message, the section) stays on the command or control, once. When a command or control has `readings`, a `title`, `args` or range of a reading replaces, for that reading, the one of the command or control; a reading that names none says it as it stands.

**The range of a reading of a control** is a pair `[lowest, highest]` of integers, the form of the `range` of 0.6. It is written in the reference scale of the control (proposal 0009): the control's `scale`, else the scale of its most precise way, or, for a control with no way, its `range`. It lies inside that scale, bounds included, its lowest not above its highest: a reader MUST refuse a range outside the scale, or with its lowest above its highest, as `BadValue` at the `range` of the reading. The range is compared with nothing when the reference scale is not known, as `default` is: a `params` entry whose ways give none, and an entry with no way that has neither `scale` nor `range`. A firmware that produces a wider range than the scale is written by making the control's `scale` that wide, never by a range outside it.

`args` is a key of a command's reading: on a reading of a control a reader MUST refuse it, as `UnknownField`.

`readings` is a field read by a program, on a command and on a control: it does not start with `_` and is not a `notes`. A reader MUST refuse an unknown key of a reading, as `UnknownField`.

### The closed list of states

| Word | Says |
|---|---|
| `valid` | the service that wrote the descriptor holds the reading as confirmed |
| `to-confirm` | the service holds the reading as not yet confirmed |
| `refuted` | the service holds the reading as contradicted |

Each state is a fact the reader passes on without judging it: the format does not say what makes a reading `valid`, and the threshold that decides the state stays with the service that writes the descriptor. A reader MUST refuse a word outside the three, as `BadValue`. The ask of SyntHub wrote the middle state "to confirm"; the owner chose `to-confirm`, one token. The field is named `state`, never `status`: in MIDI 2.0 Property Exchange `status` is the code of a reply in the header of a message, a different meaning.

What a reader does with each state, so that the field has one meaning:

- `valid`: the reading may be used as the command or control.
- `to-confirm`: the reading may be used; a tool that shows it says it is not confirmed.
- `refuted`: the reading is kept as a record and MUST NOT be offered as the command or control by default.

### How a reading names its firmware and its source

`firmware` is a copy of the text of `_sources[].firmware`, not a reference to it: a reading holds for a firmware, and the source that gave it may document another. `source` names the entry of `_sources` that gave the reading, by its `id` and not by its place, so that reordering `_sources` breaks nothing and the credit and the licence (`_sources[].license`) stay where the format already puts them. Each entry of `_sources` gains a short field `id`, an identifier the reader checks. The `id`s are unique across the union of the sources of a file with `midi_of` and those of its target (the union of `spec/0.6.md`, section 23, "A file with `midi_of`"); a reader MUST refuse a duplicate, as `BadValue`, the error this text already gives to an unknown `source`. The `source` of a reading is resolved in that union and MUST be one of its `id`s (otherwise `BadValue`). The two fields of a reading have two meanings: `firmware` says where the reading holds, `source` says where it comes from. `source` stays optional: a reading without `source` is the statement of the descriptor itself, which the `_sources` of the descriptor credit.

**No reserved `id`.** The format reserves no `id` for a source. A publisher that does not reveal where it took a fact cites itself: the entry of `_sources` that a reading names is the publisher (its title, its type, its licence). The provenance upstream stays with the publisher, outside the descriptor, as an editor keeps their notes; the format never carries it.

The rule of credit ("every descriptor names its sources and their licence") holds as it is: attribution is said per reading of a command or of a control, and every reading is either credited by the entry of `_sources` it names, or is the statement of the descriptor, credited by its `_sources`. No reading is left uncredited, and the rule has no exception.

`readings` leaves the notes: it is the first field of a command or a control that a program reads and that holds provenance. `_sources` itself stays a note for people, except its `id`, which a program reads; the rest of a source is not read by a program.

### Two readings of one command, side by side

```json
{
  "title": "Filter cutoff",
  "section": "Filter",
  "readings": [
    { "state": "valid",     "firmware": "1.2", "source": "manual", "title": "Filter cutoff", "args": [{ "name": "value", "type": "int", "min": 0, "max": 127 }] },
    { "state": "to-confirm", "firmware": "1.3", "source": "synthhub", "title": "Cutoff",       "args": [{ "name": "value", "type": "int", "min": 0, "max": 255 }] }
  ]
}
```

with, in `_sources`, the entries `{ "id": "manual", … }` and `{ "id": "synthhub", "title": "SyntHub collection", "type": "community-re", "license": "CC-BY-SA-4.0", "firmware": "1.3" }`: the second reading cites the publisher itself, which does not say where it took the fact.

There is one command, one descriptor, one device; the two readings sit in it, each with its state and its firmware. A control of `cc_params` or `params` carries its readings the same way, with its range in place of `args`. Two readings may share a firmware (two accounts on the same firmware that disagree); the format keeps both.

### How a reader picks a reading

The reader picks by firmware only; beyond that the choice is the tool's. The rule: for a firmware asked, the readings whose `firmware` is the same text, then among them the states in the order `valid`, `to-confirm`, `refuted` (a `refuted` reading is returned only when asked for). When that list is empty, because the firmware asked has no reading or its only readings are `refuted` and not asked for, the reader returns all the readings of the command that are not `refuted` (all of them when `refuted` is asked for), in the order written, none chosen, with their firmwares and states: the format does not say a reading of 1.2 holds on 1.3, since firmwares are free text (`spec/0.6.md` section 14: a reader MUST NOT refuse the `version` of a firmware for its form).

Reason: choosing between two contradictory readings on a firmware nobody confirmed is a judgement on the data, and the data is the catalogue's (see the tension below).

A figure: a command with the two readings above. A tool on firmware 1.2 receives the first (valid). On 1.3 it receives the second, with the state `to-confirm`, and shows it as unconfirmed. On 1.4, a firmware with no reading, it receives both, neither chosen, each with its firmware and state. On `unknown` it receives the readings whose `firmware` is `unknown`, and if none, both again. Were the reading of 1.3 `refuted`, a tool on 1.3 that does not ask for the refuted readings would receive the reading of 1.2 alone; one that asks for them would receive the reading of 1.3.

### What becomes of a 0.6 descriptor read as 0.7

The converter writes a 0.6 command as it is, with no `readings`: a command without `readings` is one reading, its own, and a reader returns it with no state. This keeps every 0.6 descriptor readable and valid as 0.7 without inventing a fact the descriptor never held. The owner decided that the state of such a command is not the format's question but the catalogue's: the format gives no state to a command that has none, the converter writes none, and the service that serves the descriptor writes the states it holds. The proposal does not say that a 0.6 reading is confirmed, nor that it is not.

### The cases of conformance, as they would be written

Under `conformance/0.7/read/`, each with its input and its expected verdict:

1. `reading-state-accepted` — a device descriptor of 0.7 with a command whose `readings` has one entry `{state: "valid", firmware: "1.2"}`. Verdict: accepted; the reader returns the state `valid` and the firmware `1.2`.
2. `reading-state-outside-the-three-refused` — the same descriptor with `state: "probably"`. Verdict: refused as `BadValue`, at the pointer of the state.
3. `readings-of-one-command-on-two-firmwares-accepted` — one command with two readings, `{valid, 1.2, title "Filter cutoff", max 127}` and `{to-confirm, 1.3, title "Cutoff", max 255}`. Verdict: accepted; the reader returns both, each with its state and firmware, in the order written.
4. `source-id-duplicated-refused` — an `id` of source written twice, in one file or between a file with `midi_of` and its target. Verdict: refused as `BadValue`.

For the readings of a control, under `conformance/0.7/read/`: `control-reading-state-accepted`, `control-reading-args-refused` (refused as `UnknownField`) and `control-reading-range-outside-the-scale-refused` (refused as `BadValue`).

And, in `conformance/0.7/convert/`: `command-without-readings-stays-as-it-is` — a 0.6 device with one command. Verdict: converted without a `readings`; read as 0.7, the command is returned with no state.

## The tensions with the intentions of OSD

- **One meaning per field.** `firmware` repeats the text of `_sources[].firmware`: two fields with one text, but not one meaning (where the reading holds, against what the source documents). The proposal keeps both and says so; the alternative (a reading referring to a source only, the firmware taken from it) gives one meaning to the pair but ties a reading to the one firmware its source documents, which is false for a source that documents none (`unknown`). The text of the state is a closed word, not a note and not a phrase in a string.
- **Names reused from MIDI, MIDNAM and OSCQuery.** The proposal reuses no name from them: none of the three carries alternative values or a state of confirmation (see "What a reference does"). `readings`, `state`, `id` and the three words are new names, and `state` is chosen so as not to be `status`, which Property Exchange uses for a reply code.
- **No reader privileged.** The three states and the rule are stated from the side of the descriptor; no reader's behaviour is the reference, and a tool that picks otherwise (the format leaves the choice, above) stays a conforming reader.
- **The format stays strict, the uncertainty of the data stays in the catalogue.** This is the main tension. A state, however closed, is a statement about the quality of data, which the intention of OSD gives to the catalogue. The owner decided that the state belongs to the format, with the line the proposal draws: the format says that a state exists, its three words, and what a reader does with each; it does not say what decides it. The threshold stays with the service, and the reader never computes a state. A descriptor with `readings` is still strict (closed words, a `firmware` and a `state` required).
- **Credit.** The rule holds as it is, with no uncredited reading and no exception. A publisher that does not reveal where it took a fact cites itself as the source (its title, type and licence in `_sources`) and keeps the provenance upstream outside the descriptor, so that a fact can be withdrawn on request. A reading without `source` is the statement of the descriptor, credited by its `_sources`. The confirmations of a state (the reports and checks that make a reading `valid` or `refuted`) are private: they make the state of a reading, never its source, and never appear in a descriptor. A reading may be in any state whatever its source: the state comes from the confirmation, not from the source.

## What a reference does

- **Wikidata** (read at its source, 2026-11-24). Several values of one property sit side by side on one item, each statement with its own rank, qualifiers and references. https://www.wikidata.org/wiki/Help:Ranking: "The default rank is the "normal" rank; statements-value pairs may also be marked with "preferred" or "deprecated" ranks."; "The deprecated rank is used for statements that are known to include errors" or that "represent outdated knowledge."; a reader by default: "per default preferred statement(s) for a property will be used if they exist, otherwise normal statement(s) will be used"; "deprecated statements will never be used unless that is specifically requested."; "There can be any number of statements with each rank". https://www.wikidata.org/wiki/Help:Qualifiers: qualifiers "constrain the validity of the value". https://www.wikidata.org/wiki/Help:Sources: "references are used to point to specific sources that back up the data provided in a statement." https://www.wikidata.org/wiki/Help:Deprecation: statements are ranked deprecated "(and not removed)"; "All statements, including deprecated ones, must be verifiable."
  - What the proposal takes: a value is never removed for being contradicted, it keeps a state (`refuted` as the deprecated rank); several readings sit side by side with no limit; a reading points to its source; a default reader skips the refuted one unless asked; the firmware is the qualifier that bounds where a reading holds.
  - Where it departs: Wikidata's default rank is `normal`, a neutral state, and its preferred rank says a consensus; OSD's three words say a confirmation, which is the service's judgement. The proposal has no rank that says "preferred" among readings, and no neutral state: a command with no state is a command without `readings`.
- **MIDI 2.0 Property Exchange**, controller resources (M2-117-UM v1.0), not copied here. The device itself answers with the list of its controllers (keys such as `title`, `ctrlType`, `ctrlIndex`, `default`, `minMax`, `priority`). Its only `status` is the code of a reply in the header of a message, not a state of a datum. It has no alternative values and no state of confirmation: what a device says of itself is the fact. The proposal takes from it the avoidance of the name `status`; a reading confirmed by the device itself is what Property Exchange calls the device's answer.
- **MIDNAM** (MIDINameDocument10.dtd, version 1.0 of 2003-01-19), read likewise, not copied here. Its elements are Author, MasterDeviceNames, ExtendingDeviceNames, Manufacturer, Model, Device, PatchBank, PatchNameList, Patch, NoteNameList, Note, ControlNameList, Control, ValueNameList, Value, MIDICommands and the device modes. One Author for the whole document; no state, no alternative values.
- **OSCQuery** (the proposal of Vidvox), read likewise. A node has required attributes (TYPE, …) and optional ones (ACCESS, VALUE, RANGE, UNIT, TAGS, EXTENDED_TYPE, CRITICAL, CLIPMODE); an optional attribute not given is "not relevant". No state of confirmation, no alternative values.

None of the three carries alternative values or a state of confirmation; the form follows Wikidata.

## Objections

None yet.

## Questions for the owner

What was searched: the text of 0.6 (sections 4, 6.6, 14), the device schema of 0.6 (`command`, `source`), the proposals 0001 to 0005, Wikidata's help pages and the three references as quoted above.

The owner answered the questions of the draft (2026-11-24); their answers are written in the text above. None remains open.
