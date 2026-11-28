//! In 0.7 a `license` is an SPDX expression (`spec/0.7.md` 6.7; SPDX 2.3, annex « SPDX License
//! Expressions »): ids, `WITH` an exception, `AND`, `OR` and parentheses, or one of the special
//! values alone. The reader and the schemas check its shape, never the SPDX registry, and
//! `LicenseExpression` is the one analysis of it. Versions before 0.7 judge a `license` as before.
// @lit spec/0.7.md spec/proposals/** conformance/VERSION conformance/0.7/read/** schemas/0.7/**

use crate::common::credit::{catalogue, converted, descriptor, made_on, read, warned, with, written_back};
use crate::common::{self, Schemas};

use osd::{Descriptor, FaultKind, Kind, LicenseExpression, Version};
use serde_json::{json, Value};

/// Expressions of 0.7, each well formed.
const EXPRESSIONS: &[&str] = &[
    "GPL-2.0+",
    "LicenseRef-x",
    "DocumentRef-a:LicenseRef-b",
    "DocumentRef-spdx-tool-1.2:LicenseRef-MIT-Style-1",
    "GPL-3.0-only WITH GPL-3.0-linking-exception",
    "GPL-2.0-or-later WITH i2p-gpl-java-exception",
    "Apache-2.0 WITH 389-exception",
    "GPL-2.0+ WITH Bison-exception-2.2",
    "DocumentRef-a:LicenseRef-b WITH Some.exception-1",
    "Foo-1.0 WITH Bar",
    "MIT OR Apache-2.0",
    "MIT AND ISC",
    "MIT   OR    Apache-2.0",
    "GPL-2.0-only  WITH  Classpath-exception-2.0",
    "(MIT)",
    "((MIT))",
    "(MIT OR ISC)",
    "( MIT OR ISC )",
    "(MIT)OR(ISC)",
    "(MIT)AND ISC",
    "MIT AND(ISC OR Zlib)",
    "(MIT OR ISC)AND Zlib",
    "MIT OR Apache-2.0 AND ISC",
    "(GPL-2.0-only WITH Classpath-exception-2.0 OR MIT) AND (ISC OR (Zlib AND BSD-3-Clause))",
    "DocumentRef-a:LicenseRef-b OR MIT",
    "ANDERSEN OR ORIGIN-1.0 AND WITHIN",
];

/// The special values, each accepted alone.
const SPECIAL: [&str; 3] = ["public-domain", "NOASSERTION", "NONE"];

/// Malformed texts whose parentheses are balanced: the reader and the schema refuse each.
const MALFORMED: &[&str] = &[
    // an operator without an operand
    "MIT OR", "MIT AND", "AND MIT", "OR MIT", "AND", "OR", "WITH", "MIT WITH", "WITH x", "MIT OR OR ISC", "MIT AND OR ISC",
    "(MIT OR) AND ISC", "(AND MIT)",
    // two ids without an operator
    "CC BY", "MIT ISC", "MITOR ISC", "MIT ORISC", "(MIT) (ISC)", "(MIT)(ISC)", "MIT (ISC)", "(MIT)ISC", "MIT +",
    // parentheses
    "()", "( )", "MIT OR ()",
    // WITH after a closing parenthesis, repeated, or before a parenthesis
    "(MIT) WITH x", "(MIT OR ISC) WITH x", "MIT WITH x WITH y", "MIT WITH (x)", "MIT WITH(x)",
    // operators in lower case
    "MIT or ISC", "MIT and ISC", "MIT Or ISC", "MIT with x",
    // spaces at the ends, tabs, line breaks and other blanks
    " MIT", "MIT ", " MIT OR ISC", "MIT OR ISC ", " (MIT)", "(MIT) ", "MIT\tOR ISC", "MIT OR\tISC", "MIT\nOR ISC",
    "MIT OR\nISC", "MIT OR ISC\n", "MIT\r\nOR ISC", "MIT\u{a0}OR ISC",
    // `+` and `:` in an exception
    "MIT WITH x+", "MIT WITH a:b", "MIT WITH DocumentRef-a:LicenseRef-b", "MIT WITH x_y",
    // `:` outside `DocumentRef-…:LicenseRef-…`
    "A:B", "MIT:LicenseRef-x", "DocumentRef-a:MIT", "LicenseRef-a:LicenseRef-b", "DocumentRef-:LicenseRef-b",
    "DocumentRef-a:LicenseRef-", "DocumentRef-a:LicenseRef-b+", "DocumentRef-a+:LicenseRef-b",
    "DocumentRef-a:LicenseRef-b:LicenseRef-c", "documentref-a:LicenseRef-b", "DocumentRef-a: LicenseRef-b",
    "MIT OR DocumentRef-a:b",
    // ids that have not the shape
    "", " ", "MIT & ISC", "MIT,ISC", "MIT/ISC", "[MIT]", "MIT_1", "+",
    // the same refusals with ids that start in lower case
    "cc by", "two words", "mit isc", "mit or isc", "mit and isc", "mit with x", "curl With x", "mit AND", "OR mit", "curl WITH",
    "mit AND OR", "OR OR or", "mit OR AND", "curl WITH WITH", "mit_1", "_mit", "+mit", "+curl OR isc", "curl WITH x+", "curl:x",
    "mit:LicenseRef-x", "documentref-a:licenseref-b", "DocumentRef-a:licenseref-b", "curl,isc",
];

