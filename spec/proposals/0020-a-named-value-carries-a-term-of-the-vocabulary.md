# 0020 — A named value carries a term of the vocabulary of roles

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: a named value of a control (section 8 of [0.7](../0.7.md), `values`) gains one field, `term`, a value term of the vocabulary of roles.

## Why

What this brings to the sharing of sounds on devices: a host knows that the choice a maker calls "LPF 24" on one device and "Low Pass" on another is the same filter type, so a sound or a gesture carries from one to the other.

In 0.7 a named value is `{ "value": integer, "title": string }`. Its title is the maker's word, free text. [State 2 of the vocabulary](../vocabulary/2.md) holds value terms (`sine`, `low-pass`), and no field of a description can carry one.

## The change

### The form

```json
{
  "osc": "/filter/type",
  "ways": [{ "ctrlType": "cc", "ctrlIndex": [70] }],
  "role": { "parts": [{ "part": "filter" }], "setting": "filter-type" },
  "values": [
    { "value": 0, "title": "LPF 24", "term": "low-pass" },
    { "value": 1, "title": "HPF 12", "term": "high-pass" },
    { "value": 2, "title": "Formant", "term": "x-acme-formant" }
  ]
}
```

(in a description whose `role_vocabulary` is `"2"`.)

`term` is optional, a field of its own of a named value, beside `value` and `title`. It is either a term of type `value` of the state that `role_vocabulary` names, or an extension `x-<registrant>-<term>`, in the form and the registry the roles use (`schemas/extensions.md`). The title stays the maker's word. Two named values of a control can carry one term (two slopes of a low-pass filter).

### What a reader refuses

- A value term on a control with no role: `Missing` at the `role` of the control. The role says which setting the value is a choice of, and it brings `role_vocabulary` with it.
- A value term whose `applies_to` does not name the setting of the control's role: `BadValue` at the `term`. So `low-pass` on the role `/Oscillator/Waveform` is refused.
- A `term` that is no term of type `value` of the state: `BadValue` at the `term`.

A reader checks neither an extension nor a term on a role whose setting is an extension: what an extension touches is not checked, as in a role.

### What a converter does

A converter from 0.7 adds nothing: it writes no `term`, and reads none into a title. A converter to a format with no such field reports what it cannot carry.

## What a reference does

- **SFZ**, opcode `fil_type` (`sfzformat.com/opcodes/fil_type`, read on 2026-11-27): a closed list of words for the filter types, the word being the value itself.
- **MIDI 2.0 Property Exchange**, the `ctrlMapList` of M2-117-UM: a list of named values, each a number and a title. It was not read again at its source for this draft: what [0.7](../0.7.md) says of it is taken over, and is to be read again before the proposal is a candidate.

**The departure.** SFZ writes the shared word as the value itself, and the Property Exchange list carries a title alone. OSD keeps the number and the maker's title, and adds the shared term in a field of its own: a device keeps its own words and its own numbers, and the meaning is shared without a string to read.

## Objections

None yet.