# OSCQuery

The namespace of the OSC addresses a program answers, as the OSCQuery proposal defines it (github.com/Vidvox/OSCQueryProposal, the README, cited here by section and not copied). The proposal has no machine-readable schema: the checks of `osd-formats` are written from it, each citing it. A test needs a line below for each attribute of a node and each key of a range, and the checker of an export refuses a namespace that breaks the proposal (a `FULL_PATH` that is not the path, a type that is no tag, a `RANGE` of the wrong length, an `ACCESS` out of 0 to 3, an attribute the proposal does not have, a container with a type).

The first behaviour is the one of SyntHub's service. Where the service departs from the proposal, the export follows the proposal, and the difference is listed with the reason « le service s'écarte de la référence » in the section "Differences from the service" below.

## What the export writes

A device becomes a namespace: the root container holds one container, named after the first segment of the device's `osc_prefix` (the one container the addresses share), and below it the containers and methods of the addresses of its controls, its parameters (`params`), its commands and its triggers. A control is a method with `ACCESS` 3 (read and write); a command is a method with `ACCESS` 2 (write only); a method's `TYPE` is the tags of its arguments, its `RANGE` the bounds of each argument, its `EXTENDED_TYPE` the names of its arguments. The `DESCRIPTION` of the device node carries the credit and the licence of the sources. A device with no `sources` is not exported (`NoSources`); an address two nodes would take is named in the losses; a capture of a command's address (`{n}`) stays `{n}` in its segment, as the descriptor 0.4 wrote it and as the service's export does, so `/track/{n}/volume` is a method under the containers `track` and `{n}`; a command whose address a node already holds, or whose path has a method on it, is named in the losses; no value the descriptor does not give is invented. The `values` of an argument are the `VALS` of its `RANGE`, the list the proposal says: ascending integers for an integer argument, labels for a text argument; the labels of an integer argument, the integers of a text argument have no place in it and are in the losses. A controller entry with no `range` is written with the bounds MIDI fixes for its kind: 0 to 127 for a 7-bit controller, 0 to 16383 for a 14-bit one, an NRPN or an RPN; an entry of no such kind is written with no bound. An entry with no way (its message is to be completed), of `cc_params` or `params`, is written with the entry's `range` as its `RANGE` when it has one, and each of its `scale`, `transmit`, `recognize` and `nrpn_lsb_range` is named in the losses.

A trigger is a method with `ACCESS` 2 whose `FULL_PATH` is the device's prefix followed by the trigger's `osc`, under the containers of that address, and whose `TYPE` is `I`, the Impulse of OSC 1.1, an event with no value (OSCQueryProposal, « Data Types and UI Item conventions », `N,I`: Momentary Button). It has no `RANGE`, `EXTENDED_TYPE`, `OVERLOADS` or `DESCRIPTION`. No velocity, note or channel argument is written: the descriptor has no field for the arguments of a trigger (osd-formats/docs/CADRE.md R9). The proposal makes a node a method or a container, never both. Controls, parameters and commands are placed first, then the triggers in the order written: a trigger whose address a node already holds (a method, or a container of other addresses), or whose path has a method on it, is not written and is a loss at `/triggers/entries/N` saying the address is taken. Each field of a trigger the method does not carry (`title`, `description`, `sound`, `section`, `channel`, `channel_mode`, `note_range`, `note`, `velocity_usage`, `comments`) is, when present, a loss at `/triggers/entries/N/<field>`, and so is each `_` key of an entry, and of the block at `/triggers/<key>`. For a trigger with no `note` whose `note_range` holds more than one note, the loss at `note_range` says that the method does not say which note of the range to play. The import reads the method of a trigger back as a command, not as a trigger.

## What the import reads

The JSON is read as a namespace by the proposal (`InvalidFile`, at its JSON pointer, when it is not one). The unique container at the head is the prefix and the device's name; a method with `ACCESS` 2 or 3 becomes a command, its arguments typed by its tags, named by its `EXTENDED_TYPE` and bounded by its `RANGE`; a container below the device's that holds no method giving a command, at any depth, is not written by the export and is lost at its pointer, or at that of a container that holds it; a segment, or a part of a segment, of the form `{name}` (letters, digits and `_`) is a capture, so the command's address is an array of text and capture segments; a segment with braces that is no capture (`{a,b}`, `{1x}`, two captures side by side) is not imported and the losses name its place; a read-only method (`ACCESS` 1) sends nothing and is lost. A writable method whose `TYPE` is exactly `I`, the method of a trigger among them, becomes a command at its address with no `args` and no `frame`, and no `triggers` block is made: the losses name at its `TYPE` that the impulse tag is not kept and that the export of that command writes no `TYPE`, and name at its place each `RANGE` entry that holds a key and each `EXTENDED_TYPE` entry, a `RANGE` object that holds a key or an `EXTENDED_TYPE` text written alone at its attribute. A `TYPE` holding `N`, or `I` beside other tags, has no argument type: the method is not imported. A `VALS` is named in the losses at its pointer with its reason and no `values` is read from it: the descriptor's `values` are labels with their integers, and a list has only one of the two (a `VALS` that is not a list is named whole); the round trip of the named values holds for MIDNAM alone, OSCQuery gives back the bounds of the range and loses the labels. The source of the descriptor is the one given to the import: the `DESCRIPTION` is never copied as a credit or a licence. The date of the import the caller passes in that source (`imported_at`) is kept as the note `_imported_at` of the source added, its value unchanged, 0.7 having no field for it: a `_imported_at` the source already holds stays, and the date goes under `_imported_at_2` (then `_imported_at_3`, then the next number not taken). It is no loss, no date is invented, and the other keys of the source are kept.