/// Ids of 0.7 that no capital starts, and expressions of such ids, each well formed.
const LOWER_CASE: &[&str] = &[
    "curl", "mit", "free", "zlib-acknowledgement", "bzip2-1.0.6", "libpng-2.0", "0BSD", "389", "curl+", "mit++", "0BSD+",
    // an idstring may start with any of its characters
    "-a", ".a", "-", ".", ".+",
    // the operators in lower case are ids by their shape
    "and", "or", "with", "and OR or", "with WITH with", "and AND (or OR with)",
    "mit OR isc", "curl WITH x", "curl+ WITH x", "(mit)", "(curl)OR(0BSD)", "libpng-2.0 AND (bzip2-1.0.6 OR zlib-acknowledgement)",
    "mit OR GPL-2.0-only",
    // whatever the case
    "Mit", "CURL", "cUrL", "Zlib-Acknowledgement", "And", "Or", "With",
];

/// Texts whose parentheses are not balanced: only the reader can count them.
const UNBALANCED: &[&str] = &["(MIT", "MIT)", "((MIT)", "(MIT))", "(MIT OR ISC", "MIT) OR (ISC", ")MIT(", "(MIT OR (ISC AND Zlib)"];

/// The special values in whatever case, as an operand or an exception of an expression.
fn special_inside() -> Vec<String> {
    let mut out = Vec::new();
    for v in ["public-domain", "NOASSERTION", "NONE", "Public-Domain", "PUBLIC-DOMAIN", "NoAssertion", "Noassertion", "noassertion", "None", "none"] {
        for text in [
            format!("{v} OR MIT"),
            format!("MIT AND {v}"),
            format!("MIT WITH {v}"),
            format!("{v} WITH x"),
            format!("({v})"),
            format!("(MIT OR {v}) AND ISC"),
        ] {
            out.push(text);
        }
    }
    out
}

fn refused_texts() -> Vec<String> {
    let mut out: Vec<String> = MALFORMED.iter().map(|s| s.to_string()).collect();
    out.extend(special_inside());
    out.extend(["none", "None", "Noassertion", "Public-Domain"].map(String::from));
    out
}

/// A 0.7 descriptor that writes `license` at one field, with the kind and the pointer of the field.
fn fields(license: &str) -> Vec<(Kind, &'static str, Value)> {
    let source = json!([{"type": "vendor-doc"}, {"type": "community-re", "license": license}]);
    let d = |kind, extra| descriptor("0.7", kind, extra);
    vec![
        (Kind::Device, "/sources/1/license", d(Kind::Device, json!({"sources": source}))),
        (Kind::Device, "/midi_of_sources/1/license", d(Kind::Device, json!({"midi_of_sources": source}))),
        (Kind::Profile, "/sources/1/license", d(Kind::Profile, json!({"sources": source}))),
        (Kind::Sound, "/sources/1/license", d(Kind::Sound, json!({"sources": source}))),
        (Kind::Sound, "/made_on/sources/1/license", d(Kind::Sound, json!({"made_on": made_on(json!({"sources": source}))}))),
        (Kind::Sound, "/license", d(Kind::Sound, json!({"license": license}))),
        (Kind::Fingerprint, "/sources/1/license", d(Kind::Fingerprint, json!({"sources": source}))),
        (Kind::Firmware, "/sources/1/license", d(Kind::Firmware, json!({"sources": source}))),
        (Kind::Firmware, "/license", d(Kind::Firmware, json!({"license": license}))),
        (Kind::Assembly, "/sources/1/license", d(Kind::Assembly, json!({"sources": source}))),
    ]
}

// ---- the reader of 0.7 ----

#[test]
fn each_license_field_of_0_7_reads_an_expression_and_writes_it_back_as_written() {
    for license in EXPRESSIONS.iter().chain(&SPECIAL) {
        for (kind, pointer, doc) in fields(license) {
            let back = written_back(&doc, kind);
            assert_eq!(back.pointer(pointer), Some(&json!(license)), "{kind:?} {pointer}");
        }
    }
}

#[test]
fn each_license_field_of_0_7_refuses_a_malformed_expression_as_a_bad_value_at_the_field() {
    let mut failures = Vec::new();
    for license in refused_texts().iter().map(String::as_str).chain(UNBALANCED.iter().copied()) {
        for (kind, pointer, doc) in fields(license) {
            match osd::read(&doc.to_string(), kind) {
                Ok(_) => failures.push(format!("{license:?} read at {kind:?} {pointer}")),
                Err(f) if (f.kind, f.pointer.as_str()) != (FaultKind::BadValue, pointer) => {
                    failures.push(format!("{license:?} at {kind:?} {pointer}: {f}"))
                }
                Err(_) => {}
            }
        }
    }
    common::report("licences are not refused at their field", failures);
}

#[test]
fn the_refusal_of_a_license_of_0_7_says_an_spdx_expression_is_expected() {
    for (kind, pointer, doc) in fields("MIT or ISC") {
        let f = osd::read(&doc.to_string(), kind).expect_err("refused");
        assert!(f.rule.contains("SPDX") && f.rule.contains("expression"), "{kind:?} {pointer}: {}", f.rule);
    }
}

/// Expressions nested 100 000 levels deep, balanced.
fn deep() -> [String; 3] {
    const N: usize = 100_000;
    [
        format!("{}MIT{}", "(".repeat(N), ")".repeat(N)),
        format!("{}ISC{}", "(MIT OR ".repeat(N), ")".repeat(N)),
        format!("{}ISC{}", "(".repeat(N), " AND MIT)".repeat(N)),
    ]
}

