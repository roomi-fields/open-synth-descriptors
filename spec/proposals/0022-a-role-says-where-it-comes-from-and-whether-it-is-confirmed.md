# 0022 — A role says where it comes from and whether it is confirmed

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: the role of an entry of `cc_params` and of `params` gains three optional keys, `origin`, `state` and `source`.

## Why

What this brings to the sharing of sounds on devices: roles laid in mass say where they come from and whether they are to confirm, so an agent trusts each as it deserves.

A role says what a control does in the words every device shares. Roles can be laid in mass, by a rule, from the MIDI standard, from another description, by a small model, from a public source or by the agent of a user. In 0.7 a role is `{ "parts": [...], "setting": term }`: a role that a model proposed and a role that a person checked are written alike, and a role found to be wrong can only be removed, so the next laying may propose it again.

### The use: the six levels of the method of SyntHub

The use is the method of SyntHub to lay roles, which has six levels (owner, 2026-11-27). Each has its word:

| Level of the method of SyntHub | `origin` |
|---|---|
| rule | `rule` |
| MIDI norm | `norm` |
| inheritance | `inheritance` |
| small model | `model` |
| public source | `source` |
| agent of the user | `contributor` |

## The change

### The three keys

| Key of a role | Form | Says |
|---|---|---|
| `origin` | one of `rule`, `norm`, `inheritance`, `model`, `source`, `contributor`, or an extension `x-<registrant>-<term>`; optional | where the role comes from |
| `state` | one of `valid`, `to-confirm`, `refuted`; optional, only beside `origin` | what the role is worth, as the service that wrote the descriptor holds it |
| `source` | string, never empty; optional, only beside `origin` | the `id` of the source that gave the role |

A word of `origin` is compared as written, case included. An extension has the form of section 6.2 of [0.7](../0.7.md). `origin` may be written without `state`. `state` and `source` are written only with `origin`. A role without `origin` stays valid and reads as today. `source` is optional whatever the origin.

The keys belong to the role of a control: an entry of `cc_params` and an entry of `params`. The objects `source` and `destination` of [draft 0021, a source or a destination of modulation](0021-a-source-or-destination-of-modulation-carries-its-meaning.md) are not roles of a control, and take none of the three keys.

Two names are used twice and mean different things:

- the `state` of a role is what that role is worth. It is not the state of the vocabulary that `role_vocabulary` names, which says which terms the role is read against;
- `source` as a word of `origin` says what kind of origin the role has. The key `source` of a role names one entry of the sources. Neither is the field `source` of a named value of draft 0021, which says which signal a choice of a selector takes.

### What each origin says

| `origin` | What it says of the role |
|---|---|
| `rule` | A deterministic rule of the publisher set it from what the description says of the setting, such as its name and its section. |
| `norm` | It comes from the meaning that the MIDI standard gives to the message that reaches the control. |
| `inheritance` | The publisher took it over from a role set or proposed on the same message of another description of the same device (another source, another firmware, the target of a `midi_of`) and writes it in this file. |
| `model` | A language model proposed it. |
| `source` | A public source says it. |
| `contributor` | A person, the author of the descriptor included, or an agent acting for a user proposed it. |

A role that a resolution takes from the target of a `midi_of` keeps the keys written in the target: it does not become `inheritance`.

### What the `source` of a role names

The `source` of a role is judged as the `source` of a reading (section 5.3.4 of 0.7):

- in a resolution, for a file without `midi_of`, the end of a `midi_of` chain included, it is first an `id` of the `sources` or `midi_of_sources` of that file itself, whether the join keeps the role or discards it, and a refusal is in that file;
- in a resolution, for a file with `midi_of`, it is an `id` of the `sources` or `midi_of_sources` written in the file asked for and its `midi_of` targets, without the sources the resolution adds from profiles; such a role that the joined device does not keep is not judged;
- in a file without `midi_of` read without being resolved, it is an `id` of an entry of its `sources` or `midi_of_sources`, an entry that carries `profile` included;
- in a `midi_of` file read alone, it is not judged: a reader MUST NOT refuse the file for it, and the resolution judges it.

