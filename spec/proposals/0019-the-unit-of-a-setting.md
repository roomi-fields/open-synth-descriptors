# 0019 — The unit of a setting

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: an optional field, `unit`, on a control, on a way of a control and on an argument of a command, taken from a closed list with extensions.

## Why

What this brings to the sharing of sounds on devices: an agent or a person reads « 800 Hz » rather than a position, so a setting means the same thing to whoever shares it.

0.7 says the numbers of a setting (its scale, its default, its named values, the range of an argument) and says nothing of what one of them counts. A title or a note may say it in words, which no program reads. The import of an OSCQuery namespace meets the attribute `UNIT` and loses it, and the import of a MIDNAM document does the same with `Values@Units`.

## The change

### The field

`unit` is one word. It says that a written number `n` means `n` of that unit: no factor and no offset lie between the number and the unit. The unit qualifies the value. It says nothing of the bytes of a message.

| Word | What one is |
|---|---|
| `Hz` | one hertz, a frequency |
| `s` | one second, a duration |
| `dB` | one decibel, a ratio of levels |
| `%` | one hundredth of a whole |
| `semitones` | one semitone, an interval of pitch |
| `cents` | one cent, one hundredth of a semitone |

The words are matched as written, case included. A value outside the list is an extension `x-<registrant>-<term>`, in the form of section 6.2 of [0.7](../0.7.md).

The schema:

```json
{
  "anyOf": [
    { "enum": ["Hz", "s", "dB", "%", "semitones", "cents"] },
    { "type": "string", "pattern": "^x-[a-z0-9]+-[a-z0-9-]+$" }
  ]
}
```

It is referenced, as the optional property `unit`, from an entry of `cc_params`, an entry of `params`, a way of either, and the argument of a command.

### The unit of a control

The `unit` of an entry of `cc_params` or of `params` qualifies the numbers written on the reference scale of the control (section 5.3.3 of 0.7): its `scale`, its `default`, the numbers of its `values` and the `range` of each of its readings. On an entry with no way and no `scale`, the `range` of the entry is the reference scale, and the unit qualifies it. On an entry with no way that has a `scale` too, the unit qualifies the `range` only when it equals the `scale`.

The numbers of a way follow from the reference scale by the affine scaling of section 5.3.3. The unit of the control does not qualify the numbers of a way whose range differs from the reference scale.

The unit does not qualify what `transform` produces. It does not qualify `step_count`, which is a count of steps, nor the numbers of a message (`ctrlIndex`, `value_bits`, `nrpn_lsb_range`, `slice`).

A reading of a control has no `unit` of its own: its `range` is in the unit of the control.

### The unit of a way

The `unit` of a way qualifies the `range` written on that way, and nothing else. It is admitted only when all of these hold:

- the control has a `unit`;
- the two units count the same quantity and are two different words of the list, which today means `semitones` and `cents`;
- the way writes its `range`;
- the reference scale of the control is known, its lowest value below its highest;
- the two ranges agree by the affine scaling of section 5.3.3: each bound of the range in `cents` is 100 times the matching bound of the range in `semitones`.

A way with no `unit` says nothing more than in 0.7.

### The unit of an argument

The `unit` of an argument of a command (`commands[].args[]`, and the `args` of a reading of a command) has the same words and the same meaning: the value of the argument is a number of that unit. It qualifies the value that the caller gives, the `range`, the `default` and the integers of `values`. It does not qualify what an element of a frame or a binding of `pre` makes of the value. It is admitted on an argument of type `u7`, `u14`, `s32`, `int` or `float`.

A reading of a command replaces the arguments of the command, so each of its arguments carries its own `unit` or none.

### What takes a unit, and what does not

An invented device, "Lume". The members of its descriptor that the unit touches:

```json
{
  "cc_params": {
    "entries": [
      { "osc": "/transpose", "unit": "semitones", "scale": [-24, 24], "default": 0,
        "ways": [{ "ctrlType": "cc", "ctrlIndex": [20], "range": [40, 88] }] },
      { "osc": "/gain", "unit": "dB", "scale": [-6, 6], "default": 0,
        "ways": [{ "ctrlType": "cc", "ctrlIndex": [21], "range": [0, 120] }] },
      { "osc": "/interval", "unit": "cents", "default": 700,
        "ways": [
          { "ctrlType": "nrpn", "ctrlIndex": [1, 0], "value_bits": 14, "range": [0, 1200] },
          { "ctrlType": "cc", "ctrlIndex": [22], "range": [0, 12], "unit": "semitones" }
        ] },
      { "osc": "/attack",
        "ways": [{ "ctrlType": "nrpn", "ctrlIndex": [1, 1], "value_bits": 14, "range": [0, 5000] }] },
      { "osc": "/cutoff",
        "ways": [{ "ctrlType": "cc", "ctrlIndex": [23], "range": [0, 127] }] },
      { "osc": "/length", "unit": "x-lume-bars", "scale": [1, 8],
        "ways": [{ "ctrlType": "cc", "ctrlIndex": [24], "range": [1, 8] }] }
    ]
  },
  "commands": [
    { "osc": "/vibrato",
      "args": [{ "name": "rate", "type": "u7", "range": [1, 20], "unit": "Hz" }],
      "frame": [240, 125, 17, 1, { "arg": "rate" }, 247] }
  ]
}
```