#[test]
fn an_expression_nested_100_000_levels_is_read_when_balanced() {
    for license in deep() {
        let doc = descriptor("0.7", Kind::Sound, json!({"license": license}));
        let Descriptor::Sound(s) = read(&doc, Kind::Sound) else { panic!("a sound") };
        assert!(s.license.as_deref() == Some(license.as_str()), "the expression is not kept as written");
    }
}

#[test]
fn an_expression_nested_100_000_levels_is_refused_when_not_balanced() {
    for license in deep() {
        for broken in [format!("({license}"), format!("{license})"), "(".repeat(100_000), ")".repeat(100_000)] {
            let doc = descriptor("0.7", Kind::Sound, json!({"license": broken}));
            let kind_and_pointer = osd::read(&doc.to_string(), Kind::Sound).map(|_| ()).map_err(|f| (f.kind, f.pointer));
            assert!(kind_and_pointer == Err((FaultKind::BadValue, "/license".to_string())), "not refused at /license");
        }
    }
}

// ---- resolution and the complete descriptor ----

#[test]
fn a_resolution_and_its_complete_descriptor_give_each_expression_as_written() {
    let source = |title: &str, license: &str| json!({"type": "vendor-doc", "title": title, "license": license});
    let (own, inherited, maker) =
        ("(MIT)OR(ISC)", "GPL-3.0-only  WITH  GPL-3.0-linking-exception", "( DocumentRef-a:LicenseRef-b AND GPL-2.0+ )");
    let device = |extra| descriptor("0.7", Kind::Device, extra);
    let files = catalogue(&[
        ("acme/a", device(json!({"midi_of": "acme/b", "sources": [source("own", own)]}))),
        ("acme/b", device(json!({"identity": {"profile": "acme"}, "sources": [source("inherited", inherited)]}))),
        (
            "acme/_profile",
            with(descriptor("0.7", Kind::Profile, json!({})), json!({"sources": [source("maker", maker)]})),
        ),
    ]);
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    let full: Value = serde_json::from_str(&osd::write(&osd::complete_descriptor(&r))).unwrap();
    let licences = |doc: &Value| {
        let mut out: Vec<String> = ["sources", "midi_of_sources"]
            .iter()
            .flat_map(|k| doc[*k].as_array().cloned().unwrap_or_default())
            .map(|s| s["license"].as_str().unwrap().to_string())
            .collect();
        out.sort();
        out
    };
    let mut want = vec![own.to_string(), inherited.to_string(), maker.to_string()];
    want.sort();
    let resolved: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device.clone())))).unwrap();
    assert_eq!(licences(&resolved), want, "{resolved}");
    assert_eq!(licences(&full), want, "{full}");
    assert_eq!(written_back(&full, Kind::Device), full);
}

// ---- the schemas of 0.7 ----

#[test]
fn the_schemas_of_0_7_accept_every_expression_the_reader_accepts() {
    let schemas = Schemas::load("0.7");
    let mut failures = Vec::new();
    for license in EXPRESSIONS.iter().chain(&SPECIAL) {
        for (kind, pointer, doc) in fields(license) {
            if let Some(e) = schemas.check(kind, &doc) {
                failures.push(format!("{license:?} at {kind:?} {pointer}: {e}"));
            }
        }
    }
    common::report("expressions are refused by the schema", failures);
}

#[test]
fn the_schemas_of_0_7_refuse_every_malformed_expression_but_the_unbalanced_ones() {
    let schemas = Schemas::load("0.7");
    let mut failures = Vec::new();
    for license in refused_texts() {
        for (kind, pointer, doc) in fields(&license) {
            // A `made_on` is reported by the schema at or under the field.
            if !schemas.errors(kind, &doc).iter().any(|(at, _)| pointer.starts_with(at.as_str())) {
                failures.push(format!("{license:?} at {kind:?} {pointer}"));
            }
        }
    }
    common::report("malformed expressions are accepted by the schema", failures);
}

#[test]
fn the_schemas_of_0_7_describe_a_license_as_an_spdx_expression() {
    fn licences<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
        match v {
            Value::Object(m) => {
                for (k, v) in m {
                    if k == "license" && v.get("type") == Some(&json!("string")) {
                        out.push(v);
                    }
                    licences(v, out);
                }
            }
            Value::Array(a) => a.iter().for_each(|v| licences(v, out)),
            _ => {}
        }
    }
    for kind in ["device", "sound", "firmware"] {
        let path = common::repo().join(format!("schemas/0.7/{kind}.schema.json"));
        let schema: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut found = Vec::new();
        licences(&schema, &mut found);
        assert!(!found.is_empty(), "{kind}: no license");
        for l in found {
            let d = l["description"].as_str().unwrap_or_default();
            assert!(d.contains("SPDX") && d.contains("expression"), "{kind}: {d}");
        }
    }
}

// ---- an id has the shape of an SPDX idstring, whatever its first character ----

#[test]
fn each_license_field_of_0_7_reads_an_id_of_any_first_character_and_writes_it_back_as_written() {
    let mut failures = Vec::new();
    for license in LOWER_CASE {
        for (kind, pointer, doc) in fields(license) {
            match osd::read(&doc.to_string(), kind) {
                Ok(d) => {
                    let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
                    if back.pointer(pointer) != Some(&json!(license)) {
                        failures.push(format!("{license:?} at {kind:?} {pointer} is not written back as written"));
                    }
                }
                Err(f) => failures.push(format!("{license:?} at {kind:?} {pointer}: {f}")),
            }
        }
    }
    common::report("licences are not read", failures);
}

