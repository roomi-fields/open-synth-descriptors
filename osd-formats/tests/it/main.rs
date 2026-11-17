//! The integration tests of the crate osd-formats, one executable: each test file a module of
//! tests/it/, declared here (`mod <fichier>;`).

#[path = "../common/mod.rs"]
mod common;

mod midi_csv_columns;
mod midi_csv_export;
mod midi_csv_import;
mod midi_csv_round_trip;
mod no_io;
