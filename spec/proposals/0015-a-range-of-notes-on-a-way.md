# 0015 — A range of notes on a way

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: the ways of section 5.3.3 of 0.7 (a way of kind `note` may give a range of notes instead of one number), and the rule that a message is claimed once (one exception: a note written alone inside a range).

## Why

What this brings to the sharing of sounds on devices: a software or a device that answers a range of notes (otis answers the notes 48 to 51) is said in one way, and one note of the range may still be written alone, with fields of its own.

What is wrong without it. A way of kind `note` of 0.7 gives one number, so a range is written as one control per note, or as one control with one way per note. Neither says that the notes are one thing, and the first repeats every field of the control for each note.

## The change

Everything of 0.7 that this section does not name stays as it is. In particular, `triggers.note_range` keeps its form and its meaning.

### The field

A way of an entry of `cc_params` whose `ctrlType` is `note` gives its notes in one of two forms:

| Field | Says |
|---|---|
| `ctrlIndex` | `[note]`, one note, as in 0.7 |
| `note_range` | `[low, high]`, two integers from 0 to 127, `low` at most `high`: every note from `low` to `high`, bounds included |

A way gives exactly one of the two. The name and the form are those of `triggers.note_range`. `low` may equal `high`: the way then covers one note, as `lokey` equal to `hikey` does in SFZ.

In this proposal, a way writes a **note alone** when it gives `ctrlIndex: [n]`, or `note_range: [n, n]`. It writes a **range** when it gives `note_range: [low, high]` with `low` below `high`. The two forms of a note alone mean the same, and every rule below treats them alike.

```json
{ "ctrlType": "note", "note_range": [48, 51] }
```

### The schema

In `$defs/cc_way` of the device schema of 0.8, `properties` gains:

```json
"note_range": {
  "type": "array",
  "items": { "type": "integer", "minimum": 0, "maximum": 127 },
  "minItems": 2,
  "maxItems": 2,
  "description": "For a note way, in place of ctrlIndex: the lowest and highest note the way covers, bounds included, lowest at most highest (the reader refuses the reverse)."
}
```

The first condition of `allOf` in `cc_way` (`ctrlIndex` forbidden for `chPress` and `pBend`, required for every other kind) is replaced by these four. The other conditions stay.

```json
[
  {
    "if": { "required": ["ctrlType"], "properties": { "ctrlType": { "enum": ["chPress", "pBend"] } } },
    "then": { "properties": { "ctrlIndex": false } }
  },
  {
    "if": { "required": ["ctrlType"], "properties": { "ctrlType": { "enum": ["cc", "nrpn", "rpn", "pPress"] } } },
    "then": { "required": ["ctrlIndex"] }
  },
  {
    "if": { "required": ["ctrlType"], "properties": { "ctrlType": { "const": "note" } } },
    "then": {
      "if": { "required": ["ctrlIndex"] },
      "then": { "properties": { "note_range": false } },
      "else": {
        "if": { "required": ["note_range"] },
        "else": { "required": ["ctrlIndex"] }
      }
    }
  },
  {
    "if": { "required": ["ctrlType"], "properties": { "ctrlType": { "not": { "const": "note" } } } },
    "then": { "properties": { "note_range": false } }
  }
]
```

`sysex_way`, the way of a `params` entry, does not gain the field.

### What a reader refuses

`W` stands for `/cc_params/entries/N/ways/M`.

| Fault | At | Kind | Seen by |
|---|---|---|---|
| a `note_range` that is not a list of two (`48`, `[48]`, `[48, 49, 50]`, `null`) | `W/note_range` | `BadValue` | the schema and the reader |
| a bound that is not an integer (`[48, "51"]`, `[48, 50.5]`) | `W/note_range/K`, `K` 0 or 1 | `BadValue` | the schema and the reader |
| a bound outside 0 to 127 | `W/note_range/K` | `BadValue` | the schema and the reader |
| `low` above `high` (`[51, 48]`) | `W/note_range` | `BadValue` | the reader alone: the schema cannot compare two members |
| `note_range` and `ctrlIndex` on one `note` way | `W/note_range` | `BadValue` | the schema and the reader |
| a `note` way with neither | `W/ctrlIndex` | `Missing`, as in 0.7 | the schema (at the way `W`, whose required member is absent) and the reader |
| `note_range` on a way of `cc_params` of another `ctrlType` | `W/note_range` | `BadValue` | the schema and the reader |
| `note_range` on a way of a `params` entry, a `sysex` way included | `/params/entries/N/ways/M/note_range` | `UnknownField` | the schema (at the way, which allows no other member) and the reader |
| `note_range` on a way in a descriptor of 0.7 | `W/note_range`, or the same place under `/params` | `UnknownField` | the schema of 0.7 (at the way) and the reader |
| `note_range` on a control of 0.4 to 0.6, which has no ways | `/cc_params/entries/N/note_range`, or `/cc_params/entries/N/also/K/note_range` | `UnknownField` | the schema of the version and the reader |

