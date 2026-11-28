# The Electra One preset

The preset format of the Electra One as its maker documents it: the page "Preset format description" (docs.electra.one/developers/presetformat.html, documentation 5.0.0), cited here by section and not copied (the page declares no licence). The maker publishes no machine-readable schema, so the checks of `osd-formats` are written from that page, each citing it.

The importer reads versions 2 and 3 (page, "How the controller reads a document": the document is rejected unless the version is 2 or 3); the exporter writes 3. A control with a 7-bit CC, a 14-bit CC, an NRPN, an RPN or a relative CC becomes a control of `cc_params.entries`: its name is its `title`, its group's name is its `section`. A SysEx message becomes a command with its bytes; when its template holds a value byte, the command has the argument `value`. The address `osc` is built from section and name. The fate of each attribute of a preset and of each property of an OSD 0.7 device is in the two tables at the end of the page. The device is read back with `osd` before it is returned.

## What the import reads and what it names

- **Losses.** Each key of a preset, at any depth, that the importer does not read is in the losses at its pointer (a key escaped as a segment of a JSON pointer), or at the pointer of a block that holds it, documented by the maker or not: what is only the look of the preset, the setup of the controller (its devices, the `deviceId` of each message), the `id` of a control and of a value, the name of a group no control or command written is in. Nothing is invented for it. `version`, `groupId`, `overlayId` and the `id` of a group, an overlay or a device message are followed and not named.
- **Range.** The range of a way, and of the argument of a command, is the MIDI range of the message (page, section "Message"): each bound is `message.min` or `message.max`, else the value's `min` or `max`; on a control of type `pad`, `offValue` and `onValue` when it gives them, a lone one or `offValue` not below `onValue` giving no range and a loss. A range needs both bounds: a lone bound gives none and is lost at its pointer. A range equal to the whole scale of the way (or of the argument), with no bound on the value, gives none. On a pad, `offValue` and `onValue` are always named in the losses, each at its pointer, whether their range is carried (a part of the scale, or the whole scale with a bound on the value) or not: nothing in the descriptor says the control is a pad, so no pad is written back. A bound that differs from the range carried is lost at its pointer. A bound that is not a whole number is refused at its pointer. Each bound of a value whose message is not carried (a command without an argument, a message of a lost kind, a template that is not carried, a message an earlier control claims) is lost at `/controls/N/values/M/min` and `/max`; a value without a message is lost whole at `/controls/N/values/M`.
- **Relative.** A message of kind `cc7`, `cc14`, `nrpn` or `rpn` with `relative: true`, and a `relcc` message, is a way with `transmit` and `recognize` `relative`; `relativeMode`, the encoding of the steps, is lost.
- **Claims.** A message of a kind marked lost is named at `/controls/N/values/M/message`, and its line says it is lost in the round trip. A message is claimed once, as the 0.7 reader judges it (a 14-bit CC claims its two numbers, a relative CC its number, an NRPN and an RPN of one number are two messages; no channel is written): a value whose message a control written before it claims gives no control and is lost at its message, the other controls and the commands staying.
- **Named values.** The `values` of a control are the items of its overlay, in their order, that have a text label and a whole value inside the scale of its way (its `range`, else what its message carries, bounds included), two items of one label included; any other item is lost at `/overlays/K/items/I`, the loss naming the control, and a control of which no value remains has no `values`. An overlay no control written reads is lost at `/overlays/K`.

## Kinds of message

The `type` of a message (page, section "Message", attribute `type`). A message with no type is a SysEx message; a type the maker does not document is refused on import, at `/controls/N/values/M/message/type`.

| Kind | Fate | Reason | Piece |
|---|---|---|---|
| `cc7` | control | `cc`; the parameter number is 0 to 127 | presetformat, Message, `type` and `parameterNumber` |
| `cc14` | control | `cc` and `cc_lsb` 32 above; the parameter number is 0 to 31 | presetformat, Message, `type` and `bitWidth` (14 for cc14) |
| `nrpn` | control | `nrpn_msb` and `nrpn_lsb`, the 14-bit number split in two | presetformat, Message, `parameterNumber` (14-bit) |
| `rpn` | control | `rpn_msb` and `rpn_lsb`, the 14-bit number split in two | presetformat, Message, `parameterNumber` (14-bit) |
| `sysex` | command | `frame`, the bytes of the template framed by F0 and F7, with a checksum or a value byte as OSD writes them (see "The elements of a SysEx template"); the template is written in the message or named by its id; one with another byte built at send time (parameter, function, any) is lost, and one received only (`direction` `in`) is never sent | presetformat, Message, `data` and "SysEx template" |
| `note` | lost | a note is played, the descriptor has no field for sending one from a control | presetformat, Message, `type` |
| `program` | lost | a program change is a recall, which `programs` describes, not a control | presetformat, Message, `type` |
| `start` | lost | a realtime message is no control of the descriptor | presetformat, Message, `type` |
| `stop` | lost | a realtime message is no control of the descriptor | presetformat, Message, `type` |
| `tune` | lost | a realtime message is no control of the descriptor | presetformat, Message, `type` |
| `atpoly` | lost | the descriptor has no control bound to this channel message | presetformat, Message, `type` |
| `atchannel` | lost | the descriptor has no control bound to this channel message | presetformat, Message, `type` |
| `pitchbend` | lost | the descriptor has no control bound to this channel message | presetformat, Message, `type` |
| `spp` | lost | a song position is no control of the descriptor | presetformat, Message, `type` |
| `relcc` | control | a way `cc` with `transmit` and `recognize` `relative` and `value_bits` 7, written back as a `relcc` message; the export writes the steps as `twosComplement`, and the import names the encoding of the steps lost, the way has no field for it | presetformat, Message, `type` and `relative` |
| `macro` | lost | a macro holds a parameter of the performance, which the descriptor has no field for | presetformat, Message, `type` |
| `virtual` | lost | the value sends nothing: no MIDI binding to describe | presetformat, Message, "virtual and none" |
| `none` | lost | the value sends nothing: no MIDI binding to describe | presetformat, Message, "virtual and none" |

