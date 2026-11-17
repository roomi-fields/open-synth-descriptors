# osd-formats — frame

## Role

- **R1** `osd-formats` is the one place where OSD meets the formats others already use: it reads a file of another format into an OSD descriptor, and writes a descriptor in another format.

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
- **R10** Dropping the source's credit or licence, and writing a device whose sources forbid sharing in that format: a typed fault naming the source.
- **R11** A file that is not valid in its format.
- **R12** Running another format's code: a KnobKraft adaptation is read by its host, not here.
- **R13** Any I/O.

## Invariants

- **R14** Every export is valid against its format's own schema or DTD.
- **R15** What an import returns reads strictly with `osd` in the latest version.
- **R16** A descriptor exported then imported back keeps everything the format can carry.

## Cost

- **R17** Translating one device takes under 10 ms on a desktop machine at rest.
