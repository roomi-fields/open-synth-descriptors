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
        assert_permitted(license);
    }
}

/// The source line alone, citing the licence, and no loss about the sources.
fn assert_permitted(license: &str) {
    let (text, lost) = exported(&with_license(license));
    let h = header(&text);
    assert_eq!(h.len(), 5, "{license}: {h:?}");
    assert!(h[4].starts_with("# Source: ") && h[4].contains(license), "{license}: {h:?}");
    assert!(!places(&lost).iter().any(|p| p.starts_with("/sources")), "{license}: {lost:?}");
}

/// The warning line under the source line, citing the whole licence, and the loss with its words.
fn assert_warned(warning: &str, license: &str) {
    let (text, lost) = exported(&with_license(license));
    let h = header(&text);
    assert_eq!(h.len(), 6, "{license}: {h:?}");
    assert_eq!(h[5], format!("{warning}{license}"));
    let l = lost.iter().find(|l| l.place == "/sources/0/license").unwrap_or_else(|| panic!("{license}: {lost:?}"));
    assert_eq!(l.reason, h[5].trim_start_matches("# "));
}

#[test]
fn an_or_is_allowed_when_one_branch_is() {
    for license in ["MIT OR Apache-2.0", "MIT OR GPL-2.0-only", "GPL-2.0-only OR MIT", "Foo-1.0 OR CC-BY-4.0", "(CC-BY-NC-4.0)OR(MIT)"] {
        assert_permitted(license);
    }
}

#[test]
fn an_or_is_not_allowed_when_every_branch_is_not_allowed() {
    for license in ["GPL-2.0-only OR CC-BY-NC-4.0", "GPL-2.0-only OR LGPL-2.1-only OR EPL-1.0"] {
        assert_warned(NOT_ALLOWED, license);
    }
}

#[test]
fn an_or_with_no_allowed_branch_and_one_not_recognised_is_not_recognised() {
    for license in ["GPL-2.0-only OR Foo-1.0", "Foo-1.0 OR GPL-2.0-only", "Foo-1.0 OR LicenseRef-x"] {
        assert_warned(NOT_RECOGNISED, license);
    }
}

#[test]
fn an_and_is_allowed_when_every_branch_is() {
    for license in ["MIT AND Apache-2.0", "MIT AND CC-BY-4.0 AND GPL-3.0-only", "(MIT) AND (Apache-2.0)"] {
        assert_permitted(license);
    }
}

#[test]
fn an_and_is_not_allowed_when_one_branch_is_not_allowed() {
    for license in ["MIT AND GPL-2.0-only", "GPL-2.0-only AND MIT", "Foo-1.0 AND CC-BY-NC-4.0", "MIT AND Foo-1.0 AND GPL-2.0-only"] {
        assert_warned(NOT_ALLOWED, license);
    }
}

#[test]
fn an_and_with_no_branch_forbidden_and_one_not_recognised_is_not_recognised() {
    for license in ["MIT AND Foo-1.0", "LicenseRef-x AND MIT", "Foo-1.0 AND Bar-1.0"] {
        assert_warned(NOT_RECOGNISED, license);
    }
}

#[test]
fn an_expression_is_judged_by_the_spdx_precedence_and_its_parentheses() {
    for license in [
        "GPL-2.0-only AND Foo-1.0 OR MIT",
        "MIT OR GPL-2.0-only AND Foo-1.0",
        "Apache-2.0 AND (GPL-2.0-only OR (MIT AND CC-BY-4.0))",
    ] {
        assert_permitted(license);
    }
    for license in ["(MIT OR GPL-2.0-only) AND CC-BY-NC-4.0", "GPL-2.0-only AND (MIT OR Apache-2.0)"] {
        assert_warned(NOT_ALLOWED, license);
    }
    assert_warned(NOT_RECOGNISED, "MIT AND (GPL-2.0-only OR Foo-1.0)");
}

#[test]
fn a_licence_with_an_exception_is_allowed_when_the_licence_alone_is() {
    for license in [
        "GPL-3.0-only WITH GPL-3.0-linking-exception",
        "MIT WITH Some-exception",
        "GPL-2.0-or-later WITH Classpath-exception-2.0",
        "MIT AND (GPL-3.0-only WITH GPL-3.0-linking-exception)",
    ] {
        assert_permitted(license);
    }
}

#[test]
fn a_licence_not_allowed_or_not_recognised_with_an_exception_is_not_recognised() {
    for license in [
        "GPL-2.0-only WITH Classpath-exception-2.0",
        "Foo-1.0 WITH Bar",
        "MIT AND GPL-2.0-only WITH Classpath-exception-2.0",
        "GPL-2.0-only WITH Classpath-exception-2.0 OR CC-BY-NC-4.0",
    ] {
        assert_warned(NOT_RECOGNISED, license);
    }
}

#[test]
fn a_licence_or_any_later_version_is_allowed_when_it_or_a_later_version_is() {
    for license in [
        "GPL-2.0+",
        "GPL-2.0-or-later",
        "gpl-2.0-or-later",
        " gpl-2.0+ ",
        "GPL-1.0+",
        "GPL-3.0+",
        "AGPL-1.0-or-later",
        "CC-BY-3.0-or-later",
        "Apache-1.1+",
        "MIT+",
        "GPL-2.0+ OR CC-BY-NC-4.0",
    ] {
        assert_permitted(license);
    }
}

#[test]
fn a_licence_or_any_later_version_with_no_allowed_version_is_judged_as_the_licence() {
    for license in ["LGPL-2.1-or-later", "LGPL-2.1+", "LGPL-2.0+", "EPL-1.0+", "CC-BY-SA-3.0-or-later", "CC-BY-NC-3.0+"] {
        assert_warned(NOT_ALLOWED, license);
    }
    // A later version is of the same licence, and never an earlier one.
    for license in ["Foo-1.0+", "Foo-1.0-or-later", "GPL-4.0+", "CC-BY-3.0", "CC-BY-3.0-only"] {
        assert_warned(NOT_RECOGNISED, license);
    }
    assert_warned(NOT_ALLOWED, "GPL-2.0-only");
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
fn an_expression_of_ids_in_lower_case_is_judged_by_the_rules_of_its_licences() {
    for license in ["mit OR GPL-2.0-only", "gpl-2.0-only OR apache-2.0", "mit AND cc-by-4.0", "(curl)OR(mit)"] {
        assert_permitted(license);
    }
    for license in ["mit AND gpl-2.0-only", "gpl-2.0-only OR cc-by-nc-4.0"] {
        assert_warned(NOT_ALLOWED, license);
    }
    for license in ["curl", "mit AND curl", "gpl-2.0-only OR zlib-acknowledgement"] {
        assert_warned(NOT_RECOGNISED, license);
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
    for license in ["GPL-2.0-only", "GPL-2.0", "gpl-2.0-only", "CC-BY-NC-4.0"] {
        let (text, lost) = exported(&with_license(license));
        let h = header(&text);
        assert_eq!(h.len(), 6, "{license}: {h:?}");
        assert!(h[5].starts_with(NOT_ALLOWED), "{license}: {h:?}");
        assert!(places(&lost).contains(&"/sources/0/license"), "{license}: {lost:?}");
    }
}

#[test]
fn a_source_with_a_free_text_licence_a_malformed_expression_or_an_unknown_identifier_is_not_recognised() {
    for license in ["MIT-like", "MIT or Apache-2.0", "MIT OR", "(MIT", "MIT OR NONE", "Foo-1.0", "all rights reserved"] {
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
