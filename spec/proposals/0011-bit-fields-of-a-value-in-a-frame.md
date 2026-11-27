# 0011 — Bit fields: an argument of a frame says which bits of its value a byte carries

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: the `arg` element of a frame (section 8.1 of [0.7](../0.7.md)) gains the key `bits`, which replaces the encodings `u14_msb` and `u14_lsb` wherever an argument is bound to a value that is sent.

## Why

What this brings to the sharing of sounds on devices: a description gives the caller one value, in its own unit, and says itself how that value is cut into bytes, so any tool sends it.

The need. A device may expect a wide value cut into groups of four bits, or one byte that joins the highest bit of a value and a row number.

What a script does today. It cuts the value into those bytes. In 0.7 the script is the Lua of `commands[].script` (a script that changes the message before it is sent) or of `custom_commands[].script` (a script that builds the message).

What is missing for sharing. The 0.7 frame cuts a value only by the encodings of an argument, in groups of 7 bits. An expression has no division, no shift and no mask, and its results stay from 0 to 127. So any other cut is written in Lua, which a reader without Lua does not run: that reader cannot send the value, and no reader can tell from the script which bits go where.

## The change

### The form

An element `arg` of a frame MAY carry `bits`, an array of two integers `[high, low]`: the bits of the value that the byte carries, both bounds included, bit 0 being the least significant.

| Element | Meaning |
|---|---|
| `{"arg": name, "bits": [high, low]}` | one 7-bit byte: the bits `high` to `low` of the value, moved down so that bit `low` is bit 0 of the byte |
| `{"arg": name, "bits": [high, low], "add": n}` | the same field plus `n` (0 to 127), wrapping on 7 bits: `(field + n) & 0x7F` |

The schema gains one form of the element, beside the two of 0.7:

```json
{
  "type": "object",
  "required": ["arg", "bits"],
  "additionalProperties": false,
  "properties": {
    "arg": { "type": "string" },
    "bits": { "type": "array", "items": { "type": "integer", "minimum": 0, "maximum": 31 }, "minItems": 2, "maxItems": 2 },
    "add": { "type": "integer", "minimum": 0, "maximum": 127 }
  }
}
```

A reader MUST refuse, as `BadValue` at `bits`:

- `high` below `low`;
- a field of more than 7 bits;
- a `low` of 0 on a field that holds the highest value the argument can take (the element without `bits` says it);
- `bits` on an argument that is no integer of 0 or more: a `float`, a `string`, an `s32`, or a range that goes below 0;
- `bits` at a place where the element is not read (the table below).

An element with `encoding` and `bits` is refused by the schema: a byte has one writing.

The highest value of an argument is the upper bound of its `range`; without a `range`, the highest of its type (127 for `u7`, 16383 for `u14`, 1 for `bool`) or of its `values` (`enum`); a value bound by `pre` has 127. An argument of another case (`int` with no `range`), a number of an entry of `params` (`pr`, `p`, `c`, `r`, `id`) and the `value` of `params.set_frame` have no highest value stated here: the third refusal does not apply to them.

A reader SHOULD warn of a command whose frame writes an argument by fields that leave a bit of its highest value in no byte: part of the value is then never sent.

### With the encodings, `add` and `pre`

**One writing for one thing.** Section 8.1 of 0.7 defines the encodings `u14_msb` and `u14_lsb` for an unsigned value that is sent: each is one byte, `u14_msb` the bits 13 to 7, `u14_lsb` the bits 6 to 0. Those are two bit fields. So, in `commands[].frame`, `params.get_frame` and `params.set_frame`:

- `u14_msb` is written `bits: [13, 7]`;
- `u14_lsb` is written `bits: [6, 0]`, or as the element alone when the highest value of the argument is 127 or less;
- the element alone, with or without `add`, writes an argument or a bound value whose highest value is 127 or less (what it writes for a value below 0, which section 8.1 of 0.7 does not define, this draft does not say); on an argument of a command that `bits` can write and that can go above 127, a reader of 0.8 refuses it, as `BadValue` at the element, and the argument is written by fields;
- a reader of 0.8 refuses `u14_msb` and `u14_lsb` at those three places, as `BadValue` at `encoding`.

At every other place the two encodings stay as in 0.7, and `bits` is refused, so each place keeps one writing.

The other encodings stay as they are. `s32` writes a whole signed value in its own bytes; `ascii` and `bytes` write a text or a list of bytes, which have no bits to choose. None of the three takes `bits`.

**Open point.** Section 8.1 of 0.7 does not define the bytes that `ascii` and `bytes` write. The text of 0.8 has to say them; this draft proposes none.

**`add`.** It keeps its meaning of section 8.1: it is added to the byte, here the field, and wraps on 7 bits. It carries nothing from one field to the next.

**`pre`.** An expression does not change. A frame element may name a value bound by `pre` and take a field of it; that value is from 0 to 127, as in 0.7. A field is taken by the frame, after `pre`: no operation of an expression reads one.

**The checksum** is computed on the bytes written, fields included, as in 0.7.

### Where the form is read

The places are the eight of section 8.1, `programs.after[]` (of the form of `before`), the `match_frame` of a sound transfer and the messages of the steps of a file write.

| Place | Sent or recognised | `bits` |
|---|---|---|
| `commands[].frame` | sent | read, on an argument of the command or a value bound by `pre` |
| `params.get_frame` | sent | read, on a number of the entry (`pr`, `p`, `c`, `r`, `id`) |
| `params.set_frame` | sent | read, on a number of the entry and on `value` |
| `identity.request` | sent | refused: no argument is bound there |
| `programs.before[]`, `programs.after[]` | sent | refused: no argument is bound there |
| `programs.dump.request` | sent | refused: no argument is bound there |
| `resources[].via.file_write.steps[].send` | sent | refused: no argument is bound there |
| `identity.reply` | recognised | refused |
| `replies[].match_frame` | recognised | refused |
| `sound_transfers[].match_frame` | recognised | refused |
| `resources[].via.file_write.steps[].reply` | recognised | refused |

