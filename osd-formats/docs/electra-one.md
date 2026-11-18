# The Electra One preset

The preset format of the Electra One as its maker documents it: the page "Preset format description" (docs.electra.one/developers/presetformat.html, documentation 5.0.0), cited here by section and not copied (the page declares no licence). The maker publishes no machine-readable schema, so the checks of `osd-formats` are written from that page, each citing it.

The importer reads versions 2 and 3 (page, "How the controller reads a document": the document is rejected unless the version is 2 or 3); the exporter writes 3. A control with a 7-bit CC, a 14-bit CC, an NRPN or an RPN becomes a control of `cc_params.entries`: its name is the remark `_name`, its group's name is its `section`, the min and max of its value are its `range`. A SysEx message becomes a command with its bytes. The address `osc` is built from section and name. A message of a kind marked lost is named in the losses at `/controls/N/values/M/message`, and its line says it is lost in the round trip. What is only the look of the preset (pages, positions, colours, shapes, the ports and channels of its devices) is in the losses on import, and nothing is invented for it.

## Kinds of message

The `type` of a message (page, section "Message", attribute `type`). A message with no type is a SysEx message; a type the maker does not document is refused on import, at `/controls/N/values/M/message/type`.

| Kind | Fate | Reason | Piece |
|---|---|---|---|
| `cc7` | control | `cc`; the parameter number is 0 to 127 | presetformat, Message, `type` and `parameterNumber` |
| `cc14` | control | `cc` and `cc_lsb` 32 above; the parameter number is 0 to 31 | presetformat, Message, `type` and `bitWidth` (14 for cc14) |
| `nrpn` | control | `nrpn_msb` and `nrpn_lsb`, the 14-bit number split in two | presetformat, Message, `parameterNumber` (14-bit) |
| `rpn` | control | `rpn_msb` and `rpn_lsb`, the 14-bit number split in two | presetformat, Message, `parameterNumber` (14-bit) |
| `sysex` | command | `frame`, the fixed bytes of the template framed by F0 and F7; a template with a byte built at send time (value, parameter, checksum, function, any) or named by id is lost | presetformat, Message, `data` and "SysEx template" |
| `note` | lost | a note is played, the descriptor has no field for sending one from a control | presetformat, Message, `type` |
| `program` | lost | a program change is a recall, which `programs` describes, not a control | presetformat, Message, `type` |
| `start` | lost | a realtime message is no control of the descriptor | presetformat, Message, `type` |
| `stop` | lost | a realtime message is no control of the descriptor | presetformat, Message, `type` |
| `tune` | lost | a realtime message is no control of the descriptor | presetformat, Message, `type` |
| `atpoly` | lost | the descriptor has no control bound to this channel message | presetformat, Message, `type` |
| `atchannel` | lost | the descriptor has no control bound to this channel message | presetformat, Message, `type` |
| `pitchbend` | lost | the descriptor has no control bound to this channel message | presetformat, Message, `type` |
| `spp` | lost | a song position is no control of the descriptor | presetformat, Message, `type` |
| `relcc` | lost | a relative CC steps a value, the descriptor has no field for steps | presetformat, Message, `type` and `relative` |
| `macro` | lost | a macro holds a parameter of the performance, which the descriptor has no field for | presetformat, Message, `type` |
| `virtual` | lost | the value sends nothing: no MIDI binding to describe | presetformat, Message, "virtual and none" |
| `none` | lost | the value sends nothing: no MIDI binding to describe | presetformat, Message, "virtual and none" |

## Numbers the export lays out by

The export fills the slots in order: the 12 pots of a control set, then the next of the 3 control sets, then the next page. The controls are laid out by section, in the order the descriptor first names each, and in the descriptor's order within a section. A control past the 432nd is named in the losses by its JSON pointer.

| Limit | Number | Piece |
|---|---|---|
| `pages` | 12 | presetformat, "Model differences": highest page id (mk2; the mini stops at 16) |
| `control sets` | 3 | presetformat, "Model differences": control sets |
| `pots` | 12 | presetformat, "Model differences": highest potId |
| `controls` | 432 | presetformat, "Size limits": controls |
| `devices` | 32 | presetformat, "Size limits": devices read |

## What the export writes for a control

The page declares no free text a person reads for a description or an author (its top-level fields are `version`, `name`, `projectId`, `pages`, `devices`, `overlays`, `groups`, `controls`; the maker's editor keeps them apart, tied by `projectId`): the credit stays in the losses.

| Element | Rule | Piece |
|---|---|---|
| `control name` | The remark `_name`, else the setting of the `role`, else the last segment of the address made readable (`/oscillator_1/octave` → `Octave`); cut to 40 characters; two controls of one section never share a name (the second becomes `Octave 2`). A group name is cut to 40, a preset name to 20. | presetformat, Control, `name` (maxLength 40); Group, `name` (40); Preset, `name` (20) |
| `list` | `values` name the positions, or `expects` is `discrete` with 3 positions or more in its `range`: a control of type `list` whose value refers (`overlayId`) to its own overlay, one item per position, the label a value name or the number, cut to 20 characters and kept apart. Named `values` win over `expects`. No limit on items. Import reads the labels back as `values`; an overlay no control refers to is lost. | presetformat, Overlay, `items` (no limit) and item `label` (maxLength 20); Control, `type` list |
| `pad` | `expects` is `switch`, `momentary` or `trigger`, or `discrete` with 2 positions and no named `values`; a command is a pad. | presetformat, Control, `type` pad |
| `fader` | Every other control. | presetformat, Control, `type` fader |
| `group bounds` | A group is a rectangle of whole cells that frames its own controls only; a group spread over two rows begins on a new row, and the next group begins on a new row after it, so no two groups of a page overlap. | presetformat, Group, `bounds` |
| `credit` | Not written: the preset shows no text for it; `/_sources` is in the losses, printed by the command. | presetformat, "Preset", the fields of the document |
| `ways` | The first way of the descriptor with a complete binding is written, with its own range; the export never reorders the ways (the order is the writer's). Each way not written is in the losses at `/cc_params/entries/N/also/M`, naming the way written and the way lost. | presetformat, Message, `type` and `parameterNumber` |

The export writes one device (id 1). Control ids and group ids share one id space (page, "How the controller reads a document"): controls take 1 to N, groups the numbers after. A group is written per section and page, its bounds those of its controls. What the descriptor holds and the preset cannot (credit and licence in `_sources`, orientation, channel, transforms, roles, other blocks) is in the losses by its JSON pointer.
