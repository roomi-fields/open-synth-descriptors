# Deluge device definition

The device definition the Deluge reads (community firmware 1.3, DelugeFirmware at 680b93c, cited here by file and line and not copied): an XML `midiDevice` holding a `ccLabels` tag whose attributes are `number="name"`. The firmware only reads it; there is no import.

## What the export writes

An XML document: one comment per source with its credit and licence, then `<midiDevice>` holding one `<ccLabels ... />` with one attribute per CC, in ascending order of the numbers. Only a command with a CC from 0 to 119 is written. Its name is the role path, else its `section` when that starts with `/`, else its `osc` address; a free section is never a name. The name is escaped for XML and is not cut: the firmware keeps the text of the attribute as it is.

Two commands with the same CC write one attribute: the first in the order of the descriptor (the `cc_params` blocks, then their entries) keeps the number, each next one is named lost. A command with no CC (NRPN, RPN), a CC above 119, and each SysEx param are not written and are named lost at their JSON pointer. A written command's range, values, channel and each other field is named lost at its pointer. A device with no `_sources` is not exported (`NoSources`). The definition has no field for a credit: the losses name `/_sources`, and the comment carries it, with each `--` broken by a space.

## What the import reads

Nothing: the import is refused (`InvalidFile`), the Deluge does not import.

## Tags and attributes of the reader

The fate is `carried` when the export writes it, `lost` otherwise.

| Element | Fate | Reason | Piece |
|---|---|---|---|
| `midiDevice` | carried | the root tag of the definition | midi_instrument.cpp, readCCLabelsFromFile |
| `ccLabels` | carried | the tag whose attributes name the CC | midi_instrument.cpp:605-625 |
| `0-119` | carried | one attribute per CC number from 0 to 119, its value the name; a CC above 119 is lost | midi_instrument.cpp:605-625; src/definitions_cxx.hpp:770 |
| `hideUnlabeledCC` | lost | the descriptor has no field to hide the CC without a name: the export never writes it | midi_instrument.cpp:629 |
| `value` | lost | the attribute of `hideUnlabeledCC`, written with it | midi_instrument.cpp:629 |
