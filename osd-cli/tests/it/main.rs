//! The integration tests of the crate osd-cli, one executable: each test file a module of
//! tests/it/, declared here (`mod <fichier>;`).

#[path = "../common/mod.rs"]
mod common;

mod convert;
mod formats;
mod formats_electra;
mod init;
mod program;
mod validate;