#[test]
fn the_reader_of_0_7_warns_of_no_license_whatever_the_case_of_its_ids() {
    for license in ["mit", "Mit", "CURL", "curl", "Zlib-Acknowledgement", "free", "MIT OR isc"] {
        for (kind, pointer, doc) in fields(license) {
            assert_eq!(warned(&doc, kind), [], "{license:?} at {kind:?} {pointer}");
        }
    }
}

#[test]
fn the_schemas_of_0_7_accept_every_id_of_any_first_character() {
    let schemas = Schemas::load("0.7");
    let mut failures = Vec::new();
    for license in LOWER_CASE {
        for (kind, pointer, doc) in fields(license) {
            if let Some(e) = schemas.check(kind, &doc) {
                failures.push(format!("{license:?} at {kind:?} {pointer}: {e}"));
            }
        }
    }
    common::report("ids are refused by the schema", failures);
}

/// Texts near the special values and the operators, and every text of one or two characters of a
/// small alphabet, each with whether it is a `license` of 0.7.
fn texts_near_the_edges() -> Vec<(String, bool)> {
    fn cases(word: &str) -> [String; 3] {
        let mut capital = word.to_string();
        capital[..1].make_ascii_uppercase();
        [word.to_string(), word.to_ascii_uppercase(), capital]
    }
    let mut out = Vec::new();
    for value in SPECIAL {
        for word in cases(&value.to_ascii_lowercase()).into_iter().chain([value.to_string()]) {
            out.push((word.clone(), word == value));
            // A part of a special value, or a special value with more, is an id like another.
            for end in 1..word.len() {
                out.push((word[..end].to_string(), true));
                out.push((word[end..].to_string(), true));
            }
            for id in [format!("{word}x"), format!("x{word}"), format!("{word}-1.0"), format!("{word}.x"), format!("{word}+")] {
                out.push((id, true));
            }
        }
    }
    for operator in ["and", "or", "with"] {
        for word in cases(operator) {
            let reserved = word == operator.to_ascii_uppercase();
            out.push((word.clone(), !reserved));
            for id in [format!("{word}x"), format!("x{word}"), format!("{word}-1.0"), format!("{word}+")] {
                out.push((id, true));
            }
        }
    }
    const ALPHABET: [char; 8] = ['a', 'Z', '0', '.', '-', '+', '_', ':'];
    let shaped = |c: char| !matches!(c, '_' | ':');
    for first in ALPHABET {
        let starts = shaped(first) && first != '+';
        out.push((first.to_string(), starts));
        for second in ALPHABET {
            out.push((format!("{first}{second}"), starts && shaped(second)));
        }
    }
    out
}

#[test]
fn the_reader_and_the_schemas_of_0_7_agree_on_the_texts_near_the_special_values_and_the_shape_of_an_id() {
    let schemas = Schemas::load("0.7");
    let mut failures = Vec::new();
    for (license, admitted) in texts_near_the_edges() {
        for (kind, pointer, doc) in fields(&license) {
            if !matches!(pointer, "/license" | "/sources/1/license") {
                continue;
            }
            let read = osd::read(&doc.to_string(), kind).map(|_| ()).map_err(|f| (f.kind, f.pointer));
            let want = if admitted { Ok(()) } else { Err((FaultKind::BadValue, pointer.to_string())) };
            if read != want {
                failures.push(format!("the reader on {license:?} at {kind:?} {pointer}: {read:?}"));
            }
            if schemas.check(kind, &doc).is_none() != admitted {
                failures.push(format!("the schema on {license:?} at {kind:?} {pointer}: accepts is not {admitted}"));
            }
        }
    }
    failures.dedup();
    common::report("texts are judged wrongly", failures);
}

#[test]
fn license_expression_parses_each_id_of_any_first_character_as_written() {
    for text in LOWER_CASE {
        assert!(LicenseExpression::parse(text).is_some(), "{text:?}");
    }
    for id in ["curl", "mit", "zlib-acknowledgement", "bzip2-1.0.6", "0BSD", "and", "or", "with", "CURL", "mit+"] {
        assert_eq!(tree(id), leaf(id));
    }
    assert_eq!(tree("mit OR isc"), or([leaf("mit"), leaf("isc")]));
    assert_eq!(tree("curl WITH x"), leaf("curl|x"));
    assert_eq!(tree("and AND or OR with WITH with"), or([and([leaf("and"), leaf("or")]), leaf("with|with")]));
}

// ---- the versions before 0.7, and the conversion ----

/// A sound or a firmware of 0.5 with `license`.
fn with_license_0_5(kind: Kind, license: &str) -> Value {
    let target = json!({"descriptor": "m-vave/fm-1", "vendor": "M-VAVE", "model": "FM-1"});
    match kind {
        Kind::Sound => descriptor("0.5", Kind::Sound, json!({"license": license})),
        _ => json!({
            "$schema": "https://w3id.org/osd/0.5/firmware.schema.json", "name": "TRIBE", "version": "0.3-beta",
            "device": target, "license": license
        }),
    }
}

/// Expressions that hold more than one id: no version before 0.7 admits one.
const BEYOND_ONE_ID: [&str; 5] =
    ["MIT OR Apache-2.0", "MIT AND ISC", "(MIT)", "GPL-3.0-only WITH GPL-3.0-linking-exception", "DocumentRef-a:LicenseRef-b"];

