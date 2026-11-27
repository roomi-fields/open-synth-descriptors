# 0014 — Places of a sound: bits of a byte, signed values and 7-bit words

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: a place of a sound transfer (section 12.1 of [0.7](../0.7.md)) gains the keys `byte_bits`, `word7_order` and `sign`, which say that a value lies on some bits of a byte, is cut into 7-bit words, or is signed.

## Why

What this brings to the sharing of sounds on devices: the values of a real sound are read by role from its data by any tool, a first step to making the same sound on another device.

The need. A device may keep a value on some bits of a byte, as a signed number, or cut into bytes that hold 7 bits each.

What is missing for sharing. A place of 0.7 gives the unsigned integer of whole bytes. Every other form is read by the Lua `script` of the place, which a reader without Lua does not run: the setting is then absent, and two sounds cannot be compared on it. Without a script the control is not placed at all, as the third line of section 12.3 of 0.7 shows.

### The devices that need each form

A device is named here only on a public document of its maker or a cited source.

| Form | Device | The public fact | Read at its source for this draft |
|---|---|---|---|
| bits of a byte | Korg minilogue xd | KORG minilogue xd MIDI Implementation, TABLE 2 "PROGRAM PARAMETER": offset 162, bit 0, values 0 and 1, "Step 1 Active Step Off/On" | no: taken from section 12.3 of 0.7, which states no revision for the table (the note of section 10.2 cites revision 1.01, 2020.2.10, of the same document for another part) |
| signed value | no device established | — | — |
| 7-bit words | no device established | — | — |

The signed value and the 7-bit words therefore have examples on an invented device only. A public fact on a real device is to be added for each before the proposal is a candidate, or its absence stays written here.

## The change

### The form

A place of section 12.1 keeps `control`, `from`, `length` and `script`. A place with `from` and `length` MAY add:

| Key | Meaning |
|---|---|
| `byte_bits` | `[high, low]`: the value is the bits `high` to `low` of the one byte of the place, both bounds included, bit 0 being the least significant; the field is moved down so that bit `low` is bit 0 of the value. `length` is 1 |
| `word7_order` | `msb_first` or `lsb_first`: each of the `length` neighbouring bytes of the place is a word of 7 bits, its bits 6 to 0, and the value joins the words. `msb_first`: the first byte holds the highest bits. `lsb_first`: the first byte holds the lowest bits. `length` is 2 to 9 |
| `sign` | `twos_complement`: the value is signed, in two's complement on the width of the place |

Without any of the three keys, a place means what it means in 0.7: the unsigned integer of whole bytes, the first the highest.

**The reading, in this order.**

1. Take the `length` bytes of the data from `from` (the data unpacked, the name in place, as in 0.7).
2. Make the integer of the width of the place:
   - with `byte_bits`: the field of the byte; width `high - low + 1`;
   - with `word7_order`: the sum of each word times 128 to the power of its rank, rank 0 being the lowest word; width `7 × length`. Every bit 7 of these bytes is 0;
   - otherwise: the bytes, the first the highest; width `8 × length`.
3. With `sign`: when the highest bit of that width is 1, subtract 2 to the power of the width. The sign is read on the width of the place and on no other: not on the byte around a field, not on the `range` of a control.

No other operation is done: no offset, no scale, no bound.

**The maximum width** of a value is 64 bits for a place with `sign` or `word7_order`: a signed place of whole bytes is at most 8 bytes, and a place of words at most 9 words (63 bits). An unsigned place of whole bytes stays as in 0.7: no `length` is refused for being large. The device schema of 0.7 says that a reader reads up to 8 bytes: in 0.8, a reader MUST leave out of the settings the control of such a place of more than 8 bytes.

**`sign`.** `twos_complement` is the one code of the list. It has the meaning proposal 0012 gives these words: a value of 7 bits written `v & 0x7F` is read back as `v` by a place of width 7. There is no word for "unsigned": an unsigned place has no `sign`.

### The combinations

