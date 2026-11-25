//! The lines of `osd-formats` and `osd-cli` in « ## Consumers » of osd/docs/INTERFACE.md
//! name exactly what the sources of each read from `osd`, and no writing hides a name.
//!
//! The sources are lexed by `proc-macro2`, the lexer of Rust procedural macros: a comment
//! gives no token and the text of a string is one literal, so neither is a reading, and the
//! tokens of a macro call are read as any other.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, TokenStream, TokenTree};

use crate::common;

const INTERFACE: &str = "osd/docs/INTERFACE.md";

/// The consumers of `osd` in this repository; a consumer elsewhere cannot be read.
const CONSUMERS: [&str; 2] = ["osd-formats", "osd-cli"];

/// The keywords after which `::osd` still starts at the crate.
const BEFORE_A_LEADING_PATH: [&str; 15] =
    ["as", "break", "dyn", "else", "for", "if", "impl", "in", "match", "move", "mut", "pub", "return", "use", "where"];

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Refusal {
    /// A name the consumer reads and its line does not list.
    Unlisted { consumer: String, name: String },
    /// A name the line lists and the consumer does not read.
    NoLongerRead { consumer: String, name: String },
    /// A writing after which the names read cannot be told, in a file named from the root.
    Hidden { file: String, form: String },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::Unlisted { consumer, name } => {
                write!(f, "`{consumer}` reads `{name}`, which its line in « ## Consumers » of {INTERFACE} does not list")
            }
            Refusal::NoLongerRead { consumer, name } => {
                write!(f, "the line of `{consumer}` in « ## Consumers » of {INTERFACE} lists `{name}`, which it does not read")
            }
            Refusal::Hidden { file, form } => write!(f, "{file}: `{form}` hides the names read from `osd`"),
        }
    }
}

/// What the sources of one consumer show.
#[derive(Default)]
struct Reading {
    names: BTreeSet<String>,
    hidden: Vec<String>,
}

fn word(token: Option<&TokenTree>) -> Option<String> {
    match token {
        Some(TokenTree::Ident(i)) => Some(i.to_string()),
        _ => None,
    }
}

fn punct(token: Option<&TokenTree>, c: char) -> bool {
    matches!(token, Some(TokenTree::Punct(p)) if p.as_char() == c)
}

/// `::` at `at`.
fn separator(tokens: &[TokenTree], at: usize) -> bool {
    punct(tokens.get(at), ':') && punct(tokens.get(at + 1), ':')
}

fn braced(token: Option<&TokenTree>) -> Option<Vec<TokenTree>> {
    match token {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => Some(g.stream().into_iter().collect()),
        _ => None,
    }
}

fn items(group: &[TokenTree]) -> impl Iterator<Item = &[TokenTree]> {
    group.split(|t| punct(Some(t), ',')).filter(|item| !item.is_empty())
}

/// A use tree that starts at the root of the paths.
fn use_from_root(tree: &[TokenTree], reading: &mut Reading) {
    let tree = if separator(tree, 0) { &tree[2..] } else { tree };
    if let Some(group) = braced(tree.first()) {
        for item in items(&group) {
            use_from_root(item, reading);
        }
    } else if word(tree.first()).as_deref() == Some("osd") {
        if separator(tree, 1) {
            use_under_the_crate(&tree[3..], false, reading);
        } else if word(tree.get(1)).as_deref() == Some("as") {
            let alias = word(tree.get(2)).unwrap_or_default();
            reading.hidden.push(format!("use osd as {alias}"));
        }
    }
}

/// What follows `osd::` in a use tree.
fn use_under_the_crate(tree: &[TokenTree], grouped: bool, reading: &mut Reading) {
    if punct(tree.first(), '*') {
        reading.hidden.push(if grouped { "use osd::{*}" } else { "use osd::*" }.to_string());
    } else if let Some(group) = braced(tree.first()) {
        for item in items(&group) {
            use_under_the_crate(item, true, reading);
        }
    } else if let Some(name) = word(tree.first()) {
        if name != "self" {
            reading.names.insert(name);
        } else if word(tree.get(1)).as_deref() == Some("as") {
            let alias = word(tree.get(2)).unwrap_or_default();
            reading.hidden.push(format!("use osd::{{self as {alias}}}"));
        }
    }
}

/// Whether the `osd` at `at` names the crate, not a segment under another path.
fn starts_at_the_crate(tokens: &[TokenTree], at: usize) -> bool {
    if at < 3 || !separator(tokens, at - 2) {
        return true;
    }
    word(tokens.get(at - 3)).is_none_or(|before| BEFORE_A_LEADING_PATH.contains(&before.as_str()))
}

