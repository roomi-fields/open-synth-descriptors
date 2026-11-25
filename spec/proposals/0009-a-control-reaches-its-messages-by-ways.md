# 0009 — A control reaches its messages by ways, each with its direction and its context

## Status

- State: `candidate`
- Candidate since: 2026-11-24
- Under the trial: —

The owner answered the questions of the first draft on 2026-11-24; the answers are written into the text where they decide, and no question remains open (section "The owner's answers"). The proposal is a `candidate` since 2026-11-24, waiting at least 14 days, open to objections.

## What it changes

The text and the schemas of 0.7: how an entry of `cc_params` (and of `params`) says the messages that reach it. The flat fields of an entry (`cc`, `cc_lsb`, `nrpn_msb`, `nrpn_lsb`, `rpn_msb`, `rpn_lsb`, `value_bits`, `range`), `also`, and the `transmit` and `recognize` of the entry give way to one list, `ways`. Sections 22.3 and 22.6 of `spec/0.6.md` are rewritten on it.

## Why

What this brings to the sharing of sounds on devices: a control says in one structure every message that reaches it, in which direction and in which context, so a tool reads one form instead of three, and the devices 0.6 cannot describe become describable.

What is wrong without it. 0.6 says the link between a control and its messages three ways:

1. the flat fields of the entry give its first message;
2. `also` gives the next ones (`spec/0.6.md:1137`);
3. proposal 0003 (candidate) adds `in`, a context, on the entry.

And `transmit` and `recognize`, on the entry, say the direction and the coding of the whole control, though a control reached by a CC and an NRPN may send the one and only recognise the other. One thing has several forms. 0.6 also has no word for three kinds of message that real controls use: channel pressure, polyphonic pressure and pitch bend (the words `chPress`, `pPress`, `pBend` of the Property Exchange norm), and a note (a button, a pad), with named `values` for the states of a light.

## The change

### The form

A control (an entry of `cc_params`, of `params`) has `ways`, a list of one way or more. The flat fields, `also`, `transmit`, `recognize` and `in` are no longer fields of the entry: they live on the ways. The entry keeps what is the control's own: its `osc` address, `role`, `default`, `priority`, `step_count`, `values`, `expects`, `section`, `transform`, its title and description, its `channel`, and its `scale` (below).

A way says one message and how it is reached:

| Field | Says |
|---|---|
| `ctrlType` | the message kind: `cc`, `nrpn`, `rpn`, `chPress`, `pPress`, `pBend`, `note`, `sysex` (for a `params` entry only) |
| `ctrlIndex` | the numbers of the message, a list of integers, by kind (below); absent for `chPress` and `pBend` |
| `value_bits` | the width of the value: 7 or 14; required with `nrpn` and `rpn`, and with a `cc` that has an LSB (14); optional with a simple `cc` (7, the width its kind implies) |
| `range` | lowest and highest value of the message, where needed |
| `transmit` | `absolute`, `relative`, `both`, `none` (the four words of M2-117-UM v1.0, tables 6 and 12) |
| `recognize` | the same four words |
| `in` | the context the way holds in (proposal 0003) |
| `slice` | the slice of values the way holds in (proposal 0003) |
| `nrpn_lsb_range` | for an `nrpn` way only: `[lowest, highest]`, the parameter LSB numbers of the block the way stands for (section 22.4 of 0.6, unchanged); it is not the range of the value |

`values`, `expects`, `section`, `transform` and `role` are not on the way: each says the control (what its values mean, what it expects, where it is laid out, how it is computed, what it is), not one message that reaches it.

#### `value_bits`

`value_bits` says the width of the value of a way. 0.6 rule 22.2 (`spec/0.6.md`, section 22.2) requires it on an NRPN or an RPN and refuses it on any other way, since the bits of a CC follow from `cc_lsb`. In 0.7:

- an `nrpn` or an `rpn` way MUST give `value_bits` (`Missing` when absent, `BadValue` for a width other than 7 or 14);
- a `cc` way with an LSB (`ctrlIndex` of two integers) MUST give `value_bits: 14`;
- a simple `cc` way MAY omit it: it takes 7 bits and the range 0 to 127 its kind implies; it MAY give `value_bits: 7`; any other width is `BadValue`;
- `chPress`, `pPress` and `note` carry no `value_bits` (7 bits, implied); `pBend` is 14 bits and MAY give `value_bits: 14`.

So rule 22.2 becomes: required where the message does not imply it, optional (and then only the implied width) where it does. A reader MUST NOT read the absence on a simple `cc` as a missing field.

