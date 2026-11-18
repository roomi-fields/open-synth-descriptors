//! What the two text formats without a field for a source share: the sentence that carries the
//! credit and the licence of a device's sources, and the words of an address.

use osd::Source;

/// One source as a sentence: who made it, where, under which licence, of which kind.
fn source_text(s: &Source) -> String {
    let mut parts: Vec<String> = [&s.author, &s.attribution, &s.url].into_iter().flatten().filter(|p| !p.trim().is_empty()).cloned().collect();
    if let Some(l) = s.license.as_ref().filter(|l| !l.trim().is_empty()) {
        parts.push(l.clone());
    }
    let mut text = parts.join(", ");
    if !s.kind.is_empty() {
        text.push_str(&format!(" ({})", s.kind));
    }
    text
}

/// The credit and licence of every source, one after the other.
pub(crate) fn credit_text(sources: &[Source]) -> String {
    sources.iter().map(source_text).collect::<Vec<_>>().join("; ")
}

/// The keys of a source that no text carries: its firmware, commit, import date, notes and
/// the keys of its own.
pub(crate) fn uncarried_source_fields(s: &Source) -> Vec<String> {
    let mut out = Vec::new();
    for (name, present) in [("firmware", s.firmware.is_some()), ("commit", s.commit.is_some()), ("imported_at", s.imported_at.is_some()), ("notes", s.notes.is_some())] {
        if present {
            out.push(name.to_string());
        }
    }
    out.extend(s.extra.keys().cloned());
    out
}

/// The words of an address: `/filter/cutoff` is `Filter Cutoff`, `/amp_env/attack` is
/// `Amp Env Attack`.
pub(crate) fn address_words(address: &str) -> String {
    address
        .split(['/', '_', '-', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            cs.next().map(|f| f.to_uppercase().chain(cs).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A name as the segment of an address: lower case, its words joined by underscores.
pub(crate) fn name_segment(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}