When one way holds several of these faults, a reader reports the first in this order: a field its version or its table does not define; `note_range` on another `ctrlType`; `note_range` beside `ctrlIndex`; the form of the list; the bounds; their order. A descriptor of 0.4 to 0.6 that holds `ways` is refused at `ways`, as today. A file with `midi_of` is judged on its own text, in its own version, before it is joined, as for any field.

### What a way claims

The way of a range claims each of its notes, bounds included, as the message of a note alone is claimed: on each side where it works (`transmit` and `recognize`, as in 5.3.3), on the channel of its control, else of its table, else on no channel, with its coordinates `in` and `slice`.

The rule of 5.3.3 stays, with the one exception that this proposal adds. Two claims of one note meet when they are on one same side, on the same channel (or both on none), and their contexts can hold together (the one rule of claims). Then:

| The two ways that meet | Verdict |
|---|---|
| two ranges, sharing one note or more, one inside the other included | refused |
| two notes alone of one same number, inside a range or not | refused, as in 0.7 |
| a note alone and a range that holds it | **admitted**, whichever is written first, in one control or in two |

A reader MUST refuse, as `BadValue` at the later way (`/cc_params/entries/N/ways/M`), a way whose claim is refused against an earlier one, and the rule MUST name the earlier way by the same form of pointer; when several earlier ways refuse it, the first of them in the written order. The refusal holds across the ways of one control as across the controls. The reader alone sees it: the schema accepts the descriptor.

The messages that were apart stay apart: a `note` and a `pPress` of one number are two messages; ways on different channels, or one on a channel and one on none, claim two messages; ways with no side in common, or whose contexts cannot hold together, do not meet.

### A note written alone inside a range

The note alone is an entry of the descriptor or a way of its own, with its own fields. It is not a correction of the range and it completes nothing in it: both claim the note, and both apply.

For a note `n`, on each side, channel and context where a note alone and a range both claim it, the message reaches **both**. A reader MUST give the two in this order: the way of the note alone, then the way of the range. What the note alone writes is added to what the range writes, as two regions of SFZ that cover one key, or two zones of SoundFont, each apply. Everywhere else (another side, another channel, a context where one of the two does not hold), only the way that claims the note there is reached.

- When the two ways belong to two controls, the two controls are reached, each with its own fields and its own value, read from the message by its own way and written on its own reference scale.
- When the two ways belong to one control, the control is reached once, and the value of that note is read by the way of the note alone: its `range`, `transmit`, `recognize`, `in` and `slice`.

**No field passes from one entry or one way to the other.** `channel`, `ctrlType`, `ctrlIndex`, `note_range`, `in`, `slice`, `transmit`, `recognize`, `value_bits`, `range`, `role`, `title`, `description`, `values`, `scale`, `transform`, `default` and every other field stay where they are written. A control that does not write a field does not take it from the other. Each control keeps the same fields for all its messages, whatever note, side or context reaches it.

### Each control and way judged on its own fields

A reader judges each control and each way on its own fields. Two values that the note alone and the range write differently are no fault: a control with a range way and `scale: [0, 1]`, and another control with the note alone inside it and `default: 100`, each valid by itself, are both read. The faults of 0.7 keep their kind and their address (a `default` outside the scale of its own control is still `BadValue` at that `default`). A reader gives the descriptor and writes it back as written: it rewrites no `note_range: [n, n]` into `ctrlIndex: [n]`, copies no field and merges no entry.

**A file with `midi_of`.** Read alone, the file is judged on its own entries only, as in 0.7: a note alone of the file that lies inside a range of its target is read, since the range is not in the file; a coordinate of a dimension the file does not know separates nothing, and an entry with no channel in the file has none. Joined to its target (section 23), the descriptor holds the entries of both, a route of the file replacing the route of the target at the same address, and the claims are those of the joined descriptor, with its contexts and the channel of its tables. So a note alone of the file inside a range of the target is admitted and reaches both; a range of the file that shares a note with a range of the target, on a side and channel of both, in contexts that can hold together, is refused at the join as above, though each file was read alone. A join may also separate two ways that met alone, or bring together two ways that were apart: the joined verdict is that of the resolution.

### What a receiver and a sender do

**A receiver** recognises every note of the range, on the sides, channel and contexts of the way, as a message of the control. The value of the control is the value that the message carries, read and scaled by the way as for a note way of 0.7 (whose named values are values of the velocity, proposal 0009). Which note of the range arrived is not the value. The way gives the note number no other meaning: not a pitch, not an index in a list, not a step. A tool takes none from it that the descriptor does not write.

