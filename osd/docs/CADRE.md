# osd — frame

## Role

- **R1** `osd` turns OSD descriptor files into typed Rust values, and only that: it reads, resolves, reports capabilities and converts between versions.

## Receives

- **R2** The text of a descriptor file, and the kind expected (device, profile, sound, fingerprint, firmware) or the kind its `$schema` declares.
- **R3** For resolution, a file source: an object that returns the text of a descriptor named by its catalogue id (`maker/name`), or says it has none.

## Returns

- **R4** A typed descriptor, every field of its schema present in the types, documentary fields included (`_notes`, `section`, `_sources`, `_coverage`, value labels).
- **R5** A resolved device: the device with its maker profiles and shared MIDI applied, the files it was built from, and the capabilities it declares.
- **R6** A descriptor converted to a newer version, or the fault that prevents it.
- **R7** A typed fault for every refusal, naming the file, the JSON pointer and the rule.

## Knows

- **R8** The format, version by version: the schemas, the resolution rules, the converters.

## Does not know

- **R9** Bytes on a wire, MIDI ports, networks, servers, catalogues as folders, and what an engine does with a descriptor; the caller supplies the files, and the engine executes the device.

## Refuses

- **R10** A field its version does not define, a value outside its type or its closed list, a missing required field, a version it does not read.
- **R11** A shared-MIDI or profile link that names no file, forms a cycle, or goes deeper than 8.
- **R12** Any I/O, and any dependency that performs I/O.

## Invariants

- **R13** Every conformance case of every version it reads passes.
- **R14** What it returns for a descriptor depends only on the descriptor and the files its source returns.
- **R15** The schema its types describe equals the published schema of the newest version.

## Cost

- **R16** Reading and resolving one device takes under 50 µs per kB of the files it reads, and the whole catalogue under 2 s, on a desktop machine at rest; measured by a bench over the catalogue.
