# The pencilresearch/midi CSV

The columns of `template.csv` of github.com/pencilresearch/midi (commit 215c64e8d3b3, licence CC-BY-SA-4.0), and the fate of each in `osd-formats`. The importer turns a row into one control of `cc_params.entries`: a row with a CC and an NRPN is one control whose `ways` are the CC (the first way) then the NRPN, since a setting reached by two messages is one control; a row with neither is lost. A message is claimed once: a way whose message (kind, numbers; a 14-bit CC claims its LSB too) an earlier row or the base already holds is lost at its cell (`cc_msb` or `nrpn_msb`), the row keeps its other way, and a row left with none is lost, named at its line (`line N`) before the cells that say why, so an imported control claims no message that a control of an earlier row or of the base already claims, which the 0.7 reader, of the version the import produces, would refuse. The one range a row gives goes to both ways. The control has one default, in the space of its first way's range: when the NRPN is the more precise, the control gets `scale`, the CC's range, so that the default keeps its value. A default is judged on the scale of its way: its range, else 0 to 127 for a CC of one number and 0 to 16383 for a 14-bit pair or an NRPN; outside it, the default is lost at its cell and the rest of the row is carried. With a CC and an NRPN, the CC's default is taken; the NRPN's default alone (a CC default lost counts as absent) is taken with the CC staying first when the two ranges are equal, unless the row gives no range and the default is above 127; when the ranges differ, or in that last case, it makes the NRPN the first way and the CC the way after it, so the default stays in the NRPN's scale; the NRPN's default beside a CC default is lost. A `cc_lsb` equal to its `cc_msb` is no 14-bit pair, which takes two distinct numbers: the way is a CC of that one number, without `value_bits`, claimed once, and the cell `cc_lsb` is lost. The exporter writes one row per control, from its first CC way and its first NRPN way; the range the two ways share is written once, in the CC columns. Since the import gives the one range of a row to both ways, a control with a CC and an NRPN of which one way alone has a range, when the message of the other implies another scale (0 to 127 for a CC of one number or a 7-bit NRPN, 0 to 16383 for a 14-bit pair or a 14-bit NRPN), is written with both scales in the bounds columns: that range, and the implied scale. Imported back, each way keeps its scale, now carried as its `range`, and neither takes the other's. When the implied scale equals the range, the range is written once. The CSV does not say the order of the ways: the import puts the CC first, unless a default in `nrpn_default_value` beside a CC of another range, or above 127 on a row without a range, makes the NRPN first. A control whose first way is an NRPN and whose second is a CC, without that default, comes back CC first, and the export names it as a loss at the control (`/cc_params/entries/N`); with that default the order comes back and no loss is named. When the export itself writes a default in `nrpn_default_value` beside a CC, the row it writes comes back NRPN first, so a control in the order CC then NRPN written that way is named as a loss of the order at the control. Any other way (a second CC, a second NRPN, an RPN, a pressure, a bend, a note) is lost at its pointer (`/cc_params/entries/N/ways/M`): the round trip OSD, CSV, OSD is not promised for it, while the round trip CSV, OSD, CSV of a row loses nothing. The importer produces a 0.7 descriptor, whose controls carry a `default`. The exporter writes a control's default, which is in the reference scale (`scale`, else the range of the most precise way), unchanged, to the column of the way that gives that scale: `cc_default_value` when it is a CC, or an NRPN whose range equals the CC's (the import puts the CC first then); `nrpn_default_value` when it is an NRPN alone, or with a CC of another range, or, without a range on either, with a default above 127. The ranges compared here are those of the row: with one way alone with a range and another scale implied by the other, they differ, and the default goes to the column of the way that gives the reference scale, which the import puts first. A control with an NRPN and a 7-bit CC, neither with a range, whose default of 127 or less is on the scale of the NRPN, is written with the range each way implies in the bounds columns (CC 0 to 127, NRPN 0 to 16383) and the default in `nrpn_default_value`: imported back, the NRPN is the first way, each way keeps its scale, now carried as its `range`, and the default keeps its value on 0 to 16383; in the order CC then NRPN, the lost order is named at the control. A default on a control whose reference way is neither (an RPN) while a CC or NRPN comes later, or on a control with neither, is lost at `/cc_params/entries/N/default`, written in no column.

The CSV has no column for the width of an NRPN value, and the import reads every NRPN on 14 bits (data MSB and LSB). The export names the width of the NRPN way of 7 bits it writes on the row (alone or beside a CC, with or without a range) as a loss at `/cc_params/entries/N/ways/M/value_bits`; when the row writes no bounds for that way (neither its own nor the shared range), the loss also says that its scale becomes 0 to 16383. No range is added for it: on the way of 14 bits the import creates, a range of 0 to 127 would send the data LSB alone. When the row has no CC and the export writes in `nrpn_default_value` the default of such a way without a range, the cell is kept as it is and `/cc_params/entries/N/default` is named too: the import reads this default on the scale 0 to 16383, not 0 to 127.