What the mature references do: Property Exchange has `numSigBits` on every controller entry (32 if absent) and no `value_bits`; MIDNAM names the type of a control (`7bit`, `14bit`) and has no width field. `value_bits` is an OSD word, and the departure has its reason: 0.6 already says it (`value_bits` of an NRPN or an RPN), and the converter from 0.6 carries it without loss; the word is kept so that no descriptor of 0.6 is rewritten into another meaning.

#### The scale of a control

A control may give `scale`, `[lowest, highest]`: the range its `default`, `values`, `step_count` and readings are written in, the **reference scale**. Without it, the reference scale is the range of the **most precise way** (the way whose maximum value is the highest; its `range`, else the range its kind and `value_bits` imply: 0 to 127 for 7 bits, 0 to 16383 for 14). When two ways have the same highest maximum, the first of the two in the written order is the reference.

`default`, `values`, `step_count` and the readings of a control are written in the reference scale, and a tool converts them to the scale of each way when it needs the way's value. The conversion is a proportional scaling from the reference scale to the way's, rounded to the nearest (the half upward), never truncated: `round(v × (way_max − way_min) / (ref_max − ref_min))`, for scales that start at 0. The reference value stays the value of the control: the less precise way carries its nearest approximation, and the most precise way carries it without loss. Example, with a 14-bit reference (0 to 16383): a `default` of 8192 becomes 64 on a 7-bit way (8192 × 127 / 16383 ≈ 63.5, rounded up), and stays 8192 on the 14-bit way. So no information that a way can carry is lost.

The same shape in the other direction: a value received on a way is scaled to the reference scale in the same manner.

`ctrlIndex` by `ctrlType`. When there are two integers, the order is always MSB then LSB.

| `ctrlType` | `ctrlIndex` | Notes |
|---|---|---|
| `cc` | `[msb]` (7 bits), or `[msb, lsb]` (14 bits, with `value_bits: 14`) | the second is a 14-bit CC (`cc` and `cc_lsb` of 0.6) |
| `nrpn` | `[msb, lsb]` | `nrpn_msb`, `nrpn_lsb` of 0.6 |
| `rpn` | `[msb, lsb]` | `rpn_msb`, `rpn_lsb` of 0.6 |
| `pPress` | `[note]` | the note number, 0 to 127 |
| `note` | `[note]` | the note number, 0 to 127 |
| `chPress`, `pBend` | none: the field is absent, never an empty list | one message per channel |
| `sysex` | `[pr, p, c, r]` or `[id]`: the numbers of the 0.6 params entry that it gives, in that order | a `params` entry only (below) |

`nrpn_lsb_range` claims, as in 0.6, only the message of its own `ctrlIndex`, not the whole block: a reader MUST NOT claim the LSB numbers of the block. A reader MUST keep it as written and write it back.

### The ways of a `params` entry

A `params` entry (a parameter read and written by SysEx) has `ways` like a control. Its way is of kind `sysex`: its `ctrlIndex` carries the numbers that address the parameter in the maker's frame, from the fields of the 0.6 entry: `pr`, `p`, `c`, `r` (each one byte, 0 to 255, only those the entry gives) and `id` (one integer). A `sysex` way carries `range`, `value_bits` where the 0.6 entry gave it, `transmit` and `recognize`, and, with `in` and `slice`, the same context as any way. The entry keeps `osc`, `role`, `default`, `values`, `expects`, `section`, `transform`, `priority` and `step_count`. The kinds of MIDI (`cc`, `nrpn`, `rpn`, `chPress`, `pPress`, `pBend`, `note`) are not those of a `params` way, and `sysex` is not that of a `cc_params` way.

The converter writes, for a 0.6 `params` entry: one way, `ctrlType: sysex`, `ctrlIndex` the numbers `pr`, `p`, `c`, `r` that the entry gives (or `[id]`), the entry's `range` on the way, its `transmit` and `recognize` on the way; the rest stays on the entry.

What the mature references do: Property Exchange controller resources describe MIDI controller messages and have no kind for a SysEx parameter; MIDNAM neither. `sysex` is an OSD word, and the departure has its reason: a SysEx parameter is a control of the device, and putting it in a way lets one structure carry every control; the converter from 0.6 writes it without loss.

The names of a way's fields are Property Exchange's: `ctrlType` for the kind and `ctrlIndex` for the numbers, a list of integers as in PE (one number for a CC or a note, two for an NRPN or an RPN). The kinds are PE's `ctrlType` words where PE has them (`cc`, `nrpn`, `rpn`, `chPress`, `pPress`, `pBend`); `note` is OSD's.

