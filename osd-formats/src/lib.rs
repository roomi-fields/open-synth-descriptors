//! The crate osd-formats: its interface, docs/INTERFACE.md, says what it offers.
//!
//! Each format has one importer and one exporter around the `osd` types; each returns its
//! value with the list of what it could not carry. Nothing here reads a file, writes one or
//! opens a port.

mod csv;
mod midi_csv;

use std::fmt;

/// The formats read or written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// The pencilresearch/midi CSV (`template.csv`).
    MidiCsv,
}

impl Format {
    /// Every format, in the order the interface lists them.
    pub const ALL: [Format; 1] = [Format::MidiCsv];

    /// The name the command and the documents give it.
    pub fn name(self) -> &'static str {
        match self {
            Format::MidiCsv => "midi-csv",
        }
    }

    /// The format a name designates.
    pub fn from_name(name: &str) -> Option<Format> {
        Format::ALL.into_iter().find(|f| f.name() == name)
    }
}

/// What a translation returns: its value and what it could not carry.
#[derive(Clone, Debug, PartialEq)]
pub struct Translated<T> {
    pub value: T,
    pub lost: Vec<Loss>,
}

/// One element not carried: its place (`line N, column c` in a source file, a JSON pointer in
/// a descriptor) and the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loss {
    pub place: String,
    pub reason: String,
}

/// Why a translation is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    /// The source given to an import has no `url`, `author` or `attribution`, or no `license`.
    SourceIncomplete,
    /// The text is not valid in its format.
    InvalidFile,
    /// The device to export has no `_sources` entry.
    NoSources,
}

/// A typed refusal: its kind, the place in the source and the rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub kind: FaultKind,
    pub place: String,
    pub rule: String,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {}: {}", self.kind, self.place, self.rule)
    }
}

impl std::error::Error for Fault {}

/// Reads `text`, a file of `format`, into an OSD device carrying `source` as its credit.
pub fn import(text: &str, format: Format, source: &osd::Source) -> Result<Translated<osd::Device>, Fault> {
    match format {
        Format::MidiCsv => midi_csv::import(text, source),
    }
}

/// Writes `device` in `format`.
pub fn export(device: &osd::Resolved, format: Format) -> Result<Translated<String>, Fault> {
    match format {
        Format::MidiCsv => midi_csv::export(device),
    }
}
