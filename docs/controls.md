# Controls at the precision of MIDI 2.0

A control entry of 0.6 (CC, 14-bit CC, NRPN, RPN, and the param entry) says what MIDI-CI Property Exchange Controller Resources (M2-117-UM v1.0.1, table 5) says of a control, so a tool lays controls out and an agent moves them without guessing. Each property below has its OSD field, or the reason it is not carried.

| Property | OSD field | Reason |
|---|---|---|
| `title` | not carried | The label of a control belongs to the role (`role`) and to the tool that lays it out; the address names the control. |
| `description` | not carried | Free text for people is a `_` note, never read by a program. |
| `channel` | `channel` | The MIDI channel of the control, 0 to 15. |
| `ctrlType` | not carried | The kind of message is given by which address fields the entry holds (`cc`, `nrpn_msb`, `rpn_msb`, `p`). |
| `ctrlIndex` | not carried | The index is the number in those address fields (`cc`, `nrpn_lsb`…). |
| `priority` | `priority` | Same meaning: 1 essential to 5 hidden. |
| `default` | `default` | The raw value on an initialised sound, in the space of `range`. |
| `transmit` | `transmit` | Same four words: absolute, relative, both, none. |
| `recognize` | `recognize` | Same four words: absolute, relative, both, none. |
| `numSigBits` | not carried | The bits follow from `range` and from `cc_lsb` (14 bits); a separate count could contradict them. |
| `paramPath` | not carried | OSD says it as `role`, a path of parts and a setting, which the reader writes back as a `paramPath`. |
| `typeHint` | `expects` | `expects` keeps its own six words; the correspondence is below. |
| `ctrlMapId` | carried by `values` | The names are inline in the entry (label with its integer), not the id of a map shared between controls; each value lies inside `range`. |
| `stepCount` | `step_count` | The number of steps, 2 or more. |
| `minMax` | `range` | Lowest and highest value. |

## A control reached by several ways

A setting that a device accepts by several messages (a CC and an NRPN, say) is one control, with one address. The flat fields of the entry (`cc`, `cc_lsb`, `nrpn_msb`, `nrpn_lsb`, `rpn_msb`, `rpn_lsb`, `range`) are its first way, exactly as in 0.5; the optional `also`, a list, gives the ways after it, each with one message kind (a CC, a 14-bit CC with its `cc_lsb`, an NRPN or an RPN) and its own `range`. A way that mixes two kinds, or names none, is refused at its pointer.

- **Order.** The ways are in the order of preference the writer gave them: the flat way first, then `also` in order. A reader gives them as written (`CcEntry::ways`) and never reorders them.
- **Sender.** A sender picks the first way it can send: a descriptor that writes a 14-bit CC or an NRPN before a 7-bit CC serves a sender that does both.
- **Receiver.** A receiver recognises every way as the same control.
- **One claim.** Two messages are the same when they share kind, channel and numbers; a 14-bit CC claims its CC number and its LSB number, so it collides with a simple CC on either. A message is claimed once, across the ways as across the controls, inside one control as between two. The reader refuses the later claim and names both places.
- **Conversion.** `convert` from 0.5 keeps each control as it is: it never joins two controls.

## `expects` and `typeHint`

| `expects` | `typeHint` | Reason |
|---|---|---|
| `continuous` | `continuous` | Same meaning. |
| `switch` | `toggle` | A control that stays on or off. |
| `momentary` | `momentary` | Same meaning. |
| `discrete` | `valueSelect` | A choice among named `values`. |
| `trigger` | `momentary` | The standard has no trigger: the nearest is momentary. The loss is OSD to MIDI 2.0 only; coming back, `momentary` reads as `momentary`. |
| `clock` | no `typeHint` | The standard has no clock: the control carries no hint. The loss is OSD to MIDI 2.0 only. |

The standard's `relative` has no `expects` word: it is carried by `transmit` and `recognize` (`relative` or `both`).