**A sender** picks the first way it can send, as in 5.3.3. A range does not name a note to send. A sender that has a note of the range from outside the descriptor (the person or the program that asks for it names it) MAY send the way with that note. A sender that has none MUST pass over the way to the next way it can send, and MUST NOT choose a note itself, the lowest bound included. When no way is left, it sends nothing for the control and says so to its caller. A note alone, `[n, n]` included, names its note and is sent as in 0.7.

### The converter

Every `note` way of 0.7 is a way of 0.8 with the same meaning. A converter MUST write it unchanged. It MUST NOT group neighbouring notes into a range, in one control or across controls, and it writes no `note_range`: a 0.7 descriptor does not say that two notes are one thing. A 0.7 descriptor admitted in 0.7 stays admitted: it holds no range.

### The example: otis

What is said of otis here is only that it receives the notes 1 and 2, the ranges 36 to 37 and 48 to 51, and the CC 112. **These numbers were not read again at their source. A public source, with its revision, is to be added before the proposal is a candidate.** The draft gives no role and no title to them, and the addresses are placeholders chosen for the draft. The entry at `/notes/49` is an **illustration** of a note written alone inside a range, not a fact about otis.

```json
{
  "$schema": "https://w3id.org/osd/0.8/device.schema.json",
  "device": { "name": "otis", "osc_prefix": "/otis" },
  "cc_params": {
    "entries": [
      { "osc": "/notes/1", "ways": [{ "ctrlType": "note", "ctrlIndex": [1], "transmit": "none" }] },
      { "osc": "/notes/2", "ways": [{ "ctrlType": "note", "ctrlIndex": [2], "transmit": "none" }] },
      { "osc": "/notes/36_37", "ways": [{ "ctrlType": "note", "note_range": [36, 37], "transmit": "none" }] },
      { "osc": "/notes/48_51", "ways": [{ "ctrlType": "note", "note_range": [48, 51], "transmit": "none" }] },
      {
        "osc": "/notes/49",
        "_illustration": "Not a fact about otis: shows a note written alone inside a range.",
        "default": 100,
        "ways": [{ "ctrlType": "note", "ctrlIndex": [49], "transmit": "none" }]
      },
      { "osc": "/cc/112", "ways": [{ "ctrlType": "cc", "ctrlIndex": [112], "transmit": "none" }] }
    ]
  }
}
```

What a reader gives on the receiving side (no way works on the other side, each saying `transmit: none`):

| Message | Reached, in order |
|---|---|
| note 1 | `/notes/1` |
| note 37 | `/notes/36_37` |
| note 49 | `/notes/49`, then `/notes/48_51`; the `default` of `/notes/49` belongs to it alone, and `/notes/48_51` has none |
| note 50 | `/notes/48_51` |
| note 40 | nothing |
| CC 112 | `/cc/112` |

### Cases with their verdict

On an invented device, "Quad", with `"contexts": { "mode": { "values": [{ "id": "play" }, { "id": "edit" }] } }`. Each way is of `ctrlType` `note` unless said otherwise; "range 36–40" is `note_range: [36, 40]`, "note 38" is `ctrlIndex: [38]`. A way with no direction written works on both sides. The controls are the entries 0, 1 and 2 of `cc_params`, one way each unless said otherwise, in the order of the columns. `channel`, `scale` and `default` are fields of the entry, `in`, `slice`, `range` and the directions fields of the way. A refusal is `BadValue` at the way named.

