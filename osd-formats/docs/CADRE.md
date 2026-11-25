# osd-formats — frame

## Role

- **R1** `osd-formats` is the one place where OSD meets the formats others already use: it reads a file of another format into an OSD descriptor, and writes a descriptor in another format: the pencilresearch/midi CSV and its triggers CSV, Electra One presets, MIDNAM, OSCQuery, `midi2-controllers`, the controller resources of MIDI 2.0 Property Exchange, and the Deluge device definition (`deluge`, write only: the Deluge reads it, it is not imported), and last the skeleton of a KnobKraft Orm adaptation (`knobkraft`, write only: an adaptation is code that only its host runs, it is not imported).

## Receives

- **R2** The text of a file in another format and the name of that format; or a resolved device from `osd` and the format to write.

## Returns

- **R3** An OSD descriptor, or the text of the file in the other format, each with the source's credit and licence.
- **R4** With every result, the list of what could not be carried: each element named, with the reason.
- **R5** A typed fault for every refusal.

## Knows

- **R6** The `osd` types, and each format's own specification.

## Does not know

- **R7** The command, any catalogue, any service, any network, ports, where files are stored.

## Refuses

- **R8** Losing something silently: what a translation cannot carry is in the list it returns.
- **R9** Inventing a value the source does not give.
- **R10** Dropping the source's credit or licence: a device without sources is not written.
- **R11** A file that is not valid in its format.
- **R12** Running another format's code: a KnobKraft adaptation is read by its host, not here.
- **R13** Any I/O.

## Invariants

- **R14** Every export is valid against its format's own schema or DTD.
- **R15** What an import returns reads strictly with `osd` in the latest version.
- **R16** A descriptor exported then imported back keeps everything the format can carry.

## Cost

- **R17** Translating one device takes under 10 ms on a desktop machine at rest.

## Structure

The model is pandoc's: one reader and one writer per format, each a module of its own behind one common contract.

- **R18** Each format is one module of its own, which fulfils one common contract: its name, its media type, its extension, and the import and the export it offers. The list of formats holds one line per module, and no other place of the crate names a format.
- **R19** Each format has its page (`docs/<format>.md`: what it carries, what it loses, its source and its licence) and its tests; a guard refuses a format without its page or without its tests.
- **R20** A format is changed or added by a pull request that holds its module, its page, its tests and its line in the list; the guide of contributors (`CONTRIBUTING.md`, « Add a converter ») says how.