#[test]
fn in_0_5_and_0_6_an_expression_stays_refused_at_the_field() {
    for license in BEYOND_ONE_ID {
        let source = json!({"_sources": [{"type": "vendor-doc", "license": license}]});
        let cases = [
            (Kind::Device, "/_sources/0/license", descriptor("0.5", Kind::Device, source.clone())),
            (Kind::Device, "/_sources/0/license", descriptor("0.6", Kind::Device, source)),
            (Kind::Sound, "/license", with_license_0_5(Kind::Sound, license)),
            (Kind::Firmware, "/license", with_license_0_5(Kind::Firmware, license)),
            (Kind::Sound, "/license", descriptor("0.6", Kind::Sound, json!({"license": license}))),
            (Kind::Firmware, "/license", descriptor("0.6", Kind::Firmware, json!({"license": license}))),
        ];
        for (kind, pointer, doc) in cases {
            let f = osd::read(&doc.to_string(), kind).expect_err(&format!("read: {doc}"));
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{doc}: {f}");
        }
    }
}

/// A 0.4 text with `license` at each field the table of 0.4 to 0.5 judges is not converted.
fn assert_not_convertible_from_0_4(license: &str) {
    let source = json!([{"type": "vendor-doc", "license": license}]);
    let mut cases = vec![
        (Kind::Device, "/_sources/0/license", descriptor("0.5", Kind::Device, json!({"_sources": source}))),
        (Kind::Profile, "/_sources/0/license", descriptor("0.5", Kind::Profile, json!({"_sources": source}))),
        (Kind::Sound, "/license", with_license_0_5(Kind::Sound, license)),
        (Kind::Firmware, "/license", with_license_0_5(Kind::Firmware, license)),
    ];
    for (kind, pointer, doc) in &mut cases {
        // The 0.4 form of the same text names no `$schema`.
        doc.as_object_mut().unwrap().remove("$schema");
        // A maker profile of 0.4 is judged by the table on its way to 0.7 only.
        let targets: &[Version] = if *kind == Kind::Profile { &[Version::V0_7] } else { &[Version::V0_5, Version::V0_7] };
        for &target in targets {
            let f = osd::convert_to(&doc.to_string(), *kind, target).expect_err(&format!("converted: {doc}"));
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::NotConvertible, *pointer), "{doc}: {f}");
        }
    }
}

#[test]
fn an_expression_in_a_0_4_license_stays_not_convertible_at_its_pointer_up_to_0_7() {
    for license in BEYOND_ONE_ID {
        assert_not_convertible_from_0_4(license);
    }
}

/// Single ids whose first character is no capital: no version before 0.7 admits one.
const NO_CAPITAL_FIRST: [&str; 6] = ["curl", "mit", "free", "zlib-acknowledgement", "bzip2-1.0.6", "0BSD"];

#[test]
fn in_0_5_and_0_6_an_id_that_no_capital_starts_stays_refused_by_the_reader_and_the_schemas() {
    let (schemas_0_5, schemas_0_6) = (Schemas::load("0.5"), Schemas::load("0.6"));
    for license in ["free", "mit", "curl"] {
        let source = json!({"_sources": [{"type": "vendor-doc", "license": license}]});
        let cases = [
            (&schemas_0_5, Kind::Device, "/_sources/0/license", descriptor("0.5", Kind::Device, source.clone())),
            (&schemas_0_6, Kind::Device, "/_sources/0/license", descriptor("0.6", Kind::Device, source)),
            (&schemas_0_5, Kind::Sound, "/license", with_license_0_5(Kind::Sound, license)),
            (&schemas_0_5, Kind::Firmware, "/license", with_license_0_5(Kind::Firmware, license)),
            (&schemas_0_6, Kind::Sound, "/license", descriptor("0.6", Kind::Sound, json!({"license": license}))),
            (&schemas_0_6, Kind::Firmware, "/license", descriptor("0.6", Kind::Firmware, json!({"license": license}))),
        ];
        for (schemas, kind, pointer, doc) in cases {
            let f = osd::read(&doc.to_string(), kind).expect_err(&format!("read: {doc}"));
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{doc}: {f}");
            assert!(schemas.errors(kind, &doc).iter().any(|(at, _)| pointer.starts_with(at.as_str())), "schema: {doc}");
        }
    }
}

#[test]
fn an_id_that_no_capital_starts_stays_not_convertible_where_the_table_of_0_4_judges_it() {
    for license in NO_CAPITAL_FIRST {
        assert_not_convertible_from_0_4(license);
    }
}

/// The 0.4 `_sources` of each kind that no table judges, and the later ones, with the version.
fn unjudged_sources(note: Value) -> Vec<(Kind, Value)> {
    let mut out = Vec::new();
    for (kind, path) in [
        (Kind::Sound, "examples/0.4/warm-pad.sound.json"),
        (Kind::Fingerprint, "examples/0.4/minilogue.fingerprint.json"),
        (Kind::Firmware, "examples/0.4/tribe-0.3-beta.firmware.json"),
    ] {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(common::repo().join(path)).unwrap()).unwrap();
        out.push((kind, with(doc, json!({"_sources": note}))));
    }
    for (kind, path) in [
        (Kind::Profile, "examples/0.5/acme.profile.json"),
        (Kind::Sound, "examples/0.5/warm-pad.sound.json"),
        (Kind::Fingerprint, "examples/0.5/minilogue.fingerprint.json"),
        (Kind::Firmware, "examples/0.5/tribe-0.3-beta.firmware.json"),
    ] {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(common::repo().join(path)).unwrap()).unwrap();
        out.push((kind, with(doc, json!({"_sources": note}))));
    }
    for kind in FREE_KINDS {
        out.push((kind, descriptor("0.6", kind, json!({"_sources": note}))));
    }
    out
}

