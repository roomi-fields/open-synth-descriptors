//! The integration tests of the crate osd-cli, one executable: each test file a module of
//! tests/it/, declared here (`mod <fichier>;`).

#[path = "../common/mod.rs"]
mod common;

mod conformance_reader;
mod convert;
mod formats;
mod formats_deluge;
mod formats_electra;
mod formats_midi2_controllers;
mod formats_midnam;
mod formats_of_the_list;
mod formats_oscquery;
mod formats_triggers;
mod import_into_conversion;
mod import_source_refused;
mod init;
mod partial_controls;
mod program;
mod providers;
mod resolve;
mod uncredited_sources;
mod unconfirmed_places;
mod validate;
mod ways;