- `/transpose` travels centred on 64. It takes `semitones` through a `scale` in semitones, tied to the range of its way by the affine scaling: −24 gives 40, 0 gives 64, 24 gives 88.
- `/gain` is set by the device in tenths of a decibel, 0 to 120. It takes `dB` only through a `scale` in whole decibels, which does not say the tenths: a `default` or a named value of −1.5 dB cannot be written.
- `/interval` is counted in cents on one message and in semitones on another. The second way says its unit, and 700 gives 7 on it.
- `/attack` is counted in milliseconds. It does not take `s`: 5000 would mean 5000 seconds.
- `/cutoff` is a position on a curve that ends in a frequency. It takes no unit: 64 is not 64 Hz.
- `/length` takes an extension.
- The argument `rate` of `/vibrato` is a number of hertz, and the byte of the frame is that number.

How many settings of real devices gain a unit with this field is not measured.

### The curve is left to later

This draft says no curve and no factor. A curve ties a position to a value in a unit, and it differs from one device to the next. It is left to a later proposal, device by device, when a source states it. Until then a position on a curve carries no unit.

### What a reader refuses

| Fault | At | Kind |
|---|---|---|
| a `unit` that is neither a word of the list nor an extension (`"hz"`, `"ms"`, a value that is no string) | the `unit`: `/cc_params/entries/N/unit`, `/params/entries/N/unit`, `/cc_params/entries/N/ways/M/unit`, `/params/entries/N/ways/M/unit`, `/commands/N/args/K/unit`, `/commands/N/readings/R/args/K/unit` | `BadValue` |
| a `unit` on a way of a control that has none | the `unit` of the way | `BadValue` |
| a `unit` on a way that repeats the unit of the control | the `unit` of the way | `BadValue` |
| a `unit` on a way that counts another quantity than the unit of the control, an extension on either side included | the `unit` of the way | `BadValue` |
| a `unit` on a way that writes no `range`, or of a control whose reference scale is not known or holds one value | the `unit` of the way | `BadValue` |
| two ranges that do not agree by the affine scaling | the `unit` of the way | `BadValue` |
| a `unit` on an argument of type `bool`, `string` or `enum` | the `unit` of the argument | `BadValue` |
| a `unit` in a reading of a control | `/cc_params/entries/N/readings/R/unit`, `/params/entries/N/readings/R/unit` | `UnknownField` |
| a `unit` anywhere in a descriptor of 0.4 to 0.7 | the `unit` | `UnknownField` |

The schema checks the form of the word. The other rules compare two members, and the reader checks them.

### What a reader does with a unit

A reader gives each `unit` as written and writes it back. It converts no number from one unit to another.

A reader accepts an extension it does not know and treats it as the default of the field, which is no unit: a tool shows the number with no unit that it claims to understand, and computes nothing from the word.

### The converters

The converter from 0.7 writes no `unit`. No unit is guessed from an address, a title, a description, a note or a name of a value. The converter loses nothing and adds nothing for this field.

A converter to a format that has no unit writes the numbers unchanged and reports each `unit` as lost, at its pointer. It applies no factor and writes the word in no other text.

### OSCQuery

`UNIT` is an array with one string for each value of the method, in the order of its arguments. `null` holds the place of a value with no unit. When all the values share one unit, one element alone may stand for them.

| `unit` | The export writes | The import reads |
|---|---|---|
| `Hz` | `time.hz` | `time.hz` |
| `s` | `time.second` | `time.second` |
| `dB` | `gain.db-raw` | `gain.db-raw`; `gain.db`, whose clipping to a minimum is named in the losses |
| `%` | `%` | `%` |
| `semitones` | `semitones` | `semitones` |
| `cents` | `cents` | `cents` |
| an extension | the same string | a string of the form of an extension |

The proposed names of OSCQuery, as read, hold none for a percentage and none for an interval in semitones. They hold `time.cents`, but the excerpt read gives the name with no definition, and the name stands in the family `time`: nothing read says that it counts an interval. The draft does not match it with `cents`. The export writes the word of OSD for `%`, `semitones` and `cents`, outside the proposed names, and the import reads `time.cents` as a string outside the third column.