#[test]
fn an_underscore_source_no_table_judges_goes_to_sources_with_an_id_that_no_capital_starts() {
    let schemas = Schemas::load("0.7");
    let entries: Vec<Value> = NO_CAPITAL_FIRST
        .into_iter()
        .chain(["mit OR isc", "curl WITH x"])
        .map(|l| json!({"type": "vendor-doc", "license": l}))
        .collect();
    let note = Value::Array(entries);
    for (kind, doc) in unjudged_sources(note.clone()) {
        let out = converted(&doc, kind, &schemas);
        assert_eq!(out["sources"], note, "{kind:?}: {doc}");
        assert!(out.get("_sources_2").is_none(), "{kind:?}: {out}");
    }
    let doc = descriptor("0.6", Kind::Sound, json!({"made_on": made_on(json!({"_sources": note}))}));
    assert_eq!(converted(&doc, Kind::Sound, &schemas)["made_on"]["sources"], note);
}

#[test]
fn an_underscore_midi_of_source_of_a_device_goes_to_midi_of_sources_with_an_id_that_no_capital_starts() {
    let schemas = Schemas::load("0.7");
    let note = json!([{"type": "vendor-doc", "license": "curl"}, {"type": "community-re", "license": "mit OR isc"}]);
    for version in ["0.4", "0.5", "0.6"] {
        let mut doc = descriptor("0.6", Kind::Device, json!({"_midi_of_sources": note}));
        match version {
            "0.4" => drop(doc.as_object_mut().unwrap().remove("$schema")),
            _ => doc["$schema"] = json!(format!("https://w3id.org/osd/{version}/device.schema.json")),
        }
        let out = converted(&doc, Kind::Device, &schemas);
        assert_eq!(out["midi_of_sources"], note, "{version}: {out}");
    }
}

#[test]
fn an_underscore_midi_of_source_of_a_profile_stays_a_note() {
    let schemas = Schemas::load("0.7");
    let note = json!([{"type": "vendor-doc", "license": "curl"}]);
    let out = converted(&descriptor("0.6", Kind::Profile, json!({"_midi_of_sources": note})), Kind::Profile, &schemas);
    assert_eq!(out["_midi_of_sources"], note);
    assert!(out.get("midi_of_sources").is_none(), "{out}");
}

/// Single ids 0.6 admits, some of them close to an operator or oddly written.
const SINGLE_IDS: [&str; 12] = [
    "MIT", "GPL-3.0-or-later", "GPL-2.0+", "LicenseRef-x+", "DocumentRef-a", "ANDERSEN", "OR2", "WITHIN", "X", "A+B", "Z..", "A--",
];

#[test]
fn a_single_id_accepted_in_0_6_is_accepted_in_0_7_and_kept_by_the_conversion() {
    let schemas = Schemas::load("0.7");
    for license in SINGLE_IDS.into_iter().chain(SPECIAL) {
        for kind in [Kind::Sound, Kind::Firmware] {
            let doc = descriptor("0.6", kind, json!({"license": license}));
            read(&doc, kind);
            assert_eq!(converted(&doc, kind, &schemas)["license"], json!(license), "{kind:?}");
        }
        let doc = descriptor("0.6", Kind::Device, json!({"_sources": [{"type": "vendor-doc", "license": license}]}));
        read(&doc, Kind::Device);
        assert_eq!(converted(&doc, Kind::Device, &schemas)["sources"][0]["license"], json!(license));
    }
}

/// The kinds whose `_sources` was free before 0.7.
const FREE_KINDS: [Kind; 5] = [Kind::Profile, Kind::Sound, Kind::Fingerprint, Kind::Firmware, Kind::Assembly];

#[test]
fn a_free_underscore_source_whose_license_is_a_well_formed_expression_goes_to_sources() {
    let schemas = Schemas::load("0.7");
    let entries: Vec<Value> = EXPRESSIONS.iter().map(|l| json!({"type": "vendor-doc", "license": l})).collect();
    for kind in FREE_KINDS {
        let out = converted(&descriptor("0.6", kind, json!({"_sources": entries})), kind, &schemas);
        assert_eq!(out["sources"], Value::Array(entries.clone()), "{kind:?}");
        assert!(out.get("_sources_2").is_none(), "{kind:?}: {out}");
    }
    let doc = descriptor("0.6", Kind::Sound, json!({"made_on": made_on(json!({"_sources": entries}))}));
    let out = converted(&doc, Kind::Sound, &schemas);
    assert_eq!(out["made_on"]["sources"], Value::Array(entries));
}

#[test]
fn a_free_underscore_source_whose_license_is_malformed_stays_under_the_note() {
    let schemas = Schemas::load("0.7");
    let good = json!({"type": "vendor-doc", "license": "MIT OR ISC"});
    let bad: Vec<Value> = ["MIT or ISC", "(MIT", "MIT OR NONE", "AND", "OR", "WITH"]
        .iter()
        .map(|l| json!({"type": "vendor-doc", "license": l}))
        .collect();
    let mut note = vec![good.clone()];
    note.extend(bad.clone());
    for kind in FREE_KINDS {
        let out = converted(&descriptor("0.6", kind, json!({"_sources": note})), kind, &schemas);
        assert_eq!(out["sources"], json!([good]), "{kind:?}");
        assert_eq!(out["_sources_2"], Value::Array(bad.clone()), "{kind:?}");
    }
}

// ---- the one analysis of an expression ----

/// What an expression says, the operands of one operator gathered and put in one order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Leaf(String),
    And(Vec<Node>),
    Or(Vec<Node>),
}

