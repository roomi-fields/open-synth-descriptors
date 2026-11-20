# MIDI 2.0 controller list

The controller resources of MIDI-CI Property Exchange (M2-117-UM v1.0, tables 6 and 12, cited here and not copied): `ChCtrlList` and `AllCtrlList`, arrays of controls, and `CtrlMapList`, the named values the controls share. The format is JSON.

The standard names nine `ctrlType` values: `cc`, `chPress`, `pPress`, `nrpn`, `rpn`, `pBend`, and, for MIDI 2.0 only, `pnrc`, `pnac`, `pnp`. None describes a setting set by SysEx.

## What the export writes

One JSON object `{ "ChCtrlList": [...], "CtrlMapList": [...] }`. Each control of `cc_params` (CC, 14-bit CC, NRPN, RPN), in the order of the device, is one entry of `ChCtrlList`. A control with `values` gets a `ctrlMapId` and one entry of `CtrlMapList`; equal lists of values share one id. A device with no `_sources` is not exported (`NoSources`). The list has no place for a credit or a licence: the losses name `/_sources`, and the command prints them.

The SysEx settings of the `params` block stay whole in the OSD descriptor and are not written: the standard has no `ctrlType` for them. The losses name them once, at `/params`, with the number of entries not written.

The standard requires a `title`, the descriptor holds none: the title written is the `paramPath` of the control's role, else its `section` when that starts with `/`, else its `osc` address. A `section` is written as a `paramPath` only for a control without a role and only when it starts with `/`; a free section is never written as a path.

## What the import reads

A `ChCtrlList` or `AllCtrlList` array, or the object above. A `ctrlType` outside the nine of the standard (`param` among them), a `ctrlMapId` that the object's `CtrlMapList` does not hold, an entry without `ctrlType` or `ctrlIndex`, or a JSON that is not a list is refused (`InvalidFile`) at its JSON pointer. An entry of `chPress`, `pPress`, `pBend`, `pnrc`, `pnac` or `pnp` is a type of the standard that the descriptor has no field for: it is not carried, and the losses name it at the entry's pointer. The import never makes a `params` block, so it never makes up a SysEx frame. A `paramPath` whose segments are known terms of the role vocabulary (state 1) becomes a role; another is kept as the control's `section` when it starts with `/`, and the losses name it "no known role" at the entry's pointer. The device has the source's author as vendor (else the host of its url) and the name "MIDI 2.0 device": the list holds no identity, and the losses name `/vendor` and `/name`. The source of the descriptor is the one given to the import.

## Properties of the controller object

The fate is `carried` when the export writes it and the import reads it back, `lost` otherwise.

| Property | Fate | Reason | Piece |
|---|---|---|---|
| `title` | lost | the label belongs to the role and to the tool that lays it out: the export derives one, the import carries none and names it at its pointer | M2-117-UM v1.0, tables 6 and 12: title; docs/controls.md |
| `description` | lost | free text for people is not read by a program | M2-117-UM v1.0, tables 6 and 12: description |
| `channel` | carried | the MIDI channel of a CC, NRPN or RPN entry, 0 to 15 | M2-117-UM v1.0, tables 6 and 12: channel |
| `ctrlType` | carried | `cc`, `nrpn` and `rpn` are written and read back; `chPress`, `pPress`, `pBend`, `pnrc`, `pnac` and `pnp` are read and lost, the descriptor has no field for them | M2-117-UM v1.0, tables 6 and 12: ctrlType |
| `ctrlIndex` | carried | `[cc]` or `[msb, lsb]`, the numbers of the address fields | M2-117-UM v1.0, tables 6 and 12: ctrlIndex |
| `priority` | carried | same meaning, 1 to 5 | M2-117-UM v1.0, tables 6 and 12: priority |
| `default` | carried | the raw value in the space of the range | M2-117-UM v1.0, tables 6 and 12: default |
| `transmit` | carried | the same four words | M2-117-UM v1.0, tables 6 and 12: transmit |
| `recognize` | carried | the same four words | M2-117-UM v1.0, tables 6 and 12: recognize |
| `numSigBits` | carried | follows from the range and from the LSB of a 14-bit CC (the LSB is the MSB + 32); a count with no range gives the range | M2-117-UM v1.0, tables 6 and 12: numSigBits |
| `paramPath` | carried | the role, a path of parts and a setting; a path with no known role is kept as the section | M2-117-UM v1.0, tables 6 and 12: paramPath |
| `typeHint` | carried | `expects` by the table of docs/controls.md; `trigger` is written as `momentary`, `clock` has no hint, `relative` has no `expects` word | M2-117-UM v1.0, tables 6 and 12: typeHint |
| `ctrlMapId` | carried | the named values, a map shared by controls with equal lists; the numbering of the ids is not kept | M2-117-UM v1.0, tables 6 and 12: ctrlMapId |
| `stepCount` | carried | the number of steps, 2 or more | M2-117-UM v1.0, tables 6 and 12: stepCount |
| `minMax` | carried | the range | M2-117-UM v1.0, tables 6 and 12: minMax |