| `byte_bits` | `word7_order` | `sign` | Verdict |
|---|---|---|---|
| no | no | no | allowed: the place of 0.7 |
| yes | no | no | allowed |
| no | yes | no | allowed |
| no | no | yes | allowed: signed whole bytes |
| yes | no | yes | allowed on a field of 2 bits or more; refused on a field of 1 bit |
| no | yes | yes | allowed: the sign is the highest bit of the highest word |
| yes | yes | no | refused: a field of one byte is not a word to join |
| yes | yes | yes | refused, for the same reason |

A `script` takes none of `from`, `length`, `byte_bits`, `word7_order` and `sign`.

**One writing for one value.** `byte_bits: [7, 0]` is refused: it is the whole byte, written without the key. `word7_order` on one byte is refused: one word of 7 bits is `byte_bits: [6, 0]`. Two different questions keep two different writings: `byte_bits: [6, 0]` reads 7 bits of a byte and leaves bit 7 aside, whatever it holds; a place without the key reads 8 bits.

**`byte_bits` and `bits`.** The name says that these are the bits of a byte of the data. Proposal 0011 uses `bits` for the bits of a value that a byte of a frame carries; a place gives `bits` no meaning, and a reader MUST refuse it in a place as `UnknownField`.

### The schema

The definition `sound_setting` of the device schema of 0.8 would read (the descriptions are left out here):

```json
{
  "type": "object",
  "required": ["control"],
  "properties": {
    "control": { "type": "string", "pattern": "^/(cc_params|params)/entries/[0-9]+$" },
    "from": { "type": "integer", "minimum": 0 },
    "length": { "type": "integer", "minimum": 1 },
    "byte_bits": {
      "type": "array",
      "items": { "type": "integer", "minimum": 0, "maximum": 7 },
      "minItems": 2,
      "maxItems": 2,
      "not": { "const": [7, 0] }
    },
    "word7_order": { "enum": ["msb_first", "lsb_first"] },
    "sign": { "enum": ["twos_complement"] },
    "script": { "type": "string" }
  },
  "patternProperties": { "^_": {} },
  "additionalProperties": false,
  "dependentRequired": { "from": ["length"], "length": ["from"] },
  "anyOf": [{ "required": ["from"] }, { "required": ["script"] }],
  "dependentSchemas": {
    "script": {
      "not": {
        "anyOf": [
          { "required": ["from"] },
          { "required": ["byte_bits"] },
          { "required": ["word7_order"] },
          { "required": ["sign"] }
        ]
      }
    },
    "byte_bits": {
      "properties": { "word7_order": false },
      "if": { "not": { "required": ["word7_order"] } },
      "then": { "properties": { "length": { "const": 1 } } }
    },
    "word7_order": {
      "if": { "not": { "required": ["byte_bits"] } },
      "then": { "properties": { "length": { "minimum": 2, "maximum": 9 } } }
    },
    "sign": {
      "if": { "not": { "anyOf": [{ "required": ["byte_bits"] }, { "required": ["word7_order"] }] } },
      "then": { "properties": { "length": { "maximum": 8 } } }
    }
  }
}
```

The schema does not compare the two numbers of `byte_bits`: the reader checks their order, and the width of a signed field.

### The faults of the descriptor

A reader MUST refuse each of these when it reads the descriptor. The address is given from the place, `/sound_transfers/T/places/P`.

| Fault | Kind | Address | Found by |
|---|---|---|---|
| `byte_bits` that is not a list of two | `BadValue` | `…/byte_bits` | schema |
| a bound of `byte_bits` that is not an integer from 0 to 7 | `BadValue` | `…/byte_bits/0` or `/1` | schema |
| `byte_bits: [7, 0]` | `BadValue` | `…/byte_bits` | schema |
| `high` below `low` | `BadValue` | `…/byte_bits` | reader |
| `byte_bits` with a `length` other than 1 | `BadValue` | `…/length` | schema |
| `word7_order` of another word | `BadValue` | `…/word7_order` | schema |
| `word7_order` beside `byte_bits` | `BadValue` | `…/word7_order` | schema |
| `word7_order` with a `length` of 1, or above 9 | `BadValue` | `…/length` | schema |
| `sign` of another word | `BadValue` | `…/sign` | schema |
| `sign` on a field of 1 bit | `BadValue` | `…/sign` | reader |
| `sign` on more than 8 whole bytes | `BadValue` | `…/length` | schema |
| a `script` with `byte_bits`, `word7_order` or `sign` | `BadValue` | the place | schema |
| one of the three keys on a place with neither `from`, `length` nor `script` | `BadValue` | the place | schema |

