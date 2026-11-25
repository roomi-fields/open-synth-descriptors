//! The skeleton of a KnobKraft adaptation states its own licence (AGPL-3.0-only, the licence of
//! KnobKraft Orm and of its adaptations) before the credit of the sources, and warns, never
//! refuses, about a source whose licence is not allowed under the AGPL-3.0 or not recognised.
//! The text is compared line by line, never parsed nor run.

use crate::common::knobkraft::*;
use serde_json::{json, Value};

const SPDX: &str = "# SPDX-License-Identifier: AGPL-3.0-only";
const NOT_ALLOWED: &str = "# Licence not allowed under AGPL-3.0: ";
const NOT_RECOGNISED: &str = "# Licence not recognised, not checked against AGPL-3.0: ";

/// A device whose sources carry the licences as given: the descriptor is read with a licence it
/// accepts, then the licence text is laid over, so that text a reader refuses (free text, a
/// blank) still reaches the export.
fn with_sources(mut sources: Value) -> osd::Resolved {
    let raw: Vec<Option<String>> = sources.as_array().unwrap().iter().map(|s| s["license"].as_str().map(str::to_string)).collect();
    for s in sources.as_array_mut().unwrap() {
        if s.get("license").is_some() {
            s["license"] = json!("CC-BY-4.0");
        }
    }
    let mut d = device(json!({ "sources": sources }));
    for (s, license) in d.device.sources.as_mut().unwrap().iter_mut().zip(raw) {
        s.license = license;
    }
    d
}

fn with_license(license: &str) -> osd::Resolved {
    with_sources(json!([{"type": "community-re", "url": "https://example.org/d", "author": "Dee", "license": license}]))
}

/// The comment lines of the header, up to the first blank line.
fn header(text: &str) -> Vec<String> {
    text.lines().take_while(|l| !l.is_empty()).map(str::to_string).collect()
}

fn read(path: &str) -> String {
    let file = format!("{}/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"))
}

#[test]
fn the_header_is_written_line_by_line_with_the_skeleton_licence_before_the_sources() {
    let (text, _) = exported(&device(json!({})));
    let expected = [
        "# Skeleton of a KnobKraft Orm adaptation, written from an Open Synth Descriptors device.",
        "# KnobKraft Orm, by Christof Ruch, https://github.com/christofmuc/KnobKraft-orm, under the AGPL-3.0 licence;",
        "# the Adaptation Programming Guide of KnobKraft Orm names the functions below.",
        SPDX,
        "# Source: Dee, https://example.org/d, CC-BY-4.0 (community-re)",
    ];
    assert_eq!(header(&text), expected);
}

#[test]
fn the_skeleton_licence_is_stated_once_and_not_confused_with_a_source_licence() {
    let d = with_sources(json!([
        {"type": "community-re", "author": "Dee", "license": "MIT"},
        {"type": "vendor-doc", "author": "Eve", "license": "CC-BY-4.0"}
    ]));
    let (text, _) = exported(&d);
    assert_eq!(text.matches("SPDX-License-Identifier").count(), 1, "{text}");
    let h = header(&text);
    let spdx = h.iter().position(|l| l == SPDX).expect("the SPDX line");
    for (i, l) in h.iter().enumerate().filter(|(_, l)| l.starts_with("# Source: ")) {
        assert!(i > spdx, "{l} comes before the SPDX line");
        assert!(!l.contains("AGPL"), "a source line carries the skeleton's licence: {l}");
    }
    assert!(h.contains(&"# Source: Dee, MIT (community-re)".to_string()), "{h:?}");
    assert!(h.contains(&"# Source: Eve, CC-BY-4.0 (vendor-doc)".to_string()), "{h:?}");
}

#[test]
fn a_source_under_a_compatible_licence_gets_its_source_line_alone_and_no_loss() {
    for license in ["CC-BY-4.0", "MIT", "Apache-2.0", "GPL-3.0", "AGPL-3.0", "AGPL-3.0-only", "GPL-3.0-or-later"] {
        let (text, lost) = exported(&with_license(license));
        let h = header(&text);
        assert_eq!(h.len(), 5, "{license}: {h:?}");
        assert!(h[4].starts_with("# Source: ") && h[4].contains(license), "{license}: {h:?}");
        assert!(!places(&lost).iter().any(|p| p.starts_with("/sources")), "{license}: {lost:?}");
    }
}

#[test]
fn the_licence_is_read_by_its_identifier_without_spaces_or_case() {
    for license in [" cc-by-4.0 ", "mit", "  Apache-2.0", "gpl-3.0-or-later "] {
        let (text, lost) = exported(&with_license(license));
        assert_eq!(header(&text).len(), 5, "{license:?}: {text}");
        assert!(!places(&lost).iter().any(|p| p.starts_with("/sources")), "{license:?}: {lost:?}");
    }
}

