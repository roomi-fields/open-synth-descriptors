# MIDNAM

The names document of the MIDI Association: MIDINameDocument 1.0, its DTD `MIDINameDocument10.dtd` and the events DTD `MIDIEvents10.dtd` it includes. Both files are kept in `osd-formats/tests/data/` and cited here by element and attribute, not copied. The rules of the export are the DTD's: a test reads every declaration of the two files and needs a line for it in the table of the verifier of an export, and a line for it below. A document that breaks the DTD is refused on import (`InvalidFile`, at its line); on `xmllint --dtdvalid` the night plays the exports when the program is present.

The first behaviour is the one of SyntHub's service. Where the service departs from the DTD, the export follows the DTD, and the difference is listed with the reason « le service s'écarte de la référence » in the section "Differences from the service" below.

## What the export writes

A device becomes one `MasterDeviceNames` with its `Manufacturer` and `Model`, one `CustomDeviceMode`, one `ChannelNameSet` and the lists the descriptor holds: its controls become a `ControlNameList` (a `Control` is a CC, a 14-bit CC, an NRPN or an RPN), its triggers a `NoteNameList`, its programs a `PatchBank` of `Patch` numbers. The name of a control is its `title`, otherwise the words of its address; a patch with no name is named `<bank> <number>`. `osc_prefix` is lost: MIDNAM names, it does not address. The `Author` holds the credit and the licence of the sources; a device with no `sources` is not exported (`NoSources`). What the DTD cannot hold of the descriptor is in the losses, by its JSON pointer, with its reason; nothing is invented for what the descriptor does not say.

The `values` of a control are written as `ValueNameList` (one `Value` per label, its `Number` and its `Name`) that its `Values` uses by `UsesValueNameList`; equal lists are written once and shared. A control with no `range` is written with the bounds MIDI fixes for its type: 0 to 127 for a 7-bit controller, 0 to 16383 for a 14-bit one, an NRPN or an RPN.

## What the import reads

Every element of the document is read by the DTD first. The first device and its first model are read; the controls, notes and patches are carried, and each element the descriptor cannot hold has a loss at its line (`line N`, the element or `Element@Attribute` leading the reason). The `ValueNameList` of a control, inline or by `UsesValueNameList`, becomes its `values`; two names on the same number are both read, and a `Value` whose number is not whole is lost while the others are carried. A `Value` outside the range of a control that uses it is lost at the line of the `Value`, its reason naming the control; the `Values` of the control keeps its bounds and its default. The grouping of a `NoteGroup` is lost at its line (`NoteGroup@Name`), named or not, while its notes are read. A `Values` that states the bounds MIDI fixes for the type of its control (0 to 127, or 0 to 16383) gives an entry without `range` and no loss: those bounds are the ones MIDI imposes. A `Patch@Number` that is not a whole number and a `ProgramChange` that is no program change number are lost at their line. So is the `Name` of a set or of a list of names that the export writes otherwise (`ChannelNameSet`, `ControlNameList`, `NoteNameList`, `PatchNameList` and the references to them). The source of the descriptor is the one given to the import: the `Author` is never copied as a credit or a licence. The date of the import the caller passes in that source (`imported_at`) is kept as the note `_imported_at` of the source added, its value unchanged, 0.7 having no field for it: a `_imported_at` the source already holds stays, and the date goes under `_imported_at_2` (then `_imported_at_3`, then the next number not taken). It is no loss, no date is invented, and the other keys of the source are kept.

## Elements and attributes

The fate is `carried` when the export writes it and the import reads it back, `lost` otherwise. A line of the round trip keeps what the fate `carried` lists.