The import puts the unit on the argument of the command it makes, at the same position. A string outside the third column (`time.ms`, `time.cents`, `time.midinote`, `gain.linear`, `distance.m`) gives no unit and is named in the losses at `…/UNIT/K`: no factor is applied and no near unit is chosen. `null` gives no unit and no loss. A `UNIT` that is not an array, whose length is neither 1 nor the number of arguments, or an element that is neither a string nor `null`, is named in the losses and gives no unit. So is a unit that falls on an argument of text or of boolean type. A `UNIT` on a node that gives no command is lost with it.

The export writes `UNIT` for a method only when at least one of its arguments has a unit for the `RANGE` written, with `null` for the others.

- For a command, each argument gives its own `unit`.
- For a control, the `RANGE` is that of the way the export carries. When that way has a `unit`, the export writes it. Otherwise it writes the unit of the control only when the numbers of the written `RANGE` are those of the reference scale, both bounds equal. For an entry with no way, the same test applies to its `range`.
- In every other case, no `RANGE` written included, no unit is written for that method, and the `unit` is named in the losses at its pointer. On "Lume", `/transpose` and `/gain` lose their unit; `/interval` writes `cents`.

### MIDNAM

`Values@Units` is an `NMTOKEN`, which the DTD ties to no list. The import of 0.7 loses it (`osd-formats/src/midnam.rs`).

With the field, the import writes the `unit` of the control when `Units` is exactly `Hz`, `s`, `dB`, `semitones`, `cents` or a string of the form of an extension, and when `Min` and `Max` are both read: the range they give is the reference scale of the imported control. Any other token, or a `Units` beside a bound that is not read, is lost and named, as today.

The export writes `Units` under the condition said for the `RANGE` of OSCQuery, applied to `Min` and `Max`. `%` is no `NMTOKEN`, so `Units` cannot hold it: the export writes none and names the loss. `Values@Mapping` stays lost.

## What a reference does

| Reference | What it says of the unit | Read for this draft |
|---|---|---|
| OSCQuery (github.com/Vidvox/OSCQueryProposal, `README.md` and `units.md`) | An optional attribute `UNIT`, an array with one string per value, `null` as a placeholder, one value allowed for all. The string "should describe the units of the value, from a list of commonly-accepted values"; `units.md` gives "proposed examples", an open list of names `category.name`. | Excerpts read at the source on 2026-11-27 by a maintainer; the author of this draft worked from those excerpts. |
| Cmajor (cmajor.dev/docs/PatchFormat) | `unit`, "an optional string which the host will display as the units for this value. E.g. “%” or “dB”". The excerpt read states a use for display and does not say that the value is in the unit. | Excerpt read at the source on 2026-11-27 by a maintainer. |
| SoundFont 2.04 (sfspec24.pdf) | The unit is fixed for each generator by the specification. It defines logarithmic units (cent, centibel, a tenth of a decibel, timecent); `initialFilterFc` is in absolute cents, its frequency in Hz defined by the parameter. | Excerpts read at the source on 2026-11-27 by a maintainer. |
| MIDNAM (`MIDINameDocument10.dtd`) | `Values` has `Units`, an `NMTOKEN`, optional, beside `Mapping`. The declaration gives no list. | The DTD kept in `osd-formats/tests/data/`, read for this draft. |
| MIDI 2.0 Property Exchange, controller resources (M2-117-UM) | No field for a unit among the properties of a controller. | Not read again: taken from the table of section 22.1 of 0.7. To be read at the source before the proposal is a candidate. |

Where the draft departs:

- **From MIDI 2.0 Property Exchange**, which has no unit field. Reason: a description is read with no device in hand, by an agent that has to say « 800 Hz »; a unit left to a title or a description is text that no program reads, and structure belongs in a field that the schema checks. The field is optional, so nothing of 0.7 changes. An export to Property Exchange reports it as lost.
- **From OSCQuery**, whose list is open and whose names are strings `category.name`. Reason: one word for one unit, in a closed list that the schema checks, with the extension form of 0.7 for the rest. The table above says which proposed names of OSCQuery carry a word of the list; the import names every other as lost.
- **From Cmajor**, whose unit is a free string to display. Reason: here the number is in the unit, and a program may rely on it.
- **From SoundFont 2.04**, whose units are fixed per setting and may carry a factor. Reason: the descriptions are of any device, so each setting says its unit, and the factors are left to later with the curves.

## Tensions with the intentions of OSD

- **Universal, facilitator, semantic.** The number says what it counts, for any device and any reader.
- **One meaning per field.** `unit` says what one of a number is; the curve and the factor are not in it.
- **No vendor field.** The six words are common units; an extension names its registrant.
- **No loss.** Nothing of 0.7 changes. A format that cannot hold a unit reports it.
- **One way to say one thing.** The unit of a control is said once. A way repeats nothing: it speaks only when it counts the same quantity in another unit.

## Open point

The unit of a way serves one case, `semitones` against `cents`, of which no current use is known. It is to be confirmed or removed before the proposal is a candidate.

## Objections

None yet.