fn leaf(id: &str) -> Node {
    Node::Leaf(id.to_string())
}

fn and(operands: impl IntoIterator<Item = Node>) -> Node {
    let mut all: Vec<Node> = operands.into_iter().flat_map(|n| if let Node::And(v) = n { v } else { vec![n] }).collect();
    all.sort();
    Node::And(all)
}

fn or(operands: impl IntoIterator<Item = Node>) -> Node {
    let mut all: Vec<Node> = operands.into_iter().flat_map(|n| if let Node::Or(v) = n { v } else { vec![n] }).collect();
    all.sort();
    Node::Or(all)
}

/// The tree of `text`, a leaf written `id` or `id|exception`.
fn tree(text: &str) -> Node {
    let e = LicenseExpression::parse(text).unwrap_or_else(|| panic!("not parsed: {text:?}"));
    e.judge(
        |id, exception| match exception {
            Some(x) => leaf(&format!("{id}|{x}")),
            None => leaf(id),
        },
        |a, b| and([a, b]),
        |a, b| or([a, b]),
    )
}

#[test]
fn license_expression_parses_each_well_formed_expression_and_no_special_value_nor_malformed_text() {
    for text in EXPRESSIONS {
        assert!(LicenseExpression::parse(text).is_some(), "{text:?}");
    }
    for text in SINGLE_IDS {
        assert!(LicenseExpression::parse(text).is_some(), "{text:?}");
    }
    let refused = refused_texts();
    for text in SPECIAL.into_iter().chain(UNBALANCED.iter().copied()).chain(refused.iter().map(String::as_str)) {
        assert!(LicenseExpression::parse(text).is_none(), "{text:?}");
    }
}

#[test]
fn license_expression_gives_each_leaf_its_id_with_its_plus_and_its_exception() {
    assert_eq!(tree("MIT"), leaf("MIT"));
    assert_eq!(tree("GPL-2.0+"), leaf("GPL-2.0+"));
    assert_eq!(tree("((DocumentRef-a:LicenseRef-b))"), leaf("DocumentRef-a:LicenseRef-b"));
    assert_eq!(tree("GPL-2.0+  WITH  i2p-gpl-java-exception"), leaf("GPL-2.0+|i2p-gpl-java-exception"));
    assert_eq!(tree("MIT AND ISC"), and([leaf("MIT"), leaf("ISC")]));
    assert_eq!(tree("(MIT)OR(ISC)"), or([leaf("MIT"), leaf("ISC")]));
}

#[test]
fn license_expression_binds_with_before_and_before_or_and_follows_the_parentheses() {
    let [a, b, c, d] = ["A", "B", "C", "D"].map(leaf);
    assert_eq!(tree("A OR B AND C"), or([a.clone(), and([b.clone(), c.clone()])]));
    assert_eq!(tree("A AND B OR C"), or([and([a.clone(), b.clone()]), c.clone()]));
    assert_eq!(tree("A AND B OR C AND D"), or([and([a.clone(), b.clone()]), and([c.clone(), d.clone()])]));
    assert_eq!(tree("(A OR B) AND C"), and([or([a.clone(), b.clone()]), c.clone()]));
    assert_eq!(tree("A AND (B OR C) AND D"), and([a.clone(), or([b.clone(), c.clone()]), d.clone()]));
    assert_eq!(tree("A OR (B OR C AND D)"), or([a.clone(), b.clone(), and([c.clone(), d.clone()])]));
    assert_eq!(tree("A AND B WITH x OR C WITH y"), or([and([a, leaf("B|x")]), leaf("C|y")]));
}

#[test]
fn license_expression_judges_into_any_type() {
    let has_mit = |text: &str| {
        LicenseExpression::parse(text).unwrap().judge(|id, _| id == "MIT", |a, b| a && b, |a, b| a || b)
    };
    assert!(has_mit("MIT OR ISC AND Zlib"));
    assert!(!has_mit("(MIT OR ISC) AND Zlib"));
    assert!(!has_mit("MIT AND ISC"));
}

#[test]
fn license_expression_parses_judges_and_drops_a_deep_expression_without_overflowing_the_stack() {
    let mut texts = deep().to_vec();
    texts.push(format!("MIT{}", " OR MIT".repeat(100_000)));
    texts.push(format!("MIT{}", " AND MIT".repeat(100_000)));
    let leaves = [1, 100_001, 100_001, 100_001, 100_001];
    for (text, want) in texts.iter().zip(leaves) {
        let e = LicenseExpression::parse(text).expect("a deep expression");
        let got = e.judge(|_, _| 1usize, |a, b| a + b, |a, b| a + b);
        assert_eq!(got, want);
    }
    assert!(LicenseExpression::parse(&"(".repeat(100_000)).is_none());
}

// ---- the text, the proposal and the cases ----

/// The section 6.7 of the text of 0.7.
fn section_of_license() -> String {
    let text = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    let start = text.find("### 6.7 `license`").expect("section 6.7");
    let end = start + text[start..].find("### 6.8").expect("section 6.8");
    text[start..end].to_string()
}

