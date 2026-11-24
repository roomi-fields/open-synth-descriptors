# 0003 — A control holds in a context: one general way to say that a message means this here and that there

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

This draft decides nothing: the content of 0.7 is the owner's. Every field name and every example below is a way to write the idea, to be settled by the owner, not a decision.

## What it changes

The text and the schemas of 0.7, or a later version: the rule of section 22.6 of `spec/0.6.md` (a message is claimed once), and a general way for a device to declare the contexts it works in and for a control to say in which of them it holds.

## Why

What this brings to the sharing of sounds on devices: a descriptor can say truly what a real device does when one message reaches several controls, so that the descriptors of the catalogue convert to the current version instead of staying behind.

What is wrong without it. 0.6 refuses a descriptor in which two controls claim the same message (`spec/0.6.md:1145`). SyntHub measured on its 1 573 published descriptors, all stored in 0.5 (measure of 2026-11-23, with the reader osd 0.6.0): the reader refuses 217 of them once converted, and 246 would not pass to 0.6 in all. Ten of the 217 were then read against their manuals. Some were the same control written twice, a fault of the descriptor that 0.6 rightly refuses. The others are real devices that 0.6 cannot describe.

### The three cases, with their evidence

The evidence is SyntHub's: a sort of the 217 descriptors, then ten of them read against the manuals of their devices (2026-11-23). **It was not re-read for this draft**: the manuals and the two reports are held by SyntHub, not in this repository. The numbers are cited, not measured again.

1. **The meaning of a message depends on a mode of the device.** Korg drumlogue: a global setting, single-channel or multi-channel, with two implementation tables (manual p. 60-61). Expert Sleepers disting EX: the loaded algorithm changes the assignments (manual p. 29). 7 pairs of the 49 read.
2. **The channel that tells two controls apart is a setting of the user, not a fixed number.** 1010music bento: a track answers on its "MIDI In Ch" (manual p. 385); also yarns/loom, op_xy, digitone and drumlogue. 15 pairs of the 49. The descriptor has no number to write, so the two controls look like the same message without a channel, which is what `spec/0.6.md:1131` says they are.
3. **One message is cut in slices of values, each slice a control.** Novation Circuit Tracks: one NRPN (0:122) shared by eight switches, each on a slice of values; the descriptor is true to the manual. 5 pairs of the 49.

The 49 pairs read hold three other things, which this proposal does not cover: 15 pairs are the same control written twice, 1 is an error of the descriptor, 6 are not established.

### One thing, not three

The three cases say one thing: *this message means this, in such a context*. A mode, a part whose channel the user sets and a slice of values are three contexts. There may be many others, and the format must not gain one field per case found. So this proposal designs one general structure; the three cases are its first uses and its proof.

The uncertainty of the data stays out of the format: contradictions between sources, and their "to validate" state, live in the catalogue (SyntHub keeps one descriptor per source), not in a descriptor.

## The change

### The structure

**A device declares its contexts.** `contexts` is an object. Each key names a *dimension*; its `values` list the values the dimension takes, each an object with an `id` and, optionally, a `channel`.

```json
"contexts": {
  "channel_setting": { "values": [ { "id": "single" }, { "id": "multi" } ] },
  "part": { "values": [ { "id": "track1" }, { "id": "track2" } ] }
}
```

- A dimension is a mode (the device is in one value at a time: `channel_setting`) or a set of parts (all hold at once, a message tells them apart by its channel: `part`). The rule below does not need the difference, so the structure has no field for it.
- A value with a `channel` (0 to 15, the field of `cc_params`) says that the messages of that value arrive on that channel. A value with no `channel` says that the channel is not the descriptor's: it is a setting of the user (case 2).
- Two values of one dimension never pin the same channel, a fault at the second.

**A control says the context it holds in.** An entry of `cc_params` may carry `in`, an object: each key a dimension of the device, each value one `id` or a list of ids.

```json
{ "cc": 7, "in": { "part": "track1" } }
{ "cc": 7, "in": { "part": ["track2", "track3"], "channel_setting": "multi" } }
```

A dimension an entry does not name leaves it holding in every value of that dimension. An entry with no `in` holds in every context. A dimension or a value `in` names that the device does not declare is a fault at its pointer (`BadValue`).

