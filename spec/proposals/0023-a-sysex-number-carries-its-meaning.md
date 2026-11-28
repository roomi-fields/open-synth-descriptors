# 0023 — A number of a SysEx way carries its meaning

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: section 5.3.3 of [0.7](../0.7.md) (the `sysex` way) gains the field `meanings`, section 21.1 (the words defined) defines it, and a state of the vocabulary of roles gains a list of address words.

## Why

What this brings to the sharing of sounds on devices: each number of a SysEx address can say in common words what kind of thing it addresses, so a tool knows it on any device (owner, 2026-11-27: allow giving the meaning, prefer it, normalize it).

In 0.7 the numbers `pr`, `p`, `c`, `r` and `id` of a `sysex` way are named after their place in the maker's frame. Neither 0.7 nor the sources read for this draft say what the letters `pr`, `p`, `c` and `r` stand for: that is to be verified ("à vérifier"). The frame says where each number goes in the message, and nothing says what it addresses. A tool that reads two devices cannot tell which number picks a parameter and which picks the thing that holds it.

## The change

### The field `meanings`

A `sysex` way MAY give `meanings`, a list of one meaning or more. Each meaning is an object that says what one number of the way addresses, or what several numbers address together.

| Key of a meaning | Form | Says |
|---|---|---|
| `number` | one of `pr`, `p`, `c`, `r`, `id` | the one number of the way that the meaning is of; exclusive of `numbers` |
| `numbers` | a list of two, three or four of `pr`, `p`, `c`, `r` | the numbers of the way that together hold one quantity, the most significant first |
| `number_bits` | `7` or `8`; required with `numbers`, absent with `number` | how many bits each number of `numbers` counts in the quantity, the same for all |
| `term` | a string | what is addressed: a term of the vocabulary of roles, an address word, or an extension |

A meaning gives exactly one of `number` and `numbers`. A key that starts with `_` is a note for people, as elsewhere.

The names `pr`, `p`, `c`, `r` and `id` keep saying the place of a number in the frame: no field is renamed, and the frames of `params` are unchanged. A meaning says what the number addresses. A meaning names only numbers the way gives, and a number is named by one meaning at most. No string holds structure, and no other field says what a number addresses.

### A quantity cut into several numbers

A maker may cut one quantity into several numbers of the frame. It is written as one meaning with `numbers` and `number_bits`. Its value is the concatenation of the numbers, the first holding the highest bits: for the numbers n1 to nk and the width w, the value is n1 × 2^(w×(k−1)) + … + n(k−1) × 2^w + nk. The numbers are read as the way writes them, before the frame does anything to them (an `add`, an `encoding`, a checksum). Each number MUST be below 2^w.

A number enters one meaning only. `id` is already a whole quantity: it enters no `numbers`, and it may carry a meaning of its own. Since 0.7 defines `id` as the parameter's own number, an author SHOULD NOT write the term `parameter` on it: the field already says it. A meaning cannot give two widths: a quantity cut into numbers of different widths waits for a source that shows one.

EXAMPLE On an invented device, "Orage":

| Way | Meaning | Value of the quantity |
|---|---|---|
| `"c": 2, "r": 45` | `{ "numbers": ["c", "r"], "number_bits": 7, "term": "parameter" }` | 2 × 128 + 45 = 301 |
| `"p": 1, "c": 2, "r": 3` | `{ "numbers": ["p", "c", "r"], "number_bits": 8, "term": "parameter" }` | 1 × 65536 + 2 × 256 + 3 = 66051 |
| `"pr": 1, "p": 0, "c": 0, "r": 1` | `{ "numbers": ["pr", "p", "c", "r"], "number_bits": 7, "term": "parameter" }` | 1 × 2097152 + 1 = 2097153 |

The order of `numbers` is the order of significance, whatever the order of the numbers in the frame: `["r", "c"]` with `"c": 2, "r": 45` is 45 × 128 + 2 = 5762.

### The term

A `term` is read in this order:

1. an extension `x-<registrant>-<term>` (`^x-[a-z0-9]+-[a-z0-9-]+$`);
2. otherwise a name of the state that `role_vocabulary` names: a term of type `part`, or an address word of that state.