Two ways of a control may differ in kind, numbers, width, range, direction and context: each is the way's own.

### Why `note`

PE has no kind for a note, since its controllers are continuous (M2-117-UM). The kind is named `note`: the word is MIDI's, and it means a note on / note off message of a given number (and its channel). A button or a pad that sends a note is a control reached by a `note` way. `note` and the `values` of its control (the states of a light: a value of the velocity and its title) are general: no field and no key of them belongs to a product, and the form of a partner's device is taken in only as entries of `sources` (the source is credited by an entry of the sources, never by the shape of a field).

A note number as the message of a control (`note`) and the same number as a polyphonic pressure (`pPress`) are two messages: their kinds differ, so they are not claimed against each other (see the rule below).

### The order of the ways, and what a sender and a receiver do

This is 0.6's rule of `also`, said on the list.

- The ways are in the order written. A reader MUST NOT reorder them.
- A **receiver** recognises every way whose `recognize` is not `none` as a message of the same control.
- A **sender** picks the first way it can send: the first whose `transmit` is not `none`, whose kind and width it can write, and whose context holds (see the rule).
- A way with `transmit: none` is still recognised if its `recognize` allows it; the sender skips it and takes the next.
- A way with `recognize: none` is sent when its `transmit` allows it; the receiver does not take its message for this control.
- A way with both at `none` says a message the device neither sends nor takes for this control: a reader keeps it, a tool ignores it, and it claims nothing. The descriptor may name it for a source that lists it.
- A way with **neither** `transmit` nor `recognize` written says nothing of its direction (as a midi.guide line does): it is both a sent and a recognised message, and **claims on both sides**. Only an explicit `none` removes a side. The absence is not `none`.

`relative` is carried on the way by `transmit` and `recognize` (`relative` or `both`) and says nothing more than in `spec/0.6.md` section 22.3: the norm, not the device, says how a relative value is coded.

### The message claimed once, by side, with contexts

0.6 says a message is claimed once (`spec/0.6.md:1145`). Proposal 0003 says: once by claims that can hold together. Here, written on the ways:

- A **claim** is a way on one side: its message (`ctrlType`, channel, `ctrlIndex`) with its coordinates (the sets of `in`, and the interval of `slice`).
- A way claims its message **on each side it works**: as a receiver when `recognize` is not `none`, as a sender when `transmit` is not `none`. A way with both at `none` claims nothing.
- A way claims **the numbers it occupies**: `[7]` claims the CC number 7; `[7, 39]` claims the CC numbers 7 and 39 (a 14-bit CC occupies its MSB and its LSB). An NRPN or an RPN `[msb, lsb]` is one message; `[note]` claims the note.
- **The same message in the same context** is: the same `ctrlType`, a `ctrlIndex` that shares a claimed number (for `cc`; for the other kinds, the same list), the same channel (or both with none), and contexts that overlap. Two contexts overlap unless a coordinate named by both has two sets with nothing in common; a coordinate that only one names leaves them overlapping.
- **Two ways overlap only if they claim on the same side.** Two ways of the same message that claim on different sides are admitted.
- A reader MUST refuse the later claim of a message when the two ways overlap, across the ways of a control as across the controls, at the later place, naming the earlier.
- Two ways of the same message whose contexts do not overlap are admitted.

This is PE's own freedom: PE lists a message twice, once to send and once to recognise; here the one way says both, and a way that works one side only claims that side only.

Examples, on the NRPN `[0, 122]` with no channel:

- Refused, a partial overlap: two ways in the same mode `multi`, one on the slice `[0, 63]` and one on `[32, 95]`. They share the values 32 to 63.
- Accepted, disjoint slices: `[0, 63]` and `[64, 127]` in the same mode.
- Accepted, separate parts: two ways with `in: { "part": "track1" }` and `in: { "part": "track2" }`.

Examples, on the CC:

- Refused, a number in common: a way `{ "ctrlType": "cc", "ctrlIndex": [39] }` (7 bits) against a way `{ "ctrlType": "cc", "ctrlIndex": [7, 39], "value_bits": 14 }`. Both claim the CC number 39 on the same sides.
- Accepted, one way receives and the other sends the same CC: way 1 `{ "ctrlType": "cc", "ctrlIndex": [7], "transmit": "absolute", "recognize": "none" }`, way 2 `{ "ctrlType": "cc", "ctrlIndex": [7], "transmit": "none", "recognize": "absolute" }`. Way 1 claims the sender side, way 2 the receiver side.
- Refused on the sender side: two ways `{ "ctrlType": "cc", "ctrlIndex": [7], "transmit": "absolute", "recognize": "none" }`. Both claim the CC 7 as a sender.