### A role carries no firmware

A reading carries the firmware it holds for, because two accounts of one control may differ from one firmware to the next. A role has no such key. A device file describes one device from one source, and a firmware with its own MIDI is its own file (section 5.3 of 0.7): the role holds for what its file describes. The source that `source` names can say the firmware it documents. A key of firmware on a role would call for one role per firmware, side by side, and a control has one role.

### The schema

The fragment shows what changes in the device schema. The `role` of the entries of `cc_params` and of `params` refers to a new definition, `control_role`: the `role` of 0.7 with three more properties. The definition `role` does not change, so whatever else refers to it gains no key.

```json
{
  "$defs": {
    "control_role": {
      "type": "object",
      "required": ["parts", "setting"],
      "additionalProperties": false,
      "patternProperties": { "^_": true },
      "dependentRequired": { "state": ["origin"], "source": ["origin"] },
      "properties": {
        "parts": { "type": "array", "minItems": 1, "items": { "$ref": "#/$defs/role_part" } },
        "setting": { "type": "string", "pattern": "^[a-z][a-z0-9-]*$" },
        "origin": {
          "anyOf": [
            { "enum": ["rule", "norm", "inheritance", "model", "source", "contributor"] },
            { "type": "string", "pattern": "^x-[a-z0-9]+-[a-z0-9-]+$" }
          ]
        },
        "state": { "$ref": "#/$defs/reading/properties/state" },
        "source": { "type": "string", "minLength": 1 }
      }
    }
  }
}
```

### An example

An invented device, "Brume". Its firmware 2.0 has its own file, whose author took one role from the description of firmware 1.0 and proposed another. The members of the descriptor that the keys touch:

```json
{
  "role_vocabulary": "1",
  "sources": [
    { "id": "brume-1", "type": "community-re", "firmware": "1.0", "author": "A. Example",
      "url": "https://example.org/brume/1.0.device.json", "license": "CC0-1.0" },
    { "id": "bench", "type": "hardware-observed", "firmware": "2.0", "author": "B. Example",
      "license": "CC0-1.0" }
  ],
  "cc_params": {
    "entries": [
      { "osc": "/filter/cutoff",
        "role": { "parts": [{ "part": "filter" }], "setting": "cutoff",
                  "origin": "inheritance", "state": "to-confirm", "source": "brume-1" },
        "ways": [{ "ctrlType": "cc", "ctrlIndex": [74] }] },
      { "osc": "/osc2/tune",
        "role": { "parts": [{ "part": "oscillator", "index": 2 }], "setting": "pitch",
                  "origin": "contributor" },
        "ways": [{ "ctrlType": "cc", "ctrlIndex": [36] }] }
    ]
  }
}
```

The first role comes from the description of firmware 1.0, which its `source` credits, and is not confirmed. The second was proposed by a contributor, and nothing is said of its confirmation.

### What a reader does

A reader gives `origin`, `state` and `source` as written and writes them back. It computes none of them.

A reader accepts an extension of `origin` that it does not know, and treats it as an unknown origin.

The format draws no rank and no degree of trust from an origin: how far a tool trusts a role of a given origin belongs to the tool.

The three words of `state` are those of the readings (section 5.3.4 of 0.7):

| `state` | What a reader and a tool do |
|---|---|
| none | The role serves as today. Nothing is said of its confirmation. |
| `valid` | The role serves as the role. |
| `to-confirm` | The role serves, and a tool that shows it says it is not confirmed. |
| `refuted` | The role is checked as every role is (its form, its terms against the vocabulary), given as written with its state, and written back. It is not offered as a role by default. |

**A refuted role.** By default the control gives no `paramPath`, and it is missing from the settings of a sound and from their comparison (section 12.2 of 0.7). A caller may ask for the refuted roles; the way to ask belongs to the tool.

The use of the word is this. A publisher who has no better role keeps the contradicted one, so a new laying of roles in mass knows that it is set aside. A publisher who has the correct role writes it in its place, with its own origin, and the format then keeps nothing of the former one.

