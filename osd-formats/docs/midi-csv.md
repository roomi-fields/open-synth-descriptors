# The pencilresearch/midi CSV

The columns of `template.csv` of github.com/pencilresearch/midi (commit 215c64e8d3b3, licence CC-BY-SA-4.0), and the fate of each in `osd-formats`. The importer turns a row into one control of `cc_params.entries`: a row with a CC and an NRPN is one control whose flat fields are the CC (the first way) and whose `also` holds the NRPN, since a setting reached by two messages is one control; a row with neither is lost. A message is claimed once: a way whose message (kind, numbers; a 14-bit CC claims its LSB too) an earlier row or the base already holds is lost at its cell (`cc_msb` or `nrpn_msb`), the row keeps its other way, and a row left with none is lost, so the descriptor read back is never refused by the 0.6 reader. The one range a row gives goes to both ways. The control has one default, in the space of its first way's range. With a CC and an NRPN, the CC's default is taken; the NRPN's default alone is taken when the two ranges are equal (the CC stays first), and when they differ it makes the NRPN the first way and the CC the way after it, so the default stays in the NRPN's range; the NRPN's default beside a CC default is lost. The exporter writes one row per control, from its first CC way and its first NRPN way; the range the two ways share is written once, in the CC columns. Any other way (a second CC, a second NRPN, an RPN) is lost at its pointer (`/cc_params/entries/N/also/M`): the round trip OSD, CSV, OSD is not promised for it, while the round trip CSV, OSD, CSV of a row loses nothing. The importer produces a 0.6 descriptor, whose controls carry a `default`. The exporter writes a control's default to the column of its first way: `cc_default_value` when the first way is a CC, or an NRPN whose range equals the CC's (the import puts the CC first then); `nrpn_default_value` when the first way is an NRPN alone or with a CC of another range. A default on a control whose first way is neither (an RPN) while a CC or NRPN comes later, or on a control with neither, is lost at `/cc_params/entries/N/default`, written in no column.

| Column | Fate | Reason or note |
|---|---|---|
| `manufacturer` | `device.vendor` | the first row's; a different value on a later row is lost |
| `device` | `device.name` | the first row's; a different value on a later row is lost |
| `section` | `section` of the control | carried as it is |
| `parameter_name` | remark `_name`; the address `osc` is built from section and name | the entry has no field for a name |
| `parameter_description` | remark `_description` | the entry has no field for a description |
| `cc_msb` | `cc` | 0 to 127 |
| `cc_lsb` | `cc_lsb` | the pair is a 14-bit control |
| `cc_min_value` | `range`, first bound | both bounds are needed, else the lone bound is lost |
| `cc_max_value` | `range`, second bound | both bounds are needed, else the lone bound is lost |
| `cc_default_value` | `default` | the default of a control whose first way is the CC (or an NRPN of the same range); lost when it is outside the control's range |
| `nrpn_msb` | `nrpn_msb` | 0 to 127 |
| `nrpn_lsb` | `nrpn_lsb` | 0 to 127 |
| `nrpn_min_value` | `range`, first bound | on the NRPN way (`also`), or on the control when the row has no CC; lost when the row has no NRPN |
| `nrpn_max_value` | `range`, second bound | on the NRPN way (`also`), or on the control when the row has no CC; lost when the row has no NRPN |
| `nrpn_default_value` | `default` | the default of a control whose first way is the NRPN: alone, or with a CC of a different range (the NRPN is then first); with equal ranges the CC stays first; lost when it is outside the range, beside a CC default, or when the row has no NRPN |
| `orientation` | `orientation` | `0-based` and `centered`; any other word is lost |
| `notes` | remark `_notes` | the entry has no field for notes |
| `usage` | remark `_usage` | carried whole, never cut |

The credit of the sources has no column in the format: the repository carries one licence for all its files (its `LICENSE`), and no file or column per device. The exporter lists `/_sources` in its losses, once, whatever the number of sources, and writes no cell for it; `osd export midi-csv` prints on the error output the credit to carry with the file: for each source its author or url, its licence and its attribution. A device without sources is refused.
