# MIDNAM

The names document of the MIDI Association: MIDINameDocument 1.0, its DTD `MIDINameDocument10.dtd` and the events DTD `MIDIEvents10.dtd` it includes. Both files are kept in `osd-formats/tests/data/` and cited here by element and attribute, not copied. The rules of the export are the DTD's: a test reads every declaration of the two files and needs a line for it in the table of the verifier of an export, and a line for it below. A document that breaks the DTD is refused on import (`InvalidFile`, at its line); on `xmllint --dtdvalid` the night plays the exports when the program is present.

The first behaviour is the one of SyntHub's service. Where the service departs from the DTD, the export follows the DTD, and the difference is listed with the reason « le service s'écarte de la référence » in the section "Differences from the service" below.

## What the export writes

A device becomes one `MasterDeviceNames` with its `Manufacturer` and `Model`, one `CustomDeviceMode`, one `ChannelNameSet` and the lists the descriptor holds: its controls become a `ControlNameList` (a `Control` is a CC, a 14-bit CC, an NRPN or an RPN), its triggers a `NoteNameList`, its programs a `PatchBank` of `Patch` numbers. The name of a control is its remark `_name`, otherwise the words of its address; a patch with no name is named `<bank> <number>`. `osc_prefix` is lost: MIDNAM names, it does not address. The `Author` holds the credit and the licence of the sources; a device with no `_sources` is not exported (`NoSources`). What the DTD cannot hold of the descriptor is in the losses, by its JSON pointer, with its reason; nothing is invented for what the descriptor does not say.

## What the import reads

Every element of the document is read by the DTD first. The first device and its first model are read; the controls, notes and patches are carried, and each element the descriptor cannot hold has a loss at its line (`line N`, the element or `Element@Attribute` leading the reason). The source of the descriptor is the one given to the import: the `Author` is never copied as a credit or a licence.

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
| `Control@Type` | carried | `cc`, `cc` with `cc_lsb`, `nrpn_msb` with `nrpn_lsb`, `rpn_msb` with `rpn_lsb` | MIDINameDocument10.dtd, ATTLIST Control |
| `Control@Number` | carried | the controller or parameter number | MIDINameDocument10.dtd, ATTLIST Control |
| `Control@Name` | carried | the remark `_name` of a control | MIDINameDocument10.dtd, ATTLIST Control |
| `Values` | carried | the range of a control | MIDINameDocument10.dtd, ELEMENT Values |
| `Values@Min` | carried | the lowest value of the range | MIDINameDocument10.dtd, ATTLIST Values |
| `Values@Max` | carried | the highest value of the range | MIDINameDocument10.dtd, ATTLIST Values |
| `Values@Default` | carried | the default value of a control | MIDINameDocument10.dtd, ATTLIST Values |
| `Values@Units` | lost | the descriptor holds no unit or mapping of a control's value | MIDINameDocument10.dtd, ATTLIST Values |
| `Values@Mapping` | lost | the descriptor holds no unit or mapping of a control's value | MIDINameDocument10.dtd, ATTLIST Values |
| `UsesValueNameList` | lost | the descriptor holds no names for the values of a control | MIDINameDocument10.dtd, ELEMENT UsesValueNameList |
| `UsesValueNameList@Name` | lost | the descriptor holds no names for the values of a control | MIDINameDocument10.dtd, ATTLIST UsesValueNameList |
| `ValueNameList` | lost | the descriptor holds no names for the values of a control | MIDINameDocument10.dtd, ELEMENT ValueNameList |
| `ValueNameList@Name` | lost | the descriptor holds no names for the values of a control | MIDINameDocument10.dtd, ATTLIST ValueNameList |
| `Value` | lost | the descriptor holds no names for the values of a control | MIDINameDocument10.dtd, ELEMENT Value |
| `Value@Number` | lost | the descriptor holds no names for the values of a control | MIDINameDocument10.dtd, ATTLIST Value |
| `Value@Name` | lost | the descriptor holds no names for the values of a control | MIDINameDocument10.dtd, ATTLIST Value |
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

The service of SyntHub is not readable from this repository (ticket OSD-sf1.7, question 2): no pair of its outputs is frozen here, so no difference is listed yet. The equality with its exporter is proved where it is replaced by this package (osc-bridge, OB-20q.7), and each difference found there enters this section with its reason.
