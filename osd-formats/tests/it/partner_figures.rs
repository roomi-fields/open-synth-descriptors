//! The figures each page publishes on the public files of its partner
//! (docs/<format>.md, « Round trip on public files »). The files stay out of the repository: the
//! replay reads them in the folder `OSD_PARTNER_FILES` names, and without it says it is skipped.

use crate::common::fields::page;
use crate::common::partner::{importable, listed, page_faults, replay, section, SECTION};
use osd_formats::Format;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

/// The folder the value `named` of `OSD_PARTNER_FILES` gives to the test `test`; without one,
/// `out` is told that this test is skipped.
fn folder_named(test: &str, named: Option<OsString>, out: &mut dyn Write) -> Option<PathBuf> {
    if named.is_none() {
        writeln!(out, "{test}: skipped, OSD_PARTNER_FILES is not set").unwrap();
    }
    named.map(PathBuf::from)
}

fn replayed(format: Format) {
    let test = std::thread::current().name().expect("a test runs on a named thread").to_string();
    // The skip line is written to stderr itself, so it shows on a green run too.
    let Some(folder) = folder_named(&test, std::env::var_os("OSD_PARTNER_FILES"), &mut std::io::stderr()) else { return };
    let faults = replay(format, &folder, &page(format.name()));
    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

#[test]
fn the_figures_of_the_midi_csv_page_are_those_counted_on_its_public_files() {
    replayed(Format::MidiCsv);
}

#[test]
fn the_figures_of_the_triggers_csv_page_are_those_counted_on_its_public_files() {
    replayed(Format::MidiTriggersCsv);
}

#[test]
fn the_figures_of_the_electra_one_page_are_those_counted_on_its_public_files() {
    replayed(Format::ElectraOne);
}

#[test]
fn the_figures_of_the_midnam_page_are_those_counted_on_its_public_files() {
    replayed(Format::Midnam);
}

#[test]
fn the_figures_of_the_oscquery_page_are_those_counted_on_its_public_files() {
    replayed(Format::OscQuery);
}

#[test]
fn the_figures_of_the_midi2_controllers_page_are_those_counted_on_its_public_files() {
    replayed(Format::Midi2Controllers);
}

#[test]
fn a_replay_without_the_folder_of_partner_files_is_told_skipped_by_name() {
    let mut out = Vec::new();
    assert!(folder_named("partner_figures::a_replay", None, &mut out).is_none());
    assert_eq!(String::from_utf8(out).unwrap(), "partner_figures::a_replay: skipped, OSD_PARTNER_FILES is not set\n");
}

// ---- the pages --------------------------------------------------------------------------------

fn faults_of(format: Format) {
    let faults = page_faults(format, &page(format.name()));
    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

#[test]
fn the_midi_csv_page_publishes_the_round_trip_figures_of_public_files() {
    faults_of(Format::MidiCsv);
}

#[test]
fn the_triggers_csv_page_publishes_the_round_trip_figures_of_public_files() {
    faults_of(Format::MidiTriggersCsv);
}

#[test]
fn the_electra_one_page_publishes_the_round_trip_figures_of_public_files() {
    faults_of(Format::ElectraOne);
}

#[test]
fn the_midnam_page_publishes_the_round_trip_figures_of_public_files() {
    faults_of(Format::Midnam);
}

#[test]
fn the_oscquery_page_publishes_the_round_trip_figures_of_public_files() {
    faults_of(Format::OscQuery);
}

#[test]
fn the_midi2_controllers_page_publishes_its_round_trip_figures_or_says_no_public_file_was_found() {
    faults_of(Format::Midi2Controllers);
}

#[test]
fn the_pages_of_the_formats_that_are_not_imported_say_there_is_no_round_trip() {
    let faults: Vec<String> = Format::ALL.into_iter().filter(|f| !importable(*f)).flat_map(|f| page_faults(f, &page(f.name()))).collect();
    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

#[test]
fn the_oscquery_page_lists_the_namespace_of_the_rnbo_runner_and_says_why_the_vidvox_capture_is_refused() {
    const REVISION: &str = "09326e9f3acd1a54a8b9ea39c1b9314eb6d2f2f8";
    let page = page(Format::OscQuery.name());
    let files = listed(&page);
    let runner = files.iter().find(|f| f.name() == "example-http.json").unwrap_or_else(|| panic!("docs/oscquery.md does not list example-http.json: {files:?}"));
    assert!(runner.address.contains("Cycling74/rnbo.oscquery.runner") && runner.address.ends_with("/docs/example-http.json"), "{runner:?}");
    assert_eq!(runner.revision, REVISION);
    assert_eq!(runner.sha256, "39c9122f34885ab3f88d67e9a35f6c8a29c25cc6ae347b7330f0ad806e57a13c");
    assert!(runner.licence.contains("MIT") && runner.credit.contains("Cycling '74"), "{runner:?}");
    assert!(!files.iter().any(|f| f.name() == "helper-root.json"), "a refused file has no figure: {files:?}");
    let prose = section(&page).expect("the section").join(" ");
    assert!(prose.contains("helper-root.json") && prose.contains("`/CONTENTS`") && !prose.contains("/CONTENTS/tuples"), "helper-root.json is refused at /CONTENTS");
}

// ---- what the checks refuse -------------------------------------------------------------------

const ONE_ROW: &str = "manufacturer,device,section,parameter_name,parameter_description,cc_msb,cc_lsb,cc_min_value,cc_max_value,cc_default_value,nrpn_msb,nrpn_lsb,nrpn_min_value,nrpn_max_value,nrpn_default_value,orientation,notes,usage\nLofty,Trundler,,Cutoff,,74,,,,,,,,,,,,\n";

const REVISION: &str = "215c64e8d3b3215c64e8d3b3215c64e8d3b32151";

fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// A page whose one file is `ONE_ROW`, with its figures.
fn good_page() -> String {
    format!(
        "# A format\n\n{SECTION}\n\nThe first file of the repository. `carried` comes back, `lost` does not; the key of a row is its CC.\n\n| Address | Revision | SHA-256 | Licence | Credit |\n|---|---|---|---|---|\n| https://example.org/{REVISION}/one%20row.csv | {REVISION} | {} | CC-BY-SA-4.0 | Pencilresearch |\n\n| Kind | Carried | Lost | Structure |\n|---|---|---|---|\n| `cc_msb` | 1 | 0 | 0 |\n| `device` | 1 | 0 | 0 |\n| `manufacturer` | 1 | 0 | 0 |\n| `parameter_name` | 1 | 0 | 0 |\n| **Total** | 4 | 0 | 0 |\n\n## After\n",
        sha(ONE_ROW)
    )
}

fn refused(format: Format, page: &str, why: &str) {
    let faults = page_faults(format, page);
    assert!(faults.iter().any(|f| f.contains(why)), "{why}: {faults:?}");
}

#[test]
fn a_complete_page_is_accepted() {
    assert_eq!(page_faults(Format::MidiCsv, &good_page()), Vec::<String>::new());
}

#[test]
fn a_page_without_the_section_is_refused() {
    refused(Format::OscQuery, "# A format\n\nNothing.\n", "no section");
}

#[test]
fn a_page_of_the_five_formats_with_public_files_is_refused_without_a_file_and_its_figures() {
    let none = format!("{SECTION}\n\nNo public file: searched everywhere. `carried` `lost` key\n");
    for format in [Format::MidiCsv, Format::MidiTriggersCsv, Format::ElectraOne, Format::Midnam, Format::OscQuery] {
        refused(format, &none, "no public file listed");
    }
    assert_eq!(page_faults(Format::Midi2Controllers, &none), Vec::<String>::new());
    refused(Format::Midi2Controllers, &format!("{SECTION}\n\n`carried` `lost` key\n"), "no public file listed");
    let no_figures = good_page().split("| Kind |").next().unwrap().to_string();
    refused(Format::MidiCsv, &no_figures, "no table of figures");
}

#[test]
fn a_file_without_its_pinned_address_its_hash_or_its_licence_is_refused() {
    let page = good_page();
    refused(Format::MidiCsv, &page.replace(&format!("example.org/{REVISION}/"), "example.org/main/"), "not pinned");
    refused(Format::MidiCsv, &page.replace(&sha(ONE_ROW), ""), "no SHA-256");
    refused(Format::MidiCsv, &page.replace("CC-BY-SA-4.0", ""), "no licence");
    refused(Format::MidiCsv, &page.replace("Pencilresearch", ""), "no credit");
}

#[test]
fn totals_that_are_not_the_sums_of_the_rows_are_refused() {
    refused(Format::MidiCsv, &good_page().replace("| **Total** | 4 | 0 |", "| **Total** | 3 | 1 |"), "not the sums");
}

fn folder(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("midi-csv")).unwrap();
    for (name, text) in files {
        std::fs::write(dir.path().join("midi-csv").join(name), text).unwrap();
    }
    dir
}

fn replay_refuses(files: &[(&str, &str)], page: &str, why: &str) {
    let dir = folder(files);
    let faults = replay(Format::MidiCsv, dir.path(), page);
    assert!(faults.iter().any(|f| f.contains(why)), "{why}: {faults:?}");
}

#[test]
fn a_replay_accepts_the_figures_counted_on_the_listed_files() {
    let dir = folder(&[("one row.csv", ONE_ROW)]);
    assert_eq!(replay(Format::MidiCsv, dir.path(), &good_page()), Vec::<String>::new());
}

#[test]
fn a_replay_refuses_a_changed_file_an_unlisted_one_and_an_absent_one() {
    let changed = ONE_ROW.replace("74", "75");
    replay_refuses(&[("one row.csv", &changed)], &good_page(), "its SHA-256 is");
    replay_refuses(&[("one row.csv", ONE_ROW), ("other.csv", ONE_ROW)], &good_page(), "does not list the file other.csv");
    replay_refuses(&[], &good_page(), "is absent");
}

#[test]
fn a_replay_refuses_any_figure_that_differs_from_the_count() {
    for (from, to) in [
        ("| `cc_msb` | 1 | 0 | 0 |", "| `cc_msb` | 0 | 1 | 0 |"),
        ("| `cc_msb` | 1 | 0 | 0 |\n", ""),
        ("| **Total** | 4 |", "| **Total** | 5 |"),
    ] {
        replay_refuses(&[("one row.csv", ONE_ROW)], &good_page().replace(from, to), "differ from the count");
    }
}