A column's fate is what the import does with a filled cell: `carried`, it is read; `conditional`, it is read or lost, as the note says.

| Column | Fate | Reason or note |
|---|---|---|
| `manufacturer` | conditional | `device.vendor`: the first row's; a different value on a later row is lost |
| `device` | conditional | `device.name`: the first row's; a different value on a later row is lost |
| `section` | carried | `section` of the control: carried as it is |
| `parameter_name` | carried | `title`; the address `osc` is built from section and name: an empty cell writes no `title`, and the address is never copied into it; an export without a `title` writes the address in the cell |
| `parameter_description` | carried | `description`: carried as it is |
| `cc_msb` | conditional | `ways[]`, `ctrlType: cc`, `ctrlIndex[0]`: 0 to 127 |
| `cc_lsb` | conditional | `ctrlIndex[1]`, with `value_bits: 14`: the pair is a 14-bit control; equal to `cc_msb`, it is lost and the CC is written alone, without `value_bits` |
| `cc_min_value` | conditional | `range`, first bound: both bounds are needed, else the lone bound is lost |
| `cc_max_value` | conditional | `range`, second bound: both bounds are needed, else the lone bound is lost |
| `cc_default_value` | conditional | `default`: the default of a control whose first way is the CC (or an NRPN of the same range); lost when it is outside the control's range, or, without a range, outside 0 to 127 (a CC of one number) or 0 to 16383 (a 14-bit pair) |
| `nrpn_msb` | conditional | `ways[]`, `ctrlType: nrpn`, `ctrlIndex[0]`: 0 to 127; an NRPN needs both its MSB and its LSB, else the row loses it |
| `nrpn_lsb` | carried | `ctrlIndex[1]`: 0 to 127 |
| `nrpn_min_value` | conditional | `range`, first bound: on the NRPN way; lost when the row has no NRPN |
| `nrpn_max_value` | conditional | `range`, second bound: on the NRPN way; lost when the row has no NRPN |
| `nrpn_default_value` | conditional | `default`: the default of a control whose first way is the NRPN: alone, or with a CC of a different range (the NRPN is then first); with equal ranges the CC stays first, unless the row gives no range and the default is above 127; lost when it is outside the range (without a range, 0 to 16383), beside a CC default, or when the row has no NRPN |
| `orientation` | conditional | `polarity`: `0-based` is `unipolar`, `centered` is `bipolar`; any other word is lost |
| `notes` | carried | `notes`: carried as it is |
| `usage` | carried | `value_description`: carried whole, never cut |

## What the export names

The export imports the file it writes and compares the device with the one it was given, each control and way beside the one it became: every field that comes back missing, changed or added is in the losses, at its JSON pointer in the device given or at the pointer of a block that holds it. The losses come in the order of the descriptor given: list items by their index, a block before what lies under it, the fields of an object in the order `osd` writes them. So an address or a `title` the import builds otherwise, a range it adds, and `device.name` and `device.vendor` of a file with no row it takes are named.

A control with no way (its message is to be completed) is written as a row with no CC and no NRPN, which an import takes for no control: the export names it as a loss at the control (`/cc_params/entries/N`), and each of its `scale`, `range`, `transmit`, `recognize` and `nrpn_lsb_range` at its pointer.

The date of the import the caller passes in its source (`imported_at`) is kept as the note `_imported_at` of the source added, its value unchanged, 0.7 having no field for it: a `_imported_at` the source already holds stays, and the date goes under `_imported_at_2` (then `_imported_at_3`, then the next number not taken). It is no loss, no date is invented, and the other keys of the source are kept.

The credit of the sources has no column in the format: the repository carries one licence for all its files (its `LICENSE`), and no file or column per device. The exporter lists `/sources` in its losses, once, whatever the number of sources, and writes no cell for it; `osd export midi-csv` prints on the error output the credit to carry with the file: for each source its author or url, its licence and its attribution. A device without sources is refused.

## The OSD 0.7 device

What a device of 0.7 keeps through an export then an import. `carried`: it comes back equal; `lost`: it is in the losses of the export; `conditional`: either, as its condition says. A block lost whole has one row; `_*` is the `_` notes of an object.

