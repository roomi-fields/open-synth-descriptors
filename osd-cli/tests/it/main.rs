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
mod formats_oscquery;
mod formats_triggers;
mod init;
mod program;
mod resolve;
mod validate;
