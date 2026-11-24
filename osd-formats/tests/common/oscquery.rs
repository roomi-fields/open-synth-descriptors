//! What the OSCQuery tests share: the attributes of a namespace as the OSCQuery proposal
//! (github.com/Vidvox/OSCQueryProposal) lists them, the rules of a namespace written as a
//! check, and the namespaces and devices the tests build.

#![allow(dead_code)]

use serde_json::{json, Value};

/// The attributes of a node of the namespace (the proposal, "Attributes"), and the keys of a
/// RANGE entry.
pub const ATTRIBUTES: [&str; 13] =
    ["FULL_PATH", "CONTENTS", "TYPE", "ACCESS", "RANGE", "DESCRIPTION", "TAGS", "EXTENDED_TYPE", "UNIT", "CRITICAL", "CLIPMODE", "VALUE", "OVERLOADS"];
pub const RANGE_KEYS: [&str; 3] = ["MIN", "MAX", "VALS"];

/// The type tags of the proposal ("Type tags": the OSC 1.0 and 1.1 tags).
pub const TAGS: &str = "ifsbhtdScrmTFNI";

/// The attributes the exporter and the importer carry both ways.
pub const CARRIED: [&str; 8] = ["FULL_PATH", "CONTENTS", "TYPE", "ACCESS", "RANGE", "EXTENDED_TYPE", "MIN", "MAX"];

fn node_violations(node: &Value, path: &str, pointer: &str, out: &mut Vec<String>) {
    let Some(o) = node.as_object() else {
        out.push(format!("{pointer}: a node is an object"));
        return;
    };
    for k in o.keys() {
        if !ATTRIBUTES.contains(&k.as_str()) {
            out.push(format!("{pointer}/{k}: not an attribute of the proposal"));
        }
    }
    match o.get("FULL_PATH").and_then(Value::as_str) {
        Some(p) if p == path => {}
        other => out.push(format!("{pointer}/FULL_PATH: {other:?}, {path:?} expected")),
    }
    let access = o.get("ACCESS").and_then(Value::as_i64);
    if let Some(a) = access {
        if !(0..=3).contains(&a) {
            out.push(format!("{pointer}/ACCESS: {a}, 0 to 3"));
        }
    } else if o.contains_key("ACCESS") {
        out.push(format!("{pointer}/ACCESS: an integer"));
    }
    let types: Option<Vec<char>> = o.get("TYPE").and_then(Value::as_str).map(|t| t.chars().filter(|c| !matches!(c, '[' | ']')).collect());
    if let Some(t) = o.get("TYPE") {
        match t.as_str() {
            None => out.push(format!("{pointer}/TYPE: a string of type tags")),
            Some(s) => {
                if let Some(c) = s.chars().find(|c| !TAGS.contains(*c) && !matches!(c, '[' | ']')) {
                    out.push(format!("{pointer}/TYPE: {c:?} is not a type tag"));
                }
                if access == Some(0) {
                    out.push(format!("{pointer}/TYPE: a container has no type"));
                }
            }
        }
    }
    if let Some(r) = o.get("RANGE") {
        match (r.as_array(), &types) {
            (Some(items), Some(t)) if items.len() == t.len() => {
                for (i, it) in items.iter().enumerate() {
                    if it.is_null() {
                        continue;
                    }
                    let Some(e) = it.as_object() else {
                        out.push(format!("{pointer}/RANGE/{i}: null or an object"));
                        continue;
                    };
                    for k in e.keys() {
                        if !RANGE_KEYS.contains(&k.as_str()) {
                            out.push(format!("{pointer}/RANGE/{i}/{k}: not MIN, MAX or VALS"));
                        }
                    }
                    if e.get("VALS").is_some_and(|v| !v.is_array()) {
                        out.push(format!("{pointer}/RANGE/{i}/VALS: a list of the permitted values"));
                    }
                    if let (Some(lo), Some(hi)) = (e.get("MIN").and_then(Value::as_f64), e.get("MAX").and_then(Value::as_f64)) {
                        if lo > hi {
                            out.push(format!("{pointer}/RANGE/{i}: MIN above MAX"));
                        }
                    }
                }
            }
            _ => out.push(format!("{pointer}/RANGE: one entry per argument of TYPE")),
        }
    }
    if let (Some(e), Some(t)) = (o.get("EXTENDED_TYPE"), &types) {
        if e.as_array().map(Vec::len) != Some(t.len()) {
            out.push(format!("{pointer}/EXTENDED_TYPE: one entry per argument of TYPE"));
        }
    }
    if o.get("DESCRIPTION").is_some_and(|d| !d.is_string()) {
        out.push(format!("{pointer}/DESCRIPTION: a string"));
    }
    if o.get("TAGS").is_some_and(|d| !d.as_array().is_some_and(|a| a.iter().all(Value::is_string))) {
        out.push(format!("{pointer}/TAGS: an array of strings"));
    }
    match o.get("CONTENTS") {
        None => {}
        Some(Value::Object(children)) => {
            let base = path.trim_end_matches('/');
            for (k, c) in children {
                node_violations(c, &format!("{base}/{k}"), &format!("{pointer}/CONTENTS/{k}"), out);
            }
        }
        Some(_) => out.push(format!("{pointer}/CONTENTS: an object of nodes")),
    }
}