| | Entry 0 | Entry 1 | Entry 2 | Verdict |
|---|---|---|---|---|
| 1 | range 36–40 | range 40–44 | | refused at entry 1: both claim the note 40 |
| 2 | range 36–48 | range 38–40 | | refused at entry 1: a range inside a range |
| 3 | range 36–40, `transmit: none` | range 36–40, `recognize: none` | | admitted: no side in common |
| 4 | range 36–40, `channel: 0` | range 36–40, `channel: 1` | | admitted: two channels |
| 5 | range 36–40, `channel: 0` | range 36–40, no channel | | admitted: a channel and none |
| 6 | range 36–40, `in: { "mode": "play" }` | range 38–44, `in: { "mode": "edit" }` | | admitted: the contexts cannot hold together |
| 7 | range 36–40, `in: { "mode": "play" }` | range 38–44, no `in` | | refused at entry 1 |
| 8 | range 36–40, `slice: [0, 63]` | range 36–40, `slice: [64, 127]` | | admitted: two slices that do not meet |
| 9 | range 36–40 | note 38 | | admitted: the note 38 reaches entry 1, then entry 0 |
| 10 | note 38 | range 36–40 | | admitted: the note 38 reaches entry 0, then entry 1 |
| 11 | range 36–40 | range 38–38 | | admitted: a range of one note is a note alone |
| 12 | range 38–38 | note 38 | | refused at entry 1: one note alone twice |
| 13 | range 36–40 | note 38 | note 38 | refused at entry 2, naming entry 1 |
| 14 | range 36–40 | `pPress`, `ctrlIndex: [38]` | | admitted: two messages |
| 15 | range 36–40, `transmit: none` | range 38–44, `recognize: none` | note 39 | admitted: received, the note 39 reaches entry 2 then entry 0; sent, entry 2 then entry 1 |
| 16 | range 36–40 | two ways: `cc`, `ctrlIndex: [20]`; then note 38 | | admitted: the CC 20 reaches entry 1 alone; the note 38 reaches entry 1 by its note way, then entry 0 |
| 17 | two ways: range 36–40; then note 38, `range: [0, 1]` | | | admitted: the note 38 reaches entry 0 once, its value read by the way of the note alone |
| 18 | range 36–40, `in: { "mode": "play" }` | note 38 | | admitted: in `play`, entry 1 then entry 0; in `edit`, entry 1 alone |
| 19 | range 36–40; entry `scale: [0, 1]` | note 38; entry `default: 100` | | admitted: each control is judged on its own fields |

## What a reference does

| Reference | What it does of the need | Read for this draft |
|---|---|---|
| SFZ (sfzformat.com: the pages of `lokey` / `hikey`, `region`, `group`, `global`) | A region covers the keys `lokey` to `hikey`, 0 to 127 in SFZ 1; for one key, `key` is the more convenient form. All the input conditions of a region must be met for it to play. An opcode of `global` is inherited by `group` and `region`, and the nearer level wins. | Read at the source on 2026-11-27. |
| SFZ, regions that overlap | Two regions whose conditions one note meets both sound. | The pages read give each region its own conditions; none of them states this in one sentence. To be read at a source that states it before the proposal is a candidate. |
| SoundFont 2.04 (sfspec24.pdf, from synthfont.com: sections 7.3, 7.7, 8.1, 9.4 and the glossary) | `keyRange` gives the lowest and highest key for which a zone of a preset or of an instrument is active; a zone is undefined outside it. The first zone may be a global zone, whose generators affect all the other zones of the object; a generator of a local instrument zone replaces the same generator of the global one. A range of a preset zone filters, by intersection, the ranges of the instrument it uses. | Read at the source on 2026-11-27. |
| SoundFont 2.04, zones that overlap | Two zones of one object that cover one key both sound. | No sentence of the specification found that states it (searched for "overlap", "note-on" and the zone sections). To be read again before the proposal is a candidate. |
| MIDNAM (`MIDINameDocument10.dtd`) | A `NoteNameList` holds `Note` elements, each with one `Number` and a `Name`, possibly grouped in a `NoteGroup` with a name. The declaration has no range of notes. | The DTD kept in `osd-formats/tests/data/`, read for this draft. |
| MIDI 2.0 Property Exchange, controller resources (M2-117-UM) | No kind of message for a note (its controllers are continuous), so no range of notes. | Not read again: taken from proposal 0009. To be read at the source before the proposal is a candidate. |
| OSCQuery (github.com/Vidvox/OSCQueryProposal, `README.md`) | It describes OSC methods; `RANGE` bounds the values of a method's arguments, and the type of a MIDI message has none. It describes no MIDI note that a method answers. | Read at the source on 2026-11-27. |

**What OSD takes**: the range of keys on one entry, as `lokey` / `hikey` and `keyRange`, in the name and form of `triggers.note_range`; and, for a note written alone inside a range only, the superposition of SFZ regions and SoundFont zones: both apply.

**What OSD does not take**: inheritance. In these references it follows a link of structure (a region in its group and its global header, a local zone beside the global zone of its object). Two controls of OSD have no such link: neither belongs to the other, so a field has nowhere to come from.

**Where the draft departs, and why.**

- **Two ranges that overlap are refused.** SFZ and SoundFont let regions and zones overlap, for layers of sound. A descriptor says what a message means, and the rule of 5.3.3 refuses two controls that claim one message, so that a message is read one way. Two ranges that overlap are two notes claimed twice.
- **One message may reach two controls, in one case only**: a note written alone inside a range, on one side and one channel. It is the one exception to the message claimed once. The reason is the owner's decision: a note written alone may carry options in addition to those of the range. The references are SFZ, where overlapping regions sound together, and SoundFont 2.04, whose zones cover keys.

## Objections

None yet.