## The elements of a SysEx template

A template is the `data` of a SysEx message (page, "SysEx template"). A `data` that is a number is the id of a template in the `messages` of the device the message's `deviceId` names, and gives the command the same template written inline would give; an id that names no template, and a template whose `direction` (the message's, else the template's) is `in`, which is never sent, give no command and a loss at the message. The F0 and F7 bytes are dropped wherever they appear, and the frame is framed by them, so a checksum `start` counts the bytes as the frame holds them (F0 is 0). Each element is a byte, or an object by its `type`:

- `checksum` with an `algorithm` (`roland`, `waldorf`, `fractal`; default `roland`), a `start` (a number or a text of digits; default 0) and a `length` of 1 or more becomes `{checksum, from, length}` with `twos7`, `sum7` or `xor7`. The `length` has the default 0 on the page ("How many bytes to sum"): a checksum without it sums no byte, which OSD cannot write, so it is treated as `length: 0`, no command is written and a loss at the message names the checksum without length and the default 0. An algorithm OSD does not write, a start or a length that is no whole number, is a loss too and no command is written.
- `value` taking the 7 bits of the value (no rules, or `parameterBitPosition` 0, `byteBitPosition` 0, `bitWidth` 7) becomes `{arg: value}`; a template that takes also the bits 7 to 13 gives a 14-bit argument written `u14_lsb` and `u14_msb` in the order of the template. Any other rule (packed bits, a byte position, another parameter) is a loss.
- `parameter`, `function` (only its name is read, never its Lua) and any other type are a loss that names the control.

A loss is at `/controls/N/values/M/message` and the command of that control is not written; the other controls are converted. The caller's source carries `revision` when the caller passes it, and the import writes there the `sha256` of the preset text received and, under `parameter_numbers`, the parameter number of each command by its address. The date of the preset the caller passes (`imported_at`) is kept as the note `_imported_at` of the source added, its value unchanged, 0.7 having no field for it: a `_imported_at` the source already holds stays, and the date goes under `_imported_at_2` (then `_imported_at_3`, then the next number not taken). It is no loss, no date is invented, and the other keys of the source are kept.

## What the former importer kept