The order of the ways stays the sender's rule: it takes the first sendable way whose context holds. A way whose context does not hold at the moment is skipped; the next is tried.

`note` number 60 and `pPress` number 60 on one channel: different kinds, two messages, no claim between them.

### What the converter writes

For a 0.6 control the converter writes, without loss:

| 0.6 | 0.7 |
|---|---|
| `cc` | first way, `ctrlType: cc`, `ctrlIndex: [cc]` |
| `cc_lsb` | the same way, `ctrlIndex: [cc, cc_lsb]` (MSB then LSB); `value_bits: 14` |
| `nrpn_msb`, `nrpn_lsb` | first way, `ctrlType: nrpn`, `ctrlIndex: [nrpn_msb, nrpn_lsb]` |
| `rpn_msb`, `rpn_lsb` | first way, `ctrlType: rpn`, `ctrlIndex: [rpn_msb, rpn_lsb]` |
| `value_bits` (NRPN, RPN) | the way's `value_bits`; a simple CC has none in 0.6 and the converter writes none |
| `range` | the way's `range` |
| `nrpn_lsb_range` | the NRPN way's `nrpn_lsb_range` |
| each element of `also`, in order | the next ways, same rules, its own `range` and `value_bits` |
| `transmit`, `recognize` of the entry | written on every way (0.6 has one value for the control) |
| `values` (a dictionary label → integer) | the entry's `values`, a list of `{ "value": integer, "title": label }` in the order of the dictionary, as Property Exchange's `ctrlMapId` list gives `value` and `title`; the integers are in the reference scale (below) |
| `expects`, `section`, `transform`, `role`, `default`, `priority`, `step_count` | the same fields of the entry, unchanged |
| a `params` entry's `pr`, `p`, `c`, `r`, `id`, `range`, `transmit`, `recognize` | one way of kind `sysex` (above) |
| the scale | the range of the first way, written as `scale` on the control only if the control has a `default`, `values` or `step_count` and its first way is not the most precise (below) |
| `in` of a 0.7 draft of 0003 | `in` of the first way, and of each other |

**The scale the converter writes.** The scale of a 0.6 control is the range of its input, that is, of its first way (0.6 says `default` "in the space of `range`", section 22.1). The converter writes it as `scale` on the control only when the control has a `default`, `values` or `step_count` and its first way is not the most precise (another way has a higher maximum). Otherwise the first way is the reference already, and nothing is written. So each 0.6 value keeps its value. A simple CC without an explicit `value_bits` takes 7 bits and the range 0 to 127 that its kind implies.

**Why the 0.6 `value_bits` rule and the `sysex` way depart from the references.** See "What a reference does": Property Exchange and MIDNAM have neither a `value_bits` on a CC nor a way of kind `sysex`. Both are OSD's words, kept for compatibility: the converter from 0.6 writes them without loss.

A converter MUST keep the order. A converter MUST NOT join two controls. A converter writes no value that a later version adds, and no value its source lacks: a way gets `transmit` or `recognize` only if the control had it.

#### Worked example

A control of 0.6, a CC and an NRPN for the same setting, in the order of preference, absolute to send and any coding to recognise:

```json
{ "osc": "/filter/cutoff", "cc": 74, "range": [0, 127], "transmit": "absolute", "recognize": "both",
  "also": [ { "nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "range": [0, 16383] } ] }
```

becomes, with every field in its place (`cc` in way 1's `ctrlIndex`, `range` in way 1, `also` in way 2, `nrpn_msb` and `nrpn_lsb` in way 2's `ctrlIndex`, `value_bits` and `range` in way 2, `transmit` and `recognize` on both):

```json
{ "osc": "/filter/cutoff", "ways": [
  { "ctrlType": "cc",   "ctrlIndex": [74], "range": [0, 127],
    "transmit": "absolute", "recognize": "both" },
  { "ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14, "range": [0, 16383],
    "transmit": "absolute", "recognize": "both" } ] }
```

(A plain CC has no `value_bits`: 0.6 refuses it there, and the kind implies 7 bits. The control has no `default`, `values` or `step_count`, so no `scale` is written.)

#### A control whose ways differ in direction

