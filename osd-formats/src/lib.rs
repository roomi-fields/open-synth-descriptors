//! The crate osd-formats: its interface, docs/INTERFACE.md, says what it offers.
//!
//! Each format is one module that fulfils one common contract around the `osd` types, and one line of
//! the list below; each importer and exporter returns its value with the list of what it could not
//! carry. Nothing here reads a file, writes one or opens a port.

mod base;
mod credit;
mod csv;
mod importer;
mod xml;

use std::fmt;

/// An importer: reads a file of its format into a device that carries the source given.
type Import = fn(&str, &osd::Source) -> Result<Translated<osd::Device>, Fault>;

/// What adds a file of its format to a base.
type ImportInto = fn(osd::Device, &str, &osd::Source) -> Result<Translated<osd::Device>, Fault>;

/// What each format module declares as its `CONTRACT`.
pub(crate) struct Contract {
    /// The name the command and the documents give the format.
    pub name: &'static str,
    /// The media type of the file written.
    pub media_type: &'static str,
    /// The usual extension of that file, without its dot.
    pub extension: &'static str,
    /// Reads a file of the format into a device; a format that is exported only refuses here.
    pub import: Import,
    /// Writes a device in the format.
    pub export: fn(&osd::Resolved) -> Result<Translated<String>, Fault>,
    /// Adds a file of the format to a base; `None` for a format that adds nothing to a base.
    pub import_into: Option<ImportInto>,
}

/// The list of formats, one line each, `Variant => module: "documentation",`: declares the modules
/// and builds `Format`, whose every answer comes from the contract of the module.
macro_rules! formats {
    ($($variant:ident => $module:ident: $documentation:literal,)+) => {
        $(mod $module;)+

        /// The formats read or written.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Format {
            $(#[doc = $documentation] $variant,)+
        }

        impl Format {
            /// Every format, in the order the interface lists them.
            pub const ALL: [Format; [$(stringify!($variant)),+].len()] = [$(Format::$variant),+];

            pub(crate) const fn contract(self) -> &'static $crate::Contract {
                match self {
                    $(Format::$variant => &$module::CONTRACT,)+
                }
            }

            /// The name the command and the documents give it.
            pub fn name(self) -> &'static str {
                self.contract().name
            }

            /// The media type of the file written in this format. IANA-registered where one
            /// exists (RFC 8259 JSON, RFC 7303 XML, RFC 4180 CSV); each departure has its reason
            /// in docs/INTERFACE.md.
            pub fn media_type(self) -> &'static str {
                self.contract().media_type
            }

            /// The usual extension of the file written in this format, without its dot. Two
            /// formats may share one.
            pub fn extension(self) -> &'static str {
                self.contract().extension
            }

            /// Whether a file of this format can be added to a descriptor ([`import_into`]).
            ///
            /// [`import_into`]: $crate::import_into
            pub fn imports_into(self) -> bool {
                self.contract().import_into.is_some()
            }

            /// The format a name designates.
            pub fn from_name(name: &str) -> Option<Format> {
                Format::ALL.into_iter().find(|f| f.name() == name)
            }
        }
    };
}

formats! {
    MidiCsv => midi_csv: "The pencilresearch/midi CSV (`template.csv`).",
    MidiTriggersCsv => midi_triggers_csv: "The pencilresearch/midi triggers CSV (`template.triggers.csv`).",
    ElectraOne => electra_one: "The Electra One preset (JSON, versions 2 and 3).",
    Midnam => midnam: "The MIDNAM names document of the MIDI Association (MIDINameDocument 1.0).",
    OscQuery => oscquery: "The OSCQuery namespace (JSON).",
    Midi2Controllers => midi2_controllers: "The controller resources of MIDI-CI Property Exchange (M2-117-UM): ChCtrlList and CtrlMapList (JSON).",
    Deluge => deluge: "The Deluge device definition (XML `midiDevice` with `ccLabels`); exported only.",
    Knobkraft => knobkraft: "The skeleton of a KnobKraft Orm adaptation (Python text); exported only, never run here.",
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

/// Names in `losses`, each at its field, what no way of the control `/cc_params/entries/{n}` says
/// ([`osd::CcEntry::fields_no_way_says`]): of a device before 0.7, a message not fully known and the
/// block beside it. A format is written from the ways and no number is added to complete one.
pub(crate) fn lose_what_no_way_says(e: &osd::CcEntry, n: usize, losses: &mut Vec<Loss>) {
    losses.extend(e.fields_no_way_says().into_iter().map(|field| Loss {
        place: format!("/cc_params/entries/{n}/{field}"),
        reason: "no way of the control says this field (a message not fully known, or the block beside one): it is not written, and no number is added to complete it".to_string(),
    }));
}

/// Why a translation is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    /// The source given to an import has no `url`, `author` or `attribution`, or no `license`.
    SourceIncomplete,
    /// The 0.7 reader of `osd` refuses the source given to an import in the descriptor it would
    /// write.
    SourceRefused,
    /// The text is not valid in its format.
    InvalidFile,
    /// The device to export has no `sources` entry.
    NoSources,
    /// The device to export has no `sound_transfers`.
    NoSoundTransfers,
    /// The base of `import_into` does not convert to 0.7, or, of 0.7, does not read.
    NotConvertible,
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

/// Reads `text`, a file of `format`, into an OSD device carrying `source` as its credit, its
/// `imported_at` written as the note `_imported_at` (the next free `_imported_at_N` when taken).
/// The device is returned only when the reader of `osd` reads it (`InvalidFile` otherwise).
pub fn import(text: &str, format: Format, source: &osd::Source) -> Result<Translated<osd::Device>, Fault> {
    (format.contract().import)(text, &importer::note_date(source))
}

/// The credit and licence to carry with a file whose format has no place for them: one line per
/// source of `device`, or none when `lost` does not list `/sources`.
pub fn credit_to_carry(device: &osd::Resolved, lost: &[Loss]) -> Vec<String> {
    if !lost.iter().any(|l| l.place == "/sources") {
        return Vec::new();
    }
    device.device.sources.as_deref().unwrap_or_default().iter().map(credit::source_text).collect()
}

/// Writes `device` in `format`.
pub fn export(device: &osd::Resolved, format: Format) -> Result<Translated<String>, Fault> {
    (format.contract().export)(device)
}

/// Reads `text`, a file of a format that [`Format::imports_into`] a descriptor, into `base`: the
/// entries come after those `base` holds, its identity stays, and `source` follows its sources,
/// its `imported_at` written as the note `_imported_at` as by [`import`].
/// The descriptor comes out in 0.7: an earlier base is first converted by the chain of `osd`
/// (`convert_to`), and one that does not convert, or a base of 0.7 the reader refuses, is
/// `NotConvertible`. As by [`import`], the descriptor is returned only when the reader of `osd`
/// reads it (`InvalidFile` otherwise).
pub fn import_into(base: osd::Device, text: &str, format: Format, source: &osd::Source) -> Result<Translated<osd::Device>, Fault> {
    let Some(add) = format.contract().import_into else {
        let adding: Vec<&str> = Format::ALL.into_iter().filter(|f| f.imports_into()).map(Format::name).collect();
        return Err(Fault {
            kind: FaultKind::InvalidFile,
            place: "format".to_string(),
            rule: format!("{} cannot be added to a descriptor: only these formats can: {}", format.name(), adding.join(", ")),
        });
    };
    add(base, text, &importer::note_date(source))
}

#[cfg(test)]
mod tests;