**A message declares its slices.** A way (the flat fields of an entry, or an element of `also`) may carry `slice`, `[lowest, highest]`, a closed interval of the values of the message (case 3). The dimension is the value of the message itself: no device declares it. `slice` is on the way, not on the entry, since the message is the way's.

```json
{ "nrpn_msb": 0, "nrpn_lsb": 122, "value_bits": 7, "slice": [0, 15] }
{ "nrpn_msb": 0, "nrpn_lsb": 122, "value_bits": 7, "slice": [16, 31] }
```

The `range` of the control stays what it was: the range of the value of the message, within the slice, not rescaled. A slice holds no arithmetic and no string to parse (one meaning per field).

### The rule

0.6 says: a message is claimed once. The proposal says: a message is claimed once *by claims that can hold together*.

- A **claim** is a message (its kind, its channel, its numbers: section 22.5 and 22.6 unchanged) with its **coordinates**: for each dimension the entry names in `in`, the set of ids; and, if the way has a `slice`, the interval, under the one name `value`.
- Two claims **can hold together** unless a coordinate named by both has two sets with nothing in common. A coordinate that only one names leaves them together.
- A reader MUST refuse the later claim of a message, at its place, naming the earlier, when the two claims can hold together. Two claims of the same message that cannot hold together are admitted.

That is one rule. A mode, a part and a slice are not three rules: each is a coordinate, and the same test (do the two sets meet) decides it. Two ways of one control have the control's contexts, so they meet on every dimension of `in`, and only their slices can separate them.

### Who declares what (the unification)

| Dimension | Declared by | Why |
|---|---|---|
| a mode, a part | the device (`contexts`) | the values are the device's: the user does not choose them in the message |
| the value | the message itself (`slice`) | the values it cuts are in the message; no declaration can add to them |

The slice and the mode are one structure *for the rule*: a set of values per coordinate and a test of meeting. They differ in where the values come from, which is why the slice is written on the way and the mode on the device. If the owner finds that difference too large for one structure, the slice can stay a separate proposal without breaking the rest (the rule above works with `in` alone); the draft says it unifies, and that the cost of leaving the slice out is case 3 only.

### The cases written

**Case 1, a mode.** Two controls on one CC, separated by the mode:

```json
"contexts": { "channel_setting": { "values": [ { "id": "single" }, { "id": "multi" } ] } },
"cc_params": { "entries": [
  { "cc": 7, "in": { "channel_setting": "single" } },
  { "cc": 7, "in": { "channel_setting": "multi" } }
] }
```

For the disting EX the dimension is `algorithm` and its values are the algorithms that change the assignments.

**Case 2, a part whose channel the user sets.**

```json
"contexts": { "part": { "values": [ { "id": "track1" }, { "id": "track2" } ] } },
"cc_params": { "entries": [
  { "cc": 7, "in": { "part": "track1" } },
  { "cc": 7, "in": { "part": "track2" } }
] }
```

Neither value has a `channel`: the user sets it, the descriptor does not know it.

**Case 3, slices of values.** Two of the eight switches of the NRPN 0:122, as in "The structure". The bounds there show the shape; the real ones are in the manual, which was not read here.

### A worked example of the rule

Seven controls on one message, the NRPN 0:122 with no channel, in a device with a dimension `channel_setting` (`single`, `multi`) and a dimension `part` (`track1`, `track2`):

| | Control | Coordinates |
|---|---|---|
| a | level of the single part | `channel_setting: single` |
| b | level in multi mode | `channel_setting: multi` |
| c | level of track 1 | `part: track1` |
| d | level of track 2 | `part: track2` |
| e | switch on the low slice | `slice: [0, 63]` |
| f | switch on the high slice | `slice: [64, 127]` |
| g | a control with no context | none |

21 pairs. Three are admitted; the other 18 are refused, with the reason.

| Pair | Verdict | Reason |
|---|---|---|
| a–b | admitted | `channel_setting` is named by both, `{single}` and `{multi}` have nothing in common |
| c–d | admitted | `part` is named by both, `{track1}` and `{track2}` have nothing in common: two parts do not share a channel |
| e–f | admitted | `value` is named by both, `[0,63]` and `[64,127]` have nothing in common |
| a–c, a–d, b–c, b–d | refused | no coordinate is named by both: in a mode a part also holds |
| a–e, a–f, b–e, b–f, c–e, c–f, d–e, d–f | refused | no coordinate is named by both: a value of the mode, of the part, may fall in the slice |
| g with a, b, c, d, e, f (6 pairs) | refused | a control with no context holds in every context, so with each other control of the message |