## The arrays of `RANGE` and `EXTENDED_TYPE`

The proposal ("Other notes on attributes") makes both follow the `TYPE`. The import reads each in three forms:

- one entry per tag, the form the export writes;
- arrays nested as the brackets of the `TYPE` (`i[ii]i`: an entry, an array of two, an entry), each argument taking the entry of its place. An array that does not follow the brackets is refused (`InvalidFile`) at its pointer, and the rules of an entry (null or an object, `MIN` and `MAX` numbers, `MIN` not above `MAX`) hold at every depth. The losses of an entry are at its pointer in the file. The export writes one entry per tag: each bound or name read from a place it does not write it back at is lost at its pointer. So is a `null` of `RANGE` it does not write back at its place (it writes `null` for an argument without bounds, and no `RANGE` when no argument has any), and an entry of `EXTENDED_TYPE` that is no text, the export writing a name for each argument;
- a value written alone, an object for `RANGE`, a text for `EXTENDED_TYPE`: it holds for each argument of the `TYPE`. It is equal to the list that repeats it for each argument: when the export writes it so, it is carried and no loss covers it. When an argument does not take it (a name already taken or that is no name, bounds that are not whole or lack one of the two), or when the `TYPE` has no tag, it is lost at its pointer. The `VALS` of a `RANGE` written alone is lost at `RANGE/VALS`, which covers neither `MIN` nor `MAX`.

A value written alone of another type, or an array of another length, is refused at its pointer. A `TYPE` of no tag gives a command without argument and is lost, the export writing no `TYPE`. A method whose `TYPE` is exactly `I` follows its own rule, above.

## Attributes of a node and keys of a range

The fate is `carried` when the export writes it and the import reads it back, `lost` otherwise.

| Attribute | Fate | Reason | Piece |
|---|---|---|---|
| `FULL_PATH` | carried | the address of the node | OSCQueryProposal, "Attributes": FULL_PATH |
| `CONTENTS` | carried | the containers below an address | OSCQueryProposal, "Attributes": CONTENTS |
| `TYPE` | carried | the tags of the arguments of a method; the arrays of the type are lost, and so is a tag the export writes as another (`h` as `i`, `d` as `f`, `S` and `c` as `s`, `F` as `T`), named at the `TYPE` of the method; a `TYPE` of exactly `I` (the method of a trigger) is read as a command with no argument, the tag not kept and named at the `TYPE` | OSCQueryProposal, "Attributes": TYPE and "Type tags" |
| `ACCESS` | carried | 2 for a command and for a trigger, 3 for a control; 1 is lost on import, nothing is read of 0 | OSCQueryProposal, "Attributes": ACCESS |
| `RANGE` | carried | the bounds of each argument, one entry per tag, in the arrays of the type or written alone; an entry the export writes back at another place is lost | OSCQueryProposal, "Attributes": RANGE |
| `DESCRIPTION` | lost | the device node carries the credit of the sources on export; no other text is held | OSCQueryProposal, "Attributes": DESCRIPTION |
| `TAGS` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": TAGS |
| `EXTENDED_TYPE` | carried | the names of the arguments of a method, one per tag, in the arrays of the type or one text for all; a name the export writes back at another place, or that an argument does not take, is lost | OSCQueryProposal, "Attributes": EXTENDED_TYPE |
| `UNIT` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": UNIT |
| `CRITICAL` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": CRITICAL |
| `CLIPMODE` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": CLIPMODE |
| `VALUE` | lost | the namespace holds the current value, the descriptor a default: none is invented | OSCQueryProposal, "Attributes": VALUE |
| `OVERLOADS` | lost | the descriptor has no field for it | OSCQueryProposal, "Attributes": OVERLOADS |
| `MIN` | carried | the lowest value of an argument | OSCQueryProposal, "Attributes": RANGE, MIN |
| `MAX` | carried | the highest value of an argument | OSCQueryProposal, "Attributes": RANGE, MAX |
| `VALS` | lost | the export writes the list of permitted values (the integers of an integer argument, the labels of a text argument) and names in the losses what the list leaves out, the labels or the integers; the import reads no `values` from it, a list having no label or no integer, and names it | OSCQueryProposal, "Attributes": RANGE, VALS |

