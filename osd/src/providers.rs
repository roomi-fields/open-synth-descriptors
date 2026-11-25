//! The table of providers: who supplies a source, cited by identifier (0.7, proposal 0010). The
//! table is a data file beside the format, with a version of its own; the reader carries a copy
//! of `schemas/providers.json` and warns of an identifier its copy does not hold
//! ([`crate::warnings`]).

use serde_json::Value;
use std::collections::HashSet;
use std::sync::OnceLock;

/// The version of the table of providers this reader carries: the date of its last change.
pub const PROVIDERS_VERSION: &str = "2026-11-24";

const TABLE: &str = include_str!("../providers.json");

/// `^[a-z0-9]+(-[a-z0-9]+)*$`, at most 64 characters: the form of an identifier of the table.
pub(crate) fn is_provider(id: &str) -> bool {
    id.len() <= 64 && id.split('-').all(|g| !g.is_empty() && g.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
}

pub(crate) fn known() -> &'static HashSet<String> {
    static IDS: OnceLock<HashSet<String>> = OnceLock::new();
    IDS.get_or_init(|| {
        let table: Value = serde_json::from_str(TABLE).expect("the embedded table is JSON");
        let entries = table["providers"].as_array().expect("the embedded table holds providers");
        entries.iter().filter_map(|e| e["id"].as_str()).map(str::to_string).collect()
    })
}
