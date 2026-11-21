//! The integration tests of the crate osd-formats, one executable: each test file a module of
//! tests/it/, declared here (`mod <fichier>;`).

#[path = "../common/mod.rs"]
mod common;

mod budget_device;
mod deluge_export;
mod deluge_table;
mod electra_export;
mod electra_import;
mod electra_playable;
mod electra_round_trip;
mod electra_table;
mod format_media;
mod format_names;
mod import_into;
mod knobkraft_export;
mod knobkraft_licence;
mod knobkraft_table;
mod midi2_controllers_export;
mod midi2_controllers_import;
mod midi2_controllers_round_trip;
mod midi2_controllers_table;
mod standard_citations;
mod midi_csv_columns;
mod midi_csv_export;
mod midi_csv_import;
mod midi_csv_round_trip;
mod midi_triggers_csv_columns;
mod midi_triggers_csv_export;
mod midi_triggers_csv_import;
mod midi_triggers_csv_round_trip;
mod midnam_export;
mod midnam_import;
mod midnam_round_trip;
mod midnam_table;
mod midnam_values;
mod no_io;
mod oscquery_export;
mod oscquery_import;
mod oscquery_round_trip;
mod oscquery_table;
mod oscquery_values;
mod resource_exports;
