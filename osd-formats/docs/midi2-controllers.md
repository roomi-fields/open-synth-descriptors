# MIDI 2.0 controller list

The controller resources of MIDI-CI Property Exchange (M2-117-UM v1.0, tables 6 and 12, cited here and not copied): `ChCtrlList` and `AllCtrlList`, arrays of controls, and `CtrlMapList`, the named values the controls share. The format is JSON.

The standard names nine `ctrlType` values: `cc`, `chPress`, `pPress`, `nrpn`, `rpn`, `pBend`, and, for MIDI 2.0 only, `pnrc`, `pnac`, `pnp`. None describes a setting set by SysEx.

## What the export writes

One JSON object `{ "ChCtrlList": [...], "CtrlMapList": [...] }`. Each control of `cc_params` (CC, 14-bit CC, NRPN, RPN), in the order of the device, is one entry of `ChCtrlList`. A control with `values` gets a `ctrlMapId` and one entry of `CtrlMapList`; equal lists of values share one id. A device with no `sources` is not exported (`NoSources`). The list has no place for a credit or a licence: the losses name `/sources`, and the command prints them.

The SysEx settings of the `params` block stay whole in the OSD descriptor and are not written: the standard has no `ctrlType` for them. The losses name them once, at `/params`, with the number of entries not written.

The standard requires a `title`: the export writes the `title` of the control, and, only when `title` is absent, the `paramPath` of its role, else its `section` when that starts with `/`, else its `osc` address; the `description` of the control is written when it has one. A control without a `title` comes back from a round trip with the derived title as its explicit `title`. A `section` is written as a `paramPath` only for a control without a role and only when it starts with `/`; a free section is never written as a path.

## What the import reads

A `ChCtrlList` or `AllCtrlList` array, or the object above. A `ctrlType` outside the nine of the standard (`param` among them), a `ctrlMapId` that the object's `CtrlMapList` does not hold, an entry without `ctrlType` or `ctrlIndex`, or a JSON that is not a list is refused (`InvalidFile`) at its JSON pointer. An entry of `chPress`, `pPress` or `pBend` becomes a way of that `ctrlType`. 

An entry of `pnrc`, `pnac` or `pnp` is a type of the standard that the descriptor has no field for: it is not carried, and the losses name it at the entry's pointer. The import never makes a `params` block, so it never makes up a SysEx frame. A `paramPath` whose segments are known terms of the role vocabulary (state 1) becomes a role; another is kept as the control's `section` when it starts with `/`, and the losses name it "no known role" at the entry's pointer. The device has the source's author as vendor (else the host of its url) and the name "MIDI 2.0 device": the list holds no identity, and the losses name `/vendor` and `/name`. The source of the descriptor is the one given to the import. The date of the import the caller passes in that source (`imported_at`) is kept as the note `_imported_at` of the source added, its value unchanged, 0.7 having no field for it: a `_imported_at` the source already holds stays, and the date goes under `_imported_at_2` (then `_imported_at_3`, then the next number not taken). It is no loss, no date is invented, and the other keys of the source are kept.

## Properties of the controller object

The fate is `carried` when the export writes it and the import reads it back, `lost` otherwise.

| Property | Fate | Reason | Piece |
|---|---|---|---|
| `title` | carried | the `title` of the control, written and read back; the export derives one when `title` is absent; a title that is not a string or is empty is not read and is named at its pointer | M2-117-UM v1.0, tables 6 and 12: title; spec/0.6.md |
| `description` | carried | the `description` of the control, written and read back; an empty one is kept; one that is not a string is not read and is named at its pointer | M2-117-UM v1.0, tables 6 and 12: description |
| `channel` | carried | the norm counts it 1-256, across the Groups of a Function Block; OSD's is 0 to 15, one Group: the export writes it + 1, the import reads it − 1 and refuses a channel above 16 naming its Group, or below 1 | M2-117-UM v1.0, tables 6 and 12: channel |
| `ctrlType` | carried | `cc`, `nrpn`, `rpn`, `chPress`, `pPress` and `pBend` are ways, written and read back; `pnrc`, `pnac` and `pnp` are lost on import (0.7 has no ctrlType for them) | M2-117-UM v1.0, tables 6 and 12: ctrlType |
| `ctrlIndex` | carried | `[cc]` or `[msb, lsb]`, the numbers of the address fields | M2-117-UM v1.0, tables 6 and 12: ctrlIndex |
| `priority` | carried | same meaning, 1 to 5 | M2-117-UM v1.0, tables 6 and 12: priority |
| `default` | carried | an unsigned 32-bit value in the norm, the raw value of the range in OSD: scaled up and back by M2-115-U v1.0.2, Min-Center-Max (CC, NRPN, RPN of LSB 32 to 127) or Zero-Extension (RPN of LSB 0 to 31); a negative default is lost | M2-117-UM v1.0, tables 6 and 12: default |
| `transmit` | carried | the same four words | M2-117-UM v1.0, tables 6 and 12: transmit |
| `recognize` | carried | the same four words | M2-117-UM v1.0, tables 6 and 12: recognize |
| `numSigBits` | carried | the bits used by the 32-bit values (32 when not provided): follows from the range and from the LSB of a 14-bit CC (the LSB is the MSB + 32); a count with no range gives the range | M2-117-UM v1.0, tables 6 and 12: numSigBits |
| `paramPath` | carried | the role, a path of parts and a setting; a path with no known role is kept as the section | M2-117-UM v1.0, tables 6 and 12: paramPath |
| `typeHint` | carried | `expects` by the table of spec/0.6.md; `trigger` is written as `momentary`, `clock` has no hint, `relative` has no `expects` word | M2-117-UM v1.0, tables 6 and 12: typeHint |
| `ctrlMapId` | carried | the named values, a map shared by controls with equal lists; the numbering of the ids is not kept | M2-117-UM v1.0, tables 6 and 12: ctrlMapId |
| `stepCount` | carried | the number of steps, 2 or more | M2-117-UM v1.0, tables 6 and 12: stepCount |
| `minMax` | carried | two unsigned 32-bit values in the norm, the range in OSD: scaled as `default` is (M2-115-U v1.0.2, Min-Center-Max or Zero-Extension); a negative bound is lost | M2-117-UM v1.0, tables 6 and 12: minMax |
