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
