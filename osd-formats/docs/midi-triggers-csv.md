# The pencilresearch/midi triggers CSV

The columns of `template.triggers.csv` of github.com/pencilresearch/midi (commit 215c64e8d3b3, licence CC-BY-SA-4.0), and the fate of each in `osd-formats`. The importer turns a row with a note range (both bounds, 0 to 127, the minimum not above the maximum) into one entry of `triggers.entries`; any other row is lost by its line, with its reason. The exporter writes one row per entry. `osc` is no column: the importer builds it from section and name (the rule of the controller CSV, with the suffixes `-2`, `-3` for a repeat), the exporter does not write it. The importer produces a 0.6 descriptor.

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

What the export cannot carry is lost by its JSON pointer: `channel_mode`, the other remarks of an entry, the identity beyond vendor and name, and every block but `triggers`.