| OSD field | Fate | Reason or condition |
|---|---|---|
| `/$schema` | carried |  |
| `/capabilities` | lost | The CSV holds no script and declares no capability |
| `/role_vocabulary` | lost | no role is written |
| `/midi_of` | lost | the CSV carries controls only |
| `/contexts` | lost | the CSV carries controls only |
| `/loads_sounds_of` | lost | the CSV carries controls only |
| `/sound_transfers` | lost | the CSV carries controls only |
| `/resources` | lost | the CSV carries controls only |
| `/identity` | lost | the CSV carries controls only; `/identity/manufacturerId` is named too |
| `/programs` | lost | the CSV carries controls only |
| `/sysex` | lost | the CSV carries controls only |
| `/params` | lost | the CSV carries controls only |
| `/triggers` | lost | the CSV carries controls only |
| `/midi_out` | lost | the CSV carries controls only |
| `/midi_in` | lost | the CSV carries controls only |
| `/replies` | lost | the CSV carries controls only |
| `/subscriptions` | lost | the CSV carries controls only |
| `/custom_commands` | lost | the CSV carries controls only |
| `/sources` | lost | no field for the credit and the licence |
| `/midi_of_sources` | lost | the CSV carries controls only |
| `/_coverage` | lost | the CSV carries controls only |
| `/_*` | lost | no field for a note |
| `/commands` | lost | the CSV carries controls only |
| `/device` | conditional | carried when each of its properties is |
| `/device/name` | conditional | written in the `device` column of each row: lost from a file without a row an import takes |
| `/device/vendor` | conditional | written in the `manufacturer` column of each row: lost from a file without a row an import takes |
| `/device/osc_prefix` | conditional | no column: carried when it is the prefix the import builds from the name of the device |
| `/device/device_id` | lost | the CSV has no column for it |
| `/device/rate_limit_hz` | lost | the CSV has no column for it |
| `/device/kind` | lost | the CSV has no column for it |
| `/device/stands_for` | lost | the CSV has no column for it |
| `/device/fidelity` | lost | the CSV has no column for it |
| `/device/transport` | lost | the CSV has no column for it |
| `/device/_*` | lost | the CSV has no column for a note |
| `/cc_params` | conditional | carried when its entries are, without a channel or a note |
| `/cc_params/channel` | lost | the CSV has no column for a channel |
| `/cc_params/entries` | conditional | a control with a CC or an NRPN way is a row; one without is dropped by the import |
| `/cc_params/_*` | lost | the CSV has no column for a note |
| `/cc_params/entries/N/osc` | conditional | no column: carried when it is the address the import builds from the section and the name |
| `/cc_params/entries/N/title` | conditional | `parameter_name`; a control without a title is written with its address there, which comes back as a title |
| `/cc_params/entries/N/description` | carried | `parameter_description` |
| `/cc_params/entries/N/value_description` | carried | `usage` |
| `/cc_params/entries/N/notes` | carried | `notes` |
| `/cc_params/entries/N/scale` | conditional | no column: carried when it is the scale the import deduces from a default and ways of different precision |
| `/cc_params/entries/N/range` | lost | on a control without a way: the row holds no message, and the import drops it |
| `/cc_params/entries/N/transmit` | lost | on a control without a way: the row holds no message, and the import drops it |
| `/cc_params/entries/N/recognize` | lost | on a control without a way: the row holds no message, and the import drops it |
| `/cc_params/entries/N/nrpn_lsb_range` | lost | on a control without a way: the row holds no message, and the import drops it |
| `/cc_params/entries/N/default` | conditional | `cc_default_value` or `nrpn_default_value`: lost when it is in the scale of a way that has no column |
| `/cc_params/entries/N/priority` | lost | the CSV has no column for it |
| `/cc_params/entries/N/step_count` | lost | the CSV has no column for it |
| `/cc_params/entries/N/values` | lost | the CSV has no column for named values |
| `/cc_params/entries/N/polarity` | carried | `orientation` |
| `/cc_params/entries/N/channel` | lost | the CSV has no column for a channel |
| `/cc_params/entries/N/section` | carried | `section` |
| `/cc_params/entries/N/expects` | lost | the CSV has no column for it |
| `/cc_params/entries/N/transform` | lost | the CSV holds no script |
| `/cc_params/entries/N/role` | lost | the CSV has no column for it |
| `/cc_params/entries/N/ways` | conditional | the first CC way and the first NRPN way |
| `/cc_params/entries/N/readings` | lost | the CSV has no column for it |
| `/cc_params/entries/N/_*` | lost | the CSV has no column for a note |
| `/cc_params/entries/N/ways/M/ctrlType` | conditional | `cc` and `nrpn` have columns, one way of each |
| `/cc_params/entries/N/ways/M/ctrlIndex` | conditional | `cc_msb`, `cc_lsb`, `nrpn_msb`, `nrpn_lsb`, of the way written |
| `/cc_params/entries/N/ways/M/value_bits` | conditional | no column: the import writes 14 for a CC of two numbers and for an NRPN, none for a CC of one number |
| `/cc_params/entries/N/ways/M/range` | conditional | the bounds columns: one range of a row goes to both ways, so the scale of a way without a range may come back as its range |
| `/cc_params/entries/N/ways/M/transmit` | lost | the CSV has no column for a direction |
| `/cc_params/entries/N/ways/M/recognize` | lost | the CSV has no column for a direction |
| `/cc_params/entries/N/ways/M/nrpn_lsb_range` | lost | the CSV has no column for it |
| `/cc_params/entries/N/ways/M/in` | lost | the CSV has no column for a context |
| `/cc_params/entries/N/ways/M/slice` | lost | the CSV has no column for it |
| `/cc_params/entries/N/ways/M/_*` | lost | the CSV has no column for a note |
| `/sources/N/id` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/type` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/provider` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/url` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/firmware` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/author` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/attribution` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/license` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/commit` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/profile` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| /sources/N/imported_at | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/notes` | lost | The CSV has no field for the credit or the licence of a source: `/sources` is lost, and the command prints the credit to carry |
| `/sources/N/_*` | lost | The CSV has no field for a note |