#[test]
fn a_source_under_a_licence_not_allowed_under_the_agpl_is_named_under_its_source_line_with_a_loss() {
    let (text, lost) = exported(&with_license("GPL-2.0-only"));
    let h = header(&text);
    assert_eq!(h.len(), 6, "{h:?}");
    assert_eq!(h[4], "# Source: Dee, https://example.org/d, GPL-2.0-only (community-re)");
    assert_eq!(h[5], format!("{NOT_ALLOWED}GPL-2.0-only"));
    let l = lost.iter().find(|l| l.place == "/sources/0/license").unwrap_or_else(|| panic!("{lost:?}"));
    assert_eq!(l.reason, h[5].trim_start_matches("# "));
}

#[test]
fn another_known_licence_outside_the_list_is_not_allowed_too() {
    for license in ["GPL-2.0-only", "GPL-2.0", "gpl-2.0-or-later", "CC-BY-NC-4.0"] {
        let (text, lost) = exported(&with_license(license));
        let h = header(&text);
        assert_eq!(h.len(), 6, "{license}: {h:?}");
        assert!(h[5].starts_with(NOT_ALLOWED), "{license}: {h:?}");
        assert!(places(&lost).contains(&"/sources/0/license"), "{license}: {lost:?}");
    }
}

#[test]
fn a_source_with_a_free_text_licence_an_expression_or_an_unknown_identifier_is_not_recognised() {
    for license in ["MIT-like", "MIT OR Apache-2.0", "Foo-1.0", "all rights reserved"] {
        let (text, lost) = exported(&with_license(license));
        let h = header(&text);
        assert_eq!(h.len(), 6, "{license}: {h:?}");
        assert!(h[4].starts_with("# Source: "), "{h:?}");
        assert_eq!(h[5], format!("{NOT_RECOGNISED}{license}"));
        let l = lost.iter().find(|l| l.place == "/sources/0/license").unwrap_or_else(|| panic!("{license}: {lost:?}"));
        assert_eq!(l.reason, h[5].trim_start_matches("# "));
    }
}

#[test]
fn a_source_without_a_licence_is_not_recognised() {
    for source in [json!({"type": "community-re", "author": "Dee"}), json!({"type": "community-re", "author": "Dee", "license": "  "})] {
        let (text, lost) = exported(&with_sources(json!([source])));
        let h = header(&text);
        assert_eq!(h.len(), 6, "{h:?}");
        assert!(h[5].starts_with(NOT_RECOGNISED.trim_end()), "{h:?}");
        assert!(places(&lost).contains(&"/sources/0/license"), "{lost:?}");
    }
}

#[test]
fn each_source_gets_its_own_warning_under_its_own_line_and_its_own_loss_place() {
    let d = with_sources(json!([
        {"type": "community-re", "author": "Dee", "license": "MIT"},
        {"type": "community-re", "author": "Eve", "license": "GPL-2.0-only"},
        {"type": "community-re", "author": "Fay", "license": "MIT-like"}
    ]));
    let (text, lost) = exported(&d);
    let h = header(&text);
    let at = |needle: &str| h.iter().position(|l| l.starts_with("# Source: ") && l.contains(needle)).unwrap_or_else(|| panic!("{h:?}"));
    assert_eq!(h[at("Eve") + 1], format!("{NOT_ALLOWED}GPL-2.0-only"));
    assert_eq!(h[at("Fay") + 1], format!("{NOT_RECOGNISED}MIT-like"));
    assert!(!h[at("Dee") + 1].starts_with("# Licence"), "{h:?}");
    let p = places(&lost);
    assert!(p.contains(&"/sources/1/license") && p.contains(&"/sources/2/license"), "{p:?}");
    assert!(!p.contains(&"/sources/0/license"), "{p:?}");
}

#[test]
fn a_source_under_a_licence_not_allowed_is_warned_and_never_refused() {
    let (text, _) = exported(&with_license("CC-BY-NC-4.0"));
    assert!(text.contains("def name():"), "{text}");
}

#[test]
fn the_document_and_the_interface_say_the_skeleton_licence_and_why() {
    for path in ["docs/knobkraft.md", "docs/INTERFACE.md"] {
        let doc = read(path);
        assert!(doc.contains("AGPL-3.0-only"), "{path} does not say AGPL-3.0-only");
    }
    let doc = read("docs/knobkraft.md");
    assert!(doc.contains("SPDX-License-Identifier"), "knobkraft.md does not name the SPDX line");
    assert!(doc.contains("9e2522c"), "knobkraft.md does not give the revision of its piece");
}