When the vocabulary of roles names the thing addressed, the author MUST write its term: a number whose values pick filters is `filter`, never an address word. An address word serves for what no term names. A number whose values pick objects that hold parameters, of several kinds or of a kind no term names, is `slot`.

Only a term of type `part` may be a meaning. A term of type `value` is a choice of a setting, not a thing an address reaches. A term of type `setting` says what is set, which the role of the entry already says once: a number that picks which setting is reached is `parameter`.

A descriptor with a term that is not an extension MUST name its state in `role_vocabulary`, the field that already names the state of the roles. A file with `midi_of` follows the rules of the roles: the meanings of each file are read against the state that file names, and states are never merged (sections 23.35 and 23.9 of 0.7).

### The address words

The address words are a list of their own, governed as the vocabulary of roles is. Each word has its name, in the exact spelling below, a definition of one sentence, and a `source` (a document read, and its place in it) or a `shared_by` (two devices of two makers at least), in the forms of [state 1](../vocabulary/1.md), section 7. It may list makers' words, each with its origin.

| Word | Definition | Source | Makers' words |
|---|---|---|---|
| `parameter` | A parameter of the device, which the number tells apart from the others that the rest of the address leaves open. | Global Parameter Control (CA# 24, 1999-03-05): "Parameter ID", "Parameter to be controlled" | none |
| `slot` | An object inside the device that holds parameters, which the number tells apart from the others. | Global Parameter Control (CA# 24): "slot path", "Slot Number MSB", "Slot Number LSB" | `block` (Yamaha, reface Data List, "Parameter Block") |
| `key` | A key of the keyboard whose own parameters the message reaches. | Key-Based Instrument Controllers (CA# 23, 1999-02-28): "Key number" | none |

**Where the list lives.** In the folder of a state of the vocabulary of roles, `schemas/vocabulary/<state>/addresses.json`, with its own schema, and in a section of the text of that state. The list enters with a state, grows only by a later one, and a state keeps every word of the state before. This draft writes `"3"` for the first state that holds it; the owner gives the number.

**How it grows.** A word enters by a proposal ([PROCESS.md](../PROCESS.md)), like a term, and follows the rule of entry of [state 2](../vocabulary/2.md), section 8, read over both lists: a publisher MUST NOT enter, as a term or as an address word, a function that a term, a composition of terms or an address word already names. A part of an instrument enters as a term of the vocabulary of roles, not as an address word. A reader refuses a state that holds one name in both lists, or a maker's word under two names.

**What does not enter.** A word that says only the rank of a number in a composed address (High, Mid, Low, MSB, LSB) says where the number is, which the way already says. `bank` and `index` do not enter: Property Exchange uses them for the two numbers of an NRPN or an RPN, and no document read for this draft uses them as the name of a part of a SysEx address. The maker's id, the device id, the model id and a command id are parts of the message: the frame and `identity` already say them, and they have no word here.

**Extensions.** A `term` may be an extension `x-<registrant>-<term>`, in the form of section 6.2 of 0.7 and in the registry of [schemas/extensions.md](../../schemas/extensions.md). A reader accepts one it does not know, gives it as written, writes it back, and treats the meaning as unknown: for a tool, the numbers it names mean what numbers with no meaning mean. The reader still checks the numbers it names and their width. An extension asks for no `role_vocabulary`.

### A number whose meaning no source says

A number whose meaning no source says carries none. Its meaning is to be verified ("à vérifier"); that is not a word of any list, and nothing is written for it.

- An author of a descriptor SHOULD write the meaning of a number when the source of the descriptor says it.
- An author MUST NOT deduce a meaning from the letter of the number, from its place in the frame, from its rank among the numbers, from its value, or from the OSC address of the entry.
- A reader MUST NOT refuse a way for a number without a meaning, or for a way without `meanings`.

### Why on the way and not on the block `params`

The block has one frame for its entries, so `p` goes to the same place in every message. What `p` addresses may still differ from one entry to the next. In the reface Data List of Yamaha, the reface DX puts the operator number in "Address Mid" for the parameters whose "Address High" is `31`, and `00` there for the common voice parameters at `30`: one place, two meanings. A quantity may also be cut into numbers for some entries and not for others, and the way is where the numbers are written. A default on the block with exceptions on the ways would be a second place that says the same thing, and a reader would have to join them.

The cost is repetition. The two meanings of the example below take 101 characters of compact JSON; a device of 500 such entries carries about 50 kB more.

### The schema

The fragment shows what changes in the device schema of 0.8: the definition `sysex_way` gains one property, and a definition is added. The rest of `sysex_way` is that of 0.7.

```json
{
  "$defs": {
    "sysex_way": {
      "properties": {
        "meanings": { "type": "array", "minItems": 1, "items": { "$ref": "#/$defs/number_meaning" } }
      }
    },
    "number_meaning": {
      "type": "object",
      "required": ["term"],
      "additionalProperties": false,
      "patternProperties": { "^_": true },
      "properties": {
        "number": { "enum": ["pr", "p", "c", "r", "id"] },
        "numbers": { "type": "array", "minItems": 2, "maxItems": 4, "items": { "enum": ["pr", "p", "c", "r"] } },
        "number_bits": { "enum": [7, 8] },
        "term": { "type": "string", "pattern": "^[a-z][a-z0-9-]*$" }
      },
      "if": { "required": ["numbers"] },
      "then": { "required": ["number_bits"], "properties": { "number": false } },
      "else": { "required": ["number"], "properties": { "number_bits": false } }
    }
  }
}
```

The schema of the address list of a state has the forms of a term's `name`, `definition`, `source`, `shared_by` and `words`.

### The text of 0.8

In section 5.3.3, the table of the fields of a way gains the row:

| Field | Says |
|---|---|
| `meanings` | on a `sysex` way only, optional: what its numbers address |

and the rules of this section follow "The numbers of a `sysex` way". In section 21.1, the rows `param_entry.pr`, `p`, `c`, `r` and `id` stay, and one row is added:

| Word | Definition |
|---|---|
| `sysex_way.meanings` | what the numbers of a `sysex` way address, in common words: each meaning names one number, or the numbers that together hold one quantity, and the term of what is addressed; the name of a number says its place in the frame, its meaning says what it addresses |

### An example

The members of a descriptor of an invented device, "Orage", that the field touches:

```json
{
  "role_vocabulary": "3",
  "params": {
    "get_frame": [240, 125, { "arg": "pr" }, { "arg": "p" }, { "arg": "c" }, { "arg": "r" }, 247],
    "set_frame": [240, 125, { "arg": "pr" }, { "arg": "p" }, { "arg": "c" }, { "arg": "r" }, { "arg": "value" }, 247],
    "value_type": "u7",
    "entries": [
      { "osc": "/filter2/cutoff",
        "role": { "parts": [{ "part": "filter", "index": 2 }], "setting": "cutoff" },
        "ways": [{ "ctrlType": "sysex", "pr": 16, "p": 1, "c": 2, "r": 45,
          "meanings": [
            { "number": "p", "term": "filter" },
            { "numbers": ["c", "r"], "number_bits": 7, "term": "parameter" }
          ] }] }
    ]
  }
}
```

A tool learns from it that in this address:

- `p` designates a filter;
- `c` and `r` together designate a parameter, whose number is 301;
- `pr` carries no meaning: the tool knows its value and its place, and nothing of what it addresses.

It does not learn which filter the value 1 of `p` designates, nor that another value reaches another filter by the same rule, and it cannot build the address of any other thing from this one. The role alone says the part and its index: here the filter 2. A meaning carries no index, and a reader compares neither with the other.

### What a reader does

A reader gives `meanings` as written, in the order written, and writes them back. It adds none and computes none; the value of a cut quantity is for a tool to compute.

### What a reader refuses

`W` stands for `/params/entries/N/ways/M`, and `K` for `W/meanings/I`. A way that carries a fault of section 5.3.3 of 0.7 is refused for it first. The faults below are then judged in the order of the table, and within a row at the earliest meaning.

| Fault | At | Kind | Seen by |
|---|---|---|---|
| `meanings` that is no list, or an empty list | `W/meanings` | `BadValue` | the schema and the reader |
| a meaning that is no object | `K` | `BadValue` | the schema and the reader |
| a key a meaning does not define | the key | `UnknownField` | the schema and the reader |
| `number` and `numbers` together | `K/number` | `UnknownField` | the schema and the reader |
| neither `number` nor `numbers` | `K/number` | `Missing` | the schema and the reader |
| no `term` | `K/term` | `Missing` | the schema and the reader |
| a `term` that is no string, or has not the form (`"Slot"`) | `K/term` | `BadValue` | the schema and the reader |
| a `number` that is not one of the five names | `K/number` | `BadValue` | the schema and the reader |
| `numbers` that is no list, or holds fewer than two or more than four (one number alone) | `K/numbers` | `BadValue` | the schema and the reader |
| `id`, or a name that is none of `pr`, `p`, `c`, `r`, in `numbers` | `K/numbers/J` | `BadValue` | the schema and the reader |
| `numbers` without `number_bits` | `K/number_bits` | `Missing` | the schema and the reader |
| a `number_bits` other than 7 or 8 | `K/number_bits` | `BadValue` | the schema and the reader |
| `number_bits` beside `number` | `K/number_bits` | `UnknownField` | the schema and the reader |
| a meaning of a number the way does not give | `K/number` or `K/numbers/J` | `BadValue` | the reader |
| a number named a second time, in the same meaning or in a later one | the second naming, `K/number` or `K/numbers/J` | `BadValue` | the reader |
| a number of `numbers` that is 2^`number_bits` or more | `K/numbers/J` | `BadValue` | the reader |
| a `term` that is no extension, in a descriptor without `role_vocabulary` | `/role_vocabulary` | `Missing` | the reader |
| a `term` that is no extension and that the named state holds neither as a term of type `part` nor as an address word (a word outside both lists, a term of a later state, a setting, a value) | `K/term` | `BadValue` | the reader |
| `meanings` on a way of a kind of MIDI | `W/meanings` (on `cc_params`, `/cc_params/entries/N/ways/M/meanings`) | `UnknownField` | the schema and the reader; judged where 0.7 judges a field the kind does not hold |
| `meanings` on a way of a descriptor of 0.7 | `W/meanings` | `UnknownField` | the schema of 0.7 and the reader |
| `meanings` on an entry of `params` of a descriptor of 0.6 or earlier, which has no way | `/params/entries/N/meanings` | `UnknownField` | the schema of that version and the reader |

A `role_vocabulary` that names a state the reader does not hold is refused as in 0.7.

The schema sees the form of a meaning. It does not compare a meaning with the numbers of its way or with another meaning, does not hold a number to the width, and does not hold the lists: those are the reader's alone.

### Cases

Each row gives `meanings` on the way `{ "ctrlType": "sysex", "pr": 16, "p": 1, "c": 2, "r": 45 }` of the first entry of "Orage", in a descriptor of 0.8 with `"role_vocabulary": "3"` and no `midi_of`, unless the row says otherwise. `P` stands for `"term": "parameter"`, and `M` for `/params/entries/0/ways/0/meanings`.

| Case | `meanings` | Verdict |
|---|---|---|
| the example | `[{ "number": "p", "term": "filter" }, { "numbers": ["c", "r"], "number_bits": 7, P }]` | admitted |
| no `meanings` | absent | admitted |
| an unknown extension, on the entry without its `role`, no `role_vocabulary` | `[{ "number": "p", "term": "x-acme-layer" }]` | admitted: an unknown meaning |
| a meaning of `id`, on a way that gives `id` alone | `[{ "number": "id", "term": "x-acme-control" }]` | admitted |
| an empty list | `[]` | `BadValue` at `M` |
| a string | `["p=filter"]` | `BadValue` at `M/0` |
| an index | `[{ "number": "p", "term": "filter", "index": 2 }]` | `UnknownField` at `M/0/index` |
| both forms | `[{ "number": "p", "numbers": ["c", "r"], "number_bits": 7, P }]` | `UnknownField` at `M/0/number` |
| no number | `[{ P }]` | `Missing` at `M/0/number` |
| no term | `[{ "number": "p" }]` | `Missing` at `M/0/term` |
| a capital | `[{ "number": "p", "term": "Filter" }]` | `BadValue` at `M/0/term` |
| an unknown number | `[{ "number": "q", P }]` | `BadValue` at `M/0/number` |
| one number in a composition | `[{ "numbers": ["c"], "number_bits": 7, P }]` | `BadValue` at `M/0/numbers` |
| `id` in a composition | `[{ "numbers": ["id", "r"], "number_bits": 7, P }]` | `BadValue` at `M/0/numbers/0` |
| no width | `[{ "numbers": ["c", "r"], P }]` | `Missing` at `M/0/number_bits` |
| another width | `[{ "numbers": ["c", "r"], "number_bits": 4, P }]` | `BadValue` at `M/0/number_bits` |
| a width for one number | `[{ "number": "c", "number_bits": 7, P }]` | `UnknownField` at `M/0/number_bits` |
| a number the way does not give, on a way without `pr` | `[{ "number": "pr", "term": "slot" }]` | `BadValue` at `M/0/number` |
| a number in two meanings | `[{ "number": "c", "term": "slot" }, { "numbers": ["c", "r"], "number_bits": 7, P }]` | `BadValue` at `M/1/numbers/0` |
| a number twice in one composition | `[{ "numbers": ["c", "c"], "number_bits": 7, P }]` | `BadValue` at `M/0/numbers/1` |
| a number beyond the width, with `"c": 200` | `[{ "numbers": ["c", "r"], "number_bits": 7, P }]` | `BadValue` at `M/0/numbers/0` |
| the same number in 8 bits, with `"c": 200` | `[{ "numbers": ["c", "r"], "number_bits": 8, P }]` | admitted: 51245 |
| a word of a term, on the entry without its `role`, no `role_vocabulary` | `[{ "number": "p", "term": "filter" }]` | `Missing` at `/role_vocabulary` |
| an address word, `"role_vocabulary": "2"` | `[{ "number": "p", "term": "slot" }]` | `BadValue` at `M/0/term` |
| a word of neither list | `[{ "number": "p", "term": "bank" }]` | `BadValue` at `M/0/term` |
| a setting | `[{ "number": "p", "term": "cutoff" }]` | `BadValue` at `M/0/term` |
| `meanings` on a way `{ "ctrlType": "cc", "ctrlIndex": [74] }` of `cc_params` | `[{ "number": "p", "term": "filter" }]` | `UnknownField` at `/cc_params/entries/0/ways/0/meanings` |
| the way of "Orage" in a descriptor of 0.7, `"role_vocabulary": "2"` | `[{ "number": "p", "term": "filter" }]` | `UnknownField` at `M` |
| a 0.6 descriptor, `"role_vocabulary": "2"`, the field on the entry | `[{ "number": "p", "term": "filter" }]` | `UnknownField` at `/params/entries/0/meanings` |

### The converters

The converter from 0.7 writes no `meanings`. It guesses none from a note, a name, a title or any text, and it loses and adds nothing for this field. An entry of 0.6 or earlier becomes a way with no meaning, as it does in 0.7.

A converter to a format without this field (MIDI 2.0 Property Exchange, MIDNAM, OSCQuery) names in its losses each `meanings` written, at its pointer, whether or not it can write the rest of the way.

### Open points

The owner decides them at the acceptance.

- The list of address words: `parameter`, `slot` and `key`, their definitions, and whether `key`, sourced by a Universal message alone, enters.
- The place of the field, on the way, and its names `meanings`, `number`, `numbers`, `number_bits`, `term`.
- The place of the list, in a state of the vocabulary of roles, and the number of that state. It widens what `role_vocabulary` says: the state of the roles and of the meanings.
- The correspondence between the value of a number and the index of a part: no meaning says that the value 1 of `p` is the filter 2.
- The order of containment of two numbers that both say `slot`, as the successive entries of a slot path: a meaning says nothing of it.
- Whether `id` should carry a meaning at all, since 0.7 already says what it is.
- A number that holds the channel, as in the Key-Based Instrument Control message, has no word: an entry and a context already say a channel.
- Whether a Part of a Roland address map is the part `track` of state 2 is for the vocabulary to say.
- No document read for this draft shows a quantity of an address cut into numbers of 8 bits: the width 8 is admitted because 0.7 lets `pr`, `p`, `c` and `r` hold a whole byte.
- Numbers of mixed widths in one quantity wait for a source that shows them.

## What a reference does

All were read at their source for this draft on 2026-11-28, in a copy of the document, by extracting its text. "Names" gives what the document calls the parts of an address.

| Reference | Revision read | Names |
|---|---|---|
| MIDI 1.0 Detailed Specification, System Exclusive | 4.2.1, revised February 1996 | "ID number" (the manufacturer's, or Universal), then, in a Universal message, "device ID", "sub-ID#1" and "sub-ID#2". Under a maker's ID the format is the maker's: no part of an address is named. |
| Global Parameter Control, a Universal Real Time message of MIDI 1.0 | Confirmation of Approval CA# 24, 1999-03-05 | A "slot path" of "Slot Number MSB" and "Slot Number LSB" entries, which "is used to identify the object within the device where the parameters reside"; "Parameter ID", "MSB first", of a "Parameter ID Width" in bytes; "Parameter Value". |
| Key-Based Instrument Controllers, a Universal Real Time message of MIDI 1.0 | Confirmation of Approval CA# 23, 1999-02-28 | "MIDI Channel Number", "Key number", "Controller Number". |
| MIDI-CI | M2-101-UM v1.2.1, 2025-12-18; the standard format of its messages (section 5.2.1) read, and its text searched for "parameter address", "parameter ID" and "parameter number" | "Device ID", "Sub-ID#1", "Sub-ID#2", "Source MUID", "Destination MUID": the parts of its own messages. The search found no name for a parameter address. |
| MIDI 2.0 Property Exchange, controller resources | M2-117-UM v1.0.1, 2024-11-15, and v1.0, 2023-06-15, which 0.7 cites | `ctrlType` is `cc`, `chPress`, `pPress`, `nrpn`, `rpn`, `pBend`, `pnrc`, `pnac` or `pnp`: no SysEx kind, so it names no part of a SysEx address. Of `ctrlIndex`: "For (N)RPN this will be the Bank (MSB) and Index (LSB)". |
| MIDNAM | `MIDIEvents10.dtd` and `MIDINameDocument10.dtd`, version 1.0, 19 January 2003 | The element `SysEx` is a string of hex bytes with the elements `SysExDeviceID` and `SysExChannel`. It names no parameter address. |
| Yamaha, reface CS/DX/CP/YC Data List | published 04/2016, PO-B0 (`data.yamaha.com`) | "Model ID", "Address High", "Address Mid", "Address Low"; a "Parameter Block" has a "Top Address"; in the table of the reface DX, "op = operator number" stands in "Address Mid". |
| Roland, JD-Xi MIDI Implementation | Version 1.00, May 1, 2015 (`static.roland.com`) | "model ID", "command ID", an "address" from "MSB" to "LSB"; a "Parameter Address Map" adds an "Offset Address" to a "Start Address"; its descriptions name what a range holds ("Temporary Tone", "Program Part", "Partial"). |

**What OSD takes, one word for each thing addressed.**

| Thing | Word | Why this one | Makers' words kept |
|---|---|---|---|
| the parameter reached | `parameter` | A MIDI standard names it ("Parameter ID"). The word of the thing is taken; "ID" says that it is a number, which every meaning is of. | none |
| the object that holds parameters | `slot` | A MIDI standard names it, so its word is taken over a maker's. | `block` (Yamaha) |
| the key whose parameters are reached | `key` | A MIDI standard names it ("Key number"). | none |

The cut of one quantity into numbers, the most significant first, is that of the "Parameter ID" of Global Parameter Control.

**Where the draft departs, and why.**

- **A meaning per number, not a fixed structure of address.** Global Parameter Control fixes a slot path and then a parameter. A maker's frame has its own order, which the frames of `params` already say: a meaning only says what each number addresses.
- **Names in lower case.** The standards write "Parameter ID" and "Slot Number"; the words take the form of the terms of the vocabulary, so one comparison serves both lists.
- **No rank words.** Yamaha and Roland name numbers High, Mid, Low, MSB and LSB. That is a place, which `pr`, `p`, `c` and `r` already say.
- **No sum of addresses.** The Roland map adds an offset to a start address. Those are several quantities added, not one quantity cut: a descriptor writes the numbers of the resulting address, and gives a meaning to those its source explains.
- **Not in `ctrlIndex`.** Property Exchange has no SysEx kind, so `ctrlIndex` stays with the kinds of MIDI, as in 0.7, and the new field takes no name of the standard.
- **A width per quantity of 7 or 8 bits.** Global Parameter Control counts its "Parameter ID Width" in bytes of a message. A number of a way holds up to a byte of 8 bits, so the meaning says how many bits count.

## Objections

None yet.
