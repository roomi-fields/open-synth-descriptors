//! The section « Round trip on public files » of the page of a format: what it has to say, and the
//! replay of its figures on the partner's files, kept outside the repository, in the folder
//! `OSD_PARTNER_FILES` names, as `<folder>/<format>/<last segment of the address>`.
//!
//! The section holds its prose (the rule by which the files were chosen, what `carried` and
//! `lost` mean, the keys that pair the elements), a table of the files
//! `| Address | Revision | SHA-256 | Licence | Credit |`, then a table
//! `| Kind | Carried | Lost | Structure |` of the figures of all its files together, one row per
//! kind and a last row `Total`. A format that is exported only has no figures: its page says
//! « no round trip », there being no import. The MIDI 2.0 page may instead say « No public file »
//! and what was searched.

#![allow(dead_code)]

use super::round_trip::{count, Count, Figures};
use osd_formats::Format;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

pub const SECTION: &str = "## Round trip on public files";
pub const FILES: [&str; 5] = ["Address", "Revision", "SHA-256", "Licence", "Credit"];
pub const FIGURES: [&str; 4] = ["Kind", "Carried", "Lost", "Structure"];

/// A file a page lists.
#[derive(Clone, Debug)]
pub struct Listed {
    pub address: String,
    pub revision: String,
    pub sha256: String,
    pub licence: String,
    pub credit: String,
}

