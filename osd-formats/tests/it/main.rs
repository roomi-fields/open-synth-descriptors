//! The integration tests of the crate osd-formats, one executable: each test file a module of
//! tests/it/, declared here (`mod <fichier>;`).

#[path = "../common/mod.rs"]
mod common;

mod electra_export;
mod electra_import;
mod electra_round_trip;
mod electra_table;
mod format_names;
mod midi_csv_columns;
mod midi_csv_export;
mod midi_csv_import;
mod midi_csv_round_trip;
mod midnam_export;
mod midnam_import;
mod midnam_round_trip;
mod midnam_table;
mod no_io;
mod oscquery_export;
mod oscquery_import;
mod oscquery_round_trip;
mod oscquery_table;