The faults of 0.7 stay with their addresses: a `control` that names no entry, a place that goes past the data, a place and a script together, `from` without `length` or the reverse, a script without the `lua` capability and the reverse. A place goes past the data by its bytes, `from` plus `length`, whatever the form. In a descriptor of 0.7 or earlier the three keys are fields the version does not define: `UnknownField` at the key.

### When the data of a sound contradicts the form

These are no fault of the descriptor, which stays read. They are met on the data of one sound:

- the data does not hold the bytes of the place (a data given by a script may be shorter than the place asks);
- a byte of a place with `word7_order` has its bit 7 set: it is no word of 7 bits.

A reader MUST leave the control out of the settings of that sound, and MUST NOT mask the bit, shorten the place or give a part of the value. The other settings of the sound are given. In a comparison (section 12.2) this setting is `absent`, never `equal`; when both sounds lack it, it is not listed, as in 0.7.

A field of `byte_bits` cannot be contradicted by its byte: the bits outside the field are not part of the value, and the reader does not look at them.

### The value of a setting

The text of section 12.2 would give, for each setting, "the control, its role and its raw value", the raw value being the integer the place reads, after the bits, the words and the sign that the place declares, and after nothing else.

- **No conversion to the scale of the control.** A value stored with an offset is not a signed value, and it is read as it is stored: the OCTAVE of the Korg minilogue xd, which the table of section 12.3 of 0.7 writes `0~4 = -2~+2`, gives 0 to 4.
- **No refusal and no bounding.** A value outside the `range` or the `scale` of its control is given as it is read, a negative value on a scale 0 to 127 included.
- **The place alone says the reading.** Neither `value_type` of `params`, nor the type of an argument, nor a `range` changes how the data is read.
- **The type of the value.** It is an integer that may be negative. A value is from −2⁶³ to 2⁶³ − 1 for a signed place and from 0 to 2⁶⁴ − 1 for an unsigned one: a reader gives it in a form that holds both without rounding. A caller of 0.7, to which a value is unsigned, has this to change.
- **Comparison.** Two settings are `equal` when the two integers read are equal, and `differ` otherwise.

### Examples

The data bytes are written in hexadecimal. "Orbe" is an invented device.

| Device | Place (`control` left out) | Data bytes of the place | Value read |
|---|---|---|---|
| Korg minilogue xd, Step 1 Active | `"from": 162, "length": 1, "byte_bits": [0, 0]` | `01` | 1 |
| Korg minilogue xd, Step 1 Active | the same | `02` | 0 |
| Orbe, a field that does not start at bit 0 | `"from": 3, "length": 1, "byte_bits": [6, 4]` | `6D` (0110 1101) | 6 (110) |
| Orbe, signed byte | `"from": 4, "length": 1, "sign": "twos_complement"` | `FD` | 253 − 256 = −3 |
| Orbe, signed field of 7 bits | `"from": 5, "length": 1, "byte_bits": [6, 0], "sign": "twos_complement"` | `7D`, or `FD` | 125 − 128 = −3 |
| Orbe, signed field above bit 0 | `"from": 6, "length": 1, "byte_bits": [7, 4], "sign": "twos_complement"` | `B5` | 11 − 16 = −5 |
| Orbe, signed whole bytes | `"from": 7, "length": 2, "sign": "twos_complement"` | `FF 9C` | 65436 − 65536 = −100 |
| Orbe, words, high first | `"from": 9, "length": 2, "word7_order": "msb_first"` | `15 3C` | 21 × 128 + 60 = 2748 |
| Orbe, words, low first | `"from": 11, "length": 2, "word7_order": "lsb_first"` | `3C 15` | 60 + 21 × 128 = 2748 |
| Orbe, signed words, high first | `"from": 13, "length": 2, "word7_order": "msb_first", "sign": "twos_complement"` | `7F 7D` | 16381 − 16384 = −3 |
| Orbe, signed words, low first | `"from": 15, "length": 2, "word7_order": "lsb_first", "sign": "twos_complement"` | `7D 7F` | −3 |
| Orbe, three words | `"from": 17, "length": 3, "word7_order": "msb_first"` | `01 00 00` | 16384 |
| Orbe, data that contradicts the form | `"from": 9, "length": 2, "word7_order": "msb_first"` | `95 3C` | no setting |