The refusal of a–c is the price of a plain rule. On a device where the parts exist only in multi mode, the descriptor says it in `a` and `c` themselves: `c` becomes `{ "part": "track1", "channel_setting": "multi" }`, which then meets `b` on `channel_setting` and is refused against `b` as a true double, and is separated from `a`. The descriptor writes what it knows; the reader never guesses that a part implies a mode.

### When the channel is a setting of the user

The descriptor does not know the channel, so the reader has no number to compare. It decides from the coordinates, never from a guessed number:

- **Two controls of different parts** cannot hold together without knowing the channel: the dimension `part` declares that the device tells its parts apart, and the reader takes the declaration as true. Admitted (c–d).
- **Two controls of the same part** are refused as today: they would answer the same message on the same channel, known or not (two controls that both name `track1`: their sets meet).
- **A control with no context** holds in every context, so it conflicts with every other control of the same message. Refused (g with all).
- A reader cannot check that the user gave two parts two channels: that is a setting of the device, not a fault of a descriptor.

### What the rule of 0.6 becomes

The context is optional. A descriptor of 0.5 or 0.6 has no `contexts`, no `in`, no `slice`: every claim has no coordinate, so every pair can hold together, and the rule refuses the later claim of a message exactly as `spec/0.6.md:1145` does. 0.6's rule is the case of the general rule where all contexts hold together; the text of 0.7 states one rule and no second one beside it. The converter carries each control as it is.

### What the converter does

The converter never invents a value that a later version adds (`spec/0.6.md`, section 7). It cannot know the mode, the part or the slice of a 0.5 control from its numbers, so it writes none.

| 0.5 descriptor of | The converter | Then |
|---|---|---|
| case 1 (two controls on a message, a mode apart) | refuses with `NotConvertible`, at the later control, naming the earlier, as the reader does since its correction of 2026-11-23, as for any 0.5 descriptor whose controls clash | the catalogue writes the descriptor in 0.7 with its `contexts` |
| case 2 (the channel not written) | the same | the same |
| case 3 (a slice) | the same | the same |
| none of the three, no clash | carries each control as it is, `$schema` aside | none |

So the structure does not make the 217 convert by itself; it makes them *writable* in the current version. Whether the path from the stored 0.5 of SyntHub is a rewrite of the descriptors in 0.7 by the catalogue (recommended: the converter then has no hint to trust) or a hint given to the converter (rejected: it is a value the converter would invent) is for the owner.

### What it counts on the catalogue

**How it was taken.** The existing reader was not played on the catalogue for this draft: the catalogue of SyntHub is not in this repository. Every number below is SyntHub's, from its measure of 2026-11-23, taken as reported.

| Figure | Source | Status |
|---|---|---|
| 1 573 published descriptors, all in 0.5 | SyntHub, 2026-11-23 | measured by SyntHub with osd 0.6.0, not replayed here |
| 217 refused by the reader after a conversion that succeeds | SyntHub, 2026-11-23 | measured by SyntHub, not replayed here |
| 246 that would not pass to 0.6 (217, 16 mixing CC and NRPN, 13 through `midi_of`) | SyntHub, 2026-11-23 | measured by SyntHub, not replayed here |
| 12 048 clashes in the 217; classes A 3 261, B 5 119, C 3 668 | SyntHub, 2026-11-23 | an estimate: heuristic sort on addresses, ranges and sections, no manual read; B is an upper bound |
| descriptors with a clash of the class: A 61, B 148, C 148; A alone 15 | SyntHub, 2026-11-23 | same estimate |
| 10 descriptors read against their manuals, 49 pairs of 1 766 | SyntHub, 2026-11-23 | measured by SyntHub; a sample chosen in B, not drawn at random |

What the structure would write, from the sample of ten read (49 pairs): case 2, 15; case 1, 7; case 3, 5; that is 27 pairs of 49 (55 %). The same control twice (15), an error (1) and the not established (6) it does not write: they stay refused, and the first is for the catalogue to merge. **This 55 % is a measure on a sample of ten chosen in class B; it does not extend to the 217.** The only bound on the catalogue is the upper one: at most the descriptors with a clash of class B (148 of 217) could become writable, and 15 descriptors, with a clash of class A alone, need no context. How many would convert is **not measured**: it needs each descriptor read against its manual, which is the work SyntHub did for ten.