## Round trip on public files

The files are, in the repository pencilresearch/midi at the revision below, the first CSV in byte order of path (a triggers CSV apart) of each of the first ten manufacturer folders in byte order. The rule reads the names alone, not the figures. The files are not kept in this repository. The test `partner_figures` replays the count on the files of the folder `OSD_PARTNER_FILES` names, as `<folder>/midi-csv/<last segment of the address, percent-decoded>`, and says it is skipped without it. Under `npm test`, a result is replayed from the cache while the value of the variable and the sources stay the same, and a file changed under the same path is not seen: the figures are proved by `OSD_PARTNER_FILES=<folder> npm run test:forcer -- osd-formats/tests/it/partner_figures.rs`.

A file is imported, then the device is exported in the same format. A value is each filled cell of a row, under the name of its column, counted once under its kind. `structure`: a kind this page classes as structure the import follows. `lost`: a loss of the import covers the value or a block that holds it, even when the export writes an equal one on its own, or the exported file does not hold it equal on the paired element. `carried`: every other. Equal is as the format reads it: a number written as text equals that number, an omitted attribute equals the default the format's document gives. The three columns of a kind add up to the number of its values in the files.

A value belongs to its row. The key of a row is its CC (`cc_msb`), else its NRPN (`nrpn_msb` and `nrpn_lsb`), else its rank among the rows that have neither. Elements of one key are paired in the order of the files, and an element left without a pair loses all its values. A row with a CC and an NRPN of one range says that range, and its default, for both.

| Address | Revision | SHA-256 | Licence | Credit |
|---|---|---|---|---|
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/1010music/bento.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 407d6f19bf3f15c25cc2c19612b74d8215ed9436493bf8942b4b3a9af5ceda06 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/ALM%20Busy%20Circuits/mmMidi.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | c27217b9095aa2a9d123d30d9f2db7be917233c5502f6bfabab411dbdf3b2053 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/ASM/Hydrasynth.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 67a2ba5ff2559d2ce1368c3a636ebb4b10e73ddc32abd22a15ff121bcab440c8 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Abildgard/Droid-3.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 464860547d8793508af435d603e2d2ce0741889e446bc69094b1dc2d8b3e1c24 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Access/Virus%20A.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | f7980bb4239b5711f15f00c1da296e120dce25e18a9cd7e907a778e90d2e57ca | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Animal%20Factory%20Amplification/Dirty%20Mirror.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 58c5f990f04b1e3bd9725fa2730636c2505232f2f14cd8bd902dccea1f6c3717 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Antonus/2600%20Original%20Size.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 548562ddf4b34e295fe1c0649a256a0c2547515600d2dec9dae68198d4943bd0 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Arturia/MatrixBrute.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 098604e9daa55ef98d833a11286614e96de887a6c4a4333a777761098d069181 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Audiothingies/Doctor%20A.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | fb4c266784ef677846a9c87895cf6e2096f97d5197dd79a268012cc7f6a9fe23 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/BOSS/GM-800.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 7953e4058e6ecf8ab38d032e82fb906decc17f93b9b8b806849328d1c7094ac8 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |

| Kind | Carried | Lost | Structure |
|---|---|---|---|
| `cc_default_value` | 2 | 0 | 0 |
| `cc_lsb` | 1 | 0 | 0 |
| `cc_max_value` | 545 | 222 | 0 |
| `cc_min_value` | 545 | 222 | 0 |
| `cc_msb` | 545 | 222 | 0 |
| `device` | 545 | 222 | 0 |
| `manufacturer` | 545 | 222 | 0 |
| `notes` | 68 | 4 | 0 |
| `orientation` | 545 | 222 | 0 |
| `parameter_description` | 32 | 0 | 0 |
| `parameter_name` | 545 | 222 | 0 |
| `section` | 525 | 222 | 0 |
| `usage` | 69 | 44 | 0 |
| **Total** | 4512 | 1824 | 0 |