0.6 gives one `transmit` and one `recognize` per control, so a converted control has equal values on all its ways. A 0.7 descriptor may then give each way its own, as an author refines it, and a tool reads each way alone:

```json
{ "osc": "/amp/level", "ways": [
  { "ctrlType": "nrpn", "ctrlIndex": [0, 5], "value_bits": 14,
    "transmit": "none", "recognize": "absolute" },
  { "ctrlType": "cc", "ctrlIndex": [7], "value_bits": 7,
    "transmit": "absolute", "recognize": "none" } ] }
```

The receiver recognises the NRPN (way 1) and not the CC (way 2). The sender skips way 1 (`transmit: none`) and sends the CC. Each way kept its own two values; nothing was merged.

### The round trip with the partners' data: no count is needed

A field or a form that disappears with its information would call for a count of what the partners' data (Electra One, midi.guide, KnobKraft Orm) holds in it. None disappears here: the flat fields, `also`, `transmit` and `recognize` change form without loss, the converter carrying each value into a way (table above). So no count is needed, and the examples below show that the round trip stays exact.

**A midi.guide line with a CC and an NRPN.** midi.guide carries no direction: a line gives a name, a CC and an NRPN. In the flat form of 0.6 it is:

```json
{ "osc": "/filter/cutoff", "cc": 74, "nrpn_msb": 1, "nrpn_lsb": 8 }
```

The two ways written from it have no `transmit` and no `recognize`, since the source carries none and the converter writes no value the source lacks:

```json
{ "osc": "/filter/cutoff", "ways": [
  { "ctrlType": "cc",   "ctrlIndex": [74] },
  { "ctrlType": "nrpn", "ctrlIndex": [1, 8] } ] }
```

Written back to a midi.guide line, way 1 gives `cc` 74 and way 2 gives the NRPN 1:8: the line is found again, with nothing added and nothing lost.

**An Electra One control in the relative mode.** The mode is the coding of the way, as in 0.6, and is kept without loss. The control, with its source credited in `sources`: an entry of the closed list of types (`contributed`) and only the keys of a source (`type`, `notes`); no field has the product's name:

```json
{ "sources": [ { "type": "contributed", "notes": "control in the relative mode" } ],
  "cc_params": [
    { "osc": "/encoder/1", "ways": [
      { "ctrlType": "cc", "ctrlIndex": [20], "value_bits": 7,
        "transmit": "relative", "recognize": "relative" } ] } ] }
```

One way, with `transmit: relative` and `recognize: relative`. Written back to a control of the source, the way gives its CC and its relative mode, and read again it is found unchanged.

### The cases, as they would be written

Each with its input and its verdict.

1. **A control with two ways.** `ways` of a `cc` `[74]` and an `nrpn` `[1, 8]` with `value_bits` 14. Verdict: valid; the order is kept.
2. **A pressure control.** `ways: [ { "ctrlType": "chPress", "transmit": "absolute", "recognize": "absolute" } ]`. Verdict: valid.
3. **A pitch bend control.** `ways: [ { "ctrlType": "pBend", "value_bits": 14, "range": [0, 16383] } ]`. Verdict: valid.
4. **A note control with values.** `{ "values": [ { "value": 0, "title": "off" }, { "value": 127, "title": "on" } ], "ways": [ { "ctrlType": "note", "ctrlIndex": [36] } ] }`. Verdict: valid.
5. **A message claimed twice in the same context.** Two controls with a `cc` `[7]` way and no `in`. Verdict: refused as `BadValue` at the later way, naming the earlier.
6. **The same message in two contexts.** The same two controls, with `in: { "part": "track1" }` and `in: { "part": "track2" }`, the device declaring `part`. Verdict: accepted.
7. **A 0.6 control converted.** The worked example above. Verdict: the converter writes the two ways shown; the reader accepts them.
8. **A partial overlap.** Two ways of one NRPN in one mode, slices `[0, 63]` and `[32, 95]`. Verdict: refused as `BadValue` at the later way.
9. **A note and a polyphonic pressure of the same number.** Verdict: accepted, two messages.
10. **A 7-bit CC against a 14-bit CC with a number in common.** A way `cc` `[39]` and a way `cc` `[7, 39]` with `value_bits: 14`. Verdict: refused as `BadValue` at the later way, naming the earlier.
11. **One way receives, the other sends.** The same CC `[7]`: a way with `recognize: none` and `transmit: absolute`, a way with `transmit: none` and `recognize: absolute`. Verdict: accepted.
12. **Two ways both sending the same message.** Two ways of the CC `[7]`, each with `recognize: none` and `transmit: absolute`. Verdict: refused as `BadValue` at the later way, on the sender side.
13. **A pressure way with an empty `ctrlIndex`.** `{ "ctrlType": "chPress", "ctrlIndex": [] }`. Verdict: refused as `BadValue` at `ctrlIndex`: the field is absent for `chPress` and `pBend`.
14. **A way with neither direction.** Two ways of the CC `[7]` with no `transmit` and no `recognize`. Verdict: refused as `BadValue` at the later way: each claims both sides.
15. **A reference scale.** A control with `default: 8192`, a 14-bit NRPN way and a 7-bit CC way. Verdict: valid; the reference is the NRPN (the most precise); the default is 8192 on the NRPN and 64 on the CC.
16. **A `params` entry.** `ways: [ { "ctrlType": "sysex", "ctrlIndex": [16, 2, 0, 0] } ]`. Verdict: valid; the same way on a `cc_params` entry is `BadValue`.

