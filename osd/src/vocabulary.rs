//! The closed vocabularies of 0.5 and the table that maps the makers' words
//! of 0.4 to them. Each closed list is a set of defined terms; a value outside
//! a list is valid only as an extension `x-<registrant>-<term>`, except
//! `identity.firmware.decode`, which has none. The README writes the lists and
//! the tables once.

use crate::{Fault, FaultKind};

/// How the bytes of an identity reply read as a firmware version.
pub(crate) const DECODE: [&str; 6] = ["raw_bytes", "json", "korg_minor_major", "yamaha_tenths", "ascii", "lua"];
/// What a control expects.
pub(crate) const EXPECTS: [&str; 6] = ["continuous", "switch", "momentary", "trigger", "clock", "discrete"];
/// What a program change or a bank selects; the sound kind's list.
pub(crate) const KINDS: [&str; 9] =
    ["sound", "kit", "sequence", "project", "bank", "clip", "performance", "scene", "setlist"];
/// What kind of source a piece of data comes from.
pub(crate) const SOURCE_TYPES: [&str; 14] = [
    "hardware-verified",
    "hardware-observed",
    "software-verified",
    "vendor-doc",
    "vendor-osc-api",
    "vendor-self-described",
    "third-party-osc",
    "community-re",
    "electra-preset",
    "pencilresearch",
    "knobkraft",
    "midnam",
    "contributed",
    "test-fixture",
];

/// The makers' words for what a program change selects, with the term each
/// one merges into.
const SELECTS_SYNONYMS: [(&str, &str); 18] = [
    ("preset", "sound"),
    ("patch", "sound"),
    ("program", "sound"),
    ("voice", "sound"),
    ("tone", "sound"),
    ("instrument", "sound"),
    ("pattern", "sequence"),
    ("song", "sequence"),
    ("drum kit", "kit"),
    ("rhythm set", "kit"),
    ("drum voice", "kit"),
    ("studio set", "performance"),
    ("combination", "performance"),
    ("multi", "performance"),
    ("setup", "performance"),
    ("mixing voice", "performance"),
    ("performance", "performance"),
    ("snapshot", "scene"),
];

/// More makers' words that merge into `setlist`.
const SETLIST_SYNONYMS: [&str; 3] = ["playlist slot", "set list slot", "live set"];

/// The catalogue's words with no term: each converts to `x-osd-<word>`.
const CATALOGUE_EXTENSIONS: [&str; 6] = ["sampler element", "track", "canvas", "session", "synth slot", "drum slot"];

/// The makers' words for a source type, with the term each one merges into.
const SOURCE_SYNONYMS: [(&str, &str); 3] =
    [("vendor", "vendor-doc"), ("agent-contributed", "contributed"), ("hardware-verified-partial", "hardware-verified")];

/// `^x-[a-z0-9]+-[a-z0-9-]+$`.
pub(crate) fn is_extension(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("x-") else { return false };
    let Some((registrant, term)) = rest.split_once('-') else { return false };
    let low = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    !registrant.is_empty() && registrant.chars().all(low) && !term.is_empty() && term.chars().all(|c| low(c) || c == '-')
}

/// An SPDX licence id by its shape (capital first, then letters, digits, `.`,
/// `+`, `-`), `public-domain`, or SPDX's `NOASSERTION` and `NONE`; the registry
/// itself is not read. Other spellings of the special values are refused.
pub(crate) fn is_license(s: &str) -> bool {
    if s == "public-domain" || s == "NOASSERTION" || s == "NONE" {
        return true;
    }
    if s.eq_ignore_ascii_case("public-domain") || s.eq_ignore_ascii_case("none") || s.eq_ignore_ascii_case("noassertion") {
        return false;
    }
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
}


fn fault(kind: FaultKind, pointer: &str, rule: impl Into<String>) -> Fault {
    Fault { kind, file: None, pointer: pointer.to_string(), rule: rule.into() }
}

fn list_or_extension(value: &str, list: &[&str], pointer: &str) -> Result<(), Fault> {
    if list.contains(&value) || is_extension(value) {
        return Ok(());
    }
    Err(fault(
        FaultKind::BadValue,
        pointer,
        format!("one of {} or an extension x-<registrant>-<term>", list.join(", ")),
    ))
}

// ---- 0.5: the values read ---------------------------------------------------

