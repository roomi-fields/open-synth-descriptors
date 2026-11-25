# 0008 — What 0.7 removes and what changes form, the partners kept

## Status

- State: `candidate`
- Candidate since: 2026-11-24
- Under the trial: —

This proposal decides nothing until it is accepted. The owner answered the questions of the draft on 2026-11-24; the last section records the answers; none remains open.

## What it changes

The version 0.7: the device schema (`schemas/0.7/device.schema.json`) and the text of 0.7 change the form of the fields of section 2 and drop the seven fields of section 3 (`capabilities` and `_coverage` stay); the converter 0.6 to 0.7 carries every change. Nothing of 0.6 or 0.6.1 changes. Proposal 0006 is amended: a source's `name` becomes `title`.

## Why

What this brings to the sharing of sounds on devices: 0.7 keeps every fact a partner exchanges and drops what says nothing, so a descriptor says each thing once and a round trip with Electra One, midi.guide or KnobKraft Orm loses nothing.

What is wrong without it: the text of 0.6 says "a key that starts with `_` is a note for people" (`spec/0.6.md`, section 4, line 145), yet programs read keys that start with `_`; a vendor's name sits in a key (`_electra`) and in the keys of a source; several fields say a fact no partner has and no reader of OSD needs.

The owner's decisions that frame it (2026-11-24):

