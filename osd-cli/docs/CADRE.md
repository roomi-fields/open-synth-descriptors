# osd-cli — frame

## Role

- **R1** `osd-cli` is the `osd` command: what a person or an agent runs to work on a descriptor file without writing code.

## Receives

- **R2** The paths of descriptor files, and the word of the operation: validate, convert, init, import, export.

## Returns

- **R3** For a validation, a verdict per file, each fault with its file, its JSON pointer and its rule; exit code 0 when every file is right, 1 when a file has a fault, 2 when the command itself is misused.
- **R4** For a conversion, the descriptor in the latest version, or the fault that prevents it.
- **R5** For an init, a template of the kind asked, valid against the latest version's schema once its marked places are filled.

## Knows

- **R6** `osd` and `osd-formats`, by their interfaces.

## Does not know

- **R7** Any device, any MIDI port, any network, any service, any catalogue.

## Refuses

- **R8** Saying a descriptor is true to its device: that is checked with the device plugged in, by an engine such as osc-bridge.
- **R9** Overwriting a file it was not told to write.
- **R10** Holding a rule of the format or of a translation: each lives in `osd` or in `osd-formats`.

## Invariants

- **R11** What it says of a file is what `osd` says of its text.

## Cost

- **R12** Validating one descriptor takes under 100 ms on a desktop machine at rest, start of the program included.