impl Listed {
    /// The name the file has in its folder: the last segment of its address, percent-decoded.
    pub fn name(&self) -> String {
        let last = self.address.trim_end_matches('/').rsplit('/').next().unwrap_or("");
        let last = last.split(['?', '#']).next().unwrap_or("");
        let bytes = last.as_bytes();
        let (mut out, mut i) = (Vec::new(), 0);
        while i < bytes.len() {
            let code = (bytes[i] == b'%').then(|| last.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())).flatten();
            match code {
                Some(b) => {
                    out.push(b);
                    i += 3;
                }
                None => {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    }
}

/// The lines of the section, up to the next section of the same level.
pub fn section(page: &str) -> Option<Vec<&str>> {
    let mut lines = page.lines().skip_while(|l| l.trim() != SECTION);
    lines.next()?;
    Some(lines.take_while(|l| !l.starts_with("## ")).collect())
}

/// The lines of a page outside this section: where the table of destiny of its format stands.
pub fn outside_section(page: &str) -> Vec<&str> {
    let mut within = false;
    page.lines()
        .filter(|l| {
            if l.starts_with("## ") {
                within = l.trim() == SECTION;
            }
            !within
        })
        .collect()
}

/// A cell without its backticks, its emphasis and its angle brackets.
fn cell(c: &str) -> String {
    c.trim().trim_matches('*').trim_matches('`').trim_matches(['<', '>']).to_string()
}

/// The rows of the table of the section whose header has these columns.
fn table(lines: &[&str], columns: &[&str]) -> Option<Vec<Vec<String>>> {
    let cells = |l: &str| -> Vec<String> { l.trim().trim_matches('|').split('|').map(cell).collect() };
    let mut lines = lines.iter().map(|l| l.trim());
    lines.by_ref().find(|l| l.starts_with('|') && cells(l) == columns)?;
    Some(lines.take_while(|l| l.starts_with('|')).filter(|l| !l.replace(['|', '-', ':', ' '], "").is_empty()).map(cells).collect())
}

pub fn listed(page: &str) -> Vec<Listed> {
    let Some(lines) = section(page) else { return Vec::new() };
    table(&lines, &FILES)
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.len() == 5)
        .map(|r| Listed { address: r[0].clone(), revision: r[1].clone(), sha256: r[2].clone(), licence: r[3].clone(), credit: r[4].clone() })
        .collect()
}

/// The figures a page gives: each kind's, and the row of the totals.
pub fn page_figures(page: &str) -> Result<(BTreeMap<String, Figures>, Figures), String> {
    let lines = section(page).ok_or("no section")?;
    let rows = table(&lines, &FIGURES).ok_or("no table of figures | Kind | Carried | Lost | Structure |")?;
    let (mut kinds, mut total) = (BTreeMap::new(), None);
    for r in rows {
        if r.len() != 4 {
            return Err(format!("a row of the figures has four cells: {r:?}"));
        }
        let n = |i: usize| r[i].parse::<usize>().map_err(|_| format!("{}: {:?} is not a number", r[0], r[i]));
        let f = Figures::of(n(1)?, n(2)?, n(3)?);
        if r[0] == "Total" {
            total = Some(f);
        } else if kinds.insert(r[0].clone(), f).is_some() {
            return Err(format!("the kind {} has two rows", r[0]));
        }
    }
    let total = total.ok_or("the figures have no row Total")?;
    if kinds.is_empty() {
        return Err("the figures have no row of a kind".to_string());
    }
    Ok((kinds, total))
}

fn is_hex(s: &str, lengths: std::ops::RangeInclusive<usize>) -> bool {
    lengths.contains(&s.len()) && s.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// Whether a format is read, and so has a round trip.
pub fn importable(format: Format) -> bool {
    !matches!(format, Format::Deluge | Format::Knobkraft)
}

/// Why the page of a format is refused.
pub fn page_faults(format: Format, page: &str) -> Vec<String> {
    let name = format.name();
    let lower = page.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    if !importable(format) {
        let said = lower.contains("no round trip") && lower.contains("import");
        return if said { vec![] } else { vec![format!("{name}: the page does not say there is no round trip, the format not being imported")] };
    }
    let Some(lines) = section(page) else { return vec![format!("{name}: no section « {SECTION} »")] };
    let mut out = Vec::new();
    let prose = lines.iter().filter(|l| !l.trim_start().starts_with('|') && !l.starts_with('#')).copied().collect::<Vec<_>>().join(" ");
    for word in ["`carried`", "`lost`", "key"] {
        if !prose.contains(word) {
            out.push(format!("{name}: the section does not say {word}"));
        }
    }
    let files = listed(page);
    if files.is_empty() {
        let said = prose.to_lowercase();
        if format == Format::Midi2Controllers && said.contains("no public file") && said.contains("searched") {
            return out;
        }
        out.push(format!("{name}: no public file listed in a table | Address | Revision | SHA-256 | Licence | Credit |"));
        return out;
    }
    let mut names = std::collections::BTreeSet::new();
    for f in &files {
        let at = if f.address.is_empty() { "a file" } else { &f.address };
        if !f.address.starts_with("https://") {
            out.push(format!("{name}: {at}: no address"));
        }
        if !is_hex(&f.revision, 7..=40) || !f.address.contains(&f.revision) {
            out.push(format!("{name}: {at}: the address is not pinned at the revision {:?}", f.revision));
        }
        if !is_hex(&f.sha256, 64..=64) {
            out.push(format!("{name}: {at}: no SHA-256"));
        }
        if f.licence.is_empty() {
            out.push(format!("{name}: {at}: no licence"));
        }
        if f.credit.is_empty() {
            out.push(format!("{name}: {at}: no credit"));
        }
        if !names.insert(f.name()) {
            out.push(format!("{name}: two files named {}", f.name()));
        }
    }
    match page_figures(page) {
        Err(why) => out.push(format!("{name}: {why}")),
        Ok((kinds, total)) => {
            let mut sum = Figures::default();
            kinds.values().for_each(|f| sum.add(*f));
            if sum != total {
                out.push(format!("{name}: the totals {total:?} are not the sums of the rows {sum:?}"));
            }
            if total.total() == 0 {
                out.push(format!("{name}: the figures count no value"));
            }
        }
    }
    out
}

/// Why the figures of a page are not those of the files in `<folder>/<format>/`.
pub fn replay(format: Format, folder: &Path, page: &str) -> Vec<String> {
    let name = format.name();
    let dir = folder.join(name);
    let files = listed(page);
    let mut present: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .map(|e| e.expect("a folder entry").file_name().to_string_lossy().into_owned())
        .collect();
    present.sort();
    let mut out: Vec<String> = present
        .iter()
        .filter(|p| !files.iter().any(|f| f.name() == **p))
        .map(|p| format!("{name}: the page does not list the file {p}"))
        .collect();
    let mut counted = Count::default();
    for f in &files {
        let path = dir.join(f.name());
        let Ok(bytes) = std::fs::read(&path) else {
            out.push(format!("{name}: the listed file {} is absent", path.display()));
            continue;
        };
        let sha: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        if sha != f.sha256 {
            out.push(format!("{name}: {}: its SHA-256 is {sha}, the page says {}", f.name(), f.sha256));
            continue;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            out.push(format!("{name}: {}: not UTF-8", f.name()));
            continue;
        };
        counted.add(count(format, &text));
    }
    if !out.is_empty() || files.is_empty() {
        return out;
    }
    out.extend(counted.unnamed.iter().map(|u| format!("{name}: lost and named by no loss: {u}")));
    match page_figures(page) {
        Err(why) => out.push(format!("{name}: {why}")),
        Ok((kinds, total)) => {
            if kinds != counted.kinds || total != counted.total() {
                out.push(format!("{name}: the figures of the page differ from the count, which is:\n{}", counted.markdown()));
            }
        }
    }
    out
}
