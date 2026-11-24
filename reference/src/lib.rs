//! osd-reference: the reference of a version of the format, built from its schemas.
//! Its interface, docs/INTERFACE.md, says what it offers.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// A property: a key of an object `properties`, at any depth, `$defs` included.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    /// JSON Pointer of the property inside its schema, e.g. `/properties/a/properties/b`.
    pub pointer: String,
    /// Its `type`, or `$ref <target>`, or `-` when it states neither.
    pub type_: String,
    /// Its own `description`; the description of a `$ref` target does not count.
    pub description: Option<String>,
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn type_of(schema: &Value) -> String {
    match schema.get("type") {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Array(ts)) => ts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" | "),
        _ => match schema.get("$ref").and_then(Value::as_str) {
            Some(r) => format!("$ref {r}"),
            None => "-".to_string(),
        },
    }
}

fn walk(value: &Value, at: &str, out: &mut Vec<Property>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(props)) = map.get("properties") {
                for (key, schema) in props {
                    out.push(Property {
                        pointer: format!("{at}/properties/{}", escape(key)),
                        type_: type_of(schema),
                        description: schema
                            .get("description")
                            .map(|d| d.as_str().map_or_else(|| d.to_string(), str::to_string)),
                    });
                }
            }
            for (key, v) in map {
                walk(v, &format!("{at}/{}", escape(key)), out);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                walk(v, &format!("{at}/{i}"), out);
            }
        }
        _ => {}
    }
}

/// Every property of a schema, sorted by pointer.
pub fn properties(schema: &Value) -> Vec<Property> {
    let mut out = Vec::new();
    walk(schema, "#", &mut out);
    out.sort_by(|a, b| a.pointer.cmp(&b.pointer));
    out
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The page of a kind, in Markdown.
pub fn page(kind: &str, schema: &Value) -> String {
    let props = properties(schema);
    let described = props.iter().filter(|p| p.description.is_some()).count();
    let mut s = format!("# Reference of the kind `{kind}`\n\n");
    s.push_str(&format!("{described} of {} properties carry a description.\n\n", props.len()));
    s.push_str("| Path | Type | Description |\n|---|---|---|\n");
    for p in &props {
        let d = match &p.description {
            Some(d) => cell(d),
            None => "*undescribed*".to_string(),
        };
        s.push_str(&format!("| `{}` | {} | {} |\n", p.pointer, cell(&p.type_), d));
    }
    s
}

/// The schemas of a folder, by kind (`<kind>.schema.json`), refused when the folder holds none,
/// when a file is not JSON, or when a reference no file of the folder answers.
pub fn load(folder: &Path) -> Result<BTreeMap<String, Value>, String> {
    let entries = std::fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut schemas = BTreeMap::new();
    let mut names = BTreeMap::new();
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let Some(kind) = name.strip_suffix(".schema.json") else { continue };
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
        let value: Value = serde_json::from_str(&text).map_err(|e| format!("{name}: not JSON: {e}"))?;
        names.insert(name.clone(), kind.to_string());
        schemas.insert(kind.to_string(), value);
    }
    if schemas.is_empty() {
        return Err(format!("{}: no schema (*.schema.json)", folder.display()));
    }
    for (name, kind) in &names {
        check_refs(name, &schemas[kind], &schemas[kind], &names, &schemas)?;
    }
    Ok(schemas)
}

fn check_refs(
    file: &str,
    node: &Value,
    own: &Value,
    names: &BTreeMap<String, String>,
    schemas: &BTreeMap<String, Value>,
) -> Result<(), String> {
    match node {
        Value::Object(map) => {
            if let Some(Value::String(r)) = map.get("$ref") {
                let (target_file, pointer) = r.split_once('#').unwrap_or((r.as_str(), ""));
                let target = if target_file.is_empty() {
                    Some(own)
                } else {
                    names.get(target_file).map(|k| &schemas[k])
                };
                match target {
                    Some(t) if pointer.is_empty() || t.pointer(pointer).is_some() => {}
                    _ => return Err(format!("{file}: reference {r} answered by no file of the folder")),
                }
            }
            map.values().try_for_each(|v| check_refs(file, v, own, names, schemas))
        }
        Value::Array(items) => items.iter().try_for_each(|v| check_refs(file, v, own, names, schemas)),
        _ => Ok(()),
    }
}

/// `(described, total)` of a schema.
pub fn count(schema: &Value) -> (usize, usize) {
    let all = properties(schema);
    (all.iter().filter(|p| p.description.is_some()).count(), all.len())
}