**The fields of other drafts that suppose a role.** [Draft 0020, a named value carries a term](0020-a-named-value-carries-a-term-of-the-vocabulary.md), and draft 0021 add to a control fields that are read against its role. Where either is accepted, a refuted role is still the role of the control for their checks: the fields draw no fault for lack of a role, they are checked against it, and a reader gives them as written and writes them back. By default a tool offers none of them as the meaning of a choice, and does not take the control for a selector or for a part of a route, since each rests on a role that is not offered. A caller that asks for the refuted roles gets them with the role. This proposal depends on neither draft.

### What a reader refuses

`R` stands for `/cc_params/entries/N/role` or `/params/entries/N/role`.

| Fault | At | Kind | Seen by |
|---|---|---|---|
| `state` without `origin` | `R/origin` | `Missing` | the schema and the reader |
| `source` without `origin` | `R/origin` | `Missing` | the schema and the reader |
| an `origin` that is neither a word of the list nor of the form of an extension (`"Rule"`, `"guess"`, a value that is no string) | `R/origin` | `BadValue` | the schema and the reader |
| a `state` outside the three words | `R/state` | `BadValue` | the schema and the reader |
| a `source` that is empty or no string | `R/source` | `BadValue` | the schema and the reader |
| a `source` that names no `id` of the set it is judged in | `R/source` | `BadValue` | the reader |
| `origin`, `state` or `source` in a role of a descriptor of 0.6 or 0.7 | `R/origin`, `R/state`, `R/source` | `UnknownField` | the schema of that version and the reader |
| the same keys in a descriptor of 0.4 or 0.5, which has no role | `R` | `UnknownField` | the schema of that version and the reader |

The schema sees the form of each key and the two keys that call for `origin`. Whether a `source` names a source is the reader's alone.

### Cases

Each row is the role of the first entry of `cc_params` of a descriptor of the version, otherwise valid, with no `midi_of`. In 0.8 and 0.7 its `sources` hold one source, of `id` `bench`; the descriptors of 0.6 and 0.5 have no `sources`. All but the one of 0.5 name a `role_vocabulary`. `B` stands for `"parts": [{ "part": "filter" }], "setting": "cutoff"`, and `R` for `/cc_params/entries/0/role`.

| Version | Role | Verdict |
|---|---|---|
| 0.8 | `{ B }` | admitted, read as today |
| 0.8 | `{ B, "origin": "rule" }` | admitted: an origin without a state |
| 0.8 | `{ B, "origin": "norm", "state": "valid" }` | admitted |
| 0.8 | `{ B, "origin": "model", "state": "to-confirm" }` | admitted |
| 0.8 | `{ B, "origin": "source", "state": "refuted", "source": "bench" }` | admitted: no `paramPath` by default |
| 0.8 | `{ B, "origin": "contributor", "source": "bench" }` | admitted |
| 0.8 | `{ B, "origin": "x-acme-survey" }` | admitted: an unknown origin |
| 0.8 | `{ B, "state": "valid" }` | `Missing` at `R/origin` |
| 0.8 | `{ B, "source": "bench" }` | `Missing` at `R/origin` |
| 0.8 | `{ B, "origin": "Rule" }` | `BadValue` at `R/origin` |
| 0.8 | `{ B, "origin": "guess" }` | `BadValue` at `R/origin` |
| 0.8 | `{ B, "origin": "rule", "state": "confirmed" }` | `BadValue` at `R/state` |
| 0.8 | `{ B, "origin": "source", "source": "" }` | `BadValue` at `R/source` |
| 0.8 | `{ B, "origin": "source", "source": "manual" }` | `BadValue` at `R/source`, by the reader alone |
| 0.7 | `{ B, "origin": "rule" }` | `UnknownField` at `R/origin` |
| 0.6 | `{ B, "origin": "rule" }` | `UnknownField` at `R/origin` |
| 0.5 | `{ B, "origin": "rule" }` | `UnknownField` at `R` |

### The converters

The converter from 0.7 writes no `origin`, no `state` and no `source`. It draws none of them from a note, from the sources or from the readings. A role without these keys stays as it is.