fn scan(stream: TokenStream, reading: &mut Reading) {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut at = 0;
    while at < tokens.len() {
        match &tokens[at] {
            TokenTree::Group(g) => scan(g.stream(), reading),
            TokenTree::Ident(i) if i == "use" && !punct(tokens.get(at + 1), '<') => {
                let end = tokens[at..].iter().position(|t| punct(Some(t), ';')).map_or(tokens.len(), |n| at + n);
                use_from_root(&tokens[at + 1..end], reading);
                at = end;
            }
            TokenTree::Ident(i) if i == "extern" => {
                let after = |n: usize| word(tokens.get(at + n));
                if after(1).as_deref() == Some("crate") && after(2).as_deref() == Some("osd") && after(3).as_deref() == Some("as") {
                    reading.hidden.push(format!("extern crate osd as {}", after(4).unwrap_or_default()));
                }
            }
            TokenTree::Ident(i) if i == "osd" && separator(&tokens, at + 1) && starts_at_the_crate(&tokens, at) => {
                if let Some(name) = word(tokens.get(at + 3)) {
                    reading.names.insert(name);
                }
            }
            _ => {}
        }
        at += 1;
    }
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// The names the line of `consumer` lists in « ## Consumers »; none when it has no line.
fn listed(interface: &str, consumer: &str) -> BTreeSet<String> {
    let start = format!("- `{consumer}` ");
    interface
        .lines()
        .skip_while(|l| l.trim_end() != "## Consumers")
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .find(|l| l.starts_with(&start))
        .and_then(|l| l.split_once(':'))
        .map(|(_, names)| names.split('`').skip(1).step_by(2).map(str::to_string).collect())
        .unwrap_or_default()
}

/// The guard on the repository at `root`: every refusal, in order.
fn refusals(root: &Path) -> Vec<Refusal> {
    let interface = std::fs::read_to_string(root.join(INTERFACE)).unwrap_or_else(|e| panic!("{INTERFACE}: {e}"));
    let mut out = Vec::new();
    for consumer in CONSUMERS {
        let mut read = BTreeSet::new();
        for path in rust_files(&root.join(consumer).join("src")) {
            let file = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{file}: {e}"));
            let stream: TokenStream = text.parse().unwrap_or_else(|e| panic!("{file} is not lexed: {e}"));
            let mut reading = Reading::default();
            scan(stream, &mut reading);
            read.extend(reading.names);
            out.extend(reading.hidden.into_iter().map(|form| Refusal::Hidden { file: file.clone(), form }));
        }
        let listed = listed(&interface, consumer);
        let consumer = consumer.to_string();
        out.extend(read.difference(&listed).map(|name| Refusal::Unlisted { consumer: consumer.clone(), name: name.clone() }));
        out.extend(listed.difference(&read).map(|name| Refusal::NoLongerRead { consumer: consumer.clone(), name: name.clone() }));
    }
    out.sort();
    out
}

/// A repository in a temporary folder: the two lines of « ## Consumers » and the sources.
fn repository(formats_line: &str, cli_line: &str, files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let interface = format!(
        "# osd — interface\n\n## Overview\n\n| Export |\n|---|\n| `resolve` |\n\n## Consumers\n\n\
         - `synth-core` (osc-bridge): `read`, `NobodyReadsThis`\n\
         - `osd-formats` (this repository): {formats_line}\n\
         - `osd-cli` (this repository): {cli_line}\n\
         - the SyntHub tools: `Profile`\n\n<!-- généré: début -->\n<!-- généré: fin -->\n"
    );
    let put = |path: &str, text: &str| {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    put(INTERFACE, &interface);
    put("osd-formats/src/lib.rs", "");
    put("osd-cli/src/lib.rs", "");
    for (path, text) in files {
        put(path, text);
    }
    dir
}

fn unlisted(consumer: &str, name: &str) -> Refusal {
    Refusal::Unlisted { consumer: consumer.into(), name: name.into() }
}

fn no_longer_read(consumer: &str, name: &str) -> Refusal {
    Refusal::NoLongerRead { consumer: consumer.into(), name: name.into() }
}

#[test]
fn the_consumers_lines_of_the_interface_say_exactly_what_each_consumer_of_this_repository_reads() {
    let found = refusals(&common::repo());
    let lines: Vec<String> = found.iter().map(Refusal::to_string).collect();
    assert!(found.is_empty(), "{} refusals:\n{}", found.len(), lines.join("\n"));
}

#[test]
fn a_name_a_consumer_reads_and_its_line_omits_is_refused_by_name() {
    let dir = repository(
        "`read`",
        "`read`",
        &[
            ("osd-formats/src/lib.rs", "use osd::{read, Source};\n"),
            ("osd-cli/src/lib.rs", "fn f() { osd::read(); osd::write(); let _: osd::Kind; }\n"),
        ],
    );
    let found = refusals(dir.path());
    assert_eq!(found, [unlisted("osd-cli", "Kind"), unlisted("osd-cli", "write"), unlisted("osd-formats", "Source")]);
    let said = found[0].to_string();
    assert!(said.contains("`osd-cli`") && said.contains("`Kind`"), "{said}");
}

#[test]
fn a_name_listed_on_the_line_of_another_consumer_does_not_cover_a_reading() {
    let dir = repository("`write`", "`read`", &[
        ("osd-formats/src/lib.rs", "fn f() { osd::read(); }\n"),
        ("osd-cli/src/lib.rs", "fn f() { osd::write(); }\n"),
    ]);
    assert_eq!(
        refusals(dir.path()),
        [unlisted("osd-cli", "write"), unlisted("osd-formats", "read"), no_longer_read("osd-cli", "read"), no_longer_read("osd-formats", "write")]
    );
}

#[test]
fn a_name_a_line_lists_and_its_consumer_no_longer_reads_is_refused_by_name() {
    let dir = repository(
        "`Source`",
        "`read`, `convert`, `Warning`",
        &[("osd-cli/src/lib.rs", "// osd::convert was here\nfn f() { osd::read(); }\n")],
    );
    let found = refusals(dir.path());
    assert_eq!(
        found,
        [no_longer_read("osd-cli", "Warning"), no_longer_read("osd-cli", "convert"), no_longer_read("osd-formats", "Source")]
    );
    let said = found[0].to_string();
    assert!(said.contains("`osd-cli`") && said.contains("`Warning`"), "{said}");
}

#[test]
fn lines_that_say_what_is_read_pass_and_the_consumers_elsewhere_are_not_judged() {
    let dir = repository(
        "`Source`",
        "`read`, `Kind`",
        &[
            ("osd-formats/src/lib.rs", "use osd::Source;\n"),
            ("osd-cli/src/lib.rs", "fn f() { osd::read(osd::Kind::Device); osd::read(osd::Kind::Sound); }\n"),
        ],
    );
    assert_eq!(refusals(dir.path()), []);
}

#[test]
fn every_rust_file_under_src_is_read_subfolders_and_test_files_included() {
    let dir = repository(
        "",
        "",
        &[
            ("osd-cli/src/deep/er/module.rs", "fn f() { osd::resolve(); }\n"),
            ("osd-cli/src/tests.rs", "use osd::Resolved;\n"),
            ("osd-cli/src/tests/case.rs", "#[test]\nfn t() { assert!(matches!(v, osd::Version::V0_7)); }\n"),
            ("osd-cli/src/notes.md", "osd::NotRust\n"),
        ],
    );
    assert_eq!(
        refusals(dir.path()),
        [unlisted("osd-cli", "Resolved"), unlisted("osd-cli", "Version"), unlisted("osd-cli", "resolve")]
    );
}

#[test]
fn a_path_reads_its_first_segment_wherever_it_is_written() {
    let source = r#"
        impl osd::Files for Folder {}
        impl ::osd::Fault {}
        fn f(d: &osd::Descriptor) -> Result<osd::Device, ::osd::FaultKind> {
            let v = vec![osd::VERSIONS];
            println!("{}", osd::PUBLISHED.name());
            let osd::Descriptor::Sound(s) = d else { return osd::write(d) };
        }
    "#;
    let dir = repository("", "", &[("osd-cli/src/lib.rs", source)]);
    let names = ["Descriptor", "Device", "Fault", "FaultKind", "Files", "PUBLISHED", "VERSIONS", "write"];
    assert_eq!(refusals(dir.path()), names.map(|n| unlisted("osd-cli", n)));
}

#[test]
fn a_use_reads_the_original_name_of_each_entry_of_its_group() {
    let source = r#"
        use osd::Fault;
        use osd::Source as Origin;
        pub use ::osd::Resolved;
        pub(crate) use osd::{
            self,
            Address, // not::Commented
            Kind::{self, Device},
            read as load,
            segment::Part as Piece,
            {Way, write as save},
        };
        use {std::fmt, osd::{Command, Role}};
        fn f() { use osd::roles; }
    "#;
    let dir = repository("", "", &[("osd-formats/src/lib.rs", source)]);
    let names = ["Address", "Command", "Fault", "Kind", "Resolved", "Role", "Source", "Way", "read", "roles", "segment", "write"];
    assert_eq!(refusals(dir.path()), names.map(|n| unlisted("osd-formats", n)));
}

#[test]
fn a_name_in_a_comment_is_not_a_reading() {
    let source = r#"
        //! The crate calls [`osd::InTheCrateDoc`].
        /// Converted by [`osd::InADocComment`].
        // use osd::{InALineComment};
        /* osd::InABlockComment /* osd::Nested */ osd::StillInside */
        fn f() { osd::read(); } // osd::AfterTheCode
    "#;
    let dir = repository("", "`read`", &[("osd-cli/src/lib.rs", source)]);
    assert_eq!(refusals(dir.path()), []);
}

#[test]
fn a_path_of_another_crate_or_module_is_not_a_reading() {
    let source = r#"
        use osd_formats::{import, export};
        use other::osd::Elsewhere;
        use crate::{osd::Local};
        fn f() { osd_formats::import_into(); self::osd::local(); other::osd::thing(); }
    "#;
    let dir = repository("", "", &[("osd-cli/src/lib.rs", source)]);
    assert_eq!(refusals(dir.path()), []);
}

#[test]
fn the_slashes_of_a_string_do_not_hide_a_reading_after_them_on_the_line() {
    let source = r##"
        fn f() {
            let a = ("https://w3id.org/osd", osd::AfterAUrl);
            let b = ('"', "//", osd::AfterACharQuote);
            let c = (r#"a "quote" /* "#, osd::AfterARawString);
            let d = ("an escaped \" //", osd::AfterAnEscape);
        }
        fn g<'a>(x: &'a str) -> &'a str { osd::AfterALifetime }
    "##;
    let dir = repository("", "", &[("osd-cli/src/lib.rs", source)]);
    let names = ["AfterACharQuote", "AfterALifetime", "AfterARawString", "AfterAUrl", "AfterAnEscape"];
    assert_eq!(refusals(dir.path()), names.map(|n| unlisted("osd-cli", n)));
}

#[test]
fn a_glob_at_the_root_of_the_crate_and_a_renaming_of_the_crate_are_refused_by_file_and_form() {
    let cases = [
        ("use osd::*;", "use osd::*"),
        ("pub use ::osd::*;", "use osd::*"),
        ("use {std::fmt, osd::*};", "use osd::*"),
        ("use osd::{*};", "use osd::{*}"),
        ("use osd::{\n    self,\n    *,\n};", "use osd::{*}"),
        ("use osd as x;", "use osd as x"),
        ("pub use ::osd as reader;", "use osd as reader"),
        ("use {osd as x};", "use osd as x"),
        ("use osd::{self as x};", "use osd::{self as x}"),
        ("extern crate osd as x;", "extern crate osd as x"),
        ("fn f() {\n    use osd::*;\n}", "use osd::*"),
    ];
    for (source, form) in cases {
        let dir = repository("", "", &[("osd-cli/src/deep/hidden.rs", source)]);
        let found = refusals(dir.path());
        let expected = Refusal::Hidden { file: "osd-cli/src/deep/hidden.rs".into(), form: form.into() };
        assert_eq!(found, [expected], "{source}");
        let said = found[0].to_string();
        assert!(said.contains("osd-cli/src/deep/hidden.rs") && said.contains(form), "{said}");
    }
}

#[test]
fn a_hiding_writing_leaves_the_other_names_of_its_group_read() {
    let dir = repository("`read`, `write`", "", &[("osd-formats/src/lib.rs", "use osd::{read, self as x, write};\n")]);
    let expected = Refusal::Hidden { file: "osd-formats/src/lib.rs".into(), form: "use osd::{self as x}".into() };
    assert_eq!(refusals(dir.path()), [expected]);
}

#[test]
fn a_glob_under_a_name_reads_that_name() {
    let source = "use osd::Kind::*;\nuse osd::{Version::*, segment::Part::{*}};\n";
    let dir = repository("", "`Kind`, `Version`, `segment`", &[("osd-cli/src/lib.rs", source)]);
    assert_eq!(refusals(dir.path()), []);
}

#[test]
fn the_other_globs_renamings_and_casts_hide_nothing() {
    let source = r#"
        extern crate osd;
        use osd::{self, read};
        use osd_formats::*;
        use osd_formats as formats;
        use super::*;
        use std::io::Write as _;
        use other::osd as theirs;
        fn f(osd: u32) { let _ = osd as u64; }
    "#;
    let dir = repository("", "`read`", &[("osd-cli/src/lib.rs", source)]);
    assert_eq!(refusals(dir.path()), []);
}