The line of section 12.3 of 0.7 that has no place, offset 162, bit 0, is then written:

```json
{ "control": "/params/entries/2", "from": 162, "length": 1, "byte_bits": [0, 0] }
```

(`/params/entries/2` being the control Step 1 Active of the descriptor.) As the rest of that section, this is according to the document, not read on a device.

### From 0.7 to 0.8

- Every place of 0.7, whole bytes or script, is a place of 0.8 with the same meaning. A converter writes it unchanged, in its order, with its `_` notes.
- A converter adds none of the three keys, places no control that had no place, and turns no script into a form: a script is opaque.
- The places are written in the device descriptor. A sound descriptor of 0.7 holds no place: nothing in it changes for this proposal.
- The settings a reader of 0.8 gives for a sound under a converted descriptor are those of 0.7: the same controls, the same integers. Nothing is lost and nothing is added.
- The three keys cannot already be in a text of 0.7, which refuses them. The change of `$schema` is that of every conversion.

### Beside proposals 0011 and 0012

This proposal holds whether they are accepted or refused: it defines its own words and takes nothing from their keys or types.

If proposal 0011 is accepted, this proposal replaces its sentence "The places of a value in the data of a sound (section 12.1) keep their own form, whole bytes or a script."

If proposal 0012 is accepted, this proposal replaces its sentence "The places of a value in the data of a sound (section 12.1) keep their form: an unsigned integer of whole bytes, or a script."

Both proposals keep what they say of messages: a `capture` keeps one byte, and a value read from a reply is not covered here.

### What stays a script

For a place, these stay a Lua script:

- **a value made of pieces of bits of bytes apart**, or of some bits of one byte joined to another byte: `word7_order` joins whole words of neighbouring bytes and nothing else. No device with a public fact has been brought forward for a form of pieces; one that is would be written here as an allowed combination;
- a field across two bytes of 8 bits;
- whole bytes with the lowest first;
- another code of the sign (one's complement, a sign beside a magnitude);
- a conversion of the stored value: an offset, a factor, a table, decimal digits;
- a position that depends on the data.

A script is needed there because each asks either for a computation or for a form no cited device has been shown to need, and the format adds a form on a need.

## What a reference does

No norm was read at its source for this draft. Each statement below is to be read again at the source before the proposal is a candidate.

- **MIDI 2.0 Property Exchange.** The controller list (M2-117-UM) bounds values by unsigned integers and describes no place of a value in the data of a stored sound. Taken over from section 22.1 of 0.7 and from proposals 0011 and 0012.
- **MIDNAM.** It names patches, controls and values; as recalled, it places no value in the data of a SysEx dump.
- **OSCQuery.** It describes OSC addresses and arguments, no byte of stored data.
- **Kaitai Struct**, a declarative description of binary formats, as recalled: it declares integers of a number of bits and signed integers in the structure, with no script for either. It is the model followed here: the form of a value is declared in keys that the schema checks.

As far as recalled, none of the three norms for describing devices answers the need, so the form extends the places of 0.7.

## Objections

None yet.
