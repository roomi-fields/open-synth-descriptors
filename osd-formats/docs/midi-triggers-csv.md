# The pencilresearch/midi triggers CSV

The columns of `template.triggers.csv` of github.com/pencilresearch/midi (commit 215c64e8d3b3, licence CC-BY-SA-4.0), and the fate of each in `osd-formats`. The importer turns a row with a note range (both bounds, 0 to 127, the minimum not above the maximum) into one entry of `triggers.entries`; any other row is lost by its line, with its reason; a file that gives no entry gives a device without a `triggers` block. The exporter writes one row per entry. `osc` is no column: the importer builds it from section and name (the rule of the controller CSV, with the suffixes `-2`, `-3` for a repeat), the exporter does not write it. The importer produces a 0.7 descriptor.

| Column | Fate | Reason or note |
|---|---|---|
| `manufacturer` | `device.vendor` | the first row's; a different value on a later row is lost |
| `device` | `device.name` | the first row's; a different value on a later row is lost |
| `section` | `section` | carried as it is |
| `sound_name` | `sound` | the address `osc` is built from section and name |
| `sound_description` | `description` | carried as it is |
| `midi_channel` | `channel` | an integer from 1 to 16, held as 0 to 15; any other cell (omni, any…) is lost, never a `channel_mode` |
| `note_number_min_value` | `note_range`, first bound | both bounds are needed, else the row is lost |
| `note_number_max_value` | `note_range`, second bound | both bounds are needed, else the row is lost |
| `note_number_default_value` | `note` | lost when it is outside the range |
| `comments` | `comments` | carried as it is |
| `velocity_usage` | `velocity_usage` | carried as it is |

The date of the import the caller passes in its source (`imported_at`) is kept as the note `_imported_at` of the source added, its value unchanged, 0.7 having no field for it: a `_imported_at` the source already holds stays, and the date goes under `_imported_at_2` (then `_imported_at_3`, then the next number not taken). It is no loss, no date is invented, and the other keys of the source are kept.

The export imports the file it writes and names in the losses every field of the device given that comes back missing, changed or added, at its JSON pointer or at a block that holds it, in the order of the descriptor given; a field that comes back equal is not named. So are lost: `channel_mode`, the `title` and the other remarks of an entry, the identity beyond vendor and name, every block but `triggers`, and an `osc` or an `osc_prefix` that is not the one the import builds. `/sources` is always named, the CSV having no column for the credit or the licence: the caller carries them beside the file. A device with no trigger writes no row, so its `name` and `vendor` are named too.

## Round trip on public files

The files are every `*.triggers.csv` of the repository pencilresearch/midi at the revision below, its template apart. The rule reads the names alone, not the figures. The files are not kept in this repository. The test `partner_figures` replays the count on the files of the folder `OSD_PARTNER_FILES` names, as `<folder>/midi-triggers-csv/<last segment of the address, percent-decoded>`, and says it is skipped without it. Under `npm test`, a result is replayed from the cache while the value of the variable and the sources stay the same, and a file changed under the same path is not seen: the figures are proved by `OSD_PARTNER_FILES=<folder> npm run test:forcer -- osd-formats/tests/it/partner_figures.rs`.

A file is imported, then the device is exported in the same format. A value is each filled cell of a row, under the name of its column, counted once under its kind. `structure`: a kind this page classes as structure the import follows. `lost`: a loss of the import covers the value or a block that holds it, even when the export writes an equal one on its own, or the exported file does not hold it equal on the paired element. `carried`: every other. Equal is as the format reads it: a number written as text equals that number, an omitted attribute equals the default the format's document gives. The three columns of a kind add up to the number of its values in the files.

A value belongs to its row. The key of a row is its note range (`note_number_min_value` and `note_number_max_value`), else its rank among the rows that have none. Elements of one key are paired in the order of the files, and an element left without a pair loses all its values.