**Reading a value back is not covered.** A `capture` keeps one byte. A value that a device sends over several bytes is not put together by this form: a reply that needs it stays a script (`replies[].script`). The places of a value in the data of a sound (section 12.1) keep their own form, whole bytes or a script.

**Open point.** Section 8.1 of 0.7 does not say which bytes the element `{"arg": "value"}` writes in `params.set_frame` under each `value_type`, nor what the element alone writes for a value below 0 or above 127, a number of an entry above 127 included. This draft leaves the element alone as 0.7 has it in `params.get_frame` and `params.set_frame`, and `bits` writes exactly its field under any `value_type`. The text of 0.8 has to say those bytes before this proposal is accepted; the fields that repeat the element alone there have then to be refused.

### Example

An invented device, "Orbe". Its messages start with `F0 7D 10` and end with `F7`; `7D` is the SysEx id kept for non-commercial use.

The filter cutoff goes from 0 to 4095 and travels in three groups of four bits.

```json
{
  "osc": "/filter/cutoff",
  "args": [{ "name": "cutoff", "type": "u14", "range": [0, 4095] }],
  "frame": [240, 125, 16, 2,
    { "arg": "cutoff", "bits": [11, 8] },
    { "arg": "cutoff", "bits": [7, 4] },
    { "arg": "cutoff", "bits": [3, 0] },
    247]
}
```

The value 2748 gives `F0 7D 10 02 0A 0B 0C F7`.

The waveform goes from 0 to 255. Its first byte is 32 plus the highest bit, the second carries the 7 others.

```json
{
  "osc": "/wave",
  "args": [{ "name": "wave", "type": "u14", "range": [0, 255] }],
  "frame": [240, 125, 16, 5,
    { "arg": "wave", "bits": [7, 7], "add": 32 },
    { "arg": "wave", "bits": [6, 0] },
    247]
}
```

The value 200 gives `F0 7D 10 05 21 48 F7`.

### What a reader and a converter do

A reader checks the element against the schema and the refusals above, and gives it back as written. A tool that sends the message writes each field as the form says. No script and no `lua` capability is needed for these cuts.

A converter from 0.7 rewrites, at `commands[].frame`, `params.get_frame` and `params.set_frame`, `u14_msb` as `bits: [13, 7]` and `u14_lsb` as `bits: [6, 0]` or as the element alone, by the rules of "One writing for one thing". It writes an element alone on an argument of a command that `bits` can write and that can go above 127 as `bits: [6, 0]`: this takes the "one 7-bit byte" of 0.7 as the low 7 bits, a reading that 0.7 does not write out; with an `add`, the same `add` stays beside `bits`. It leaves the elements alone of `params.get_frame` and `params.set_frame` as they are (the open point above). It refuses, as `NotConvertible` at the element, `u14_msb` or `u14_lsb` at those three places on an argument that `bits` cannot write. It changes no other place and no script: a script is opaque, and a converter never turns one into fields.

### What stays a script

The owner decided that Lua stays, as the declared and opaque extension, for the rarer and more complex cases. For this need these stay a script:

- a computation on the whole value before it is cut (a multiplication, a division, decimal digits);
- one byte that joins a field of a wide value to another argument: an element names one argument and adds a constant (two values that each fit from 0 to 127 may be joined by `pre`, as in 0.7);
- a number of bytes that depends on the value;
- putting a value back together from the bytes a device sends.

A script is needed there because each asks for a computation, and a frame states only which bits go where.

## What a reference does

None of the three norms below was read at its source for this draft: each statement is taken over from an earlier exploration of the need, and is to be read again at the source before the proposal is a candidate.

- **MIDI 2.0 Property Exchange.** The controller list (M2-117-UM) describes no setting sent by SysEx, so it says nothing of the cut.
- **MIDNAM.** It lets a SysEx multiply and shift the channel or the device id, with no value of a setting; an affine transformation does not take a bit field.
- **OSCQuery.** It describes OSC arguments and no MIDI byte.

The form therefore extends the frame elements of the format. A controller format that the converters of this repository read, the Electra One preset, places bits of a value in a byte by numbers (`parameterBitPosition`, `byteBitPosition`, `bitWidth`), as recorded in `osd-formats/docs/electra-one.md`; this draft names the two bounds of the field instead and always puts the field at bit 0 of the byte.

## Tensions with the intentions of OSD

- **Universal, facilitator, semantic.** Any device that cuts a value into bytes is described the same way; the caller handles one value in its unit.
- **No vendor field.** `bits` names no maker and no product.
- **No loss.** The two encodings removed from three places have an exact writing in fields. The warning of unsent bits shows a cut that drops part of a value.
- **One way to say one thing.** The encodings that a field repeats leave the places where fields are read, and a field that repeats the element alone is refused. Two things remain: the open point on the frames of `params`, and `u14_msb` and `u14_lsb`, which stay at the places where `bits` is not read, so the format as a whole keeps two writings of these two fields, one for each kind of place.
- **Structure in JSON.** `bits` is an array of two integers; no string is parsed.
- **Against the elements and expressions of 0.7.** One key more on `arg`; the other elements (`device_id`, `identity`, `checksum`, `any`, `capture`) and the expressions are unchanged.

## Objections

None yet.
