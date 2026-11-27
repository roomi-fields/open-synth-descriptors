# 0009 — A control reaches its messages by ways, each with its direction and its context

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

Amended on 2026-11-25 with the precisions the building of 0.7 brought to the scaling, the `sysex` way, the controls that mix kinds of message and the controls with no way, and the rewritten examples; the same day, the numbers of a `sysex` way are bounded by the length of their list, so that no `id` a published version admits is refused or lost. This change of a `candidate` text puts it back to `draft` ([PROCESS.md](../PROCESS.md), section 2): the delay of 14 days starts again the day it enters `candidate` again.

Amended on 2026-11-27: a `sysex` way gives its numbers as the fields `pr`, `p`, `c`, `r` and `id`, with no `ctrlIndex` and no note, so that no name is lost (a defect: `p` could be read as `pr`); a negative `id` is a number like the others.

Amended on 2026-11-27: `transmit` and `recognize` are said to be read from the point of view of the device or the software described, as in M2-117-UM (the owner's decision).

The owner answered the questions of the first draft on 2026-11-24; the answers are written into the text where they decide, and no question remains open (section "The owner's answers").

## What it changes

The text and the schemas of 0.7: how an entry of `cc_params` (and of `params`) says the messages that reach it. The flat fields of an entry (`cc`, `cc_lsb`, `nrpn_msb`, `nrpn_lsb`, `rpn_msb`, `rpn_lsb`, `value_bits`, `range`), `also`, and the `transmit` and `recognize` of the entry give way to one list, `ways`; only an entry with no way keeps `range`, `transmit`, `recognize` and `nrpn_lsb_range` on itself. Sections 22.3 and 22.6 of `spec/0.6.md` are rewritten on it.

## Why

What this brings to the sharing of sounds on devices: a control says in one structure every message that reaches it, in which direction and in which context, so a tool reads one form instead of three, and the devices 0.6 cannot describe become describable.

What is wrong without it. 0.6 says the link between a control and its messages three ways:

1. the flat fields of the entry give its first message;
2. `also` gives the next ones (`spec/0.6.md:1137`);
3. proposal 0003 (candidate) adds `in`, a context, on the entry.

And `transmit` and `recognize`, on the entry, say the direction and the coding of the whole control, though a control reached by a CC and an NRPN may send the one and only recognise the other. One thing has several forms. 0.6 also has no word for three kinds of message that real controls use: channel pressure, polyphonic pressure and pitch bend (the words `chPress`, `pPress`, `pBend` of the Property Exchange norm), and a note (a button, a pad), with named `values` for the states of a light.

## The change

### The form

A control (an entry of `cc_params`, of `params`) has `ways`, a list of zero or more ways (a control with none is partial, below). The flat fields, `also` and `in` are no longer fields of the entry: they live on the ways, as `range`, `transmit`, `recognize` and `nrpn_lsb_range` do on an entry that has a way. A way of a `cc_params` entry may carry `in` and `slice`; a way of a `params` entry does not, and a reader refuses them there as `UnknownField` at `/params/entries/N/ways/M/in` or `/slice` (cases `conformance/0.7/read/in-on-a-params-way-refused` and `conformance/0.7/read/slice-on-a-params-way-refused`). The entry keeps what is the control's own: its `osc` address, `role`, `default`, `priority`, `step_count`, `values`, `expects`, `section`, `transform`, its title and description, its `channel`, and its `scale` (below).

A way says one message and how it is reached:

| Field | Says |
|---|---|
| `ctrlType` | the message kind: `cc`, `nrpn`, `rpn`, `chPress`, `pPress`, `pBend`, `note`, `sysex` (for a `params` entry only) |
| `ctrlIndex` | the numbers of a message of MIDI, a list of integers, by kind (below); absent for `chPress`, `pBend` and `sysex` |
| `pr`, `p`, `c`, `r`, `id` | on a `sysex` way only: the numbers of the maker's frame, each under its name (below) |
| `value_bits` | the width of the value: 7 or 14; required with `nrpn` and `rpn`, and with a `cc` that has an LSB (14); optional with a simple `cc` (7, the width its kind implies) |
| `range` | lowest and highest value of the message, where needed |
| `transmit` | `absolute`, `relative`, `both`, `none` (the four words of M2-117-UM v1.0, tables 6 and 12) |
| `recognize` | the same four words |
| `in` | the context the way holds in (proposal 0003); on a way of a `cc_params` entry only: `UnknownField` on a way of a `params` entry (`0.7/read/in-on-a-params-way-refused`) |
| `slice` | the slice of values the way holds in (proposal 0003); on a way of a `cc_params` entry only: `UnknownField` on a way of a `params` entry (`0.7/read/slice-on-a-params-way-refused`) |
| `nrpn_lsb_range` | for an `nrpn` way only: `[lowest, highest]`, the parameter LSB numbers of the block the way stands for (section 22.4 of 0.6, unchanged); it is not the range of the value |

**A control with no way** (the owner's decision of 2026-11-25). A control may have `ways: []`: a partial control, to be completed. Its description knows the control and not yet the message that reaches it; the empty list says that, and nothing else. Such a control claims no message. Only it carries `range`, `transmit`, `recognize` and, in `cc_params`, `nrpn_lsb_range` on the entry; beside a way, a reader refuses each of them as `UnknownField`. A reader accepts a control with no way and warns `PartialControl` at its pointer; it is never a fault. A parameter that sends no message, such as a virtual parameter of a controller, is not such a control and stays out of the controls (proposal 0008). Cases of the suite: `conformance/0.7/read/control-without-a-way-warned`, `conformance/0.7/read/params-entry-without-a-way-warned`, `conformance/0.7/read/transmit-beside-a-way-refused` and `conformance/0.7/convert/entries-without-a-message-keep-their-fields`.

A departure from Property Exchange: it requires `ctrlType` on each controller (M2-117-UM), so it has no controller with no message. Reason: a description gathered from several sources can know a control before its message, and the owner decided that this partial knowledge is kept, to be completed.

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

For a control with no way, the reference scale is its `scale`, else its `range`; with neither, it is not known.

`default`, `values`, `step_count` and the readings of a control are written in the reference scale, and a tool converts them to the scale of each way when it needs the way's value. The conversion is the affine scaling of one range onto the other, rounded to the nearest, the half upward (away from `ref_min`), never truncated: `way = way_min + round((v − ref_min) × (way_max − way_min) / (ref_max − ref_min))`. For ranges that start at 0 it is `round(v × way_max / ref_max)`. With the reference `[-64, 63]` and a way `[0, 127]`: −64 gives 0, 0 gives 64, 63 gives 127. The reference value stays the value of the control: the less precise way carries its nearest approximation, and the most precise way carries it without loss. Example, with a 14-bit reference (0 to 16383): a `default` of 8192 becomes 64 on a 7-bit way (8192 × 127 / 16383 ≈ 63.5, rounded up), and stays 8192 on the 14-bit way. So no information that a way can carry is lost.

The same formula holds in the other direction, the two ranges exchanged: a value received on a way is scaled to the reference scale in the same manner.

`ctrlIndex` by `ctrlType`. When there are two integers, the order is always MSB then LSB.

| `ctrlType` | `ctrlIndex` | Notes |
|---|---|---|
| `cc` | `[msb]` (7 bits), or `[msb, lsb]` (14 bits, with `value_bits: 14`) | the second is a 14-bit CC (`cc` and `cc_lsb` of 0.6) |
| `nrpn` | `[msb, lsb]` | `nrpn_msb`, `nrpn_lsb` of 0.6 |
| `rpn` | `[msb, lsb]` | `rpn_msb`, `rpn_lsb` of 0.6 |
| `pPress` | `[note]` | the note number, 0 to 127 |
| `note` | `[note]` | the note number, 0 to 127 |
| `chPress`, `pBend` | none: the field is absent, never an empty list | one message per channel |
| `sysex` | none: its numbers are the fields `pr`, `p`, `c`, `r` and `id` of the way (below) | a `params` entry only (below) |

`nrpn_lsb_range` claims, as in 0.6, only the message of its own `ctrlIndex`, not the whole block: a reader MUST NOT claim the LSB numbers of the block. A reader MUST keep it as written and write it back.

### The ways of a `params` entry

A `params` entry (a parameter read and written by SysEx) has `ways` like a control. Its way is of kind `sysex`: it gives each number that addresses the parameter in the maker's frame as a field under its name, from the fields of the 0.6 entry: `pr`, `p`, `c`, `r` (each one byte, 0 to 255) and `id` (any integer, negative too, as 0.6 publishes it, `spec/0.6.md`, table of words), only those the entry gives, and at least one. It has no `ctrlIndex`. A reader refuses a `sysex` way with none of the five numbers as `BadValue` at the way, a `ctrlIndex` on a `sysex` way as `UnknownField` at `ctrlIndex`, a byte outside 0 to 255 or a number that is not an integer as `BadValue` at the field, and any of the five on a way of a kind of MIDI as `UnknownField` at the field. Of several of these faults it reports the first in this order: a kind its table does not take, a field its kind does not hold, a number of a wrong value, none of the five numbers. A `sysex` way carries `range`, `transmit` and `recognize`. It carries no context: a reader refuses `in` and `slice` on a way of a `params` entry as `UnknownField` at `/params/entries/N/ways/M/in` or `/slice` (cases `conformance/0.7/read/in-on-a-params-way-refused` and `conformance/0.7/read/slice-on-a-params-way-refused`). A 0.6 `params` entry has no `value_bits`, and the converter writes none on a `sysex` way. The entry keeps `osc`, `role`, `default`, `values`, `expects`, `section`, `transform`, `priority` and `step_count`. The kinds of MIDI (`cc`, `nrpn`, `rpn`, `chPress`, `pPress`, `pBend`, `note`) are not those of a `params` way, and `sysex` is not that of a `cc_params` way.

The converter writes, for a 0.6 `params` entry: one way, `ctrlType: sysex`, with each of `pr`, `p`, `c`, `r` and `id` that the entry gives under its name, the entry's `range` on the way, its `transmit` and `recognize` on the way; the rest stays on the entry. A 0.6 `params` entry with none of `pr`, `p`, `c`, `r` and `id` gets `ways: []` and keeps its `range`, `transmit` and `recognize` on the entry. A 0.6 text read and written again in 0.6 is unchanged. Cases of the suite: `conformance/0.7/read/sysex-way-with-id-beyond-a-byte-accepted`, `conformance/0.7/read/sysex-way-with-negative-id-accepted`, `conformance/0.7/read/sysex-way-byte-above-255-refused`, `conformance/0.7/read/sysex-way-byte-below-0-refused`, `conformance/0.7/read/sysex-way-byte-not-an-integer-refused`, `conformance/0.7/read/sysex-way-id-not-an-integer-refused`, `conformance/0.7/read/sysex-way-without-a-number-refused`, `conformance/0.7/read/ctrlindex-on-a-sysex-way-refused`, `conformance/0.7/read/sysex-number-on-a-way-of-midi-refused`, `conformance/0.7/convert/params-p-c-r-keep-their-names`, `conformance/0.7/convert/params-pr-alone-keeps-its-name`, `conformance/0.7/convert/params-id-beside-pr-keeps-both`, `conformance/0.7/convert/params-id-beyond-a-byte-becomes-the-id-of-the-way` and `conformance/0.7/convert/params-negative-id-alone-becomes-the-id-of-the-way`.

What the mature references do: Property Exchange controller resources describe MIDI controller messages and have no kind for a SysEx parameter; MIDNAM neither. `sysex` is an OSD word, and the departure has its reason: a SysEx parameter is a control of the device, and putting it in a way lets one structure carry every control; the converter from 0.6 writes it without loss.

The names of a way's fields are Property Exchange's where it has them: `ctrlType` for the kind and, for the kinds of MIDI, `ctrlIndex` for the numbers, a list of integers as in PE (one number for a CC or a note, two for an NRPN or an RPN). The kinds are PE's `ctrlType` words where PE has them (`cc`, `nrpn`, `rpn`, `chPress`, `pPress`, `pBend`); `note` is OSD's. PE has no SysEx kind, so `ctrlIndex` has no model there: a `sysex` way names its numbers `pr`, `p`, `c`, `r` and `id`, the names of 0.6 and of the frames of `params`, one meaning per field.

Two ways of a control may differ in kind, numbers, width, range, direction and context: each is the way's own.

### Why `note`

PE has no kind for a note, since its controllers are continuous (M2-117-UM). The kind is named `note`: the word is MIDI's, and it means a note on / note off message of a given number (and its channel). A button or a pad that sends a note is a control reached by a `note` way. `note` and the `values` of its control (the states of a light: a value of the velocity and its title) are general: no field and no key of them belongs to a product, and the form of a partner's device is taken in only as entries of `sources` (the source is credited by an entry of the sources, never by the shape of a field).

A note number as the message of a control (`note`) and the same number as a polyphonic pressure (`pPress`) are two messages: their kinds differ, so they are not claimed against each other (see the rule below).

### The order of the ways, and what a tool that sends and a tool that listens do

This is 0.6's rule of `also`, said on the list.

- The ways are in the order written. A reader MUST NOT reorder them.
- `transmit` and `recognize` are read from the point of view of the device or the software the descriptor describes, as the controller list of M2-117-UM v1.0 (tables 6 and 12) says them of a device: `transmit` other than `none`, it sends the message; `recognize` other than `none`, it receives it.
- A **tool that sends** to the device takes the first way it can write: the first whose `recognize` is not `none`, whose kind and width it can write, and whose context holds (see the rule).
- A **tool that listens** to the device recognises every way whose `transmit` is not `none` as a message of the same control.
- A way with `recognize: none` is still listened for if its `transmit` allows it; the tool that sends skips it and takes the next.
- A way with `transmit: none` is sent to the device when its `recognize` allows it; the tool that listens does not expect its message from the device.
- A way with both at `none` says a message the device neither sends nor takes for this control: a reader keeps it, a tool ignores it, and it claims nothing. The descriptor may name it for a source that lists it.
- A way with **neither** `transmit` nor `recognize` written says nothing of its direction (as a midi.guide line does): it is both a sent and a recognised message, and **claims on both sides**. Only an explicit `none` removes a side. The absence is not `none`.

`relative` is carried on the way by `transmit` and `recognize` (`relative` or `both`) and says nothing more than in `spec/0.6.md` section 22.3: the norm, not the device, says how a relative value is coded.

### The message claimed once, by side, with contexts

0.6 says a message is claimed once (`spec/0.6.md:1145`). Proposal 0003 says: once by claims that can hold together. Here, written on the ways:

- A **claim** is a way on one side: its message (`ctrlType`, channel, `ctrlIndex`) with its coordinates (the sets of `in`, and the interval of `slice`).
- A way claims its message **on each side it works**: the side where the device receives it when `recognize` is not `none`, the side where it sends it when `transmit` is not `none`. A way with both at `none` claims nothing.
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
- Accepted, one way receives and the other sends the same CC: way 1 `{ "ctrlType": "cc", "ctrlIndex": [7], "transmit": "absolute", "recognize": "none" }`, way 2 `{ "ctrlType": "cc", "ctrlIndex": [7], "transmit": "none", "recognize": "absolute" }`. Way 1 claims the side where the device sends, way 2 the side where it receives.
- Refused on the sender side: two ways `{ "ctrlType": "cc", "ctrlIndex": [7], "transmit": "absolute", "recognize": "none" }`. Both claim the CC 7 on the side where the device sends.

The order of the ways stays the rule of the tool that sends: it takes the first way it can send whose context holds. A way whose context does not hold at the moment is skipped; the next is tried.

`note` number 60 and `pPress` number 60 on one channel: different kinds, two messages, no claim between them.

### What the converter writes

For a 0.6 control the converter writes the following, losing no value: a block (`nrpn_lsb_range`) that no single `nrpn` way can receive, and a `range` that no way takes, are each kept as a note.

| 0.6 | 0.7 |
|---|---|
| `cc` | first way, `ctrlType: cc`, `ctrlIndex: [cc]` |
| `cc_lsb` | the same way, `ctrlIndex: [cc, cc_lsb]` (MSB then LSB); `value_bits: 14` |
| `nrpn_msb`, `nrpn_lsb` | first way, `ctrlType: nrpn`, `ctrlIndex: [nrpn_msb, nrpn_lsb]` |
| `rpn_msb`, `rpn_lsb` | first way, `ctrlType: rpn`, `ctrlIndex: [rpn_msb, rpn_lsb]` |
| `value_bits` (NRPN, RPN) | the way's `value_bits`; a simple CC has none in 0.6 and the converter writes none |
| `range`, on an entry that gives a flat message | the first way's `range` |
| `range`, on an entry that gives no flat message and a non-empty `also` | the note `_range` of the entry (below); when the entry has no way, its `range` stays on it |
| `nrpn_lsb_range`, on an entry that has a way | the `nrpn_lsb_range` of one `nrpn` way (below); when no single way can receive it, the note `_nrpn_lsb_range` of the entry |
| each element of `also`, in order | the next ways, same rules, its own `range` and `value_bits` |
| `transmit`, `recognize` of the entry | written on every way (0.6 has one value for the control) |
| `values` (a dictionary label → integer) | the entry's `values`, a list of `{ "value": integer, "title": label }` in the order of the dictionary, as Property Exchange's `ctrlMapId` list gives `value` and `title`; the integers are in the reference scale (below) |
| `expects`, `section`, `transform`, `role`, `default`, `priority`, `step_count` | the same fields of the entry, unchanged |
| a `params` entry's `pr`, `p`, `c`, `r`, `id`, `range`, `transmit`, `recognize` | one way of kind `sysex`, each number under its name (above) |
| the scale | the range of the first way, written as `scale` on the control only if the control has a `default`, `values` or `step_count` and its first way is not the most precise; for a `range` that no way takes, that `range` (below) |
| an entry that gives no message fully known | `ways: []`; its `range`, `transmit`, `recognize` and `nrpn_lsb_range` stay on the entry, unchanged, with no `scale` |
| an NRPN or an RPN given by one number, a `cc_lsb` without `cc` | no way: the note `_incomplete` of the entry (below) |

**The block the converter writes.** An entry of `cc_params` that gives `nrpn_lsb_range` and has a way in 0.7 has it written on one `nrpn` way and on no other: the way of the flat fields when they give both `nrpn_msb` and `nrpn_lsb`; else, when they give neither, the single `nrpn` way written from `also`. When no way can receive it (the flat fields give one number of an NRPN alone; or they give none, and `also` gives no `nrpn` way or several), the converter does not refuse the entry for it and chooses no way: it keeps the value, unchanged, on the entry under `_nrpn_lsb_range`, a note for people; when that key is taken, under `_nrpn_lsb_range_2`, `_nrpn_lsb_range_3` and so on, the first free. A control of 0.4 or 0.5 that mixes kinds of message follows its own rules (below). Cases of the suite, under `conformance/0.7/convert/`: `nrpn-block-goes-to-the-only-nrpn-way-of-also`, `nrpn-block-beside-a-cc-of-0-4-kept-as-note`, `nrpn-block-beside-a-cc-of-0-5-kept-as-note`, `nrpn-block-beside-a-cc-of-0-6-kept-as-note`, `nrpn-block-beside-also-without-nrpn-kept-as-note`, `nrpn-block-beside-two-nrpn-ways-of-also-kept-as-note` and `nrpn-block-note-takes-the-next-free-key`.

**The range no way takes.** An entry of `cc_params` that gives `range`, no flat message field and a non-empty `also`, and has a way in 0.7, has a `range` that no way takes without its being moved, and an entry that has a way carries none. The converter does not refuse the entry for it and writes it on no way: it keeps the value, unchanged, on the entry under `_range`, a note for people; when that key is taken, under `_range_2`, `_range_3` and so on, the first free. A way of such an entry has a `range` only when its element of `also` gave one. When the entry has no way, because no element of its `also` is a message fully known, its `range` stays on it, with no `_range` and no `scale`. Cases of the suite, under `conformance/0.7/convert/`: `range-beside-also-alone-stays-a-note` and `range-beside-also-alone-gives-the-scale-of-a-default`.

**The scale the converter writes.** The scale of a 0.6 control is the range of its input, that is, of its first way (0.6 says `default` "in the space of `range`", section 22.1). The converter writes it as `scale` on the control only when the control has a `default`, `values` or `step_count` and its first way is not the most precise (another way has a higher maximum). Otherwise the first way is the reference already, and nothing is written. For an entry whose `range` no way takes (above), the scale of its input is that `range`: when the control has a `default`, `values` or `step_count`, the converter writes `scale`, equal to the `range`, if the most precise way has another scale, whatever the scale of the first way, and no `scale` otherwise; without these three fields it writes no `scale`, and the reference scale is that of the ways. So each 0.6 value keeps its value. A simple CC without an explicit `value_bits` takes 7 bits and the range 0 to 127 that its kind implies.

**Why the 0.6 `value_bits` rule and the `sysex` way depart from the references.** See "What a reference does": Property Exchange and MIDNAM have neither a `value_bits` on a CC nor a way of kind `sysex`. Both are OSD's words, kept for compatibility: the converter from 0.6 writes them without loss.

A converter MUST keep the order the source says: the flat fields, then `also`. A converter MUST NOT join two controls. A converter writes no value that a later version adds, and no value its source lacks: a way gets `transmit` or `recognize` only if the control had it.

**A message not fully known** (the owner's decision of 2026-11-25: partial information is kept, to be completed). An NRPN or an RPN given by one number, and a `cc_lsb` without its `cc`, have no way: `ctrlIndex` of an NRPN or an RPN holds two numbers, as in Property Exchange, and `[n]` of a `cc` is the CC n. The converter writes no way and no number for such a message and refuses nothing for it. It keeps it in the note `_incomplete` of the entry (`_incomplete_2`, `_incomplete_3`… when taken), a note for people: a list of one object per message, the flat fields then `also`, with the numbers given, `value_bits` when the text gives it, and for an element of `also` its `range` and its `_` notes. The `range` of the flat fields goes there only when the entry keeps a way; the `nrpn_lsb_range` beside an incomplete flat NRPN goes on no way (`_nrpn_lsb_range` when the entry has ways, else on the entry). When the first message is the incomplete one, a way remains and the control has a `default`, `values` or `step_count`, `scale` keeps the scale of that first message if the ways would give another. A control left with no way is partial, warned `PartialControl`.

#### Worked example

A control of 0.6, a CC and an NRPN for the same setting, in the order of preference, absolute to send and any coding to recognise:

```json
{ "cc_params": { "entries": [
  { "osc": "/filter/cutoff", "cc": 74, "range": [0, 127], "transmit": "absolute", "recognize": "both",
    "also": [ { "nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "range": [0, 16383] } ] } ] } }
```

becomes, with every field in its place (`cc` in way 1's `ctrlIndex`, `range` in way 1, `also` in way 2, `nrpn_msb` and `nrpn_lsb` in way 2's `ctrlIndex`, `value_bits` and `range` in way 2, `transmit` and `recognize` on both):

```json
{ "cc_params": { "entries": [
  { "osc": "/filter/cutoff", "ways": [
    { "ctrlType": "cc",   "ctrlIndex": [74], "range": [0, 127],
      "transmit": "absolute", "recognize": "both" },
    { "ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14, "range": [0, 16383],
      "transmit": "absolute", "recognize": "both" } ] } ] } }
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

The device receives the NRPN (way 1) and does not send it; it sends the CC (way 2) and does not receive it. A tool that sends to the device takes way 1; a tool that listens to it recognises the CC. Each way kept its own two values; nothing was merged.

#### A control of 0.4 or 0.5 that mixes kinds of message

The flat fields of a control of 0.4 or 0.5 may mix a CC, an NRPN and an RPN, which 0.6 refuses. In 0.7 each message has its way, so the converter to 0.7 turns such a control of `cc_params` into one control with one way per message:

- **One way per kind, in a fixed order**: the CC, the NRPN, the RPN, whatever the order of the text. "A converter MUST keep the order" is about the order the source says (the flat fields, then `also`); the flat fields of a mix say none.
- **Numbers and width.** The CC way has `ctrlIndex` `[cc]`, or `[cc, cc_lsb]` with `value_bits: 14`; an NRPN or an RPN way has `ctrlIndex` `[msb, lsb]` and `value_bits: 14`, the only coding of 0.4 and 0.5. A number 0 is a number of the source and is kept. A message of the mix not fully known goes into the note `_incomplete` (above), with the control's `range` when the control keeps a way.
- **The other fields.** The control's `range` is written on every way, its `nrpn_lsb_range` on the NRPN way alone. The converter writes no `transmit`, no `recognize`, no `scale` and no note key for the mix, but for `_incomplete` and `_nrpn_lsb_range`; the other fields of the control convert as those of any control.

The converter refuses, as `NotConvertible`, and returns no text for:

- at the control (`/cc_params/entries/N`), a mix with an `nrpn_lsb_range` and no NRPN, neither `nrpn_msb` nor `nrpn_lsb`: no way holds it;
- a message claimed twice, by a control that mixes kinds and another control, on the same channel (a 14-bit CC claiming its CC number and its LSB number): at the later control, the rule naming the first one;
- a control of a fingerprint (`controls`, `names[].control`) that mixes kinds of message, at the control (`/controls/N`, `/names/N/control`): it is reached by no way.

The conversion to 0.6 still refuses a control that mixes kinds of message: 0.6 does not change.

Cases of the suite, under `conformance/0.7/convert/`: `mixed-cc-and-nrpn-control-of-0-4-becomes-two-ways`, `mixed-cc-and-nrpn-control-of-0-5-becomes-two-ways`, `mixed-cc-and-rpn-control-becomes-two-ways`, `mixed-nrpn-and-rpn-control-becomes-two-ways`, `mixed-three-kinds-control-becomes-three-ways`, `mixed-control-with-nrpn-of-one-number-kept-as-note`, `mixed-control-with-cc-lsb-without-cc-kept-as-note`, `mixed-control-with-block-without-nrpn-refused`, `mixed-control-message-claimed-twice-refused`, `fingerprint-control-mixing-cc-and-nrpn-refused` and `fingerprint-name-control-mixing-cc-and-nrpn-refused`.

### The round trip with the partners' data: no count is needed

A field or a form that disappears with its information would call for a count of what the partners' data (Electra One, midi.guide, KnobKraft Orm) holds in it. None disappears here: the flat fields, `also`, `transmit` and `recognize` change form without loss, the converter carrying each value into a way (table above), but for a block (`nrpn_lsb_range`) that no single `nrpn` way can receive, which it keeps, unchanged, on the entry in the note `_nrpn_lsb_range` ("The block the converter writes"), and for a `range` that no way takes, which it keeps, unchanged, on the entry in the note `_range` ("The range no way takes"). So no count is needed, and the examples below show that the round trip stays exact.

**A midi.guide line with a CC and an NRPN.** midi.guide carries no direction: a line gives a name, a CC and an NRPN. In 0.6, whose flat fields never mix two kinds, it is a CC and an NRPN in `also`, the NRPN with its `value_bits`:

```json
{ "cc_params": { "entries": [
  { "osc": "/filter/cutoff", "cc": 74,
    "also": [ { "nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14 } ] } ] } }
```

The two ways written from it have no `transmit` and no `recognize`, since the source carries none and the converter writes no value the source lacks:

```json
{ "cc_params": { "entries": [
  { "osc": "/filter/cutoff", "ways": [
    { "ctrlType": "cc",   "ctrlIndex": [74] },
    { "ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14 } ] } ] } }
```

The simple CC has no `value_bits`; the NRPN way keeps the one its entry gave. A 0.4 or 0.5 text may give the same line in its flat fields (`cc`, `nrpn_msb`, `nrpn_lsb`): the converter writes the same two ways, the NRPN way with `value_bits: 14`, the only coding of 0.4 and 0.5. Case of the suite: `conformance/0.7/convert/midi-guide-line-becomes-two-ways-without-direction`.

Written back to a midi.guide line, way 1 gives `cc` 74 and way 2 gives the NRPN 1:8: the line is found again, with nothing added and nothing lost.

**An Electra One control in the relative mode.** The mode is the coding of the way, as in 0.6, and is kept without loss. In 0.6, with its source:

```json
{ "_sources": [ { "type": "electra-preset", "notes": "control in the relative mode" } ],
  "cc_params": { "entries": [
    { "osc": "/encoder/1", "cc": 20, "transmit": "relative", "recognize": "relative" } ] } }
```

The converter writes the source as proposal 0010 says (`contributed`, the partner named by `provider`; no field has the product's name) and the control with one way:

```json
{ "sources": [ { "type": "contributed", "provider": "electra-one",
    "notes": "type in 0.6: electra-preset\ncontrol in the relative mode" } ],
  "cc_params": { "entries": [
    { "osc": "/encoder/1", "ways": [
      { "ctrlType": "cc", "ctrlIndex": [20],
        "transmit": "relative", "recognize": "relative" } ] } ] } }
```

One way, with `transmit: relative` and `recognize: relative`, and no `value_bits`, since the simple CC had none. Written back to a control of the source, the way gives its CC and its relative mode, and read again it is found unchanged. Case of the suite: `conformance/0.7/convert/relative-control-keeps-its-coding-on-its-way`.

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
12. **Two ways both sending the same message.** Two ways of the CC `[7]`, each with `recognize: none` and `transmit: absolute`. Verdict: refused as `BadValue` at the later way, on the side where the device sends.
13. **A pressure way with an empty `ctrlIndex`.** `{ "ctrlType": "chPress", "ctrlIndex": [] }`. Verdict: refused as `BadValue` at `ctrlIndex`: the field is absent for `chPress` and `pBend`.
14. **A way with neither direction.** Two ways of the CC `[7]` with no `transmit` and no `recognize`. Verdict: refused as `BadValue` at the later way: each claims both sides.
15. **A reference scale.** A control with `default: 8192`, a 14-bit NRPN way and a 7-bit CC way. Verdict: valid; the reference is the NRPN (the most precise); the default is 8192 on the NRPN and 64 on the CC.
16. **A `params` entry.** `ways: [ { "ctrlType": "sysex", "pr": 16, "p": 2, "c": 0, "r": 0 } ]`. Verdict: valid; the same way on a `cc_params` entry is `BadValue`. In the suite: the conversion of a 0.6 `params` entry is `conformance/0.7/convert/params-entry-becomes-a-sysex-way`.
17. **A control with no way.** `ways: []`. Verdict: valid, with the warning `PartialControl` at the entry (`conformance/0.7/read/control-without-a-way-warned`).

### The flat fields and `also` do not exist in 0.7

One form only. A 0.7 descriptor with `cc`, `nrpn_msb`, `also` and the others on an entry is refused at its field, as are `range`, `transmit`, `recognize` and `nrpn_lsb_range` on an entry that has a way. In the same way `_sources` becomes `sources`, and a key with `_` stays a note for people only.

## What a reference does

**MIDI 2.0 Property Exchange**, controller resources (M2-117-UM v1.0). Each controller entry has `ctrlType` (`cc`, `chPress`, `pPress`, `nrpn`, `rpn`, `pBend`, and per-note kinds), `ctrlIndex` (an array of the numbers of the message), `channel`, `transmit` and `recognize`. A controller that both sends and recognises the same message may be listed twice for the same `ctrlType`, `ctrlIndex` and `channel`, one entry for what it sends, one for what it recognises (section 2.1.3): PE ties direction to an entry, not to the control. **MIDNAM** (`MIDINameDocument10.dtd`): a `Control` has `Type` (7bit, 14bit, RPN, NRPN) and `Number`: one message per control. Neither has a `value_bits` on a CC (Property Exchange has `numSigBits`, MIDNAM the type) nor a way of kind `sysex`: OSD's `value_bits` and `sysex` are the departure, written with their reason in "`value_bits`" and "The ways of a `params` entry" (compatibility, by the converter from 0.6, without loss). For `values`, Property Exchange's `ctrlMapId` list gives `value` and `title`; OSD takes both words.

## Tensions with the intentions of OSD

- **PE's words.** Kept for the kinds, for `ctrlType`, `ctrlIndex` (a list of integers, as in PE) and for `transmit` and `recognize`. `note` is OSD's, since PE has none (see "Why `note`").
- **The direction on a way.** PE ties direction to an entry, so two entries for one message that is both sent and recognised. This proposal keeps `transmit` and `recognize` on the way and departs from PE: a way that works both ways is one way, not two. The reason: one structure, where a way says its message, its direction and its context; two entries for one message would make two ways claim the same message. A tool that wants PE's form splits a way in two entries without loss; the reverse (merging two PE entries) is also exact when they have the same message.
- **One meaning per field.** `ways` holds structure in JSON objects the schema checks; no string is parsed. `ctrlIndex` is a list of integers whose order is fixed by the kind (MSB then LSB), as in PE. `slice` and `in` stay objects.
- **A control with no way.** Property Exchange requires `ctrlType` on each controller; here a control may have `ways: []`, for the reason written in "The form".
- **MIDNAM.** One message per control; here a control has several ways. The reason: a midi.guide line carries a CC and an NRPN for one control (0.6 already says it).

## Objections

None yet.

## The owner's answers

The owner answered on 2026-11-24, with five answers; no question remains open.

1. **The direction.** Option A: `transmit` and `recognize` stay on the way; the departure from Property Exchange is written with its reason (section "Tensions with the intentions of OSD").
2. **The partners' data.** No count is needed, since no field disappears with its information (section "The round trip with the partners' data").
3. **A way claims its message only on the side where it works** (section "The message claimed once, by side, with contexts").
4. **The word `note`** is the kind of a note message, MIDI's word.
5. **The names of a way's fields** are Property Exchange's: `ctrlType` for the kind and `ctrlIndex` for the numbers, a list of integers, not an object. A `sysex` way, which PE does not have, names its numbers `pr`, `p`, `c`, `r` and `id` (amendment of 2026-11-27).

The owner decided two more points on 2026-11-24:

6. **The ways of a `params` entry.** A way of kind `sysex`: a `params` entry has ways like a control (section "The ways of a `params` entry").
7. **The reference scale.** A control may state its own scale; without it, the scale of its most precise way is the reference; `default`, `values`, `step_count` and readings are written in it and converted to each way's (section "The scale of a control").

The owner decided one more point on 2026-11-25:

8. **A control with no message** is kept, with `ways: []`, partial and to be completed (section "The form").
9. **A message not fully known** has no way and is kept in the note `_incomplete` (section "What the converter writes").