1. The standard is a facilitator of SyntHub's needs, never an obstacle: what SyntHub needs to say finds a place; the format decides how it is said.
2. A standard holds no field specific to one product or vendor: the information and its source are kept, the structure is general (a source is credited by an entry of the sources, never by the shape of a field).
3. Before a field or a form is removed, its data from the partners (Electra One, midi.guide, KnobKraft Orm) is counted; a field a partner produces or reads stays, or changes form without loss.
4. One thing is said one way: no two structures for the same fact.
5. A key starting with `_` is accepted, kept and written back by a reader, and no program reads its meaning; everything a program reads is an ordinary field.
6. The display of a controller stays outside the descriptor of a device in 0.7 (see below).
7. Scripts. OSD is a universal, semantic description of devices; a script is read by one engine only. Scripts stay in 0.7 as a declared extension (`capabilities`, kept), opaque to OSD; they remain needed in some cases (for example a challenge in the discovery of a device). OSD says one thing about them: a script designates another control by its address, never by a number. The name and the form of the function a script calls belong to the engine's own contract (osc-bridge publishes its own), not to OSD. The converter 0.6 to 0.7 does not rewrite the text of a script; the rewriting of old scripts belongs to the tools of the engine that runs them.
8. Virtual parameters. A parameter that sends no message (Electra One's `virtual` type) is a behaviour of a controller, not a property of the device: 0.7 has no field `virtual` and no control without a way for it. Its data stays as a note, with its source. Meta-parameters are left to a later proposal.
9. Source types. 0008 withdraws the single type `partner-data`: the provenance of a source is the object of proposal 0010 (a source cites a public table of makers, firmwares and partners), and 0008 refers to it.

This text quotes no figure: for each field it says whether a partner produces or reads it, and what an exchange would lose.

## The change

### 1 The key `_sources` becomes `sources`, and `_` is only a note

`_sources` becomes `sources`; `_midi_of_sources` (written by the resolver, `osd/src/resolve.rs`, lines 262 and 308) becomes `midi_of_sources`.

**The rule on a key starting with `_`, in one sentence, the same as proposal 0007's: a reader accepts it, keeps it and writes it back, and no program reads its meaning.** The reader does today what this says (`osd/src/json.rs`, lines 97-108: the keys starting with `_` are kept as remarks and written back as they are), so no information is lost. A publisher keeps its notes under `_`, and no note is refused. The converter obeys the same rule on its side: it deletes no text written for people (section 3 shows, for each key it still drops, that the key is not such a text).

A 0.7 descriptor that carries `_sources` is always refused, alone or beside `sources`, as `BadValue` at `/_sources` (not as `UnknownField`). `_sources` is the one key starting with `_` that a reader looks at: the refusal is a refusal of migration, so that a descriptor never credits its sources under the old name; the reader does not read the key's value. Every other key starting with `_` is kept. There is no soft migration: the converter 0.6 to 0.7 turns a 0.6 `_sources` into `sources` and writes no `_sources`.

**The keys starting with `_` that programs read in 0.6, and what each becomes in 0.7.** This table is a list of migrations: it says what the converter does with each key, not a reading that stays in 0.7; after 0.7 no program reads a key starting with `_`. The search that established the list: `grep -n '"_[a-z_]*"'` over `osd/src` and `osd-formats/src` (code that reads or writes a key by name), read against the schema of 0.6 (`schemas/0.6/*.json`, keys `_sources` and `_coverage`) and the text of 0.6 (`_usage`, `_limitations`, `_profile`, `_midi_of_sources`).

| Key in 0.6 | Read by | What the converter makes of it |
|---|---|---|
| `_sources` | the reader (`osd/src/device.rs:69`), the resolver, the credit of the converters | `sources` (a change of form); a 0.7 descriptor that holds `_sources` is refused |
| `_midi_of_sources` | the resolver (`osd/src/resolve.rs:262`) | `midi_of_sources` (a change of form) |
| `_usage` | the CSV of midi.guide (`osd-formats/src/midi_csv.rs:53, 545`) | `value_description` (a change of form, below) |
| `_name` | the converters of Electra One, MIDNAM and the CSV, on the entries of `cc_params` (`electra_one.rs:637`, `midnam.rs:528`, `midi_csv.rs:539`) | `title`, on an entry of `cc_params` as on an entry of `params`, as proposal 0007 decides |
| `_description` | the CSV of midi.guide and of triggers (`midi_csv.rs:543`, `midi_triggers_csv.rs:39`) | `description`, on an entry of `cc_params`, of `params` and on a trigger, as proposal 0007 decides |
| `_notes` | the CSV of midi.guide (`midi_csv.rs:544`, a column Notes written back) | `notes` on a control (a change of form, below) |
| `_electra` | osc-bridge, the engine that runs the scripts | `id` on a command for a `sysex` shape; `_electra` left as a note, under its `_` key, for any other shape, `virtual` included (section 2) |
| `_coverage` | the reader only stores it and writes it back (`osd/src/device.rs:71`); no converter reads its value | stays a note, kept; not removed (section 3) |
| `_limitations` | no program | stays a note |

`_profile` in `_profile` and `_profile.<family>` is the name of a file, not a key. `_note`, `_comment` and `_notes` elsewhere stay notes, as proposal 0007 keeps them.

### 2 The changes of form

Each row: the old field, the new field, what the converter writes. A partner "reads" a field when its format has a place for the fact; "produces" when its descriptors carry it.

| Old field | New field | The converter writes | Partner, and what a round trip loses |
|---|---|---|---|
| `orientation` of a control | `polarity`, absent / `unipolar` / `bipolar` | `centered` becomes `bipolar`, `0-based` becomes `unipolar`, absent stays absent, any other value is `NotConvertible` at the field | midi.guide: its CSV has an orientation column, read and written back; it writes only the two words (`osd-formats/src/midi_csv.rs:336-341`, any other word is already a loss of the import, line 566 writes back the two). The round trip centered↔bipolar and 0-based↔unipolar is exact: nothing lost. Electra One and KnobKraft Orm have no field for it: nothing to lose |
| `_usage` | `value_description`, a text for people of what the values mean, beside `values` | the text unchanged | midi.guide: a usage column, read and written back; nothing lost |
| `device.manufacturer_id` | `identity.manufacturerId` | the same bytes, except in a file with `midi_of` (below) | KnobKraft Orm: its adaptations carry a real manufacturer byte in most of their descriptors; nothing lost. Property Exchange DeviceInfo names the fact `manufacturerId` |
| `_notes` of a control | `notes`, a text on the control, with the meaning `notes` has elsewhere (reserve, history: the sorting rule of proposal 0007) | the text unchanged | midi.guide: a Notes column, read and written back; nothing lost |
| `_electra`, `{"parameter": {"type": "sysex", "number": N}}` on a command | `id`, an integer on the command | `N` | Electra One: produces it. The round trip keeps the number |
| `_electra`, `{"parameter": {"type": "virtual", "number": N}}` on a command | none: no `id`, no `virtual`; the key stays a note | nothing: `_electra` is kept and written back under its `_` key | Electra One: produces it; the note keeps the data, and the entry of `sources` that credits the preset keeps its source. Nothing lost |
| `_sources[].preset_name`, `_sources[].name` | `sources[].title` | the text of `preset_name`; the text of `name` when there is no `preset_name`; the other key stays (below) | Electra One produces the first; nothing lost |
| `_sources[].revision` | `sources[].version` | the text | nothing lost |
| `_sources[].content_sha256` | `sources[].sha256` (the word the sound schema already uses for a file) | the digest | nothing lost |
| `_sources[].preset_id`, `_sources[].adaptation` | none when the end of the source's `url` is the same text; otherwise the key stays as it is | dropped when the `url` ends with it | Electra One and KnobKraft Orm produce them; the `url` holds the same fact; nothing lost |
| `_sources[].type`: `electra-preset`, `pencilresearch`, `knobkraft`, `midnam` | `sources[].type`, the same word | untouched; the provenance of a source, and so the fate of these four words, belongs to proposal 0010 | nothing changes here, so nothing lost |
| any other key of a source | the same key | untouched | nothing lost |
| `_sources` | `sources` | renamed; a 0.7 descriptor that holds `_sources` is refused | none |
| `_midi_of_sources` | `midi_of_sources` | renamed | none |

**`_electra` becomes `id` for a `sysex` parameter; a `virtual` one stays a note.** `_electra` has two real forms, both read from the tests of osc-bridge's Electra One support, the engine that runs the scripts, which keys a parameter by its type and its number together. Both sit on a command, under `parameter`, with a `type` word and an integer `number`:

1. `sysex`: the number is the device's own parameter number.

   ```json
   { "osc": "/level_s1", "_electra": { "parameter": { "type": "sysex", "number": 1 } }, "args": [], "frame": [] }
   ```

   In 0.7: `{ "osc": "/level_s1", "id": 1, "args": [], "frame": [] }`.

2. `virtual`: the number is the one the preset's author gave to a value that the tool holds; the parameter sends no message.

   ```json
   { "osc": "/dev_id", "_electra": { "parameter": { "type": "virtual", "number": 405 } }, "args": [], "frame": [] }
   ```

   In 0.7 the command is the same: no `id`, no `virtual`, and `_electra` stays under its `_` key as a note, kept and written back as it is.

**No field `virtual`, and no control without a way for it.** A parameter that sends no message is a behaviour of a controller, not a property of the device. MIDI 2.0 Property Exchange requires a `ctrlType` on each controller (M2-117-UM, the ChCtrlList entry), so it has no control without a way; OSCQuery knows no MIDI, so it has no such notion either. Neither reference has the notion, and a universal description of devices does not take it in. The data of Electra One is not lost: the `_electra` object stays as a note, and the entry of `sources` that credits the Electra One preset remains its source; the converter writes no source of its own. Meta-parameters (values a tool holds and the controls that reach them) are left to a later proposal.

A `_electra` of any other shape (another `type` word, a `number` that is not an integer, no `parameter`) is not converted either: the key stays where it is, as a note, kept and written back, and no `id` is written; the converter guesses nothing and loses nothing. The engine's table lists other types (`cc7`, `nrpn`, …); no data of the drivers served carries them, and they would go to `ways` by proposal 0009, not to `id`.

The general structure is one optional field of a command, no field of a product: `id`, an integer, written from a `sysex` `_electra`.

`id` is not a field specific to one product. It has the meaning of `params.entries[].id` in 0.6 (`spec/0.6.md`, line 987: "the parameter's own number when the maker numbers its parameters one by one"): the number by which a source designates a parameter, an integer not limited to a byte. Electra One's `sysex` number is exactly that, the number of a parameter of the device; the word `id` is the word 0.6 already uses for it, and nothing in its name or its form belongs to Electra One. It is not an identifier of the command and it is not unique: two commands may reach one parameter, and the descriptors served hold such repeats. The converter copies the number and refuses no repeat. No script designates a control by `id`: the number is a fact of the source, optional and not unique. A script that reads another control designates it by its address.

**The sources: what stays open, and each key.** A source stays open in 0.7, as in 0.6 (`additionalProperties: true`, `schemas/0.6/device.schema.json`, `$defs/source`; the reader keeps "its other keys, as they are", `osd/src/device.rs`, `Source`): a publisher may add a key to a source and no reader refuses it. What changes is that no key the format names is named after a product; the values of `type` are left to proposal 0010 (below). The converter knows six keys of the extra data of a source and each has a fate: `preset_name` and `name` become `title`; `revision` becomes `version`; `content_sha256` becomes `sha256`; `preset_id` and `adaptation` are dropped only when the end of the `url` is the same text (so no fact is lost), and kept otherwise. Every other key of a source is left as it is. Where the target already exists, it wins and the old key stays, as proposal 0007 does for a text: a source with `title` and `preset_name` keeps both; a source with `version` and `revision` keeps both.

**`name` against `preset_name`.** When a source has both and no `title`, `title` is the text of `preset_name`, the name of the thing the source is (the Electra One preset), and `name` stays as it is; when it has `name` alone, `title` is the text of `name`. A source that has a `title` keeps it, and both old keys stay. So no text is lost and the converter never picks between two names by deleting one.

**The source types named after a product** (`spec/0.6.md`, section 6.6, line 256 and the following: `electra-preset`, `pencilresearch`, `knobkraft`, `midnam`) are left as 0.6 has them: the converter 0.6 to 0.7 does not touch a source's `type`, and rewrites no `notes`. The provenance of a source (a source cites a public table of makers, firmwares and partners), and so the fate of those four words, is the object of proposal 0010; the rule that no product name stands in a field is held by 0010, not by this proposal. The text of 0.7 for section 6.6 keeps its four rows until 0010 is accepted.

**`device.device_id` stays where it is.** In 0.7 `device.device_id` keeps its place and its form: its original intention was not found (proposal 0005), and two fields are not merged, nor one moved under the name of the other, without knowing that they say the same thing. Only `manufacturer_id` moves to `identity`.

**`identity.manufacturerId` in a file with `midi_of`, and under a null key.** A file with `midi_of` replaces the block `identity` of its target whole, not key by key (`spec/0.6.md`, section 23.8), and takes the keys of `device` other than the five identity keys from its target (23.4: `manufacturer_id` is not one of them). Moving the key into `identity` therefore changes who supplies it, and 0.7 says how:

- The block is whole, `manufacturerId` included: a 0.7 file with `midi_of` and an `identity` block gives the resolved device that block, and a `manufacturerId` the block omits is absent from the result, as any other key of the replaced block is (23.8). A 0.7 file with `midi_of` and no `identity` block takes its target's `identity`, `manufacturerId` with it, as it took the target's `manufacturer_id` in 0.6.
- The converter moves `device.manufacturer_id` of a 0.6 file into `identity.manufacturerId` when the file has no `midi_of`, or has one and already an `identity` block. A file with `midi_of` and no `identity` block never had its own `manufacturer_id` read (23.4 takes it from the target); writing an `identity` block for it would replace the target's whole block (23.8) and lose the target's keys, so the converter writes no block and keeps the value as a note, `device._manufacturer_id`, kept and written back. Nothing is lost and nothing is replaced.
- `manufacturerId` is a key directly in `identity`, so the rule of a null key (`spec/0.6.md`, section 23.25, lines 1392-1410) applies to it as to the other optional keys of the block: `null` is accepted, and a resolving reader drops the profile's `manufacturerId` from the resolved description when the block sets it to `null`, and accepts a `null` on a profile that does not give it.
- The spelling is camelCase, where `device_id`, `reply_text` and the other keys of `identity` are snake_case. The reference decides (arbitration, first step): Property Exchange's DeviceInfo names the fact `manufacturerId` (M2-105-UM) and the owner validated the name (2026-11-24). The inconsistency inside `identity` is real and stated: a departure from the one-spelling habit of OSD's own keys, taken for the reference's name; a later version may reconsider it.

**A script designates another control by its address.** A script is a field marked `x-osd-extension` (`spec/0.6.md`, line 147): it is read by one engine only, the one that runs it, and OSD keeps it as a declared extension (`capabilities`, kept), opaque to OSD. Scripts remain needed in some cases (for example a challenge in the discovery of a device). OSD defines no script function: the name and the form of the function a script calls belong to the engine's own contract (osc-bridge publishes its own), because a universal format names no engine's function.

OSD says one thing about scripts, a rule on an engine that runs the scripts of a 0.7 descriptor: **a script designates another control by its address, never by a number.** The rule is not a check of a reader: OSD never reads the text of a script, so a reader neither refuses nor warns. It rests on the address, the `osc` of a command (present on every control and unique in the descriptor), and not on `id`, which is optional and not unique. Property Exchange designates a parameter by a path (`paramPath`) and OSCQuery by its address; neither by a maker's number.

**The converter 0.6 to 0.7 leaves the text of a script as it is.** It searches no script and rewrites none. The rewriting of old scripts, 0.6 scripts that read by number, belongs to the tools of the engine that runs them, which alone know the function they call. A 0.6 script that reads by number stays unchanged in a 0.7 descriptor: an assumed loss, written in the tensions below.

**`orientation` example.** A 0.6 control `{"cc": 10, "osc": "/pan", "orientation": "centered"}` becomes in 0.7 `{"cc": 10, "osc": "/pan", "polarity": "bipolar"}`; `"0-based"` becomes `"unipolar"`; a control with no `orientation` has no `polarity`. A 0.6 value outside the two words is `NotConvertible` at `/cc_params/entries/<n>/orientation`; the schema of 0.6 already forbids it.

**The names.** `polarity`, `unipolar` and `bipolar` are the words of MIDI 2.0 ("unipolar and bipolar values", Min-Center-Max scaling, M2-115); `value_description` applies the Property Exchange word `description` to the values; `manufacturerId` is the DeviceInfo key (M2-105-UM). The owner validated these names on 2026-11-24.

### 3 The removals

Each field is dropped by the converter. The piece is the definition in 0.6 and the format of the partner; "readers" are the readers of OSD this removal touches. The loss is ranged: *assumed* (the fact is a template, or in no partner and no reader needs it) or *tension* (a reader of OSD or a rule of 0.6 relies on it; stated in the section on the tensions).

| Field | Definition in 0.6 | Partners | Readers of OSD touched | Loss |
|---|---|---|---|---|
| `transform_reverse` | a Lua expression, entries of `cc_params` and `params` (`osd/src/device.rs:1186, 1267`) | none of the three has it | the reader (`device.rs:339` counts it among the fields that require the `lua` capability); the Electra One converter lists it as lost (`electra_one.rs:646`) | tension: it is the one half of a Lua pair with `transform`, and a script that runs only forward loses the way back |
| `device.revision` | template text (`osd/src/device.rs:947`) | none; its value is a template | the resolver reads it as an identity key of a file with `midi_of` (`spec/0.6.md:1202`) | tension: an identity key leaves the list; provenance is in `sources[].version` and `firmware` |
| `transport.rate_limit_hz` | the most messages a second on a transport (`osd/src/device.rs:1007`) | none | none found | assumed (the `device.rate_limit_hz` at `device.rs:954` stays) |
| `sysex.footer` | a list of bytes (`osd/src/device.rs:91`) | none; always `F7` | the frames of a SysEx message | assumed: the byte is fixed by MIDI |
| `midi_out.note_offset` | an integer (`osd/src/device.rs:132`) | none; never non-zero | none found | assumed |
| `identity.sends` | what the device sends by itself (`osd/src/device.rs:1050`) | none | none found (the `sends` of `fingerprint.rs:30` is another field and stays) | assumed |
| `sources[].imported_at` | a date of import (`osd/src/device.rs:1477`) | none | the credit of the converters (`osd-formats/src/credit.rs:28` lists it as not carried) | assumed |

**None of the seven is a text for people.** The converter deletes no text written for people (the rule of section 1), so each key it still drops carries its reason and the piece that shows it:

| Field | What it holds, and who reads it |
|---|---|
| `transform_reverse` | A Lua expression: code that a script engine runs (`osd/src/device.rs:1186, 1267`, `x-osd-extension`); read by osc-bridge's engine, not by people |
| `device.revision` | A value the resolver reads as one of the five identity keys of a file with `midi_of` (`spec/0.6.md`, section 23.4; `osd/src/resolve.rs:61`, `IDENTITY`): a program reads it |
| `transport.rate_limit_hz` | A number of messages a second (`osd/src/device.rs:1007`); no text |
| `sysex.footer` | A list of bytes, always `F7` (`osd/src/device.rs:91`); no text |
| `midi_out.note_offset` | An integer, never non-zero (`osd/src/device.rs:132`); no text |
| `identity.sends` | What the device sends by itself, read by a reader as a structure (`osd/src/device.rs:1050`); no text for people |
| `sources[].imported_at` | A date of import, a machine's stamp (`osd/src/device.rs:1477`); `osd-formats/src/credit.rs:28` lists it as not carried |

`_coverage` is not in the table: it is not removed. Its schema in 0.6 says "What the source covers, counted" (`schemas/0.6/device.schema.json`, key `_coverage`), a map that no program reads the value of (the reader only stores it, `osd/src/device.rs:71`, and the converters list it as lost) and that may hold text written for people. A key whose content is not shown to be no text is not removed: in 0.7 it is a key starting with `_`, accepted, kept and written back. The rule of section 1 holds.

**`capabilities` stays.** 0.7 does not remove `capabilities`. The architecture says "A descriptor that needs a script declares the capability… a reader that runs no code knows, when it loads a descriptor, what it cannot do" (`docs/ARCHITECTURE.md`, line 30), and that a version before 1.0 "adds and renames only through its converter" (line 29): a removal is not on that list. The field, the sections 23.14 and the rules on a `midi_of` file that adds a script (`spec/0.6.md`, sections 23.9 and 23.14) keep their object and read as they do in 0.6. The removal of `transform_reverse` above changes only which fields require the capability. The converter therefore removes from `capabilities` the capability that only a removed field required (`lua`, when no other field holds Lua after the removals), and the key when the list becomes empty; it adds and corrects nothing else, so the converted descriptor declares what its fields require.

**What stays.** `role`, `role_vocabulary`, `stands_for` and `fidelity` stay in 0.7, and so do the sections of 0.6 built on them (20, 23.35 and 23.36). The reason: no partner produces them, but the catalogue is stored in 0.5, so its not holding them proves nothing yet; they are measured again once the catalogue has passed to 0.6, and removed then only if the measure shows nothing reads them.

## What stays outside

**The display of a controller stays outside the descriptor of a device in 0.7**: the place, the page and the colour of a control on an Electra One screen. The owner's reason (decision 6): its complexity; it may be reconsidered in a later version, if the same need appears for virtual synths and devices built on Linux, not now. The look of a control is a fact of a tool that lays it out, as the converter says today (`electra_one.rs`, the constant `LOOK`: "the descriptor says what a device speaks, not a screen"). A later version may reconsider it.

Not in this proposal: `also`, `transmit`, `recognize` and the flat fields of a control, which a partner needs; they go into one structure of the ways of a control, in a sibling proposal. `_limitations` and the other free notes stay notes.

## The cases of conformance, as they would be written

Under `conformance/0.7/convert/`:

1. `polarity-from-orientation` — a 0.6 device with `orientation: "centered"` on one control, `"0-based"` on another and none on a third; the 0.7 output has `bipolar`, `unipolar` and no `polarity`.
2. `orientation-other-word-not-convertible` — an `orientation` word outside the two, in a 0.6 text that a converter receives without having checked it against the schema; verdict: the converter refuses it, with the kind of error `NotConvertible` (`spec/0.6.md`, section 4: "an element that a converter cannot turn into the later version"), at the pointer `/cc_params/entries/<n>/orientation`.
3. `usage-becomes-value-description` — `_usage` on a control; the 0.7 output has `value_description` and no `_usage`.
4. `electra-number-becomes-id` — a command with `_electra` of type `sysex` and one number, and a command with `_electra` of type `virtual`; the output has `id` with the number on the first; the second gets neither `id` nor `virtual` and keeps its `_electra` as a note. A second command with the same `sysex` number is converted: the converter keeps both, refuses neither. An `_electra` of another shape stays as a note, with no `id`.
5. `manufacturer-id-moves-to-identity` — `device.manufacturer_id` moves to `identity.manufacturerId`; `device.device_id` stays where it is. A file with `midi_of` and no `identity` block gets no `identity` block: its value stays as the note `device._manufacturer_id`.
6. `source-product-keys-become-general-fields` — a source with `preset_name`, `revision`, `content_sha256`, `preset_id` and a `url` that ends with it; the output has `title`, `version`, `sha256`, the `url` and none of the rest. A source with `preset_name` and `name` keeps `name`; a source of type `electra-preset` keeps that type and its `notes` as they are: the converter does not touch a source's `type`, and the provenance of a source belongs to proposal 0010.
7. `sources-key-renamed` — `_sources` and `_midi_of_sources` become `sources` and `midi_of_sources`.
8. `removed-fields-dropped` — one descriptor with each of the seven removed fields; the output has none. `capabilities` and `_coverage` stay.
9. `script-text-unchanged` — a descriptor with a script, one that reads another control by a number among them; the output holds the text of every script unchanged, and `capabilities` still declares it. No case of OSD reads the text of a script.
10. `notes-column-becomes-notes` — `_notes` on a control; the output has `notes` and no `_notes`.

Under `conformance/0.7/read/`:

11. `underscore-sources-refused` — a 0.7 descriptor with `_sources`, alone or beside `sources`: refused as `BadValue` at `/_sources`.
12. `underscore-key-is-kept` — `_limitations` on a control: accepted; the reader keeps the key and writes it back as it is, and no program reads its meaning. The same verdict as case 13 of proposal 0007.

## What a reference does

- **MIDI 2.0 Property Exchange** (DeviceInfo, M2-105-UM; controller resources, M2-117-UM; M2-115 for unipolar and bipolar values), read at its source, never copied here. OSD reuses its names where they overlap: `manufacturerId`, `description`, unipolar and bipolar. A departure is not made here.
- **OpenAPI and JSON Schema.** A field removed in a new version is carried by a converter, the old version staying valid; the converter 0.6 to 0.7 does so. Read in the proposals already accepted into the process (`spec/PROCESS.md`, section 5).
- **The partners** (Electra One, midi.guide / pencilresearch, KnobKraft Orm): their formats were read through the readers and converters of OSD (`osd-formats/src`). This text says what each partner produces or reads; it quotes no figure.

## The tensions with the intentions of OSD

- **Compatibility.** A descriptor valid in 0.6 reads in 0.7 through its converter, and the converter refuses only what cannot become a field of 0.7. One removal weighs on a 0.6 rule: `device.revision` (identity key of `midi_of`, `spec/0.6.md:1202`). It loses a fact that no partner carries; the cost is on a rule of the OSD text, stated here. The fields the settings of a sound and section 20 rest on stay.
- **One meaning per field.** `id` on a command keeps the meaning `params.entries[].id` has; `value_description` and `description` are two texts of two things (the values, the control). `device.device_id` stays in `device` because 0.6 already has `identity.device_id`, a rule with `min` and `max` (`osd/src/device.rs:1041`, `spec/0.6.md:354`), and the list of bytes of `device` is another meaning: moving it under the same name would put two meanings under one name, and merging the two is not done without knowing they say the same thing.
- **A script that reads another control by number: an assumed loss.** A 0.6 script that reads by number stays as it is in a 0.7 descriptor, because the converter reads no script and the format holds no script contract (`spec/0.6.md`, line 147). Compatibility is weighed: the converter keeps the descriptor valid, the capability still declares the script, and the tools of the engine rewrite the text. The converter emits no warning, since it reads no script.
- **Names reused from MIDI, MIDNAM and OSCQuery.** `polarity`, `manufacturerId` and `description` are Property Exchange's; `title` and `version` are common words; `sha256` is the word the sound schema uses. No new name is invented where one exists.
- **Credit.** `sources` is read by programs as it always was; credit now holds in an ordinary field. A source loses its product keys, and its `url`, `title` and `version` hold the same facts. Nothing of the rule changes.
- **The architecture: a script is declared.** `docs/ARCHITECTURE.md`, line 30, says "A descriptor that needs a script declares the capability"; 0.7 keeps `capabilities` for that reason, and a version before 1.0 only adds and renames through its converter (line 29), so no removal of a declared fact is made.
- **A key starting with `_`.** The rule is one sentence (accepted, kept, written back, never read by a program), the reader's behaviour today; the converter deletes no text for people, and the seven dropped fields are shown not to be such a text. No tension with compatibility remains: nothing is dropped that a reader of 0.6 kept for people.
- **The `identity` block of a file with `midi_of`.** `manufacturerId` follows the whole replacement of 23.8, so a 0.7 file that writes an `identity` block must write the key; the converter never writes a block that would replace the target's.
- **No reader privileged.** The product-key shapes (`preset_name`, `adaptation`) disappear from the format; Electra One's `sysex` number is carried as a general `id`, and its other forms as a note, their source by `sources`. The product-named source types (`electra-preset`, `pencilresearch`, `knobkraft`, `midnam`) stay as 0.6 has them: this tension stays open until proposal 0010, which decides the provenance of a source, settles them.

## Objections

None yet.

## Questions for the owner

The owner answered questions 1 to 9 on 2026-11-24; none remains open. Each answer is written into the text where it decides, and recorded here.

1. **A script that reads another parameter by its number.** Answered: the script reads the other control by its address (the identifier of a control in OSD: on every control, unique, stable from one device to the next). `id` stays a fact of the device, optional and not unique; no script designates a control by it. Scripts stay a declared extension, opaque to OSD; the converter leaves their text as it is. See "A script designates another control by its address" in section 2.
2. **`_notes` of midi.guide.** Answered: a field `notes` on a control, with the meaning of `notes` elsewhere (reserve, history: the sorting rule of proposal 0007).
3. **`role`, `role_vocabulary`, `stands_for`, `fidelity`.** Answered: they stay in 0.7; they are measured again once the catalogue has passed to 0.6.
4. **`device.device_id`.** Answered: it stays where it is in 0.7, its original intention not having been found (proposal 0005).
5. **`_usage` and proposal 0007.** Answered: settled; proposal 0007 says `value_description`.
6. **A 0.7 descriptor with `_sources`.** Answered: always refused, no soft migration; the converter turns a 0.6 `_sources` into `sources`.
7. **`capabilities`.** Answered: kept (the owner's decision of 2026-11-24); this proposal no longer removes it.
8. **`_notes` of a control against 0007.** Answered: the latest decision wins, `notes`; proposal 0007 says the same.
9. **The two new names, `virtual` and `partner-data`.** Answered: neither is proposed. `virtual` is a behaviour of a controller, not a property of the device: 0.7 has no field `virtual` and no control without a way for it (Property Exchange requires a `ctrlType`, M2-117-UM; OSCQuery knows no MIDI); the `virtual` form of `_electra` stays a note, with its source; meta-parameters are left to a later proposal. `partner-data` is withdrawn: the provenance of a source is the object of proposal 0010, to which this proposal refers.

What was searched: the text of 0.6 (sections 4, 7, 20, 21, 23), the device schema and the readers of OSD (`osd/src`, `osd-formats/src`), the proposals 0003 to 0007, and the owner's decisions of 2026-11-24.