| Element | Fate | Reason | Piece |
|---|---|---|---|
| `MIDINameDocument` | carried | the root of the document, one device | MIDINameDocument10.dtd, ELEMENT MIDINameDocument |
| `Author` | lost | the credit of the file is not copied; the export writes the sources' credit and licence there | MIDINameDocument10.dtd, ELEMENT Author |
| `MasterDeviceNames` | carried | the device, its manufacturer and its model | MIDINameDocument10.dtd, ELEMENT MasterDeviceNames |
| `ExtendingDeviceNames` | lost | a descriptor describes one device, not an extension of another | MIDINameDocument10.dtd, ELEMENT ExtendingDeviceNames |
| `Manufacturer` | carried | the vendor of the device | MIDINameDocument10.dtd, ELEMENT Manufacturer |
| `Model` | carried | the name of the device | MIDINameDocument10.dtd, ELEMENT Model |
| `Device` | lost | the form with an id of a device has no field in the descriptor | MIDINameDocument10.dtd, ELEMENT Device |
| `Device@Name` | lost | the form with an id of a device has no field in the descriptor | MIDINameDocument10.dtd, ATTLIST Device |
| `Device@UniqueID` | lost | the descriptor has no unique id of a device | MIDINameDocument10.dtd, ATTLIST Device |
| `StandardDeviceMode` | lost | the descriptor holds no device modes | MIDINameDocument10.dtd, ELEMENT StandardDeviceMode |
| `StandardDeviceMode@Name` | lost | the descriptor holds no device modes | MIDINameDocument10.dtd, ATTLIST StandardDeviceMode |
| `CustomDeviceMode` | lost | the descriptor holds no device modes; the export writes one, named Default | MIDINameDocument10.dtd, ELEMENT CustomDeviceMode |
| `CustomDeviceMode@Name` | lost | the descriptor holds no device modes | MIDINameDocument10.dtd, ATTLIST CustomDeviceMode |
| `DeviceModeEnable` | lost | the descriptor holds no command that enters a mode | MIDINameDocument10.dtd, ELEMENT DeviceModeEnable |
| `DeviceModeDisable` | lost | the descriptor holds no command that leaves a mode | MIDINameDocument10.dtd, ELEMENT DeviceModeDisable |
| `SupportsStandardDeviceMode` | lost | the descriptor holds no device modes | MIDINameDocument10.dtd, ELEMENT SupportsStandardDeviceMode |
| `SupportsStandardDeviceMode@Name` | lost | the descriptor holds no device modes | MIDINameDocument10.dtd, ATTLIST SupportsStandardDeviceMode |
| `ChannelNameSetAssignments` | lost | the descriptor does not say which set each channel uses | MIDINameDocument10.dtd, ELEMENT ChannelNameSetAssignments |
| `ChannelNameSetAssign` | lost | the descriptor does not say which set each channel uses | MIDINameDocument10.dtd, ELEMENT ChannelNameSetAssign |
| `ChannelNameSetAssign@Channel` | lost | the descriptor does not say which set each channel uses | MIDINameDocument10.dtd, ATTLIST ChannelNameSetAssign |
| `ChannelNameSetAssign@NameSet` | lost | the descriptor does not say which set each channel uses | MIDINameDocument10.dtd, ATTLIST ChannelNameSetAssign |
| `ChannelNameSet` | lost | the descriptor has one set of names; the export writes one, named Names | MIDINameDocument10.dtd, ELEMENT ChannelNameSet |
| `ChannelNameSet@Name` | lost | the descriptor has one set of names | MIDINameDocument10.dtd, ATTLIST ChannelNameSet |
| `AvailableForChannels` | lost | the descriptor does not say on which channels its names apply | MIDINameDocument10.dtd, ELEMENT AvailableForChannels |
| `AvailableChannel` | lost | the descriptor does not say on which channels its names apply | MIDINameDocument10.dtd, ELEMENT AvailableChannel |
| `AvailableChannel@Channel` | lost | the descriptor does not say on which channels its names apply | MIDINameDocument10.dtd, ATTLIST AvailableChannel |
| `AvailableChannel@Available` | lost | the descriptor does not say on which channels its names apply | MIDINameDocument10.dtd, ATTLIST AvailableChannel |
| `PatchBank` | carried | a bank of programs, with its Bank Select | MIDINameDocument10.dtd, ELEMENT PatchBank |
| `PatchBank@Name` | carried | the name of the bank | MIDINameDocument10.dtd, ATTLIST PatchBank |
| `PatchBank@ROM` | lost | the descriptor does not say whether a bank is read-only | MIDINameDocument10.dtd, ATTLIST PatchBank |
| `UsesPatchNameList` | lost | a list of patch names shared by name has no field; the import reads the list it names | MIDINameDocument10.dtd, ELEMENT UsesPatchNameList |
| `UsesPatchNameList@Name` | lost | the reference is resolved on import and not kept | MIDINameDocument10.dtd, ATTLIST UsesPatchNameList |
| `PatchNameList` | carried | the programs of a bank | MIDINameDocument10.dtd, ELEMENT PatchNameList |
| `PatchNameList@Name` | lost | the descriptor does not name a list of programs | MIDINameDocument10.dtd, ATTLIST PatchNameList |
| `Patch` | carried | a program of a bank | MIDINameDocument10.dtd, ELEMENT Patch |
| `Patch@Number` | carried | the program number in its bank | MIDINameDocument10.dtd, ATTLIST Patch |
| `Patch@Name` | lost | the descriptor does not name its programs: the export names a patch by its bank and number | MIDINameDocument10.dtd, ATTLIST Patch |
| `Patch@ProgramChange` | carried | the program change number a program is recalled by | MIDINameDocument10.dtd, ATTLIST Patch |
| `UsesNoteNameList` | lost | a list of note names shared by name has no field; the import reads the list it names | MIDINameDocument10.dtd, ELEMENT UsesNoteNameList |
| `UsesNoteNameList@Name` | lost | the reference is resolved on import and not kept | MIDINameDocument10.dtd, ATTLIST UsesNoteNameList |
| `NoteNameList` | carried | the notes of the triggers | MIDINameDocument10.dtd, ELEMENT NoteNameList |
| `NoteNameList@Name` | lost | the descriptor does not name a list of notes | MIDINameDocument10.dtd, ATTLIST NoteNameList |
| `NoteGroup` | lost | the descriptor does not group its sounds: the notes of the group are read one by one | MIDINameDocument10.dtd, ELEMENT NoteGroup |
| `NoteGroup@Name` | lost | the descriptor does not group its sounds | MIDINameDocument10.dtd, ATTLIST NoteGroup |
| `Note` | carried | a trigger and its name | MIDINameDocument10.dtd, ELEMENT Note |
| `Note@Number` | carried | the note number of a trigger | MIDINameDocument10.dtd, ATTLIST Note |
| `Note@Name` | carried | the name of a trigger | MIDINameDocument10.dtd, ATTLIST Note |
| `UsesControlNameList` | lost | a list of control names shared by name has no field; the import reads the list it names | MIDINameDocument10.dtd, ELEMENT UsesControlNameList |
| `UsesControlNameList@Name` | lost | the reference is resolved on import and not kept | MIDINameDocument10.dtd, ATTLIST UsesControlNameList |
| `ControlNameList` | carried | the controls of the device | MIDINameDocument10.dtd, ELEMENT ControlNameList |
| `ControlNameList@Name` | lost | the descriptor does not name a list of controls | MIDINameDocument10.dtd, ATTLIST ControlNameList |
| `Control` | carried | a control: its binding, its name and its values | MIDINameDocument10.dtd, ELEMENT Control |
| `Control@Type` | carried | a way of `ctrlType` `cc`, `cc` with two numbers, `nrpn` or `rpn`; the first such way is written, each other way is a loss at `/cc_params/entries/N/ways/M` | MIDINameDocument10.dtd, ATTLIST Control |
| `Control@Number` | carried | the controller or parameter number | MIDINameDocument10.dtd, ATTLIST Control |
| `Control@Name` | carried | the `title` of a control | MIDINameDocument10.dtd, ATTLIST Control |
| `Values` | carried | the range of a control | MIDINameDocument10.dtd, ELEMENT Values |
| `Values@Min` | carried | the lowest value of the range | MIDINameDocument10.dtd, ATTLIST Values |
| `Values@Max` | carried | the highest value of the range | MIDINameDocument10.dtd, ATTLIST Values |
| `Values@Default` | carried | the default value of a control | MIDINameDocument10.dtd, ATTLIST Values |
| `Values@Units` | lost | the descriptor holds no unit or mapping of a control's value | MIDINameDocument10.dtd, ATTLIST Values |
| `Values@Mapping` | lost | the descriptor holds no unit or mapping of a control's value | MIDINameDocument10.dtd, ATTLIST Values |
| `UsesValueNameList` | lost | a list of value names shared by name has no field; the export writes one per list shared, the import reads the list it names | MIDINameDocument10.dtd, ELEMENT UsesValueNameList |
| `UsesValueNameList@Name` | lost | the reference is resolved on import and not kept | MIDINameDocument10.dtd, ATTLIST UsesValueNameList |
| `ValueNameList` | carried | the `values` of a control, equal lists written once; a list no control uses is lost | MIDINameDocument10.dtd, ELEMENT ValueNameList |
| `ValueNameList@Name` | lost | the descriptor does not name a list of values; the export names it to share it | MIDINameDocument10.dtd, ATTLIST ValueNameList |
| `Value` | carried | a label of the `values` of a control | MIDINameDocument10.dtd, ELEMENT Value |
| `Value@Number` | carried | the integer of a label; a number that is not whole is lost with its Value | MIDINameDocument10.dtd, ATTLIST Value |
| `Value@Name` | carried | the label of a value | MIDINameDocument10.dtd, ATTLIST Value |
| `MIDICommands` | carried | the commands of a bank: its Bank Select | MIDINameDocument10.dtd, ELEMENT MIDICommands |
| `PatchMIDICommands` | lost | what a patch adds to its bank is no field of the descriptor | MIDINameDocument10.dtd, ELEMENT PatchMIDICommands |
| `MIDIDelay` | lost | a delay between commands has no field in the descriptor | MIDINameDocument10.dtd, ELEMENT MIDIDelay |
| `MIDIDelay@Milliseconds` | lost | a delay between commands has no field in the descriptor | MIDINameDocument10.dtd, ATTLIST MIDIDelay |
| `NoteOn` | lost | a note played is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT NoteOn |
| `NoteOn@Channel` | lost | a note played is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST NoteOn |
| `NoteOn@Note` | lost | a note played is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST NoteOn |
| `NoteOn@Velocity` | lost | a note played is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST NoteOn |
| `NoteOff` | lost | a note released is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT NoteOff |
| `NoteOff@Channel` | lost | a note released is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST NoteOff |
| `NoteOff@Note` | lost | a note released is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST NoteOff |
| `NoteOff@Velocity` | lost | a note released is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST NoteOff |
| `PolyKeyPressure` | lost | a pressure is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT PolyKeyPressure |
| `PolyKeyPressure@Channel` | lost | a pressure is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST PolyKeyPressure |
| `PolyKeyPressure@Note` | lost | a pressure is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST PolyKeyPressure |
| `PolyKeyPressure@Pressure` | lost | a pressure is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST PolyKeyPressure |
| `ControlChange` | carried | the Bank Select (controls 0 and 32) of a bank | MIDIEvents10.dtd, ELEMENT ControlChange |
| `ControlChange@Channel` | lost | the descriptor has no channel for a bank select; the export writes none | MIDIEvents10.dtd, ATTLIST ControlChange |
| `ControlChange@Control` | carried | 0 for the MSB of a bank, 32 for its LSB | MIDIEvents10.dtd, ATTLIST ControlChange |
| `ControlChange@Value` | carried | the MSB or the LSB of the bank | MIDIEvents10.dtd, ATTLIST ControlChange |
| `ProgramChange` | lost | a program change in a bank's commands is no field: the patch number is the recall | MIDIEvents10.dtd, ELEMENT ProgramChange |
| `ProgramChange@Channel` | lost | a program change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST ProgramChange |
| `ProgramChange@Number` | lost | a program change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST ProgramChange |
| `ChannelKeyPressure` | lost | a pressure is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT ChannelKeyPressure |
| `ChannelKeyPressure@Channel` | lost | a pressure is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST ChannelKeyPressure |
| `ChannelKeyPressure@Pressure` | lost | a pressure is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST ChannelKeyPressure |
| `PitchBendChange` | lost | a pitch bend is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT PitchBendChange |
| `PitchBendChange@Channel` | lost | a pitch bend is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST PitchBendChange |
| `PitchBendChange@Value` | lost | a pitch bend is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST PitchBendChange |
| `AllSoundOff` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT AllSoundOff |
| `AllSoundOff@Channel` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST AllSoundOff |
| `ResetAllControllers` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT ResetAllControllers |
| `ResetAllControllers@Channel` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST ResetAllControllers |
| `LocalControl` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT LocalControl |
| `LocalControl@Channel` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST LocalControl |
| `LocalControl@Value` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST LocalControl |
| `AllNotesOff` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT AllNotesOff |
| `AllNotesOff@Channel` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST AllNotesOff |
| `OmniOff` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT OmniOff |
| `OmniOff@Channel` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST OmniOff |
| `OmniOn` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT OmniOn |
| `OmniOn@Channel` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST OmniOn |
| `MonoMode` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT MonoMode |
| `MonoMode@Channel` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST MonoMode |
| `MonoMode@Value` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST MonoMode |
| `PolyMode` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT PolyMode |
| `PolyMode@Channel` | lost | a channel mode message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST PolyMode |
| `ControlChange14` | lost | a 14-bit change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ELEMENT ControlChange14 |
| `ControlChange14@Channel` | lost | a 14-bit change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST ControlChange14 |
| `ControlChange14@Control` | lost | a 14-bit change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST ControlChange14 |
| `ControlChange14@Value` | lost | a 14-bit change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST ControlChange14 |
| `RPNChange` | lost | an RPN change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ELEMENT RPNChange |
| `RPNChange@Channel` | lost | an RPN change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST RPNChange |
| `RPNChange@RPN` | lost | an RPN change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST RPNChange |
| `RPNChange@Value` | lost | an RPN change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST RPNChange |
| `NRPNChange` | lost | an NRPN change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ELEMENT NRPNChange |
| `NRPNChange@Channel` | lost | an NRPN change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST NRPNChange |
| `NRPNChange@NRPN` | lost | an NRPN change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST NRPNChange |
| `NRPNChange@Value` | lost | an NRPN change in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST NRPNChange |
| `SysEx` | lost | a SysEx message in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ELEMENT SysEx |
| `SysExDeviceID` | lost | a SysEx message in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ELEMENT SysExDeviceID |
| `SysExDeviceID@Multiplier` | lost | a SysEx message in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST SysExDeviceID |
| `SysExDeviceID@Offset` | lost | a SysEx message in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST SysExDeviceID |
| `SysExChannel` | lost | a SysEx message in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ELEMENT SysExChannel |
| `SysExChannel@Multiplier` | lost | a SysEx message in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST SysExChannel |
| `SysExChannel@Offset` | lost | a SysEx message in a bank's commands is no field of the descriptor | MIDIEvents10.dtd, ATTLIST SysExChannel |
| `MTCQuarterFrame` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT MTCQuarterFrame |
| `MTCQuarterFrame@MessageType` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST MTCQuarterFrame |
| `MTCQuarterFrame@DataNibble` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST MTCQuarterFrame |
| `SongPositionPointer` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT SongPositionPointer |
| `SongPositionPointer@Position` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST SongPositionPointer |
| `SongSelect` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT SongSelect |
| `SongSelect@Number` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ATTLIST SongSelect |
| `TuneRequest` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT TuneRequest |
| `TimingClock` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT TimingClock |
| `Start` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT Start |
| `Continue` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT Continue |
| `Stop` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT Stop |
| `ActiveSensing` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT ActiveSensing |
| `SystemReset` | lost | a system message is no bank command of the descriptor | MIDIEvents10.dtd, ELEMENT SystemReset |