A converter to a format that has not these keys (0.7 and earlier, the `paramPath` of MIDI 2.0 Property Exchange, MIDNAM, OSCQuery):

- writes a role with no state, a `valid` one or a `to-confirm` one wherever it writes a role today, and names in its losses each `origin`, `state` and `source` written, at its pointer;
- does not write a `refuted` role, and names it as lost.

### Open points

The owner decides them at the acceptance.

- A contradicted role whose origin the publisher does not know cannot be written: `state` calls for `origin`, and none of the six words says that the origin is unknown.
- A second, competing role on one control, as when two origins disagree and neither is refuted, has no place.

## What a reference does

| Reference | What it does of the need | Read for this draft |
|---|---|---|
| The readings of OSD 0.7, sections 5.3.4 and 5.3.6 | Several accounts of one command or control sit side by side, each with its `state` (`valid`, `to-confirm`, `refuted`), its firmware and, optionally, the `id` of its source. A `refuted` one is kept as a record and is not offered by default. A reader passes the state on and never computes it. | Read in the text of 0.7 on 2026-11-27. |
| Wikidata, ranks (`https://www.wikidata.org/wiki/Help:Ranking`) | Ranks annotate "multiple values for a statement": `normal` by default, `preferred` or `deprecated`. For templates and queries, "deprecated statements will never be used unless that is specifically requested". | Read at the source on 2026-11-27. |
| Wikidata, references and deprecation (`Help:Sources`, `Help:Deprecation`) | "references are used to point to specific sources that back up the data provided in a statement". A value to set aside is ranked deprecated "(and not removed)", which "allows other users to know not to re-add the value to the item". | Read at the source on 2026-11-27. |
| Wikidata, qualifiers (`Help:Qualifiers`) | A qualifier of a statement can "constrain the validity of the value". | Read at the source on 2026-11-27. |
| MIDI 2.0 Property Exchange, controller resources (M2-117-UM v1.0) | A controller has a `paramPath`. [Draft 0006](0006-a-command-carries-its-readings-each-with-its-state-and-its-firmware.md) reports no state of confirmation and no alternative values. | Not read again at its source for this draft: taken from the text of OSD and from draft 0006, and to be read again before the proposal is a candidate. No claim is made here on a field for the origin of a `paramPath`. |
| MIDNAM (`MIDINameDocument10.dtd`) | A `Control` has a `Type`, a `Number`, a `Name` and its values: no role, no origin and no state. The document names one `Author`. | The DTD kept in `osd-formats/tests/data/`, read on 2026-11-27. |
| OSCQuery (`github.com/Vidvox/OSCQueryProposal`, `README.md`) | An optional attribute such as `EXTENDED_TYPE` says what a value means, and `TAGS` describes a node. No optional attribute it lists says where such a word comes from or whether it is confirmed. | Read at the source on 2026-11-27. |

**What OSD takes**: the three words of the readings and their meaning; from Wikidata, a contradicted value that is kept so that it is not proposed again, and skipped unless asked, and a pointer from a value to the source that backs it.

**Where the draft departs, and why.**

- **One role per control, with no competing roles side by side.** The statements of Wikidata and the readings of OSD hold several accounts side by side; a role does not. A role is what a person or an agent asks a control by, and what the settings of two sounds are compared by (section 12.2 of 0.7): one control gives one answer. A second role would ask every reader to choose between them.
- **An origin is a word of a list.** Wikidata says where a value comes from by a reference that points to a source. A rule or a model has no source to point to, and a program compares a word that the schema checks without reading a text. The key `source` keeps the pointer where there is a source.
- **No firmware.** A reading and a qualifier of Wikidata bound where a value holds. A role holds for what its file describes, and a firmware with its own MIDI is its own file.
- **No neutral rank.** As for the readings, a role of which nothing is said has no `state`, and no word says that one role is preferred.
- **An origin and a state, which MIDNAM and OSCQuery do not write**, and of which draft 0006 reports no state in Property Exchange. Reason: roles laid in mass are not all worth the same, and the agent that reads them needs to know it.
## Objections

None yet.
