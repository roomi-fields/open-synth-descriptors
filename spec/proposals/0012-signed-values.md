# 0012 — Signed values: the types s7 and s14

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: two types more, `s7` and `s14`, for an argument of a command and for `params.value_type`, and the bytes a frame writes for them.

## Why

What this brings to the sharing of sounds on devices: a negative value (a transposition, a detune, a pan) is given in its own unit, and the description says how the device reads its sign, so any tool sends it.

The need. A device expects such a value in two's complement, sometimes with a byte kept for the sign, or shifted so that it stays positive.

What a script does today. It codes the sign into the bytes. In 0.7 the script is the Lua of `commands[].script` (a script that changes the message before it is sent) or of `custom_commands[].script` (a script that builds the message).

What is missing for sharing. The only signed type a frame writes in 0.7 is `s32`, whose bytes section 8.1 does not define. The short arguments (`u7`, `u14`) are unsigned, and an expression gives only values from 0 to 127. So a short signed value is coded in Lua, which a reader without Lua does not run.

## The change

### The types

| Type | Values | Bytes |
|---|---|---|
| `s7` | −64 to 63 | one byte: the two's complement of the value on 7 bits, `v & 0x7F` |
| `s14` | −8192 to 8191 | the two's complement of the value on 14 bits, `v & 0x3FFF`, in two bytes of 7 bits, each written by one element |

`s7` and `s14` join the two closed lists of types, which the schema checks:

```json
{ "enum": ["u7", "u14", "s7", "s14", "s32", "int", "float", "bool", "string", "enum"] }
```

for `type` of an argument (of a command and of its readings), and

```json
{ "enum": ["u7", "u14", "s7", "s14", "s32"] }
```

for `params.value_type`.

The elements that write them are elements of 0.7:

| Type | Element | Byte |
|---|---|---|
| `s7` | `{"arg": name}` | the 7 bits of the two's complement |
| `s7` | `{"arg": name, "add": n}` | `(v + n) & 0x7F`: the value shifted by `n` |
| `s14` | `{"arg": name, "encoding": "u14_msb"}` | the bits 13 to 7 of the two's complement |
| `s14` | `{"arg": name, "encoding": "u14_lsb"}` | the bits 6 to 0 |

The two encodings say how 14 bits are cut; the type says the sign. Section 8.1 of 0.7 defines their bytes for an unsigned value that is sent, the halves of a 14-bit value; the table above takes the same halves of the two's complement.

A reader MUST refuse, as `BadValue` at the faulty place:

- a `range` of an `s7` or `s14` argument outside the values of its type;
- an `s14` argument written by an element without `encoding` (it does not say which byte);
- an `encoding` on an element of an `s7` argument;
- an `encoding` other than `u14_msb` and `u14_lsb` on an element of an `s14` argument.

### With `s32`

`s32` stays as in 0.7, as a type and as an encoding. Its bytes, which section 8.1 of 0.7 does not define, are an open point of this draft for 0.8. A value has one type, so an `s7` or `s14` value is never written by the encoding `s32`, which is refused on it by the last two rules.

### With `params`: `value_type`, `value_bits`, `range`

- `params.value_type` MAY be `s7` or `s14`: the element `value` of `set_frame` then follows the table above. A file with `midi_of` takes `value_type` from its target, as in 0.7.
- A `value_bits` given on a way of an entry of such a block MUST be the width of the type, 7 or 14; any other is `BadValue` at `value_bits`.
- The `range` of a way, and the `scale`, `default` and `values` of the entry, are written in the signed value itself. A `range` of a way outside the values of the type is `BadValue` at the `range`.

The addition of the two words to `value_type` goes beyond the arguments of a command: without it a signed parameter of the shared frame would have only `s32`.

### With `cc_params` and `polarity`

The controls of `cc_params` do not change. A CC, an NRPN or an RPN carries an unsigned value (section 22.1 of 0.7: a negative bound is not written), and `polarity` says where the values of such a control start, `bipolar` for a rest position in the middle, not how a byte codes a sign. An entry of `params` has no `polarity`, and a signed type asks for none.

### With the expressions

A `{"arg": name}` of a signed argument is an operand that keeps its sign. The bounds of section 8.2 do not change: the result of `add`, `sub` and `mul`, and the value bound by `pre`, are bounded from 0 to 127, so a negative value bound as it is gives 0. A byte kept for the sign is therefore written with two bindings, for a magnitude of 127 or less:

```json
"pre": [
  { "bind": "sign", "expr": { "lt": [{ "arg": "pan" }, 0] } },
  { "bind": "size", "expr": { "if": [{ "lt": [{ "arg": "pan" }, 0] }, { "sub": [0, { "arg": "pan" }] }, { "arg": "pan" }] } }
]
```

A literal stays from 0 to 127.

### Where the form is read

The places are the eight of section 8.1 of 0.7, `programs.after[]` (of the form of `before`), the `match_frame` of a sound transfer and the messages of the steps of a file write.

| Place | Sent or recognised | A signed value |
|---|---|---|
| `commands[].frame` | sent | read, on an `s7` or `s14` argument of the command |
| `params.get_frame` | sent | none: it carries the numbers of the entry, which are unsigned |
| `params.set_frame` | sent | read, on `value` under `value_type` `s7` or `s14` |
| `identity.request` | sent | none: no argument is bound there |
| `programs.before[]`, `programs.after[]` | sent | none: no argument is bound there |
| `programs.dump.request` | sent | none: no argument is bound there |
| `resources[].via.file_write.steps[].send` | sent | none: no argument is bound there |
| `identity.reply` | recognised | not covered |
| `replies[].match_frame` | recognised | not covered |
| `sound_transfers[].match_frame` | recognised | not covered |
| `resources[].via.file_write.steps[].reply` | recognised | not covered |

