# The pencilresearch/midi CSV

The columns of `template.csv` of github.com/pencilresearch/midi (commit 215c64e8d3b3, licence CC-BY-SA-4.0), and the fate of each in `osd-formats`. The importer turns a row with a CC into one control of `cc_params.entries`, and an NRPN into another control; a row with neither is lost. The exporter writes one row per entry. The importer produces a 0.6 descriptor, whose controls carry a `default`. The exporter writes a control's default to `cc_default_value` when it has a CC, else to `nrpn_default_value`; a default on a control with neither is lost.

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
| `cc_default_value` | `default` | on the CC control; lost when it is outside the control's range |
| `nrpn_msb` | `nrpn_msb` | 0 to 127 |
| `nrpn_lsb` | `nrpn_lsb` | 0 to 127 |
| `nrpn_min_value` | `range`, first bound | on the NRPN control; lost when the row has no NRPN |
| `nrpn_max_value` | `range`, second bound | on the NRPN control; lost when the row has no NRPN |
| `nrpn_default_value` | `default` | on the NRPN control; lost when it is outside the control's range or the row has no NRPN |
| `orientation` | `orientation` | `0-based` and `centered`; any other word is lost |
| `notes` | remark `_notes` | the entry has no field for notes |
| `usage` | remark `_usage` | carried whole, never cut |