## Differences from the service

The service of SyntHub is not readable from this repository: no pair of its outputs is frozen here, so no difference is listed yet. The equality with its exporter is proved where it is replaced by this package (osc-bridge, OB-20q.7), and each difference found there enters this section with its reason.

## Round trip on public files

The files are the first ten `.midnam` files in byte order of name in `share/patchfiles` of Ardour at the revision below. The rule reads the names alone, not the figures. The files are not kept in this repository. The test `partner_figures` replays the count on the files of the folder `OSD_PARTNER_FILES` names, as `<folder>/midnam/<last segment of the address, percent-decoded>`, and says it is skipped without it. Under `npm test`, a result is replayed from the cache while the value of the variable and the sources stay the same, and a file changed under the same path is not seen: the figures are proved by `OSD_PARTNER_FILES=<folder> npm run test:forcer -- osd-formats/tests/it/partner_figures.rs`.

A file is imported, then the device is exported in the same format. A value is each attribute and each element text, under `Element@Attribute` or the element, counted once under its kind. `structure`: a kind this page classes as structure the import follows. `lost`: a loss of the import covers the value or a block that holds it, even when the export writes an equal one on its own, or the exported file does not hold it equal on the paired element. `carried`: every other. Equal is as the format reads it: a number written as text equals that number, an omitted attribute equals the default the format's document gives. The three columns of a kind add up to the number of its values in the files.