**Reading a signed value back is not covered.** A `capture` keeps one byte, with no sign. A device that sends a signed value is read by a script (`replies[].script`). The places of a value in the data of a sound (section 12.1) keep their form: an unsigned integer of whole bytes, or a script.

### With bit fields, if proposal 0011 is accepted too

This part holds only if [proposal 0011](0011-bit-fields-of-a-value-in-a-frame.md) is accepted.

- `bits` on an element of an `s7` or `s14` argument, or of `value` under one of these types, takes its field from the two's complement on the type's width. A `high` of 7 or more on `s7`, of 14 or more on `s14`, is `BadValue` at `bits`.
- Two refusals of 0011 do not apply to these two types: that of an argument that is no integer of 0 or more (a range below 0 included), and that of a `low` of 0 on a field that holds the highest value of the argument. The next two points take the place of the latter.
- An `s14` value is then written by `bits` at the three places where 0011 reads it, in place of `u14_msb` (`[13, 7]`) and `u14_lsb` (`[6, 0]`): an element with `bits` is not refused for lacking an `encoding`. No element alone writes an `s14`, so no field repeats one: `[6, 0]` writes the low byte whatever the `range`, `[-100, 100]` as `[-500, 500]`.
- An `s7` value is written by the element alone wherever a field would write the same byte for every value: `bits: [6, 0]` is `BadValue` at `bits`, and so is, on an argument whose `range` starts at 0 or above, a field of `low` 0 that holds its upper bound. Any other field of an `s7` is read.
- The warning of 0011 on bits that no byte carries judges, for these two types, every bit of the type's width.
- `add` adds to the field and carries nothing to the next. An `s14` value shifted by a multiple of 128 is the `add` on the field `[13, 7]`; any other shift stays a script.

The detune of the invented device below, written with fields:

```json
{
  "osc": "/detune",
  "args": [{ "name": "detune", "type": "s14", "range": [-500, 500] }],
  "frame": [240, 125, 16, 7,
    { "arg": "detune", "bits": [13, 7] },
    { "arg": "detune", "bits": [6, 0] },
    247]
}
```

### Example

An invented device, "Orbe". Its messages start with `F0 7D 10` and end with `F7`; `7D` is the SysEx id kept for non-commercial use.

The detune goes from −500 to 500 and travels on 14 bits, the high byte first.

```json
{
  "osc": "/detune",
  "args": [{ "name": "detune", "type": "s14", "range": [-500, 500] }],
  "frame": [240, 125, 16, 7,
    { "arg": "detune", "encoding": "u14_msb" },
    { "arg": "detune", "encoding": "u14_lsb" },
    247]
}
```

The value −3 gives `F0 7D 10 07 7F 7D F7`, and the value 3 gives `F0 7D 10 07 00 03 F7`.

The transpose goes from −24 to 24 and travels shifted by 64.

```json
{
  "osc": "/transpose",
  "args": [{ "name": "steps", "type": "s7", "range": [-24, 24] }],
  "frame": [240, 125, 16, 8, { "arg": "steps", "add": 64 }, 247]
}
```

The value −5 gives `F0 7D 10 08 3B F7`.

### What a reader and a converter do

A reader checks the types against the schema and the refusals above, and gives the signed values as written. A tool that sends the message codes them as the tables say. No script and no `lua` capability is needed for these codings.

A converter from 0.7 has nothing to rewrite: no descriptor of 0.7 holds the two types, and their addition changes the meaning of no other. It turns no script into a type. A converter to a format that has no signed value reports what it cannot carry.

### What stays a script

The owner decided that Lua stays, as the declared and opaque extension, for the rarer and more complex cases. For this need these stay a script:

- a sign and a magnitude above 127, since an expression stays from 0 to 127;
- a shift of an `s14` value, since 0.7 gives no element both `encoding` and `add`;
- another code of the sign (one's complement, a sign bit inside a byte that also carries something else);
- a width other than 7, 14 and the `s32` of 0.7;
- reading a signed value from the bytes a device sends.

A script is needed there because the two types state one code, two's complement, with at most a shift on one byte.

## What a reference does

None of the three norms below was read at its source for this draft: each statement is taken over from an earlier exploration of the need, and is to be read again at the source before the proposal is a candidate.

- **MIDI 2.0 Property Exchange.** The controller list (M2-117-UM) bounds values by unsigned integers and describes no setting sent by SysEx. Section 22.1 of 0.7 quotes it, "Values declared in unsigned 32 bit values".
- **MIDNAM.** A SysEx of MIDNAM carries no value of a setting.
- **OSCQuery.** It bounds OSC arguments, which may be negative, without saying their MIDI bytes.

None of the three says how a sign is coded in a message, which the form says.

## Tensions with the intentions of OSD

- **Universal, facilitator, semantic.** The value keeps its sign and its unit for the caller; the coding belongs to the description.
- **No vendor field.** The two types are named after their width, as `u7`, `u14` and `s32` are.
- **No loss.** Nothing of 0.7 changes. A negative value bound by `pre` gives 0, as every result below 0 does; the frame writes a signed argument directly.
- **One way to say one thing.** A value has one type, said at one place: the `type` of an argument, or `value_type`. The encodings `u14_msb` and `u14_lsb` are used again rather than doubled by signed ones.
- **Structure in JSON.** Two words of two closed lists; no string is parsed.
- **Against the elements and expressions of 0.7.** No new element and no new operation.

## Objections

None yet.