#[test]
fn the_section_of_the_license_cites_the_proposal_that_brings_the_expression() {
    let name = "-a-licence-is-an-spdx-expression.md";
    let file = std::fs::read_dir(common::repo().join("spec/proposals"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .find(|f| f.ends_with(name))
        .unwrap_or_else(|| panic!("no spec/proposals/<n>{name}"));
    let section = section_of_license();
    let number = file.split('-').next().unwrap();
    assert!(section.contains(&file) || section.contains(&format!("proposal {number}")), "6.7 does not cite {file}");
}

/// The cases of 6.7: their name, whether the reader refuses, whether the schema accepts a refusal,
/// and what the licence of the case holds.
const CASES: [(&str, bool, bool, &[&str]); 5] = [
    ("license-with-exception-accepted", false, false, &[" WITH "]),
    ("license-and-or-parentheses-accepted", false, false, &[" AND ", " OR ", "("]),
    ("license-expression-malformed-refused", true, false, &[]),
    ("license-parenthesis-unclosed-refused", true, true, &["("]),
    ("license-special-value-in-expression-refused", true, false, &[]),
];

#[test]
fn the_cases_of_an_expression_exist_under_their_names_and_the_section_of_the_license_names_them() {
    let section = section_of_license();
    let cases = section.lines().filter(|l| l.starts_with("Cases:")).collect::<Vec<_>>().join(" ");
    for (name, refused, schema_accepts, holds) in CASES {
        let dir = common::repo().join("conformance/0.7/read").join(name);
        let file = |f: &str| -> Value {
            let text = std::fs::read_to_string(dir.join(f)).unwrap_or_else(|e| panic!("{name}/{f}: {e}"));
            serde_json::from_str(&text).unwrap()
        };
        let (case, doc) = (file("case.json"), file("descriptor.json"));
        if refused {
            assert_eq!(case["fault"]["kind"], "BadValue", "{name}");
            let pointer = case["fault"]["pointer"].as_str().unwrap();
            assert!(pointer.ends_with("/license"), "{name}: {pointer}");
        } else {
            assert!(case.get("result").is_some(), "{name}");
        }
        assert_eq!(case.get("schema_accepts") == Some(&json!(true)), schema_accepts, "{name}");
        let text = doc.to_string();
        for part in holds {
            assert!(text.contains(part), "{name}: no {part:?} in {text}");
        }
        assert!(cases.contains(&format!("0.7/read/{name}")), "6.7 does not name {name}: {cases}");
    }
}

#[test]
fn the_version_of_the_suite_rose_by_a_minor_number_for_the_cases_of_an_expression() {
    let text = std::fs::read_to_string(common::repo().join("conformance/VERSION")).unwrap();
    let parts: Vec<u32> = text.trim().split('.').map(|p| p.parse().unwrap()).collect();
    assert!(parts[0] > 1 || parts[1] >= 19, "{text}");
}

#[test]
fn the_case_of_an_id_in_lower_case_is_accepted_by_the_reader_and_the_schema_and_named_by_the_section() {
    let name = "license-lowercase-id-accepted";
    let dir = common::repo().join("conformance/0.7/read").join(name);
    let file = |f: &str| -> Value {
        let text = std::fs::read_to_string(dir.join(f)).unwrap_or_else(|e| panic!("{name}/{f}: {e}"));
        serde_json::from_str(&text).unwrap()
    };
    let (case, doc) = (file("case.json"), file("descriptor.json"));
    assert!(case.get("result").is_some() && case.get("fault").is_none(), "{case}");
    assert!(case.get("schema_accepts").is_none(), "{case}");
    let kind = case["read"].as_str().unwrap();
    assert_eq!(kind, "device", "the case reads a device");
    assert_eq!(Schemas::load("0.7").check(Kind::Device, &doc), None, "{doc}");
    assert_eq!(written_back(&doc, Kind::Device), case["result"]);
    let sources = doc["sources"].as_array().expect("sources");
    assert!(
        sources.iter().filter_map(|s| s["license"].as_str()).any(|l| l.starts_with(|c: char| c.is_ascii_lowercase())),
        "no id in lower case: {doc}"
    );
    let section = section_of_license();
    let cases = section.lines().filter(|l| l.starts_with("Cases:")).collect::<Vec<_>>().join(" ");
    assert!(cases.contains(&format!("0.7/read/{name}")), "6.7 does not name {name}: {cases}");
}

#[test]
fn the_version_of_the_suite_rose_by_a_minor_number_for_the_case_of_an_id_in_lower_case() {
    let text = std::fs::read_to_string(common::repo().join("conformance/VERSION")).unwrap();
    let parts: Vec<u32> = text.trim().split('.').map(|p| p.parse().unwrap()).collect();
    assert!(parts[0] > 1 || parts[1] >= 20, "{text}");
}

/// The proposal that brings the expression.
fn proposal_of_license() -> String {
    std::fs::read_to_string(common::repo().join("spec/proposals/0024-a-licence-is-an-spdx-expression.md")).unwrap()
}

#[test]
fn the_text_and_the_proposal_no_longer_ask_a_capital_first_of_a_licence_id() {
    for (name, text) in [("6.7", section_of_license()), ("proposal 0024", proposal_of_license())] {
        let rule = text.lines().find(|l| l.starts_with("license-id")).unwrap_or_else(|| panic!("{name}: no license-id"));
        assert!(!rule.contains("capital"), "{name}: {rule}");
        // The check of the reader and the departures from SPDX say nothing of a capital.
        for line in text.lines() {
            let departure = line.starts_with("> NOTE The other departures") || line.starts_with("- a licence id");
            if departure || line.starts_with("A reader MUST check the shape") {
                assert!(!line.contains("capital"), "{name}: {line}");
            }
        }
    }
}

#[test]
fn the_section_of_the_license_says_the_table_of_0_4_keeps_the_former_shape_as_not_convertible() {
    let section = section_of_license();
    assert!(section.contains("NotConvertible"), "{section}");
}