A value belongs to its element. The key of a Control is its type and number; of a Note, its number; of a PatchBank, its name; of a Patch, its bank and its program change, else its number; of a list of value or patch names, the control or bank that uses it; of any other element, its parent with its name, number or channel, else its rank among its siblings. Elements of one key are paired in the order of the files, and an element left without a pair loses all its values.

| Address | Revision | SHA-256 | Licence | Credit |
|---|---|---|---|---|
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/ART_Multiverb_Alpha_2_0.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | 6ab3bb48c82aad85d18769eba301c770ba460293ce2b416e695b1f437788e12e | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Access_Virus_A.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | 2e4574f005c79048ca95a9c724060089bf4990e405e628161a0a4e00b5f2222e | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Access_Virus_B_KB_Indigo.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | c6bfecf74d5e89b0cf7678e4903d90a85eeb666f68c5ee7c9d2549b4f03bd394 | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Access_Virus_Rack.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | ed3ab729305430c0bfc30077db2d4cd1fa1a699010fdeff564539e1304bfe7af | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Alesis_DM5.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | 846cb4b585f2c6d6ff5691c5a01134bf9ce56c5092983f8ee2f6214d63267e4f | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Alesis_MIDIVerb_4.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | 72b33708632a780f5bfcaabf787cd1944c97f27d56a54d9f8ad0e9b030788c61 | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Alesis_MIDIVerb_II.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | 0b46b88b025669a559330721c48cc06d83bfab5ab2457aca640ffd8e8be1a35f | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Alesis_MIDIVerb_III.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | e422a1ff0cdbc55108c54395cadf4822c5636b3532627155a78f684919755e3c | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Alesis_MicroVerb4.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | ae615b1472b92b3f2e527f6425c887d186984a1da4cc18cc3d588c60c584970c | GPL-2.0-or-later | the Ardour project; the Author written in each file |
| <https://raw.githubusercontent.com/Ardour/ardour/eb8afaddfae2eedb6a8c34b546b1f0b310f09343/share/patchfiles/Alesis_NanoBass.midnam> | eb8afaddfae2eedb6a8c34b546b1f0b310f09343 | ac445c80ce01277fee5b194705fb1aadcb64eda5cc50c74af2e54dedf2beb92b | GPL-2.0-or-later | the Ardour project; the Author written in each file |