To measure it, SyntHub plays the existing reader on its catalogue with the contexts written for the cases above (once the schemas exist), and compares the refusals; this is a ticket of the cycle after acceptance, not of this draft.

## What a reference does

The reading level of each statement is given. **MIDNAM was read at its source**: the DTD `MIDINameDocument10.dtd` of the MIDI Association, kept in `osd-formats/tests/data/` (lines below are of that file). **MIDI 2.0 Property Exchange was read at its source by the supervisor**, on the workshop machine, on 2026-11-23: the document M2-117-UM v1.0, "MIDI-CI Property Exchange Controller Resources" (19 May 2023). The document is not public and is not copied here: the facts below are written in our own words, each with its section number as the address. The owner can check them on the same document.

### MIDNAM (read at the DTD)

- **A mode** (case 1). `CustomDeviceMode` and `StandardDeviceMode` have a required `Name` (lines 44-53); each holds `ChannelNameSetAssignments`, a list of `ChannelNameSetAssign` that give each `Channel` a `NameSet` (lines 61-65). A device has several modes (`MasterDeviceNames` takes `(CustomDeviceMode | SupportsStandardDeviceMode)+`, line 30), and each mode assigns channels to sets of names. So MIDNAM names a mode and what it assigns. The proposal writes the same thing with the device declaring the mode and each control naming it; it differs in that MIDNAM states, per mode, a full assignment, where OSD states per control the modes it holds in. The reason: a control is the unit that OSD describes, and most controls hold in every mode and say nothing.
- **A control list per mode** (case 1, the controls themselves). Beyond the assignment, each mode also carries its own name lists: `CustomDeviceMode` holds `ChannelNameSetAssignments` then `(%namelist;)*` (lines 49-50), and the entity `namelist` is `PatchNameList | NoteNameList | ControlNameList | ValueNameList` (line 16), so a mode may hold its own `ControlNameList`, a list of `Control+` each with `Type`, `Number` and `Name` (lines 117-125). A `ChannelNameSet` references one too (`UsesControlNameList`, line 112; `noteorctl_listorref`, line 17), and a `Patch` may (line 90). So in MIDNAM the same CC number can be named differently in two modes by two `ControlNameList`s. The proposal departs: it keeps one list of controls and states on each control the modes it holds in, rather than one full list per mode. The reason: a control that holds in every mode is written once instead of once per mode, and the reader's one-message-one-control rule then runs on one list.
- **A channel set by the user** (case 2). `ChannelNameSet` has `AvailableForChannels`, a list of `AvailableChannel` with `Channel` (required) and `Available` (`true` by default) (lines 67-75); `ChannelNameSetAssign@Channel` is required (line 64). The channel is always a number written in the document, so MIDNAM cannot say "the user's setting". **The proposal departs from MIDNAM here**: a value of a dimension may have no `channel`. The reason: the evidence (bento, manual p. 385) is that the number is not the descriptor's, and a number invented would be false.
- **A slice of values** (case 3). `Control` has `Type`, `Number` and `Name` and holds `Values` (lines 121-125): `Min`, `Max` and a `ValueNameList` of `Value` with `Number` and `Name`. A `Value` gives a name to one number of a control; the DTD has nothing that gives a control to a slice of the values of another. The nearest way is one `Control` whose `ValueNameList` names the values of every slice. That makes eight switches one control with named values, not eight controls. **The proposal departs from MIDNAM here**: it makes each slice a control. The reason: OSD's unit is the control (a name, a role, a range, a default per control), and the catalogue's descriptor for Circuit Tracks has eight switches.

### MIDI 2.0 Property Exchange, ChCtrlList and AllCtrlList (M2-117-UM v1.0, read at its source)