## Differences from the service

The service of SyntHub is not readable from this repository: no pair of its outputs is frozen here. The equality with its exporter is proved where it is replaced by this package (osc-bridge, OB-20q.7), and each difference found there enters this section with its reason.

- The service's exporter wrote velocity, note and channel arguments for a trigger, with `OVERLOADS`; the export writes a method of type `I` with no argument. Reason: the descriptor has no field for the arguments of a trigger, and the export invents none (osd-formats/docs/CADRE.md R9).

## Round trip on public files

The files are the captures of a running OSCQuery program that gluon/oscq-inspector keeps in `tests/fixtures/` at the revision below, as its README lists them under "Real server captures", each namespace once (`max-hostinfo.json` is the same bytes as `max-root.json`, and `helper-hostinfo.json` is no namespace), that the import reads: Max 9 of Cycling '74; and [`docs/example-http.json`](https://raw.githubusercontent.com/Cycling74/rnbo.oscquery.runner/09326e9f3acd1a54a8b9ea39c1b9314eb6d2f2f8/docs/example-http.json) of Cycling '74's runner (MIT). The rule reads where the files come from and whether they are read, not their figures. One namespace file is refused and has no figure: [`helper-root.json`](https://raw.githubusercontent.com/gluon/oscq-inspector/5c79da2c3d0e5eac6613ff000e953d2ec0482501/tests/fixtures/helper-root.json), the capture of the Vidvox OSCQuery Helper (`InvalidFile` at `/CONTENTS`: it holds three containers at its root, where the namespace of one device has one). The files are not kept in this repository. The test `partner_figures` replays the count on the files of the folder `OSD_PARTNER_FILES` names, as `<folder>/oscquery/<last segment of the address, percent-decoded>`, and says it is skipped without it. Under `npm test`, a result is replayed from the cache while the value of the variable and the sources stay the same, and a file changed under the same path is not seen: the figures are proved by `OSD_PARTNER_FILES=<folder> npm run test:forcer -- osd-formats/tests/it/partner_figures.rs`.

A file is imported, then the device is exported in the same format. A value is each leaf of the JSON, under its attribute path inside its node, with list indices written N, counted once under its kind. `structure`: a kind this page classes as structure the import follows. `lost`: a loss of the import covers the value or a block that holds it, even when the export writes an equal one on its own, or the exported file does not hold it equal on the paired element. `carried`: every other. Equal is as the format reads it: a number written as text equals that number, an omitted attribute equals the default the format's document gives. The three columns of a kind add up to the number of its values in the files.

A value belongs to its node, whose key is its address. Elements of one key are paired in the order of the files, and an element left without a pair loses all its values.

| Address | Revision | SHA-256 | Licence | Credit |
|---|---|---|---|---|
| <https://raw.githubusercontent.com/gluon/oscq-inspector/5c79da2c3d0e5eac6613ff000e953d2ec0482501/tests/fixtures/max-root.json> | 5c79da2c3d0e5eac6613ff000e953d2ec0482501 | df3cf2d12655baafc878eed0bca1bb5b01b9c61468e5d7326ad3f35634f875cb | GPL-3.0-or-later | Julien Bayle, gluon/oscq-inspector |
| <https://raw.githubusercontent.com/Cycling74/rnbo.oscquery.runner/09326e9f3acd1a54a8b9ea39c1b9314eb6d2f2f8/docs/example-http.json> | 09326e9f3acd1a54a8b9ea39c1b9314eb6d2f2f8 | 39c9122f34885ab3f88d67e9a35f6c8a29c25cc6ae347b7330f0ad806e57a13c | MIT | Cycling '74 |

| Kind | Carried | Lost | Structure |
|---|---|---|---|
| `/ACCESS` | 6 | 59 | 0 |
| `/CLIPMODE` | 0 | 35 | 0 |
| `/DESCRIPTION` | 0 | 21 | 0 |
| `/EXTENDED_TYPE` | 0 | 11 | 0 |
| `/FULL_PATH` | 44 | 48 | 0 |
| `/RANGE/N/MAX` | 3 | 0 | 0 |
| `/RANGE/N/MIN` | 3 | 1 | 0 |
| `/RANGE/N/VALS/N` | 0 | 18 | 0 |
| `/TYPE` | 14 | 51 | 0 |
| `/VALUE` | 0 | 24 | 0 |
| `/VALUE/N` | 0 | 35 | 0 |
| **Total** | 70 | 303 | 0 |