| Kind | Carried | Lost | Structure |
|---|---|---|---|
| `Author` | 0 | 10 | 0 |
| `AvailableChannel@Available` | 0 | 160 | 0 |
| `AvailableChannel@Channel` | 0 | 160 | 0 |
| `ChannelNameSet@Name` | 0 | 10 | 0 |
| `ChannelNameSetAssign@Channel` | 0 | 145 | 0 |
| `ChannelNameSetAssign@NameSet` | 0 | 145 | 0 |
| `Control@Name` | 2 | 0 | 0 |
| `Control@Number` | 2 | 0 | 0 |
| `Control@Type` | 2 | 0 | 0 |
| `ControlChange@Control` | 46 | 0 | 0 |
| `ControlChange@Value` | 46 | 0 | 0 |
| `ControlNameList@Name` | 0 | 1 | 0 |
| `CustomDeviceMode@Name` | 0 | 10 | 0 |
| `Manufacturer` | 10 | 0 | 0 |
| `Model` | 10 | 2 | 0 |
| `Note@Name` | 1281 | 0 | 0 |
| `Note@Number` | 1281 | 0 | 0 |
| `NoteGroup@Name` | 0 | 76 | 0 |
| `NoteNameList@Name` | 0 | 21 | 0 |
| `Patch@Name` | 0 | 3621 | 0 |
| `Patch@Number` | 836 | 2785 | 0 |
| `Patch@ProgramChange` | 3620 | 1 | 0 |
| `PatchBank@Name` | 42 | 0 | 0 |
| `PatchNameList@Name` | 2 | 0 | 0 |
| `SysEx` | 0 | 1 | 0 |
| `UsesControlNameList@Name` | 0 | 1 | 0 |
| `UsesNoteNameList@Name` | 0 | 21 | 0 |
| `UsesPatchNameList@Name` | 0 | 2 | 0 |
| **Total** | 7180 | 7172 | 0 |