| Address | Revision | SHA-256 | Licence | Credit |
|---|---|---|---|---|
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Alesis/SR-16.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 7b8a124c4aabebbcd02b4140c4263d45aa87389337c7fb2c9f9bcb16159f7d58 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Behringer/LM%20DRUM.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 72a7ddbedc11f93c8f59815b32356a97209ae23c968ef1b30b3fe2d5031c8fb4 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Elektron/Analog%20Rytm%20MKII.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 85c81bf6f1b5163c00959e1259a1e935d2b73791a1537925db1581d355927386 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Elektron/Digitakt%20II.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | c6a0c15c433bf655852321b4f62ff854a93d8d151b27b594e932de078f33c31d | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Elektron/Model%20Samples.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | b7b3513632a2638320120ed7e00d1564d80b60067790552a8eb4a91946ac2937 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Elektron/Syntakt.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 831683a410ee054b2d283cbba0d024773c0974b3a92dc06d3d5d25027950fc05 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Jomox/Alpha%20Base.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | a3e7b2432bff6998a182a560ef58f01f16567cf7f917cbca74ad34b4f6995a16 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/KORG/drumlogue.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | b07898fbd5cd1be48979c7222934313f1d8e1d813a6a6cdde455e5c4bc1b2bd5 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/KORG/phase8.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 94e5ae448ab556a93fab0825ebc1b5506ad3589bcaf356824bb4145f61031f06 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/KORG/volca%20drum.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 20d2d0971b451dcbb274f949edcac6b34ea0c78e20b9f645c9f2e0b7b4aad6d6 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/M-VAVE/FM1%20Community.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | c6638119167e44a3b0d9f5df83990a35c2892ab9da1a1897f709b56587fb3148 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/MIDI%20Association/Generic%20MIDI%201.0.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 997244989a7efc12e277443197e570da3c283d432c89f3cb9041b63c22e437f2 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Novation/DrumStation.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 61b3c3e8fcd83f2d834f3cc5de23cbad03779d93a01a80087a2744e6c1d08110 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Roland/T-8.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 56e6c86e56563fc54b2a552d209289fa84b48de2fa5fa41d2e56b082e7199728 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Roland/TR-1000.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | ac04dfdb9d63ace6bf05d52d760c18519110c270b645097fbb07d34a61bb39ae | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Roland/TR-6S.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 087e101e60f7a97ec7bf32e29453ad291cb3bd58a145f62d763c6c5b09cb5965 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Roland/TR-8S.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 8bb40471b6885d5fc16e1e640ff655fd3d838577f935bbd520ddcb82b3d44f33 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Sleepy%20Circuits/Hypno.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | aa0c4afd79ae49523a26b71afc36b6e184cd39df1c4ed0f3fef2f234febccbf4 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Teenage%20Engineering/EP-133%20K.O.%20II.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | ed9df223ab901e187ca52ab21fd71093eedba206fe963dd6b89cc863b8212705 | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |
| <https://raw.githubusercontent.com/pencilresearch/midi/215c64e8d3b33e0b248f62f75531b409a002ea2f/Vermona/DRM1%20MKIV.triggers.csv> | 215c64e8d3b33e0b248f62f75531b409a002ea2f | 02f8622be9070b30c00520fb4d6e5180f8ae1c34de73627dadea4364718fc72c | CC-BY-SA-4.0 | pencilresearch/midi (midi.guide) and its contributors; the portions about a device may belong to its manufacturer |

| Kind | Carried | Lost | Structure |
|---|---|---|---|
| `comments` | 116 | 0 | 0 |
| `device` | 550 | 0 | 0 |
| `manufacturer` | 550 | 0 | 0 |
| `midi_channel` | 272 | 278 | 0 |
| `note_number_default_value` | 475 | 0 | 0 |
| `note_number_max_value` | 550 | 0 | 0 |
| `note_number_min_value` | 550 | 0 | 0 |
| `section` | 550 | 0 | 0 |
| `sound_description` | 84 | 0 | 0 |
| `sound_name` | 550 | 0 | 0 |
| `velocity_usage` | 284 | 0 | 0 |
| **Total** | 4531 | 278 | 0 |