/// The violations of the proposal in a namespace text.
pub fn violations(text: &str) -> Vec<String> {
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => return vec![format!("not JSON: {e}")],
    };
    let mut out = Vec::new();
    node_violations(&v, "/", "", &mut out);
    out
}

/// The node at an OSC address of a namespace.
pub fn node_at<'a>(ns: &'a Value, address: &str) -> Option<&'a Value> {
    address.split('/').filter(|s| !s.is_empty()).try_fold(ns, |n, seg| n.get("CONTENTS")?.get(seg))
}

/// A method node of a namespace: its address, TYPE and RANGE.
pub fn methods(ns: &Value) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    fn walk(n: &Value, out: &mut Vec<(String, Value)>) {
        if n["ACCESS"].as_i64().is_some_and(|a| a > 0) {
            out.push((n["FULL_PATH"].as_str().unwrap().to_string(), n.clone()));
        }
        if let Some(c) = n["CONTENTS"].as_object() {
            c.values().for_each(|c| walk(c, out));
        }
    }
    walk(ns, &mut out);
    out
}

/// A method node: ACCESS 2, TYPE and RANGE entries `(min, max)`.
pub fn method(path: &str, types: &str, ranges: &[Option<(i64, i64)>]) -> Value {
    let mut m = json!({"FULL_PATH": path, "ACCESS": 2, "TYPE": types});
    if ranges.iter().any(Option::is_some) {
        m["RANGE"] = Value::Array(ranges.iter().map(|r| r.map_or(Value::Null, |(lo, hi)| json!({"MIN": lo, "MAX": hi}))).collect());
    }
    m
}

/// A container node with its children, each by its path segment.
pub fn container(path: &str, children: Vec<(&str, Value)>) -> Value {
    let contents: serde_json::Map<String, Value> = children.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    json!({"FULL_PATH": path, "ACCESS": 0, "CONTENTS": contents})
}

/// A namespace of one device under `/synth`: the methods given as (segment path, node).
pub fn namespace(methods: Vec<(&str, Value)>) -> Value {
    let kids: Vec<(&str, Value)> = methods;
    container("/", vec![("synth", container("/synth", kids))])
}

/// A source with the credit and licence of a namespace.
pub fn source() -> osd::Source {
    let mut s = super::source();
    s.url = Some("http://127.0.0.1:9000".to_string());
    s.author = Some("A tool".to_string());
    s.license = Some("CC0-1.0".to_string());
    s
}

pub fn exported(device: &osd::Resolved) -> (Value, Vec<osd_formats::Loss>) {
    let t = osd_formats::export(device, osd_formats::Format::OscQuery).expect("the device exports");
    let v = violations(&t.value);
    assert!(v.is_empty(), "the export breaks the proposal: {v:?}\n{}", t.value);
    (serde_json::from_str(&t.value).unwrap(), t.lost)
}

pub fn imported(ns: &Value) -> (Value, Vec<osd_formats::Loss>) {
    let t = osd_formats::import(&ns.to_string(), osd_formats::Format::OscQuery, &source()).unwrap_or_else(|f| panic!("{f}\n{ns}"));
    (super::electra::json_of(&t.value), t.lost)
}
