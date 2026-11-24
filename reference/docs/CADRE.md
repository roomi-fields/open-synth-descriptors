# osd-reference — frame

## Role

- **R1** `osd-reference` builds the reference of a version of the format from its schemas: one page per kind of descriptor, each property with its path, its type and its description.

## Receives

- **R2** The folder of the schemas of one version (`schemas/<version>/`), and an output folder.

## Returns

- **R3** One Markdown page per kind (device, profile, sound, fingerprint, firmware, assembly), which lists every property of the schema, at any depth; a property without description is listed and marked so.
- **R4** For each kind, the count of its properties and of those that carry a description, and the list of those that carry none.
- **R5** The same pages and the same counts for the same schemas.

## Knows

- **R6** JSON Schema (draft 2020-12), as far as the schemas of OSD use it.

## Does not know

- **R7** The reader: it reads none of `osd`, `osd-formats`, `osd-cli`, and no descriptor.
- **R8** Any network: a reference to another schema is followed in the folder given.

## Refuses

- **R9** Writing or correcting a description: a description lives in the schema, and a schema that lacks one is corrected by a patch version of the format.
- **R10** Stating a rule the schema does not state: what a field means beyond its description is in the text of the version.
- **R11** A page that is committed: the pages are built. Only the list of the properties without description is committed, as a generated file.

## Invariants

- **R12** A page lists as many properties as the count of its schema.
- **R13** The count of described properties of a kind never goes below its recorded floor.

## Cost

- **R14** Building the pages of one version takes under 5 s on a desktop machine at rest.
