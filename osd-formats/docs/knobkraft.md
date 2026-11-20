# KnobKraft Orm adaptation

KnobKraft Orm, by Christof Ruch (https://github.com/christofmuc/KnobKraft-orm, licence AGPL-3.0), is a librarian for synthesizer sounds. It knows a device through an adaptation: a Python module that its host, KnobKraft Orm, loads and runs. Its functions are named in the "Adaptation Programming Guide" and checked by the generic tests of the "Adaptation Testing Guide", both of KnobKraft Orm; they are cited here and not copied. Every text the export writes names KnobKraft Orm, its author and its licence, then the credit of each source of the device.

The guides are cited at the revision 9e2522cd44765603162b4afd31419c3597ff85fc of KnobKraft Orm (release 2.10.2 plus 9 commits). The file `adaptations/Adaptation Programming Guide.md` of that revision points to the maintained guide, `docs/programming-guide.md`; the sections below are the headings of that file.

## What the export writes

A skeleton: a Python text of comments (the credit), then the functions whose constants the descriptor gives, each under the exact name of the guide:

- `name()`, the device's name;
- `numberOfBanks()`, the count of `programs.banks`;
- `numberOfPatchesPerBank()`, the size of the banks when all are the same;
- `createEditBufferRequest(channel)`, the bytes of `programs.dump.request` when the dump is a `request` or an `edit_buffer` made of fixed bytes.

The export writes no value the descriptor does not give. Each other function is not written and is named lost, by its name, with its reason: it is left to write by hand. A device without `_sources` is not exported (`NoSources`), nor one without `sound_transfers` (`NoSoundTransfers`).

## What of an adaptation is data of a device, and what is code that only the host runs

The data of a device are the constants an adaptation returns: its name, its numbers of banks and of programs, the bytes of a request. The descriptor gives them, and the export writes them. Everything else of an adaptation is code that only the host runs: recognising a dump, reading a name or a number out of it, converting between messages, cutting a bank. The skeleton leaves that code to write by hand.

## What the import reads

Nothing: the import is refused (`InvalidFile`). osd-formats never reads, parses nor runs an adaptation, not even to take its constants (R12). That the skeleton passes the generic test suite of KnobKraft is proved outside this crate, by a test of the night that runs the suite on an exported skeleton.

## Functions of the interface

The fate is `written` when the export writes the function, `lost` otherwise. The piece is the guide, the section of its file `docs/programming-guide.md` that names the function, and the revision of KnobKraft Orm (9e2522c, in full above).

| Function | Fate | Reason | Piece |
|---|---|---|---|
| `name` | written | the descriptor gives the device's name | Adaptation Programming Guide, § Identity, 9e2522c |
| `numberOfBanks` | written | the count of the banks of the descriptor; lost when it gives none | Adaptation Programming Guide, § Storage size, 9e2522c |
| `numberOfPatchesPerBank` | written | the size of the banks when all are equal; lost otherwise | Adaptation Programming Guide, § Storage size, 9e2522c |
| `createEditBufferRequest` | written | the request bytes of the dump when fixed; lost when built at send time or absent | Adaptation Programming Guide, § Requesting the edit buffer from the synth, 9e2522c |
| `createDeviceDetectMessage` | lost | the detection message is code: the identity is not turned into it | Adaptation Programming Guide, § Device detection, 9e2522c |
| `needsChannelSpecificDetection` | lost | it follows the detection message | Adaptation Programming Guide, § Optionally specifying to not to channel specific detection, 9e2522c |
| `channelIfValidDeviceResponse` | lost | it reads a reply: code the descriptor does not give | Adaptation Programming Guide, § Checking if reply came, 9e2522c |
| `deviceDetectWaitMilliseconds` | lost | it follows the detection message | Adaptation Programming Guide, § Optionally specifying a different wait time than 200 ms., 9e2522c |
| `isEditBufferDump` | lost | recognising a dump is code | Adaptation Programming Guide, § Checking if a MIDI message is an edit buffer dump, 9e2522c |
| `convertToEditBuffer` | lost | converting a dump is code | Adaptation Programming Guide, § Creating the edit buffer to send, 9e2522c |
| `nameFromDump` | lost | reading a name out of a dump is code | Adaptation Programming Guide, § Getting the patch's name, 9e2522c |
| `renamePatch` | lost | writing a name into a dump is code | Adaptation Programming Guide, § Renaming patches, 9e2522c |
| `isDefaultName` | lost | the descriptor does not say which names are defaults | Adaptation Programming Guide, § Better handling of given names vs default names, 9e2522c |
| `calculateFingerprint` | lost | which bytes make the sound is code | Adaptation Programming Guide, § Better duplicate detection, 9e2522c |
| `createProgramDumpRequest` | lost | the descriptor gives one request of the current sound, not one per program | Adaptation Programming Guide, § Requesting a specific program at a specific memory position, 9e2522c |
| `isSingleProgramDump` | lost | recognising a dump is code | Adaptation Programming Guide, § Checking if a message is a program dump, 9e2522c |
| `convertToProgramDump` | lost | converting a dump is code | Adaptation Programming Guide, § Creating a message that stores a patch in a specific memory location, 9e2522c |
| `numberFromDump` | lost | reading a number out of a dump is code | Adaptation Programming Guide, § Program Dump Capability, 9e2522c |
| `createBankDumpRequest` | lost | the descriptor does not give a request of a whole bank | Adaptation Programming Guide, § Requesting a full bank dump, 9e2522c |
| `isPartOfBankDump` | lost | recognising a part of a bank dump is code | Adaptation Programming Guide, § Checking if a MIDI message is part of the reply, 9e2522c |
| `isBankDumpFinished` | lost | recognising the end of a bank dump is code | Adaptation Programming Guide, § Testing if all messages have been received, 9e2522c |
| `extractPatchesFromBank` | lost | cutting a bank dump is code | Adaptation Programming Guide, § Extracting the patches from a bank dump, 9e2522c |
| `convertPatchesToBankDump` | lost | building a bank dump is code | Adaptation Programming Guide, § Bank Dump Capability, 9e2522c |
| `friendlyBankName` | lost | the bank names are not turned into a function | Adaptation Programming Guide, § Better bank names, 9e2522c |