### The flat fields and `also` do not exist in 0.7

One form only. A 0.7 descriptor with `cc`, `nrpn_msb`, `also` and the others on an entry is refused at its field. In the same way `_sources` becomes `sources`, and a key with `_` stays a note for people only.

## What a reference does

**MIDI 2.0 Property Exchange**, controller resources (M2-117-UM v1.0). Each controller entry has `ctrlType` (`cc`, `chPress`, `pPress`, `nrpn`, `rpn`, `pBend`, and per-note kinds), `ctrlIndex` (an array of the numbers of the message), `channel`, `transmit` and `recognize`. A controller that both sends and recognises the same message may be listed twice for the same `ctrlType`, `ctrlIndex` and `channel`, one entry for what it sends, one for what it recognises (section 2.1.3): PE ties direction to an entry, not to the control. **MIDNAM** (`MIDINameDocument10.dtd`): a `Control` has `Type` (7bit, 14bit, RPN, NRPN) and `Number`: one message per control. Neither has a `value_bits` on a CC (Property Exchange has `numSigBits`, MIDNAM the type) nor a way of kind `sysex`: OSD's `value_bits` and `sysex` are the departure, written with their reason in "`value_bits`" and "The ways of a `params` entry" (compatibility, by the converter from 0.6, without loss). For `values`, Property Exchange's `ctrlMapId` list gives `value` and `title`; OSD takes both words.

## Tensions with the intentions of OSD

- **PE's words.** Kept for the kinds, for `ctrlType`, `ctrlIndex` (a list of integers, as in PE) and for `transmit` and `recognize`. `note` is OSD's, since PE has none (see "Why `note`").
- **The direction on a way.** PE ties direction to an entry, so two entries for one message that is both sent and recognised. This proposal keeps `transmit` and `recognize` on the way and departs from PE: a way that works both ways is one way, not two. The reason: one structure, where a way says its message, its direction and its context; two entries for one message would make two ways claim the same message. A tool that wants PE's form splits a way in two entries without loss; the reverse (merging two PE entries) is also exact when they have the same message.
- **One meaning per field.** `ways` holds structure in JSON objects the schema checks; no string is parsed. `ctrlIndex` is a list of integers whose order is fixed by the kind (MSB then LSB), as in PE. `slice` and `in` stay objects.
- **MIDNAM.** One message per control; here a control has several ways. The reason: a midi.guide line carries a CC and an NRPN for one control (0.6 already says it).

## Objections

None yet.

## The owner's answers

The owner answered on 2026-11-24, with five answers; no question remains open.

1. **The direction.** Option A: `transmit` and `recognize` stay on the way; the departure from Property Exchange is written with its reason (section "Tensions with the intentions of OSD").
2. **The partners' data.** No count is needed, since no field disappears with its information (section "The round trip with the partners' data").
3. **A way claims its message only on the side where it works** (section "The message claimed once, by side, with contexts").
4. **The word `note`** is the kind of a note message, MIDI's word.
5. **The names of a way's fields** are Property Exchange's: `ctrlType` for the kind and `ctrlIndex` for the numbers, a list of integers, not an object.

The owner decided two more points on 2026-11-24:

6. **The ways of a `params` entry.** A way of kind `sysex`: a `params` entry has ways like a control (section "The ways of a `params` entry").
7. **The reference scale.** A control may state its own scale; without it, the scale of its most precise way is the reference; `default`, `values`, `step_count` and readings are written in it and converted to each way's (section "The scale of a control").