- **The list is live** (section 3; section 2 for `AllCtrlList`). `ChCtrlList` is the current set of the controller messages a device recognises or sends ("Active Controller Messages", defined in the terms of the document) for one entry of the `ChannelList`; it cannot be used without that entry. `AllCtrlList` (section 2) is the same for all the channels currently active, each entry carrying a `channel` property. So Property Exchange has no static field for a mode: the list is asked for while the device runs and says what holds now; a device whose mode changes answers another list. A context is the state of the device at the time of the question, not a declared dimension.
- **A part is a ChannelList entry** (case 2; sections 3.4 and 3.4.1). The list of a channel is reached through a link of a `ChannelList` entry, with a `resId` per entry (examples `ch1`, `ch10`); a link with the role `cluster` declares the list of a cluster of channels (a manager and its members). So the part is an entry of the `ChannelList` and its channel is read from the device; it is not a fixed number written in the controller list. **OSD takes this**: a part declared as an entry, with a channel that the device or the user gives.
- **A limited range, and the values of one control** (case 3; sections 2.1.1.2, 2.1.1.3, 3.1.1 and 4). `minMax`, two integers, declares the limited range of values a message is recognised in; it is not used with `stepCount` on the same entry. An entry with the `typeHint` `valueSelect` names a `CtrlMapList` by `ctrlMapId`, which gives the enumerated values of that one control, each with its label. So Property Exchange writes the values of a message as named values of ONE control; it has no way to give several entries each a slice of one message. **OSD takes the limited range of values on a control**, and departs for the slice (below).
- **Matching** (sections 2.1.2 and 3.3). An entry is matched to an incoming message by `ctrlType` and `ctrlIndex` (and `channel` in `AllCtrlList`). This is the same key as the rule "one message, one control" uses.
- **Where OSD departs, and why.** A descriptor is a file read with no device plugged in, so the contexts are declared in it instead of being asked for live; this is the one departure from the live list, and it is forced by what a descriptor is. For the slice (case 3) the reason is the one given for MIDNAM: OSD's unit is the control, so each slice is a control, where Property Exchange would give one control with named values.

## Two other contexts the structure writes the same way

None found in the catalogue (the catalogue was not read for this draft); two from the references, at the level of knowledge of the references, not re-read here:

1. **A General MIDI device's drum channel.** MIDNAM's `ChannelNameSet` with `AvailableForChannels` is the structure by which a note means a drum on channel 10 and a pitch elsewhere. As a dimension: `part` with the values `drums` (`channel: 9`) and `melodic`. The DTD gives the structure (lines 67-75, and `NoteNameList` in the same set); no real MIDNAM file of a GM device was read for this draft.
2. **The zone of MPE.** The MPE specification (MIDI Association) splits the channels into a lower zone and an upper zone, a master channel and member channels. As a dimension: `zone`, with the values `lower` (`channel: 0`) and `upper` (`channel: 15`) for the master channel; the same message means a per-zone message. Not re-read for this draft; the 0.6 text lists MPE under "Left for a later version" (`spec/0.6.md:1572`), so this is a candidate use, not a promise.

Neither needs a field of its own: each is a dimension with values.

## What the structure does not cover

Left to a later proposal:

- **The controls of `params` (SysEx)** and the `commands`, `programs` and `sound` of a device: `in` is written on `cc_params` only.
- **How a tool knows the current context.** The format says in which context a control holds; it does not say which context the device is in at this moment (the MIDNAM `DeviceModeEnable` is the neighbour: the message that enters a mode).
- **A relation between dimensions** (a part exists only in the multi mode): the descriptor writes it in each control (see the worked example); a dimension that declares its own condition is for later.
- **A model of the multitimbral device or of MPE** (`spec/0.6.md:1572`): a part here only separates two controls.
- **The uncertainty of the data**, which stays in the catalogue (owner's decision, 2026-11-23).
- **The same control written twice**, and the error of a descriptor: faults of the descriptor, refused as today.
- **A context of a value of another control** (a switch that changes the meaning of a CC).

## Objections

None yet. Questions for the owner, which this draft recommends and does not settle:

1. The names (`contexts`, `in`, `slice`, `channel_setting`) and the shape of the `values` (objects with an `id`, to leave room for notes and a `channel`).
2. Whether the slice and the mode share one rule (recommended) or the slice waits for its own proposal.
3. Whether the part dimension is trusted to separate (recommended: yes, a declaration of the device) or the reader refuses two parts whose channels are both unknown.
4. The path from the stored 0.5 to 0.7 (the catalogue rewrites, recommended) rather than a hint to the converter.
5. The reading of the manuals, before the draft passes to `candidate` (Property Exchange is read at its source, M2-117-UM v1.0).
