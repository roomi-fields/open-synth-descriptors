# OSCQuery

The namespace of the OSC addresses a program answers, as the OSCQuery proposal defines it (github.com/Vidvox/OSCQueryProposal, the README, cited here by section and not copied). The proposal has no machine-readable schema: the checks of `osd-formats` are written from it, each citing it. A test needs a line below for each attribute of a node and each key of a range, and the checker of an export refuses a namespace that breaks the proposal (a `FULL_PATH` that is not the path, a type that is no tag, a `RANGE` of the wrong length, an `ACCESS` out of 0 to 3, an attribute the proposal does not have, a container with a type).

The first behaviour is the one of SyntHub's service. Where the service departs from the proposal, the export follows the proposal, and the difference is listed with the reason « le service s'écarte de la référence » in the section "Differences from the service" below.

## What the export writes

A device becomes a namespace: the root container holds one container, named after the first segment of the device's `osc_prefix` (the one container the addresses share), and below it the containers and methods of the addresses of its controls, its commands and its triggers. A control is a method with `ACCESS` 3 (read and write); a command is a method with `ACCESS` 2 (write only); a method's `TYPE` is the tags of its arguments, its `RANGE` the bounds of each argument, its `EXTENDED_TYPE` the names of its arguments. The `DESCRIPTION` of the device node carries the credit and the licence of the sources. A device with no `sources` is not exported (`NoSources`); an address two nodes would take is named in the losses; a capture of a command's address (`{n}`) stays `{n}` in its segment, as the descriptor 0.4 wrote it and as the service's export does, so `/track/{n}/volume` is a method under the containers `track` and `{n}`; a command whose address a node already holds, or whose path has a method on it, is named in the losses; no value the descriptor does not give is invented. The `values` of an argument are the `VALS` of its `RANGE`, the list the proposal says: ascending integers for an integer argument, labels for a text argument; the labels of an integer argument, the integers of a text argument have no place in it and are in the losses. A controller entry with no `range` is written with the bounds MIDI fixes for its kind: 0 to 127 for a 7-bit controller, 0 to 16383 for a 14-bit one, an NRPN or an RPN; an entry of no such kind is written with no bound. An entry with no way (its message is to be completed), of `cc_params` or `params`, is written with the entry's `range` as its `RANGE` when it has one, and each of its `scale`, `transmit`, `recognize` and `nrpn_lsb_range` is named in the losses.

## What the import reads

The JSON is read as a namespace by the proposal (`InvalidFile`, at its JSON pointer, when it is not one). The unique container at the head is the prefix and the device's name; a method with `ACCESS` 2 or 3 becomes a command, its arguments typed by its tags and named by its `EXTENDED_TYPE`; a segment, or a part of a segment, of the form `{name}` (letters, digits and `_`) is a capture, so the command's address is an array of text and capture segments; a segment with braces that is no capture (`{a,b}`, `{1x}`, two captures side by side) is not imported and the losses name its place; a read-only method (`ACCESS` 1) sends nothing and is lost. A `VALS` is named in the losses at its pointer with its reason and no `values` is read from it: the descriptor's `values` are labels with their integers, and a list has only one of the two (a `VALS` that is not a list is named whole); the round trip of the named values holds for MIDNAM alone, OSCQuery gives back the bounds of the range and loses the labels. The source of the descriptor is the one given to the import: the `DESCRIPTION` is never copied as a credit or a licence. The date of the import the caller passes in that source (`imported_at`) is kept as the note `_imported_at` of the source added, its value unchanged, 0.7 having no field for it: a `_imported_at` the source already holds stays, and the date goes under `_imported_at_2` (then `_imported_at_3`, then the next number not taken). It is no loss, no date is invented, and the other keys of the source are kept.

## Attributes of a node and keys of a range

The fate is `carried` when the export writes it and the import reads it back, `lost` otherwise.

