# OSD — how a published version is corrected

This text states how a published version of OSD, and a published state of the vocabulary of roles, is corrected. It binds one conformance class, the publisher. It binds no reader and no descriptor.

## 1 Terms

### 1.1 Publisher

A publisher is the person or the tool that adds a version of OSD, or a state of the vocabulary of roles, to the files that adopters fetch.

### 1.2 Published

A version, or a state of the vocabulary of roles, is published when its files are recorded in `schemas/SHA256SUMS`. A published version is a folder `schemas/<x.y>/`; a published state is a folder `schemas/vocabulary/<state>/`. Each file of the folder is recorded with its checksum.

### 1.3 Patch version

A patch version x.y.z is a version that corrects the published version x.y. It has its own text `spec/<x.y.z>.md` and its own folder `schemas/<x.y.z>/`.

## 2 Rules

### 2.1 Published files

A publisher MUST NOT edit in place a file recorded in `schemas/SHA256SUMS`, whether or not anyone has adopted the version or the state that holds it.

### 2.2 A correction of a version

A publisher MUST publish a correction of a published version x.y as a patch version x.y.z.

### 2.3 The files of a patch version

A publisher MUST record the files of the folder `schemas/<x.y.z>/` in `schemas/SHA256SUMS`, beside the files of the folder `schemas/<x.y>/` it corrects.

### 2.4 The version a patch corrects

A publisher MUST NOT change a file of the folder `schemas/<x.y>/` when it publishes a patch version x.y.z.

### 2.5 A correction of the text alone

A publisher MUST give a patch version that corrects the text alone a folder `schemas/<x.y.z>/` that holds a copy of the unchanged schemas of x.y.

### 2.6 Fields and meanings

A publisher MUST NOT publish a patch version x.y.z that changes a field or the meaning of a field of x.y.

### 2.7 Descriptors

A publisher MUST publish a patch version x.y.z for which every descriptor valid for x.y is valid.

### 2.8 Conformance cases

A publisher MUST publish a patch version x.y.z that passes every conformance case of x.y unchanged.

### 2.9 The vocabulary of roles

A publisher MUST publish a correction of a published state of the vocabulary of roles as a new state, beside the corrected state.

### 2.10 The corrected state

A publisher MUST NOT change a file of the folder of a state of the vocabulary of roles that a new state corrects.

## 3 Departures (informative)

NOTE The conformance cases are the proof that a patch version changes no field and no meaning; the owner, who publishes, judges it.

NOTE The rule follows the patch versions of OpenAPI (3.0.1, 3.0.2): a correction is a new text at its own address. It departs from OpenAPI in one point. OpenAPI patch versions correct the text of the specification, while its JSON schemas live in the repository and are updated in place. OSD gives each patch version its own folder of schemas and its own checksums, so that nothing a reader has fetched at an address changes under it.

NOTE One correction in place exists, and it is the owner's one departure from the rule. On 2026-11-21 the citation of the standard in `schemas/vocabulary/1/vocabulary.schema.json` was corrected in place (the MIDI-CI Property Exchange Controller Resources are M2-117-UM v1.0, tables 6 and 12; the schema named a wrong version and a wrong table), because no one had adopted that state yet. The checksums record the corrected file. The rule holds from 2026-11-23; no other published file has been corrected in place.