The importer SyntHub used before `osd-formats` (its script and its re-import test, readable in SyntHub's history before SH-cgi.16). One row per behaviour: kept with the name of its case, or lost with the place of its loss.

| Behaviour | Fate | Case or loss | Piece |
|---|---|---|---|
| `checksum` | kept | `a_checksum_element_becomes_a_checksum_of_the_frame_that_sends_the_bytes_the_maker_s_rule_gives` | presetformat, SysEx template, `checksum` |
| `checksum without a known algorithm or start` | lost | `/controls/N/values/M/message` | presetformat, SysEx template, `algorithm` and `start` |
| `byte computed from the value` | kept | `a_value_byte_on_seven_bits_becomes_an_argument_and_the_frame_sends_the_value` | presetformat, SysEx template, `value` |
| `byte computed from the parameter number` | lost | `/controls/N/values/M/message` | presetformat, SysEx template, `parameter` |
| `lua function of one return` | lost | `/controls/N/values/M/message` | presetformat, SysEx template, `function`; the Lua is not in the preset, OSD has no form |
| `lua function with a branch` | lost | `/controls/N/values/M/message` | presetformat, SysEx template, `function`; the Lua is not in the preset, OSD has no form |
| `lua function calling an API of the controller` | lost | `/controls/N/values/M/message` | presetformat, SysEx template, `function`; the Lua is not read |
| `lua function not defined` | lost | `/controls/N/values/M/message` | presetformat, SysEx template, `function`; the Lua is not read |
| `parameter number of a command` | kept | `the_parameter_number_of_each_command_is_kept_in_the_source_under_the_address_of_the_command` | presetformat, Message, `parameterNumber` |
| `date of the preset` | kept | `the_date_of_the_preset_the_caller_gives_is_kept_as_the_note_imported_at_of_the_source` | the caller's `Source`; written as the note `_imported_at`, 0.7 having no field for the date of an import (`sources[].imported_at` removed, 0008) |
| `revision and hash of the preset` | kept | `the_revision_and_the_hash_sit_together_in_the_source_beside_its_credit` | the caller's `Source` for the revision; the import for the SHA-256 |
| `silent address` | lost | `none: the policy of a re-import tool (leave a published driver intact when no preset is fetched), not a conversion` | SyntHub's re-import tool |

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
| `control name` | The `title`, else the setting of the `role`, else the last segment of the address made readable (`/oscillator_1/octave` → `Octave`); cut to 40 characters; two controls of one section never share a name (the second becomes `Octave 2`). A group name is cut to 40, a preset name to 20. | presetformat, Control, `name` (maxLength 40); Group, `name` (40); Preset, `name` (20) |
| `list` | `values` name the positions, or `expects` is `discrete` with 3 positions or more in its `range`: a control of type `list` whose value refers (`overlayId`) to its own overlay, one item per position, the label a value name or the number, cut to 20 characters and kept apart. Named `values` win over `expects`. No limit on items. Import reads the labels back as `values`, each item judged on the scale of the control (see above); an overlay no control written reads is lost. | presetformat, Overlay, `items` (no limit) and item `label` (maxLength 20); Control, `type` list |
| `pad` | `expects` is `switch`, `momentary` or `trigger`, or `discrete` with 2 positions and no named `values`; a command is a pad. | presetformat, Control, `type` pad |
| `fader` | Every other control. | presetformat, Control, `type` fader |
| `group bounds` | A group is a rectangle of whole cells that frames its own controls only; a group spread over two rows begins on a new row, and the next group begins on a new row after it, so no two groups of a page overlap. | presetformat, Group, `bounds` |
| `credit` | Not written: the preset shows no text for it; `/sources` is in the losses, printed by the command. | presetformat, "Preset", the fields of the document |
| `ways` | The first way of the descriptor with a complete binding is written, with its own range, relative when its `transmit` is `relative` (`relcc` for a CC of one number, else `relative: true`); the export never reorders the ways (the order is the writer's). Each way not written, of any kind (a pressure, a bend, a note and a `sysex` way too), is in the losses at `/cc_params/entries/N/ways/M`, naming the way written and the way lost. | presetformat, Message, `type` and `parameterNumber` |

The export writes one device (id 1). Control ids and group ids share one id space (page, "How the controller reads a document"): controls take 1 to N, groups the numbers after. A group is written per section and page, its bounds those of its controls.

## What the export writes of a message and of a command

Each message has the `min` and `max` of the scale of its way or argument (its range, else what it carries); on a pad also `offValue` and `onValue`, and `onValue` 127 on the pad of a command without argument. The value has `min` and `max` only when the descriptor gives a range. A command is written as a SysEx template when its frame holds fixed bytes between F0 and F7, the argument `value` (u7, or u14 sent as u14_lsb and u14_msb, with nothing added) and checksums (`twos7` as `roland`, `sum7` as `waldorf`, `xor7` as `fractal`), each with its `start` and `length` written (a checksum without length covers every byte from its start up to itself); a command with an argument is a fader, one without a pad. Any other frame is lost at `/commands/N/frame`, and each field of its argument at its pointer.

## What the export names

The export imports the preset it writes and compares the device with the one it was given, each control, command and way beside the one it became: every field that comes back missing, changed or added is in the losses, at its JSON pointer in the device given or at the pointer of a block that holds it. A control past the 432nd, and a control that comes back in another order because the preset lays controls out by group, are named at their pointer. So is a command whose parameter number a source keeps (`parameter_numbers`): the preset written has none for a SysEx message. The losses come in the order of the descriptor given: list items by their index, a block before what lies under it, the fields of an object in the order `osd` writes them.

## The attributes of a preset

Copied by hand from docs.electra.one/developers/presetformat.html, documentation 5.0.0. `carried`: the import reads it into the device; `lost`: it is in the losses; `conditional`: either, as the reason says; `structure`: the import follows it and does not name it.

| Preset attribute | Fate | Reason | Section |
|---|---|---|---|
| `version` | structure | 2 or 3, else the preset is refused | Top level |
| `name` | carried | `device.name`, which gives `device.osc_prefix`; an empty name gives "Electra One preset" | Top level |
| `projectId` | lost | the identity of the preset in the maker's editor, not of the device | Top level |
| `pages` | lost | the pages are the look of the preset | Top level |
| `devices` | carried | read for the templates its messages hold; each of its other attributes has its row | Top level |
| `overlays` | carried | read for the labels of the controls that refer to them; an overlay no control written reads is lost | Top level |
| `groups` | carried | read for the section of the controls and commands | Top level |
| `controls` | carried | the controls and commands of the device | Top level |
| `devices[].id` | lost | a number of the preset; it is followed to find a template named by id | Device |
| `devices[].name` | lost | the setting of the controller, no field of the device: not the name of the device the preset speaks to, which the preset may rename | Device |
| `devices[].port` | lost | the setting of the controller, no field of the device | Device |
| `devices[].channel` | lost | the channel the controller is set to: the setting of the controller, no field of the device | Device |
| `devices[].interfaces` | lost | the setting of the controller, no field of the device | Device |
| `devices[].runningStatus` | lost | the setting of the controller, no field of the device | Device |
| `devices[].rate` | lost | the rate the controller is set to send at, not a limit of the device | Device |
| `devices[].patch` | lost | how the controller reads a patch: scripted or matched replies the importer does not convert | Device |
| `devices[].messages` | carried | the templates a SysEx message names by id | Device |
| `devices[].messages[].id` | structure | followed from the `data` of a message | Device |
| `devices[].messages[].direction` | lost | read when the message gives none (`in` is never sent); lost when the message gives its own, or no message names the template | Device |
| `devices[].messages[].data` | carried | the `frame` of the command of each message that names the template | Device |
| `overlays[].id` | structure | followed from `overlayId` | Overlay |
| `overlays[].items` | carried | the `values` of a control | Overlay |
| `overlays[].items[].value` | conditional | a whole value inside the scale of the way of a control that refers to the overlay | Overlay |
| `overlays[].items[].label` | conditional | the `title` of a named value, of an item whose value is carried | Overlay |
| `overlays[].items[].bitmap` | lost | the look or the behaviour of the control on the controller | Overlay |
| `overlays[].items[].color` | lost | the look or the behaviour of the control on the controller | Overlay |
| `groups[].id` | structure | followed from `groupId` | Group |
| `groups[].pageId` | lost | the look or the behaviour of the control on the controller | Group |
| `groups[].name` | conditional | the `section` of the controls and commands carried; lost when none is in the group | Group |
| `groups[].bounds` | lost | the look of the group on the controller | Group |
| `groups[].color` | lost | the look of the group on the controller | Group |
| `groups[].variant` | lost | the look of the group on the controller | Group |
| `groups[].font` | lost | the look of the group on the controller | Group |
| `groups[].values` | lost | the look of the group on the controller | Group |
| `controls[].id` | lost | a number of the preset | Control |
| `controls[].type` | lost | the look of the control; `pad` is followed to read `offValue` and `onValue` | Control |
| `controls[].name` | conditional | the `title`; lost when nothing of the control is carried | Control |
| `controls[].color` | lost | the look or the behaviour of the control on the controller | Control |
| `controls[].variant` | lost | the look or the behaviour of the control on the controller | Control |
| `controls[].mode` | lost | the look or the behaviour of the control on the controller | Control |
| `controls[].font` | lost | the look or the behaviour of the control on the controller | Control |
| `controls[].bounds` | lost | the look or the behaviour of the control on the controller | Control |
| `controls[].pageId` | lost | the look or the behaviour of the control on the controller | Control |
| `controls[].controlSetId` | lost | the look or the behaviour of the control on the controller | Control |
| `controls[].groupId` | structure | followed to the group, whose name is the `section` | Control |
| `controls[].visible` | lost | the look or the behaviour of the control on the controller | Control |
| `controls[].inputs` | lost | the pots of the controller | Control |
| `controls[].values` | carried | one control or command per value | Control |
| `controls[].events` | lost | scripts of the controller | Control |
| `controls[].valueRefs` | lost | values of the controller shown by the control | Control |
| `controls[].values[].id` | lost | a name inside the preset | Value |
| `controls[].values[].name` | lost | how the value is shown on the controller | Value |
| `controls[].values[].min` | conditional | a bound of the range when the message gives none for that side; lost when it differs from the range carried, when the range lacks the other bound, or when nothing takes it | Value |
| `controls[].values[].max` | conditional | as `min` | Value |
| `controls[].values[].defaultValue` | lost | the value loaded with the preset, not that of an initialised sound | Value |
| `controls[].values[].placeholder` | lost | how the value is shown on the controller | Value |
| `controls[].values[].sensitivity` | lost | how the controller turns the value | Value |
| `controls[].values[].formatter` | lost | a script of the controller | Value |
| `controls[].values[].function` | lost | a script of the controller | Value |
| `controls[].values[].overlayId` | structure | followed to the overlay, whose items are the `values` | Value |
| `controls[].values[].message` | conditional | a way or a command; a message of a lost kind, a claimed one or a template OSD does not write is lost | Message |
| `controls[].values[].message.deviceId` | lost | the setting of the controller, no field of the device; it chooses the device that holds a template named by id | Message |
| `controls[].values[].message.type` | conditional | the kind of the way, or a command (`sysex`); lost with its message (see "Kinds of message") | Message |
| `controls[].values[].message.id` | lost | a name inside the preset | Message |
| `controls[].values[].message.parameterNumber` | conditional | the numbers of the way, or `parameter_numbers` of the source for a command; lost with its message | Message |
| `controls[].values[].message.min` | conditional | the first bound of the range of the way or of the argument; lost when the range lacks the other bound, when a pad gives its own, or when nothing takes it | Message |
| `controls[].values[].message.max` | conditional | the second bound, as `min` | Message |
| `controls[].values[].message.onValue` | lost | named on every pad; with `offValue` below it, it is carried as the second bound of the range, but no pad is written back; lost alone, not above `offValue`, or on another control | Message |
| `controls[].values[].message.offValue` | lost | named on every pad; with `onValue` above it, it is carried as the first bound of the range, but no pad is written back; lost alone, not below `onValue`, or on another control | Message |
| `controls[].values[].message.value` | lost | a value the controller sends, no field of the way | Message |
| `controls[].values[].message.lsbFirst` | lost | how the controller encodes or sends the value | Message |
| `controls[].values[].message.resetRpn` | lost | how the controller encodes or sends the value | Message |
| `controls[].values[].message.signMode` | lost | how the controller encodes or sends the value | Message |
| `controls[].values[].message.bitWidth` | lost | how the controller encodes or sends the value | Message |
| `controls[].values[].message.relative` | carried | `true`: the way sends and receives relative values (`transmit` and `recognize` `relative`); a text on a `relcc` message names the encoding of the steps and is lost | Message |
| `controls[].values[].message.relativeMode` | lost | the encoding of the steps: the way has no field for it | Message |
| `controls[].values[].message.accelerated` | lost | how the controller turns the value | Message |
| `controls[].values[].message.data` | conditional | the template, or its id, of a SysEx message: the `frame` of the command; lost with a template OSD does not write | Message |
| `controls[].values[].message.direction` | carried | of a SysEx message: `in` is never sent and gives no command, the loss at the message; lost on another kind | Message |
| `template.value.rules[].type` | lost | a rule that names a parameter type takes another parameter than the value | SysEx template |
| `template.value.rules[].parameterNumber` | lost | another parameter than the value | SysEx template |
| `template.value.rules[].parameterBitPosition` | conditional | 0 or 7: a whole 7-bit byte of the value; any other is lost with the command | SysEx template |
| `template.value.rules[].byteBitPosition` | conditional | 0; any other is lost with the command | SysEx template |
| `template.value.rules[].bitWidth` | conditional | 7; any other is lost with the command | SysEx template |
| `template.parameter.parameterBitPosition` | lost | a byte built from the parameter number has no form in OSD | SysEx template |
| `template.parameter.byteBitPosition` | lost | a byte built from the parameter number has no form in OSD | SysEx template |
| `template.parameter.bitWidth` | lost | a byte built from the parameter number has no form in OSD | SysEx template |
| `template.checksum.algorithm` | conditional | `roland`, `waldorf`, `fractal`; another is lost with the command | SysEx template |
| `template.checksum.start` | conditional | a whole number; another is lost with the command | SysEx template |
| `template.checksum.length` | conditional | a whole number of bytes, 1 or more; another is lost with the command, and so is a checksum without it (default 0: no byte summed) | SysEx template |
| `template.function.name` | lost | a Lua function has no form in OSD and is not read | SysEx template |
| `template.any` | lost | any other element has no form in OSD | SysEx template |

## The OSD 0.7 device

What a device of 0.7 keeps through an export then an import. `carried`: it comes back equal; `lost`: it is in the losses of the export; `conditional`: either, as its condition says. A block lost whole has one row; `_*` is the `_` notes of an object.

| OSD field | Fate | Reason or condition |
|---|---|---|
| `/$schema` | carried |  |
| `/capabilities` | lost | The preset holds no script and declares no capability |
| `/role_vocabulary` | lost | no role is written |
| `/midi_of` | lost | the preset carries controls and SysEx messages only |
| `/contexts` | lost | the preset carries controls and SysEx messages only |
| `/loads_sounds_of` | lost | the preset carries controls and SysEx messages only |
| `/sound_transfers` | lost | the preset carries controls and SysEx messages only |
| `/resources` | lost | the preset carries controls and SysEx messages only |
| `/identity` | lost | the preset carries controls and SysEx messages only; `/identity/manufacturerId` is named too |
| `/programs` | lost | the preset carries controls and SysEx messages only |
| `/sysex` | lost | the preset carries controls and SysEx messages only |
| `/params` | lost | the preset carries controls and SysEx messages only |
| `/triggers` | lost | the preset carries controls and SysEx messages only |
| `/midi_out` | lost | the preset carries controls and SysEx messages only |
| `/midi_in` | lost | the preset carries controls and SysEx messages only |
| `/replies` | lost | the preset carries controls and SysEx messages only |
| `/subscriptions` | lost | the preset carries controls and SysEx messages only |
| `/custom_commands` | lost | the preset carries controls and SysEx messages only |
| `/sources` | lost | no field for the credit and the licence |
| `/midi_of_sources` | lost | the preset carries controls and SysEx messages only |
| `/_coverage` | lost | the preset carries controls and SysEx messages only |
| `/_*` | lost | no field for a note |
| `/commands` | conditional | a command whose frame a SysEx template holds is a control of the preset |
| `/device` | conditional | carried when each of its properties is |
| `/device/name` | conditional | the preset name, cut to 20 characters |
| `/device/vendor` | lost | the preset has no field for it |
| `/device/osc_prefix` | conditional | no field: carried when it is the prefix the import builds from the name of the preset |
| `/device/device_id` | lost | the preset has no field for it |
| `/device/rate_limit_hz` | lost | the preset has no field for it |
| `/device/kind` | lost | the preset has no field for it |
| `/device/stands_for` | lost | the preset has no field for it |
| `/device/fidelity` | lost | the preset has no field for it |
| `/device/transport` | lost | the preset has no field for it |
| `/device/_*` | lost | the preset has no field for a note |
| `/cc_params` | conditional | carried when its entries are, without a channel or a note |
| `/cc_params/channel` | lost | the channel of a preset belongs to its devices, the setting of the controller |
| `/cc_params/entries` | conditional | a control with a way a message of the preset says, among the first 432, in the order of the groups |
| `/cc_params/_*` | lost | the preset has no field for a note |
| `/cc_params/entries/N/osc` | conditional | no field: carried when it is the address the import builds from the section and the name |
| `/cc_params/entries/N/title` | conditional | the control name: cut to 40 characters, numbered when taken in its group; a control without a title gets one from its role or its address |
| `/cc_params/entries/N/description` | lost | the preset has no field for it |
| `/cc_params/entries/N/value_description` | lost | the preset has no field for it |
| `/cc_params/entries/N/notes` | lost | the preset has no field for it |
| `/cc_params/entries/N/scale` | lost | the preset has no field for it |
| `/cc_params/entries/N/range` | lost | on a control without a way: no message is written |
| `/cc_params/entries/N/transmit` | lost | on a control without a way: no message is written |
| `/cc_params/entries/N/recognize` | lost | on a control without a way: no message is written |
| `/cc_params/entries/N/nrpn_lsb_range` | lost | on a control without a way: no message is written |
| `/cc_params/entries/N/default` | lost | `defaultValue` is the value loaded with the preset, not that of an initialised sound |
| `/cc_params/entries/N/priority` | lost | the preset has no field for it |
| `/cc_params/entries/N/step_count` | lost | the preset has no field for it |
| `/cc_params/entries/N/values` | conditional | the labels of an overlay, cut to 20 characters and kept apart |
| `/cc_params/entries/N/polarity` | lost | the preset has no field for a polarity |
| `/cc_params/entries/N/channel` | lost | the channel of a preset belongs to its devices, the setting of the controller |
| `/cc_params/entries/N/section` | conditional | the name of the group, cut to 40 characters |
| `/cc_params/entries/N/expects` | lost | no field: it chooses a pad or a list, which the import does not read back |
| `/cc_params/entries/N/transform` | lost | the preset holds no script of a descriptor |
| `/cc_params/entries/N/role` | lost | no field; its setting names a control without a title |
| `/cc_params/entries/N/ways` | conditional | a control of the preset sends one message: the first way it can say |
| `/cc_params/entries/N/readings` | lost | the preset has no field for it |
| `/cc_params/entries/N/_*` | lost | the preset has no field for a note |
| `/cc_params/entries/N/ways/M/ctrlType` | conditional | `cc`, `nrpn` and `rpn` of the way written |
| `/cc_params/entries/N/ways/M/ctrlIndex` | conditional | `parameterNumber`; a 14-bit CC pairs a CC from 0 to 31 with the CC 32 above it, else its MSB alone is written |
| `/cc_params/entries/N/ways/M/value_bits` | conditional | no field: the import reads 7 for a CC of one number and 14 for a 14-bit CC, an NRPN or an RPN |
| `/cc_params/entries/N/ways/M/range` | conditional | `min` and `max` of the message and of the value; carried on the way written |
| `/cc_params/entries/N/ways/M/transmit` | conditional | `relcc` or `relative: true`: a way relative in both directions comes back, another direction does not |
| `/cc_params/entries/N/ways/M/recognize` | conditional | `relcc` or `relative: true`: a way relative in both directions comes back, another direction does not |
| `/cc_params/entries/N/ways/M/nrpn_lsb_range` | lost | the preset has no field for it |
| `/cc_params/entries/N/ways/M/in` | lost | the preset has no field for a context |
| `/cc_params/entries/N/ways/M/slice` | lost | the preset has no field for it |
| `/cc_params/entries/N/ways/M/_*` | lost | the preset has no field for a note |
| `/commands/N/osc` | conditional | no field: carried when it is the address the import builds from the section and the name |
| `/commands/N/id` | lost | the id of a control of the preset is its place, not the number of the command |
| `/commands/N/title` | conditional | the control name: cut and numbered as that of a control; a command without a title gets one from its address |
| `/commands/N/description` | lost | the preset has no field for it |
| `/commands/N/section` | conditional | the name of the group, cut to 40 characters |
| `/commands/N/notes` | lost | the preset has no field for it |
| `/commands/N/args` | conditional | the one argument `value` a template sends |
| `/commands/N/readings` | lost | the preset has no field for it |
| `/commands/N/pre` | lost | the preset computes no value of a descriptor |
| `/commands/N/frame` | conditional | a template: fixed bytes between F0 and F7, the argument `value` (u7, or u14 as u14_lsb and u14_msb, nothing added) and checksums (twos7, sum7, xor7); any other frame is lost |
| `/commands/N/forward` | lost | the preset sends MIDI only |
| `/commands/N/script` | lost | the preset holds no script of a descriptor |
| `/commands/N/_*` | lost | the preset has no field for a note |
| `/commands/N/args/M/name` | conditional | `value` is the argument of a template; any other is lost with its command |
| `/commands/N/args/M/type` | conditional | u7 and u14 in a template; any other is lost with its command |
| `/commands/N/args/M/range` | conditional | `min` and `max` of the message and of the value, on a command written |
| `/commands/N/args/M/values` | lost | a template names no value |
| `/commands/N/args/M/default` | lost | `defaultValue` is the value loaded with the preset, not that of an initialised sound |
| `/commands/N/args/M/_*` | lost | the preset has no field for a note |
| `/sources/N/id` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/type` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/provider` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/url` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/firmware` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/author` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/attribution` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/license` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/commit` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/profile` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| /sources/N/imported_at | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/notes` | lost | The preset has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/_*` | lost | The preset has no field for a note |

## Round trip on public files

The files are every `.epr` preset of the two public repositories this page and the interface cite for the extension, electraone/firmware-mk1 (the maker's) and jorisroling/bitwig-electra-one, at the revisions below. The rule reads the names alone, not the figures. The files are not kept in this repository. The test `partner_figures` replays the count on the files of the folder `OSD_PARTNER_FILES` names, as `<folder>/electra-one/<last segment of the address, percent-decoded>`, and says it is skipped without it. Under `npm test`, a result is replayed from the cache while the value of the variable and the sources stay the same, and a file changed under the same path is not seen: the figures are proved by `OSD_PARTNER_FILES=<folder> npm run test:forcer -- osd-formats/tests/it/partner_figures.rs`.

A file is imported, then the device is exported in the same format. A value is each leaf of the JSON, under its attribute path with list indices written N, counted once under its kind. `structure`: a kind this page classes as structure the import follows. `lost`: a loss of the import covers the value or a block that holds it, even when the export writes an equal one on its own, or the exported file does not hold it equal on the paired element. `carried`: every other. Equal is as the format reads it: a number written as text equals that number, an omitted attribute equals the default the format's document gives. The three columns of a kind add up to the number of its values in the files.

A value belongs to its nearest object. The key of a control, of a value and of a message is the message (its type and parameter number, or the bytes of its template); of a template element, its message and its rank; of an overlay, the message of the first value that names it, and of an item, its number; of a page, a group and a device, its name; of any other object, the key of its parent with its own name, id or rank; the root occurs once. A template named by id is read at the message that names it. Elements of one key are paired in the order of the files, and an element left without a pair loses all its values. F0 and F7 in a template are the framing, a byte written as text is hexadecimal, and `relcc` is a relative `cc7`. `structure`: the attributes the table of this page gives that fate, and the number of a `message.data`, the id that names its template.

| Address | Revision | SHA-256 | Licence | Credit |
|---|---|---|---|---|
| <https://raw.githubusercontent.com/electraone/firmware-mk1/93ad3745c68fe3d54e094a00e5da8d788c3182a2/tools/compat/presets/compat.epr> | 93ad3745c68fe3d54e094a00e5da8d788c3182a2 | 4040dedc52d6b02cd63de9b4992344cc6064fa4f96bd825aea16e5b0b0ee348a | GPL-3.0 | Electra One, electraone/firmware-mk1 |
| <https://raw.githubusercontent.com/electraone/firmware-mk1/93ad3745c68fe3d54e094a00e5da8d788c3182a2/tools/compat/presets/stress-max.epr> | 93ad3745c68fe3d54e094a00e5da8d788c3182a2 | 3fcb743d5904bbc88c40dcbb83d8ccc791e1dc746b58700d1c79d391ba472b4a | GPL-3.0 | Electra One, electraone/firmware-mk1 |
| <https://raw.githubusercontent.com/electraone/firmware-mk1/93ad3745c68fe3d54e094a00e5da8d788c3182a2/tools/compat/presets/stress-mixed.epr> | 93ad3745c68fe3d54e094a00e5da8d788c3182a2 | dfbbe0956d5140e9e64d5444face467aa941398628feabe58b123b5cac4531ca | GPL-3.0 | Electra One, electraone/firmware-mk1 |
| <https://raw.githubusercontent.com/electraone/firmware-mk1/93ad3745c68fe3d54e094a00e5da8d788c3182a2/tools/compat/presets/stress-sysex.epr> | 93ad3745c68fe3d54e094a00e5da8d788c3182a2 | 921fbeda9ff8a733bf0f73323718149e2aa6c45683ac7a080f370748121801fb | GPL-3.0 | Electra One, electraone/firmware-mk1 |
| <https://raw.githubusercontent.com/jorisroling/bitwig-electra-one/b360395321e3f3b37e9893eb047865ff5089032e/Bitwig%20Control.epr> | b360395321e3f3b37e9893eb047865ff5089032e | 4f2250271c909071aaac0b0b4f036ec44239fb3f47442d78557c50f010128d61 | MIT | Joris Röling, jorisroling/bitwig-electra-one |

| Kind | Carried | Lost | Structure |
|---|---|---|---|
| `/controls/N/bounds/N` | 0 | 4200 | 0 |
| `/controls/N/color` | 0 | 1050 | 0 |
| `/controls/N/controlSetId` | 0 | 1050 | 0 |
| `/controls/N/id` | 0 | 1050 | 0 |
| `/controls/N/inputs/N/potId` | 0 | 1050 | 0 |
| `/controls/N/inputs/N/valueId` | 0 | 1050 | 0 |
| `/controls/N/mode` | 0 | 978 | 0 |
| `/controls/N/name` | 567 | 483 | 0 |
| `/controls/N/pageId` | 0 | 1050 | 0 |
| `/controls/N/type` | 0 | 1050 | 0 |
| `/controls/N/values/N/defaultValue` | 0 | 1914 | 0 |
| `/controls/N/values/N/formatter` | 0 | 411 | 0 |
| `/controls/N/values/N/function` | 0 | 3 | 0 |
| `/controls/N/values/N/id` | 0 | 1986 | 0 |
| `/controls/N/values/N/max` | 675 | 1203 | 0 |
| `/controls/N/values/N/message/data/N` | 3024 | 0 | 0 |
| `/controls/N/values/N/message/data/N/type` | 432 | 0 | 0 |
| `/controls/N/values/N/message/deviceId` | 0 | 1986 | 0 |
| `/controls/N/values/N/message/lsbFirst` | 0 | 54 | 0 |
| `/controls/N/values/N/message/max` | 729 | 1221 | 0 |
| `/controls/N/values/N/message/min` | 729 | 1221 | 0 |
| `/controls/N/values/N/message/offValue` | 0 | 36 | 0 |
| `/controls/N/values/N/message/onValue` | 0 | 36 | 0 |
| `/controls/N/values/N/message/parameterNumber` | 315 | 1671 | 0 |
| `/controls/N/values/N/message/type` | 747 | 1239 | 0 |
| `/controls/N/values/N/min` | 675 | 1203 | 0 |
| `/controls/N/values/N/overlayId` | 0 | 0 | 96 |
| `/controls/N/visible` | 0 | 1050 | 0 |
| `/devices/N/channel` | 0 | 14 | 0 |
| `/devices/N/id` | 0 | 14 | 0 |
| `/devices/N/name` | 0 | 14 | 0 |
| `/devices/N/patch/N/request/N` | 0 | 3 | 0 |
| `/devices/N/port` | 0 | 14 | 0 |
| `/devices/N/rate` | 0 | 13 | 0 |
| `/groups/N/bounds/N` | 0 | 660 | 0 |
| `/groups/N/color` | 0 | 165 | 0 |
| `/groups/N/id` | 0 | 0 | 165 |
| `/groups/N/label` | 0 | 156 | 0 |
| `/groups/N/name` | 0 | 9 | 0 |
| `/groups/N/pageId` | 0 | 165 | 0 |
| `/name` | 5 | 0 | 0 |
| `/overlays/N/id` | 0 | 0 | 9 |
| `/overlays/N/items/N/label` | 0 | 576 | 0 |
| `/overlays/N/items/N/value` | 192 | 384 | 0 |
| `/pages/N/defaultControlSetId` | 0 | 27 | 0 |
| `/pages/N/id` | 0 | 30 | 0 |
| `/pages/N/name` | 0 | 30 | 0 |
| `/projectId` | 0 | 5 | 0 |
| `/version` | 0 | 0 | 5 |
| **Total** | 8090 | 30524 | 275 |