| Attribute | Fate | Reason | Piece |
|---|---|---|---|
| `FULL_PATH` | carried | the address of the node | OSCQueryProposal, "Attributes": FULL_PATH |
| `CONTENTS` | carried | the containers below an address | OSCQueryProposal, "Attributes": CONTENTS |
| `TYPE` | carried | the tags of the arguments of a method; the arrays of the type are lost, and so is a tag the export writes as another (`h` as `i`, `d` as `f`, `S` and `c` as `s`, `F` as `T`), named at the `TYPE` of the method | OSCQueryProposal, "Attributes": TYPE and "Type tags" |
| `ACCESS` | carried | 2 for a command, 3 for a control; 1 is lost on import, nothing is read of 0 | OSCQueryProposal, "Attributes": ACCESS |
| `RANGE` | carried | the bounds of each argument | OSCQueryProposal, "Attributes": RANGE |
| `DESCRIPTION` | lost | the device node carries the credit of the sources on export; no other text is held | OSCQueryProposal, "Attributes": DESCRIPTION |
| `TAGS` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": TAGS |
| `EXTENDED_TYPE` | carried | the names of the arguments of a method | OSCQueryProposal, "Attributes": EXTENDED_TYPE |
| `UNIT` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": UNIT |
| `CRITICAL` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": CRITICAL |
| `CLIPMODE` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": CLIPMODE |
| `VALUE` | lost | the namespace holds the current value, the descriptor a default: none is invented | OSCQueryProposal, "Attributes": VALUE |
| `OVERLOADS` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": OVERLOADS |
| `MIN` | carried | the lowest value of an argument | OSCQueryProposal, "Attributes": RANGE, MIN |
| `MAX` | carried | the highest value of an argument | OSCQueryProposal, "Attributes": RANGE, MAX |
| `VALS` | lost | the export writes the list of permitted values (the integers of an integer argument, the labels of a text argument) and names in the losses what the list leaves out, the labels or the integers; the import reads no `values` from it, a list having no label or no integer, and names it | OSCQueryProposal, "Attributes": RANGE, VALS |

## Differences from the service

The service of SyntHub is not readable from this repository: no pair of its outputs is frozen here, so no difference is listed yet. The equality with its exporter is proved where it is replaced by this package (osc-bridge, OB-20q.7), and each difference found there enters this section with its reason.

## Round trip on public files

The files are the captures of a running OSCQuery program that gluon/oscq-inspector keeps in `tests/fixtures/` at the revision below, as its README lists them under "Real server captures", each namespace once (`max-hostinfo.json` is the same bytes as `max-root.json`, and `helper-hostinfo.json` is no namespace), that the import reads: Max 9 of Cycling '74. The rule reads where the files come from and whether they are read, not their figures. Two namespace files are refused and have no figure: [`helper-root.json`](https://raw.githubusercontent.com/gluon/oscq-inspector/5c79da2c3d0e5eac6613ff000e953d2ec0482501/tests/fixtures/helper-root.json), the capture of the Vidvox OSCQuery Helper (`InvalidFile` at `/CONTENTS/tuples/CONTENTS/first/RANGE`: the import asks one entry of `RANGE` per argument of `TYPE` and counts the arguments inside an array of the type one by one); and [`docs/example-http.json`](https://raw.githubusercontent.com/Cycling74/rnbo.oscquery.runner/09326e9f3acd1a54a8b9ea39c1b9314eb6d2f2f8/docs/example-http.json) of Cycling '74's runner (MIT), whose `EXTENDED_TYPE` is the text `list` beside an empty `TYPE`, where the proposal gives one entry per argument (`InvalidFile` at `/CONTENTS/rnbo/CONTENTS/inst/CONTENTS/0/CONTENTS/jack/CONTENTS/audio_ins/EXTENDED_TYPE`). The files are not kept in this repository. The test `partner_figures` replays the count on the files of the folder `OSD_PARTNER_FILES` names, as `<folder>/oscquery/<last segment of the address, percent-decoded>`, and says it is skipped without it. Under `npm test`, a result is replayed from the cache while the value of the variable and the sources stay the same, and a file changed under the same path is not seen: the figures are proved by `OSD_PARTNER_FILES=<folder> npm run test:forcer -- osd-formats/tests/it/partner_figures.rs`.

A file is imported, then the device is exported in the same format. A value is each leaf of the JSON, under its attribute path inside its node, with list indices written N, counted once under its kind. `structure`: a kind this page classes as structure the import follows. `lost`: a loss of the import covers the value or a block that holds it, even when the export writes an equal one on its own, or the exported file does not hold it equal on the paired element. `carried`: every other. Equal is as the format reads it: a number written as text equals that number, an omitted attribute equals the default the format's document gives. The three columns of a kind add up to the number of its values in the files.

A value belongs to its node, whose key is its address. Elements of one key are paired in the order of the files, and an element left without a pair loses all its values.

| Address | Revision | SHA-256 | Licence | Credit |
|---|---|---|---|---|
| <https://raw.githubusercontent.com/gluon/oscq-inspector/5c79da2c3d0e5eac6613ff000e953d2ec0482501/tests/fixtures/max-root.json> | 5c79da2c3d0e5eac6613ff000e953d2ec0482501 | df3cf2d12655baafc878eed0bca1bb5b01b9c61468e5d7326ad3f35634f875cb | GPL-3.0-or-later | Julien Bayle, gluon/oscq-inspector |

| Kind | Carried | Lost | Structure |
|---|---|---|---|
| `/ACCESS` | 0 | 30 | 0 |
| `/FULL_PATH` | 12 | 24 | 0 |
| `/TYPE` | 0 | 30 | 0 |
| `/VALUE/N` | 0 | 30 | 0 |
| **Total** | 12 | 114 | 0 |