pub(crate) fn check_decode(decode: &str, has_field: bool, has_script: bool, pointer: &str) -> Result<(), Fault> {
    let at = format!("{pointer}/decode");
    if !DECODE.contains(&decode) {
        return Err(fault(FaultKind::BadValue, &at, format!("one of {} (no extension)", DECODE.join(", "))));
    }
    if decode == "json" && !has_field {
        return Err(fault(FaultKind::Missing, &format!("{pointer}/field"), "field is required with decode json"));
    }
    if decode == "lua" && !has_script {
        return Err(fault(FaultKind::Missing, &format!("{pointer}/script"), "script is required with decode lua"));
    }
    Ok(())
}

pub(crate) fn check_expects(value: &str, pointer: &str) -> Result<(), Fault> {
    list_or_extension(value, &EXPECTS, pointer)
}

pub(crate) fn check_selects(value: &str, pointer: &str) -> Result<(), Fault> {
    list_or_extension(value, &KINDS, pointer)
}

pub(crate) fn check_license(license: Option<&str>, pointer: &str) -> Result<(), Fault> {
    match license {
        Some(l) if !is_license(l) => Err(fault(FaultKind::BadValue, pointer, LICENSE_RULE)),
        _ => Ok(()),
    }
}

pub(crate) fn check_source(kind: &str, license: Option<&str>, pointer: &str) -> Result<(), Fault> {
    list_or_extension(kind, &SOURCE_TYPES, &format!("{pointer}/type"))?;
    match license {
        Some(l) if !is_license(l) => Err(fault(
            FaultKind::BadValue,
            &format!("{pointer}/license"),
            LICENSE_RULE,
        )),
        _ => Ok(()),
    }
}

// ---- 0.4 to 0.5: the table --------------------------------------------------

fn not_convertible(pointer: &str, rule: impl Into<String>) -> Fault {
    fault(FaultKind::NotConvertible, pointer, rule)
}

/// The 0.5 term of a 0.4 `decode`: itself, or a fault.
pub(crate) fn convert_decode(decode: &str, has_field: bool, has_script: bool, pointer: &str) -> Result<(), Fault> {
    check_decode(decode, has_field, has_script, pointer)
        .map_err(|f| not_convertible(&f.pointer, format!("0.5 holds {}", f.rule)))
}

/// The 0.5 term of a 0.4 `expects`: itself, or a fault.
pub(crate) fn convert_expects(value: &str, pointer: &str) -> Result<String, Fault> {
    if EXPECTS.contains(&value) {
        Ok(value.to_string())
    } else {
        Err(not_convertible(pointer, format!("one of {}", EXPECTS.join(", "))))
    }
}

/// The 0.5 term of a 0.4 `selects`: a term, a synonym's term, or a catalogue
/// word as `x-osd-<word>`; any other word, the case included, is a fault.
pub(crate) fn convert_selects(value: &str, pointer: &str) -> Result<String, Fault> {
    if KINDS.contains(&value) {
        return Ok(value.to_string());
    }
    if let Some((_, term)) = SELECTS_SYNONYMS.iter().find(|(w, _)| *w == value) {
        return Ok(term.to_string());
    }
    if SETLIST_SYNONYMS.contains(&value) {
        return Ok("setlist".to_string());
    }
    if CATALOGUE_EXTENSIONS.contains(&value) {
        return Ok(format!("x-osd-{}", value.replace(' ', "-")));
    }
    Err(not_convertible(pointer, "a term, a maker word of the README's table or a catalogue word"))
}

/// The 0.5 term of a 0.4 source type.
pub(crate) fn convert_source_type(value: &str, pointer: &str) -> Result<String, Fault> {
    if SOURCE_TYPES.contains(&value) {
        return Ok(value.to_string());
    }
    if let Some((_, term)) = SOURCE_SYNONYMS.iter().find(|(w, _)| *w == value) {
        return Ok(term.to_string());
    }
    Err(not_convertible(pointer, "a source type of the README's list or table"))
}

/// The 0.5 licence of a 0.4 one: `none` becomes `NONE`, another admitted value
/// is itself, anything else is a fault.
pub(crate) fn convert_license(value: &str, pointer: &str) -> Result<String, Fault> {
    if value == "none" {
        Ok("NONE".to_string())
    } else if is_license(value) {
        Ok(value.to_string())
    } else {
        Err(not_convertible(pointer, LICENSE_RULE))
    }
}

/// What a licence is, in words.
pub(crate) const LICENSE_RULE: &str = "an SPDX licence id, public-domain, NOASSERTION or NONE";
